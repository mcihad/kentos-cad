//! Nokta editörü's writes (docs/adr/0153 §3, §4; the web's
//! `ui/bottom/pointEdit.ts`), apart from the view: a cell given a value, with
//! the line work that follows the point, and Satır ekle's draft. Every write
//! goes through the product commands; a cell is one undo step, “Nokta
//! düzenle”. `fixtures/point-editor/v1/edits.json` holds both platforms to the
//! same drawing, messages and steps.

use std::collections::BTreeMap;

use kentos_contracts::{
    AreaPart, CommandResult, EditOperation, EntitiesEdit, EntitiesSetProperties, Entity,
    EntityEdit, EntityGeometry, PointCreate, PointEntity, PropertiesOperation, RingGeometry,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::ops::elevation::Elevated;
use kentos_geometry_core::ops::point_editor::follow_point;
use kentos_geometry_core::text::edit::increment;
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::elevation::paths as elevated_paths;
use kentos_native_application::{ExecutionContext, edit, point, set};

/// The cells that are edited (the core's column keys).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditColumn {
    Name,
    East,
    North,
    Z,
    Code,
}

impl EditColumn {
    /// By the core's key: `name`, `east`, `north`, `z`, `code`.
    #[cfg(test)]
    pub fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "name" => Self::Name,
            "east" => Self::East,
            "north" => Self::North,
            "z" => Self::Z,
            "code" => Self::Code,
            _ => return None,
        })
    }

    /// What its value is called in a message: Y, X, Z.
    fn word(self) -> &'static str {
        match self {
            Self::East => "Y",
            Self::North => "X",
            _ => "Z",
        }
    }
}

/// The editor's own messages start so; the commands' refusals are said as they are.
pub const PREFIX: &str = "Nokta editörü: ";
pub const STEP: &str = "Nokta düzenle";
/// Satır ekle's step: the point command's.
pub const DRAFT_STEP: &str = "Ekle";

/// What came of a write: what to say (warnings), the undo step written
/// (none: nothing), and whether the cell stays open.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outcome {
    pub said: Vec<String>,
    pub step: Option<&'static str>,
    pub stay: bool,
}

/// A number as JavaScript writes it (`String(n)`): the shortest digits that read back.
fn js_number(v: f64) -> String {
    if v == 0.0 {
        return "0".to_owned();
    }
    format!("{v}")
}

/// The text a cell's editor opens with: the name and the code as written, a
/// number whole (the web's `cellText`).
pub fn cell_text(p: &PointEntity, col: EditColumn) -> String {
    match col {
        EditColumn::Name => p.base.label.clone().unwrap_or_default(),
        EditColumn::Code => p.base.attrs.get("Kod").cloned().unwrap_or_default(),
        EditColumn::East => js_number(p.p.x),
        EditColumn::North => js_number(p.p.y),
        EditColumn::Z => p.z.map(js_number).unwrap_or_default(),
    }
}

/// Whether another point has the name (trimmed).
fn named(doc: &Document, name: &str, except: Option<u32>) -> bool {
    doc.entities().any(|e| match e {
        Entity::Point(p) => {
            Some(p.base.id) != except && p.base.label.as_deref().map(js_trim) == Some(name)
        }
        _ => false,
    })
}

fn same_name(name: &str) -> String {
    format!("{PREFIX}“{name}” adında başka bir nokta da var.")
}

fn wire(p: kentos_geometry_core::Vec2) -> kentos_contracts::Vec2 {
    kentos_contracts::Vec2 { x: p.x, y: p.y }
}

fn wires(pts: &[kentos_geometry_core::Vec2]) -> Vec<kentos_contracts::Vec2> {
    pts.iter().map(|p| wire(*p)).collect()
}

/// Line work's geometry with its paths (in `elevation::paths`' order: the
/// outer ring, its holes, then each part's ring and holes) put back.
pub fn with_paths(e: &Entity, ps: &[Elevated]) -> Option<EntityGeometry> {
    match e {
        Entity::Line(_) => Some(EntityGeometry::Line {
            a: wire(ps[0].pts[0]),
            b: wire(ps[0].pts[1]),
            zs: Some(ps[0].zs.clone()),
        }),
        Entity::Polyline(l) => Some(EntityGeometry::Polyline {
            pts: wires(&ps[0].pts),
            bulges: l.bulges.clone(),
            zs: Some(ps[0].zs.clone()),
        }),
        Entity::Polygon(a) => {
            let mut k = 1;
            let mut ring = |bulges: &Option<Vec<f64>>| {
                let p = &ps[k];
                k += 1;
                RingGeometry {
                    pts: wires(&p.pts),
                    bulges: bulges.clone(),
                    zs: Some(p.zs.clone()),
                }
            };
            let holes: Vec<RingGeometry> =
                a.holes.iter().flatten().map(|h| ring(&h.bulges)).collect();
            let parts: Vec<AreaPart> = a
                .parts
                .iter()
                .flatten()
                .map(|part| {
                    let own = ring(&part.bulges);
                    AreaPart {
                        pts: own.pts,
                        bulges: own.bulges,
                        zs: own.zs,
                        holes: part
                            .holes
                            .as_ref()
                            .map(|hs| hs.iter().map(|h| ring(&h.bulges)).collect()),
                    }
                })
                .collect();
            Some(EntityGeometry::Polygon {
                pts: wires(&ps[0].pts),
                bulges: a.bulges.clone(),
                holes: (!holes.is_empty()).then_some(holes),
                zs: Some(ps[0].zs.clone()),
                parts: (!parts.is_empty()).then_some(parts),
            })
        }
        _ => None,
    }
}

/// A command's refusal: its error, when it wrote nothing.
fn refusal<T>(result: CommandResult<T>) -> Option<kentos_contracts::CommandError> {
    match result {
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => Some(error),
        _ => None,
    }
}

/// Runs `write` as one undo step “Nokta düzenle”; a refusal is rolled back and returned.
fn step(doc: &mut Document, write: impl FnOnce(&mut Document) -> Option<String>) -> Option<String> {
    doc.transact(STEP, |doc| match write(doc) {
        Some(refused) => Err(refused),
        None => Ok(()),
    })
    .err()
}

/// A point's cell given `text` (docs/adr/0153 §3; the web's `writeCell`).
pub fn write_cell(
    doc: &mut Document,
    slot: Slot,
    col: EditColumn,
    text: &str,
    follow: bool,
) -> Outcome {
    let (Some(Entity::Point(p)), Some(uid)) = (doc.get(slot).cloned(), doc.uid(slot)) else {
        return Outcome::default();
    };
    let uid = uid.to_string();
    if matches!(col, EditColumn::Name | EditColumn::Code) {
        let v = Some(js_trim(text).to_owned()).filter(|v| !v.is_empty());
        let old = match col {
            EditColumn::Name => p
                .base
                .label
                .as_deref()
                .map(js_trim)
                .filter(|v| !v.is_empty())
                .map(str::to_owned),
            _ => p.base.attrs.get("Kod").cloned(),
        };
        if v == old {
            return Outcome::default();
        }
        let refused = step(doc, |doc| {
            let input = EntitiesSetProperties {
                uids: vec![uid.clone()],
                layer_id: None,
                color: None,
                line_weight: None,
                symbol: None,
                attrs: (col == EditColumn::Code)
                    .then(|| BTreeMap::from([("Kod".to_owned(), v.clone())])),
                label: (col == EditColumn::Name).then(|| v.clone()),
                operation: if col == EditColumn::Name {
                    PropertiesOperation::Label
                } else {
                    PropertiesOperation::Attributes
                },
                expected_revision: None,
            };
            refusal(set::execute(&mut ExecutionContext::new(doc), input)).map(|e| e.message)
        });
        if let Some(refused) = refused {
            return Outcome {
                said: vec![refused],
                ..Outcome::default()
            };
        }
        let said = match (&v, col) {
            (Some(name), EditColumn::Name) if named(doc, name, Some(p.base.id)) => {
                vec![same_name(name)]
            }
            _ => Vec::new(),
        };
        return Outcome {
            said,
            step: Some(STEP),
            stay: false,
        };
    }
    let blank = js_trim(text).is_empty();
    let v = if col == EditColumn::Z && blank {
        None
    } else {
        match parse_number(text) {
            Some(v) => Some(v),
            None => {
                return Outcome {
                    said: vec![format!("{PREFIX}{} bir sayı olmalı.", col.word())],
                    step: None,
                    stay: true,
                };
            }
        }
    };
    let from = kentos_geometry_core::Vec2::new(p.p.x, p.p.y);
    let to = match col {
        EditColumn::East => kentos_geometry_core::Vec2::new(v.unwrap_or(from.x), from.y),
        EditColumn::North => kentos_geometry_core::Vec2::new(from.x, v.unwrap_or(from.y)),
        _ => from,
    };
    let unchanged = match col {
        EditColumn::Z => v == p.z,
        _ => to == from,
    };
    if unchanged {
        return Outcome::default();
    }
    let z = if col == EditColumn::Z { v } else { p.z };
    let mut changes = vec![EntityEdit::Update {
        uid,
        geometry: EntityGeometry::Point { p: wire(to), z },
    }];
    if follow {
        for other in doc.entities() {
            if !matches!(
                other,
                Entity::Line(_) | Entity::Polyline(_) | Entity::Polygon(_)
            ) {
                continue;
            }
            let set_z = col == EditColumn::Z;
            let Some(moved) = follow_point(
                &elevated_paths(other),
                from,
                to,
                set_z,
                if set_z { v } else { None },
            ) else {
                continue;
            };
            if let (Some(geometry), Some(uid)) =
                (with_paths(other, &moved), doc.uid(Slot(other.base().id)))
            {
                changes.push(EntityEdit::Update {
                    uid: uid.to_string(),
                    geometry,
                });
            }
        }
    }
    let operation = if col == EditColumn::Z {
        EditOperation::Elevation
    } else {
        EditOperation::Properties
    };
    let refused = step(doc, |doc| {
        let input = EntitiesEdit {
            operation,
            changes,
            expected_revision: None,
        };
        refusal(edit::execute(&mut ExecutionContext::new(doc), input)).map(|e| {
            // The point is the first change: a refusal of another is a line work's lock.
            let other = !e.path.as_deref().unwrap_or("").starts_with("changes[0]");
            if e.code == "layer_locked" && other {
                format!(
                    "{PREFIX}Bağlı çizgilerden biri kilitli katmanda; katmanın kilidini açın ya da Bağlı çizgiler izler'i kapatın."
                )
            } else {
                e.message
            }
        })
    });
    match refused {
        Some(refused) => Outcome {
            said: vec![refused],
            ..Outcome::default()
        },
        None => Outcome {
            said: Vec::new(),
            step: Some(STEP),
            stay: false,
        },
    }
}

/// Satır ekle's row: its cells as typed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Draft {
    pub name: String,
    pub east: String,
    pub north: String,
    pub z: String,
    pub code: String,
}

/// A draft written: what came of it, and the next draft's name (Artır's;
/// empty when the name does not end with a number); none when not written.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DraftOutcome {
    pub outcome: Outcome,
    pub next: Option<String>,
}

/// Satır ekle's row written (docs/adr/0153 §4; the web's `writeDraft`).
pub fn write_draft(
    doc: &mut Document,
    d: &Draft,
    layer_id: &str,
    color: Option<&str>,
) -> DraftOutcome {
    let fail = |said: String| DraftOutcome {
        outcome: Outcome {
            said: vec![said],
            step: None,
            stay: true,
        },
        next: None,
    };
    let (east, north) = (js_trim(&d.east), js_trim(&d.north));
    if east.is_empty() && north.is_empty() {
        return fail(format!("{PREFIX}Y ve X yazılmalı."));
    }
    if east.is_empty() {
        return fail(format!("{PREFIX}Y yazılmalı."));
    }
    if north.is_empty() {
        return fail(format!("{PREFIX}X yazılmalı."));
    }
    let Some(x) = parse_number(east) else {
        return fail(format!("{PREFIX}Y bir sayı olmalı."));
    };
    let Some(y) = parse_number(north) else {
        return fail(format!("{PREFIX}X bir sayı olmalı."));
    };
    let z = if js_trim(&d.z).is_empty() {
        None
    } else {
        match parse_number(&d.z) {
            Some(z) => Some(z),
            None => return fail(format!("{PREFIX}Z bir sayı olmalı.")),
        }
    };
    let name = js_trim(&d.name).to_owned();
    let code = js_trim(&d.code).to_owned();
    let said = if !name.is_empty() && named(doc, &name, None) {
        vec![same_name(&name)]
    } else {
        Vec::new()
    };
    let input = PointCreate {
        layer_id: layer_id.to_owned(),
        p: kentos_contracts::Vec2 { x, y },
        z,
        label: (!name.is_empty()).then(|| name.clone()),
        color: color.map(str::to_owned),
        attrs: (!code.is_empty()).then(|| BTreeMap::from([("Kod".to_owned(), code.clone())])),
        expected_revision: None,
    };
    if let Some(e) = refusal(point::execute(&mut ExecutionContext::new(doc), input)) {
        return fail(e.message);
    }
    DraftOutcome {
        outcome: Outcome {
            said,
            step: Some(DRAFT_STEP),
            stay: false,
        },
        next: Some(if name.is_empty() {
            String::new()
        } else {
            increment(&name).unwrap_or_default()
        }),
    }
}
