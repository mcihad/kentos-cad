//! The drawing area's visual conformance (plan/render.md RM-16, docs/adr/9224), the core's test kit (docs/adr/9199
//! §4.11): what the area shows right after an edit, the camera untouched, against what an app that opens the edited
//! drawing afresh shows with the same camera.
//!
//! The owner, 24 November 2026: a window or a door put on a wall did not show until the view was zoomed or moved
//! (docs/adr/9221). A case is a user's edit made with the tool a user takes, on a drawing whose pictures were already
//! drawn, so that every kept part (the styled cache, the deep view's, the glyph layers, the highlight) has to follow
//! the edit. The twin app opens the drawing as the edit left it and draws everything afresh. Their drawing areas are
//! compared pixel by pixel; a pixel matches when the other picture has its colour (each channel within [`CHANNEL`])
//! within one pixel of it.
//!
//! The pictures are the window's own (`Renderer::screenshot` through [`Harness`]), so a GPU is needed: the cases run
//! with `KENTOS_GPU_TESTS=1`, as the renderer's own GPU tests do, and are skipped otherwise. A case whose pictures
//! differ keeps both and their difference in `.run/gui-evidence/render/rm16/` (with `KENTOS_RM16_KEEP=1`, every case).
//!
//! Test code only.
#![cfg_attr(not(test), allow(dead_code))]

use iced::keyboard::key::Named;

use crate::app::{App, Message};
use crate::document::Document;
use crate::tools_scenes::hover;
use crate::window_testing::{self, Harness, WINDOW};

/// A channel apart by more than this is another colour.
pub const CHANNEL: u8 = 40;
/// Pictures drawn until two in a row are the same, at most this many.
const FRAMES: usize = 8;

/// Whether the GPU cases run (`KENTOS_GPU_TESTS=1`).
pub fn on_gpu() -> bool {
    std::env::var_os("KENTOS_GPU_TESTS").is_some()
}

/// A booted app with its window without a window, laid out.
pub fn booted() -> (App, Harness) {
    let (mut app, _) = App::boot(None);
    let mut h = Harness::new();
    assert!(
        h.on_gpu(),
        "KENTOS_GPU_TESTS=1 asks for a GPU (wgpu); this is no pass without one"
    );
    h.settle(&mut app);
    (app, h)
}

/// The window's picture once it holds still: frames until two pictures in a row are the same.
pub fn still(app: &mut App, h: &mut Harness) -> Vec<u8> {
    h.settle(app);
    let mut last = Vec::new();
    for _ in 0..FRAMES {
        h.frame(app, &[]);
        let picture = h.picture(app);
        if picture == last {
            break;
        }
        last = picture;
    }
    last
}

/// The tool left and nothing selected, as a user's two Escapes leave it, and the pointer at `away` (world, east and
/// north from the drawing's origin of `tools_scenes`).
pub fn quiet(app: &mut App, h: &mut Harness, away: [f64; 2]) {
    for _ in 0..2 {
        h.frame(app, &window_testing::key(Named::Escape));
    }
    hover(app, away);
}

/// Where two pictures differ inside a region: pixels compared, pixels not the same, pixels with no match within one
/// pixel.
#[derive(Debug, Clone, Copy, Default)]
pub struct Difference {
    pub compared: u64,
    pub different: u64,
    pub unmatched: u64,
}

/// One pixel's match: the other picture has a colour within [`CHANNEL`] of it within one pixel.
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

/// The drawing area of `app` in whole pixels of the window, two pixels in from its edges: `[x0, y0, x1, y1]`.
pub fn area(app: &App) -> [usize; 4] {
    let b = app.viewport.bounds;
    [
        (b.x + 2.0).max(0.0) as usize,
        (b.y + 2.0).max(0.0) as usize,
        ((b.x + b.width - 2.0).max(0.0) as usize).min(WINDOW.width as usize),
        ((b.y + b.height - 2.0).max(0.0) as usize).min(WINDOW.height as usize),
    ]
}

/// Two window pictures compared over the region `r`.
pub fn compare(a: &[u8], b: &[u8], r: [usize; 4]) -> Difference {
    let (w, h) = (WINDOW.width as usize, WINDOW.height as usize);
    let mut d = Difference::default();
    for y in r[1]..r[3] {
        for x in r[0]..r[2] {
            let i = (y * w + x) * 4;
            d.compared += 1;
            if a[i..i + 3] != b[i..i + 3] {
                d.different += 1;
                if !matched(a, b, w, h, x, y) || !matched(b, a, w, h, x, y) {
                    d.unmatched += 1;
                }
            }
        }
    }
    d
}

/// Two pictures and where they differ inside `r` (white: no match within a pixel, grey: not the same), kept as
/// `<name>-<first>.png`, `<name>-<second>.png` and `<name>-fark.png` in `.run/gui-evidence/render/<folder>/` (never
/// committed).
pub fn keep(folder: &str, name: &str, pictures: [(&str, &[u8]); 2], r: [usize; 4]) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.run/gui-evidence/render")
        .join(folder);
    let _ = std::fs::create_dir_all(&dir);
    let (w, h) = (WINDOW.width as usize, WINDOW.height as usize);
    let [(_, a), (_, b)] = pictures;
    let mut diff = vec![0u8; a.len()];
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            diff[i + 3] = 255;
            let inside = x >= r[0] && x < r[2] && y >= r[1] && y < r[3];
            if !inside || a[i..i + 3] == b[i..i + 3] {
                continue;
            }
            let v = if matched(a, b, w, h, x, y) && matched(b, a, w, h, x, y) {
                96
            } else {
                255
            };
            diff[i..i + 3].fill(v);
        }
    }
    for (suffix, rgba) in [pictures[0], pictures[1], ("fark", &diff[..])] {
        let image = kentos_ui::snapshot::Image {
            width: w as u32,
            height: h as u32,
            rgba: rgba.to_vec(),
        };
        let file = dir.join(format!("{name}-{suffix}.png"));
        if image.save(&file).is_ok() {
            println!("resim {}", file.display());
        }
    }
}

/// One edit, checked: the drawing `start` opens, framed by `view` once the area has its size, then what `drawn`
/// draws with the tools (the area laid out, so the clicks land where they are meant), its pictures drawn; then
/// `edit`, the user's (an edit of the drawing, or a move of the camera: a zoom); then the picture right after it (the
/// tool left, nothing selected, the pointer at `away`) against an app that opens the edited drawing afresh with the
/// same camera.
pub struct Edit<'a> {
    pub name: &'a str,
    pub start: &'a dyn Fn(&mut App),
    pub view: &'a dyn Fn(&mut App),
    pub drawn: &'a dyn Fn(&mut App),
    pub edit: &'a dyn Fn(&mut App),
    pub away: [f64; 2],
}

/// Nothing drawn before the edit.
pub fn nothing(_: &mut App) {}

impl Edit<'_> {
    /// `None` when the pictures match, what differs otherwise.
    pub fn check(&self) -> Option<String> {
        let name = self.name;
        let (mut a, mut ha) = booted();
        (self.start)(&mut a);
        still(&mut a, &mut ha);
        (self.view)(&mut a);
        still(&mut a, &mut ha);
        (self.drawn)(&mut a);
        quiet(&mut a, &mut ha, self.away);
        let before = still(&mut a, &mut ha);
        let objects = |app: &App| app.document.as_ref().map_or(0, |d| d.model.len());
        let generation = |app: &App| app.document.as_ref().map_or(0, |d| d.model.generation());
        let (count, made) = (objects(&a), generation(&a));
        let camera = |app: &App| {
            let c = app.viewport.camera;
            (c.center.x, c.center.y, c.scale, c.twist)
        };
        let looked = camera(&a);
        (self.edit)(&mut a);
        quiet(&mut a, &mut ha, self.away);
        let edited = still(&mut a, &mut ha);
        // A case is an edit of the drawing or a move of the camera (a zoom across the walls' detail).
        if generation(&a) == made && camera(&a) == looked {
            return Some(format!("{name}: the edit did not reach the drawing ({count} objects)"));
        }

        let (mut b, mut hb) = booted();
        let snapshot = a
            .document
            .as_ref()
            .expect("the drawing is open")
            .model
            .to_snapshot_v2();
        let doc = Document::from_v2(snapshot, None).expect("the edited drawing opens");
        let _ = b.update(Message::Opened(Some(Ok(Box::new(doc)))));
        b.dialog = None;
        crate::files_testing::make_cad(&mut b);
        b.tab = a.tab;
        b.command_expanded = a.command_expanded;
        still(&mut b, &mut hb);
        b.viewport.camera.center = a.viewport.camera.center;
        b.viewport.camera.scale = a.viewport.camera.scale;
        b.viewport.camera.twist = a.viewport.camera.twist;
        hover(&mut b, self.away);
        let fresh = still(&mut b, &mut hb);

        let r = area(&a);
        if r != area(&b) {
            return Some(format!(
                "{name}: the twins' drawing areas differ ({r:?} and {:?})",
                area(&b)
            ));
        }
        let changed = compare(&before, &edited, r);
        let d = compare(&edited, &fresh, r);
        println!(
            "{name}: {} piksel karşılaştırıldı, nesne {count} → {}, düzenleme {} pikseli değiştirdi; tazeden farklı \
             {}, eşi olmayan {}",
            d.compared,
            objects(&a),
            changed.different,
            d.different,
            d.unmatched
        );
        if d.unmatched > 0 || std::env::var_os("KENTOS_RM16_KEEP").is_some() {
            keep(
                "rm16",
                name,
                [("duzenlenen", &edited[..]), ("taze", &fresh[..])],
                r,
            );
        }
        if d.unmatched > 0 {
            // What a user did to see it: the wheel in and back out (the styled scene built again for the scale).
            let scale = a.viewport.camera.scale;
            a.viewport.camera.scale = scale * 4.0;
            still(&mut a, &mut ha);
            a.viewport.camera.scale = scale;
            let zoomed = still(&mut a, &mut ha);
            let z = compare(&zoomed, &fresh, r);
            keep(
                "rm16",
                &format!("{name}-tekerlek"),
                [("tekerlek", &zoomed[..]), ("taze", &fresh[..])],
                r,
            );
            // The styled scenes built anew in the same app: a kept part that missed the edit, or a drawing that
            // draws otherwise after its own history (a detail chosen on the way).
            a.viewport.forget_styled();
            let rebuilt = still(&mut a, &mut ha);
            let k = compare(&rebuilt, &fresh, r);
            return Some(format!(
                "{name}: {} pixels have no match in the fresh picture ({} not the same); after a zoom in and out \
                 {} ({} not the same); with the styled scenes built anew {} ({} not the same)",
                d.unmatched, d.different, z.unmatched, z.different, k.unmatched, k.different
            ));
        }
        (changed.different == 0).then(|| format!("{name}: the edit changed nothing on the screen"))
    }
}

/// The edits checked; a panic naming every one whose pictures differ.
pub fn all_show_at_once(edits: &[Edit<'_>]) {
    let failed: Vec<String> = edits.iter().filter_map(Edit::check).collect();
    assert!(
        failed.is_empty(),
        "the edited pictures are not the fresh ones:\n{}",
        failed.join("\n")
    );
}
