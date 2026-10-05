//! The ribbon's own panels as the web's (docs/adr/0089): Giriş's Katmanlar,
//! Şablonlar (docs/adr/0176 §4c) and Özellikler, the contextual Seçim tab
//! with its panel, the current colour of new objects. The sample drawing:
//! Kadastro holds Parsel (active) and Bina (locked); Çizim is hidden.

use kentos_contracts::{LineType, Workspace};
use kentos_domain::Slot;

use super::*;
use crate::catalog::catalog;
use crate::files_testing::app_with_drawing;

fn panel(tab: &str, label: &str) -> &'static RibbonPanel {
    catalog()
        .tabs_in(Workspace::Gis)
        .chain(catalog().contextual_in(Workspace::Gis))
        .find(|t| t.id == tab)
        .and_then(|t| t.panels.iter().find(|p| p.label == label))
        .unwrap_or_else(|| panic!("{tab} › {label}"))
}

/// The newest object's colour.
fn newest_color(app: &App) -> Option<String> {
    let doc = &app.document.as_ref().expect("a drawing").model;
    let newest = doc.entities().max_by_key(|e| e.base().id).expect("objects");
    newest.base().color.clone()
}

/// A point typed on the command line, as the user would.
fn typed(app: &mut App, text: &str) {
    let _ = app.update(Message::CommandInput(text.to_owned()));
    let _ = app.update(Message::CommandSubmitted);
}

#[test]
fn giris_has_the_webs_katmanlar_sablonlar_and_ozellikler_which_step_down_to_one_button() {
    let _typography = crate::appearance::tests::TYPOGRAPHY.lock();
    let app = app_with_drawing();
    for label in ["Katmanlar", "Şablonlar", "Özellikler"] {
        let group = app
            .ribbon_group("home", panel("home", label))
            .unwrap_or_else(|| panic!("{label} on the desktop"));
        let w = group.widths();
        // The fields shorten and the buttons lose their labels before the panel folds.
        assert!(w[0] >= w[1] && w[1] > w[2] && w[2] > w[3], "{label}: {w:?}");
    }
    // Seçim is the contextual tab's panel.
    let group = app
        .ribbon_group("selection", panel("selection", "Seçim"))
        .expect("Seçim");
    let w = group.widths();
    assert!(w[0] > w[2] && w[2] > w[3], "Seçim: {w:?}");
}

/// Şablonlar's field lists the templates drawn with last, then every
/// category's, each with its tool; the one being drawn with is chosen, and
/// a choice draws with it.
#[test]
fn the_template_field_lists_the_recent_then_the_categories_the_drawn_one_chosen() {
    use super::menus::TemplateLine;
    use serde_json::json;

    let mut app = app_with_drawing();
    let template = |id: &str, name: &str, path: &[&str], tool: &str| json!({ "kind": "template", "id": id, "name": name, "path": path, "template": { "tool": tool, "layer": { "path": [], "name": "Parsel" } } });
    let styles = kentos_contracts::ProjectStyles {
        items: vec![
            template("p-parsel", "Parsel sınırı", &["Kadastro"], "polygon"),
            template("p-ada", "Ada sınırı", &["Kadastro"], "polyline"),
            template("p-nokta", "Poligon noktası", &[], "point"),
        ],
        categories: vec![json!({ "path": ["Kadastro"] })],
    };
    app.document
        .as_mut()
        .expect("a drawing")
        .model
        .set_styles(styles);
    let _ = app.update(Message::Swallowed);
    let words = |app: &App| -> Vec<String> {
        app.template_menu_lines()
            .into_iter()
            .map(|line| match line {
                TemplateLine::Header(label) => format!("» {label}"),
                TemplateLine::Template {
                    name, tool, chosen, ..
                } => format!("{}{name} ({tool})", if chosen { "• " } else { "" }),
            })
            .collect()
    };
    assert_eq!(
        words(&app),
        [
            "» Kadastro",
            "Ada sınırı (Çoklu çizgi)",
            "Parsel sınırı (Kapalı alan)",
            "» Kategorisiz",
            "Poligon noktası (Nokta)"
        ]
    );
    let _ = app.update(Message::DrawTemplate("p-parsel".into()));
    assert_eq!(
        words(&app),
        [
            "» Son kullanılanlar",
            "• Parsel sınırı (Kapalı alan)",
            "» Kadastro",
            "Ada sınırı (Çoklu çizgi)",
            "• Parsel sınırı (Kapalı alan)",
            "» Kategorisiz",
            "Poligon noktası (Nokta)"
        ]
    );
}

#[test]
fn the_layer_field_lists_the_tree_under_its_groups_with_counts_and_locks() {
    let mut app = app_with_drawing();
    let doc = app.document.as_ref().expect("a drawing");
    let lines = app.layer_menu_items(doc);
    let seen: Vec<String> = lines
        .iter()
        .map(|line| match line {
            LayerLine::Header(path) => format!("# {path}"),
            LayerLine::Layer {
                name,
                count,
                active,
                locked,
                ..
            } => format!(
                "{name} {count}{}{}",
                if *active { " etkin" } else { "" },
                if *locked { " kilitli" } else { "" }
            ),
        })
        .collect();
    assert_eq!(
        seen,
        ["# Kadastro", "Parsel 1 etkin", "Bina 3 kilitli", "Çizim 9"]
    );
    // A choice makes the layer the active one, as the tree's double click.
    let _ = app.update(Message::Layer(crate::layering::Event::Activate(
        "cizim".to_owned(),
    )));
    let doc = app.document.as_ref().expect("a drawing");
    assert_eq!(doc.model.layers().active(), "cizim");
}

#[test]
fn a_chosen_colour_goes_into_the_next_objects_and_by_layer_leaves_it_out() {
    let mut app = app_with_drawing();
    let _ = app.update(Message::RibbonPanel(Event::Color(Some("#E5484D"))));
    // A settings change keeps the session's colour.
    app.apply_settings();
    assert_eq!(app.draft.color_text().as_deref(), Some("#E5484D"));
    let _ = app.run("tool.line");
    typed(&mut app, "487000,4420000");
    typed(&mut app, "487010,4420000");
    assert_eq!(newest_color(&app).as_deref(), Some("#E5484D"));
    let _ = app.update(Message::RibbonPanel(Event::Color(None)));
    typed(&mut app, "487010,4420010");
    assert_eq!(newest_color(&app), None, "by layer: no colour written");
}

#[test]
fn line_type_weight_and_scale_are_the_webs_fields() {
    let mut app = app_with_drawing();
    let _ = app.update(Message::RibbonPanel(Event::LineType(Some(
        LineType::Dashed,
    ))));
    let _ = app.update(Message::RibbonPanel(Event::Weight(Some(0.35))));
    assert_eq!(
        (app.new_line_type, app.draft.line_weight),
        (Some(LineType::Dashed), Some(0.35))
    );
    assert_eq!(line_type_text(app.new_line_type), "Kesikli");
    assert_eq!(weight_value_text(app.draft.line_weight), "0.35 mm");
    assert_eq!(weight_value_text(None), "Katmana göre");
    let color = |text| kentos_interaction::DraftColor::new(text);
    assert_eq!(color_text(color("#4F8EF7")), "Mavi");
    // An object template's colour that is none of the drawing colours (docs/adr/0176 §3).
    assert_eq!(color_text(color("#7a5c3e")), "#7A5C3E");
    // Ölçek is the project's: an edit of the drawing, no undo step (the web's).
    let _ = app.update(Message::RibbonPanel(Event::Scale(2000.0)));
    let doc = app.document.as_ref().expect("a drawing");
    assert_eq!(doc.settings().plot_scale, 2000.0);
    assert!(doc.dirty());
}

#[test]
fn the_seçim_tab_shows_with_a_selection_and_gives_the_ribbon_back_when_it_empties() {
    let mut app = app_with_drawing();
    app.tab = "edit";
    assert_eq!(app.shown_tab(), "edit");
    // Nothing selected: the contextual tab is not there to open.
    let _ = app.update(Message::RibbonTab("selection"));
    assert_eq!(app.shown_tab(), "edit");
    app.selection.set(vec![Slot(1), Slot(4)]);
    let _ = app.update(Message::RibbonTab("selection"));
    assert_eq!(app.shown_tab(), "selection");
    // The summary: how many, of which kinds, the most first.
    assert_eq!(app.selection_kinds().len(), 2);
    // Another tab keeps the selection; Seçim again, then the selection goes.
    let _ = app.update(Message::RibbonTab("map"));
    assert_eq!(app.shown_tab(), "map");
    let _ = app.update(Message::RibbonTab("selection"));
    let _ = app.update(Message::Run("edit.deselect"));
    assert_eq!(app.shown_tab(), "map", "back to the tab before it");
    // A new selection does not open it by itself (the web's).
    app.selection.set(vec![Slot(1)]);
    let _ = app.update(Message::RibbonPanel(Event::Color(None)));
    assert_eq!(app.shown_tab(), "map");
}

#[test]
fn the_selections_kinds_read_as_the_webs_lower_case_names() {
    let mut app = app_with_drawing();
    let doc = &app.document.as_ref().expect("a drawing").model;
    let slots: Vec<Slot> = doc
        .entities()
        .filter(|e| matches!(e.kind(), "ray" | "line" | "point"))
        .map(|e| Slot(e.base().id))
        .collect();
    app.selection.set(slots);
    let kinds: Vec<String> = app
        .selection_kinds()
        .into_iter()
        .map(|(n, kind)| format!("{n} {kind}"))
        .collect();
    // “Işın” is “ışın” in Turkish, not “işın”.
    assert!(kinds.contains(&"1 ışın".to_owned()), "{kinds:?}");
    assert!(kinds.contains(&"1 çizgi".to_owned()), "{kinds:?}");
}

/// Pictures of the panels for the owner, in `.run/shots/serit-panel-*`:
///
/// ```text
/// cargo test -p kentos-desktop ribbon_panels::tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let _typography = crate::appearance::tests::TYPOGRAPHY.lock();
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let picture = |app: &mut App, name: &str, width: f32| {
        let mut snapshot = Snapshot::new(Size::new(width, 900.0)).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(app, App::view, &mut update);
        let file = out.join(format!("{name}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());
    };
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        let mut app = app_with_drawing();
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
        app.apply_settings();
        app.tab = "home";
        for width in [1440.0, 1100.0] {
            picture(
                &mut app,
                &format!("serit-panel-giris-{width}{suffix}"),
                width,
            );
        }
        // Chosen values: a colour with its swatch, a line type, a weight, another scale.
        let _ = app.update(Message::RibbonPanel(Event::Color(Some("#E5484D"))));
        let _ = app.update(Message::RibbonPanel(Event::LineType(Some(
            LineType::Dashdot,
        ))));
        let _ = app.update(Message::RibbonPanel(Event::Weight(Some(0.35))));
        let _ = app.update(Message::RibbonPanel(Event::Scale(2000.0)));
        picture(
            &mut app,
            &format!("serit-panel-giris-secimli-1440{suffix}"),
            1440.0,
        );
        // Seçim, with four kinds selected (two shown and “2 tür daha”), then one kind.
        let doc = &app.document.as_ref().expect("a drawing").model;
        let slots: Vec<Slot> = doc
            .entities()
            .filter(|e| matches!(e.kind(), "ray" | "line" | "point" | "polygon"))
            .map(|e| Slot(e.base().id))
            .collect();
        app.selection.set(slots);
        let _ = app.update(Message::RibbonTab("selection"));
        for width in [1440.0, 1100.0] {
            picture(
                &mut app,
                &format!("serit-panel-secim-{width}{suffix}"),
                width,
            );
        }
        // Back on Giriş with the selection kept: the contextual tab waits beside the others.
        let _ = app.update(Message::RibbonTab("home"));
        picture(
            &mut app,
            &format!("serit-panel-secim-bekliyor-1440{suffix}"),
            1440.0,
        );
    }
}

/// Özellikler's lists hold the web's values (fixtures/shell/v1/ribbon.json
/// `fields`, moved there from the classic shell's file: docs/adr/0155): the
/// colours by name, the line types' names, the weights and the scales as
/// written.
#[test]
fn ozellikler_offers_the_webs_values() {
    let file: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/shell/v1/ribbon.json"))
            .expect("ribbon.json reads");
    let fields = &file["fields"];
    let colors: Vec<serde_json::Value> = DRAW_COLORS
        .iter()
        .map(|(name, value)| serde_json::json!({ "name": name, "value": value }))
        .collect();
    assert_eq!(serde_json::Value::from(colors), fields["colors"]);
    let types: serde_json::Map<String, serde_json::Value> = LINE_TYPES
        .iter()
        .map(|(t, name)| {
            let id = serde_json::to_value(t).expect("a line type's id");
            (
                id.as_str().unwrap_or_default().to_owned(),
                serde_json::Value::from(*name),
            )
        })
        .collect();
    assert_eq!(serde_json::Value::from(types), fields["lineTypes"]);
    let weights: Vec<serde_json::Value> = LINE_WEIGHTS
        .iter()
        .map(|&w| serde_json::json!({ "mm": w, "text": weight_text(w) }))
        .collect();
    assert_eq!(serde_json::Value::from(weights), fields["weights"]);
    let scales: Vec<serde_json::Value> = PLOT_SCALES
        .iter()
        .map(|&s| serde_json::json!({ "denominator": s as u64, "text": format!("1:{s}") }))
        .collect();
    assert_eq!(serde_json::Value::from(scales), fields["scales"]);
}
