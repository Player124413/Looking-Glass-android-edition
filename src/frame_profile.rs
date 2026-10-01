//! Optional local CPU profiling; inactive during ordinary play.
use std::{cell::RefCell, collections::BTreeMap, time::Instant};
thread_local! { static SAMPLES: RefCell<BTreeMap<&'static str, Vec<f64>>> = RefCell::default(); }
fn enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("LOOKING_GLASS_FRAME_PROFILE").is_some())
}
pub struct Span { label: &'static str, start: Option<Instant> }
pub fn span(label: &'static str) -> Span { Span { label, start: enabled().then(Instant::now) } }
impl Span { pub fn cancel(&mut self) { self.start = None; } }
impl Drop for Span {
    fn drop(&mut self) {
        if let Some(start) = self.start {
            let ms = start.elapsed().as_secs_f64() * 1000.;
            SAMPLES.with(|s| { let mut s=s.borrow_mut(); let v=s.entry(self.label).or_default(); if v.len()<10000 {v.push(ms);} });
        }
    }
}
pub fn report(map: &str) -> anyhow::Result<()> {
    if !enabled() { return Ok(()); }
    let report = SAMPLES.with(|s| s.borrow().iter().map(|(name,v)| {
        let mut samples = v.iter().skip(30).copied().collect::<Vec<_>>();
        samples.sort_by(f64::total_cmp);
        let n=samples.len();
        (*name, serde_json::json!({"samples":n,"median_ms":samples.get(n/2),"p95_ms":samples.get(n*95/100)}))
    }).collect::<BTreeMap<_,_>>());
    let dir = std::path::PathBuf::from(std::env::var_os("LOOKING_GLASS_FRAME_PROFILE").unwrap());
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(format!("{map}.json")), serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}
