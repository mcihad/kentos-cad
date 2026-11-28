//! Each program keeps its own layout file (docs/adr/0802): KentOS Harita's change is in its own file, not in kentos-cad's
//! nor in KentOS Mimari's, which starts from kentos-cad's. Moved from the core's layout_tests.rs: it spans the modules
//! (docs/adr/9199 §4.11).

use crate::app::{App, Message};
use crate::bottom::BottomTab;
use crate::layout::{FILE_NAME, Keeper};
use crate::layout_plan::read_layout;
use kentos_ui::theme::typography;
use kentos_ui::widget::docking::Side;
use serde_json::json;

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-yerlesim-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch folder");
    dir
}

fn dock_width(app: &App) -> f32 {
    typography::scaled(app.docks.size(Side::Right))
}

/// Each program keeps its own layout (docs/adr/0802): a module's program run for the first time starts from
/// kentos-cad's sizes and writes only its own file; kentos-cad's is left as it was.
#[test]
fn each_program_keeps_its_own_layout_file() {
    use kentos_launch::Module;
    let dir = scratch("programlar");
    std::fs::write(
        dir.join(FILE_NAME),
        json!({ "dockWidth": 400, "bottomTab": "coords", "ribbonQuickAccess": ["tool.line"] })
            .to_string(),
    )
    .expect("kentos-cad's layout");
    assert_eq!(crate::layout::file_name(None), "yerlesim.json");
    assert_eq!(
        crate::layout::file_name(Some(Module::Map)),
        "yerlesim-harita.json"
    );
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    app.layout = Keeper::open_for(&dir, Some(Module::Map));
    app.apply_layout();
    assert_eq!(
        dock_width(&app),
        400.0,
        "the first run starts from kentos-cad's sizes"
    );
    let _ = app.update(Message::BottomTab(BottomTab::Messages));
    let due = app.layout.due().expect("a change waits");
    app.layout.write(due, false);
    let own = std::fs::read_to_string(dir.join("yerlesim-harita.json")).expect("Harita's own file");
    assert_eq!(read_layout(Some(&own))["bottomTab"], "messages");
    let shared = std::fs::read_to_string(dir.join(FILE_NAME)).expect("kentos-cad's file");
    assert_eq!(
        read_layout(Some(&shared))["bottomTab"],
        "coords",
        "kentos-cad's is not touched"
    );
    // KentOS Mimari has its own too: Harita's change is not in it.
    let mimari = Keeper::open_for(&dir, Some(Module::Architecture));
    assert!(!dir.join("yerlesim-mimari.json").exists());
    drop(mimari);
    let (mut other, _) = App::boot(None);
    other.program = Some(Module::Architecture);
    other.layout = Keeper::open_for(&dir, Some(Module::Architecture));
    other.apply_layout();
    assert_eq!(
        other.bottom_tab,
        BottomTab::Coords,
        "kentos-cad's, not Harita's"
    );
}
