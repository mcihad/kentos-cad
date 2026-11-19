//! Çıkmalar (docs/adr/8152; PAİY 41/1, 41/2) worked by hand: a two-storey dwelling, Zemin kat's Salon 10 × 8 m (x
//! 0–10, y 0–8), 1. kat's Salon over it and its Balkon (an outdoor use) 2 m deep south of it (y −2–0, x 2–8). The
//! parcel x −3–13, y −5–11, its south edge Ön (5.00 m: the setback line on the Salon's south face), the others Yan
//! (3.00 m).
//!
//! - The Balkon is a projection (outside the footprint, on a storey over the ground's): 3.00 m from the south edge,
//!   past the setback line by 5.00 − 3.00 = 2.00 m > 1.50 m: 41/1 fails. No slab under it: 41/2 lacks its underside
//!   (its storey at 3.00 m ≥ 2.40 m).
//! - The Balkon 1.20 m deep (y −1.2–0) and a 15 cm slab under it on 1. kat: past the setback line by 1.20 m, its
//!   underside 3.00 − 0.15 = 2.85 m: both hold.

use kentos_commands::ArchAuditOutcome as O;
use serde_json::json;

use crate::app::{App, Message};
use crate::use_class_tests::{building, checked, p, rect, run, spaces};
use crate::viewport::Event;

fn click(app: &mut App, x: f64, y: f64) {
    let [sx, sy] = app.viewport.camera.world_to_screen(kentos_interaction::Vec2::new(x, y));
    let at = iced::Point::new(sx as f32, sy as f32);
    let _ = app.update(Message::Viewport(Event::Moved(at)));
    let _ = app.update(Message::Viewport(Event::Pressed(at)));
    let _ = app.update(Message::Viewport(Event::Released(at)));
}

/// The dwelling with its Balkon `depth` deep, the parcel drawn round it and taken in Parsel (its south edge Ön); the
/// Balkon's persistent id.
pub(crate) fn scene(depth: f64) -> (App, String) {
    let mut app = building(2, "Konut");
    run(
        &mut app,
        "cad.program.edit",
        json!({"program": {
            "buildings": [{"id": "a", "name": "A Blok", "levels": ["k00", "k01"], "usage": "Konut"}],
            "units": [{"id": "b1", "number": "1", "nature": "Konut"}],
            "briefs": [{"usage": "Balkon", "outdoor": true}]
        }}),
    );
    spaces(
        &mut app,
        vec![
            json!({"parts": rect(0.0, 0.0, 10.0, 8.0), "name": "Salon", "unit": "b1", "arch": {"level": "k00"}}),
            json!({"parts": rect(0.0, 0.0, 10.0, 8.0), "name": "Salon", "unit": "b1", "arch": {"level": "k01"}}),
        ],
    );
    let balkon = balcony(&mut app, depth);
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    run(
        &mut app,
        "cad.entities.create",
        json!({"layerId": layer, "objects": [{"geometry": {"kind": "polyline", "closed": true,
            "pts": [p(-3.0, -5.0), p(13.0, -5.0), p(13.0, 11.0), p(-3.0, 11.0)]}}]}),
    );
    app.viewport.camera.set_size(1100.0, 600.0);
    app.viewport.camera.fit(&kentos_render_wgpu::Bounds { min_x: -6.0, min_y: -8.0, max_x: 16.0, max_y: 14.0 }, 24.0);
    let _ = app.update(Message::Run("tool.siteParcel"));
    click(&mut app, 5.0, -5.0);
    click(&mut app, 5.0, -5.0);
    let _ = app.submit_line("");
    (app, balkon)
}

/// The Balkon `depth` deep on 1. kat; its persistent id.
fn balcony(app: &mut App, depth: f64) -> String {
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    let made = run(
        app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [{"parts": rect(2.0, -depth, 8.0, 0.0), "name": "Balkon", "usage": "Balkon",
            "unit": "b1", "arch": {"level": "k01"}}]}),
    );
    made["output"]["uids"][0].as_str().expect("the Balkon").to_owned()
}

/// 41/1's and 41/2's rows.
fn rows(app: &App) -> [O; 2] {
    let set = app.arch.arch_audit.rule_set.as_ref().expect("the rule set");
    ["PAİY 41/1", "PAİY 41/2"].map(|rule| {
        set.rules.iter().find(|r| r.rule == rule).unwrap_or_else(|| panic!("{rule}: {:#?}", set.rules)).state
    })
}

fn said(app: &App, code: &str) -> Vec<String> {
    let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
    audited.findings.iter().filter(|f| f.code == code).map(|f| f.message.clone()).collect()
}

#[test]
fn a_balcony_past_the_setback_line_by_more_than_1_50_m_fails_41() {
    let (mut app, balkon) = scene(2.0);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let site = doc.settings().arch.as_ref().and_then(|a| a.site.clone()).expect("the site plan");
    use kentos_commands::SiteEdgeKind::{Front, Side};
    // Only the edges set are kept; the others are Yan.
    assert_eq!((0..4).map(|i| site.edge(i).kind).collect::<Vec<_>>(), [Front, Side, Side, Side]);
    // The domain: the Balkon alone, outside the footprint, 3.00 m from the south edge, no slab under it.
    let found = doc.projections();
    assert_eq!(found.len(), 1, "{found:#?}");
    let b = &found[0];
    assert_eq!((b.room.as_str(), b.open, b.underside), (balkon.as_str(), true, None));
    assert!((b.area - 12.0).abs() < 1e-6, "{}", b.area);
    let front = b.edges.iter().find(|e| e.kind == Front).expect("the south edge");
    assert!((front.required - 5.0).abs() < 1e-9 && (front.distance - 3.0).abs() < 1e-6, "{front:?}");
    // The facts carry it; Denetim: 41/1 fails, 41/2 lacks the underside.
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    assert_eq!(facts.projections.len(), 1);
    checked(&mut app);
    assert_eq!(rows(&app), [O::Failed, O::MissingData]);
    let over = said(&app, "paiy_projection_over");
    assert_eq!(over.len(), 1, "{over:?}");
    let words = "1. kat: açık çıkma (12.00 m²): 1. kenar (ön bahçe): yapı yaklaşma sınırından 2.00 m taşıyor";
    assert!(over[0].starts_with(words), "{}", over[0]);
    // 1.20 m deep and a 15 cm slab under it: both hold.
    run(&mut app, "cad.entities.delete", json!({"uids": [balkon]}));
    let _ = balcony(&mut app, 1.2);
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    run(
        &mut app,
        "cad.slabs.create",
        json!({"layerId": layer, "slabs": [{"parts": rect(2.0, -1.2, 8.0, 0.0), "thickness": 0.15,
            "arch": {"level": "k01"}}]}),
    );
    let doc = &app.document.as_ref().expect("a drawing").model;
    let found = doc.projections();
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].underside.is_some_and(|u| (u - 2.85).abs() < 1e-6), "{:?}", found[0].underside);
    checked(&mut app);
    assert_eq!(rows(&app), [O::Passed, O::Passed]);
}

/// The site plan's projection outlines (PAİY 57/3, docs/adr/8155): the closed dashed lines carrying `Vaziyet`, each
/// with its corners sorted.
fn outlines(app: &App) -> Vec<Vec<(i64, i64)>> {
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.entities()
        .filter(|e| e.base().attrs.get("Vaziyet").map(String::as_str) == Some("Çıkma izdüşümü"))
        .map(|e| {
            let kentos_commands::Entity::Polyline(l) = e else { panic!("a projection's outline is a polyline: {e:?}") };
            assert!(l.closed && l.base.look.line_type == Some(kentos_commands::LineType::Dashed), "{l:?}");
            let mut corners: Vec<(i64, i64)> =
                l.pts.iter().map(|p| ((p.x * 100.0).round() as i64, (p.y * 100.0).round() as i64)).collect();
            corners.sort_unstable();
            corners
        })
        .collect()
}

#[test]
fn parsel_draws_the_projections_outlines_with_the_site_plan() {
    let (mut app, _) = scene(2.0);
    // The Balkon's part outside the footprint: x 2–8, y −2–0.
    assert_eq!(outlines(&app), [vec![(200, -200), (200, 0), (800, -200), (800, 0)]]);
    // Parsel again: the old outline goes with the old marks, one drawn again.
    let _ = app.update(Message::Run("tool.siteParcel"));
    let _ = app.submit_line("");
    assert_eq!(outlines(&app).len(), 1);
}

/// Denetim's 41/1 finding beside 1. kat's plan with the parcel, light and dark; `.run/gui-evidence/cikma-8152/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib projection_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/cikma-8152");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")] {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let (mut app, _) = scene(2.0);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::ActiveLevel(Some("k01".into())));
        checked(&mut app);
        let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
        app.arch.arch_audit.selected = audited.findings.iter().position(|f| f.code == "paiy_projection_over");
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-cikma-denetim-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// The site plan with the setback line and the Balkon's projection outline (PAİY 57/3), light and dark;
/// `.run/gui-evidence/cikma-izdusumu-8155/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn site_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/cikma-izdusumu-8155");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")] {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let (mut app, _) = scene(2.0);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::ActiveLevel(Some("k01".into())));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-vaziyet-cikma-izdusumu-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
