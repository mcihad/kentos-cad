//! The project's text and dimension styles saved (docs/adr/0183 §1, §5; the
//! web's `app/styleTables.ts`; the Yazı stilleri and Ölçü stilleri windows and
//! `cad.styles.edit`, docs/adr/0248 §4): the table becomes the project's (an edit,
//! never an undo step), then the objects follow in one undo step “Yazı
//! stili” or “Ölçü stili” (`cad.entities.edit`'s `textStyle` and
//! `dimensionStyle`): those of a style whose values changed take the new
//! values where they still held the old ones (`follow_text_style`,
//! `follow_dimension_style`), those of a style deleted lose their link and
//! keep their look. Objects on locked layers are left as they are and counted.

use std::collections::HashMap;

use crate::geometry::{edit_geometry, shape};
use crate::{ExecutionContext, edit};
use kentos_commands::{
    DimensionStyleDef, EditOperation, EntitiesEdit, Entity, EntityEdit, LeaderStyleDef, TextLook, TextStyleDef,
    follow_dimension_style, follow_leader_style, follow_text_style,
};
use kentos_domain::{Document, Slot};

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
        kentos_commands::CommandResult::Completed { warnings, .. } => Saved {
            changed: n,
            locked,
            said: warnings.into_iter().map(|w| w.message).collect(),
        },
        other => Saved {
            changed: 0,
            locked,
            said: crate::said_of(other),
        },
    }
}

/// Saves the leader styles (docs/adr/0326 §4): the table, then the leaders that follow them, in one step “Kılavuz
/// stili”; those on locked layers left and counted; a removed style's leaders keep their look and follow none.
pub fn save_leader_styles(doc: &mut Document, next: Vec<LeaderStyleDef>) -> Saved {
    let mut settings = doc.settings().clone();
    let before: HashMap<String, LeaderStyleDef> =
        settings.leader_styles.iter().map(|s| (s.id.clone(), s.clone())).collect();
    settings.leader_styles = next;
    doc.set_settings(settings);
    let after: HashMap<String, LeaderStyleDef> =
        doc.settings().leader_styles.iter().map(|s| (s.id.clone(), s.clone())).collect();
    let scale = doc.settings().plot_scale;
    let (mut updates, mut locked) = (Vec::new(), 0);
    for e in doc.entities() {
        let Entity::Leader(l) = e else { continue };
        let Some(old) = l.leader_style.as_ref().and_then(|id| before.get(id)) else {
            continue;
        };
        let next = match after.get(&old.id) {
            None => kentos_commands::LeaderEntity { leader_style: None, ..l.clone() },
            Some(now) if now == old => continue,
            Some(now) => follow_leader_style(old, now, l, scale),
        };
        if next == *l {
            continue;
        }
        if doc.layers().is_locked(&l.base.layer_id) {
            locked += 1;
            continue;
        }
        updates.push((Slot(l.base.id), Entity::Leader(next)));
    }
    let changed = updates.len();
    if changed > 0 {
        doc.update_many(updates, "Kılavuz stili");
    }
    Saved { changed, locked, said: Vec::new() }
}

/// The walls that take a style table, and how many a locked layer keeps.
pub type WallFollowers = (Vec<(Slot, Entity)>, usize);

/// The walls that follow the wall styles `next` (docs/adr/0339 §2): a wall of a style changed takes its components,
/// ends and fill, and the width they fix (its own when one component takes the rest); a wall of a style removed keeps
/// its look without the link. Walls on locked layers are counted, not changed. A wall the new components do not fit,
/// or whose faces they make undrawable, is the refusal (its code and words).
pub fn wall_followers(doc: &Document, next: &[kentos_commands::WallStyleDef]) -> Result<WallFollowers, (&'static str, String)> {
    let before: HashMap<&str, &kentos_commands::WallStyleDef> =
        doc.settings().wall_styles.iter().map(|s| (s.id.as_str(), s)).collect();
    let after: HashMap<&str, &kentos_commands::WallStyleDef> = next.iter().map(|s| (s.id.as_str(), s)).collect();
    let (mut updates, mut locked) = (Vec::new(), 0);
    for e in doc.entities() {
        let Entity::Wall(w) = e else { continue };
        let Some(old) = w.wall.style.as_deref().and_then(|id| before.get(id)) else {
            continue;
        };
        let mut next = w.clone();
        match after.get(old.id.as_str()) {
            None => next.wall.style = None,
            Some(now) if now == old => continue,
            Some(now) => {
                next.wall.components = now.components.clone();
                next.wall.caps = now.caps;
                next.wall.fill = now.fill.clone();
                if let Some(width) = kentos_commands::fixed_width(&now.components) {
                    next.wall.width = width;
                }
            }
        }
        if next == *w {
            continue;
        }
        if doc.layers().is_locked(&w.base.layer_id) {
            locked += 1;
            continue;
        }
        let uid = doc.uid(Slot(w.base.id)).map(|u| u.to_string()).unwrap_or_default();
        if let Some((_, why)) = next.wall.problem() {
            return Err((crate::codes::INVALID_WALL, format!("“{uid}” kimlikli duvar stilin yeni bileşenleriyle olmaz: {why}")));
        }
        let spec = kentos_formats::walls::spec(&next.wall);
        let pts = crate::walls::core_points(&next.wall.pts);
        if let Err(why) = kentos_geometry_core::geom::wall::lines(&pts, next.wall.bulges.as_deref(), &spec) {
            return Err((crate::codes::WALL_COLLAPSES, format!("“{uid}” kimlikli duvar: {why}")));
        }
        updates.push((Slot(w.base.id), Entity::Wall(next)));
    }
    Ok((updates, locked))
}

/// Saves the wall styles (docs/adr/0339 §2): the table, then the walls that follow them in one step “Duvar stili”;
/// those on locked layers left and counted. The command has refused a table its walls cannot take.
pub fn save_wall_styles(doc: &mut Document, next: Vec<kentos_commands::WallStyleDef>) -> Saved {
    let (updates, locked) = wall_followers(doc, &next).unwrap_or_default();
    let mut settings = doc.settings().clone();
    settings.wall_styles = next;
    doc.set_settings(settings);
    let changed = updates.len();
    if changed > 0 {
        doc.update_many(updates, "Duvar stili");
    }
    Saved { changed, locked, said: Vec::new() }
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
        let was = t;
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
        // A style renamed, or changed in what the text no longer held, leaves it as it is.
        if t == *was {
            continue;
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
        let was = d;
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
        if d == *was {
            continue;
        }
        updates.push((Slot(d.base.id), Entity::Dimension(d)));
    }
    follow(doc, EditOperation::DimensionStyle, updates)
}
