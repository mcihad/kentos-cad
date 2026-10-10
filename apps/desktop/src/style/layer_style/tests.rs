//! Katman stili as the user drives it: opening (a layer, not a group), the
//! kinds and their drafts, classes from the data, rules and their counts,
//! Uygula and Tamam as one undo step each, a renderer this version cannot
//! read kept as it was, Esc.

use kentos_native_style::renderer::GeometryClass;
use kentos_native_style::tally::RuleCount;
use serde_json::{Value, json};

use super::{Event, Field, Kind, RuleEdit, SetAt, Source};
use crate::app::{App, Dialog, Message};
use crate::files_testing::{app_with_drawing, last_said};

fn ls(app: &mut App, e: Event) {
    let _ = app.update(Message::LayerStyle(e));
}

fn renderer_of(app: &App, id: &str) -> Option<Value> {
    app.document
        .as_ref()
        .and_then(|d| d.model.layers().get(id))
        .and_then(|n| n.style.renderer.clone())
}

fn window(app: &App) -> &super::LayerStyleWindow {
    app.styles.layer_style.as_ref().expect("the window is open")
}

#[test]
fn opens_for_a_layer_and_not_for_a_group() {
    let mut app = app_with_drawing();
    ls(&mut app, Event::Open(Some("layer-g".into())));
    assert_eq!(app.dialog, None);
    assert!(
        last_said(&app).contains("yalnızca katmanlar"),
        "{}",
        last_said(&app)
    );
    ls(&mut app, Event::Open(Some("parsel".into())));
    assert_eq!(app.dialog, Some(Dialog::LayerStyle));
    let w = window(&app);
    assert_eq!(w.kind, Kind::Simple);
    // The attribute most objects have is the first guess for categories.
    assert_eq!(w.categorized.expr, "Ada");
    assert_eq!(w.graduated.expr, "$alan");
    assert_eq!(w.classes, vec![GeometryClass::Fill]);
    // The first rule draws everything with the layer's own look.
    assert_eq!(w.rules.len(), 1);
    assert!(
        w.rules[0]
            .symbols
            .as_ref()
            .is_some_and(|s| s.fill.is_some())
    );
    assert_eq!(w.status(), None);
    // Esc closes it with nothing written.
    app.close_dialog();
    assert!(app.styles.layer_style.is_none());
    assert_eq!(renderer_of(&app, "parsel"), None);
}

#[test]
fn categories_from_values_are_applied_as_one_undo_step() {
    let mut app = app_with_drawing();
    ls(&mut app, Event::Open(Some("parsel".into())));
    ls(&mut app, Event::Kind(Kind::Categorized));
    ls(&mut app, Event::Expr(Field::Categories, "Parsel ".into()));
    ls(&mut app, Event::Classify);
    let w = window(&app);
    assert_eq!(w.categorized.categories.len(), 1);
    assert_eq!(w.categorized.categories[0].value, "7");
    assert_eq!(
        w.status(),
        Some(("Değişiklikler henüz uygulanmadı.".into(), true))
    );
    ls(&mut app, Event::Other(true));
    ls(&mut app, Event::Apply);
    let r = renderer_of(&app, "parsel").expect("a renderer");
    assert_eq!(r["type"], "categorized");
    // The typed blank is not kept.
    assert_eq!(r["expr"], "Parsel");
    assert_eq!(r["categories"][0]["value"], "7");
    assert_eq!(
        r["categories"][0]["symbols"]["fill"]["layers"][0]["color"],
        "#E15759"
    );
    assert_eq!(r["other"]["fill"]["layers"][0]["color"], "#BAB0AC");
    assert_eq!(
        window(&app).status(),
        Some(("Stil haritaya uygulandı.".into(), false))
    );
    // Uygula again with nothing changed writes nothing.
    ls(&mut app, Event::Apply);
    ls(&mut app, Event::Done);
    assert!(app.styles.layer_style.is_none());
    let model = &mut app.document.as_mut().expect("open").model;
    assert_eq!(model.undo().as_deref(), Some("Katman stili"));
    assert!(
        model
            .layers()
            .get("parsel")
            .expect("layer")
            .style
            .renderer
            .is_none()
    );
    assert!(!model.can_undo());
}

#[test]
fn every_kind_keeps_its_draft_and_basit_goes_back_to_the_simple_look() {
    let mut app = app_with_drawing();
    ls(&mut app, Event::Open(Some("parsel".into())));
    ls(&mut app, Event::Kind(Kind::Categorized));
    ls(&mut app, Event::AddCategory);
    ls(&mut app, Event::CategoryValue(0, "7".into()));
    ls(&mut app, Event::CategoryLabel(0, "Yedi".into()));
    ls(&mut app, Event::Kind(Kind::Rules));
    ls(&mut app, Event::Kind(Kind::Categorized));
    assert_eq!(window(&app).categorized.categories[0].label, "Yedi");
    ls(&mut app, Event::Done);
    assert!(renderer_of(&app, "parsel").is_some());
    ls(&mut app, Event::Open(Some("parsel".into())));
    assert_eq!(window(&app).kind, Kind::Categorized);
    ls(&mut app, Event::Kind(Kind::Simple));
    ls(&mut app, Event::Done);
    assert_eq!(renderer_of(&app, "parsel"), None);
    // `cad.layers.renderer`'s step, the renderer taken away (docs/adr/0213 §5).
    let model = &mut app.document.as_mut().expect("open").model;
    assert_eq!(model.undo().as_deref(), Some("Katman stili"));
}

#[test]
fn classes_of_a_number_and_their_bounds_as_typed() {
    let mut app = app_with_drawing();
    ls(&mut app, Event::Open(Some("parsel".into())));
    ls(&mut app, Event::Kind(Kind::Graduated));
    ls(&mut app, Event::Count("3".into()));
    ls(&mut app, Event::Ramp("maviler"));
    ls(&mut app, Event::Graduate);
    let w = window(&app);
    // One area: its value is both ends, one class.
    assert_eq!(w.graduated.classes.len(), 1);
    assert!(last_status(&app).contains("1 sınıf, 1 sayısal değerden."));
    ls(&mut app, Event::ClassMin(0, "12,5".into()));
    ls(&mut app, Event::ClassMax(0, "abc".into()));
    let c = &window(&app).graduated.classes[0];
    assert_eq!(c.min, 12.5);
    assert!(c.max > 12.5, "a bound that is not a number is not taken");
    assert_eq!(
        window(&app).typed.get("max/0").map(String::as_str),
        Some("abc")
    );
}

fn last_status(app: &App) -> String {
    window(app)
        .said
        .as_ref()
        .map(|(t, _)| t.clone())
        .unwrap_or_default()
}

#[test]
fn rules_are_edited_as_a_tree_and_count_what_they_draw() {
    let mut app = app_with_drawing();
    ls(&mut app, Event::Open(Some("cizim".into())));
    ls(&mut app, Event::Kind(Kind::Rules));
    ls(
        &mut app,
        Event::Expr(Field::Rule(vec![0]), "$tür = 'Nokta' ".into()),
    );
    ls(&mut app, Event::AddRule);
    ls(
        &mut app,
        Event::Rule(vec![1], RuleEdit::Label("Çizgiler".into())),
    );
    ls(&mut app, Event::Rule(vec![1], RuleEdit::AddChild));
    ls(&mut app, Event::Rule(vec![1, 0], RuleEdit::Else(true)));
    ls(&mut app, Event::AddElse);
    ls(
        &mut app,
        Event::Rule(vec![2], RuleEdit::MinScale("1.000".into())),
    );
    ls(
        &mut app,
        Event::Rule(vec![2], RuleEdit::MaxScale("25 000".into())),
    );
    let w = window(&app);
    assert_eq!(w.rules[0].filter.as_deref(), Some("$tür = 'Nokta'"));
    assert_eq!(w.rules[2].min_scale, Some(1000.0));
    assert_eq!(w.rules[2].max_scale, Some(25000.0));
    assert!(w.rules[1].children()[0].is_else());
    let doc = &app.document.as_ref().expect("open").model;
    let counts = w.rule_counts(&Source {
        doc,
        store: app.spatial.store(),
    });
    // The drawing's point, everything drawn for the new rule (not the text and the
    // dimension: the style engine does not draw them), what no sibling took.
    assert_eq!(counts.get(&vec![0]), Some(&RuleCount::Count(1)));
    let all = doc.count("cizim") - 2;
    assert_eq!(counts.get(&vec![1]), Some(&RuleCount::Count(all)));
    assert_eq!(counts.get(&vec![1, 0]), Some(&RuleCount::Count(all)));
    assert_eq!(counts.get(&vec![2]), Some(&RuleCount::Count(0)));
    // A condition that does not compile says where.
    ls(
        &mut app,
        Event::Expr(Field::Rule(vec![0]), "$alan >".into()),
    );
    let w = window(&app);
    let doc = &app.document.as_ref().expect("open").model;
    let counts = w.rule_counts(&Source {
        doc,
        store: app.spatial.store(),
    });
    assert!(matches!(counts.get(&vec![0]), Some(RuleCount::Error(_))));
    // Moved, removed; the first cannot go up.
    ls(&mut app, Event::Rule(vec![0], RuleEdit::Up));
    ls(&mut app, Event::Rule(vec![0], RuleEdit::Down));
    assert_eq!(window(&app).rules[1].label, "Bütün nesneler");
    ls(&mut app, Event::Rule(vec![1], RuleEdit::Remove));
    assert_eq!(window(&app).rules.len(), 2);
    ls(&mut app, Event::Done);
    let r = renderer_of(&app, "cizim").expect("rules");
    assert_eq!(r["type"], "rules");
    assert_eq!(r["rules"][0]["label"], "Çizgiler");
    assert_eq!(r["rules"][0]["children"][0]["isElse"], true);
    assert_eq!(r["rules"][1]["maxScale"], 25000.0);
}

#[test]
fn a_slot_goes_back_to_the_simple_look() {
    let mut app = app_with_drawing();
    ls(&mut app, Event::Open(Some("parsel".into())));
    ls(&mut app, Event::Kind(Kind::Single));
    assert!(window(&app).single.fill.is_some());
    ls(
        &mut app,
        Event::Symbol(SetAt::Single, GeometryClass::Fill, None),
    );
    assert!(window(&app).single.fill.is_none());
    ls(
        &mut app,
        Event::Symbol(
            SetAt::Single,
            GeometryClass::Fill,
            Some(json!({ "ref": "x" })),
        ),
    );
    ls(&mut app, Event::Done);
    assert_eq!(
        renderer_of(&app, "parsel"),
        Some(json!({ "type": "single", "symbols": { "fill": { "ref": "x" } } }))
    );
}

#[test]
fn a_renderer_this_version_cannot_read_is_kept() {
    let mut app = app_with_drawing();
    let odd = json!({ "type": "voronoi", "cells": 5 });
    {
        let model = &mut app.document.as_mut().expect("open").model;
        let mut style = model.layers().get("parsel").expect("layer").style.clone();
        style.renderer = Some(odd.clone());
        model.set_layer_style("parsel", style, "Katman stili");
    }
    ls(&mut app, Event::Open(Some("parsel".into())));
    assert_eq!(window(&app).kind, Kind::Unknown);
    assert_eq!(window(&app).status(), None);
    ls(&mut app, Event::Done);
    assert_eq!(renderer_of(&app, "parsel"), Some(odd));
}

#[test]
fn the_command_opens_the_active_layer_s_window() {
    let mut app = app_with_drawing();
    assert!(app.available("style.layerStyle"));
    let _ = app.update(Message::Run("style.layerStyle"));
    assert_eq!(app.dialog, Some(Dialog::LayerStyle));
    let active = app
        .document
        .as_ref()
        .expect("open")
        .model
        .layers()
        .active()
        .to_owned();
    assert_eq!(window(&app).layer, active);
}

#[test]
fn closing_with_changes_not_applied_asks_first() {
    let mut app = app_with_drawing();
    ls(&mut app, Event::Open(Some("parsel".into())));
    // Nothing changed: Esc closes at once.
    app.close_dialog();
    assert!(app.styles.layer_style.is_none());
    assert_eq!(app.dialog, None);

    ls(&mut app, Event::Open(Some("parsel".into())));
    ls(&mut app, Event::Kind(Kind::Single));
    ls(&mut app, Event::Close);
    assert!(window(&app).asking, "Vazgeç asks");
    assert_eq!(app.dialog, Some(Dialog::LayerStyle));
    // Asked, the kinds stay, and Esc answers “stay”.
    ls(&mut app, Event::Step(true));
    assert_eq!(window(&app).kind, Kind::Single);
    app.close_dialog();
    assert!(!window(&app).asking);
    assert_eq!(app.dialog, Some(Dialog::LayerStyle));
    // Uygulamadan kapat drops the draft.
    ls(&mut app, Event::Close);
    ls(&mut app, Event::Discard);
    assert!(app.styles.layer_style.is_none());
    assert_eq!(app.dialog, None);
    assert_eq!(renderer_of(&app, "parsel"), None);

    // Uygula ve kapat writes it.
    ls(&mut app, Event::Open(Some("parsel".into())));
    ls(&mut app, Event::Kind(Kind::Single));
    ls(&mut app, Event::Close);
    ls(&mut app, Event::Done);
    assert!(app.styles.layer_style.is_none());
    assert_eq!(
        renderer_of(&app, "parsel").map(|r| r["type"].clone()),
        Some(json!("single"))
    );
}

#[test]
fn arrows_step_through_the_kinds() {
    let mut app = app_with_drawing();
    ls(&mut app, Event::Open(Some("parsel".into())));
    ls(&mut app, Event::Step(true));
    assert_eq!(window(&app).kind, Kind::Single);
    for _ in 0..6 {
        ls(&mut app, Event::Step(true));
    }
    assert_eq!(window(&app).kind, Kind::Rules);
    for _ in 0..10 {
        ls(&mut app, Event::Step(true));
    }
    assert_eq!(window(&app).kind, Kind::Inverted, "stops at the last");
    ls(&mut app, Event::Step(false));
    assert_eq!(window(&app).kind, Kind::Displacement);
}
