use crate::assets::Assets;
use anyhow::{ensure, Context, Result};
use std::collections::BTreeMap;

pub struct RgbaImage {
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<u8>,
}

pub fn decode_ftx(b: &[u8]) -> Result<RgbaImage> {
    ensure!(b.len() >= 12, "Truncated FTX header");
    let w = u32::from_le_bytes(b[0..4].try_into()?);
    let h = u32::from_le_bytes(b[4..8].try_into()?);
    ensure!(
        w > 0 && h > 0 && w <= 4096 && h <= 4096,
        "Invalid or oversized FTX dimensions: {w}x{h}"
    );
    let bytes = (w as usize)
        .checked_mul(h as usize)
        .and_then(|n| n.checked_mul(4))
        .context("FTX size overflow")?;
    ensure!(
        b.len() == 12 + bytes,
        "FTX payload size mismatch: {} versus {}",
        b.len(),
        12 + bytes
    );
    Ok(RgbaImage {
        width: w as u16,
        height: h as u16,
        pixels: b[12..].to_vec(),
    })
}

#[derive(Clone, Default, Debug)]
pub struct MaterialSpec {
    pub face_side: crate::materials::FaceSide,
    pub candidates: Vec<String>,
    pub hidden: bool,
    pub unlit: bool,
    pub transparent: bool,
    pub sky: bool,
    pub portal_sky: bool,
    pub camera_portal: bool,
    pub cloud_height: f32,
    pub deforms: Vec<crate::materials::Deform>,
    pub fog: Option<[f32; 4]>,
    pub stages: Vec<crate::materials::Stage>,
}

pub fn read_materials(assets: &mut Assets) -> Result<BTreeMap<String, MaterialSpec>> {
    let files = assets
        .names()
        .filter(|n| n.ends_with(".shader"))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let mut result = BTreeMap::new();
    for name in files {
        let bytes = assets.read(&name)?;
        let text = String::from_utf8_lossy(&bytes);
        result.extend(
            crate::materials::parse(&text).with_context(|| format!("Material file {name}"))?,
        );
    }
    Ok(result)
}

pub fn resolve(
    assets: &Assets,
    name: &str,
    specs: &BTreeMap<String, MaterialSpec>,
) -> Option<String> {
    let key = name.trim_start_matches('/').to_ascii_lowercase();
    let candidates = specs
        .get(&key)
        // Model declarations often name a shader using its image extension.
        .or_else(|| key.rsplit_once('.').and_then(|(stem, _)| specs.get(stem)))
        .map(|s| s.candidates.as_slice())
        .unwrap_or(&[]);
    for candidate in candidates
        .iter()
        .map(String::as_str)
        .chain(std::iter::once(key.as_str()))
    {
        let candidate = candidate.trim_start_matches('/');
        let stem = candidate
            .rsplit_once('.')
            .map(|(s, _)| s)
            .unwrap_or(candidate);
        for ext in ["ftx", "tga", "jpg", "png"] {
            let path = format!("{stem}.{ext}");
            if assets.contains(&path) {
                return Some(path);
            }
        }
    }
    None
}

pub fn decode(assets: &mut Assets, name: &str) -> Result<RgbaImage> {
    let bytes = assets.read(name)?;
    if name.ends_with(".ftx") {
        return decode_ftx(&bytes).with_context(|| name.to_owned());
    }
    let format = if name.ends_with(".tga") {
        image::ImageFormat::Tga
    } else if name.ends_with(".jpg") {
        image::ImageFormat::Jpeg
    } else {
        image::ImageFormat::Png
    };
    let mut reader = image::io::Reader::with_format(std::io::Cursor::new(bytes), format);
    let mut limits = image::io::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode()?.to_rgba8();
    ensure!(image.width() > 0 && image.height() > 0, "Empty image");
    Ok(RgbaImage {
        width: image.width() as u16,
        height: image.height() as u16,
        pixels: image.into_raw(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ftx_preserves_rgba_and_rejects_bad_sizes() {
        let mut b = Vec::new();
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&[17, 23, 42, 255]);
        let image = decode_ftx(&b).unwrap();
        assert_eq!(image.pixels, [17, 23, 42, 255]);
        b.pop();
        assert!(decode_ftx(&b).is_err());
        b[..4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(decode_ftx(&b).is_err());
        assert!(decode_ftx(&[]).is_err());
    }
}
