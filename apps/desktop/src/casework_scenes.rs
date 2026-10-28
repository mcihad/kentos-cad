//! The pictures of docs/adr/0727 (köşeli mutfak dizileri): a 4.20 × 3.00 m kitchen of four walls with no cabinet block
//! of its own. Mutfak dizisi's Biçim L under the cursor after its corner; the L written with a blind corner and wall
//! cabinets, its legs tied to the north and east walls; the same L with an open corner; a U along three walls with the
//! dishwasher beside the sink, an oven under the cooktop and the fridge at its start; a cabinet of the L selected, the
//! run's section in Öznitelikler (Dizi, Köşe, Tezgâh derinliği, Modül, Tür, Modül eni, Açılma); its module made 90 cm
//! from that section, the run laid anew; the north wall moved 40 cm, the run following it; a run before a door, the
//! clash in its preview's tag:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=mutfak-l-onizleme,mutfak-l-kor-kose,mutfak-l-acik-kose,mutfak-u-cihazlar,mutfak-oznitelikler,mutfak-modul-eni,mutfak-duvar-izler,mutfak-kapi-cakismasi cargo test -p kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::{Slot, Uuid};
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::properties::Event;
use crate::tools_scenes::{E, N, Objects, hover, layer, open_edited, run, typed};
use crate::tools_screens::Scene;

fn xy(x: f64, y: f64) -> Value {
    json!({ "x": E + x, "y": N + y })
}

fn command(app: &mut App, name: &str, input: Value) {
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        name,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
}

/// A point of the run typed: exact, as a click's screen rounding (some 1e-7 m) is not where a leg takes its modules to
/// the hair (docs/adr/0727 §1).
fn put(app: &mut App, at: [f64; 2]) {
    typed(app, &format!("{},{}", E + at[0], N + at[1]));
}

/// The kitchen: its walls' inner faces at x 0 and 4.20, y 0 and 3.00, each wall its own (north, west, east, south).
fn room(app: &mut App) {
    let mut o = Objects::new();
    for pts in [
        [(-0.125, 3.125), (4.325, 3.125)],
        [(-0.125, -0.125), (-0.125, 3.125)],
        [(4.325, 3.125), (4.325, -0.125)],
        [(4.325, -0.125), (-0.125, -0.125)],
    ] {
        o.push(
            "duvar",
            json!({
                "kind": "wall", "pts": pts.iter().map(|&(x, y)| xy(x, y)).collect::<Vec<_>>(),
                "width": 0.25, "justify": "center",
            }),
        );
    }
    o.text("cizim", [1.5, 1.2], "MUTFAK", 0.16, 0.0);
    open_edited(app, o, |d| {
        d["layers"] = json!([
            layer("cizim", "Çizim", "fg", 0.25),
            layer("duvar", "Duvar", "fg", 0.5),
            layer("mutfak", "Mutfak", "#C8A165", 0.25),
        ]);
        d["settings"]["plotScale"] = json!(50);
    });
    make_cad(app);
    if let Some(doc) = app.document.as_mut() {
        doc.model.set_active_layer("mutfak");
    }
    app.draft.snap = false;
    app.command_expanded = true;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 0.8,
            min_y: N - 0.9,
            max_x: E + 5.0,
            max_y: N + 3.8,
        },
        24.0,
    );
}

/// Biçim L: the start at the west wall, the corner at the east one, the cursor down the east wall.
fn ell_preview(app: &mut App) {
    room(app);
    run(app, "tool.casework");
    typed(app, "B");
    put(app, [0.0, 3.0]);
    put(app, [4.2, 3.0]);
    hover(app, [4.2, 0.9]);
}

/// The L written with a blind corner and wall cabinets: north wall from the west corner, down the east wall to 0.60.
fn ell_blind(app: &mut App) {
    room(app);
    run(app, "tool.casework");
    typed(app, "U");
    typed(app, "B");
    put(app, [0.0, 3.0]);
    put(app, [4.2, 3.0]);
    put(app, [4.2, 0.6]);
    hover(app, [2.0, 0.6]);
}

/// The same L with an open corner: both legs stop the countertop's depth short of the corner.
fn ell_open(app: &mut App) {
    room(app);
    run(app, "tool.casework");
    typed(app, "B");
    typed(app, "K");
    put(app, [0.0, 3.0]);
    put(app, [4.2, 3.0]);
    put(app, [4.2, 0.6]);
    hover(app, [2.0, 0.6]);
}

/// A U along the west, north and east walls: the dishwasher beside the sink, an oven under the cooktop, the fridge at
/// its start; the west leg holds its modules to the hair (0.60 + 0.90 + 0.60 after the fridge).
fn you(app: &mut App) {
    room(app);
    run(app, "tool.casework");
    typed(app, "B");
    typed(app, "B");
    typed(app, "L");
    typed(app, "Otomatik");
    typed(app, "F");
    typed(app, "Z");
    put(app, [0.0, 0.3]);
    put(app, [0.0, 3.0]);
    put(app, [4.2, 3.0]);
    put(app, [4.2, 0.3]);
    hover(app, [2.0, 0.2]);
}

/// The slots of the drawing's first run, in its order.
fn members(app: &App) -> Vec<Slot> {
    let model = &app.document.as_ref().expect("a drawing").model;
    model.casework()[0]
        .members
        .iter()
        .filter_map(|m| model.slot_of(Uuid::from_bytes(m.uid.0)))
        .collect()
}

/// The slot of the first run's object of `part` (and number).
fn member(app: &App, part: kentos_commands::CaseworkPart, index: Option<u32>) -> Slot {
    let model = &app.document.as_ref().expect("a drawing").model;
    let m = model.casework()[0]
        .members
        .iter()
        .find(|m| m.part == part && m.index == index)
        .expect("the run's object");
    model.slot_of(Uuid::from_bytes(m.uid.0)).expect("its slot")
}

/// The blind-corner L written and the tool left, its first module's cabinet selected: the run's section, the layers
/// folded so that it stands whole.
fn selected(app: &mut App) {
    use kentos_ui::widget::docking;
    ell_blind(app);
    typed(app, "");
    let slot = member(app, kentos_commands::CaseworkPart::Module, Some(1));
    app.selection.set([slot]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
    hover(app, [2.0, 0.6]);
}

/// Modül eni 90 cm picked in the section: the run laid anew, the first module wider, the filler gone.
fn widened(app: &mut App) {
    selected(app);
    let slot = member(app, kentos_commands::CaseworkPart::Module, Some(1));
    let _ = app.update(Message::Properties(Event::CaseworkValue(slot, "width", "0.9".into())));
    let slot = member(app, kentos_commands::CaseworkPart::Module, Some(1));
    app.selection.set([slot]);
}

/// The north wall moved 40 cm into the room: the L's north leg and its corner follow, its east leg keeps its length.
fn wall_moved(app: &mut App) {
    ell_blind(app);
    north_moved(app);
}

/// The tool left and the north wall moved 40 cm into the room; the run selected.
fn north_moved(app: &mut App) {
    typed(app, "");
    let model = &app.document.as_ref().expect("a drawing").model;
    let north = model.uid(Slot(1)).expect("the north wall").to_string();
    command(
        app,
        "cad.entities.transform",
        json!({"uids": [north], "transform": {"kind": "move", "dx": 0.0, "dy": -0.4}}),
    );
    app.selection.set(members(app));
    hover(app, [2.0, 0.4]);
}

/// A door on the north wall, then the L along it before its end is clicked: “1. kol K1 kapısının önüne geliyor” in the
/// preview's tag.
fn door(app: &mut App) {
    room(app);
    let model = &app.document.as_ref().expect("a drawing").model;
    let north = model.uid(Slot(1)).expect("the north wall").to_string();
    command(
        app,
        "cad.openings.create",
        json!({"layerId": "duvar", "openings": [{"host": north, "at": 1.4, "number": "K1"}]}),
    );
    run(app, "tool.casework");
    typed(app, "B");
    put(app, [0.0, 3.0]);
    put(app, [4.2, 3.0]);
    hover(app, [4.2, 0.6]);
}

/// The selected cabinet's section scrolled into view.
pub(crate) fn pointed() -> Vec<crate::tools_screens::Pointed> {
    fn scroll(s: &mut kentos_ui::snapshot::Snapshot, app: &mut App) {
        let mut update = |app: &mut App, message: Message| {
            let _ = app.update(message);
        };
        let at = crate::tools_screens::caption_at(s, app, "Genel", false);
        s.input(app, App::view, &mut update, kentos_ui::snapshot::Input::Scroll(at, -9.0));
    }
    vec![("mutfak-oznitelikler", selected, scroll), ("mutfak-modul-eni", widened, scroll)]
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("mutfak-l-onizleme", ell_preview),
        ("mutfak-l-kor-kose", ell_blind),
        ("mutfak-l-acik-kose", ell_open),
        ("mutfak-u-cihazlar", you),
        ("mutfak-duvar-izler", wall_moved),
        ("mutfak-kapi-cakismasi", door),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files_testing::app_with_drawing;
    use kentos_commands::{CaseworkCorner, CaseworkModuleKind, CaseworkPart};

    fn record(app: &App) -> kentos_commands::CaseworkRecord {
        let model = &app.document.as_ref().expect("a drawing").model;
        assert_eq!(model.casework().len(), 1, "one run");
        (*model.casework()[0]).clone()
    }

    /// The walkthrough as the pictures take it (docs/adr/0727): the L is one run tied to the north and east walls, its
    /// corner blind (or open); the U holds a dishwasher, an oven and a fridge; Modül eni lays it anew; the north wall
    /// moved takes the run with it; a door before the run is said.
    #[test]
    fn kitchen_runs_are_laid_as_the_pictures_show() {
        let mut app = app_with_drawing();
        ell_blind(&mut app);
        let r = record(&app);
        assert_eq!(r.recipe.through.as_ref().map(Vec::len), Some(1), "an L");
        assert_eq!(r.recipe.upper, Some(true));
        assert!(r.ties.iter().all(Option::is_some), "both legs tied: {:?}", r.ties);
        assert!(r.members.iter().any(|m| m.part == CaseworkPart::Corner), "a blind corner's filler");

        let mut app = app_with_drawing();
        ell_open(&mut app);
        assert_eq!(record(&app).recipe.corner, Some(CaseworkCorner::Open));

        let mut app = app_with_drawing();
        you(&mut app);
        let r = record(&app);
        assert_eq!(r.recipe.through.as_ref().map(Vec::len), Some(2), "a U");
        assert!(r.recipe.dishwasher.is_some() && r.recipe.oven == Some(true));
        assert_eq!(r.recipe.fridge, Some(kentos_commands::CaseworkEnd::Start));
        let model = &app.document.as_ref().expect("a drawing").model;
        let info = match kentos_native_application::casework_query::plan_in(model, &Default::default()) {
            kentos_commands::CommandResult::Completed { output, .. } => output,
            other => panic!("{other:?}"),
        };
        assert!(info.runs[0].modules.iter().any(|m| m.kind == CaseworkModuleKind::Dishwasher));
        // Otomatik: the sink before the west leg's blind module, the dishwasher under it, the leg full to the corner.
        assert_eq!((r.recipe.sink, r.recipe.dishwasher), (Some(2), Some(1)));
        assert!(info.runs[0].legs[0].filler.abs() < 1e-6, "{:?}", info.runs[0].legs[0]);
        assert!(
            info.runs[0]
                .modules
                .iter()
                .filter(|m| m.blind == Some(true))
                .all(|m| m.kind == CaseworkModuleKind::Base),
            "nothing in a blind corner"
        );

        let mut app = app_with_drawing();
        widened(&mut app);
        let r = record(&app);
        assert_eq!(
            r.recipe.per_module.as_ref().and_then(|m| m.first()).and_then(|m| m.width),
            Some(0.9),
            "the module's own width"
        );

        // The same app: the tool's memory keeps Biçim, so the L is drawn once.
        let mut app = app_with_drawing();
        ell_blind(&mut app);
        let before = record(&app);
        north_moved(&mut app);
        let after = record(&app);
        assert!(after.broken.is_none(), "{:?}", after.broken);
        // The north leg on the moved face (y 2.60), the corner on it, the east leg as long as it was.
        assert!((after.recipe.from.y - (N + 2.6)).abs() < 1e-6, "the run followed the wall");
        let corner = |r: &kentos_commands::CaseworkRecord| r.recipe.through.as_ref().expect("an L")[0];
        assert!((corner(&after).y - (N + 2.6)).abs() < 1e-6);
        let east = |r: &kentos_commands::CaseworkRecord| corner(r).y - r.recipe.to.y;
        assert!((east(&before) - east(&after)).abs() < 1e-6, "the east leg keeps its length");

        let mut app = app_with_drawing();
        door(&mut app);
        assert!(app.document.as_ref().expect("a drawing").model.casework().is_empty(), "a preview");
        put(&mut app, [4.2, 0.6]);
        assert_eq!(app.document.as_ref().expect("a drawing").model.casework().len(), 1);
        assert!(app.log.lines().any(|l| l.text.contains("K1 kapısının önüne geliyor")), "the clash said");
    }

    /// The section of a module's cabinet says what the run and the module are; Tezgâh derinliği, Diziyi bırak and
    /// Dizinin tamamını sil go through the run's commands (docs/adr/0727 §6).
    #[test]
    fn the_section_shows_the_run_and_edits_it() {
        let mut app = app_with_drawing();
        selected(&mut app);
        let doc = app.document.as_ref().expect("a drawing");
        let panel = app.properties_panel(doc);
        let section = panel
            .sections
            .iter()
            .find(|s| s.title == "Mutfak dizisi")
            .expect("the run's section");
        let value = |label: &str| {
            section
                .rows
                .iter()
                .find(|r| r.label == label)
                .map(|r| r.value.clone())
                .unwrap_or_else(|| panic!("{label}"))
        };
        // What the run's query says of its first module, in the section's words.
        let runs = match kentos_native_application::casework_query::plan_in(&doc.model, &Default::default()) {
            kentos_commands::CommandResult::Completed { output, .. } => output,
            other => panic!("{other:?}"),
        };
        let first = runs.runs[0].modules.iter().find(|m| m.number == 1).expect("its first module");
        assert_eq!(value("Dizi"), format!("L · {} modül", runs.runs[0].modules.len()));
        assert_eq!(runs.runs[0].modules.len(), 8);
        assert_eq!(value("Köşe"), "Kör");
        assert_eq!(value("Üst dolap"), "Açık");
        assert_eq!(value("Modül"), "1. modül");
        let kind = match first.kind {
            CaseworkModuleKind::Base => "Alt dolap",
            CaseworkModuleKind::Sink => "Evye",
            CaseworkModuleKind::Cooktop => "Ocak",
            CaseworkModuleKind::Dishwasher => "Bulaşık makinesi",
        };
        assert_eq!(value("Tür"), kind);
        assert_eq!(value("Modül eni"), format!("{} cm", (first.width * 100.0).round()));
        let hand = match first.hand {
            Some(kentos_commands::CaseworkHand::Left) => "Soldan",
            Some(kentos_commands::CaseworkHand::Right) => "Sağdan",
            Some(kentos_commands::CaseworkHand::Double) => "Çift kapı",
            None => "",
        };
        if hand.is_empty() {
            assert!(section.rows.iter().all(|r| r.label != "Açılma"), "no door, no Açılma");
        } else {
            assert_eq!(value("Açılma"), hand);
        }
        assert!(
            !panel
                .sections
                .iter()
                .filter(|s| s.title != "Mutfak dizisi")
                .any(|s| s.rows.iter().any(|r| r.label == "Modül" || r.label == "Açılma")),
            "the run's data only in its section"
        );

        let slot = member(&app, CaseworkPart::Module, Some(1));
        let _ = app.update(Message::Properties(Event::Commit(
            crate::properties::Field::CaseworkDepth(slot),
            "65 cm".into(),
        )));
        assert_eq!(record(&app).recipe.depth, Some(0.65), "the countertop's depth");

        let objects = app.document.as_ref().expect("a drawing").model.entities().count();
        let slot = member(&app, CaseworkPart::Module, Some(1));
        let _ = app.update(Message::Properties(Event::Casework(slot, "release")));
        let model = &app.document.as_ref().expect("a drawing").model;
        assert!(model.casework().is_empty(), "released");
        assert_eq!(model.entities().count(), objects, "its objects stay");

        let mut app = app_with_drawing();
        selected(&mut app);
        let walls = 5;
        let slot = member(&app, CaseworkPart::Module, Some(1));
        let _ = app.update(Message::Properties(Event::Casework(slot, "delete")));
        let model = &app.document.as_ref().expect("a drawing").model;
        assert!(model.casework().is_empty(), "the run went with its objects");
        assert_eq!(model.entities().count(), walls, "the walls and the room's name stay");
    }
}
