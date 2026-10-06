//! A group template's members (docs/adr/0176 §5; the web's
//! `tools/templateMembers.ts`): while a group template runs, every object its
//! tool writes takes its members' objects in the same undo step (the group
//! `input.rs`'s `with_tool` opens around the tool): the same shape on the
//! member's layer, its parallels, points at its vertices (Köşelere nokta's
//! rule: a shared corner once, a corner with a point passed over), a point or
//! a text at its centroid. A member's point names and texts go on in its
//! series (`App::template_names`, by the member template's id). A member that
//! makes nothing, or whose layer refuses it, is said; the object and the
//! other members stay.

use std::collections::BTreeMap;

use kentos_contracts::{
    CommandResult, EntitiesCreate, Entity, EntityGeometry, NewObject, TextAlign, Vec2 as Wire,
};
use kentos_domain::Slot;
use kentos_geometry_core::entity::entity_anchor;
use kentos_geometry_core::ops::template_members::{Side, member_offsets};
use kentos_geometry_core::ops::vertex_points::vertex_points;
use kentos_geometry_core::text::edit::increment;
use kentos_geometry_core::vec2::Vec2;
use kentos_interaction::Level;
use kentos_interaction::templates::Stamp;
use kentos_native_application::elevation::paths as elevated_paths;
use kentos_native_application::geometry::{edit_geometry, shape};
use kentos_native_application::{ExecutionContext, create};
use kentos_native_style::object_template::{Recipe, RecipeMember};

use crate::app::App;

/// A group template's member as the run writes it: its template's id (a
/// point or text template's series is kept by it) and name, its rule, the
/// layer its objects go on (found or opened when the group started), their
/// look, attributes and label, and a point template's first name and code or
/// a text template's height on paper, alignment and mask.
#[derive(Clone, Debug)]
pub(crate) struct RunMember {
    id: String,
    name: String,
    rule: String,
    distance: Option<f64>,
    side: Option<String>,
    layer_id: String,
    color: Option<String>,
    line_weight: Option<f64>,
    stamp: Stamp,
    point: Option<(Option<String>, String)>,
    text: Option<(f64, Option<TextAlign>, bool)>,
}

impl RunMember {
    /// The member `m` of a group, its template named `name` and read as `own`,
    /// writing on `layer_id`.
    pub(crate) fn new(m: &RecipeMember, name: String, own: &Recipe, layer_id: String) -> Self {
        RunMember {
            id: m.template.clone(),
            name,
            rule: m.rule.clone(),
            distance: m.distance,
            side: m.side.clone(),
            layer_id,
            color: own.color.clone(),
            line_weight: own.line_weight,
            stamp: Stamp {
                symbol: own.symbol.clone(),
                attrs: own.attrs.clone(),
                label: own.label.clone(),
            },
            point: (own.tool == "point").then(|| {
                (
                    own.point_name.clone().filter(|n| !n.is_empty()),
                    own.point_code.clone().unwrap_or_default(),
                )
            }),
            text: (own.tool == "text")
                .then_some(())
                .and(own.text_height)
                .map(|height| {
                    (
                        height,
                        own.text_align.as_deref().and_then(TextAlign::from_name),
                        own.text_mask,
                    )
                }),
        }
    }
}

fn wire(p: Vec2) -> Wire {
    Wire { x: p.x, y: p.y }
}

impl App {
    /// Every member's objects for the objects the tool wrote (`made`), each
    /// member on its layer, through `cad.entities.create`; how many were
    /// written is said.
    pub(crate) fn write_members(&mut self, made: &[Slot]) {
        let Some(run) = self.template.as_ref() else {
            return;
        };
        let (run_name, members) = (run.name.clone(), run.members.clone());
        let mut said = Vec::new();
        let mut count = 0;
        for &slot in made {
            for m in &members {
                let objects = self.member_objects(m, slot, &mut said);
                if objects.is_empty() {
                    continue;
                }
                let Some(doc) = self.document.as_mut() else {
                    return;
                };
                let n = objects.len();
                let input = EntitiesCreate {
                    layer_id: m.layer_id.clone(),
                    objects,
                    operation: None,
                    expected_revision: None,
                };
                match create::execute(&mut ExecutionContext::new(&mut doc.model), input) {
                    CommandResult::Completed { warnings, .. } => {
                        said.extend(warnings.into_iter().map(|w| w.message));
                        count += n;
                    }
                    CommandResult::Failed { error }
                    | CommandResult::Conflict { error }
                    | CommandResult::NeedsInput { error } => {
                        said.push(format!("“{}” üyesi yazılmadı: {}", m.name, error.message));
                    }
                    CommandResult::Queued { .. } | CommandResult::Cancelled => {}
                }
            }
        }
        for text in said {
            self.warn(text);
        }
        if count > 0 {
            self.say(
                Level::Info,
                format!("{run_name}: {count} üye nesnesi yazıldı."),
            );
        }
    }

    /// What one member makes from the object at `slot`; why it makes nothing
    /// goes to `said`.
    fn member_objects(
        &mut self,
        m: &RunMember,
        slot: Slot,
        said: &mut Vec<String>,
    ) -> Vec<NewObject> {
        let Some(doc) = self.document.as_ref() else {
            return Vec::new();
        };
        let Some(main) = doc.model.get(slot) else {
            return Vec::new();
        };
        let drawn = shape(main);
        let code = m
            .point
            .as_ref()
            .filter(|(_, code)| !code.is_empty())
            .map(|(_, code)| BTreeMap::from([("Kod".to_owned(), code.clone())]));
        match m.rule.as_str() {
            "same" => edit_geometry(drawn)
                .map(|g| vec![object(m, g, code_none(), Label::Stamp)])
                .unwrap_or_default(),
            "offset" => {
                let side = m
                    .side
                    .as_deref()
                    .and_then(Side::from_name)
                    .unwrap_or(Side::Both);
                match member_offsets(&drawn, m.distance.unwrap_or(0.0), side) {
                    Ok(shapes) => shapes
                        .into_iter()
                        .filter_map(edit_geometry)
                        .map(|g| object(m, g, code_none(), Label::Stamp))
                        .collect(),
                    Err(error) => {
                        said.push(format!("“{}” üyesi yazılmadı: {error}", m.name));
                        Vec::new()
                    }
                }
            }
            "vertices" => {
                // Every point's place is taken, hidden layers' too (Köşelere nokta's rule).
                let existing: Vec<Vec2> = doc
                    .model
                    .entities()
                    .filter_map(|e| match e {
                        Entity::Point(q) => Some(Vec2::new(q.p.x, q.p.y)),
                        _ => None,
                    })
                    .collect();
                let first = self
                    .template_names
                    .get(&m.id)
                    .cloned()
                    .or_else(|| m.point.as_ref().and_then(|(name, _)| name.clone()));
                let found = vertex_points(
                    &[elevated_paths(main)],
                    &existing,
                    first.as_deref().filter(|n| !n.is_empty()),
                );
                if let Some(next) = &found.next {
                    self.template_names.insert(m.id.clone(), next.clone());
                }
                found
                    .points
                    .into_iter()
                    .map(|p| {
                        let g = EntityGeometry::Point {
                            p: wire(p.p),
                            z: p.z,
                            parts: None,
                        };
                        let label = p.name.map_or(Label::Stamp, Label::Own);
                        object(m, g, code.clone(), label)
                    })
                    .collect()
            }
            "centroid" => {
                let Some(at) = entity_anchor(&drawn) else {
                    return Vec::new();
                };
                if let Some((height_mm, align, mask)) = m.text {
                    let Some(text) = self
                        .template_names
                        .get(&m.id)
                        .cloned()
                        .or_else(|| m.stamp.label.clone())
                        .filter(|t| !t.is_empty())
                    else {
                        return Vec::new();
                    };
                    let next = increment(&text).unwrap_or_else(|| text.clone());
                    let height = height_mm * doc.settings().plot_scale / 1000.0;
                    self.template_names.insert(m.id.clone(), next);
                    let g = EntityGeometry::Text {
                        p: wire(at),
                        text,
                        height,
                        rotation: 0.0,
                        align,
                        width_factor: None,
                        mask,
                        box_width: None,
                        line_spacing: None,
                        runs: Vec::new(),
                        face: Default::default(),
                    };
                    return vec![object(m, g, code_none(), Label::None)];
                }
                let name = self
                    .template_names
                    .get(&m.id)
                    .cloned()
                    .or_else(|| m.point.as_ref().and_then(|(name, _)| name.clone()))
                    .filter(|n| !n.is_empty());
                if let Some(name) = &name {
                    let next = increment(name).unwrap_or_else(|| name.clone());
                    self.template_names.insert(m.id.clone(), next);
                }
                let g = EntityGeometry::Point {
                    p: wire(at),
                    z: None,
                    parts: None,
                };
                vec![object(m, g, code, name.map_or(Label::Stamp, Label::Own))]
            }
            _ => Vec::new(),
        }
    }
}

/// The label an object takes: its own (a point's name), the member
/// template's, or none (a text's label is its first text).
enum Label {
    Own(String),
    Stamp,
    None,
}

fn code_none() -> Option<BTreeMap<String, String>> {
    None
}

/// An object with the member's look, attributes (`own`, the object's, over
/// them) and label.
fn object(
    m: &RunMember,
    geometry: EntityGeometry,
    own: Option<BTreeMap<String, String>>,
    label: Label,
) -> NewObject {
    let mut attrs = m.stamp.attrs.clone();
    attrs.extend(own.unwrap_or_default());
    NewObject {
        line_weight: geometry.draws_lines().then_some(m.line_weight).flatten(),
        geometry,
        color: m.color.clone(),
        attrs: (!attrs.is_empty()).then_some(attrs),
        label: match label {
            Label::Own(name) => Some(name),
            Label::Stamp => m.stamp.label.clone(),
            Label::None => None,
        },
        label_of: None,
        label_scale: None,
        symbol: m.stamp.symbol.clone(),
    }
}
