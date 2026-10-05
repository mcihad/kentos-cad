//! Köşe tablosu's writes (docs/adr/0172 §4, §5; the web's
//! `ui/bottom/vertexEdit.ts`), apart from the view: a vertex's cell given a
//! value, Satır ekle's draft written after a vertex, and vertices removed.
//! The paths come from the core (`ops::vertex_table`); every write goes
//! through `cad.entities.edit`. A cell is one undo step, “Köşe düzenle”; a
//! draft “Köşe ekle”, a removal “Köşe sil”.
//! `fixtures/vertex-table/v1/edits.json` holds both platforms to the same
//! drawing, messages and steps.

use kentos_contracts::{
    AreaPart, EditOperation, EntitiesEdit, Entity, EntityEdit, EntityGeometry, RingGeometry,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::ops::elevation::Elevated;
use kentos_geometry_core::ops::vertex_table::{
    self, Edited, Kind, Refusal, Row, inserted, moved, removed, with_radius, with_z,
};
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_interaction::neighbours::{lines as neighbour_lines, neighbours_in};
use kentos_interaction::spatial::Spatial;
use kentos_interaction::{Format, Level};
use kentos_native_application::elevation::paths as elevated_paths;
use kentos_native_application::geometry::{entity_of, shape};
use kentos_native_application::{ExecutionContext, edit};

use crate::points::edit::{in_step, refusal};

/// The cells that are edited, in the order Tab takes them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Column {
    East,
    North,
    Z,
    Radius,
}

impl Column {
    pub const ALL: [Column; 4] = [Column::East, Column::North, Column::Z, Column::Radius];

    /// By the cases' key: `east`, `north`, `z`, `radius`.
    #[cfg(test)]
    pub fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "east" => Self::East,
            "north" => Self::North,
            "z" => Self::Z,
            "radius" => Self::Radius,
            _ => return None,
        })
    }

    /// What its value is called in a message: east and north as the
    /// project's type names them (docs/adr/0165 §4).
    fn word(self, format: &Format) -> &'static str {
        match self {
            Self::East => format.east_label(),
            Self::North => format.north_label(),
            Self::Z => "Z",
            Self::Radius => "Yarıçap",
        }
    }
}

/// The table's own messages start so; the command's refusals are said as they are.
pub const PREFIX: &str = "Köşe tablosu: ";
pub const STEP: &str = "Köşe düzenle";
pub const ADD_STEP: &str = "Köşe ekle";
pub const REMOVE_STEP: &str = "Köşe sil";

/// A vertex: its path (the elevations' order) and its place in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct At {
    pub path: usize,
    pub index: usize,
}

/// What came of a write: what to warn of, the undo step written (none:
/// nothing), whether the cell stays open, and what to tell (the neighbours
/// that changed with it).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outcome {
    pub said: Vec<String>,
    pub step: Option<&'static str>,
    pub stay: bool,
    pub told: Vec<String>,
}

/// Topological editing, while the mode is on (docs/adr/0160, 0172 §6): the
/// geometry store in step with the drawing, and Noktalar da.
#[derive(Clone, Copy)]
pub struct Topology<'a> {
    pub spatial: &'a Spatial,
    pub points: bool,
}

/// Satır ekle's row: its cells as typed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Draft {
    pub east: String,
    pub north: String,
    pub z: String,
}

/// The object's kind for the core, when the table edits it: one line,
/// polyline or area.
pub fn kind_of(e: &Entity) -> Option<Kind> {
    match e {
        Entity::Line(_) => Some(Kind::Line),
        Entity::Polyline(_) => Some(Kind::Polyline),
        Entity::Polygon(_) => Some(Kind::Polygon),
        _ => None,
    }
}

/// The table's words for a refusal (`adding`: a new vertex's).
pub fn refusal_text(r: &Refusal, format: &Format, adding: bool) -> String {
    let words = match r {
        Refusal::OntoNeighbour if adding => "Yeni köşe komşu köşesinin yerinde olamaz.".to_owned(),
        Refusal::OntoNeighbour => {
            "Köşe komşu köşesinin yerine taşınamaz; köşeyi kaldırmak için satırı silin.".to_owned()
        }
        Refusal::NoEdge => "Bu köşeden çıkan kenar yok.".to_owned(),
        Refusal::LineArc => "Çizginin kenarı yay olamaz; önce köşe ekleyin.".to_owned(),
        Refusal::NoChord => "Kenarın iki ucu aynı yerde; yay olamaz.".to_owned(),
        Refusal::RadiusBelow { least } => format!(
            "Yarıçap kirişin yarısından küçük olamaz: en az {}.",
            format.length(*least)
        ),
        Refusal::LineEnds => "Çizginin iki ucu silinemez.".to_owned(),
        Refusal::PathMin => "Çoklu çizgide en az iki köşe kalmalı.".to_owned(),
        Refusal::RingMin => "Her halkada en az üç köşe kalmalı.".to_owned(),
        Refusal::Missing => "Köşe bulunamadı.".to_owned(),
    };
    format!("{PREFIX}{words}")
}

/// The object's rows.
pub fn rows_of(e: &Entity) -> Vec<Row> {
    vertex_table::rows(&elevated_paths(e))
}

/// A number as JavaScript writes it (`String(n)`): the shortest digits that read back.
fn js_number(v: f64) -> String {
    if v == 0.0 {
        return "0".to_owned();
    }
    format!("{v}")
}

/// The text a cell's editor opens with: the value whole (the web's
/// `cellText`), in a local project's unit (`per_metre`, docs/adr/0165 §2)
/// without the multiplication's last-digit noise.
pub fn cell_text(row: &Row, col: Column, per_metre: f64) -> String {
    let shown = |v: f64| {
        if per_metre == 1.0 {
            js_number(v)
        } else {
            js_number(
                format!("{:.14e}", v * per_metre)
                    .parse()
                    .unwrap_or(v * per_metre),
            )
        }
    };
    match col {
        Column::East => shown(row.p.x),
        Column::North => shown(row.p.y),
        Column::Z => row.z.map(shown).unwrap_or_default(),
        Column::Radius => row.radius.map(shown).unwrap_or_default(),
    }
}

/// Whether the row's cell is edited: a line's edge takes no arc, and no
/// edge leaves an open path's last vertex.
pub fn cell_editable(kind: Kind, row: &Row, col: Column) -> bool {
    col != Column::Radius || (kind != Kind::Line && row.chord.is_some())
}

fn wire(p: kentos_geometry_core::Vec2) -> kentos_contracts::Vec2 {
    kentos_contracts::Vec2 { x: p.x, y: p.y }
}

fn ring(p: &Elevated) -> RingGeometry {
    RingGeometry {
        pts: p.pts.iter().map(|q| wire(*q)).collect(),
        bulges: p.bulges.clone(),
        zs: Some(p.zs.clone()),
    }
}

/// The object's geometry with `paths` put back (in the elevations' order),
/// as `kind`: a line given a vertex is a polyline; an area keeps its holes
/// and parts, each ring its own bulges and elevations (the web's `geometryOf`).
pub fn geometry_of(e: &Entity, kind: Kind, paths: &[Elevated]) -> Option<EntityGeometry> {
    let first = paths.first()?;
    Some(match kind {
        Kind::Line => EntityGeometry::Line {
            a: wire(*first.pts.first()?),
            b: wire(*first.pts.get(1)?),
            zs: Some(first.zs.clone()),
        },
        Kind::Polyline => {
            let r = ring(first);
            // A multi-part polyline's other parts, in `elevation::paths`' order (docs/adr/0174).
            let parts: Vec<AreaPart> = match e {
                Entity::Polyline(l) => l
                    .parts
                    .iter()
                    .flatten()
                    .zip(paths.iter().skip(1))
                    .map(|(_, p)| {
                        let own = ring(p);
                        AreaPart {
                            pts: own.pts,
                            bulges: own.bulges,
                            zs: own.zs,
                            holes: None,
                        }
                    })
                    .collect(),
                _ => Vec::new(),
            };
            EntityGeometry::Polyline {
                pts: r.pts,
                bulges: r.bulges,
                zs: r.zs,
                parts: (!parts.is_empty()).then_some(parts),
            }
        }
        Kind::Polygon => {
            let Entity::Polygon(area) = e else {
                return None;
            };
            let mut rest = paths.iter().skip(1);
            let holes: Vec<RingGeometry> = area
                .holes
                .iter()
                .flatten()
                .filter_map(|_| rest.next().map(ring))
                .collect();
            let parts: Vec<AreaPart> = area
                .parts
                .iter()
                .flatten()
                .filter_map(|part| {
                    let own = ring(rest.next()?);
                    let inner: Vec<RingGeometry> = part
                        .holes
                        .iter()
                        .flatten()
                        .filter_map(|_| rest.next().map(ring))
                        .collect();
                    Some(AreaPart {
                        pts: own.pts,
                        bulges: own.bulges,
                        zs: own.zs,
                        holes: (!inner.is_empty()).then_some(inner),
                    })
                })
                .collect();
            let outer = ring(first);
            EntityGeometry::Polygon {
                pts: outer.pts,
                bulges: outer.bulges,
                holes: (!holes.is_empty()).then_some(holes),
                zs: outer.zs,
                parts: (!parts.is_empty()).then_some(parts),
            }
        }
    })
}

/// Typed coordinates, elevations and radii in metres: typed in a local
/// project's unit (docs/adr/0165 §2).
fn metres_of(doc: &Document) -> impl Fn(f64) -> f64 + use<> {
    let per_metre = doc.settings().unit().per_metre();
    move |typed| typed / per_metre
}

/// The slack of a radius: half a unit of what the table shows of a length
/// (the project's decimals, in its unit), in metres (docs/adr/0172 §4).
pub fn radius_slack(doc: &Document) -> f64 {
    let settings = doc.settings();
    // 10⁻ᵈ read from its decimal text: the nearest double, as the web's.
    let unit: f64 = format!("1e-{}", settings.length_decimals)
        .parse()
        .unwrap_or(0.001);
    0.5 * unit / settings.unit().per_metre()
}

/// The core's answer written as one undo step named `step`, the object
/// updated in place (a line that became a polyline replaced, keeping its
/// slot, id and data); a refusal said in the table's words keeps the cell
/// open, the command's refusal closes it.
fn written(
    doc: &mut Document,
    slot: Slot,
    answer: Result<Edited, Refusal>,
    step: &'static str,
    operation: EditOperation,
    adding: bool,
    topology: Option<Topology<'_>>,
) -> Outcome {
    let format = Format::of(doc.settings());
    let edited = match answer {
        Ok(edited) => edited,
        Err(r) => {
            return Outcome {
                said: vec![refusal_text(&r, &format, adding)],
                stay: true,
                ..Outcome::default()
            };
        }
    };
    let (Some(e), Some(uid)) = (doc.get(slot).cloned(), doc.uid(slot)) else {
        return Outcome::default();
    };
    let Some(geometry) = geometry_of(&e, edited.kind, &edited.paths) else {
        return Outcome::default();
    };
    // With the mode on, the neighbours' shared corners and edges go with it (docs/adr/0172 §6).
    let follow = topology.and_then(|t| {
        let after = shape(&entity_of(&geometry, e.base().clone()));
        neighbours_in(doc, t.spatial, t.points, &[(slot, shape(&e), after)])
    });
    let uid = uid.to_string();
    let change = if Some(edited.kind) == kind_of(&e) {
        EntityEdit::Update { uid, geometry }
    } else {
        EntityEdit::Replace {
            uid,
            geometry,
            keep_data: Some(true),
        }
    };
    let mut changes = vec![change];
    if let Some(n) = &follow {
        changes.extend(n.changes.iter().cloned());
    }
    let refused = in_step(doc, step, |doc| {
        let input = EntitiesEdit {
            operation,
            changes,
            expected_revision: None,
        };
        refusal(edit::execute(&mut ExecutionContext::new(doc), input)).map(|e| e.message)
    });
    if let Some(refused) = refused {
        return Outcome {
            said: vec![refused],
            ..Outcome::default()
        };
    }
    let mut out = Outcome {
        step: Some(step),
        ..Outcome::default()
    };
    for (level, text) in follow.as_ref().map(neighbour_lines).unwrap_or_default() {
        match level {
            Level::Info | Level::Success => out.told.push(text),
            _ => out.said.push(text),
        }
    }
    out
}

/// A vertex's cell given `text` (docs/adr/0172 §4; the web's
/// `writeVertexCell`): Y and X move it (its arcs keep their bulges), Z is
/// its elevation (empty removes it), Yarıçap its edge's radius (empty or 0:
/// straight). A value that is no number, or that the core refuses, keeps the
/// cell open; an unchanged one writes nothing.
pub fn write_cell(
    doc: &mut Document,
    slot: Slot,
    at: At,
    col: Column,
    text: &str,
    topology: Option<Topology<'_>>,
) -> Outcome {
    let Some(e) = doc.get(slot).cloned() else {
        return Outcome::default();
    };
    let Some(kind) = kind_of(&e) else {
        return Outcome::default();
    };
    let paths = elevated_paths(&e);
    let Some(row) = vertex_table::rows(&paths)
        .into_iter()
        .find(|r| r.path == at.path && r.index == at.index)
    else {
        return Outcome {
            said: vec![format!("{PREFIX}Köşe bulunamadı.")],
            ..Outcome::default()
        };
    };
    let format = Format::of(doc.settings());
    let empty = js_trim(text).is_empty() && matches!(col, Column::Z | Column::Radius);
    let v = if empty {
        None
    } else {
        match parse_number(text) {
            Some(v) => Some(metres_of(doc)(v)),
            None => {
                return Outcome {
                    said: vec![format!("{PREFIX}{} bir sayı olmalı.", col.word(&format))],
                    stay: true,
                    ..Outcome::default()
                };
            }
        }
    };
    let answer = match col {
        Column::East | Column::North => {
            let typed = v.unwrap_or_default();
            let to = if col == Column::East {
                kentos_geometry_core::Vec2::new(typed, row.p.y)
            } else {
                kentos_geometry_core::Vec2::new(row.p.x, typed)
            };
            if to == row.p {
                return Outcome::default();
            }
            moved(&paths, at.path, at.index, to)
        }
        Column::Z => {
            if v == row.z {
                return Outcome::default();
            }
            with_z(&paths, at.path, at.index, v)
        }
        Column::Radius => {
            // A radius within 1e-9 of the edge's is the edge's: the cell opens
            // with it, its last bit is the platform's.
            let same =
                matches!((v, row.radius), (Some(v), Some(r)) if (v - r).abs() <= 1e-9 * r.abs());
            if (matches!(v, None | Some(0.0)) && row.radius.is_none()) || same {
                return Outcome::default();
            }
            let radius = v.filter(|&r| r != 0.0);
            with_radius(kind, &paths, at.path, at.index, radius, radius_slack(doc))
        }
    };
    written(
        doc,
        slot,
        answer.map(|paths| Edited { kind, paths }),
        STEP,
        EditOperation::Properties,
        false,
        topology,
    )
}

/// Satır ekle's row written after vertex `after` (docs/adr/0172 §5; the
/// web's `writeVertexDraft`): Y and X are needed, Z is optional; through
/// `cad.entities.edit` as Köşe ekle writes (“Köşe ekle”). With the
/// outcome, where the next draft goes (after the new vertex).
pub fn write_draft(
    doc: &mut Document,
    slot: Slot,
    after: At,
    d: &Draft,
    topology: Option<Topology<'_>>,
) -> (Outcome, Option<At>) {
    let format = Format::of(doc.settings());
    let fail = |said: String| {
        (
            Outcome {
                said: vec![said],
                stay: true,
                ..Outcome::default()
            },
            None,
        )
    };
    let (east_word, north_word) = (format.east_label(), format.north_label());
    let (east, north) = (js_trim(&d.east), js_trim(&d.north));
    if east.is_empty() && north.is_empty() {
        return fail(format!("{PREFIX}{east_word} ve {north_word} yazılmalı."));
    }
    if east.is_empty() {
        return fail(format!("{PREFIX}{east_word} yazılmalı."));
    }
    if north.is_empty() {
        return fail(format!("{PREFIX}{north_word} yazılmalı."));
    }
    let metres = metres_of(doc);
    let Some(x) = parse_number(east).map(&metres) else {
        return fail(format!("{PREFIX}{east_word} bir sayı olmalı."));
    };
    let Some(y) = parse_number(north).map(&metres) else {
        return fail(format!("{PREFIX}{north_word} bir sayı olmalı."));
    };
    let z = if js_trim(&d.z).is_empty() {
        None
    } else {
        match parse_number(&d.z) {
            Some(z) => Some(metres(z)),
            None => return fail(format!("{PREFIX}Z bir sayı olmalı.")),
        }
    };
    let Some(e) = doc.get(slot).cloned() else {
        return (Outcome::default(), None);
    };
    let Some(kind) = kind_of(&e) else {
        return (Outcome::default(), None);
    };
    let answer = inserted(
        kind,
        &elevated_paths(&e),
        after.path,
        after.index,
        kentos_geometry_core::Vec2::new(x, y),
        z,
    );
    let out = written(
        doc,
        slot,
        answer,
        ADD_STEP,
        EditOperation::VertexAdd,
        true,
        topology,
    );
    let next = out.step.map(|_| At {
        path: after.path,
        index: after.index + 1,
    });
    (out, next)
}

/// The vertices `at` removed in one write (docs/adr/0172 §5; the web's
/// `removeVertices`), as Köşe sil writes (“Köşe sil”).
pub fn remove(
    doc: &mut Document,
    slot: Slot,
    at: &[At],
    topology: Option<Topology<'_>>,
) -> Outcome {
    let Some(e) = doc.get(slot).cloned() else {
        return Outcome::default();
    };
    let Some(kind) = kind_of(&e) else {
        return Outcome::default();
    };
    let at: Vec<[usize; 2]> = at.iter().map(|a| [a.path, a.index]).collect();
    let answer = removed(kind, &elevated_paths(&e), &at).map(|paths| Edited { kind, paths });
    // Nothing to keep open: a removal has no cell.
    Outcome {
        stay: false,
        ..written(
            doc,
            slot,
            answer,
            REMOVE_STEP,
            EditOperation::VertexRemove,
            false,
            topology,
        )
    }
}
