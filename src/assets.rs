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
    pub fn open(base: &Path) -> Result<Self> {
        let mut paths = fs::read_dir(base)
            .with_context(|| {
                format!(
                    "Cannot read game data at {}. Pass --data <base folder>.",
                    base.display()
                )
            })?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|s| s.eq_ignore_ascii_case("pk3")))
            .collect::<Vec<_>>();
        paths.sort_by_key(|p| p.file_name().unwrap_or_default().to_ascii_lowercase());
        if paths.is_empty() {
            bail!("No PK3 archives found in {}", base.display());
        }
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
