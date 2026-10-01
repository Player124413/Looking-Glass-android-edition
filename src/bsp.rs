use anyhow::{bail, ensure, Context, Result};
use macroquad::prelude::{vec2, vec3, Vec2, Vec3};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct MapVertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    pub light_uv: Vec2,
    pub color: [u8; 4],
}
#[derive(Debug)]
pub struct Shader {
    pub name: String,
    pub flags: i32,
    pub contents: i32,
}
#[derive(Clone, Copy, Debug)]
pub struct Plane {
    pub normal: Vec3,
    pub distance: f32,
}
#[derive(Debug)]
pub struct MapBrush {
    pub sides: std::ops::Range<usize>,
    pub shader: usize,
}
#[derive(Debug)]
pub struct Surface {
    pub shader: usize,
    pub kind: i32,
    pub first_vertex: usize,
    pub vertex_count: usize,
    pub first_index: usize,
    pub index_count: usize,
    pub lightmap: i32,
    pub patch_width: usize,
    pub patch_height: usize,
}
#[derive(Debug)]
pub struct Model {
    pub min: Vec3,
    pub max: Vec3,
    pub surfaces: std::ops::Range<usize>,
    pub brushes: std::ops::Range<usize>,
}
#[derive(Debug)]
pub struct Fog {
    pub shader: String,
    pub brush: usize,
}
#[derive(Debug)]
pub struct Bsp {
    pub visibility: crate::visibility::Visibility,
    pub difficulty: crate::powerups::Difficulty,
    pub models: Vec<Model>,
    pub fogs: Vec<Fog>,
    pub shaders: Vec<Shader>,
    pub vertices: Vec<MapVertex>,
    pub indices: Vec<usize>,
    pub surfaces: Vec<Surface>,
    pub lightmaps: Vec<u8>,
    pub entities: Vec<BTreeMap<String, String>>,
    pub world_surfaces: std::ops::Range<usize>,
    pub planes: Vec<Plane>,
    pub side_planes: Vec<usize>,
    pub brushes: Vec<MapBrush>,
    pub world_brushes: std::ops::Range<usize>,
    pub world_min: Vec3,
    pub world_max: Vec3,
}

fn i32_at(b: &[u8], o: usize) -> Result<i32> {
    let end = o.checked_add(4).context("Offset overflow")?;
    Ok(i32::from_le_bytes(
        b.get(o..end).context("Truncated BSP field")?.try_into()?,
    ))
}
fn count(b: &[u8], o: usize) -> Result<usize> {
    usize::try_from(i32_at(b, o)?).context("Negative BSP count or offset")
}
fn f32_at(b: &[u8], o: usize) -> Result<f32> {
    let f = f32::from_bits(i32_at(b, o)? as u32);
    ensure!(f.is_finite(), "Non-finite BSP coordinate");
    Ok(f)
}
fn vec_at(b: &[u8], o: usize) -> Result<Vec3> {
    Ok(vec3(f32_at(b, o)?, f32_at(b, o + 4)?, f32_at(b, o + 8)?))
}
fn records(b: &[u8], stride: usize) -> Result<std::slice::ChunksExact<'_, u8>> {
    ensure!(
        b.len() % stride == 0,
        "Lump length {} is not a multiple of {stride}",
        b.len()
    );
    Ok(b.chunks_exact(stride))
}
fn bounded(first: usize, size: usize, total: usize) -> Result<std::ops::Range<usize>> {
    let end = first.checked_add(size).context("BSP range overflow")?;
    ensure!(end <= total, "BSP range {first}..{end} exceeds {total}");
    Ok(first..end)
}

impl Bsp {
    pub fn parse(b: &[u8]) -> Result<Self> {
        ensure!(b.len() >= 172, "Truncated FAKK BSP header");
        ensure!(&b[..4] == b"FAKK", "Expected an Alice FAKK BSP");
        ensure!(
            i32_at(b, 4)? == 42,
            "Unsupported FAKK BSP version (expected 42)"
        );
        let mut lumps = Vec::new();
        let mut ranges = Vec::new();
        for i in 0..20 {
            let offset = count(b, 12 + i * 8)?;
            let length = count(b, 16 + i * 8)?;
            let range = bounded(offset, length, b.len())?;
            if length > 0 {
                ensure!(offset >= 172, "Lump overlaps header");
                ranges.push(range.clone());
            }
            lumps.push(&b[range]);
        }
        ranges.sort_by_key(|r| r.start);
        ensure!(
            ranges.windows(2).all(|w| w[0].end <= w[1].start),
            "Overlapping BSP lumps"
        );
        let shaders = records(lumps[0], 76)?
            .map(|s| {
                let n = s[..64].iter().position(|&b| b == 0).unwrap_or(64);
                Ok(Shader {
                    name: std::str::from_utf8(&s[..n])?.to_owned(),
                    flags: i32_at(s, 64)?,
                    contents: i32_at(s, 68)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let vertices = records(lumps[4], 44)?
            .map(|s| {
                Ok(MapVertex {
                    position: vec_at(s, 0)?,
                    normal: vec_at(s, 28)?.normalize_or_zero(),
                    uv: vec2(f32_at(s, 12)?, f32_at(s, 16)?),
                    light_uv: vec2(f32_at(s, 20)?, f32_at(s, 24)?),
                    color: s[40..44].try_into()?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let indices = records(lumps[5], 4)?
            .map(|s| count(s, 0))
            .collect::<Result<Vec<_>>>()?;
        ensure!(
            lumps[2].len() % (128 * 128 * 3) == 0,
            "Invalid lightmap lump"
        );
        let lightmap_count = lumps[2].len() / (128 * 128 * 3);
        let surfaces = records(lumps[3], 108)?
            .map(|s| {
                let surface = Surface {
                    shader: count(s, 0)?,
                    kind: i32_at(s, 8)?,
                    first_vertex: count(s, 12)?,
                    vertex_count: count(s, 16)?,
                    first_index: count(s, 20)?,
                    index_count: count(s, 24)?,
                    lightmap: i32_at(s, 28)?,
                    patch_width: count(s, 96)?,
                    patch_height: count(s, 100)?,
                };
                ensure!(surface.shader < shaders.len(), "Invalid shader index");
                ensure!(
                    surface.vertex_count < 60000 && surface.index_count < 180000,
                    "Surface exceeds viewer mesh capacity"
                );
                bounded(surface.first_vertex, surface.vertex_count, vertices.len())?;
                let ir = bounded(surface.first_index, surface.index_count, indices.len())?;
                ensure!(
                    indices[ir].iter().all(|&i| i < surface.vertex_count),
                    "Invalid surface-relative vertex index"
                );
                ensure!(
                    surface.lightmap < 0 || (surface.lightmap as usize) < lightmap_count,
                    "Invalid lightmap index"
                );
                if surface.kind == 2 {
                    ensure!(
                        surface.patch_width >= 3
                            && surface.patch_height >= 3
                            && surface.patch_width <= 65
                            && surface.patch_height <= 65
                            && surface.patch_width % 2 == 1
                            && surface.patch_height % 2 == 1,
                        "Invalid quadratic patch dimensions"
                    );
                    ensure!(
                        surface.patch_width * surface.patch_height == surface.vertex_count,
                        "Patch control point count mismatch"
                    );
                } else if surface.kind == 1 || surface.kind == 3 {
                    ensure!(surface.index_count % 3 == 0, "Incomplete surface triangle");
                }
                Ok(surface)
            })
            .collect::<Result<Vec<_>>>()?;
        let planes = records(lumps[1], 16)?
            .map(|s| {
                let normal = vec_at(s, 0)?;
                let distance = f32_at(s, 12)?;
                let length = normal.length();
                ensure!((length - 1.).abs() < 0.02, "Invalid collision plane normal");
                Ok(Plane {
                    normal: normal / length,
                    distance: distance / length,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let side_planes = records(lumps[10], 8)?
            .map(|s| {
                let plane = count(s, 0)?;
                ensure!(plane < planes.len(), "Invalid brush side plane");
                ensure!(count(s, 4)? < shaders.len(), "Invalid brush side material");
                Ok(plane)
            })
            .collect::<Result<Vec<_>>>()?;
        let brushes = records(lumps[11], 12)?
            .map(|s| {
                let shader = count(s, 8)?;
                ensure!(shader < shaders.len(), "Invalid brush material");
                let sides = bounded(count(s, 0)?, count(s, 4)?, side_planes.len())?;
                ensure!(
                    sides.len() >= 4 && sides.len() <= 256,
                    "Invalid convex brush side count"
                );
                Ok(MapBrush { sides, shader })
            })
            .collect::<Result<Vec<_>>>()?;
        let models = records(lumps[13], 40)?
            .map(|m| {
                let min = vec_at(m, 0)?;
                let max = vec_at(m, 12)?;
                ensure!(min.cmple(max).all(), "Inverted model bounds");
                Ok(Model {
                    min,
                    max,
                    surfaces: bounded(count(m, 24)?, count(m, 28)?, surfaces.len())?,
                    brushes: bounded(count(m, 32)?, count(m, 36)?, brushes.len())?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let world = models.first().context("Missing world model")?;
        let world_surfaces = world.surfaces.clone();
        let world_brushes = world.brushes.clone();
        let world_min = world.min;
        let world_max = world.max;
        let fogs = records(lumps[12], 72)?
            .map(|f| {
                let end = f[..64].iter().position(|&v| v == 0).unwrap_or(64);
                let brush = count(f, 64)?;
                ensure!(brush < brushes.len(), "Invalid fog brush");
                let side = i32_at(f, 68)?;
                ensure!(
                    side == -1 || (side >= 0 && (side as usize) < brushes[brush].sides.len()),
                    "Invalid fog side"
                );
                Ok(Fog {
                    shader: std::str::from_utf8(&f[..end])?.to_ascii_lowercase(),
                    brush,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let entity_text = std::str::from_utf8(lumps[14])?.trim_end_matches('\0');
        let entities = parse_entities(entity_text)?;
        let visibility = crate::visibility::Visibility::parse(
            lumps[9],
            lumps[8],
            lumps[7],
            lumps[15],
            planes.len(),
            surfaces.len(),
        )?;
        Ok(Self {
            visibility,
            difficulty: Default::default(),
            models,
            fogs,
            shaders,
            vertices,
            indices,
            surfaces,
            lightmaps: lumps[2].to_vec(),
            entities,
            world_surfaces,
            planes,
            side_planes,
            brushes,
            world_brushes,
            world_min,
            world_max,
        })
    }

    pub fn spawn(&self) -> (Vec3, f32) {
        let entity = self
            .entities
            .iter()
            .filter(|e| e.get("classname").is_some_and(|c| c == "info_player_start"))
            .min_by_key(|e| e.get("targetname").map(String::as_str).unwrap_or(""));
        if let Some(e) = entity {
            if let Some(origin) = e.get("origin") {
                let c = origin
                    .split_whitespace()
                    .filter_map(|s| s.parse::<f32>().ok())
                    .collect::<Vec<_>>();
                if c.len() == 3 && c.iter().all(|v| v.is_finite()) {
                    let yaw = e
                        .get("angle")
                        .and_then(|s| s.parse::<f32>().ok())
                        .filter(|v| v.is_finite())
                        .unwrap_or(0.)
                        .to_radians();
                    return (vec3(c[0], c[1], c[2] + 48.), yaw);
                }
            }
        }
        (
            self.vertices
                .first()
                .map(|v| v.position + vec3(0., 0., 64.))
                .unwrap_or(Vec3::ZERO),
            0.,
        )
    }

    /// Convert quadratic control grids to explicit triangles; ordinary faces use relative indices.
    pub fn triangulate(&self, surface: &Surface) -> (Vec<MapVertex>, Vec<u16>) {
        let verts =
            &self.vertices[surface.first_vertex..surface.first_vertex + surface.vertex_count];
        if surface.kind != 2 {
            return (
                verts.to_vec(),
                self.indices[surface.first_index..surface.first_index + surface.index_count]
                    .iter()
                    .map(|&i| i as u16)
                    .collect(),
            );
        }
        const STEPS: usize = 4;
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for py in (0..surface.patch_height - 2).step_by(2) {
            for px in (0..surface.patch_width - 2).step_by(2) {
                let base = vertices.len() as u16;
                for y in 0..=STEPS {
                    for x in 0..=STEPS {
                        let bu = bernstein(x as f32 / STEPS as f32);
                        let bv = bernstein(y as f32 / STEPS as f32);
                        let mut v = MapVertex {
                            position: Vec3::ZERO,
                            normal: Vec3::ZERO,
                            uv: Vec2::ZERO,
                            light_uv: Vec2::ZERO,
                            color: [255; 4],
                        };
                        let mut color = [0.; 4];
                        for j in 0..3 {
                            for i in 0..3 {
                                let c = &verts[(py + j) * surface.patch_width + px + i];
                                let w = bu[i] * bv[j];
                                v.position += c.position * w;
                                v.normal += c.normal * w;
                                v.uv += c.uv * w;
                                v.light_uv += c.light_uv * w;
                                for (k, value) in color.iter_mut().enumerate() {
                                    *value += c.color[k] as f32 * w;
                                }
                            }
                        }
                        v.color = color.map(|c| c.clamp(0., 255.) as u8);
                        v.normal = v.normal.normalize_or_zero();
                        vertices.push(v);
                    }
                }
                for y in 0..STEPS {
                    for x in 0..STEPS {
                        let a = base + (y * (STEPS + 1) + x) as u16;
                        let c = a + (STEPS + 1) as u16;
                        indices.extend_from_slice(&[a, c, a + 1, a + 1, c, c + 1]);
                    }
                }
            }
        }
        (vertices, indices)
    }
}

fn bernstein(t: f32) -> [f32; 3] {
    [(1. - t) * (1. - t), 2. * t * (1. - t), t * t]
}

/// Small lexer shared by entity and material files. Handles comments and quoted values.
pub fn tokens(s: &str) -> Result<Vec<String>> {
    let mut chars = s.chars().peekable();
    let mut out = Vec::new();
    while let Some(c) = chars.next() {
        if c.is_whitespace() || c == '\0' {
            continue;
        }
        if c == '/' && chars.peek() == Some(&'/') {
            for c in chars.by_ref() {
                if c == '\n' {
                    break;
                }
            }
            continue;
        }
        if c == '{' || c == '}' {
            out.push(c.to_string());
            continue;
        }
        if c == '"' {
            let mut token = String::new();
            let mut closed = false;
            while let Some(c) = chars.next() {
                if c == '"' {
                    closed = true;
                    break;
                }
                if c == '\\' && chars.peek() == Some(&'"') {
                    chars.next();
                    token.push('"');
                } else {
                    token.push(c);
                }
            }
            ensure!(closed, "Unterminated quoted string");
            out.push(token);
        } else {
            let mut token = c.to_string();
            while chars
                .peek()
                .is_some_and(|c| !c.is_whitespace() && *c != '{' && *c != '}')
            {
                token.push(chars.next().unwrap());
            }
            out.push(token);
        }
    }
    Ok(out)
}

fn parse_entities(s: &str) -> Result<Vec<BTreeMap<String, String>>> {
    let t = tokens(s)?;
    let mut i = 0;
    let mut out = Vec::new();
    while i < t.len() {
        ensure!(t[i] == "{", "Expected entity opening brace");
        i += 1;
        let mut entity = BTreeMap::new();
        loop {
            let key = t.get(i).context("Unclosed entity")?;
            if key == "}" {
                i += 1;
                break;
            }
            if key == "{" {
                bail!("Unexpected nested entity");
            }
            let value = t.get(i + 1).context("Missing entity value")?;
            ensure!(value != "}" && value != "{", "Missing entity value");
            entity.insert(key.to_owned(), value.to_owned());
            i += 2;
        }
        out.push(entity);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn empty_map() -> Vec<u8> {
        let mut b = vec![0u8; 212];
        b[..4].copy_from_slice(b"FAKK");
        b[4..8].copy_from_slice(&42i32.to_le_bytes());
        b[12 + 13 * 8..16 + 13 * 8].copy_from_slice(&172i32.to_le_bytes());
        b[16 + 13 * 8..20 + 13 * 8].copy_from_slice(&40i32.to_le_bytes());
        b
    }
    #[test]
    fn valid_empty_map() {
        assert!(Bsp::parse(&empty_map()).is_ok());
    }
    #[test]
    fn validates_inline_models_and_fog_brush_references() {
        let mut b = empty_map();
        b.extend_from_slice(&[0; 40]);
        b[16 + 13 * 8..20 + 13 * 8].copy_from_slice(&80i32.to_le_bytes());
        assert!(Bsp::parse(&b).is_ok());
        b[212 + 28..212 + 32].copy_from_slice(&1i32.to_le_bytes());
        assert!(Bsp::parse(&b).is_err());
        let mut b = empty_map();
        b[12 + 12 * 8..16 + 12 * 8].copy_from_slice(&212i32.to_le_bytes());
        b[16 + 12 * 8..20 + 12 * 8].copy_from_slice(&72i32.to_le_bytes());
        b.extend_from_slice(&[0; 72]);
        assert!(Bsp::parse(&b).is_err());
    }
    #[test]
    fn rejects_truncated_bad_version_negative_and_out_of_bounds() {
        for n in 0..172 {
            assert!(Bsp::parse(&vec![0; n]).is_err());
        }
        let mut b = empty_map();
        b[4] = 41;
        assert!(Bsp::parse(&b).is_err());
        b[4] = 42;
        b[12..16].copy_from_slice(&(-1i32).to_le_bytes());
        assert!(Bsp::parse(&b).is_err());
        b[12..16].copy_from_slice(&10000i32.to_le_bytes());
        assert!(Bsp::parse(&b).is_err());
    }
    #[test]
    fn entity_lexer_handles_comments_quotes_and_rejects_incomplete() {
        let e =
            parse_entities("// test\n{\"origin\" \"1 2 3\" \"classname\" \"info_player_start\"}")
                .unwrap();
        assert_eq!(e[0]["origin"], "1 2 3");
        assert!(parse_entities("{ \"key\" }").is_err());
        assert!(tokens("\"unfinished").is_err());
    }
    #[test]
    fn quadratic_basis_partitions_unity() {
        for t in [0., 0.25, 0.5, 0.75, 1.] {
            assert!((bernstein(t).iter().sum::<f32>() - 1.).abs() < 1e-6);
        }
    }
}
