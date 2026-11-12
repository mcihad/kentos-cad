//! The pictures of docs/adr/0730 (a dimension recipe's sheet scales, its values placed where they do not fit, its rows'
//! rounding, its copies): the house of `facade_dimension_scenes` at 1:50. Duvar ölçüleri as Cephe on the south wall
//! with its walls row: Sığmayan yazı → Kaydır (the new recipe's) puts the thin walls' values aside, Yerinde keeps them in
//! the danger tone; Pafta ölçekleri 1:100 1:200 said sheet by sheet; written with one Enter and a member's Öznitelikler
//! (Pafta ölçekleri, Sığmayan yazı); a 3.0012 m wall whose parts round to 3.000 against its overall's 3.001; the house
//! and its recipe copied with Kopyala, three times with its Dizi and mirrored with Aynala, each copy measuring its copy.
//! `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=olcu-dizisi-kaydir,olcu-dizisi-yerinde,olcu-dizisi-paftalar,olcu-dizisi-oznitelikler,olcu-dizisi-yuvarlama,olcu-dizisi-kopya,olcu-dizisi-kopya-yakin,olcu-dizisi-dizi,olcu-dizisi-ayna cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use crate::app::App;
use crate::facade_dimension_scenes::{SOUTH, house, look, set};
use crate::tools_scenes::{click, hover, run, typed};
use crate::tools_screens::Scene;
use crate::wall_scenes::wall_between;

/// Cephe on the south wall with its walls row: the 25 and 15 cm walls' values do not fit between their lines.
fn facade(app: &mut App) {
    house(app);
    run(app, "tool.wallDimensions");
    set(app, "M", "facade");
    set(app, "W", "yes");
    click(app, [5.0, 0.0]);
    look(app, SOUTH);
}

/// The west half of the south façade, close: the walls row's values readable.
const WEST: [f64; 4] = [-1.0, -2.6, 6.8, 0.6];

/// Sığmayan yazı → Kaydır, the new recipe's: the thin walls' values beside their lines.
fn shifted(app: &mut App) {
    facade(app);
    look(app, WEST);
    hover(app, [2.0, -1.6]);
}

/// Sığmayan yazı → Yerinde: they stay, in the danger tone, and are said.
fn kept(app: &mut App) {
    facade(app);
    set(app, "I", "keep");
    look(app, WEST);
    hover(app, [2.0, -1.6]);
}

/// Pafta ölçekleri 1:100 1:200: the rows drawn on those sheets too, each sheet's values said.
fn sheets(app: &mut App) {
    facade(app);
    set(app, "Ö", "1:200 1:100");
    hover(app, [5.0, -1.6]);
}

/// Written with one Enter; a member selected: Öznitelikler's Pafta ölçekleri and Sığmayan yazı.
fn properties(app: &mut App) {
    sheets(app);
    typed(app, "");
    let member = app
        .document
        .as_ref()
        .and_then(|d| {
            let set = d.model.dimension_sets().first()?;
            d.model.slot_of(kentos_domain::Uuid::from_bytes(set.members.first()?.uid.0))
        })
        .expect("a member");
    app.selection.set([member]);
    hover(app, [10.8, 1.2]);
}

/// A 3.0012 m wall with a 1.0004 m opening in its middle: its parts read 1.000 each, 3.000 together, its overall 3.001.
fn rounding(app: &mut App) {
    crate::wall_scenes::opened(app, crate::tools_scenes::Objects::new(), 50.0);
    look(app, [-0.8, -2.4, 3.8, 1.2]);
    app.tab = "arch";
    app.command_expanded = true;
    wall_between(app, "Dış duvar 25", [0.0, 0.0], [3.0012, 0.0]);
    let doc = app.document.as_mut().expect("a drawing");
    let wall = doc
        .model
        .entities_with_uids()
        .find(|(_, e)| matches!(e, kentos_commands::Entity::Wall(_)))
        .map(|(uid, e)| (uid.to_string(), e.base().layer_id.clone()))
        .expect("the wall");
    let made = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        "cad.openings.create",
        kentos_commands::CallOp::Execute,
        serde_json::json!({"layerId": wall.1, "openings": [{"host": wall.0, "at": 1.5006, "width": 1.0004}]}),
    )
    .expect("runs");
    assert_eq!(made["status"], "completed", "{made}");
    app.selection.clear();
    run(app, "tool.wallDimensions");
    set(app, "M", "wall");
    click(app, [1.5, 0.0]);
    hover(app, [3.2, -1.2]);
}

/// The south façade's recipe written, then the whole house and its recipe selected.
fn written_and_selected(app: &mut App) {
    facade(app);
    typed(app, "");
    app.selection.clear();
    app.draft.snap = false;
    run(app, "edit.selectAll");
}

/// Kopyala 14 m east: the copy has its own recipe on the copied walls, the same rows.
fn copied(app: &mut App) {
    written_and_selected(app);
    run(app, "tool.copy");
    click(app, [0.0, 0.0]);
    click(app, [14.0, 0.0]);
    typed(app, "");
    app.selection.clear();
    look(app, [-1.5, -4.0, 25.5, 10.5]);
    hover(app, [12.0, 10.0]);
}

/// Kopyala's Dizi: three copies along 14 m, each with its recipe.
fn arrayed(app: &mut App) {
    written_and_selected(app);
    run(app, "tool.copy");
    click(app, [0.0, 0.0]);
    typed(app, "A");
    typed(app, "4");
    click(app, [14.0, 0.0]);
    typed(app, "");
    app.selection.clear();
    look(app, [-1.5, -4.0, 53.5, 10.5]);
    hover(app, [26.0, 10.0]);
}

/// The copy's façade close, a member of its recipe selected: Öznitelikler says the copy's own recipe.
fn copied_close(app: &mut App) {
    copied(app);
    look(app, [12.8, -3.4, 25.2, 1.4]);
    let member = app
        .document
        .as_ref()
        .and_then(|d| {
            let set = d.model.dimension_sets().get(1)?;
            d.model.slot_of(kentos_domain::Uuid::from_bytes(set.members.first()?.uid.0))
        })
        .expect("the copy's member");
    app.selection.set([member]);
    hover(app, [24.8, 1.2]);
}

/// Aynala across x 12: the copy's walls run back, its recipe measures the same faces, its rows below it.
fn mirrored(app: &mut App) {
    written_and_selected(app);
    run(app, "tool.mirror");
    click(app, [12.0, -6.0]);
    click(app, [12.0, 12.0]);
    app.selection.clear();
    look(app, [-1.5, -4.0, 25.5, 10.5]);
    hover(app, [12.0, 10.0]);
}

/// The member's Öznitelikler scrolled to the recipe's rows: Pafta ölçekleri and Sığmayan yazı at its end.
pub(crate) fn pointed() -> Vec<crate::tools_screens::Pointed> {
    fn scroll(s: &mut kentos_ui::snapshot::Snapshot, app: &mut App) {
        let mut update = |app: &mut App, message: crate::app::Message| {
            let _ = app.update(message);
        };
        let at = crate::tools_screens::caption_at(s, app, "Genel", false);
        s.input(app, App::view, &mut update, kentos_ui::snapshot::Input::Scroll(at, -14.0));
    }
    fn settings(s: &mut kentos_ui::snapshot::Snapshot, app: &mut App) {
        let mut update = |app: &mut App, message: crate::app::Message| {
            let _ = app.update(message);
        };
        let at = crate::tools_screens::caption_at(s, app, "Kaynak türü", false);
        s.input(app, App::view, &mut update, kentos_ui::snapshot::Input::Scroll(at, -6.0));
    }
    vec![
        ("olcu-dizisi-oznitelikler", properties, scroll),
        ("olcu-dizisi-paftalar", sheets, settings),
    ]
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("olcu-dizisi-kaydir", shifted),
        ("olcu-dizisi-yerinde", kept),
        ("olcu-dizisi-yuvarlama", rounding),
        ("olcu-dizisi-kopya", copied),
        ("olcu-dizisi-kopya-yakin", copied_close),
        ("olcu-dizisi-dizi", arrayed),
        ("olcu-dizisi-ayna", mirrored),
    ]
}
