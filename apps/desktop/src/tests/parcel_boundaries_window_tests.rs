//! Sınır grafiği (parcel_boundaries_window.rs, docs/adr/2000) on the shared cases' drawings of `cad.parcels.boundaries`:
//! opened from Parsel alan denetimi on the parcel layer, it counts the corners and boundaries as the case does; on the
//! block with problems it lists the T junction, the split corners and the overlap, and Sorunluları seç selects their
//! parcels; Çizimde göster leaves the graph's marks on the drawing, worked out again as it changes, until İşareti
//! kaldır (docs/adr/2220); nothing is written to the drawing. And its pictures.

use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::parcel_boundaries_window::{Event as B, Show, rows};

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.parcels.boundaries.json")).expect("cases")
}

/// The drawing of the case named `name` (none: the file's own), open in Harita.
fn opened(name: Option<&str>) -> App {
    let c = cases();
    let setup = match name {
        Some(n) => c["cases"].as_array().expect("cases").iter().find(|x| x["name"] == n).expect("the case")["setup"].clone(),
        None => c["setup"].clone(),
    };
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

const PROBLEMS: &str = "sorunlu ada: T birleşimi, 0,4 µm ayrık köşeler, bir yanda iki parsel";
const APPROX: &str = "santimetreye yuvarlanmış köşeler 1 cm'lik uzaklıkla: yaklaşık yay";

/// docs/adr/3420: the neighbour drawing A1's arc with chords, its corners rounded to the centimetre. Without a distance
/// only the corner at the arc's top (exactly on it) is a missing corner; with 0.01 m the arc drawn with chords is one
/// problem with the case's boundaries, strip and widest place, said in the Fark column and the summary.
#[test]
fn an_arc_drawn_with_chords_is_one_problem_with_a_distance() {
    let mut app = opened(Some(APPROX));
    app.selection.set([kentos_domain::Slot(1), kentos_domain::Slot(3)]);
    let _ = app.update(Message::Run("parcels.check"));
    let _ = app.update(Message::ParcelBoundaries(B::Open));
    let g = app.parcel_boundaries.answer.clone().expect("an answer").expect("read");
    assert_eq!(g.problems.iter().map(|p| p.kind.as_str()).collect::<Vec<_>>(), ["missingVertex"]);
    let _ = app.update(Message::ParcelBoundaries(B::Tolerance(0.01)));
    let g = app.parcel_boundaries.answer.clone().expect("an answer").expect("read");
    let c = cases();
    let want = &c["cases"].as_array().expect("cases").iter().find(|x| x["name"] == APPROX).expect("the case")["steps"][0]
        ["result"]["output"]["problems"][0];
    assert_eq!(g.problems.len(), 1);
    let got = &g.problems[0];
    assert_eq!((got.kind.as_str(), serde_json::to_value(&got.boundaries).expect("json")), ("approxArc", want["boundaries"].clone()));
    let close = |a: Option<f64>, key: &str| (a.expect(key) - want[key].as_f64().expect(key)).abs() < 1e-9;
    assert!(close(got.area, "area") && close(got.gap, "gap"), "{got:?}");
    let f = app.format();
    let shown = rows(&g, Show::Problems, &f);
    assert_eq!(shown[0][0], "Yay kirişlerle çizilmiş");
    let b = &got.boundaries;
    assert_eq!(
        shown[0][4],
        format!("{} ile {}–{} (4 kiriş) · {}, en geniş {}", b[0], b[1], b[4], f.area(got.area.expect("area")), f.length(got.gap.expect("gap")))
    );
}

/// docs/adr/3420: the window with the distance 0.01 m and the problem listed, then on the drawing; light wide and dark
/// narrow, `.run/gui-evidence/kirisli-yay-3420/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib parcel_boundaries_window_tests::approx_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn approx_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kirisli-yay-3420");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        for (draw, file) in [(false, "1-pencere"), (true, "2-cizimde")] {
            let mut app = opened(Some(APPROX));
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            app.selection.set([kentos_domain::Slot(1), kentos_domain::Slot(3)]);
            let _ = app.update(Message::Run("view.zoomExtents"));
            let _ = app.update(Message::Run("parcels.check"));
            let _ = app.update(Message::ParcelBoundaries(B::Open));
            let _ = app.update(Message::ParcelBoundaries(B::Tolerance(0.01)));
            let _ = app.update(Message::ParcelBoundaries(B::Show(Show::Problems)));
            if draw {
                let _ = app.update(Message::ParcelBoundaries(B::Choose(0)));
                let _ = app.update(Message::ParcelBoundaries(B::Draw));
                app.selection.clear();
            }
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let path = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&path).expect("writes");
            println!("{}", path.display());
        }
    }
}
const MISMATCH: &str = "uyuşmayan üç sınır bir arada: her biri kendi sorunu";

/// docs/adr/2620: an arc against its chord, two arcs the same way and two arcs opposite ways, each a problem with its
/// boundaries, sliver and gap as the case says, in the table's Fark column.
#[test]
fn an_arc_against_its_chord_is_listed_with_its_sliver() {
    let mut app = opened(Some(MISMATCH));
    let _ = app.update(Message::Run("parcels.check"));
    let _ = app.update(Message::ParcelBoundaries(B::Open));
    let g = app.parcel_boundaries.answer.clone().expect("an answer").expect("read");
    let c = cases();
    let want = c["cases"].as_array().expect("cases").iter().find(|x| x["name"] == MISMATCH).expect("the case")["steps"][0]
        ["result"]["output"]["problems"]
        .clone();
    let want = want.as_array().expect("problems");
    assert_eq!(g.problems.len(), want.len());
    for (got, w) in g.problems.iter().zip(want) {
        assert_eq!(got.kind, w["kind"].as_str().expect("kind"));
        assert_eq!(serde_json::to_value(&got.boundaries).expect("json"), w["boundaries"]);
        assert_eq!(serde_json::to_value(&got.parcels).expect("json"), w["parcels"]);
        // The case's numbers within its own 1e-9 (the reference's arcs are mpmath's).
        let close = |a: Option<f64>, key: &str| (a.expect(key) - w[key].as_f64().expect(key)).abs() < 1e-9;
        assert!(close(got.area, "area") && close(got.gap, "gap"), "{got:?}");
    }
    let shown = rows(&g, Show::Problems, &app.format());
    assert_eq!(shown[0][0], "Yay ile kiriş uyuşmuyor");
    let f = app.format();
    assert_eq!(shown[0][4], format!("{} · {}, ortada {}", g.problems[0].boundaries.join(" ile "), f.area(6.719699229259696), f.length(1.0)));
}

#[test]
fn the_clean_block_s_graph_from_the_parcel_check() {
    let mut app = opened(None);
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Run("parcels.check"));
    let _ = app.update(Message::ParcelBoundaries(B::Open));
    assert_eq!(app.dialog, Some(Dialog::ParcelBoundaries));
    let g = app.parcel_boundaries.answer.clone().expect("an answer").expect("read");
    let want = &cases()["cases"][0]["steps"][0]["result"]["output"];
    assert_eq!(g.corners.len(), want["corners"].as_array().map_or(0, Vec::len));
    assert_eq!(g.boundaries.len(), want["boundaries"].as_array().map_or(0, Vec::len));
    assert_eq!((Value::from(g.shared), Value::from(g.outer)), (want["shared"].clone(), want["outer"].clone()));
    assert!(g.problems.is_empty());
    let first = &rows(&g, Show::Boundaries, &app.format())[0];
    assert_eq!(first[0], "S1");
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
}

#[test]
fn the_problems_are_listed_and_their_parcels_selected() {
    let mut app = opened(Some(PROBLEMS));
    let _ = app.update(Message::Run("parcels.check"));
    let _ = app.update(Message::ParcelBoundaries(B::Open));
    let g = app.parcel_boundaries.answer.clone().expect("an answer").expect("read");
    let kinds: Vec<&str> = g.problems.iter().map(|p| p.kind.as_str()).collect();
    assert_eq!(kinds, ["missingVertex", "missingVertex", "missingVertex", "splitCorner", "splitCorner", "overlap"]);
    let _ = app.update(Message::ParcelBoundaries(B::Show(Show::Problems)));
    let shown = rows(&g, Show::Problems, &app.format());
    assert_eq!(shown[0][0], "Komşuda eksik köşe");
    let _ = app.update(Message::ParcelBoundaries(B::Select));
    assert_eq!(app.dialog, None);
    assert_eq!(app.selection.ids().len(), 7, "every parcel of the block is in a problem");
}

/// Çizimde göster (docs/adr/2220): the window closes; the problems' words, the corners' and the boundaries' numbers
/// stay on the drawing as the graph says. Deleting Q7 works them out again (its overlap and its two corners go);
/// İşareti kaldır takes them away. Nothing is written but the deletion.
#[test]
fn the_graph_stays_on_the_drawing_until_unmarked() {
    let mut app = opened(Some(PROBLEMS));
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Run("parcels.check"));
    let _ = app.update(Message::ParcelBoundaries(B::Open));
    let g = app.parcel_boundaries.answer.clone().expect("an answer").expect("read");
    let _ = app.update(Message::ParcelBoundaries(B::Draw));
    assert_eq!(app.dialog, None);
    let m = app.parcel_graph_marks().expect("the marks");
    assert_eq!((m.corners.len(), m.boundaries.len()), (g.corners.len(), g.boundaries.len()));
    assert_eq!(m.corners[0].1, "K1");
    let words: Vec<&str> = m.problems.iter().map(|(_, w)| w.as_str()).collect();
    assert_eq!(
        words,
        ["Komşuda eksik köşe", "Komşuda eksik köşe", "Komşuda eksik köşe", "Ayrık köşe", "Ayrık köşe", "Çakışan kenar"]
    );
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
    app.selection.set(vec![kentos_domain::Slot(7)]);
    let _ = app.update(Message::Run("tool.erase"));
    let m = app.parcel_graph_marks().expect("the marks again");
    let words: Vec<&str> = m.problems.iter().map(|(_, w)| w.as_str()).collect();
    assert_eq!(words, ["Komşuda eksik köşe", "Ayrık köşe", "Ayrık köşe"]);
    let _ = app.update(Message::Run("data.unmark"));
    assert!(app.parcel_graph_marks().is_none());
}

/// docs/adr/3020: a row pressed is chosen and marked; Çizimde göster goes to it and the marks bring it forward — a
/// problem with a double ring at its place, a boundary along its curve (an arc's points along it), found again by its
/// id when the drawing changes; another table clears the choice; without a choice the whole graph, nothing forward.
#[test]
fn the_chosen_row_is_brought_forward_on_the_drawing() {
    let mut app = opened(Some(PROBLEMS));
    let _ = app.update(Message::Run("parcels.check"));
    let _ = app.update(Message::ParcelBoundaries(B::Open));
    let g = app.parcel_boundaries.answer.clone().expect("an answer").expect("read");
    // The last problem (the overlap) chosen in the problems' table.
    let _ = app.update(Message::ParcelBoundaries(B::Show(Show::Problems)));
    let last = g.problems.len() - 1;
    let _ = app.update(Message::ParcelBoundaries(B::Choose(last)));
    assert_eq!(app.parcel_boundaries.chosen, Some(last));
    let _ = app.update(Message::ParcelBoundaries(B::Draw));
    let m = app.parcel_graph_marks().expect("the marks");
    let (pts, name) = m.focus.clone().expect("brought forward");
    let p = g.problems[last].at;
    assert_eq!((pts.len(), name.as_str()), (1, "Çakışan kenar"));
    assert!((pts[0].x - p[0]).abs() < 1e-9 && (pts[0].y - p[1]).abs() < 1e-9);
    // The view goes to it: a box round its place, narrower than the whole graph's.
    let all = g.corners.iter().fold(None::<[f64; 4]>, |b, c| {
        let [x, y] = c.at;
        Some(b.map_or([x, y, x, y], |b| [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)]))
    });
    let focus = crate::parcel_boundaries_marks::Focus::of(&g, Show::Problems, last).expect("the row");
    let b = focus.bounds(&g, all).expect("a box");
    let a = all.expect("the graph's box");
    assert!(b[0] < p[0] && p[0] < b[2] && b[1] < p[1] && p[1] < b[3], "{b:?} {p:?}");
    assert!(b[2] - b[0] < a[2] - a[0], "{b:?} within {a:?}");
    // A boundary, chosen in the boundaries' table: along its curve; another table clears the choice.
    let _ = app.update(Message::Run("parcels.check"));
    let _ = app.update(Message::ParcelBoundaries(B::Open));
    let _ = app.update(Message::ParcelBoundaries(B::Choose(0)));
    let _ = app.update(Message::ParcelBoundaries(B::Show(Show::Corners)));
    assert_eq!(app.parcel_boundaries.chosen, None);
    let _ = app.update(Message::ParcelBoundaries(B::Show(Show::Boundaries)));
    let _ = app.update(Message::ParcelBoundaries(B::Choose(0)));
    let _ = app.update(Message::ParcelBoundaries(B::Draw));
    let m = app.parcel_graph_marks().expect("the marks");
    let (pts, name) = m.focus.clone().expect("brought forward");
    assert_eq!((name.as_str(), pts.len() >= 2), (g.boundaries[0].id.as_str(), true));
    // Without a choice: the whole graph, nothing forward.
    let _ = app.update(Message::Run("parcels.check"));
    let _ = app.update(Message::ParcelBoundaries(B::Open));
    let _ = app.update(Message::ParcelBoundaries(B::Draw));
    assert!(app.parcel_graph_marks().expect("the marks").focus.is_none());
}

/// `.run/gui-evidence/sinir-grafigi-cizimde-2220/`: the window with Çizimde göster, the block with problems on the
/// drawing, and the arc whose top is a corner of the parcels above it.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib parcel_boundaries_window_tests::drawing_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn drawing_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    const ARC: &str = "yaylı sınırda eksik köşe: üstteki iki parselin köşesi alttakinin yayının tepesinde";
    let out =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/sinir-grafigi-cizimde-2220");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        for (case, draw, file) in [(PROBLEMS, false, "1-pencere"), (PROBLEMS, true, "2-sorunlar-cizimde"), (ARC, true, "3-yay-cizimde")] {
            let mut app = opened(Some(case));
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            if case == ARC {
                app.selection.set((1..=3).map(kentos_domain::Slot).collect::<Vec<_>>());
            }
            let _ = app.update(Message::Run("parcels.check"));
            let _ = app.update(Message::ParcelBoundaries(B::Open));
            let _ = app.update(Message::ParcelBoundaries(B::Show(Show::Problems)));
            if draw {
                let _ = app.update(Message::ParcelBoundaries(B::Draw));
                app.selection.clear();
            }
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let path = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&path).expect("writes");
            println!("{}", path.display());
        }
    }
}

/// `.run/gui-evidence/sinir-grafigi-2000/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib parcel_boundaries_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/sinir-grafigi-2000");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        for (case, show, file) in [(None, Show::Boundaries, "1-sinirlar"), (Some(PROBLEMS), Show::Problems, "2-sorunlar")] {
            let mut app = opened(case);
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            let _ = app.update(Message::Run("parcels.check"));
            let _ = app.update(Message::ParcelBoundaries(B::Open));
            let _ = app.update(Message::ParcelBoundaries(B::Show(show)));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let path = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&path).expect("writes");
            println!("{}", path.display());
        }
    }
}

/// `.run/gui-evidence/yay-kiris-2620/`: the three mismatches in the window's problems with their Fark, and on the drawing.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib parcel_boundaries_window_tests::mismatch_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn mismatch_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yay-kiris-2620");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        for (draw, file) in [(false, "1-pencere"), (true, "2-cizimde")] {
            let mut app = opened(Some(MISMATCH));
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            let _ = app.update(Message::Run("parcels.check"));
            let _ = app.update(Message::ParcelBoundaries(B::Open));
            let _ = app.update(Message::ParcelBoundaries(B::Show(Show::Problems)));
            if draw {
                let _ = app.update(Message::ParcelBoundaries(B::Draw));
                app.selection.clear();
            }
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let path = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&path).expect("writes");
            println!("{}", path.display());
        }
    }
}

/// docs/adr/3020: the arc boundary chosen in the table, Çizimde göster: the view on it, the boundary along its curve in
/// the accent with its id; light wide and dark narrow, `.run/gui-evidence/satir-cizimde-3020/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib parcel_boundaries_window_tests::focus_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn focus_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    const ARC: &str = "yaylı sınırda eksik köşe: üstteki iki parselin köşesi alttakinin yayının tepesinde";
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/satir-cizimde-3020");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = opened(Some(ARC));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = app.update(Message::Run("parcels.check"));
        let _ = app.update(Message::ParcelBoundaries(B::Open));
        let g = app.parcel_boundaries.answer.clone().expect("an answer").expect("read");
        let arc = g.boundaries.iter().position(|b| b.bulge != 0.0).expect("an arc boundary");
        let _ = app.update(Message::ParcelBoundaries(B::Choose(arc)));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("pencere-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        let _ = app.update(Message::ParcelBoundaries(B::Draw));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("cizimde-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
