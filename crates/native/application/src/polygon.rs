//! `cad.polygon.create` v1 (docs/adr/0022): one closed area on a named
//! layer, as one undo step. The desktop's handler over the native document;
//! the web's is `apps/web/src/product/polygonCreate.ts`. Both pass the shared
//! cases in `fixtures/commands/v1/cad.polygon.create.json`, which fix the
//! checks, their order, the codes, paths and messages.
//!
//! The checks, in order (the first that fails answers):
//! 1. the outer ring, then each hole: at least 3 corners, every coordinate
//!    finite, one bulge per edge when bulges are given, every bulge finite;
//!    then the line weight from 0 to 100 mm when given (docs/adr/0139);
//! 2. the expected revision: decimal text, then the document's own
//!    (`conflict` when not);
//! 3. the layer: known, a layer and not a group, not locked (by itself or a
//!    group above it). A hidden one is written with a warning.
//!
//! Why this order: an input broken by itself is refused before the document
//! is looked at; and once the document has changed since the input was
//! prepared, the layer's state now says nothing about what the caller saw,
//! so the conflict comes first. Steps 2 and 3 are every create command's
//! (`checks.rs`).
//!
//! Nothing here is geometry: the checks are counts, finite numbers and the
//! layer tree's state. Geometric validity (a ring crossing itself, zero area,
//! a hole outside the ring) is not checked, as the drawing tools and the file
//! reader do not check it either.

use kentos_commands::{
    CommandResult, CommandWarning, Entity, EntityBase, PathEntity, PolygonCreate, PolygonCreated,
    PolygonPlan, Vec2,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;

/// Corners a ring needs.
const MIN_CORNERS: usize = 3;

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &PolygonCreate) -> CommandResult<()> {
    match ruled(cx.doc, cx.store, input) {
        Ok((warnings, _, _)) => CommandResult::Completed {
            output: (),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now, and the revision to expect for exactly
/// that; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &PolygonCreate) -> CommandResult<PolygonPlan> {
    match ruled(cx.doc, cx.store, input) {
        Ok((warnings, entity, _)) => CommandResult::Completed {
            output: PolygonPlan {
                entity,
                revision: cx.doc.revision().to_string(),
            },
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the polygon as one undo step (“Ekle”,
/// the document's own `add`): into the open transaction or group, if one is.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: PolygonCreate,
) -> CommandResult<PolygonCreated> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_POLYGON_CREATE,
        kentos_commands::CAD_POLYGON_CREATE_VERSION,
        input,
        write_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step of the
/// document, all or nothing, and journals it (docs/adr/0253 §1, §3). The
/// shape commands write their ring through it (`shapes.rs`).
pub(crate) fn write(
    cx: &mut ExecutionContext<'_>,
    input: PolygonCreate,
) -> CommandResult<PolygonCreated> {
    write_execute(cx, input)
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: PolygonCreate,
) -> CommandResult<PolygonCreated> {
    match ruled(cx.doc, cx.store, &input) {
        Ok(ruled) => write_ruled(cx, ruled),
        Err(stop) => stop.into(),
    }
}

/// The checks, then the rules (Çakışma, Topoloji; docs/adr/0622 §9): the warnings, the polygon as the document will
/// store it (id 0) and the neighbours' corners.
fn ruled(
    doc: &Document,
    store: Option<&kentos_geometry_core::store::Store>,
    input: &PolygonCreate,
) -> Result<Ruled, Stop> {
    let area =
        crate::area_rules::area_of(&input.pts, input.bulges.as_deref(), input.holes.as_deref());
    ruled_areas(doc, store, input, vec![area])
}

/// The warnings, the polygon to store (id 0) and the neighbours' corners.
pub(crate) type Ruled = (
    Vec<CommandWarning>,
    Entity,
    Vec<kentos_commands::EntityEdit>,
);

/// [`ruled`] for an area of several parts (Bitişik alan's region, docs/adr/0622 §9): `input` gives its first part's
/// rings, which the checks read, and its look; `areas` every part, written as one object.
pub(crate) fn ruled_areas(
    doc: &Document,
    store: Option<&kentos_geometry_core::store::Store>,
    input: &PolygonCreate,
    areas: Vec<kentos_geometry_core::geom::arrangement::Area>,
) -> Result<Ruled, Stop> {
    let mut warnings = check(doc, input)?;
    crate::area_rules::check(doc, input.rules.as_ref())?;
    let rules = crate::area_rules::apply(doc, store, areas.clone(), input.rules.as_ref())?;
    let mut entity = polygon(settled(doc, input.clone()), 0);
    let mut edits = Vec::new();
    let areas = match rules {
        Some(rules) => {
            warnings.extend(rules.warnings);
            edits = rules.edits;
            rules.areas
        }
        None => areas,
    };
    if let (Entity::Polygon(p), Some((pts, bulges, holes, parts))) =
        (&mut entity, crate::area_rules::fields(&areas))
    {
        p.pts = pts;
        p.bulges = bulges;
        p.holes = holes;
        p.parts = parts;
    }
    Ok((warnings, entity, edits))
}

/// Writes what [`ruled_areas`] made into the open step: the polygon, then the neighbours' corners.
pub(crate) fn write_ruled(
    cx: &mut ExecutionContext<'_>,
    ruled: Ruled,
) -> CommandResult<PolygonCreated> {
    let (warnings, entity, edits) = ruled;
    let (id, uid, _) = match checks::add(cx.doc, entity) {
        Ok(added) => added,
        Err(stop) => return stop.into(),
    };
    // Topoloji: the neighbours take the new area's corners in the same step (docs/adr/0162 §4).
    if let Err(stop) = crate::area_rules::write_neighbours(cx, edits) {
        return stop.into();
    }
    CommandResult::Completed {
        output: PolygonCreated {
            uid,
            id,
            revision: cx.doc.revision().to_string(),
        },
        warnings,
    }
}

/// The checks in the contract's order; the warnings when the polygon may be written.
fn check(doc: &Document, input: &PolygonCreate) -> Result<Vec<CommandWarning>, Stop> {
    check_ring(Ring::Outer, &input.pts, input.bulges.as_deref())?;
    for (h, hole) in input.holes.iter().flatten().enumerate() {
        check_ring(Ring::Hole(h), &hole.pts, hole.bulges.as_deref())?;
    }
    checks::line_weight(input.line_weight, "lineWeight")?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let warnings = checks::layer(doc, &input.layer_id)?;
    checks::object_attrs(doc, &input.layer_id, input.attrs.as_ref(), "attrs")?;
    Ok(warnings)
}

/// Which ring of the input: the outer one or a hole (0-based).
#[derive(Clone, Copy)]
enum Ring {
    Outer,
    Hole(usize),
}

impl Ring {
    /// The input path of one of its fields: `pts`, `holes[0].bulges`.
    fn path(self, field: &str) -> String {
        match self {
            Ring::Outer => field.to_owned(),
            Ring::Hole(h) => format!("holes[{h}].{field}"),
        }
    }

    /// Its corner `i` (0-based) as a message names it: “2. köşenin”, “1. deliğin 2. köşesinin”.
    fn corner(self, i: usize) -> String {
        match self {
            Ring::Outer => format!("{}. köşenin", i + 1),
            Ring::Hole(h) => format!("{}. deliğin {}. köşesinin", h + 1, i + 1),
        }
    }

    /// Its edge `i` (0-based; the last closes the ring): “2. kenarın”, “1. deliğin 2. kenarının”.
    fn edge(self, i: usize) -> String {
        match self {
            Ring::Outer => format!("{}. kenarın", i + 1),
            Ring::Hole(h) => format!("{}. deliğin {}. kenarının", h + 1, i + 1),
        }
    }
}

fn check_ring(ring: Ring, pts: &[Vec2], bulges: Option<&[f64]>) -> Result<(), Stop> {
    let n = pts.len();
    if n < MIN_CORNERS {
        let message = match ring {
            Ring::Outer => format!(
                "Kapalı alanın en az 3 köşesi olmalı; {n} köşe verildi. Eksik köşeleri ekleyin."
            ),
            Ring::Hole(h) => format!(
                "{}. deliğin en az 3 köşesi olmalı; {n} köşe verildi. Eksik köşeleri ekleyin ya da deliği çıkarın.",
                h + 1
            ),
        };
        return Err(Stop::failed(error(
            codes::TOO_FEW_CORNERS,
            message,
            Some(ring.path("pts")),
        )));
    }
    for (i, p) in pts.iter().enumerate() {
        for (axis, value) in [("x", p.x), ("y", p.y)] {
            if !value.is_finite() {
                return Err(checks::not_finite(
                    &ring.corner(i),
                    axis,
                    format!("{}[{i}].{axis}", ring.path("pts")),
                ));
            }
        }
    }
    let Some(bulges) = bulges else {
        return Ok(());
    };
    if bulges.len() != n {
        let whose = match ring {
            Ring::Outer => "Yay değerleri".to_owned(),
            Ring::Hole(h) => format!("{}. deliğin yay değerleri", h + 1),
        };
        return Err(Stop::failed(error(
            codes::BULGE_COUNT,
            format!(
                "{whose} köşe sayısı kadar olmalı, kapanış kenarı dahil her kenara bir değer: {n} köşe, {} yay değeri verildi. Eksik ya da fazla değerleri düzeltin.",
                bulges.len()
            ),
            Some(ring.path("bulges")),
        )));
    }
    for (i, b) in bulges.iter().enumerate() {
        if !b.is_finite() {
            return Err(Stop::failed(error(
                codes::NOT_FINITE,
                format!(
                    "{} yay değeri sonlu bir sayı değil (NaN ya da sonsuz). Düz kenar için 0, yay için tan(açı/4) verin.",
                    ring.edge(i)
                ),
                Some(format!("{}[{i}]", ring.path("bulges"))),
            )));
        }
    }
    Ok(())
}

/// The polygon `input` describes, as the document stores it; `id` is the
/// document's to give (0 in a plan).
fn polygon(input: PolygonCreate, id: u32) -> Entity {
    Entity::Polygon(PathEntity {
        ms: None,
        base: EntityBase {
            look: kentos_commands::ObjectLook {
                line_type: input.line_type,
                wipeout: input.wipeout,
                ..Default::default()
            },
            id,
            layer_id: input.layer_id,
            color: input.color,
            attrs: input.attrs.unwrap_or_default(),
            label: input.label,
            symbol: input.symbol,
            line_weight: input.line_weight,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        pts: input.pts,
        bulges: input.bulges,
        holes: input.holes,
        zs: None,
        parts: None,
        closed: false,
        widths: None,
    })
}

/// `input` with the attributes the object is written with: canonical, the fields' defaults filled
/// (`checks::object_attrs`; the checks refused what it refuses).
fn settled(doc: &Document, mut input: PolygonCreate) -> PolygonCreate {
    input.attrs = checks::object_attrs(doc, &input.layer_id, input.attrs.as_ref(), "attrs")
        .ok()
        .flatten();
    input
}
