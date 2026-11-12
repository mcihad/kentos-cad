//! Sınır grafiği's marks (parcel_boundaries_marks.rs, docs/adr/3820): an arc drawn with chords shows its strip — from
//! the shared case's answer a closed outline from the arc's first corner along the arc and back along the chain, its
//! area the case's strip; problems whose rings would meet on the screen share one ring and one phrase. And the
//! pictures.

use iced::Point;
use kentos_commands::ParcelsBounded;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Message};
use crate::parcel_boundaries_marks::{GraphMarks, gathered};
use crate::parcel_boundaries_window::{Event as B, Show};

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.parcels.boundaries.json")).expect("cases")
}

fn case(name: &str) -> Value {
    cases()["cases"].as_array().expect("cases").iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(name))).expect("the case").clone()
}

fn answer(name: &str) -> ParcelsBounded {
    serde_json::from_value(case(name)["steps"][0]["result"]["output"].clone()).expect("an answer")
}

/// The outline's area by the shoelace, about its first point (the map's coordinates are large).
fn area(s: &[kentos_interaction::Vec2]) -> f64 {
    let o = s[0];
    s.windows(2).map(|w| (w[0].x - o.x) * (w[1].y - o.y) - (w[1].x - o.x) * (w[0].y - o.y)).sum::<f64>() / 2.0
}

const ON_THE_ARC: &str = "yay kirişlerle çizilmiş: üç ara köşe yayın üstünde";
const UNEQUAL: &str = "eşit olmayan kirişler";

#[test]
fn an_arc_drawn_with_chords_shows_its_strip() {
    for name in [ON_THE_ARC, UNEQUAL] {
        let g = answer(name);
        let m = GraphMarks::of(&g);
        assert_eq!(m.strips.len(), 1, "{name}");
        let s = &m.strips[0];
        let p = g.problems.iter().find(|p| p.kind == "approxArc").expect("the problem");
        let arc = g.boundaries.iter().find(|b| b.id == p.boundaries[0]).expect("the arc");
        let at = |id: &str| g.corners.iter().find(|c| c.id == id).map(|c| c.at).expect("a corner");
        let (first, last) = (s[0], s[s.len() - 1]);
        let start = at(&arc.from);
        assert!((first.x - start[0]).abs() < 1e-9 && (first.y - start[1]).abs() < 1e-9, "{name}: from the arc's first corner");
        assert!((last.x - first.x).abs() < 1e-9 && (last.y - first.y).abs() < 1e-9, "{name}: closed at it again");
        // The chain's every corner is on the outline.
        for id in &p.boundaries[1..] {
            let b = g.boundaries.iter().find(|b| b.id == *id).expect("a chain boundary");
            for c in [&b.from, &b.to] {
                let q = at(c);
                assert!(s.iter().any(|v| (v.x - q[0]).abs() < 1e-9 && (v.y - q[1]).abs() < 1e-9), "{name}: {c}");
            }
        }
    }
    // The chain inside the arc: the outline's area is the case's strip, within the drawing of the arc by points.
    let g = answer(ON_THE_ARC);
    let want = g.problems[0].area.expect("an area").abs();
    let got = area(&GraphMarks::of(&g).strips[0]).abs();
    assert!((got - want).abs() <= 0.01 * want, "{got} ≠ {want}");
}

#[test]
fn problems_whose_rings_meet_share_one_ring_and_one_phrase() {
    let p = Point::new;
    let got = gathered(&[
        (p(100.0, 100.0), "Komşuda eksik köşe"),
        (p(108.0, 104.0), "Ayrık köşe"),
        (p(104.0, 96.0), "Komşuda eksik köşe"),
        (p(300.0, 100.0), "Kirişli yay"),
        (p(100.0, 118.5), "Çakışan kenar"),
    ]);
    assert_eq!(
        got,
        vec![
            (p(100.0, 100.0), "Komşuda eksik köşe (2) · Ayrık köşe".to_owned()),
            (p(300.0, 100.0), "Kirişli yay".to_owned()),
            (p(100.0, 118.5), "Çakışan kenar".to_owned()),
        ],
        "18.5 px apart: rings of 9 px do not meet"
    );
}

/// The drawing of the case named `name`, open in Harita, the graph of the case's parcels (else the parcel layer's) read
/// with `tolerance` and drawn (Çizimde göster).
fn drawn(name: &str, tolerance: Option<f64>) -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case(name)["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    // The case's own parcels when it names them (Parsel alan denetimi checks the selection), else the layer's.
    let uids: Vec<String> = serde_json::from_value(case(name)["steps"][0]["input"]["uids"].clone()).unwrap_or_default();
    let slots: Vec<kentos_domain::Slot> = app
        .document
        .as_ref()
        .map(|d| uids.iter().filter_map(|u| kentos_domain::Uuid::parse_str(u).ok().and_then(|u| d.model.slot_of(u))).collect())
        .unwrap_or_default();
    app.selection.set(slots);
    let _ = app.update(Message::Run("parcels.check"));
    let _ = app.update(Message::ParcelBoundaries(B::Open));
    if let Some(t) = tolerance {
        let _ = app.update(Message::ParcelBoundaries(B::Tolerance(t)));
    }
    let _ = app.update(Message::ParcelBoundaries(B::Show(Show::Problems)));
    let _ = app.update(Message::ParcelBoundaries(B::Draw));
    app.selection.clear();
    app
}

#[test]
fn the_strip_is_on_the_drawing() {
    let app = drawn(ON_THE_ARC, None);
    let marks = app.parcel_graph_marks().expect("marks");
    assert_eq!(marks.strips.len(), 1);
}

/// The strip close to its widest place, and the block with problems seen from afar (their rings shared), light wide
/// and dark narrow, `.run/gui-evidence/kirisli-yay-serit-3820/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib parcel_boundaries_marks_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    const PROBLEMS: &str = "sorunlu ada: T birleşimi, 0,4 µm ayrık köşeler, bir yanda iki parsel";
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kirisli-yay-serit-3820");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        for (case_name, file) in [(ON_THE_ARC, "1-serit"), (PROBLEMS, "2-uzaktan")] {
            let mut app = drawn(case_name, None);
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let g = answer(case_name);
            let all = g.corners.iter().fold([f64::MAX, f64::MAX, f64::MIN, f64::MIN], |b, c| {
                [b[0].min(c.at[0]), b[1].min(c.at[1]), b[2].max(c.at[0]), b[3].max(c.at[1])]
            });
            let bounds = if case_name == ON_THE_ARC {
                // Around the strip's widest place: a few metres, the strip wide on the screen.
                let at = g.problems[0].at;
                kentos_render_wgpu::Bounds { min_x: at[0] - 3.0, min_y: at[1] - 2.0, max_x: at[0] + 3.0, max_y: at[1] + 2.0 }
            } else {
                // From afar: the block a small part of the view, its problems' rings meeting.
                let (w, h) = (all[2] - all[0], all[3] - all[1]);
                kentos_render_wgpu::Bounds { min_x: all[0] - 6.0 * w, min_y: all[1] - 6.0 * h, max_x: all[2] + 6.0 * w, max_y: all[3] + 6.0 * h }
            };
            app.viewport.camera.fit(&bounds, 24.0);
            snapshot.settle(&mut app, App::view, &mut update);
            let path = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&path).expect("writes");
            println!("{}", path.display());
        }
    }
}
