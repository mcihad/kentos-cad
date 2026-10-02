//! The layout kept between runs in the app (docs/adr/0115): read and shown
//! at start, kept as the user changes it, written together a quarter of a
//! second later, a narrower window showing less without losing the wish.

use std::time::Duration;

use iced::Size;
use kentos_ui::theme::typography;
use kentos_ui::widget::docking::{Event as DockEvent, Side, Slot};
use serde_json::{Value, json};

use crate::app::{App, Message, Panel};
use crate::bottom::BottomTab;
use crate::layout::{FILE_NAME, Keeper};
use crate::layout_plan::read_layout;
use crate::processing::panel::Tab as ProcessingTab;

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-yerlesim-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch folder");
    dir
}

/// The app with the layout kept in `dir`, as main.rs opens it.
fn opened(dir: &std::path::Path) -> App {
    let (mut app, _) = App::boot(None);
    app.layout = Keeper::open(dir);
    app.apply_layout();
    app
}

fn dock_width(app: &App) -> f32 {
    typography::scaled(app.docks.size(Side::Right))
}

fn in_front(app: &App) -> Option<Panel> {
    let Some(Slot::Docked(side, i)) = app.docks.slot(Panel::Layers) else {
        return None;
    };
    app.docks.stacks(side).get(i).and_then(|s| s.active())
}

#[test]
fn a_kept_layout_is_shown_at_start() {
    let dir = scratch("acilis");
    std::fs::write(
        dir.join(FILE_NAME),
        json!({
            "dockWidth": 400,
            "layersFraction": 0.35,
            "bottomExpanded": true,
            "bottomHeight": 260,
            "bottomTab": "messages",
            "dockTab": "processing",
            "processingTab": "history",
            "processingFolded": ["points"],
            "ribbonTab": "modify",
            "ribbonCollapsed": true
        })
        .to_string(),
    )
    .expect("a kept layout");
    let app = opened(&dir);
    assert_eq!(dock_width(&app), 400.0);
    let weights: Vec<f32> = app
        .docks
        .stacks(Side::Right)
        .iter()
        .map(|s| s.weight)
        .collect();
    assert!((weights[0] - 0.35).abs() < 1e-6, "{weights:?}");
    assert_eq!(in_front(&app), Some(Panel::Processing));
    assert!(app.command_expanded);
    assert_eq!(app.bottom_tab, BottomTab::Messages);
    assert_eq!(app.bottom_log(), 260.0 - kentos_ui::widget::tabs::height());
    assert_eq!(app.processing.panel.tab, ProcessingTab::History);
    assert!(app.processing.panel.folded.contains("points"));
    assert_eq!(app.tab, "modify");
    assert!(app.ribbon_collapsed);
    assert!(app.right_panel_shown());
    // Nothing changed: nothing is due to be written.
    assert_eq!(app.layout.due(), None);
}

#[test]
fn changes_are_written_together_a_quarter_second_after_the_last() {
    let dir = scratch("yazma");
    // A field the desktop has no part for (the theme is a setting here) is
    // written back as it was read; the web's toolbox is no longer kept
    // (docs/adr/0155).
    std::fs::write(
        dir.join(FILE_NAME),
        json!({ "theme": "light", "ribbonQuickAccess": ["tool.line"], "oldPanel": true, "toolboxX": 240 })
            .to_string(),
    )
    .expect("a kept layout");
    let mut app = opened(&dir);
    let _ = app.update(Message::BottomTab(BottomTab::Coords));
    let _ = app.update(Message::RibbonTab("view"));
    let _ = app.update(Message::Run("view.rightPanel"));
    let due = app.layout.due().expect("a change waits");
    // Not before its time.
    app.layout.write(due - Duration::from_millis(1), false);
    let before = std::fs::read_to_string(dir.join(FILE_NAME)).expect("the old file");
    assert!(!before.contains("coords"));
    app.layout.write(due, false);
    let text = std::fs::read_to_string(dir.join(FILE_NAME)).expect("written");
    let read = read_layout(Some(&text));
    assert_eq!(read["bottomExpanded"], true);
    assert_eq!(read["bottomTab"], "coords");
    assert_eq!(read["ribbonTab"], "view");
    assert_eq!(read["rightVisible"], false);
    assert_eq!(read["theme"], "light");
    assert_eq!(read["ribbonQuickAccess"], json!(["tool.line"]));
    let written: Value = serde_json::from_str(&text).expect("JSON");
    assert!(
        written.get("oldPanel").is_none() && written.get("toolboxX").is_none(),
        "unknown fields are dropped"
    );
    assert_eq!(app.layout.due(), None);
    // Opened again: as it was left, the side panels hidden.
    let again = opened(&dir);
    assert!(!again.right_panel_shown());
    assert_eq!(again.bottom_tab, BottomTab::Coords);
    // F4 while they are hidden: their tab in front comes back too.
    let mut again = again;
    let _ = again.update(Message::Run("view.rightPanel"));
    assert!(again.right_panel_shown());
    assert_eq!(in_front(&again), Some(Panel::Layers));
}

/// Bloklar in front of Katmanlar' slot is kept, and shown so at the next start (docs/adr/0144).
#[test]
fn the_blocks_tab_in_front_is_kept_and_shown_again() {
    let dir = scratch("bloklar");
    let mut app = opened(&dir);
    let _ = app.update(Message::Run("block.panel"));
    assert_eq!(in_front(&app), Some(Panel::Blocks));
    assert_eq!(app.layout.kept()["dockTab"], "blocks");
    let due = app.layout.due().expect("a change waits");
    app.layout.write(due, false);
    assert_eq!(in_front(&opened(&dir)), Some(Panel::Blocks));
}

#[test]
fn a_narrower_window_shows_less_and_keeps_the_wish() {
    let dir = scratch("dar");
    std::fs::write(
        dir.join(FILE_NAME),
        json!({ "dockWidth": 560, "bottomHeight": 600, "bottomExpanded": true }).to_string(),
    )
    .expect("a kept layout");
    let mut app = opened(&dir);
    let bar = kentos_ui::widget::tabs::height();
    assert_eq!(dock_width(&app), 560.0);
    assert_eq!(app.bottom_log(), 540.0 - bar);
    let _ = app.update(Message::WindowResized(Size::new(1100.0, 650.0)));
    assert_eq!(dock_width(&app), 550.0);
    assert_eq!(app.bottom_log(), 390.0 - bar);
    assert_eq!(app.layout.due(), None, "the wish is not changed");
    let _ = app.update(Message::WindowResized(Size::new(1440.0, 900.0)));
    assert_eq!(dock_width(&app), 560.0);
    assert_eq!(app.bottom_log(), 540.0 - bar);
}

#[test]
fn a_dragged_edge_is_kept_by_the_webs_rule() {
    let mut app = opened(&scratch("surukle"));
    // Wider than the most: 560.
    let _ = app.update(Message::Dock(DockEvent::Resized(
        Side::Right,
        typography::unscaled(700.0),
    )));
    assert_eq!(dock_width(&app), 560.0);
    assert_eq!(app.layout.kept()["dockWidth"], 560);
    // The layer tree's share within 0.15–0.85.
    let _ = app.update(Message::Dock(DockEvent::Shared(
        Side::Right,
        vec![0.95, 0.05],
    )));
    assert_eq!(app.layout.kept()["layersFraction"], json!(0.85));
    assert!((app.docks.stacks(Side::Right)[0].weight - 0.85).abs() < 1e-6);
    // A double click puts the first width back, and the equal shares.
    let _ = app.update(Message::Dock(DockEvent::Reset(Side::Right)));
    assert_eq!(dock_width(&app), 312.0);
    assert_eq!(app.layout.kept()["dockWidth"], 312);
    let _ = app.update(Message::Dock(DockEvent::Shared(
        Side::Right,
        vec![0.5, 0.5],
    )));
    assert_eq!(app.layout.kept()["layersFraction"], json!(0.5));
}

/// The layout as it opens: the defaults with the bottom panel open, and a
/// kept one (a wide dock with İşlemler in front, the panel on Uyarılar, the
/// ribbon folded), dark and light, at 1440×900 and 1100×650;
/// `.run/shots/yerlesim-*`:
///
/// ```text
/// cargo test -p kentos-desktop layout_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for scene in ["varsayilan", "kayitli"] {
                let dir = scratch(&format!("resim-{scene}"));
                let kept = match scene {
                    "varsayilan" => json!({ "bottomExpanded": true }),
                    _ => json!({
                        "dockWidth": 480,
                        "layersFraction": 0.4,
                        "dockTab": "processing",
                        "bottomExpanded": true,
                        "bottomHeight": 300,
                        "bottomTab": "messages",
                        "ribbonCollapsed": true
                    }),
                };
                std::fs::write(dir.join(FILE_NAME), kept.to_string()).expect("a kept layout");
                let mut app = crate::files_testing::app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                app.layout = Keeper::open(&dir);
                app.apply_layout();
                let _ = app.update(Message::WindowResized(Size::new(width, height)));
                app.warn("“yollar.shp” içinde alınmayanlar: Kısa halka: 1, üçten az köşesi var; alınmadı.");
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                app.follow.show_fully();
                let file = out.join(format!("yerlesim-{scene}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
