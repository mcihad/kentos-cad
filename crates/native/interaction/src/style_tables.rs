//! The project's text and dimension styles saved (docs/adr/0183 §1, §5; the
//! web's `app/styleTables.ts`): the table becomes the project's (an edit,
//! never an undo step), then the objects follow in one undo step “Yazı
//! stili” or “Ölçü stili” (`cad.entities.edit`'s `textStyle` and
//! `dimensionStyle`): those of a style whose values changed take the new
//! values where they still held the old ones (`follow_text_style`,
//! `follow_dimension_style`), those of a style deleted lose their link and
//! keep their look. Objects on locked layers are left as they are and counted.

use std::collections::HashMap;

use kentos_contracts::{
    DimensionStyleDef, EditOperation, EntitiesEdit, Entity, EntityEdit, TextLook, TextStyleDef,
    follow_dimension_style, follow_text_style,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::geometry::{edit_geometry, shape};
use kentos_native_application::{ExecutionContext, edit};

/// What saving did: the objects that followed, those left on locked layers,
/// and what to say (the command's refusal).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Saved {
    pub changed: usize,
    pub locked: usize,
    pub said: Vec<String>,
}

/// How many texts (or dimensions) follow each style, by its id.
pub fn usage(doc: &Document, text: bool) -> HashMap<String, usize> {
    let mut out = HashMap::new();
    for e in doc.entities() {
        let id = match e {
            Entity::Text(t) if text => t.face.text_style.as_ref(),
            Entity::Dimension(d) if !text => d.look.dim_style.as_ref(),
            _ => None,
        };
        if let Some(id) = id {
            *out.entry(id.clone()).or_insert(0) += 1;
        }
    }
    out
}

/// Writes the objects as one step named after `operation`; those on locked
/// layers are counted, not written.
fn follow(doc: &mut Document, operation: EditOperation, updates: Vec<(Slot, Entity)>) -> Saved {
    let mut locked = 0;
    let mut changes = Vec::new();
    for (slot, entity) in updates {
        if doc.layers().is_locked(&entity.base().layer_id) {
            locked += 1;
            continue;
        }
        let (Some(uid), Some(geometry)) = (doc.uid(slot), edit_geometry(shape(&entity))) else {
            continue;
        };
        changes.push(EntityEdit::Update {
            uid: uid.to_string(),
            geometry,
        });
    }
    if changes.is_empty() {
        return Saved {
            locked,
            ..Saved::default()
        };
    }
    let n = changes.len();
    let input = EntitiesEdit {
        operation,
        changes,
        expected_revision: None,
    };
    match edit::execute(&mut ExecutionContext::new(doc), input) {
        kentos_contracts::CommandResult::Completed { warnings, .. } => Saved {
            changed: n,
            locked,
            said: warnings.into_iter().map(|w| w.message).collect(),
        },
        other => Saved {
            changed: 0,
            locked,
            said: crate::properties::said_of(other),
        },
    }
}

/// Saves the text styles: the table, then the texts that follow them (Yazı stili).
pub fn save_text_styles(doc: &mut Document, next: Vec<TextStyleDef>) -> Saved {
    let mut settings = doc.settings().clone();
    let before: HashMap<String, TextStyleDef> = settings
        .text_styles
        .iter()
        .map(|s| (s.id.clone(), s.clone()))
        .collect();
    settings.text_styles = next;
    doc.set_settings(settings);
    let after: HashMap<String, TextStyleDef> = doc
        .settings()
        .text_styles
        .iter()
        .map(|s| (s.id.clone(), s.clone()))
        .collect();
    let scale = doc.settings().plot_scale;
    let mut updates = Vec::new();
    for e in doc.entities() {
        let Entity::Text(t) = e else { continue };
        let Some(old) = t.face.text_style.as_ref().and_then(|id| before.get(id)) else {
            continue;
        };
        let mut t = t.clone();
        match after.get(&old.id) {
            // A style deleted: the text keeps its look, without the link.
            None => t.face.text_style = None,
            Some(now) if now == old => continue,
            Some(now) => {
                let look = follow_text_style(
                    old,
                    now,
                    &TextLook {
                        face: t.face.clone(),
                        width_factor: t.width_factor,
                        height: t.height,
                    },
                    scale,
                );
                t.face = look.face;
                t.width_factor = look.width_factor;
                t.height = look.height;
            }
        }
        updates.push((Slot(t.base.id), Entity::Text(t)));
    }
    follow(doc, EditOperation::TextStyle, updates)
}

/// Saves the dimension styles: the table, then the dimensions that follow them (Ölçü stili).
pub fn save_dimension_styles(doc: &mut Document, next: Vec<DimensionStyleDef>) -> Saved {
    let mut settings = doc.settings().clone();
    let before: HashMap<String, DimensionStyleDef> = settings
        .dimension_styles
        .iter()
        .map(|s| (s.id.clone(), s.clone()))
        .collect();
    settings.dimension_styles = next;
    doc.set_settings(settings);
    let after: HashMap<String, DimensionStyleDef> = doc
        .settings()
        .dimension_styles
        .iter()
        .map(|s| (s.id.clone(), s.clone()))
        .collect();
    let scale = doc.settings().plot_scale;
    let mut updates = Vec::new();
    for e in doc.entities() {
        let Entity::Dimension(d) = e else { continue };
        let Some(old) = d.look.dim_style.as_ref().and_then(|id| before.get(id)) else {
            continue;
        };
        let mut d = d.clone();
        match after.get(&old.id) {
            None => d.look.dim_style = None,
            Some(now) if now == old => continue,
            Some(now) => {
                let (look, height) = follow_dimension_style(old, now, &d.look, d.height, scale);
                d.look = look;
                d.height = height;
            }
        }
        updates.push((Slot(d.base.id), Entity::Dimension(d)));
    }
    follow(doc, EditOperation::DimensionStyle, updates)
}
