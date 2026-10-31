//! The pictures of docs/adr/0791 (the office material library, the wall style's material picker, a slab bound to a
//! space from Öznitelikler): the materials' shared drawing (`fixtures/commands/v1/cad.materials.edit.json` `setup`);
//! Malzemeler › Kitaplık with KentOS's library, the new materials ticked, the project's kept; every material ticked
//! with Kitaplığınkiyle değiştir and the plan counting what follows; the materials taken; Duvar stilleri with the brick
//! core bound to Tuğla (read-only hatch and pen) and the insulation bound to Taşyünü in the draft; and a slab of two
//! rooms bound to 101 Salon from its Sınır row. `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=malzeme-kitaplik,malzeme-kitaplik-degistir,malzeme-kitaplik-alindi,malzeme-stil-secici,doseme-mahal-satiri cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use kentos_ui::widget::docking;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::materials_window::{Conflict, Event, Tab};
use crate::tools_scenes::run;
use crate::tools_screens::Scene;

const CASES: &str = include_str!("../../../fixtures/commands/v1/cad.materials.edit.json");
const SLABS: &str = include_str!("../../../fixtures/commands/v1/cad.slabs.bound.json");

pub(crate) fn open(app: &mut App, doc: Value, bounds: [f64; 4]) {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(doc).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    make_cad(app);
    app.draft.snap = false;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds { min_x: bounds[0], min_y: bounds[1], max_x: bounds[2], max_y: bounds[3] },
        24.0,
    );
}

fn event(app: &mut App, e: Event) {
    let _ = app.update(Message::Materials(e));
}

fn drawing(app: &mut App) {
    let file: Value = serde_json::from_str(CASES).expect("the cases");
    open(app, file["setup"].clone(), [-1.0, -1.0, 45.0, 9.5]);
}

/// Malzemeler › Kitaplık: KentOS's library, its new materials ticked, the project's Tuğla and Betonarme kept.
pub(crate) fn library(app: &mut App) {
    drawing(app);
    run(app, "arch.materials");
    event(app, Event::Tab(Tab::Library));
}

/// Every material ticked, Kitaplığınkiyle değiştir: the plan counts those taken, the two replaced and what follows them.
fn replace(app: &mut App) {
    library(app);
    event(app, Event::PickAll(true));
    event(app, Event::Conflict(Conflict::Replace));
}

/// Seçilenleri al with the project's kept, then the project's materials: Tuğla and Betonarme, then KentOS's.
fn taken(app: &mut App) {
    library(app);
    event(app, Event::Take);
    event(app, Event::Tab(Tab::Project));
}

/// Duvar stilleri, Dış duvar: its core bound to Tuğla (hatch and pen read-only), its insulation bound to Taşyünü taken
/// from KentOS's library, in the draft.
fn style_picker(app: &mut App) {
    library(app);
    event(app, Event::PickAll(false));
    event(app, Event::Pick(7));
    event(app, Event::Take);
    event(app, Event::Close);
    run(app, "arch.wallStyles");
    let _ = app.update(Message::WallStyles(crate::wall_styles::Event::Material(2, Some("tasyunu".into()))));
}

/// Two rooms of one level, their spaces 101 Salon and 102 Mutfak, a slab bound to the kitchen's walls rebound to the
/// salon from its Sınır row; Öznitelikler says it.
pub(crate) fn slab_row(app: &mut App) {
    let file: Value = serde_json::from_str(SLABS).expect("the slab cases");
    let mut doc = file["setup"].clone();
    doc["version"] = json!(2);
    doc["uids"] = json!((1..=7).map(|i| format!("0192f5a1-7850-7000-8000-{i:012x}")).collect::<Vec<_>>());
    open(app, doc, [-1.0, -1.5, 9.0, 6.5]);
    let model = &mut app.document.as_mut().expect("the drawing").model;
    let spaces: kentos_commands::SpacesCreate = serde_json::from_value(json!({"layerId": "mahal", "spaces": [
        {"seed": {"x": 2.5, "y": 2.5}, "number": "101", "name": "Salon", "arch": {"level": "zemin"}},
        {"seed": {"x": 6.5, "y": 2.5}, "number": "102", "name": "Mutfak", "arch": {"level": "zemin"}}]}))
    .expect("spaces");
    let made = kentos_native_application::spaces_create::execute(
        &mut kentos_native_application::ExecutionContext::new(model),
        spaces,
    );
    assert!(matches!(made, kentos_commands::CommandResult::Completed { .. }), "{made:?}");
    let slab: kentos_commands::SlabsCreate = serde_json::from_value(json!({"layerId": "doseme", "slabs": [
        {"seed": {"x": 6.5, "y": 2.5}, "bound": true, "boundary": "room", "arch": {"level": "zemin"}}]}))
    .expect("slab");
    let made = kentos_native_application::slabs_create::execute(
        &mut kentos_native_application::ExecutionContext::new(model),
        slab,
    );
    assert!(matches!(made, kentos_commands::CommandResult::Completed { .. }), "{made:?}");
    let slab = model
        .entities()
        .find(|e| matches!(e, kentos_commands::Entity::Slab(_)))
        .map(|e| Slot(e.base().id))
        .expect("the slab");
    let salon = model
        .entities()
        .find(|e| matches!(e, kentos_commands::Entity::Space(s) if s.space.number.as_deref() == Some("101")))
        .and_then(|e| model.uid(Slot(e.base().id)))
        .expect("the salon")
        .to_string();
    app.selection.set([slab]);
    let bind = kentos_commands::SlabsOperation::Bind { seed: None, boundary: None, space: Some(salon), offset: None };
    let _ = app.update(Message::Properties(crate::properties::Event::Slab(slab, bind)));
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle("general")));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("malzeme-kitaplik", library),
        ("malzeme-kitaplik-degistir", replace),
        ("malzeme-kitaplik-alindi", taken),
        ("malzeme-stil-secici", style_picker),
        ("doseme-mahal-satiri", slab_row),
    ]
}
