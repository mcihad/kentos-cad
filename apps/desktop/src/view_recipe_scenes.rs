//! The pictures and the acceptance of docs/adr/0774 (Görünüş tarifi): the 2+1 plan of docs/adr/0773, two named views of
//! its ground floor — Zemin uygulama (1:50, every category) and Zemin mobilya (1:100, its tags and dimensions left out) —
//! each linked into a sheet's map, so one model draws two sheets that differ only by the view's recipe; the Görünümler
//! window with the chosen view's recipe; one door made wider and both sheets following.
//!
//!     KENTOS_SHOTS_ONLY=gorunus-tarifi-pencere,gorunus-tarifi-uygulama,gorunus-tarifi-mobilya cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture

use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::program_scenes::{command, house};
use crate::tools_scenes::{E, N};
use crate::tools_screens::Scene;

/// The ground floor's box: where both views look.
const GROUND: (f64, f64, f64, f64) = (-0.5, -0.5, 6.0, 10.5);

fn sheet(app: &mut App, m: kentos_sheet_ui::Message) {
    let task = app.update(Message::Sheet(m));
    crate::files_testing::drive(app, task);
}

fn recipe(scale: u32, hidden: &[&str]) -> Value {
    json!({"level": {"id": "zemin"}, "scale": scale, "hidden": hidden})
}

/// The two views, saved and given their recipes through `cad.views.edit`.
fn views(app: &mut App) {
    let (x0, y0, x1, y1) = GROUND;
    let center = json!({"x": E + (x0 + x1) / 2.0, "y": N + (y0 + y1) / 2.0});
    for (name, r) in [
        ("Zemin uygulama", recipe(50, &[])),
        ("Zemin mobilya", recipe(100, &["dimensions", "tags"])),
    ] {
        command(
            app,
            "cad.views.edit",
            json!({"edit": {"kind": "save", "name": name, "center": center, "width": x1 - x0, "height": y1 - y0}}),
        );
        command(
            app,
            "cad.views.edit",
            json!({"edit": {"kind": "recipe", "name": name, "recipe": r}}),
        );
    }
}

/// A new sheet whose map is linked to the view named `name` (the inspector's Kaynak: canlı bağlantı).
fn linked_sheet(app: &mut App, name: &str) {
    use kentos_sheet_ui::Message as Sheet;
    sheet(app, Sheet::NewSheet);
    if app.sheets.questions().is_some() {
        sheet(
            app,
            Sheet::Questions(kentos_sheet_ui::questions::QuestionsMessage::Make),
        );
    }
    let view = app
        .named_views()
        .iter()
        .find(|v| v.name == name)
        .cloned()
        .expect("the view");
    let map = app.sheets.open_sheet().and_then(|s| {
        s.items
            .iter()
            .find(|i| matches!(i.kind, kentos_sheet::kinds::ItemKind::Map(_)))
            .map(|i| i.id.clone())
    });
    if let Some(id) = map {
        sheet(app, Sheet::Select(vec![id.clone()]));
        sheet(
            app,
            Sheet::MapConfigure(Box::new(kentos_sheet::viewport::SetViewport {
                id,
                source: Some(kentos_sheet::viewport::Source::ViewLink { view }),
                ..Default::default()
            })),
        );
    }
}

fn both_sheets(app: &mut App) {
    house(app);
    views(app);
    linked_sheet(app, "Zemin uygulama");
    linked_sheet(app, "Zemin mobilya");
}

/// Görünümler open, Zemin mobilya chosen: its recipe.
fn window(app: &mut App) {
    house(app);
    views(app);
    let _ = app.update(Message::Run("view.namedViews"));
    let _ = app.update(Message::Views(crate::views::Event::Choose(
        "Zemin mobilya".into(),
    )));
}

/// The first sheet in front: the 1:50 plan with every category.
fn first(app: &mut App) {
    both_sheets(app);
    sheet(app, kentos_sheet_ui::Message::Tab(1));
}

/// The second: the 1:100 plan without its tags and dimensions.
fn second(app: &mut App) {
    both_sheets(app);
    sheet(app, kentos_sheet_ui::Message::Tab(2));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("gorunus-tarifi-pencere", window),
        ("gorunus-tarifi-uygulama", first),
        ("gorunus-tarifi-mobilya", second),
    ]
}

#[cfg(test)]
mod tests {
    use kentos_commands::{PhaseView, ViewCategory};

    use super::*;
    use crate::files_testing::last_said;
    use crate::views::Event;

    /// The model's side of a map's paper:model ratio (its scale's denominator).
    fn ratio(v: kentos_sheet::kinds::FixedView) -> f64 {
        let r = v.ratio.expect("a linked map's ratio");
        r.model / r.paper
    }

    fn views_event(app: &mut App, e: Event) {
        let _ = app.update(Message::Views(e));
    }

    /// docs/adr/0774 §2: Hazır ▾ fills the draft, Tarifi uygula writes it (its version one more each time), a category
    /// ticked is kept in the categories' order, Tarifi kaldır takes it away; the model does not change.
    #[test]
    fn the_window_writes_a_recipe_and_takes_it_away() {
        let mut app = crate::files_testing::app_with_drawing();
        window(&mut app);
        let model_before = app
            .document
            .as_ref()
            .expect("a drawing")
            .model
            .entities()
            .count();
        views_event(&mut app, Event::RecipePreset(3));
        views_event(&mut app, Event::RecipeApply);
        let r = app.recipe_draft("Zemin mobilya");
        assert_eq!(r.version, 2, "one write in the scene, this the second");
        assert_eq!(r.phase, Some(PhaseView::Demolition));
        assert_eq!(r.hidden, vec![ViewCategory::Blocks]);
        assert_eq!(r.scale, Some(100));
        assert_eq!(
            r.level.as_ref().map(|l| l.id.as_str()),
            Some("zemin"),
            "the preset keeps the level"
        );
        assert!(
            last_said(&app).contains("tarifi yazıldı"),
            "{}",
            last_said(&app)
        );
        views_event(&mut app, Event::RecipeHide(ViewCategory::Dimensions, true));
        views_event(&mut app, Event::RecipeApply);
        let r = app.recipe_draft("Zemin mobilya");
        assert_eq!(r.version, 3);
        assert_eq!(
            r.hidden,
            vec![ViewCategory::Dimensions, ViewCategory::Blocks]
        );
        views_event(&mut app, Event::RecipeClear);
        assert!(
            app.named_views()
                .iter()
                .find(|v| v.name == "Zemin mobilya")
                .expect("kept")
                .recipe
                .is_none()
        );
        assert_eq!(
            last_said(&app),
            "“Zemin mobilya” görünümünün tarifi kaldırıldı."
        );
        assert_eq!(
            app.document
                .as_ref()
                .expect("a drawing")
                .model
                .entities()
                .count(),
            model_before
        );
    }

    /// docs/adr/0774 §3: each linked map draws at its view's recipe scale, on its level, with its categories left out;
    /// a recipe changed after linking is the map's at once.
    #[test]
    fn linked_maps_draw_with_their_view_s_recipe() {
        let mut app = crate::files_testing::app_with_drawing();
        both_sheets(&mut app);
        let views = app.named_views().to_vec();
        let book = app.sheets.book();
        let maps: Vec<_> = book
            .sheets
            .iter()
            .flat_map(|s| s.items.iter())
            .filter(|i| matches!(i.kind, kentos_sheet::kinds::ItemKind::Map(_)))
            .cloned()
            .collect();
        assert!(maps.len() >= 2, "two linked maps");
        let scales: Vec<f64> = maps
            .iter()
            .rev()
            .take(2)
            .map(|m| {
                ratio(kentos_sheet::viewport::effective_fixed_view(m, &views).expect("linked"))
            })
            .collect();
        assert_eq!(scales, vec![100.0, 50.0]);
        let last = maps.last().expect("the second");
        let kentos_sheet::kinds::ItemKind::Map(last_map) = &last.kind else {
            unreachable!()
        };
        let recipe =
            kentos_sheet::viewport::linked_recipe(last_map, &views).expect("its view's recipe");
        assert_eq!(
            recipe.hidden,
            vec![ViewCategory::Dimensions, ViewCategory::Tags]
        );
        command(
            &mut app,
            "cad.views.edit",
            json!({"edit": {"kind": "recipe", "name": "Zemin mobilya", "recipe": {"level": {"id": "zemin"}, "scale": 200}}}),
        );
        let views = app.named_views().to_vec();
        assert_eq!(
            ratio(kentos_sheet::viewport::effective_fixed_view(last, &views).expect("linked")),
            200.0
        );
        assert!(
            kentos_sheet::viewport::linked_recipe(last_map, &views)
                .expect("kept")
                .hidden
                .is_empty()
        );
    }
}
