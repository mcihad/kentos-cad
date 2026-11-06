//! Whether the large mixed drawing holds still on a real GPU (plan/cad-advanced.md CAD-ADV-19, docs/adr/1062): the
//! window's own pictures (`Renderer::screenshot`, the app drawn as the user sees it) compared pixel by pixel, and the
//! object snap asked at the same source points from views far apart.
//!
//! - **Kaydırma:** the view moved by a whole number of pixels must show the same pixels moved by as many: lines,
//!   texts, dimensions, hatch patterns and blocks anchored to the world, none swimming. Compared in the middle of the
//!   drawing area (the overlays at its corners are fixed to the screen), far, near (1:200) and close (1:20).
//! - **Ayrıntı geçişi:** a zoom of one part in ten million across the curves' band boundary (`lod::build_band`) and
//!   across the styled layers' quarter-octave step (`quarter_scale`), against the same zoom inside one band: what
//!   changes beyond the control is a jump.
//! - **Kenet:** endpoint and nearest snaps at points on the source geometry (a circle, an arc, a long polyline's
//!   vertex, a line's end), with the same 10 px tolerance in each view: the hit must be the same number to the bit, and
//!   a circle's nearest point on its true radius, not on a chord.
//!
//! Needs a GPU (wgpu); not run by default:
//!
//! ```text
//! cargo test --release -p kentos-desktop --lib perf::stability -- --ignored --nocapture --test-threads=1
//! ```

use kentos_commands::Entity;
use kentos_domain::Slot;
use kentos_geometry_core::store::snap::SnapKind;
use kentos_render_wgpu::RenderSettings;
use kentos_render_wgpu::scene::lod;
use serde_json::{Value, json};

use super::frame::{Harness, WINDOW};
use super::mixed;
use crate::app::{App, Message};
use crate::document::Document;

const OBJECTS: usize = 10_000;
/// A channel apart by more than this is a visible difference, not an edge's rounding.
const VISIBLE: u8 = 64;

/// The app with the mixed drawing open on a GPU.
fn opened() -> (App, Harness) {
    let (mut app, _) = App::boot(None);
    let mut h = Harness::new();
    assert!(
        h.on_gpu(),
        "a GPU (wgpu) is needed; this is no pass without one"
    );
    h.settle(&mut app);
    let (snapshot, _) = mixed::drawing(OBJECTS, mixed::SEED);
    let doc = Document::from_v2(snapshot, None).expect("the drawing opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.dialog = None;
    h.settle(&mut app);
    (app, h)
}

/// The window's picture with the camera here (frames drawn until nothing more is built).
fn shot(app: &mut App, h: &mut Harness, center: [f64; 2], scale: f64) -> Vec<u8> {
    app.viewport.camera.center.x = center[0];
    app.viewport.camera.center.y = center[1];
    app.viewport.camera.scale = scale;
    for _ in 0..3 {
        h.frame(app, &[]);
    }
    h.picture(app)
}

/// The middle of the drawing area, its outer 15 % left out on every side (the screen-fixed overlays).
fn region(app: &App) -> [i64; 4] {
    let b = app.viewport.bounds;
    [
        (b.x + b.width * 0.15) as i64,
        (b.y + b.height * 0.15) as i64,
        (b.x + b.width * 0.85) as i64,
        (b.y + b.height * 0.85) as i64,
    ]
}

/// Pixels of `a` at (x, y) against `b` at (x + dx, y + dy) over the region: compared, any channel different,
/// visibly different, the largest channel difference, and pixels with ink (not the background) in `a`.
fn compare(a: &[u8], b: &[u8], r: [i64; 4], dx: i64, dy: i64) -> Value {
    let w = WINDOW.width as i64;
    let bg = background(a, r);
    let (mut n, mut any, mut visible, mut most, mut ink) = (0u64, 0u64, 0u64, 0u8, 0u64);
    for y in r[1].max(r[1] - dy)..r[3].min(r[3] - dy) {
        for x in r[0].max(r[0] - dx)..r[2].min(r[2] - dx) {
            let i = ((y * w + x) * 4) as usize;
            let j = (((y + dy) * w + x + dx) * 4) as usize;
            let d = (0..3)
                .map(|c| a[i + c].abs_diff(b[j + c]))
                .max()
                .unwrap_or(0);
            n += 1;
            any += u64::from(d > 0);
            visible += u64::from(d > VISIBLE);
            most = most.max(d);
            ink += u64::from(a[i..i + 3] != bg);
        }
    }
    json!({ "compared": n, "ink": ink, "different": any, "visible": visible, "largest": most })
}

/// The region's most common colour in `a`: the drawing's paper.
fn background(a: &[u8], r: [i64; 4]) -> [u8; 3] {
    let w = WINDOW.width as i64;
    let mut counts = std::collections::HashMap::new();
    for y in (r[1]..r[3]).step_by(3) {
        for x in (r[0]..r[2]).step_by(3) {
            let i = ((y * w + x) * 4) as usize;
            *counts.entry([a[i], a[i + 1], a[i + 2]]).or_insert(0u32) += 1;
        }
    }
    counts
        .into_iter()
        .max_by_key(|(_, n)| *n)
        .map_or([0; 3], |(c, _)| c)
}

/// The pictures for the owner (`.run/gui-evidence/cad-adv-19/`, never committed): `a`, `b`, and where they differ
/// (white: any channel apart by more than [`VISIBLE`], grey: apart at all).
fn keep(name: &str, a: &[u8], b: &[u8], dx: i64, dy: i64) {
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/cad-adv-19");
    let _ = std::fs::create_dir_all(&dir);
    let (w, h) = (WINDOW.width as i64, WINDOW.height as i64);
    let mut diff = vec![0u8; a.len()];
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            diff[i + 3] = 255;
            let (bx, by) = (x + dx, y + dy);
            if bx < 0 || by < 0 || bx >= w || by >= h {
                continue;
            }
            let j = ((by * w + bx) * 4) as usize;
            let d = (0..3)
                .map(|c| a[i + c].abs_diff(b[j + c]))
                .max()
                .unwrap_or(0);
            let v = if d > VISIBLE {
                255
            } else if d > 0 {
                96
            } else {
                0
            };
            diff[i..i + 3].fill(v);
        }
    }
    for (suffix, rgba) in [("a", a), ("b", b), ("fark", &diff[..])] {
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

/// The middle cell's `k`th object and its id.
fn middle(app: &App, k: usize) -> (u32, Entity) {
    let side = mixed::side(OBJECTS);
    let cells = OBJECTS.div_ceil(10);
    let cell = ((cells.div_ceil(side) / 2) * side + side / 2).min(cells - 1);
    let id = (cell * 10 + k + 1) as u32;
    let e = app
        .document
        .as_ref()
        .and_then(|d| d.model.get(Slot(id)))
        .cloned()
        .expect("the object");
    (id, e)
}

#[test]
#[ignore = "a measurement on a real GPU, run by hand in release mode"]
fn stability() {
    let (mut app, mut h) = opened();
    let fit = app.viewport.camera;
    let e = mixed::extent(OBJECTS);
    let mid = [(e[0] + e[2]) / 2.0, (e[1] + e[3]) / 2.0];
    let views = [
        ("uzak", [fit.center.x, fit.center.y], fit.scale),
        ("yakın 1:200", mid, 1.0 / (200.0 * 0.000_264_58)),
        (
            "milimetrik 1:20",
            [mid[0] - 8.0, mid[1] - 6.0],
            1.0 / (20.0 * 0.000_264_58),
        ),
    ];
    let r = region(&app);
    let mut out = Vec::new();
    // Pans of whole pixels: 37 right, 23 down.
    let (dx, dy) = (37i64, 23i64);
    for (name, c, s) in views {
        let a = shot(&mut app, &mut h, c, s);
        let b = shot(
            &mut app,
            &mut h,
            [c[0] - dx as f64 / s, c[1] + dy as f64 / s],
            s,
        );
        let again = shot(&mut app, &mut h, c, s);
        keep(
            &format!("kaydirma-{}", name.split(' ').next().unwrap_or(name)),
            &a,
            &b,
            dx,
            dy,
        );
        // A pan of whole 8 px cells as a control: the label cells' size (docs/adr/1062 §4).
        let cells = shot(&mut app, &mut h, [c[0] - 40.0 / s, c[1] + 24.0 / s], s);
        let row = json!({
            "view": name, "pxPerM": s, "pan": compare(&a, &b, r, dx, dy),
            "panOfWholeCells": compare(&a, &cells, r, 40, 24),
            "sameViewAgain": compare(&a, &again, r, 0, 0),
        });
        println!("kaydırma {row}");
        out.push(row);
    }
    // A pan's moved picture (docs/adr/1067). Today's frame: the view panned by whole pixels without moving, drawn
    // whole (`today`). The same pan while moving: the kept picture moved and the strips drawn (`moved`), against the
    // same view drawn whole while moving (`whole`: the styled parts built again, so nothing is held to move); then the
    // view settled, drawn whole again (`settled`), against today's. The same views before each (the grid's part and
    // the labels' placing follow where the view has been).
    for (name, c, s) in views {
        let target = [c[0] - dx as f64 / s, c[1] + dy as f64 / s];
        shot(&mut app, &mut h, c, s);
        let today = shot(&mut app, &mut h, target, s);
        shot(&mut app, &mut h, c, s);
        app.viewport.hold_moving(true);
        app.viewport.camera.center.x = target[0];
        app.viewport.camera.center.y = target[1];
        h.frame(&mut app, &[]);
        let was_moved = app.viewport.status().stats.picture_moved;
        let moved = shot(&mut app, &mut h, target, s);
        app.viewport.hold_moving(false);
        h.frame(&mut app, &[]);
        let stats = app.viewport.status().stats;
        let settled_whole = !stats.picture_kept && !stats.picture_moved;
        let settled = shot(&mut app, &mut h, target, s);
        // The moved picture against the view drawn whole while moving: the parts built again, the same view.
        shot(&mut app, &mut h, c, s);
        app.viewport.hold_moving(true);
        shot(&mut app, &mut h, target, s);
        app.styles.block_proxy = false;
        h.frame(&mut app, &[]);
        app.styles.block_proxy = true;
        let whole = shot(&mut app, &mut h, target, s);
        app.viewport.hold_moving(false);
        shot(&mut app, &mut h, target, s);
        keep(&format!("kaydirirken-{}", name.split(' ').next().unwrap_or(name)), &moved, &whole, 0, 0);
        keep(&format!("durunca-{}", name.split(' ').next().unwrap_or(name)), &settled, &today, 0, 0);
        let row = json!({
            "view": name, "pxPerM": s, "pictureMoved": was_moved, "settledDrawnWhole": settled_whole,
            "movedAgainstWhole": compare(&moved, &whole, r, 0, 0),
            "settledAgainstToday": compare(&settled, &today, r, 0, 0),
        });
        println!("kaydırırken {row}");
        out.push(row);
    }
    // Detail steps: across the curves' band and the styled quarter octave near 1:200, against a step inside one.
    let tol = RenderSettings::new(kentos_render_wgpu::Rgba8([0, 0, 0, 255])).curve_tolerance_px;
    let near = 1.0 / (200.0 * 0.000_264_58);
    let band = lod::build_band(near, tol);
    // band = floor(log2(tol / s)) − 1 changes where tol / s is a power of two.
    let band_edge = tol / 2f64.powi(band + 1);
    let quarter_edge = 2f64.powf(((near.log2() * 4.0).floor() + 0.5) / 4.0);
    let step = 1e-7;
    // The styled curves' first coarser level's ceiling (docs/adr/1063 §3): the fine chords above, the coarse below.
    let level_edge = kentos_native_style::batches::CURVE_LEVELS[0];
    for (name, edge) in [
        ("eğri bandı", band_edge),
        ("çeyrek oktav", quarter_edge),
        ("eğri düzeyi", level_edge),
    ] {
        let below = shot(&mut app, &mut h, mid, edge * (1.0 - step));
        let above = shot(&mut app, &mut h, mid, edge * (1.0 + step));
        let further = shot(&mut app, &mut h, mid, edge * (1.0 + 3.0 * step));
        let row = json!({
            "step": name, "pxPerM": edge,
            "bands": [lod::build_band(edge * (1.0 - step), tol), lod::build_band(edge * (1.0 + step), tol)],
            "quarters": [crate::style::scene::quarter_scale(edge * (1.0 - step)), crate::style::scene::quarter_scale(edge * (1.0 + step))],
            "across": compare(&below, &above, r, 0, 0), "inside": compare(&above, &further, r, 0, 0),
        });
        println!("ayrıntı {row}");
        out.push(row);
    }
    // The drawing area's shortcuts against the plain build (docs/adr/1063), one at a time, view by view (near and far,
    // on both sides of the curves' first level's ceiling, the farthest level): blocks' inserts drawn instanced must
    // show what the plain build shows (asserted below); the curves' coarser levels are measured and reported, their
    // far views drawing small circles with fewer overlapping chords (§4); the long strokes' levels (docs/adr/1064 §2)
    // likewise, on top of the other two (the app's own drawing with and without them): measured and reported, near
    // views asserted unchanged below.
    let shortcut_views: Vec<(&str, [f64; 2], f64)> = views
        .iter()
        .copied()
        .chain([
            ("düzey-alti", mid, level_edge * (1.0 - step)),
            ("düzey-ustu", mid, level_edge * (1.0 + step)),
            ("en-uzak-düzey", mid, kentos_native_style::batches::CURVE_LEVELS[2] * 0.5),
        ])
        .collect();
    // Each shortcut: what is on with it and without it (instancing, curve levels, line levels, the blocks' far proxy,
    // docs/adr/1065 §4), and its pictures' name.
    for (shortcut, with, without, tag) in [
        ("blok örnekleme", [true, false, false, false], [false; 4], "örnekleme"),
        ("eğri düzeyleri", [false, true, false, false], [false; 4], "düzeyleri"),
        ("çizgi düzeyleri", [true, true, true, false], [true, true, false, false], "cizgi-düzeyleri"),
        ("blok vekili", [true; 4], [true, true, true, false], "blok-vekili"),
    ] {
        for &(name, c, s) in &shortcut_views {
            let set = |app: &mut App, on: bool| {
                let [i, l, line, proxy] = if on { with } else { without };
                app.styles.instancing = i;
                app.styles.curve_lod = l;
                app.styles.line_lod = line;
                app.styles.block_proxy = proxy;
            };
            set(&mut app, true);
            let on = shot(&mut app, &mut h, c, s);
            set(&mut app, false);
            let off = shot(&mut app, &mut h, c, s);
            keep(
                &format!("kisayol-{tag}-{}", name.replace(' ', "-")),
                &on,
                &off,
                0,
                0,
            );
            let row = json!({ "shortcut": shortcut, "view": format!("{name} (kısayol)"), "pxPerM": s,
                "onAgainstOff": compare(&on, &off, r, 0, 0) });
            println!("kısayol {row}");
            out.push(row);
        }
    }
    app.styles.instancing = true;
    app.styles.curve_lod = true;
    app.styles.line_lod = true;
    app.styles.block_proxy = true;
    // Snaps at source points from each view, 10 px apart.
    let (_, circle) = middle(&app, 6);
    let (_, line) = middle(&app, 0);
    let (_, long) = {
        // The long polyline of the first cell of the middle row of ten.
        let side = mixed::side(OBJECTS);
        let cells = OBJECTS.div_ceil(10);
        let cell = ((cells.div_ceil(side) / 2) * side).min(cells - 1) / 10 * 10;
        let id = (cell * 10 + 9 + 1) as u32;
        (
            id,
            app.document
                .as_ref()
                .and_then(|d| d.model.get(Slot(id)))
                .cloned()
                .expect("the polyline"),
        )
    };
    let mut asks = Vec::new();
    if let Entity::Circle(c) = &circle {
        let a: f64 = 0.7;
        asks.push((
            "daire, en yakın",
            [c.c.x + c.r * a.cos(), c.c.y + c.r * a.sin()],
            SnapKind::Nearest,
            Some((c.c, c.r)),
        ));
    }
    if let Entity::Line(l) = &line {
        asks.push(("çizgi ucu", [l.a.x, l.a.y], SnapKind::Endpoint, None));
    }
    if let Entity::Polyline(p) = &long {
        let v = p.pts[200];
        asks.push(("uzun çizginin köşesi", [v.x, v.y], SnapKind::Endpoint, None));
    }
    assert!(
        asks.len() == 3,
        "the middle cell has a circle, a line and the row a long polyline"
    );
    let mut snaps = Vec::new();
    for (name, at, kind, circle) in asks {
        let hits: Vec<Option<[f64; 2]>> = views
            .iter()
            .map(|(_, c, s)| {
                app.viewport.camera.center.x = c[0];
                app.viewport.camera.center.y = c[1];
                app.viewport.camera.scale = *s;
                app.spatial
                    .snap(
                        kentos_interaction::Vec2::new(at[0], at[1]),
                        10.0 / s,
                        kind.bit(),
                        None,
                    )
                    .map(|hit| [hit.point.x, hit.point.y])
            })
            .collect();
        let same = hits.windows(2).all(|w| {
            w[0].zip(w[1]).is_some_and(|(a, b)| {
                a[0].to_bits() == b[0].to_bits() && a[1].to_bits() == b[1].to_bits()
            })
        });
        let off_radius = circle
            .and_then(|(c, rad)| hits[0].map(|h| ((h[0] - c.x).hypot(h[1] - c.y) - rad).abs()));
        let row =
            json!({ "snap": name, "hits": hits, "sameToTheBit": same, "offRadius": off_radius });
        println!("kenet {row}");
        snaps.push((same, off_radius, row));
    }
    let report = json!({ "objects": OBJECTS, "seed": mixed::SEED, "rows": out, "snaps": snaps.iter().map(|s| &s.2).collect::<Vec<_>>() });
    if let Ok(path) = std::env::var("KENTOS_PERF_OUT") {
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
            .join(format!(
                "stability-desktop-{}.json",
                super::flat::chrono_free_date()
            ));
        std::fs::write(
            &file,
            format!("{}\n", serde_json::to_string_pretty(&report).expect("JSON")),
        )
        .expect("writes");
        println!("Yazıldı: {}", file.display());
    }
    // Nothing visibly apart (a channel by more than VISIBLE) after a pan or a step across a detail band, near; the
    // overview's count is reported (a text the view's edge cuts may change who stays near it, docs/adr/1062 §4).
    for row in &out {
        let near = row["view"].as_str().is_some_and(|v| v != "uzak");
        // Instanced blocks show what the plain build shows, in every view (docs/adr/1063 §2); the curves' levels are
        // reported (§4).
        if row["shortcut"] == "blok örnekleme" {
            assert_eq!(row["onAgainstOff"]["visible"], 0, "instanced blocks draw the same: {row}");
            continue;
        }
        // The long strokes' levels draw level 0 above the first ceiling: nearer than 1:950 nothing changes
        // (docs/adr/1064 §4); farther the change is reported.
        // The blocks' far proxy draws only where an insert is a few pixels: from 1 px a metre on, nothing changes
        // (docs/adr/1065 §4); farther the change is reported.
        if row["shortcut"] == "blok vekili" {
            if row["pxPerM"].as_f64().is_some_and(|s| s >= 1.0) {
                assert_eq!(row["onAgainstOff"]["visible"], 0, "near, the blocks draw as before: {row}");
            }
            continue;
        }
        // A pan's moved picture (docs/adr/1067): the frame did move the kept picture; nothing visibly apart from the
        // view drawn whole; settled, the view is drawn whole again, pixel for pixel as drawn whole from elsewhere.
        if row.get("movedAgainstWhole").is_some() {
            assert_eq!(row["pictureMoved"], true, "a pan by whole pixels moves the picture: {row}");
            assert_eq!(row["movedAgainstWhole"]["visible"], 0, "the moved picture shows the view: {row}");
            assert_eq!(row["settledDrawnWhole"], true, "settled, the view is drawn whole: {row}");
            assert_eq!(row["settledAgainstToday"]["different"], 0, "settled, the frame is today's: {row}");
            continue;
        }
        if row["shortcut"] == "çizgi düzeyleri" {
            if row["pxPerM"].as_f64().is_some_and(|s| s > kentos_native_style::batches::CURVE_LEVELS[0]) {
                assert_eq!(row["onAgainstOff"]["visible"], 0, "near, the long strokes draw as before: {row}");
            }
            continue;
        }
        // The curves' level's step is reported: at its ceiling (4 px a metre) its control, a zoom inside level 0,
        // is not still either, whatever the levels draw (docs/adr/1063 §4).
        if row["step"] == "eğri düzeyi" || row.get("shortcut").is_some() {
            continue;
        }
        for key in ["pan", "panOfWholeCells", "across"] {
            if (near || row.get("step").is_some()) && row.get(key).is_some() {
                assert_eq!(row[key]["visible"], 0, "{key} holds still: {row}");
            }
        }
    }
    for (same, off, row) in &snaps {
        assert!(*same, "the snap does not depend on the view: {row}");
        if let Some(off) = off {
            assert!(
                *off < 1e-9,
                "the nearest point is on the true circle: {row}"
            );
        }
    }
}
