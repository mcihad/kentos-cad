//! The pictures and the acceptance of docs/adr/0781 (tarifin duvar ayrıntısı ve kırpması): the 2+1 plan of
//! docs/adr/0773 and its two recipe views of docs/adr/0774, and a third, Banyo detay, round the bathroom: 1:50, the
//! walls detailed whatever the scale, its linked map cut to the view's area. The Görünümler window with its recipe, then
//! its sheet.
//!
//!     KENTOS_SHOTS_ONLY=gorunus-ayrinti-pencere,gorunus-ayrinti-pafta cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture

use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::program_scenes::command;
use crate::tools_scenes::{E, N};
use crate::tools_screens::Scene;
use crate::view_recipe_scenes::{both_sheets, linked_sheet};

/// The bathroom's area: the room between x 10–13 and y 0–4.5, its walls and a margin.
pub(crate) const BATH: (f64, f64, f64, f64) = (9.6, -0.4, 13.4, 4.9);

pub(crate) fn bath_recipe(detail: &str, crop: bool) -> Value {
    json!({"level": {"id": "zemin"}, "scale": 50, "detail": detail, "crop": crop})
}

/// Banyo detay saved and given its recipe; its sheet linked; that sheet in front.
pub(crate) fn bath(app: &mut App) {
    let plan = both_sheets(app);
    // Layered walls, so that the detail shows: the house's own walls have no style.
    command(app, "cad.walls.edit", json!({"uids": plan.walls, "operation": {"kind": "set", "style": "Dış duvar 25"}}));
    let (x0, y0, x1, y1) = BATH;
    command(
        app,
        "cad.views.edit",
        json!({"edit": {"kind": "save", "name": "Banyo detay", "center": {"x": E + (x0 + x1) / 2.0, "y": N + (y0 + y1) / 2.0},
                        "width": x1 - x0, "height": y1 - y0}}),
    );
    command(
        app,
        "cad.views.edit",
        json!({"edit": {"kind": "recipe", "name": "Banyo detay", "recipe": bath_recipe("detailed", true)}}),
    );
    linked_sheet(app, "Banyo detay");
}

fn window(app: &mut App) {
    bath(app);
    let task = app.update(Message::Sheet(kentos_sheet_ui::Message::Tab(0)));
    crate::files_testing::drive(app, task);
    let _ = app.update(Message::Run("view.namedViews"));
    let _ = app.update(Message::Views(crate::views::Event::Choose("Banyo detay".into())));
}

fn sheet(app: &mut App) {
    bath(app);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![("gorunus-ayrinti-pencere", window), ("gorunus-ayrinti-pafta", sheet)]
}

#[cfg(test)]
mod tests {
    use kentos_commands::WallDetail;

    use super::*;
    use crate::views::Event;

    /// The front sheet's (only) linked map as the display list draws it.
    fn the_map(app: &mut App) -> kentos_sheet::display::MapPrim {
        // The sheet reads the views from its context; a command run outside the update loop does not refresh it.
        let ctx = app.sheet_context();
        app.sheets.set_context(ctx);
        app.sheets
            .export_list()
            .expect("a sheet in front")
            .prims
            .into_iter()
            .find_map(|p| match p {
                kentos_sheet::display::Prim::Map(m) => Some(m),
                _ => None,
            })
            .expect("its map")
    }

    /// The map's vectors as the PDF writes them.
    fn vectors(app: &App, map: &kentos_sheet::display::MapPrim) -> String {
        use kentos_sheet_ui::MapPainter;
        let request = kentos_sheet_ui::MapRequest::of(map, iced::Size::new(1000., 600.), 0.003, true);
        let content = app.sheet_painter().vector(&request).expect("vector content");
        serde_json::to_string(&content).expect("its JSON")
    }

    /// docs/adr/0781 §1, §2: the linked map carries its view's detail and is cut to the view's area; Ayrıntılı draws more
    /// wall lines than Şematik at the same 1:50; the crop taken away, the map draws its whole frame again.
    #[test]
    fn a_linked_map_draws_its_view_s_detail_and_crop() {
        let mut app = crate::files_testing::app_with_drawing();
        bath(&mut app);
        let map = the_map(&mut app);
        assert_eq!(map.wall_detail, Some(WallDetail::Detailed));
        assert!(!map.clip_shape.is_frame(), "cut to the view's area");
        let detailed = vectors(&app, &map);

        command(
            &mut app,
            "cad.views.edit",
            json!({"edit": {"kind": "recipe", "name": "Banyo detay", "recipe": bath_recipe("schematic", true)}}),
        );
        let map = the_map(&mut app);
        assert_eq!(map.wall_detail, Some(WallDetail::Schematic));
        let schematic = vectors(&app, &map);
        assert!(
            detailed.len() > schematic.len(),
            "Ayrıntılı draws the walls' layers: {} > {}",
            detailed.len(),
            schematic.len()
        );

        command(
            &mut app,
            "cad.views.edit",
            json!({"edit": {"kind": "recipe", "name": "Banyo detay", "recipe": {"level": {"id": "zemin"}, "scale": 50}}}),
        );
        let map = the_map(&mut app);
        assert_eq!(map.wall_detail, None);
        assert!(map.clip_shape.is_frame(), "no crop: the frame's own shape");
    }

    /// docs/adr/0781 §3: the window's Duvar ayrıntısı and the crop's tick go into the draft and Tarifi uygula writes them.
    #[test]
    fn the_window_writes_the_detail_and_the_crop() {
        let mut app = crate::files_testing::app_with_drawing();
        window(&mut app);
        let r = app.recipe_draft("Banyo detay");
        assert_eq!(r.detail, Some(WallDetail::Detailed));
        assert!(r.crop);
        let _ = app.update(Message::Views(Event::RecipeDetail(1)));
        let _ = app.update(Message::Views(Event::RecipeCrop(false)));
        let _ = app.update(Message::Views(Event::RecipeApply));
        let v = app.named_views().iter().find(|v| v.name == "Banyo detay").cloned().expect("kept");
        let r = v.recipe.expect("a recipe");
        assert_eq!(r.detail, Some(WallDetail::Schematic));
        assert!(!r.crop);
        assert_eq!(r.version, 2);
        assert!(crate::views_recipe::recipe_caption(&app, &r).contains("duvar şematik"));
    }
}
