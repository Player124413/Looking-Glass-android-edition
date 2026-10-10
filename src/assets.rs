use anyhow::{bail, Context, Result};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufReader, Read},
    path::{Path, PathBuf},
};
use zip::ZipArchive;

const MAX_ENTRY: u64 = 128 * 1024 * 1024;

/// Read-only PK3 mount. Later alphabetically sorted packs override earlier packs.
pub struct Assets {
    packs: Vec<ZipArchive<BufReader<File>>>,
    entries: BTreeMap<String, (usize, usize, u64, u32)>,
    pub base: PathBuf,
}

impl Assets {
    /// Stable identity of the mounted, override-resolved corpus, independent of its folder.
    /// Central-directory CRC/size metadata avoids decoding every asset at startup.
    pub fn fingerprint(&mut self) -> Result<String> {
        use sha2::{Digest, Sha256};
        let mut hash = Sha256::new();
        for (name, &(_pack, _index, size, crc32)) in &self.entries {
            hash.update((name.len() as u64).to_le_bytes());
            hash.update(name.as_bytes());
            hash.update(size.to_le_bytes());
            hash.update(crc32.to_le_bytes());
        }
        Ok(format!("{:x}", hash.finalize()))
    }
/// Open the PK3 corpus at `base` with all mod packs from `mods_dir` layered on top.
/// Mod packs are loaded alphabetically AFTER the base packs so any file they
/// contain overrides the original (same way pak5_mod.pk3 wins over pak0..pak3).
/// Missing/empty `mods_dir` is silently tolerated so launching without mods
/// installed works unchanged.
pub fn open(base: &Path) -> Result<Self> {
    open_with_mods(base, None)
}

pub fn open_with_mods(base: &Path, mods_dir: Option<&Path>) -> Result<Self> {
    let mut paths = collect_pk3_paths(base)?;
    if let Some(mods) = mods_dir {
        if let Ok(mut mod_paths) = collect_pk3_paths(mods) {
            paths.append(&mut mod_paths);
        }
    }
    if paths.is_empty() {
        bail!("No PK3 archives found in {}", base.display());
    }
    // Base packs are already alphabetically sorted within their directory; mod
    // packs are appended after (later wins) but also sorted so their override
    // order is deterministic by filename.
    let mut entries = BTreeMap::new();
    let mut packs = Vec::new();
    for path in paths {
        let file = File::open(&path)?;
        let mut pack = ZipArchive::new(BufReader::with_capacity(64 * 1024, file))
            .with_context(|| format!("Invalid PK3: {}", path.display()))?;
        let p = packs.len();
        for i in 0..pack.len() {
            let entry = pack.by_index_raw(i)?;
            if !entry.is_dir() {
                entries.insert(
                    normalize(entry.name()),
                    (p, i, entry.size(), entry.crc32()),
                );
            }
        }
        packs.push(pack);
    }
    Ok(Self {
        packs,
        entries,
        base: base.to_path_buf(),
    })
}

fn collect_pk3_paths(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = fs::read_dir(dir)
        .with_context(|| format!("Cannot read folder {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().is_some_and(|s| s.eq_ignore_ascii_case("pk3")))
        .collect::<Vec<_>>();
    paths.sort_by_key(|p| p.file_name().unwrap_or_default().to_ascii_lowercase());
    Ok(paths)
}

/// Extract the contents of a ZIP or 7Z archive (selected by file extension)
/// into `dest`, preserving relative sub-paths and writing any .pk3 files
/// encountered (and recursively any .zip/.7z files inside) into `dest`.
/// Used by the Android launcher's mod importer and by "Select PK3 / ZIP".
pub fn import_archive(src: &Path, dest: &Path) -> Result<usize> {
    fs::create_dir_all(dest).with_context(|| format!("creating {}", dest.display()))?;
    let lower = src
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    match lower.as_str() {
        "pk3" => {
            // Straight copy: a single pk3 lands in dest.
            let target = dest.join(src.file_name().context("archive without filename")?);
            fs::copy(src, &target)
                .with_context(|| format!("copying {} -> {}", src.display(), target.display()))?;
            Ok(1)
        }
        "zip" => import_zip(src, dest),
        "7z" | "7zip" => bail!(
            "7z archives are not supported directly. Extract '{}' on your PC/device \
             first (e.g. with ZArchiver) to get the .pk3 files inside, then select those .pk3 \
             files with the Install Mod button.",
            src.file_name().and_then(|f| f.to_str()).unwrap_or("archive")
        ),
        other => bail!("Unsupported archive type: {other}"),
    }
}

fn import_zip(src: &Path, dest: &Path) -> Result<usize> {
    let file = File::open(src)?;
    let mut zip = ZipArchive::new(BufReader::with_capacity(64 * 1024, file))
        .with_context(|| format!("Invalid ZIP: {}", src.display()))?;
    let tmp_dir = dest.join(".tmp_mod_extract");
    fs::create_dir_all(&tmp_dir).ok();
    let mut count = 0usize;
    // Walk entries by index inside a block per entry so the ZipFile borrow
    // is released before recursing into nested archives.
    let n = zip.len();
    for i in 0..n {
        // Stage 1: collect name + is_dir by briefly borrowing.
        let (name, is_dir): (String, bool) = {
            let entry = zip.by_index(i)?;
            (entry.name().to_owned(), entry.is_dir())
        };
        let rel = sanitize_archive_path(&name);
        if rel.is_empty() {
            continue;
        }
        let lower = rel.rsplit('/').next().unwrap_or("").to_ascii_lowercase();
        if lower.is_empty() || is_dir {
            continue;
        }
        let out_path = if lower.ends_with(".pk3") {
            dest.join(&rel)
        } else if lower.ends_with(".zip") || lower.ends_with(".7z") || lower.ends_with(".7zip") {
            tmp_dir.join(&rel)
        } else {
            continue;
        };
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).ok();
        }
        // Stage 2: stream entry bytes out to the target path.
        {
            let mut entry = zip.by_index(i)?;
            let mut f = File::create(&out_path)
                .with_context(|| format!("writing {}", out_path.display()))?;
            std::io::copy(&mut BufReader::new(&mut entry).take(512 * 1024 * 1024), &mut f).ok();
        }
        if lower.ends_with(".pk3") {
            count += 1;
        } else {
            // Nested archive: recurse, then delete temp copy.
            if let Ok(n) = import_archive(&out_path, dest) {
                count += n;
            }
            let _ = fs::remove_file(&out_path);
        }
    }
    let _ = fs::remove_dir_all(&tmp_dir);
    Ok(count)
}

fn sanitize_archive_path(name: &str) -> String {
    // Strip drive letters, leading /, ../ traversal segments. Return a safe
    // lower-case relative path (only the final filename matters for pk3s since
    // they are dropped into a flat destination, but preserving subfolders for
    // nested archives keeps things consistent).
    let mut out = String::with_capacity(name.len());
    for part in name.replace('\\', "/").split('/') {
        if part.is_empty() || part == "." || part == ".." {
            continue;
        }
        // Drop Windows drive letters like "C:".
        if part.ends_with(':') && part.len() == 2 {
            continue;
        }
        if !out.is_empty() {
            out.push('/');
        }
        out.push_str(part);
    }
    out.to_ascii_lowercase()
}

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }
    pub fn contains(&self, name: &str) -> bool {
        self.entries.contains_key(&normalize(name))
    }
    pub fn pack_count(&self) -> usize {
        self.packs.len()
    }
    pub fn read(&mut self, name: &str) -> Result<Vec<u8>> {
        let &(p, i, _, _) = self
            .entries
            .get(&normalize(name))
            .with_context(|| format!("Asset not found: {name}"))?;
        let entry = self.packs[p].by_index(i)?;
        if entry.size() > MAX_ENTRY {
            bail!("Asset exceeds 128 MiB limit: {name}");
        }
        let mut data = Vec::with_capacity(entry.size() as usize);
        entry
            .take(MAX_ENTRY + 1)
            .read_to_end(&mut data)
            .with_context(|| format!("Reading {name}"))?;
        if data.len() as u64 > MAX_ENTRY {
            bail!("Expanded asset exceeds limit: {name}");
        }
        Ok(data)
    }
    pub fn maps(&self) -> Vec<String> {
        self.names()
            .filter(|n| n.starts_with("maps/") && n.ends_with(".bsp"))
            .map(|n| {
                n.trim_start_matches("maps/")
                    .trim_end_matches(".bsp")
                    .to_owned()
            })
            .collect()
    }
}
fn normalize(s: &str) -> String {
    s.replace('\\', "/").to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn later_pack_wins_and_paths_are_case_insensitive() -> Result<()> {
        let dir = std::env::temp_dir().join(format!("looking-glass-vfs-{}", std::process::id()));
        fs::create_dir_all(&dir)?;
        for (name, value) in [("pak0.pk3", b"old"), ("pak5.pk3", b"new")] {
            let mut z = zip::ZipWriter::new(File::create(dir.join(name))?);
            z.start_file("Textures/Test.ftx", zip::write::FileOptions::default())?;
            z.write_all(value)?;
            z.finish()?;
        }
        let mut a = Assets::open(&dir)?;
        assert_eq!(a.read("TEXTURES\\test.ftx")?, b"new");
        assert!(a.read("missing").is_err());
        drop(a);
        for name in ["pak0.pk3", "pak5.pk3"] {
            fs::remove_file(dir.join(name))?;
        }
        fs::remove_dir(dir)?;
        Ok(())
    }
}
