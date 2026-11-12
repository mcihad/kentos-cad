//! The pictures and the acceptance of docs/adr/0778 (Proje gezgini): the 2+1 plan of docs/adr/0773 with its two
//! recipe views linked into two sheets (docs/adr/0774) and a door schedule; the Gezgin tab beside Katmanlar with the
//! tree, then with one door selected: where it shows.
//!
//!     KENTOS_SHOTS_ONLY=gezgin-agac,gezgin-nesne cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture

use serde_json::json;

use crate::app::{App, Message};
use crate::program_scenes::{command, fit};
use crate::tools_screens::Scene;
use crate::view_recipe_scenes::both_sheets;

/// The plan, its two linked sheets, a door schedule; the drawing in front, Gezgin shown.
fn project(app: &mut App) {
    both_sheets(app);
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    command(app, "cad.schedules.create", json!({"layerId": layer, "at": {"x": crate::tools_scenes::E + 20.0, "y": crate::tools_scenes::N}, "of": "openings"}));
    let task = app.update(Message::Sheet(kentos_sheet_ui::Message::Tab(0)));
    crate::files_testing::drive(app, task);
    fit(app, -0.5, -0.5, 21.5, 10.0);
    let _ = app.update(Message::Run("arch.navigator"));
}

/// The first door of the drawing, by its slot.
fn a_door(app: &App) -> kentos_domain::Slot {
    let doc = app.document.as_ref().expect("a drawing");
    doc.model
        .entities()
        .find(|e| matches!(e, kentos_commands::Entity::Opening(_)))
        .map(|e| kentos_domain::Slot(e.base().id))
        .expect("a door")
}

fn tree(app: &mut App) {
    project(app);
}

fn object(app: &mut App) {
    project(app);
    let door = a_door(app);
    app.selection.set(vec![door]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![("gezgin-agac", tree), ("gezgin-nesne", object)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::navigator::Event;

    fn nav(app: &mut App, e: Event) {
        let task = app.update(Message::Navigator(e));
        crate::files_testing::drive(app, task);
    }

    /// docs/adr/0778 §3: the tree reads the project as `cad.navigator.read` does; its rows act.
    #[test]
    fn the_tree_reads_the_project_and_its_rows_act() {
        let mut app = crate::files_testing::app_with_drawing();
        project(&mut app);
        let read = app.navigator_read().expect("a drawing");
        assert_eq!(read.levels.len(), 2);
        assert_eq!(read.views.len(), 2);
        assert_eq!(read.sheets.len(), 2);
        assert_eq!(read.schedules.len(), 1);
        assert!(read.levels[0].objects > 0);
        assert_eq!(read.levels[0].views.len(), 2, "both views draw the ground floor");
        assert_eq!(read.levels[0].maps.len(), 2, "both sheets' maps draw it");
        assert!(read.object.is_none());

        nav(&mut app, Event::Level(Some("kat-1".into())));
        assert_eq!(app.active_level().as_deref(), Some("kat-1"));
        nav(&mut app, Event::Level(None));
        assert_eq!(app.active_level(), None);

        // A sheet's row opens it.
        nav(&mut app, Event::Sheet(1));
        let second = read.sheets[1].id.clone();
        assert_eq!(app.sheets.open_sheet().map(|s| s.id.clone()), Some(second), "the second sheet in front");
    }

    /// docs/adr/0778 §2: one door selected: its level, the two views and maps drawing it, the schedule listing it; the
    /// answer read again only when the revision or the selection changes.
    #[test]
    fn one_object_says_where_it_shows() {
        let mut app = crate::files_testing::app_with_drawing();
        project(&mut app);
        let door = a_door(&app);
        app.selection.set(vec![door]);
        let read = app.navigator_read().expect("a drawing");
        let o = read.object.expect("the door");
        assert_eq!(o.kind, "opening");
        assert_eq!(o.level.as_deref(), Some("zemin"));
        assert_eq!(o.views.len(), 2);
        assert_eq!(o.maps.len(), 2);
        assert_eq!(o.schedules, vec![read.schedules[0].uid.clone()]);
        // The schedule's row selects the table.
        let table = read.schedules[0].uid.clone();
        nav(&mut app, Event::Objects(vec![table.clone()]));
        let doc = app.document.as_ref().expect("a drawing");
        let chosen: Vec<String> =
            app.selection.ids().iter().filter_map(|s| doc.model.uid(*s)).map(|u| u.to_string()).collect();
        assert_eq!(chosen, vec![table]);
        let again = app.navigator_read().expect("a drawing");
        assert!(again.object.is_some_and(|o| o.kind == "table"), "the selection changed: read again");
    }
}
