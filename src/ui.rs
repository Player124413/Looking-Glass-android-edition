//! Original archive lettering and artwork shared by menus and gameplay overlays.
use crate::{assets::Assets, bsp, texture};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

pub const INK: Color = Color::new(0.199, 0.043, 0.043, 1.);
pub const SELECTED: Color = Color::new(0.51, 0.11, 0.07, 1.);
pub const PAPER_TEXT: Color = Color::new(0.94, 0.89, 0.78, 1.);

/// Keep gameplay lettering readable above the original reference resolution,
/// while leaving enough horizontal space in small or narrow windows.
pub fn overlay_scale() -> f32 {
    (screen_height() / 900.).max(1.).min(screen_width() / 900.).clamp(0.6, 4.)
}

#[derive(Clone, Copy)]
pub struct Canvas {
    scale: f32,
    offset: Vec2,
}
impl Canvas {
    pub fn new(w: f32, h: f32) -> Self {
        let scale = (w / 640.).min(h / 480.).max(0.001);
        Self {
            scale,
            offset: (vec2(w, h) - vec2(640., 480.) * scale) * 0.5,
        }
    }
    pub fn rect(self, r: Rect) -> Rect {
        Rect::new(
            self.offset.x + r.x * self.scale,
            self.offset.y + r.y * self.scale,
            r.w * self.scale,
            r.h * self.scale,
        )
    }
    pub fn pointer(self, p: Vec2) -> Vec2 {
        (p - self.offset) / self.scale
    }
}

pub struct BitmapFont {
    texture: Texture2D,
    indices: Vec<i32>,
    glyphs: Vec<Rect>,
    height: f32,
}
impl BitmapFont {
    pub fn load(assets: &mut Assets, name: &str) -> Result<Self> {
        let tokens = bsp::tokens(&String::from_utf8_lossy(
            &assets.read(&format!("fonts/{name}.ritualfont"))?,
        ))?;
        let start = tokens
            .iter()
            .position(|s| s == "indirections")
            .context("Font indirections")?
            + 2;
        let end = tokens[start..]
            .iter()
            .position(|s| s == "}")
            .context("Font end")?
            + start;
        let indices = tokens[start..end]
            .iter()
            .map(|s| s.parse())
            .collect::<std::result::Result<Vec<i32>, _>>()?;
        ensure!(indices.len() == 256, "Invalid bitmap font indirections");
        let start = tokens
            .iter()
            .position(|s| s == "locations")
            .context("Font locations")?
            + 2;
        let mut glyphs = Vec::new();
        let mut i = start;
        while tokens.get(i).is_some_and(|s| s == "{") {
            let values = tokens
                .get(i + 1..i + 5)
                .context("Truncated glyph")?
                .iter()
                .map(|s| s.parse::<f32>())
                .collect::<std::result::Result<Vec<_>, _>>()?;
            ensure!(
                values
                    .iter()
                    .all(|n| n.is_finite() && (0. ..=4096.).contains(n)),
                "Invalid glyph"
            );
            glyphs.push(Rect::new(values[0], values[1], values[2], values[3]));
            i += 6;
        }
        ensure!(
            indices
                .iter()
                .all(|&i| i == -1 || (i >= 0 && (i as usize) < glyphs.len())),
            "Invalid glyph reference"
        );
        Ok(Self {
            texture: load_texture(assets, &format!("gfx/2d/fonts/{name}.tga"))?,
            indices,
            glyphs,
            height: tokens
                .get(2)
                .context("Font height")?
                .parse::<f32>()?
                .clamp(1., 128.),
        })
    }
    pub fn glyph(&self, c: char) -> Rect {
        let c = match c {
            '’' | '‘' => '\'',
            '“' | '”' => '"',
            '—' | '–' => '-',
            _ => c,
        };
        let n = c as usize;
        let i = self
            .indices
            .get(n)
            .copied()
            .unwrap_or(self.indices[b'?' as usize]);
        if i < 0 {
            Rect::default()
        } else {
            self.glyphs[i as usize]
        }
    }
    pub fn width(&self, text: &str, size: f32) -> f32 {
        text.chars().map(|c| self.glyph(c).w).sum::<f32>() * size / self.height
    }
    pub fn left(&self, text: &str, x: f32, y: f32, size: f32, color: Color) {
        self.draw(
            Canvas {
                scale: 1.,
                offset: Vec2::ZERO,
            },
            text,
            Rect::new(x, y - size, self.width(text, size), size),
            size,
            color,
        );
    }
    pub fn draw_left(&self, canvas: Canvas, text: &str, mut r: Rect, size: f32, color: Color) {
        r.w = r.w.min(self.width(text, size));
        self.draw(canvas, text, r, size, color);
    }
    pub fn draw(&self, canvas: Canvas, text: &str, r: Rect, size: f32, color: Color) {
        let width: f32 = text.chars().map(|c| self.glyph(c).w).sum();
        let scale = (size / self.height).min(r.w / width.max(1.));
        let mut x = r.x + (r.w - width * scale) * 0.5;
        let y = r.y + (r.h - self.height * scale) * 0.5;
        for c in text.chars() {
            let glyph = self.glyph(c);
            if glyph.w > 0. && glyph.h > 0. {
                let dest = canvas.rect(Rect::new(x, y, glyph.w * scale, glyph.h * scale));
                draw_texture_ex(
                    &self.texture,
                    dest.x,
                    dest.y,
                    color,
                    DrawTextureParams {
                        dest_size: Some(dest.size()),
                        source: Some(glyph),
                        ..Default::default()
                    },
                );
            }
            x += glyph.w * scale;
        }
    }
}

pub fn load_texture(assets: &mut Assets, path: &str) -> Result<Texture2D> {
    let im = texture::decode(assets, path)?;
    let tex = Texture2D::from_rgba8(im.width, im.height, &im.pixels);
    tex.set_filter(FilterMode::Linear);
    Ok(tex)
}

pub struct Ui {
    pub font: BitmapFont,
    pub body: BitmapFont,
    pub courier: BitmapFont,
    left: Texture2D,
    right: Texture2D,
    talk: Texture2D,
    plaque: [Texture2D; 2],
    backdrop: Texture2D,
    pause: Texture2D,
    cursor: Texture2D,
    arrow: Texture2D,
}
thread_local! { static SHARED: RefCell<Weak<Ui>> = const { RefCell::new(Weak::new()) }; }
impl Ui {
    pub fn load(assets: &mut Assets) -> Result<Rc<Self>> {
        if let Some(ui) = SHARED.with(|s| s.borrow().upgrade()) {
            return Ok(ui);
        }
        let ui = Rc::new(Self {
            font: BitmapFont::load(assets, "asrafel")?,
            body: BitmapFont::load(assets, "verdana-12")?,
            courier: BitmapFont::load(assets, "courier-14")?,
            left: load_texture(assets, "ui/dialog/leftframe.tga")?,
            right: load_texture(assets, "ui/dialog/rightframe.tga")?,
            talk: load_texture(assets, "gfx/2d/talkmenu.tga")?,
            plaque: [
                load_texture(assets, "ui/buttons/plaque.tga")?,
                load_texture(assets, "ui/buttons/plaque_hover.tga")?,
            ],
            backdrop: load_texture(assets, "gfx/2d/backtile.tga")?,
            pause: load_texture(assets, "ui/pausewatch/pausewatch.tga")?,
            cursor: load_texture(assets, "gfx/2d/mouse_arrow.tga")?,
            arrow: load_texture(assets, "ui/control/hslider_next.tga")?,
        });
        SHARED.with(|s| *s.borrow_mut() = Rc::downgrade(&ui));
        Ok(ui)
    }
    pub fn panel(&self, r: Rect) {
        nine(&self.left, Rect::new(r.x, r.y, r.w * 0.5, r.h), 16.);
        nine(
            &self.right,
            Rect::new(r.x + r.w * 0.5, r.y, r.w * 0.5, r.h),
            16.,
        );
    }
    pub fn dialog(&self, r: Rect) {
        nine(&self.talk, r, 16.);
    }
    pub fn plaque(&self, r: Rect, selected: bool) {
        nine(&self.plaque[selected as usize], r, 16.);
    }
    pub fn tile(&self, r: Rect) {
        let size = 96.;
        for y in 0..(r.h / size).ceil() as usize {
            for x in 0..(r.w / size).ceil() as usize {
                let w = (r.w - x as f32 * size).min(size);
                let h = (r.h - y as f32 * size).min(size);
                draw_texture_ex(
                    &self.backdrop,
                    r.x + x as f32 * size,
                    r.y + y as f32 * size,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(w, h)),
                        source: Some(Rect::new(0., 0., w / size * 64., h / size * 64.)),
                        ..Default::default()
                    },
                );
            }
        }
    }
    pub fn center(&self, text: &str, r: Rect, size: f32, color: Color) {
        self.font.draw(
            Canvas {
                scale: 1.,
                offset: Vec2::ZERO,
            },
            text,
            r,
            size,
            color,
        );
    }
    pub fn label(&self, text: &str, x: f32, y: f32, size: f32, color: Color) {
        self.font.left(text, x + 1., y + 1., size, BLACK);
        self.font.left(text, x, y, size, color);
    }
    pub fn fit_label(&self, text: &str, x: f32, y: f32, size: f32, width: f32, color: Color) {
        self.label(
            text,
            x,
            y,
            size * (width / self.font.width(text, size).max(1.)).min(1.),
            color,
        );
    }
    pub fn toast(&self, text: &str, y: f32) {
        let s = overlay_scale();
        let size = 22. * s;
        let w = (self.font.width(text, size) + 56. * s)
            .min(760. * s).min(screen_width() - 32. * s);
        let mut rows = crate::story::wrap(text, w - 44. * s, |t| self.font.width(t, size));
        if rows.len() > 4 {
            rows.truncate(4);
            let last = rows.last_mut().unwrap();
            while self.font.width(&format!("{last}..."), size) > w - 44. * s && !last.is_empty() {
                last.pop();
            }
            last.push_str("...");
        }
        let height = (18. + 26. * rows.len().max(1) as f32) * s;
        let y = y.clamp(12. * s, (screen_height() - height - 12. * s).max(12. * s));
        let r = Rect::new((screen_width() - w) * 0.5, y, w, height);
        self.dialog(r);
        for (i, row) in rows.iter().enumerate() {
            self.center(row, Rect::new(r.x + 22. * s, r.y + (8. + i as f32 * 26.) * s,
                r.w - 44. * s, 26. * s), size, WHITE);
        }
    }
    pub fn paused(&self, prompt: &str) {
        let s = (screen_height() / 600.).clamp(0.7, 1.5 * overlay_scale());
        draw_texture_ex(
            &self.pause,
            screen_width() * 0.5 - 48. * s,
            22. * s,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(96., 96.) * s),
                ..Default::default()
            },
        );
        self.center(
            "Paused",
            Rect::new(screen_width() * 0.5 - 80. * s, 126. * s, 160. * s, 28. * s),
            26. * s,
            PAPER_TEXT,
        );
        self.center(
            prompt,
            Rect::new(screen_width() * 0.5 - 180. * s, 158. * s, 360. * s, 24. * s),
            20. * s,
            PAPER_TEXT,
        );
    }
    pub fn marker(&self, r: Rect) {
        draw_texture_ex(
            &self.arrow,
            r.x,
            r.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(r.size()),
                ..Default::default()
            },
        );
    }
    pub fn cursor(&self) {
        let p = mouse_position();
        let scale = (screen_height() / 600.).clamp(0.75, 2.);
        draw_texture_ex(
            &self.cursor,
            p.0,
            p.1,
            WHITE,
            DrawTextureParams {
                dest_size: Some(
                    vec2(24., 24. * self.cursor.height() / self.cursor.width()) * scale,
                ),
                ..Default::default()
            },
        );
    }
}
fn nine(t: &Texture2D, r: Rect, b: f32) {
    let dx = b.min(r.w * 0.25);
    let dy = b.min(r.h * 0.25);
    let sx = [0., b, t.width() - b, t.width()];
    let sy = [0., b, t.height() - b, t.height()];
    let x = [r.x, r.x + dx, r.right() - dx, r.right()];
    let y = [r.y, r.y + dy, r.bottom() - dy, r.bottom()];
    for row in 0..3 {
        for col in 0..3 {
            draw_texture_ex(
                t,
                x[col],
                y[row],
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(x[col + 1] - x[col], y[row + 1] - y[row])),
                    source: Some(Rect::new(
                        sx[col],
                        sy[row],
                        sx[col + 1] - sx[col],
                        sy[row + 1] - sy[row],
                    )),
                    ..Default::default()
                },
            );
        }
    }
}
