//! Öznitelikler over the selection traces' drawing (fixtures/interaction/v1/objects.kcad):
//! what it shows for no object, one and several, and what its edits write.
//! Expected values are worked out by hand from the drawing.

use std::collections::BTreeMap;

use iced::keyboard::Modifiers;
use kentos_commands::{
    DimensionEntity, Entity, EntityBase, HatchEntity, HatchPattern, HatchPatternType, LineEntity,
    TextEntity, Vec2 as Wire,
};
use kentos_domain::Slot;

use super::{Choice, DimensionOp, Editor, Event, Field, Part, Row, Spot, Summary, web_number};
use crate::app::{App, Message};
use crate::selecting::tests::{click, objects};

const E: f64 = 487000.0;
const N: f64 = 4420000.0;

fn wall_dimension_set() -> (App, Slot, Slot) {
    use kentos_commands::{CallOp, DrawingUnit};
    use kentos_native_application::{ExecutionContext, registry};
    use serde_json::json;
    let mut app = objects();
    crate::files_testing::make_cad(&mut app);
    let model = &mut app.document.as_mut().unwrap().model;
    let mut settings = model.settings().clone();
    settings.drawing_unit = Some(DrawingUnit::Mm);
    settings.srid = 0;
    model.set_settings(settings);
    let wall = Slot(add(
        &mut app,
        serde_json::from_value(json!({
            "kind":"wall","id":0,"layerId":"parsel","attrs":{},
            "pts":[{"x":0.,"y":0.},{"x":20.,"y":0.}],"width":0.3,"offset":1.2
        }))
        .unwrap(),
    ));
    let model = &mut app.document.as_mut().unwrap().model;
    let uid = model.uid(wall).unwrap().to_string();
    // A door at its middle: the openings row is two dimensions, the overall row another (docs/adr/0695's 27 October
    // addendum: without an opening the two rows would be one dimension).
    let door = registry::run(
        &mut ExecutionContext::new(model),
        "cad.openings.create",
        CallOp::Execute,
        json!({"layerId":"parsel","openings":[{"host":uid,"at":10.0}]}),
    )
    .unwrap();
    assert_eq!(door["status"], "completed", "{door}");
    let result=registry::run(&mut ExecutionContext::new(model),"cad.dimensionsets.create",CallOp::Execute,json!({
        "wall":uid,"layerId":"parsel","at":{"x":10.,"y":2.4},"rows":["openings","overall"],"openings":"centers","gapMm":7.5
    })).unwrap();
    assert_eq!(result["status"], "completed", "{result}");
    let member = model.dimension_sets()[0].members[0]
        .uid
        .to_text()
        .parse()
        .unwrap();
    let member = model.slot_of(member).unwrap();
    app.selection.set([member]);
    (app, wall, member)
}

#[test]
fn wall_dimension_inspector_uses_face_offset_moves_one_row_and_preserves_text() {
    let (mut app, _, member) = wall_dimension_set();
    let shown: Vec<_> = rows(&app).into_iter().flat_map(|(_, rows)| rows).collect();
    assert!(shown.iter().any(|row| row.label == "Ölçü dizisi"));
    assert!(shown.iter().any(|row| row.label == "Sıra uzaklığı"));
    assert!(
        shown
            .iter()
            .any(|row| row.label == "Sıra aralığı" && row.unit.as_deref() == Some("mm · kağıtta"))
    );
    event(
        &mut app,
        Event::Commit(Field::DimensionText(member), "Özel <>".into()),
    );
    event(
        &mut app,
        Event::Commit(Field::DimensionOffset(member), "40 cm".into()),
    );
    let model = &app.document.as_ref().unwrap().model;
    let (set, index) = model
        .dimension_set_of_item(model.uid(member).unwrap())
        .unwrap();
    assert!(
        (model
            .dimension_set_row_offset(set, set.members[index].row)
            .unwrap()
            - 0.4)
            .abs()
            < 1e-10
    );
    assert_eq!(
        dimension_of(&app, member.0).text.as_deref(),
        Some("Özel <>")
    );
    event(
        &mut app,
        Event::Commit(Field::DimensionSetGap(member), "12 mm".into()),
    );
    assert_eq!(
        app.document.as_ref().unwrap().model.dimension_sets()[0].gap_mm,
        12.
    );
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    event(
        &mut app,
        Event::Commit(Field::DimensionSetGap(member), "12 cm".into()),
    );
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
}

/// docs/adr/0730 §7: Pafta ölçekleri typed as Diğer ölçekler are, Sığmayan yazı chosen, and Yazıları tarife döndür: each
/// one step of `cad.dimensionsets.edit`; a scale below 1 changes nothing and says why.
#[test]
fn a_recipes_sheet_scales_and_tight_values_are_set_from_the_inspector() {
    let (mut app, _, member) = wall_dimension_set();
    let shown: Vec<_> = rows(&app).into_iter().flat_map(|(_, rows)| rows).collect();
    let scales = shown.iter().find(|row| row.label == "Pafta ölçekleri").expect("the sheets row");
    assert_eq!(scales.value, "Yok");
    assert!(shown.iter().any(|row| row.label == "Sığmayan yazı" && row.value == "Yerinde"));
    event(
        &mut app,
        Event::Commit(Field::DimensionSetScales(member), "1:200, 100".into()),
    );
    let model = &app.document.as_ref().unwrap().model;
    assert_eq!(model.dimension_sets()[0].scales, [100.0, 200.0]);
    let d = dimension_of(&app, member.0);
    assert_eq!(d.annotative.as_deref().map(|a| a.scales.len()), Some(2));
    let shown: Vec<_> = rows(&app).into_iter().flat_map(|(_, rows)| rows).collect();
    assert!(shown.iter().any(|row| row.label == "Pafta ölçekleri" && row.value == "1:100, 1:200"));
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    event(
        &mut app,
        Event::Commit(Field::DimensionSetScales(member), "1:0.5".into()),
    );
    assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(), before);
    event(
        &mut app,
        Event::DimensionSetValue(member, "tight", "shift".into()),
    );
    assert_eq!(
        app.document.as_ref().unwrap().model.dimension_sets()[0].tight,
        kentos_commands::DimensionSetTight::Shift
    );
    event(&mut app, Event::DimensionSet(member, "resetText"));
    event(
        &mut app,
        Event::Commit(Field::DimensionSetScales(member), "Yok".into()),
    );
    let model = &app.document.as_ref().unwrap().model;
    assert!(model.dimension_sets()[0].scales.is_empty());
    assert!(dimension_of(&app, member.0).annotative.is_none(), "the representations go with the scales");
}

#[test]
fn wall_dimension_set_selection_is_read_only_and_release_removes_recipe_ownership() {
    let (mut app, wall, member) = wall_dimension_set();
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    event(&mut app, Event::DimensionSet(member, "select"));
    assert_eq!(app.selection.len(), 3, "the openings row's two and the overall row's one");
    assert!(
        rows(&app)
            .iter()
            .any(|(section, _)| section == "Duvar ölçüleri")
    );
    event(&mut app, Event::DimensionSet(member, "row"));
    assert_eq!(app.selection.len(), 2, "the openings row");
    assert!(app.selection.ids().contains(&member));
    event(&mut app, Event::DimensionSet(member, "source"));
    assert_eq!(app.selection.ids(), [wall]);
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
    app.selection.set([member]);
    event(&mut app, Event::DimensionFree(vec![member]));
    let model = &app.document.as_ref().unwrap().model;
    assert!(model.dimension_sets().is_empty());
    assert!(matches!(model.get(member), Some(Entity::Dimension(_))));
    assert!(
        model
            .dimension_set_of_item(model.uid(member).unwrap())
            .is_none()
    );
    assert!(undo_label(&mut app).is_some());
    assert_eq!(
        app.document.as_ref().unwrap().model.dimension_sets().len(),
        1
    );
}

fn rows(app: &App) -> Vec<(String, Vec<Row>)> {
    let doc = app.document.as_ref().expect("open");
    app.properties_panel(doc)
        .sections
        .into_iter()
        .map(|s| (s.title.into_owned(), s.rows))
        .collect()
}

/// A row's value by its section and label.
fn value(app: &App, section: &str, label: &str) -> String {
    rows(app)
        .into_iter()
        .find(|(title, _)| title == section)
        .and_then(|(_, rows)| rows.into_iter().find(|r| r.label == label))
        .map(|r| {
            let unit = r.unit.map(|u| format!(" {u}")).unwrap_or_default();
            format!("{}{unit}", r.value)
        })
        .unwrap_or_else(|| panic!("{section} / {label}"))
}

/// Whether any row can be edited.
fn editable(app: &App) -> bool {
    rows(app)
        .iter()
        .any(|(_, rows)| rows.iter().any(|r| r.editor.is_some()))
}

fn select(app: &mut App, slots: &[u32]) {
    app.selection
        .set(slots.iter().map(|s| Slot(*s)).collect::<Vec<_>>());
}

fn event(app: &mut App, event: Event) {
    let _ = app.update(Message::Properties(event));
}

fn entity(app: &App, slot: u32) -> Entity {
    app.document
        .as_ref()
        .and_then(|d| d.model.get(Slot(slot)))
        .cloned()
        .expect("the object")
}

fn undo_label(app: &mut App) -> Option<String> {
    app.document.as_mut().and_then(|d| d.model.undo())
}

fn base(layer: &str) -> EntityBase {
    EntityBase {
        look: Default::default(),
        id: 0,
        layer_id: layer.to_owned(),
        color: None,
        attrs: BTreeMap::new(),
        label: None,
        symbol: None,
        line_weight: None,
        label_pins: Vec::new(),
        attachments: Vec::new(),
    }
}

fn add(app: &mut App, e: Entity) -> u32 {
    app.document
        .as_mut()
        .expect("open")
        .model
        .add(e)
        .expect("a slot")
        .0
}

/// Where the text's Metin row (Genel closed) and the parcel's Katman row are
/// in the 1440 × 900 window (read off the pictures).
const YAZI_Y: f32 = 738.0;
const KATMAN_Y: f32 = 674.0;

mod objects;
use objects::*;

mod edits;
use edits::*;

mod elevations;
use elevations::*;

mod annotations;
use annotations::*;

mod parameters;

mod arch;
