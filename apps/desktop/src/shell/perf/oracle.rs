//! The drawing area's truth check (plan/render.md RM-02, docs/adr/9202): what the area shows with its shortcuts
//! against what it shows drawn afresh. The shortcuts are pictures kept, moved or scaled from an earlier frame and
//! labels kept while the view moves.
//!
//! Two apps open the same file; one draws every frame afresh (`Viewport::fresh`). Both take the same input: the
//! pointer, the wheel a notch at a time and in bursts, a middle-button pan, show everything, the same again in a
//! turned view. After each frame their drawing areas are compared pixel by pixel. A pixel matches when the other
//! picture has its colour within one pixel of it, since a pan's picture moves by whole pixels (docs/adr/1067). A frame
//! whose pictures still differ shows something a fresh frame would not: a picture that stayed when it should not
//! have. Up to `SAVED` such frames are saved with the fresh one and their difference to
//! `.run/gui-evidence/render/rm02/<file>/`.
//!
//! `KENTOS_PERF_FILES` as `perf::files` reads it. Not run by default:
//!
//! ```text
//! KENTOS_PERF_FILES=/yol/plan.NCZ cargo test --release -p kentos-desktop --lib perf::oracle::oracle -- \
//!   --ignored --nocapture --test-threads=1
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use iced::{Event, Point, mouse};
use kentos_ui::snapshot::Image;
use serde_json::{Value, json};

use super::frame::{Harness, WINDOW, moved};
use super::large::{open, wheel};
use crate::app::{App, Message};

/// A channel apart by more than this is another colour.
const CHANNEL: u8 = 40;
/// A frame differs when more than this share of the area's pixels have no match in the other picture.
const SHARE: f64 = 0.001;
/// Frames whose pictures differ that are saved, per file.
const SAVED: usize = 12;

/// The two apps, the shortcut one first, each with its own window without a window.
struct Twins {
    apps: [App; 2],
    harnesses: [Harness; 2],
    out: PathBuf,
    frames: usize,
    differing: Vec<Value>,
    /// The shortcut app's last frame as its view reported it, before the pictures were read back (a read back draws
    /// the frame again, and that frame keeps its picture).
    stats: kentos_render_wgpu::FrameStats,
    moving: bool,
}

/// One pixel's match: the other picture has a colour within `CHANNEL` of it within one pixel.
fn matched(a: &[u8], b: &[u8], width: usize, height: usize, x: usize, y: usize) -> bool {
    let at = |x: usize, y: usize| (y * width + x) * 4;
    let p = &a[at(x, y)..at(x, y) + 3];
    for dy in y.saturating_sub(1)..=(y + 1).min(height - 1) {
        for dx in x.saturating_sub(1)..=(x + 1).min(width - 1) {
            let q = &b[at(dx, dy)..at(dx, dy) + 3];
            if p.iter().zip(q).all(|(u, v)| u.abs_diff(*v) <= CHANNEL) {
                return true;
            }
        }
    }
    false
}

impl Twins {
    fn open(file: &Path) -> Self {
        let mut apps = [App::boot(None).0, App::boot(None).0];
        let mut harnesses = [Harness::new(), Harness::new()];
        apps[1].viewport.fresh = true;
        for (app, h) in apps.iter_mut().zip(harnesses.iter_mut()) {
            h.settle(app);
            open(app, h, file);
        }
        let name = file.file_stem().map_or_else(|| "cizim".into(), |n| n.to_string_lossy().into_owned());
        let out = root().join(".run/gui-evidence/render/rm02").join(name);
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        Self {
            apps,
            harnesses,
            out,
            frames: 0,
            differing: Vec::new(),
            stats: Default::default(),
            moving: false,
        }
    }

    /// One input to both, then their drawing areas compared.
    fn frame(&mut self, gesture: &str, events: &[Event]) {
        self.frame_with(gesture, events, |_| {});
    }

    fn frame_with(&mut self, gesture: &str, events: &[Event], act: impl Fn(&mut App)) {
        for (app, h) in self.apps.iter_mut().zip(self.harnesses.iter_mut()) {
            h.frame_with(app, events, &act);
        }
        self.stats = self.apps[0].viewport.status().stats;
        self.moving = self.apps[0].viewport.moving();
        self.compare(gesture);
    }

    fn compare(&mut self, gesture: &str) {
        self.frames += 1;
        let [a, b] = [0, 1].map(|i| self.harnesses[i].picture(&self.apps[i]));
        if a.is_empty() || a.len() != b.len() {
            return;
        }
        let (width, height) = (WINDOW.width as usize, WINDOW.height as usize);
        let area = self.apps[0].viewport.bounds;
        let (x0, y0) = (area.x.max(0.0) as usize, area.y.max(0.0) as usize);
        let (x1, y1) = (((area.x + area.width) as usize).min(width), ((area.y + area.height) as usize).min(height));
        let mut diff = vec![0u8; a.len()];
        let mut unmatched = 0usize;
        let (mut lo, mut hi) = ([usize::MAX; 2], [0usize; 2]);
        for y in y0..y1 {
            for x in x0..x1 {
                let i = (y * width + x) * 4;
                let bad = !matched(&a, &b, width, height, x, y) || !matched(&b, &a, width, height, x, y);
                diff[i..i + 4].copy_from_slice(if bad { &[220, 30, 30, 255] } else { &[0, 0, 0, 0] });
                if !bad {
                    // The shortcut picture, faint, so the difference is seen where it is.
                    for c in 0..3 {
                        diff[i + c] = 255 - (255 - a[i + c]) / 4;
                    }
                    diff[i + 3] = 255;
                    continue;
                }
                unmatched += 1;
                lo = [lo[0].min(x), lo[1].min(y)];
                hi = [hi[0].max(x), hi[1].max(y)];
            }
        }
        let pixels = (x1 - x0) * (y1 - y0);
        let share = unmatched as f64 / pixels.max(1) as f64;
        if share <= SHARE {
            return;
        }
        let stats = self.stats;
        let n = self.differing.len();
        let record = json!({
            "frame": self.frames, "gesture": gesture, "unmatchedPixels": unmatched, "share": share,
            "box": [lo[0], lo[1], hi[0], hi[1]],
            "pictureKept": stats.picture_kept, "pictureMoved": stats.picture_moved,
            "pictureZoomed": stats.picture_zoomed, "moving": self.moving,
            "scale": self.apps[0].viewport.camera.scale,
        });
        println!("FARK {record}");
        if n < SAVED || gesture.ends_with("dinlenme") {
            let save = |rgba: Vec<u8>, name: &str| {
                let image = Image { width: width as u32, height: height as u32, rgba };
                let _ = image.save(self.out.join(format!("{n:02}-{gesture}-{name}.png")));
            };
            save(a, "kisayollu");
            save(b, "taze");
            save(diff, "fark");
        }
        self.differing.push(record);
    }

    /// Both views turned by `radians` (a twisted view, docs/adr/0285 §2).
    fn twist(&mut self, radians: f64) {
        for app in &mut self.apps {
            app.viewport.camera.twist = radians;
        }
        self.frame("dondurme", &[]);
    }
}

/// The input both apps take: the pointer across the area, the wheel a notch at a time with settling, bursts in and
/// out, a pan out and back, show everything; then the same turned.
fn play(t: &mut Twins) {
    let area = t.apps[0].viewport.bounds;
    let center = area.center();
    for round in 0..2 {
        let at = |dx: f32, dy: f32| Point::new(center.x + dx * area.width, center.y + dy * area.height);
        for k in 0..12 {
            let p = at(-0.3 + 0.05 * k as f32, -0.2 + 0.03 * k as f32);
            t.frame("imlec", &[moved(p)]);
        }
        t.frame("imlec", &[moved(center)]);
        for lines in [1.0, -1.0] {
            for _ in 0..8 {
                t.frame("tekerlek", &[wheel(lines)]);
            }
            std::thread::sleep(Duration::from_millis(200));
            // The live gesture settles in this frame's redraw (viewport/live.rs); the next one shows the view built
            // for where it stopped.
            t.frame("tekerlek-dinleniyor", &[]);
            t.frame("tekerlek-dinlenme", &[]);
        }
        for lines in [1.0, -1.0] {
            let burst: Vec<Event> = (0..14).map(|_| wheel(lines)).collect();
            t.frame("patlama", &burst);
            for _ in 0..3 {
                t.frame("patlama-sonrasi", &[]);
            }
            std::thread::sleep(Duration::from_millis(200));
            t.frame("patlama-dinleniyor", &[]);
            t.frame("patlama-dinlenme", &[]);
        }
        pan(t, center);
        t.frame_with("tumu", &[], |app| {
            let _ = app.update(Message::Viewport(crate::viewport::Event::Extents));
        });
        t.frame("tumu", &[]);
        if round == 0 {
            t.twist(0.4);
        }
    }
    t.twist(0.0);
}

/// A middle-button pan from `center`, 20 steps out and back, as `frame::pan` does.
fn pan(t: &mut Twins, center: Point) {
    t.frame("kaydirma", &[moved(center), Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle))]);
    for k in 1..=20 {
        let d = 12.0 * k as f32;
        t.frame("kaydirma", &[moved(Point::new(center.x + d, center.y + d * 0.5))]);
    }
    for k in (0..20).rev() {
        let d = 12.0 * k as f32;
        t.frame("kaydirma", &[moved(Point::new(center.x + d, center.y + d * 0.5))]);
    }
    t.frame("kaydirma", &[Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Middle))]);
    std::thread::sleep(Duration::from_millis(200));
    t.frame("kaydirma-dinleniyor", &[]);
    t.frame("kaydirma-dinlenme", &[]);
}

fn check(file: &Path) -> Value {
    let mut twins = Twins::open(file);
    play(&mut twins);
    json!({
        "file": file.file_name().map(|n| n.to_string_lossy().into_owned()),
        "frames": twins.frames, "differing": twins.differing.len(), "frames_differing": twins.differing,
        "pictures": twins.out.display().to_string(),
    })
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// One file in this process (`KENTOS_ORACLE_CHILD`); prints one line of JSON.
#[test]
#[ignore = "run by perf::oracle::oracle in a process of its own"]
fn child() {
    let Ok(file) = std::env::var("KENTOS_ORACLE_CHILD") else {
        return;
    };
    println!("KENTOS_ORACLE {}", check(Path::new(&file)));
}

#[test]
#[ignore = "a check run by hand in release mode"]
fn oracle() {
    let files: Vec<String> = std::env::var("KENTOS_PERF_FILES")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|f| !f.is_empty())
        .map(str::to_owned)
        .collect();
    assert!(!files.is_empty(), "KENTOS_PERF_FILES names the drawings to check");
    let mut rows = Vec::new();
    for file in &files {
        let output = Command::new(std::env::current_exe().expect("the test binary"))
            .args(["perf::oracle::child", "--exact", "--ignored", "--nocapture"])
            .env("KENTOS_ORACLE_CHILD", file)
            .output()
            .expect("runs");
        let text = String::from_utf8_lossy(&output.stdout);
        let row = match text.lines().find_map(|l| l.strip_prefix("KENTOS_ORACLE ")) {
            Some(line) => serde_json::from_str(line).expect("JSON"),
            None => json!({
                "file": file, "failed": true,
                "stderrTail": String::from_utf8_lossy(&output.stderr).lines().rev().take(20).collect::<Vec<_>>(),
            }),
        };
        println!("{file}: {} kare, {} farklı", row["frames"], row["differing"]);
        rows.push(row);
    }
    let report = root().join(".run/gui-evidence/render/rm02/denetci.json");
    std::fs::write(&report, format!("{}\n", serde_json::to_string_pretty(&json!({ "rows": rows })).expect("JSON")))
        .expect("writes the report");
    println!("Yazıldı: {}", report.display());
}
