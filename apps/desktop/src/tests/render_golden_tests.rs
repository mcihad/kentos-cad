//! The drawing area's golden pictures (plan/render.md RM-16, docs/adr/9225): small drawings of the repository drawn at
//! fixed cameras against pictures kept in `fixtures/render/golden/`. The twin check (render_testing.rs) holds a kept
//! picture to a fresh one of the same program; this holds the program's picture to the one it drew when the golden
//! was taken: a change that draws differently (a curve back to chords, a line's weight, a lost hatch, a text's face)
//! shows here, wanted or not.
//!
//! A golden is the drawing area only (two pixels in from its edges), as the app draws it on a 1440 × 900 window with
//! its own settings (dark theme, grid, line weights shown). A pixel matches when the golden has its colour, each
//! channel within 40, within one pixel of it; a picture may have at most [`ALLOWED`] pixels unmatched, the edges'
//! smoothing of another GPU (taken on the owner's Intel UHD 770, checked on Mesa's software Vulkan). A case that fails
//! keeps its picture, the golden and their difference in `.run/gui-evidence/render/rm16-altin/`.
//!
//! ```text
//! KENTOS_GPU_TESTS=1 cargo test -p kentos-desktop --lib render_golden -- --nocapture --test-threads=1
//! KENTOS_GPU_TESTS=1 KENTOS_GOLDEN_WRITE=1 cargo test -p kentos-desktop --lib render_golden -- --test-threads=1
//! ```
//!
//! A change meant to draw otherwise writes the goldens anew (`KENTOS_GOLDEN_WRITE=1`) and says so in its ADR, with the
//! pictures before and after.

use std::path::{Path, PathBuf};

use crate::app::Message;
use crate::document::Document;
use crate::render_testing::{area, booted, compare_in, crop, on_gpu, still};
use crate::tools_scenes::run;

/// The pixels of a picture that may have no match in its golden: another GPU's smoothing of the edges. Taken on the
/// owner's Intel UHD 770, the goldens matched on Mesa's software Vulkan with at most 4. Curves drawn back with 72
/// chords (plan/render.md RM-09) left 21 and 38 in two of them.
const ALLOWED: u64 = 16;

/// Where a golden looks.
#[derive(Clone, Copy)]
enum Look {
    /// Tümünü göster.
    Whole,
    /// A point of the drawing in the middle at `scale` pixels a unit (500 and more a metre: the deep view, docs/adr/9220).
    At { center: [f64; 2], scale: f64 },
}

struct Golden {
    name: &'static str,
    fixture: &'static str,
    look: Look,
}

fn goldens() -> Vec<Golden> {
    macro_rules! fixture {
        ($name:literal) => {
            include_str!(concat!("../../../../fixtures/kcad/v2/", $name, ".json"))
        };
    }
    vec![
        Golden { name: "acikliklar", fixture: fixture!("openings"), look: Look::Whole },
        // The door, its jambs, leaf and swing (under the wall), in the deep view.
        Golden {
            name: "acikliklar-derin",
            fixture: fixture!("openings"),
            look: Look::At { center: [1.5, -0.45], scale: 520.0 },
        },
        Golden { name: "duvarlar", fixture: fixture!("walls"), look: Look::Whole },
        Golden { name: "mahaller", fixture: fixture!("spaces"), look: Look::Whole },
        Golden { name: "taramalar", fixture: fixture!("hatches"), look: Look::Whole },
        Golden { name: "cizgi-tipleri", fixture: fixture!("look"), look: Look::Whole },
        Golden { name: "cizgi-kalinliklari", fixture: fixture!("line-weights"), look: Look::Whole },
        Golden { name: "yazilar", fixture: fixture!("texts"), look: Look::Whole },
        Golden { name: "notlar", fixture: fixture!("annotation"), look: Look::Whole },
        // Every kind of the contract's sample, and a polyline out to the largest float that Tümünü göster leaves out
        // (docs/adr/9225; it made the drawing a dot and the grid count without end).
        Golden { name: "cizim", fixture: fixture!("drawing"), look: Look::Whole },
        Golden { name: "olculer", fixture: fixture!("dimensions"), look: Look::Whole },
        Golden { name: "egriler", fixture: fixture!("nurbs"), look: Look::Whole },
        Golden { name: "maskeler", fixture: fixture!("wipeouts"), look: Look::Whole },
        Golden { name: "bloklar", fixture: fixture!("blocks"), look: Look::Whole },
        Golden { name: "merdiven-yaylari", fixture: fixture!("stair-arcs"), look: Look::Whole },
    ]
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The drawing area's picture of the golden's drawing at its camera: width, height, RGBA rows.
fn picture(g: &Golden) -> (usize, usize, Vec<u8>) {
    let (mut app, mut h) = booted();
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_str(g.fixture).expect("the fixture reads");
    let doc = Document::from_v2(snapshot, None).expect("the fixture opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.dialog = None;
    crate::files_testing::make_cad(&mut app);
    still(&mut app, &mut h);
    run(&mut app, "view.zoomExtents");
    still(&mut app, &mut h);
    if let Look::At { center, scale } = g.look {
        app.viewport.camera.center.x = center[0];
        app.viewport.camera.center.y = center[1];
        app.viewport.camera.scale = scale;
    }
    let shot = still(&mut app, &mut h);
    crop(&shot, area(&app))
}

fn read(path: &Path) -> Option<(usize, usize, Vec<u8>)> {
    let bytes = std::fs::read(path).ok()?;
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes)).read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf.chunks(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        _ => return None,
    };
    Some((info.width as usize, info.height as usize, rgba))
}

fn write(path: &Path, w: usize, h: usize, rgba: &[u8]) -> std::io::Result<()> {
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut encoder = png::Encoder::new(file, w as u32, h as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::High);
    encoder
        .write_header()
        .and_then(|mut writer| writer.write_image_data(rgba))
        .map_err(std::io::Error::other)
}

/// The picture, the golden and where they differ (white: no match within a pixel, grey: not the same).
fn keep(name: &str, w: usize, h: usize, shot: &[u8], golden: &[u8]) {
    let dir = root().join(".run/gui-evidence/render/rm16-altin");
    let _ = std::fs::create_dir_all(&dir);
    let mut diff = vec![0u8; shot.len()];
    for i in (0..shot.len()).step_by(4) {
        diff[i + 3] = 255;
        if shot[i..i + 3] != golden[i..i + 3] {
            let one = compare_in(shot, golden, w, h, {
                let (x, y) = ((i / 4) % w, (i / 4) / w);
                [x, y, x + 1, y + 1]
            });
            diff[i..i + 3].fill(if one.unmatched > 0 { 255 } else { 96 });
        }
    }
    for (suffix, rgba) in [("resim", shot), ("altin", golden), ("fark", &diff[..])] {
        let file = dir.join(format!("{name}-{suffix}.png"));
        if write(&file, w, h, rgba).is_ok() {
            println!("resim {}", file.display());
        }
    }
}

#[test]
fn the_drawings_draw_as_their_golden_pictures() {
    if !on_gpu() {
        eprintln!("render_golden: KENTOS_GPU_TESTS=1 is not set; the GPU cases did not run");
        return;
    }
    let writing = std::env::var_os("KENTOS_GOLDEN_WRITE").is_some();
    let dir = root().join("fixtures/render/golden");
    let mut failed = Vec::new();
    for g in goldens() {
        let (w, h, shot) = picture(&g);
        let path = dir.join(format!("{}.png", g.name));
        if writing {
            std::fs::create_dir_all(&dir).expect("the goldens' folder");
            write(&path, w, h, &shot).expect("the golden written");
            println!("{}: yazıldı ({w} × {h})", g.name);
            continue;
        }
        let Some((gw, gh, golden)) = read(&path) else {
            failed.push(format!("{}: no golden at {} (KENTOS_GOLDEN_WRITE=1 writes it)", g.name, path.display()));
            continue;
        };
        if (gw, gh) != (w, h) {
            failed.push(format!("{}: the picture is {w} × {h}, its golden {gw} × {gh}", g.name));
            continue;
        }
        let d = compare_in(&shot, &golden, w, h, [0, 0, w, h]);
        let allowed = ALLOWED;
        println!(
            "{}: {} piksel, altından farklı {}, eşi olmayan {} (izin {allowed})",
            g.name, d.compared, d.different, d.unmatched
        );
        if d.unmatched > allowed {
            keep(g.name, w, h, &shot, &golden);
            failed.push(format!(
                "{}: {} pixels have no match in the golden ({} not the same; at most {allowed})",
                g.name, d.unmatched, d.different
            ));
        }
    }
    assert!(failed.is_empty(), "the pictures are not their goldens:\n{}", failed.join("\n"));
}
