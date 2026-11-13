//! The selection's contextual tab for curved objects and the selection tools that do not take them (usage review
//! UR-007): a lone circle's tab is Daire, an ellipse's Elips, an open elliptical arc opens the curves' tab as
//! Eliptik yay; Köşe ekle/sil and Kot ver are off, with their reason, when nothing selected is theirs.
//! `selection_fit_screens` draws a circle's tab:
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib selection_fit_tests::selection_fit_screens -- --ignored --nocapture
//! ```

use std::f64::consts::PI;

use kentos_domain::Slot;

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::tools_scenes::{Objects, open};

/// A circle (1), a full ellipse (2), a half ellipse (3), a closed area (4) and a point (5).
fn drawing() -> App {
    let (mut app, _) = App::boot(None);
    let mut o = Objects::new();
    o.circle("cizim", [0.0, 0.0], 2.0);
    o.ellipse("cizim", [10.0, 0.0], [3.0, 0.0], 0.5, 0.0, 0.0);
    o.ellipse("cizim", [20.0, 0.0], [3.0, 0.0], 0.5, 0.0, PI);
    o.path("parsel", &[[30.0, 0.0], [34.0, 0.0], [34.0, 3.0], [30.0, 3.0]], true);
    o.point("cizim", [40.0, 0.0], None);
    open(&mut app, o);
    make_cad(&mut app);
    app
}

fn select(app: &mut App, ids: &[u32]) {
    app.selection.set(ids.iter().map(|&i| Slot(i)));
}

/// The selection's tab: its id and name.
fn tab(app: &App) -> (&'static str, &'static str) {
    let tab = app.selection_tab().expect("a contextual tab");
    (tab.id, app.selection_tab_title(tab))
}

#[test]
fn a_curved_selection_names_its_tab_by_its_kind() {
    let mut app = drawing();
    select(&mut app, &[1]);
    assert_eq!(tab(&app), ("ctx.area", "Daire"));
    select(&mut app, &[2]);
    assert_eq!(tab(&app), ("ctx.area", "Elips"));
    select(&mut app, &[3]);
    assert_eq!(tab(&app), ("ctx.line", "Eliptik yay"), "an open arc is a curve, not an area");
    select(&mut app, &[1, 2]);
    assert_eq!(tab(&app), ("ctx.area", "Alan"), "two kinds: the tab's own name");
    select(&mut app, &[4]);
    assert_eq!(tab(&app), ("ctx.area", "Alan"));
}

#[test]
fn a_tool_that_takes_none_of_the_selection_is_off_and_says_why() {
    let mut app = drawing();
    for ids in [&[1][..], &[2], &[1, 2]] {
        select(&mut app, ids);
        for tool in ["tool.vertex", "tool.setElevation"] {
            assert!(!app.available(tool), "{tool} on {ids:?}");
        }
        assert!(app.available("tool.areaUnion"), "an area tool takes circles");
    }
    select(&mut app, &[1]);
    assert_eq!(
        app.why_disabled("tool.vertex").as_deref(),
        Some("Köşe ekle/sil seçili daire için çalışmaz; çizgi, çoklu çizgi ya da kapalı alan seçin.")
    );
    select(&mut app, &[1, 2]);
    assert_eq!(
        app.why_disabled("tool.setElevation").as_deref(),
        Some("Kot ver seçili daire, elips için çalışmaz; çizgi, çoklu çizgi, alan ya da nokta seçin.")
    );
    // Something that takes it, alone or beside a circle; nothing selected: the tool picks its own.
    for ids in [&[4][..], &[1, 4], &[]] {
        select(&mut app, ids);
        assert!(app.available("tool.vertex"), "{ids:?}");
        assert!(app.available("tool.setElevation"), "{ids:?}");
    }
    select(&mut app, &[5]);
    assert!(!app.available("tool.vertex") && app.available("tool.setElevation"), "a point takes an elevation");
}

/// UR-007's pictures (`.run/gui-evidence/kullanim-UR-007/`): a circle's tab, light and dark.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn selection_fit_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kullanim-UR-007");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let mut app = drawing();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::Run("view.zoomExtents"));
        select(&mut app, &[1]);
        let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("daire-sekmesi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
    // UR-010: Değiştir opened, then the circle selected: Değiştir keeps the front, Daire beside it.
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kullanim-UR-010");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let mut app = drawing();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::RibbonTab("modify"));
        select(&mut app, &[1]);
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let _ = app.update(Message::RibbonPanel(crate::ribbon_panels::Event::Color(None)));
        assert_eq!(app.shown_tab(), "modify");
        let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("degistir-acik-kalir-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// UR-009: the linear dimension's words in the menu and the command's description name no axis (a CBS project's
/// ΔY is a CAD project's ΔX).
#[test]
fn the_linear_dimensions_menu_words_name_no_axis() {
    let catalog = crate::catalog::catalog();
    let linear = catalog.method_by_alias("DIMLIN").expect("Doğrusal");
    assert_eq!(linear.description, Some("Yatay ya da düşey"));
    let description = catalog.get("tool.dimension").expect("Ölçülendirme").description;
    assert!(description.contains("doğrusal (yatay ya da düşey)") && !description.contains('Δ'), "{description}");
}

/// UR-038: a point cloud chosen with an area (the input its statistics read) keeps the cloud's tab, so its commands
/// stay one click away; a cloud with another kind goes to Seçim as before.
#[test]
fn a_cloud_with_its_inputs_keeps_the_clouds_tab() {
    let (mut app, _) = App::boot(None);
    crate::cloud_scenes::opened(&mut app, [-40.0, -40.0, 260.0, 220.0]);
    app.selection.set([Slot(1)]);
    assert_eq!(app.selection_tab().map(|t| t.id), Some("ctx.pointcloud"));
    app.selection.set([Slot(1), Slot(2)]);
    assert_eq!(app.selection_tab().map(|t| t.id), Some("ctx.pointcloud"), "the cloud and an area");
}

/// UR-039: a column placed with a point cloud's layer active, on the cloud, at the project's large coordinates: the
/// click is answered at once (the window stays responsive); run under a time limit.
#[test]
fn a_column_over_a_point_cloud_is_answered_at_once() {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let (mut app, _) = App::boot(None);
            crate::cloud_scenes::opened(&mut app, [-40.0, -40.0, 260.0, 220.0]);
            make_cad(&mut app);
            app.draft.snap = false;
            assert!(app.document.as_mut().expect("open").model.set_active_layer("koy"));
            let started = std::time::Instant::now();
            let _ = app.update(Message::Run("tool.column"));
            let at = crate::cloud_scenes::X0 + 100.0;
            let at = (at, crate::cloud_scenes::Y0 + 90.0);
            let [x, y] = app.viewport.camera.world_to_screen(kentos_render_wgpu::Vec2::new(at.0, at.1));
            let p = iced::Point::new(x as f32, y as f32);
            let _ = app.update(Message::Viewport(crate::viewport::Event::Moved(p)));
            let _ = app.update(Message::Viewport(crate::viewport::Event::Pressed(p)));
            let _ = app.update(Message::Viewport(crate::viewport::Event::Released(p)));
            let said = crate::files_testing::last_said(&app);
            let _ = tx.send((started.elapsed(), said));
        })
        .expect("a thread");
    let (took, said) = rx.recv_timeout(std::time::Duration::from_secs(20)).expect("the click answered within 20 s");
    assert!(took < std::time::Duration::from_secs(5), "{took:?}: {said}");
    assert_eq!(said, "1 kolon konuldu.");
}
