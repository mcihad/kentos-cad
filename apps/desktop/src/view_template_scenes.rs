//! The pictures and the acceptance of docs/adr/0783 (görünüş şablonu ve kalem seti): the 2+1 plan's two recipe views of
//! docs/adr/0774 follow one template, Uygulama 1:50 (walls 0.5 mm, hatches grey, dimensions blue); its sheets draw with it
//! and change together when the template does. The Görünümler window with the template chosen, then a sheet in black and
//! white.
//!
//!     KENTOS_SHOTS_ONLY=gorunus-sablon-pencere,gorunus-sablon-kalem,gorunus-sablon-pafta cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture

use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::program_scenes::command;
use crate::tools_screens::Scene;
use crate::view_recipe_scenes::both_sheets;

fn template(scale: u32, mono: bool) -> Value {
    let mut r = json!({"scale": scale, "detail": "standard",
        "pens": [{"of": "walls", "weight": 0.5}, {"of": "dimensions", "weight": 0.18, "color": "#204080"},
                 {"of": "hatches", "color": "#a0a0a0"}]});
    if mono {
        r["mono"] = json!(true);
    }
    r
}

/// The plan's sheets, Uygulama 1:50 kept, both views following it.
fn followed(app: &mut App) {
    both_sheets(app);
    command(
        app,
        "cad.views.edit",
        json!({"edit": {"kind": "template", "name": "Uygulama 1:50", "recipe": template(50, false)}}),
    );
    for view in ["Zemin uygulama", "Zemin mobilya"] {
        command(
            app,
            "cad.views.edit",
            json!({"edit": {"kind": "recipe", "name": view, "recipe": {"level": {"id": "zemin"}, "template": "Uygulama 1:50"}}}),
        );
    }
}

fn window(app: &mut App) {
    followed(app);
    let _ = app.update(Message::Run("view.namedViews"));
    let _ = app.update(Message::Views(crate::views::Event::Choose("Zemin uygulama".into())));
}

/// The window with a view of its own: Siyah beyaz and the pens.
fn pens(app: &mut App) {
    both_sheets(app);
    command(
        app,
        "cad.views.edit",
        json!({"edit": {"kind": "recipe", "name": "Zemin mobilya", "recipe": {"level": {"id": "zemin"}, "scale": 100,
               "mono": true, "pens": [{"of": "walls", "weight": 0.7}, {"of": "blocks", "weight": 0.13}]}}}),
    );
    let _ = app.update(Message::Run("view.namedViews"));
    let _ = app.update(Message::Views(crate::views::Event::Choose("Zemin mobilya".into())));
    let _ = app.update(Message::Views(crate::views::Event::TemplateName("Sunum 1:100".into())));
}

/// The sheet in front drawn by the template in black and white.
fn sheet(app: &mut App) {
    followed(app);
    command(
        app,
        "cad.views.edit",
        json!({"edit": {"kind": "template", "name": "Uygulama 1:50", "recipe": template(50, true)}}),
    );
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("gorunus-sablon-pencere", window),
        ("gorunus-sablon-kalem", pens),
        ("gorunus-sablon-pafta", sheet),
    ]
}

#[cfg(test)]
mod tests {
    use kentos_commands::PenCategory;

    use super::*;
    use crate::views::Event;

    /// The front sheet's linked map as the display list draws it.
    fn front_map(app: &mut App) -> kentos_sheet::display::MapPrim {
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

    /// docs/adr/0783 §2: both views follow the template; changing it once redraws the linked map, its level kept, and
    /// both views resolve to it.
    #[test]
    fn a_template_change_redraws_every_linked_map() {
        let mut app = crate::files_testing::app_with_drawing();
        followed(&mut app);
        let before = front_map(&mut app);
        assert_eq!(before.pens.iter().find(|p| p.of == PenCategory::Walls).and_then(|p| p.weight), Some(0.5));
        assert!(!before.mono);
        assert_eq!(before.level.as_ref().map(|l| l.id.as_str()), Some("zemin"));
        command(
            &mut app,
            "cad.views.edit",
            json!({"edit": {"kind": "template", "name": "Uygulama 1:50", "recipe": template(50, true)}}),
        );
        let after = front_map(&mut app);
        assert!(after.mono, "the linked map follows the template's change");
        assert_eq!(after.level.as_ref().map(|l| l.id.as_str()), Some("zemin"), "its level stays the view's");
        // Both views follow it: the other's resolved recipe is the template's too.
        let views = kentos_commands::resolved_named_views(app.named_views(), app.view_templates());
        assert!(views.iter().filter(|v| v.recipe.as_ref().is_some_and(|r| r.mono)).count() >= 2);
    }

    /// docs/adr/0783 §3: the window follows a template, leaves it (its fields kept as the view's own), sets a pen and
    /// black and white, and keeps the draft as a new template.
    #[test]
    fn the_window_follows_leaves_and_saves_templates() {
        let mut app = crate::files_testing::app_with_drawing();
        window(&mut app);
        let r = app.recipe_draft("Zemin uygulama");
        assert!(r.template.is_some(), "it follows Uygulama 1:50");
        // Leave the template: its fields stay.
        let _ = app.update(Message::Views(Event::RecipeTemplate(None)));
        let _ = app.update(Message::Views(Event::RecipeMono(true)));
        let _ = app.update(Message::Views(Event::RecipePen(PenCategory::Walls, Some(5))));
        let _ = app.update(Message::Views(Event::RecipeApply));
        let r = app.recipe_draft("Zemin uygulama");
        assert_eq!(r.template, None);
        assert_eq!(r.scale, Some(50));
        assert!(r.mono);
        assert_eq!(r.pen(PenCategory::Walls).and_then(|p| p.weight), Some(0.7));
        assert_eq!(r.pen(PenCategory::Hatches).and_then(|p| p.color.clone()).as_deref(), Some("#a0a0a0"));
        // Keep it as a template.
        let _ = app.update(Message::Views(Event::TemplateName("Sunum".into())));
        let _ = app.update(Message::Views(Event::TemplateSave));
        let t = app.view_templates().iter().find(|t| t.name == "Sunum").cloned().expect("saved");
        assert!(t.recipe.mono && t.recipe.level.is_none() && !t.recipe.crop);
        // Follow it again.
        let i = app.view_templates().iter().position(|x| x.name == "Sunum").expect("listed");
        let _ = app.update(Message::Views(Event::RecipeTemplate(Some(i))));
        let _ = app.update(Message::Views(Event::RecipeApply));
        let r = app.recipe_draft("Zemin uygulama");
        assert_eq!(r.template.as_deref(), Some(t.id.as_str()));
        assert_eq!(r.level.as_ref().map(|l| l.id.as_str()), Some("zemin"), "the level stays the view's");
        assert!(crate::views_recipe::recipe_caption(&app, &r).contains("şablon Sunum"));
    }
}
