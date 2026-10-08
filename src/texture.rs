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

const ETC_MODS: [[i32; 2]; 8] = [
    [2, 8],
    [5, 17],
    [9, 29],
    [13, 42],
    [18, 60],
    [24, 80],
    [33, 106],
    [47, 183],
];

const EAC_TABLE_11: [i32; 8] = [-3, -5, -7, -9, 2, 4, 6, 8];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Etc2Level {
    pub width: u16,
    pub height: u16,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Etc2Mipchain {
    pub orig_width: u16,
    pub orig_height: u16,
    pub has_alpha: bool,
    pub levels: Vec<Etc2Level>,
}

impl Etc2Mipchain {
    const MAGIC: &'static [u8; 8] = b"LGETC2V1";

    pub fn to_bytes(&self) -> Vec<u8> {
        let payload_bytes: usize = self.levels.iter().map(|l| 8 + l.bytes.len()).sum();
        let mut out = Vec::with_capacity(16 + payload_bytes);
        out.extend_from_slice(Self::MAGIC);
        out.extend_from_slice(&self.orig_width.to_le_bytes());
        out.extend_from_slice(&self.orig_height.to_le_bytes());
        out.push(u8::from(self.has_alpha));
        out.push(self.levels.len() as u8);
        out.extend_from_slice(&0u16.to_le_bytes());
        for level in &self.levels {
            out.extend_from_slice(&level.width.to_le_bytes());
            out.extend_from_slice(&level.height.to_le_bytes());
            out.extend_from_slice(&(level.bytes.len() as u32).to_le_bytes());
            out.extend_from_slice(&level.bytes);
        }
        out
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 16 || &data[0..8] != Self::MAGIC {
            return None;
        }
        let orig_width = u16::from_le_bytes([data[8], data[9]]);
        let orig_height = u16::from_le_bytes([data[10], data[11]]);
        let has_alpha = data[12] != 0;
        let count = data[13] as usize;
        if orig_width == 0 || orig_height == 0 || count == 0 || count > 16 {
            return None;
        }
        let mut cursor = 16usize;
        let mut levels = Vec::with_capacity(count);
        let bytes_per_block = if has_alpha { 16usize } else { 8usize };
        for _ in 0..count {
            if cursor + 8 > data.len() {
                return None;
            }
            let width = u16::from_le_bytes([data[cursor], data[cursor + 1]]);
            let height = u16::from_le_bytes([data[cursor + 2], data[cursor + 3]]);
            let len = u32::from_le_bytes([
                data[cursor + 4],
                data[cursor + 5],
                data[cursor + 6],
                data[cursor + 7],
            ]) as usize;
            cursor += 8;
            let expected = (width as usize).div_ceil(4) * (height as usize).div_ceil(4) * bytes_per_block;
            if width == 0 || height == 0 || len != expected || cursor + len > data.len() {
                return None;
            }
            levels.push(Etc2Level {
                width,
                height,
                bytes: data[cursor..cursor + len].to_vec(),
            });
            cursor += len;
        }
        if cursor != data.len() {
            return None;
        }
        Some(Self {
            orig_width,
            orig_height,
            has_alpha,
            levels,
        })
    }
}

#[inline]
fn fit_subblock(
    pixels: &[[u8; 4]; 16],
    indices: &[usize; 8],
    base: [i32; 3],
) -> (u64, u64, u64) {
    let mut dl = [0i32; 8];
    let mut max_abs = 0i32;
    for (slot, &k) in indices.iter().enumerate() {
        let p = pixels[k];
        let d = ((p[0] as i32 - base[0]) + (p[1] as i32 - base[1]) * 2 + (p[2] as i32 - base[2])) >> 2;
        dl[slot] = d;
        max_abs = max_abs.max(d.abs());
    }
    let first_t = if max_abs <= 12 {
        0
    } else if max_abs <= 23 {
        1
    } else if max_abs <= 35 {
        2
    } else if max_abs <= 51 {
        3
    } else if max_abs <= 70 {
        4
    } else if max_abs <= 93 {
        5
    } else {
        6
    };
    let mut best_table = first_t as u64;
    let mut best_msb = 0u64;
    let mut best_lsb = 0u64;
    let mut best_err = i32::MAX;
    for t in first_t..=(first_t + 1).min(7) {
        let [small, large] = ETC_MODS[t];
        let mid = small + large;
        let mut msb = 0u64;
        let mut lsb = 0u64;
        let mut err = 0i32;
        for (slot, &k) in indices.iter().enumerate() {
            let d = dl[slot];
            let neg = d < 0;
            let big = d.abs() * 2 > mid;
            let m = match (neg, big) {
                (false, false) => small,
                (false, true) => large,
                (true, false) => -small,
                (true, true) => -large,
            };
            let p = pixels[k];
            let dr = (base[0] + m).clamp(0, 255) - p[0] as i32;
            let dg = (base[1] + m).clamp(0, 255) - p[1] as i32;
            let db = (base[2] + m).clamp(0, 255) - p[2] as i32;
            err += dr * dr + dg * dg * 2 + db * db;
            if neg {
                msb |= 1u64 << k;
            }
            if big {
                lsb |= 1u64 << k;
            }
        }
        if err < best_err {
            best_err = err;
            best_table = t as u64;
            best_msb = msb;
            best_lsb = lsb;
        }
    }
    (best_table, best_msb, best_lsb)
}

/// Encode a 4x4 block (column-major `k = x * 4 + y`) into an 8-byte ETC2 RGB8 block.
pub fn encode_etc2_rgb8_block(pixels: &[[u8; 4]; 16]) -> [u8; 8] {
    const SUB_FLIP0: [[usize; 8]; 2] = [[0, 1, 2, 3, 4, 5, 6, 7], [8, 9, 10, 11, 12, 13, 14, 15]];
    const SUB_FLIP1: [[usize; 8]; 2] = [[0, 1, 4, 5, 8, 9, 12, 13], [2, 3, 6, 7, 10, 11, 14, 15]];

    let avg = |idxs: &[usize; 8]| -> [i32; 3] {
        let (mut r, mut g, mut b) = (0i32, 0i32, 0i32);
        for &k in idxs {
            r += pixels[k][0] as i32;
            g += pixels[k][1] as i32;
            b += pixels[k][2] as i32;
        }
        [(r + 4) >> 3, (g + 4) >> 3, (b + 4) >> 3]
    };

    let f0_a = avg(&SUB_FLIP0[0]);
    let f0_b = avg(&SUB_FLIP0[1]);
    let f1_a = avg(&SUB_FLIP1[0]);
    let f1_b = avg(&SUB_FLIP1[1]);
    let dist0 = (f0_a[0] - f0_b[0]).abs() + (f0_a[1] - f0_b[1]).abs() + (f0_a[2] - f0_b[2]).abs();
    let dist1 = (f1_a[0] - f1_b[0]).abs() + (f1_a[1] - f1_b[1]).abs() + (f1_a[2] - f1_b[2]).abs();
    let (flip, subs, avg0, avg1) = if dist1 > dist0 {
        (1u64, &SUB_FLIP1, f1_a, f1_b)
    } else {
        (0u64, &SUB_FLIP0, f0_a, f0_b)
    };

    let q5_0 = avg0.map(|c| (c * 31 + 127) / 255);
    let q5_1 = avg1.map(|c| (c * 31 + 127) / 255);
    let dr = q5_1[0] - q5_0[0];
    let dg = q5_1[1] - q5_0[1];
    let db = q5_1[2] - q5_0[2];
    let use_diff = (-4..=3).contains(&dr) && (-4..=3).contains(&dg) && (-4..=3).contains(&db);

    let (color_bits, diff_bit, base0, base1) = if use_diff {
        let b0 = q5_0.map(|q| (q << 3) | (q >> 2));
        let b1 = q5_1.map(|q| (q << 3) | (q >> 2));
        let r_field = ((q5_0[0] as u64) << 3) | ((dr as u8 & 0x7) as u64);
        let g_field = ((q5_0[1] as u64) << 3) | ((dg as u8 & 0x7) as u64);
        let b_field = ((q5_0[2] as u64) << 3) | ((db as u8 & 0x7) as u64);
        ((r_field << 16) | (g_field << 8) | b_field, 1u64, b0, b1)
    } else {
        let q4_0 = avg0.map(|c| (c * 15 + 127) / 255);
        let q4_1 = avg1.map(|c| (c * 15 + 127) / 255);
        let b0 = q4_0.map(|q| (q << 4) | q);
        let b1 = q4_1.map(|q| (q << 4) | q);
        let r_field = ((q4_0[0] as u64) << 4) | (q4_1[0] as u64);
        let g_field = ((q4_0[1] as u64) << 4) | (q4_1[1] as u64);
        let b_field = ((q4_0[2] as u64) << 4) | (q4_1[2] as u64);
        ((r_field << 16) | (g_field << 8) | b_field, 0u64, b0, b1)
    };

    let (t0, msb0, lsb0) = fit_subblock(pixels, &subs[0], base0);
    let (t1, msb1, lsb1) = fit_subblock(pixels, &subs[1], base1);
    let block = (color_bits << 40)
        | (t0 << 37)
        | (t1 << 34)
        | (diff_bit << 33)
        | (flip << 32)
        | ((msb0 | msb1) << 16)
        | (lsb0 | lsb1);
    block.to_be_bytes()
}

/// Encode a 4x4 block's alpha channel into an 8-byte EAC 8-bit alpha block.
pub fn encode_eac_alpha_block(pixels: &[[u8; 4]; 16]) -> [u8; 8] {
    let mut min_a = 255u8;
    let mut max_a = 0u8;
    for p in pixels {
        min_a = min_a.min(p[3]);
        max_a = max_a.max(p[3]);
    }
    if min_a == max_a {
        // Table 13 entry 4 has modifier 0 -> exact lossless constant alpha.
        let word = ((min_a as u64) << 56)
            | (1u64 << 52)
            | (13u64 << 48)
            | 0x9249_2492_4924u64;
        return word.to_be_bytes();
    }
    let base = ((min_a as i32 + max_a as i32 + 1) >> 1).clamp(0, 255);
    let span = (max_a - min_a) as i32;
    let mult = ((span + 16) / 17).clamp(1, 15);
    let mut indices = 0u64;
    for (k, p) in pixels.iter().enumerate() {
        let a = p[3] as i32;
        let mut best_idx = 0u64;
        let mut best_err = i32::MAX;
        for (idx, &m) in EAC_TABLE_11.iter().enumerate() {
            let val = (base + m * mult).clamp(0, 255);
            let err = (val - a).abs();
            if err < best_err {
                best_err = err;
                best_idx = idx as u64;
            }
        }
        indices |= best_idx << ((15 - k) * 3);
    }
    let word = ((base as u64) << 56) | ((mult as u64) << 52) | (11u64 << 48) | indices;
    word.to_be_bytes()
}

pub fn compress_etc2_level(width: u16, height: u16, rgba: &[u8], has_alpha: bool) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    let bx_count = w.div_ceil(4);
    let by_count = h.div_ceil(4);
    let bytes_per_block = if has_alpha { 16 } else { 8 };
    let mut out = Vec::with_capacity(bx_count * by_count * bytes_per_block);
    let mut block_pixels = [[0u8; 4]; 16];
    for by in 0..by_count {
        for bx in 0..bx_count {
            for x in 0..4 {
                let px = (bx * 4 + x).min(w - 1);
                for y in 0..4 {
                    let py = (by * 4 + y).min(h - 1);
                    let src = (py * w + px) * 4;
                    block_pixels[x * 4 + y] = [
                        rgba[src],
                        rgba[src + 1],
                        rgba[src + 2],
                        rgba[src + 3],
                    ];
                }
            }
            if has_alpha {
                out.extend_from_slice(&encode_eac_alpha_block(&block_pixels));
            }
            out.extend_from_slice(&encode_etc2_rgb8_block(&block_pixels));
        }
    }
    out
}

pub fn downsample_2x(width: u16, height: u16, rgba: &[u8]) -> (u16, u16, Vec<u8>) {
    let nw = (width / 2).max(1);
    let nh = (height / 2).max(1);
    let w = width as usize;
    let h = height as usize;
    let mut out = vec![0u8; nw as usize * nh as usize * 4];
    for y in 0..nh as usize {
        let y0 = (y * 2).min(h - 1);
        let y1 = (y * 2 + 1).min(h - 1);
        for x in 0..nw as usize {
            let x0 = (x * 2).min(w - 1);
            let x1 = (x * 2 + 1).min(w - 1);
            let i00 = (y0 * w + x0) * 4;
            let i10 = (y0 * w + x1) * 4;
            let i01 = (y1 * w + x0) * 4;
            let i11 = (y1 * w + x1) * 4;
            let dst = (y * nw as usize + x) * 4;
            for c in 0..4 {
                out[dst + c] = ((rgba[i00 + c] as u16
                    + rgba[i10 + c] as u16
                    + rgba[i01 + c] as u16
                    + rgba[i11 + c] as u16
                    + 2)
                    >> 2) as u8;
            }
        }
    }
    (nw, nh, out)
}

pub fn build_etc2_mipchain(image: RgbaImage, max_dim: u16, mipmaps: bool) -> Etc2Mipchain {
    let orig_width = image.width;
    let orig_height = image.height;
    let has_alpha = image.pixels.chunks_exact(4).any(|p| p[3] < 255);
    let mut w = image.width;
    let mut h = image.height;
    let mut pixels = image.pixels;
    while (w > max_dim || h > max_dim) && (w > 1 || h > 1) {
        (w, h, pixels) = downsample_2x(w, h, &pixels);
    }
    let mut levels = Vec::new();
    loop {
        levels.push(Etc2Level {
            width: w,
            height: h,
            bytes: compress_etc2_level(w, h, &pixels, has_alpha),
        });
        if !mipmaps || (w == 1 && h == 1) {
            break;
        }
        (w, h, pixels) = downsample_2x(w, h, &pixels);
    }
    Etc2Mipchain {
        orig_width,
        orig_height,
        has_alpha,
        levels,
    }
}

fn cache_file_path(key: &str, max_dim: u16, mipmaps: bool) -> Option<std::path::PathBuf> {
    let dir = crate::android::etc2_cache_dir()?;
    let mut hash = 0xcbf29ce484222325u64;
    for &b in key.as_bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash ^= (max_dim as u64) << 1 | u64::from(mipmaps);
    hash = hash.wrapping_mul(0x100000001b3);
    Some(dir.join(format!("{hash:016x}.etc2")))
}

const GL_COMPRESSED_RGB8_ETC2: u32 = 0x9274;
const GL_COMPRESSED_RGBA8_ETC2_EAC: u32 = 0x9278;
const GL_TEXTURE_BINDING_2D: u32 = 0x8069;

fn upload_etc2_mipchain(
    chain: &Etc2Mipchain,
    repeat: bool,
) -> Option<macroquad::prelude::Texture2D> {
    use macroquad::prelude::*;
    unsafe {
        let gl = get_internal_gl();
        let wrap = if repeat {
            macroquad::miniquad::TextureWrap::Repeat
        } else {
            macroquad::miniquad::TextureWrap::Clamp
        };
        let id = gl.quad_context.new_texture(
            macroquad::miniquad::TextureAccess::Static,
            macroquad::miniquad::TextureSource::Empty,
            macroquad::miniquad::TextureParams {
                format: macroquad::miniquad::TextureFormat::RGBA8,
                wrap,
                min_filter: FilterMode::Linear,
                mag_filter: FilterMode::Linear,
                mipmap_filter: if chain.levels.len() > 1 {
                    macroquad::miniquad::MipmapFilterMode::Linear
                } else {
                    macroquad::miniquad::MipmapFilterMode::None
                },
                width: chain.orig_width as u32,
                height: chain.orig_height as u32,
                ..Default::default()
            },
        );
        let macroquad::miniquad::RawId::OpenGl(raw) = gl.quad_context.texture_raw_id(id);
        let mut prev_tex: i32 = 0;
        macroquad::miniquad::gl::glGetIntegerv(GL_TEXTURE_BINDING_2D, &mut prev_tex);
        macroquad::miniquad::gl::glBindTexture(macroquad::miniquad::gl::GL_TEXTURE_2D, raw);
        while macroquad::miniquad::gl::glGetError() != 0 {}

        let internal_format = if chain.has_alpha {
            GL_COMPRESSED_RGBA8_ETC2_EAC
        } else {
            GL_COMPRESSED_RGB8_ETC2
        };
        let last_level = (chain.levels.len() - 1) as i32;
        macroquad::miniquad::gl::glTexParameteri(
            macroquad::miniquad::gl::GL_TEXTURE_2D,
            macroquad::miniquad::gl::GL_TEXTURE_BASE_LEVEL,
            0,
        );
        macroquad::miniquad::gl::glTexParameteri(
            macroquad::miniquad::gl::GL_TEXTURE_2D,
            macroquad::miniquad::gl::GL_TEXTURE_MAX_LEVEL,
            last_level,
        );
        for (idx, level) in chain.levels.iter().enumerate() {
            macroquad::miniquad::gl::glCompressedTexImage2D(
                macroquad::miniquad::gl::GL_TEXTURE_2D,
                idx as i32,
                internal_format,
                level.width as i32,
                level.height as i32,
                0,
                level.bytes.len() as i32,
                level.bytes.as_ptr() as *const _,
            );
        }
        let err = macroquad::miniquad::gl::glGetError();
        macroquad::miniquad::gl::glBindTexture(
            macroquad::miniquad::gl::GL_TEXTURE_2D,
            prev_tex as u32,
        );
        if err != 0 {
            gl.quad_context.delete_texture(id);
            return None;
        }
        Some(Texture2D::from_miniquad_texture(id))
    }
}

fn upload_rgba8_fallback(
    image: &RgbaImage,
    repeat: bool,
    mipmaps: bool,
) -> macroquad::prelude::Texture2D {
    use macroquad::prelude::*;
    let t = Texture2D::from_rgba8(image.width, image.height, &image.pixels);
    t.set_filter(FilterMode::Linear);
    unsafe {
        let gl = get_internal_gl();
        if repeat {
            gl.quad_context.texture_set_wrap(
                t.raw_miniquad_id(),
                macroquad::miniquad::TextureWrap::Repeat,
                macroquad::miniquad::TextureWrap::Repeat,
            );
        }
        if mipmaps {
            gl.quad_context.texture_generate_mipmaps(t.raw_miniquad_id());
            gl.quad_context.texture_set_min_filter(
                t.raw_miniquad_id(),
                FilterMode::Linear,
                macroquad::miniquad::MipmapFilterMode::Linear,
            );
        }
    }
    t
}

/// Upload a decoded image to OpenGL at its exact dimensions (no preset downscaling), using hardware ETC2 on Android.
pub fn upload_gpu_unscaled(
    cache_key: &str,
    image: RgbaImage,
    repeat: bool,
    mipmaps: bool,
) -> macroquad::prelude::Texture2D {
    upload_gpu_with_limit(
        cache_key,
        image,
        repeat,
        mipmaps,
        if crate::android::is_android() { 8192 } else { 4096 },
    )
}

/// Upload a decoded image to OpenGL, using hardware ETC2 + precomputed mipmaps + disk cache on Android.
pub fn upload_gpu(
    cache_key: &str,
    image: RgbaImage,
    repeat: bool,
    mipmaps: bool,
) -> macroquad::prelude::Texture2D {
    let max_dim = crate::android::active_preset().max_texture_size();
    upload_gpu_with_limit(cache_key, image, repeat, mipmaps, max_dim)
}

fn upload_gpu_with_limit(
    cache_key: &str,
    image: RgbaImage,
    repeat: bool,
    mipmaps: bool,
    max_dim: u16,
) -> macroquad::prelude::Texture2D {
    if !crate::android::is_android() || (image.width < 4 && image.height < 4) {
        return upload_rgba8_fallback(&image, repeat, mipmaps);
    }
    let cache_path = (!cache_key.is_empty()).then(|| cache_file_path(cache_key, max_dim, mipmaps)).flatten();
    if let Some(path) = &cache_path {
        if let Ok(bytes) = std::fs::read(path) {
            if let Some(chain) = Etc2Mipchain::from_bytes(&bytes) {
                if let Some(tex) = upload_etc2_mipchain(&chain, repeat) {
                    return tex;
                }
            }
        }
    }
    let fallback_copy = RgbaImage {
        width: image.width,
        height: image.height,
        pixels: image.pixels.clone(),
    };
    let chain = build_etc2_mipchain(image, max_dim, mipmaps);
    if let Some(path) = &cache_path {
        let _ = std::fs::write(path, chain.to_bytes());
    }
    upload_etc2_mipchain(&chain, repeat)
        .unwrap_or_else(|| upload_rgba8_fallback(&fallback_copy, repeat, mipmaps))
}

thread_local! {
    static GPU_CACHE: std::cell::RefCell<BTreeMap<(String, u16, bool, bool), macroquad::prelude::Texture2D>> =
        const { std::cell::RefCell::new(BTreeMap::new()) };
}

/// Load and upload a 3D texture by asset path, checking the in-memory and on-disk `.etc2` caches first on Android
/// so cached textures skip `.pk3` reading and image decoding entirely.
pub fn load_gpu(
    assets: &mut Assets,
    path: &str,
    repeat: bool,
    mipmaps: bool,
) -> Result<macroquad::prelude::Texture2D> {
    if crate::android::is_android() {
        let max_dim = crate::android::active_preset().max_texture_size();
        let mem_key = (path.to_owned(), max_dim, repeat, mipmaps);
        if let Some(tex) = GPU_CACHE.with(|c| c.borrow().get(&mem_key).cloned()) {
            return Ok(tex);
        }
        if let Some(cache_path) = cache_file_path(path, max_dim, mipmaps) {
            if let Ok(bytes) = std::fs::read(&cache_path) {
                if let Some(chain) = Etc2Mipchain::from_bytes(&bytes) {
                    if let Some(tex) = upload_etc2_mipchain(&chain, repeat) {
                        GPU_CACHE.with(|c| {
                            let mut map = c.borrow_mut();
                            if map.len() < 1536 {
                                map.insert(mem_key, tex.clone());
                            }
                        });
                        return Ok(tex);
                    }
                }
            }
        }
        let image = decode(assets, path)?;
        let tex = upload_gpu(path, image, repeat, mipmaps);
        GPU_CACHE.with(|c| {
            let mut map = c.borrow_mut();
            if map.len() < 1536 {
                map.insert(mem_key, tex.clone());
            }
        });
        return Ok(tex);
    }
    let image = decode(assets, path)?;
    Ok(upload_gpu(path, image, repeat, mipmaps))
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

    #[test]
    fn etc2_mipchain_compresses_and_roundtrips_binary_cache() {
        let mut pixels = Vec::with_capacity(16 * 16 * 4);
        for y in 0..16u8 {
            for x in 0..16u8 {
                pixels.extend_from_slice(&[x * 16, y * 16, 128, 255]);
            }
        }
        let chain = build_etc2_mipchain(
            RgbaImage {
                width: 16,
                height: 16,
                pixels,
            },
            8,
            true,
        );
        assert!(!chain.has_alpha);
        assert_eq!(chain.orig_width, 16);
        assert_eq!(chain.orig_height, 16);
        assert_eq!(chain.levels.len(), 4); // 8x8, 4x4, 2x2, 1x1
        assert_eq!(chain.levels[0].width, 8);
        assert_eq!(chain.levels[0].bytes.len(), 4 * 8); // 2x2 blocks * 8 bytes
        assert_eq!(chain.levels[3].width, 1);
        assert_eq!(chain.levels[3].bytes.len(), 8); // 1x1 block * 8 bytes

        let encoded = chain.to_bytes();
        let decoded = Etc2Mipchain::from_bytes(&encoded).expect("valid etc2 cache");
        assert_eq!(chain, decoded);

        // Alpha block test (constant and gradient)
        let const_block = [[10, 20, 30, 173]; 16];
        let eac_const = u64::from_be_bytes(encode_eac_alpha_block(&const_block));
        assert_eq!((eac_const >> 56) as u8, 173);
        assert_eq!((eac_const >> 48) & 0xF, 13);
    }
}
