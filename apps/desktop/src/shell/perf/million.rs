//! Where the 10⁶ mixed drawing's slow frames go (docs/adr/1066 §1): the drawing of `perf::large` (`KENTOS_PERF_SIZES`,
//! 1 000 000; its file in `.run/perf/`, made by `perf::large::large`) opened through the app's own open, then one
//! step repeated `KENTOS_PROFILE_ROUNDS` times (5) so a sampling profiler (`sample <pid>`) sees it:
//!
//! - `open`: the drawing opened again (the first frame's phases);
//! - `select`: select all, then Esc;
//! - `pan`: the far view panned (the GPU's frames; per layer, `perf::large::layers` with `KENTOS_PERF_SIZES`).
//!
//! Each frame's parts are printed. A diagnosis, not a correctness test and not run by default:
//!
//! ```text
//! KENTOS_PROFILE=select cargo test --release -p kentos-desktop --lib perf::million -- --ignored --nocapture
//! ```

use iced::keyboard::key;

use super::frame::{Harness, key, pan, percentile};
use super::large::{file_of, open, shortcuts_from_env};
use crate::app::App;

fn rounds() -> usize {
    std::env::var("KENTOS_PROFILE_ROUNDS").ok().and_then(|v| v.parse().ok()).unwrap_or(5)
}

#[test]
#[ignore = "a diagnosis, run by hand in release mode"]
fn million() {
    let n: usize = std::env::var("KENTOS_PERF_SIZES")
        .ok()
        .and_then(|v| v.split(',').next().and_then(|s| s.trim().parse().ok()))
        .unwrap_or(1_000_000);
    let file = file_of(n);
    assert!(file.exists(), "{}: made by perf::large::large", file.display());
    let what = std::env::var("KENTOS_PROFILE").unwrap_or_else(|_| "select".into());
    let (mut app, _) = App::boot(None);
    shortcuts_from_env(&mut app);
    let mut h = Harness::new();
    h.settle(&mut app);
    let opened = open(&mut app, &mut h, &file);
    println!("{n} açıldı: {opened}");
    let fitted = app.viewport.camera;
    for round in 0..rounds() {
        match what.as_str() {
            "open" => {
                let o = open(&mut app, &mut h, &file);
                println!("{n} açılış {round}: {o}");
            }
            "pan" => {
                app.viewport.camera = fitted;
                h.frame(&mut app, &[]);
                let center = app.viewport.bounds.center();
                let frames = pan(&mut app, &mut h, center);
                let gpu: Vec<f64> = frames.iter().map(|p| p.gpu_ms()).collect();
                println!("{n} kaydırma {round}: GPU p50 {:.1} ms", percentile(&gpu, 0.5));
            }
            _ => {
                let p = h.frame_with(&mut app, &[], |app| {
                    let _ = app.run("edit.selectAll");
                });
                let [ev, ap, vw, ly, dr] = p.split();
                println!(
                    "{n} hepsini seç {round}: MİB {:.1} ms (olay {ev:.1}, uygulama {ap:.1}, görünüm {vw:.1}, düzen {ly:.1}, çizim {dr:.1}), GPU {:.1} ms",
                    p.cpu(),
                    p.gpu_ms()
                );
                let next = h.frame(&mut app, &[]);
                println!("{n} sonraki kare: MİB {:.1} ms, GPU {:.1} ms", next.cpu(), next.gpu_ms());
                let esc = h.frame(&mut app, &key(key::Named::Escape));
                println!("{n} Esc: MİB {:.1} ms, GPU {:.1} ms", esc.cpu(), esc.gpu_ms());
            }
        }
    }
}
