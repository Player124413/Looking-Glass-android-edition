//! Image-backed light colours sampled on the saved presentation clock.
use crate::{assets::Assets, texture};
use anyhow::Result;
use macroquad::prelude::*;

#[derive(Clone)]
pub(super) struct Style(Vec<Vec3>);
impl Style {
    pub(super) fn load(assets: &mut Assets, name: &str) -> Result<Option<Self>> {
        // Text styles and scene-owned lights need their owning actor clock.
        if !name.ends_with(".tga") || !assets.contains(name) {
            return Ok(None);
        }
        let image = texture::decode(assets, name)?;
        Ok(Some(Self::from_image(&image)))
    }
    fn from_image(image: &texture::RgbaImage) -> Self {
        Self(
            image
                .pixels
                .chunks_exact(4)
                .take(usize::from(image.width).min(128))
                .map(|p| vec3(p[0] as f32, p[1] as f32, p[2] as f32) / 255.)
                .collect(),
        )
    }
    pub(super) fn sample(&self, clock: f32) -> Vec3 {
        if self.0.is_empty() || !clock.is_finite() {
            return Vec3::ONE;
        }
        let frame = (f64::from(clock.max(0.)) * 20.).rem_euclid(self.0.len() as f64);
        let a = frame.floor() as usize;
        // Native styles hold the final pixel for one tick before wrapping.
        self.0[a].lerp(self.0[(a + 1).min(self.0.len() - 1)], frame.fract() as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn image_style_uses_first_row_rgb_twenty_hz_and_last_pixel_hold() {
        let style = Style::from_image(&texture::RgbaImage {
            width: 2,
            height: 2,
            pixels: vec![
                0, 64, 0, 255, 200, 128, 0, 0, 255, 0, 255, 255, 255, 0, 255, 255,
            ],
        });
        assert_eq!(style.sample(0.), vec3(0., 64., 0.) / 255.);
        assert!((style.sample(0.025) - vec3(100., 96., 0.) / 255.).length() < 0.00001);
        assert_eq!(style.sample(0.075), vec3(200., 128., 0.) / 255.);
        assert!((style.sample(0.1) - style.sample(0.)).length() < 0.00001);
        let saved = 71.125;
        assert_eq!(style.sample(saved), style.sample(saved));
        assert_eq!(style.sample(f32::NAN), Vec3::ONE);
    }
}
