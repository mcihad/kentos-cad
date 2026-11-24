//! Bitişik kenar (docs/adr/8142; PAİY 39/4, BYKHY 25/1, 25/2) worked by hand: a two-storey dwelling building 12 × 5 m
//! whose west walls' outer face stands on the parcel's west edge (x = 0); the 1. kat's west wall holds a window. Parsel
//! makes the west edge, the parcel's 4th, Bitişik (Yan → Ön → Arka → Kamusal → Bitişik): the site plan asks no garden
//! there and the facts carry the walls and the window along it. Denetim: 39/4 fails (the window), 25/1 and 25/2 have
//! missing data (the walls' and the window's classes). The walls declared EI 90 and the window taken away: the three
//! pass. In ayrık nizam the edge is a side garden (0,00 < 3,00 m) and nothing is read along it.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib party_wall_tests::screens -- --ignored --nocapture
//! ```

use kentos_commands::ArchAuditOutcome as O;
use serde_json::json;
use crate::arch::ArchState;

use crate::app::{App, Message};
use crate::use_class_tests::{building, checked, declare, p, rect, run, spaces};
use crate::viewport::Event;

fn click(app: &mut App, x: f64, y: f64) {
    let [sx, sy] = app.viewport.camera.world_to_screen(kentos_interaction::Vec2::new(x, y));
    let at = iced::Point::new(sx as f32, sy as f32);
    let _ = app.update(Message::Viewport(Event::Moved(at)));
    let _ = app.update(Message::Viewport(Event::Pressed(at)));
    let _ = app.update(Message::Viewport(Event::Released(at)));
}

/// The building, its two west walls' and the window's persistent ids; the parcel drawn round it, its west edge on the
/// west walls' outer face; the site plan not yet made.
fn scene() -> (App, Vec<String>, String) {
    let mut app = building(2, "Konut");
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    let mut west = Vec::new();
    let mut window = String::new();
    for level in ["k00", "k01"] {
        let wall = |a: (f64, f64), b: (f64, f64)| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": 0.2, "arch": {"level": level}});
        let walls = run(
            &mut app,
            "cad.walls.create",
            json!({"layerId": layer, "walls": [
                wall((0.1, 0.1), (11.9, 0.1)), wall((11.9, 0.1), (11.9, 4.9)), wall((11.9, 4.9), (0.1, 4.9)),
                wall((0.1, 4.9), (0.1, 0.1))
            ]}),
        );
        let w = walls["output"]["uids"][3].as_str().expect("the west wall").to_owned();
        if level == "k01" {
            let made = run(
                &mut app,
                "cad.openings.create",
                json!({"layerId": layer, "openings": [{"host": w, "at": 2.4, "form": "window", "width": 1.2, "sill": 0.9, "head": 2.2}]}),
            );
            window = made["output"]["uids"][0].as_str().expect("the window").to_owned();
        }
        west.push(w);
        spaces(&mut app, vec![json!({"parts": rect(0.2, 0.2, 11.8, 4.8), "name": "Salon", "unit": "b1", "arch": {"level": level}})]);
    }
    // The parcel: a closed line round the building, south edge first, anticlockwise; its west edge on x = 0.
    run(
        &mut app,
        "cad.entities.create",
        json!({"layerId": layer, "objects": [{"geometry": {"kind": "polyline", "closed": true,
            "pts": [p(0.0, -4.0), p(25.0, -4.0), p(25.0, 14.0), p(0.0, 14.0)]}}]}),
    );
    app.viewport.camera.set_size(1100.0, 600.0);
    app.viewport.camera.fit(&kentos_render_wgpu::Bounds { min_x: -3.0, min_y: -6.0, max_x: 27.0, max_y: 16.0 }, 24.0);
    (app, west, window)
}

/// Parsel: the parcel taken by its south edge, the south edge Ön (one click), the west edge Bitişik (four), written.
fn parcel(app: &mut App) {
    let _ = app.update(Message::Run("tool.siteParcel"));
    click(app, 10.0, -4.0);
    click(app, 10.0, -4.0);
    for _ in 0..4 {
        click(app, 0.0, 5.0);
    }
    let _ = app.submit_line("");
}

/// 39/4's, 25/1's and 25/2's outcomes, in order.
fn rows(app: &App) -> [O; 3] {
    let set = app.modules.arch().arch_audit.rule_set.as_ref().expect("the rule set");
    ["PAİY 39/4", "BYKHY 25/1", "BYKHY 25/2"]
        .map(|rule| set.rules.iter().find(|r| r.rule == rule).unwrap_or_else(|| panic!("{rule}: {:#?}", set.rules)).state)
}

fn said(app: &App, code: &str) -> Vec<String> {
    let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
    audited.findings.iter().filter(|f| f.code == code).map(|f| f.message.clone()).collect()
}

#[test]
fn a_party_edge_s_wall_is_a_fire_wall_without_windows() {
    let (mut app, west, window) = scene();
    parcel(&mut app);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let site = doc.settings().arch.as_ref().and_then(|a| a.site.clone()).expect("the site plan");
    use kentos_commands::SiteEdgeKind::{Front, Party, Side};
    assert_eq!(site.edges.iter().map(|e| e.kind).collect::<Vec<_>>(), [Front, Side, Side, Party]);
    // No garden against the neighbour: the west edge is not short (the footprint, the rooms' outline, is 0.20 m inside
    // the walls' outer face on the edge).
    let report = doc.site_report().expect("read");
    let drawn = report.edges[3].drawn.map(|d| (d * 100.0).round() / 100.0);
    assert_eq!((report.edges[3].required, drawn), (0.0, Some(0.2)));
    assert!(crate::zoning_badge::short_edges(app.document.as_ref().expect("a drawing")).iter().all(|s| !s.starts_with("4.")));
    // The facts: the west wall on each storey, the window on 1. kat.
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    #[allow(clippy::type_complexity)]
    let party: Vec<(usize, Option<&str>, Vec<&str>, Vec<&str>)> = facts
        .party
        .iter()
        .map(|p| {
            (
                p.edge,
                p.level.as_deref(),
                p.walls.iter().map(|w| w.uid.as_str()).collect(),
                p.openings.iter().map(|o| o.kind.as_str()).collect(),
            )
        })
        .collect();
    assert_eq!(
        party,
        [(4, Some("k00"), vec![west[0].as_str()], vec![]), (4, Some("k01"), vec![west[1].as_str()], vec!["window"])]
    );
    checked(&mut app);
    assert_eq!(rows(&app), [O::Failed, O::MissingData, O::MissingData]);
    assert!(
        said(&app, "paiy_party_wall_opening").iter().any(|m| m.starts_with("4. kenar (bitişik), 1. kat: komşu parsel sınırındaki bitişik duvarda 1 pencere")),
        "{:?}",
        said(&app, "paiy_party_wall_opening")
    );
    // The walls declared EI 90, the window taken away: the three pass.
    declare(&mut app, &[&west[0], &west[1]], "EI 90");
    run(&mut app, "cad.entities.delete", json!({"uids": [window]}));
    checked(&mut app);
    assert_eq!(rows(&app), [O::Passed, O::Passed, O::Passed]);
    // In ayrık nizam the edge is a side garden, short; nothing is read along it.
    let mut settings = app.document.as_ref().expect("a drawing").model.settings().arch.clone().expect("Mimari");
    settings.layout = Some(kentos_commands::BuildingLayout::Detached);
    run(&mut app, "cad.settings.set", json!({"arch": settings}));
    let short = crate::zoning_badge::short_edges(app.document.as_ref().expect("a drawing"));
    assert!(short.contains(&"4. kenar (Bitişik (komşu bina)) 0,20 < 3,00 m".to_owned()), "{short:?}");
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert!(kentos_processing::audit_facts::building::measure(doc).party.is_empty());
    checked(&mut app);
    assert_eq!(rows(&app), [O::OutOfScope, O::OutOfScope, O::OutOfScope]);
}

/// Parsel with the west edge Bitişik, and Denetim's 39/4 finding; light and dark; `.run/gui-evidence/bitisik-8142/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/bitisik-8142");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")] {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let (mut app, ..) = scene();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        // Parsel open: the west edge turned over to Bitişik, not yet written.
        let _ = app.update(Message::Run("tool.siteParcel"));
        click(&mut app, 10.0, -4.0);
        click(&mut app, 10.0, -4.0);
        for _ in 0..4 {
            click(&mut app, 0.0, 5.0);
        }
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-parsel-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        let _ = app.submit_line("");
        checked(&mut app);
        let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
        app.modules.arch_mut().arch_audit.selected = audited.findings.iter().position(|f| f.code == "paiy_party_wall_opening");
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-denetim-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
