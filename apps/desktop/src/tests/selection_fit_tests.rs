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
