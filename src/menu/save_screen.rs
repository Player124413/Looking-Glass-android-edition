//! The archive camera's layers and shutter timing, independent of save actions.
use super::*;
use macroquad::miniquad::{
    BlendFactor, BlendState, BlendValue, Equation, PipelineParams, ShaderSource,
};

const VERTEX: &str = r#"#version 100
attribute vec3 position; attribute vec2 texcoord; attribute vec4 color0;
uniform mat4 Model; uniform mat4 Projection;
varying lowp vec4 color; varying mediump vec2 uv;
void main(){gl_Position=Projection*Model*vec4(position,1.0);uv=texcoord;color=color0/255.0;}
"#;
const FRAGMENT: &str = r#"#version 100
precision mediump float;
varying lowp vec4 color; varying mediump vec2 uv;
uniform sampler2D Texture;
void main(){gl_FragColor=texture2D(Texture,uv)*color;}
"#;

#[derive(Default)]
struct Shutter {
    from: usize,
    to: usize,
    start: f64,
    opening: bool,
}
impl Shutter {
    fn sample(&self, now: f64) -> (usize, f32) {
        let t = (now - self.start).max(0.) as f32;
        if self.opening {
            (self.to, (t / 0.18).min(1.))
        } else {
            // loadsave.urc: close in 180 ms, select at 350 ms, reopen at 500 ms.
            let open = if t < 0.18 {
                1. - t / 0.18
            } else {
                ((t - 0.5) / 0.18).clamp(0., 1.)
            };
            (if t < 0.35 { self.from } else { self.to }, open)
        }
    }
    fn select(&mut self, index: usize, now: f64) {
        self.from = self.sample(now).0;
        self.to = index;
        self.start = now;
        self.opening = false;
    }
}

pub(super) struct Film {
    material: Material,
    grain: Vec<Texture2D>,
    shutter: Shutter,
    sounds: [std::sync::Arc<[u8]>; 3],
    playing: Vec<rodio::Sink>,
}
impl Film {
    pub fn load(assets: &mut Assets) -> Result<Self> {
        let material = load_material(
            ShaderSource::Glsl {
                vertex: VERTEX,
                fragment: FRAGMENT,
            },
            MaterialParams {
                pipeline_params: PipelineParams {
                    // scripts/ui.shader, ui/loadgame/sepia1. This is a film
                    // projection over the photo, not an opaque brown rectangle.
                    color_blend: Some(BlendState::new(
                        Equation::Add,
                        BlendFactor::Value(BlendValue::DestinationColor),
                        BlendFactor::Value(BlendValue::SourceAlpha),
                    )),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .map_err(|e| anyhow::anyhow!("Save camera material: {e:?}"))?;
        let grain = (2..=6)
            .map(|i| load_texture(assets, &format!("ui/load/sepia1_{i}.tga"), false))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            material,
            grain,
            shutter: Shutter::default(),
            playing: Vec::new(),
            sounds: [
                assets.read("sound/ui/projector_long.wav")?.into(),
                assets.read("sound/ui/hover_3.wav")?.into(),
                assets.read("sound/ui/click_4.wav")?.into(),
            ],
        })
    }
    pub fn open(&mut self, index: usize, now: f64) {
        self.shutter = Shutter {
            from: index,
            to: index,
            start: now,
            opening: true,
        };
    }
    pub fn select(&mut self, index: usize, now: f64) {
        self.shutter.select(index, now);
    }
    pub fn sound(&mut self, audio: &audio::Audio, index: usize) {
        match audio.menu_sound(self.sounds[index].clone()) {
            Ok(Some(sink)) => self.playing.push(sink),
            Ok(None) => (),
            Err(e) => eprintln!("Menu sound unavailable: {e:#}"),
        }
        // Rapid slot changes never leave an unbounded queue of projector sounds.
        if self.playing.len() > 4 {
            self.playing.remove(0).stop();
        }
    }
    pub fn update_audio(&mut self, settings: audio::Settings, focused: bool) {
        self.playing.retain(|s| !s.empty());
        for sink in &self.playing {
            sink.set_volume(if settings.muted { 0. } else { settings.effects });
            if focused {
                sink.play();
            } else {
                sink.pause();
            }
        }
    }
    pub fn stop(&mut self) {
        for sink in self.playing.drain(..) {
            sink.stop();
        }
    }
    fn project(&self, canvas: Canvas, rect: Rect, now: f64) {
        let tick = (now.max(0.) * 10.) as usize;
        // A stable bounded noise sample gives the original 10 Hz alpha flicker.
        let noise =
            ((tick as u32).wrapping_mul(1664525).wrapping_add(1013904223) >> 16) as f32 / 65535.;
        let r = canvas.rect(rect);
        gl_use_material(&self.material);
        draw_texture_ex(
            &self.grain[tick % self.grain.len()],
            r.x,
            r.y,
            Color::new(1., 1., 1., 0.6 + 0.4 * noise),
            DrawTextureParams {
                dest_size: Some(r.size()),
                ..Default::default()
            },
        );
        gl_use_default_material();
    }
}

/// UTC is explicit so saves remain unambiguous across time-zone changes.
pub(super) fn timestamp(ms: u64) -> String {
    let seconds = ms / 1000;
    let days = (seconds / 86400).min(2_932_896) as i64;
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    let names = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    format!(
        "{day} {} {year}  {:02}:{:02} UTC",
        names[month as usize - 1],
        seconds / 3600 % 24,
        seconds / 60 % 60
    )
}

impl Menu {
    fn save_widget(&self, name: &str) -> &Widget {
        self.layouts["loadsave"]
            .iter()
            .find(|w| w.value("name") == name)
            .expect("validated archive save widget")
    }
    fn shot_rect(&self, index: usize) -> Rect {
        self.save_widget(&format!("LoadSaveShot{}", index + 1)).rect
    }
    pub(super) fn slot_rect(&self, index: usize) -> Rect {
        let shot = self.shot_rect(index);
        let button = self.save_widget(&format!("LoadButton{}", index + 1)).rect;
        Rect::new(shot.x, shot.y, shot.w, button.bottom() - shot.y)
    }
    fn shutter_art(&self, canvas: Canvas, name: &str, offset: f32, clip: Rect) {
        let w = self.save_widget(name);
        let r = Rect::new(w.rect.x + offset, w.rect.y, w.rect.w, w.rect.h);
        let x = r.x.max(clip.x);
        let y = r.y.max(clip.y);
        let width = r.right().min(clip.right()) - x;
        let height = r.bottom().min(clip.bottom()) - y;
        if width <= 0. || height <= 0. {
            return;
        }
        if let Some(t) = self.textures.get(&w.art().to_ascii_lowercase()) {
            let dest = canvas.rect(Rect::new(x, y, width, height));
            draw_texture_ex(
                t,
                dest.x,
                dest.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(dest.size()),
                    source: Some(Rect::new(
                        (x - r.x) / r.w * t.width(),
                        (y - r.y) / r.h * t.height(),
                        width / r.w * t.width(),
                        height / r.h * t.height(),
                    )),
                    ..Default::default()
                },
            );
        }
    }
    pub(super) fn draw_save_underlay(&self, canvas: Canvas, now: f64) {
        for index in 0..6 {
            self.save_preview(canvas, index, self.shot_rect(index));
        }
        let (shown, open) = self.save_film.shutter.sample(now);
        let rect = self.save_widget("BigSaveShot").rect;
        self.save_preview(canvas, shown, rect);
        self.save_film.project(canvas, rect, now);
        self.shutter_art(canvas, "bigcover_left", -190. * open, rect);
        self.shutter_art(canvas, "bigcover_right", 190. * open, rect);
    }
    pub(super) fn draw_save_details(&self, canvas: Canvas, page: Page) {
        for (index, slot) in Slot::ALL.into_iter().enumerate() {
            let b = self.save_widget(&format!("LoadButton{}", index + 1));
            let lit =
                index == self.save_selected || page == Page::LoadSave && index == self.selected;
            self.art(
                canvas,
                if lit { b.value("hovershader") } else { b.art() },
                b.rect,
                if lit {
                    0.9 + (get_time() as f32 * std::f32::consts::PI).sin() * 0.1
                } else {
                    1.
                },
            );
            let x = if index % 2 == 0 {
                b.rect.right() + 3.
            } else {
                self.shot_rect(index).x + 3.
            };
            self.ui.font.draw(
                canvas,
                if slot == Slot::Auto {
                    "Autosave"
                } else {
                    slot.title()
                },
                Rect::new(x, b.rect.y + 3., 80., 22.),
                16.,
                crate::ui::PAPER_TEXT,
            );
        }
        // The six existing slots occupy one page. Keep the original unlit
        // pagination/delete hardware visible without inventing new actions.
        for name in ["PrevButton", "NextButton", "DeleteButton"] {
            let w = self.save_widget(name);
            self.art(canvas, w.art(), w.rect, 0.65);
        }
        let page_num = self.save_widget("PageNum").rect;
        self.ui
            .font
            .draw(canvas, "1", page_num, 18., crate::ui::PAPER_TEXT);
        let shown = self.save_film.shutter.sample(get_time()).0;
        let title = self.save_widget("MapName").rect;
        self.ui
            .font
            .draw(canvas, &self.save_labels[shown], title, 23., crate::ui::INK);
        let status = if page == Page::LoadSave {
            match self.selected {
                6 => "Load Game",
                7 if self.save_selected == 5 => "Automatic save",
                7 => "Save Game",
                _ => &self.save_dates[shown],
            }
        } else {
            &self.save_dates[shown]
        };
        self.ui.font.draw(
            canvas,
            status,
            self.save_widget("DateTime").rect,
            18.,
            crate::ui::INK,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_changes_behind_closed_shutters_and_rapid_clicks_settle() {
        let mut s = Shutter {
            from: 0,
            to: 0,
            start: 0.,
            opening: true,
        };
        s.select(1, 1.);
        assert_eq!(s.sample(1.), (0, 1.));
        assert_eq!(s.sample(1.2), (0, 0.));
        assert_eq!(s.sample(1.4), (1, 0.));
        assert_eq!(s.sample(1.8), (1, 1.));
        s.select(2, 1.45);
        assert_eq!(s.sample(1.45).0, 1);
        assert_eq!(s.sample(2.2), (2, 1.));
    }
    #[test]
    fn save_dates_handle_epoch_and_leap_day() {
        assert_eq!(timestamp(0), "1 Jan 1970  00:00 UTC");
        assert_eq!(timestamp(951827696000), "29 Feb 2000  12:34 UTC");
        assert_eq!(timestamp(1790596800000), "28 Sep 2026  12:00 UTC");
    }
}
