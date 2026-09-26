//! `cad.polygon.create` v1 (docs/adr/0022): one closed area on a named
//! layer, as one undo step. The desktop's handler over the native document;
//! the web's is `apps/web/src/product/polygonCreate.ts`. Both pass the shared
//! cases in `fixtures/commands/v1/cad.polygon.create.json`, which fix the
//! checks, their order, the codes, paths and messages.
//!
//! The checks, in order (the first that fails answers):
//! 1. the outer ring, then each hole: at least 3 corners, every coordinate
//!    finite, one bulge per edge when bulges are given, every bulge finite;
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

use kentos_contracts::{
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
    match check(cx.doc, input) {
        Ok(warnings) => CommandResult::Completed {
            output: (),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now, and the revision to expect for exactly
/// that; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &PolygonCreate) -> CommandResult<PolygonPlan> {
    match check(cx.doc, input) {
        Ok(warnings) => CommandResult::Completed {
            output: PolygonPlan {
                entity: polygon(input.clone(), 0),
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
    let warnings = match check(cx.doc, &input) {
        Ok(warnings) => warnings,
        Err(stop) => return stop.into(),
    };
    match checks::add(cx.doc, polygon(input, 0)) {
        Ok((id, uid, revision)) => CommandResult::Completed {
            output: PolygonCreated { uid, id, revision },
            warnings,
        },
        Err(error) => CommandResult::Failed { error },
    }
}

/// The checks in the contract's order; the warnings when the polygon may be written.
fn check(doc: &Document, input: &PolygonCreate) -> Result<Vec<CommandWarning>, Stop> {
    check_ring(Ring::Outer, &input.pts, input.bulges.as_deref())?;
    for (h, hole) in input.holes.iter().flatten().enumerate() {
        check_ring(Ring::Hole(h), &hole.pts, hole.bulges.as_deref())?;
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    checks::layer(doc, &input.layer_id)
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
        return Err(Stop::Failed(error(
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
        return Err(Stop::Failed(error(
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
            return Err(Stop::Failed(error(
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
        base: EntityBase {
            id,
            layer_id: input.layer_id,
            color: input.color,
            attrs: input.attrs.unwrap_or_default(),
            label: None,
            symbol: None,
        },
        pts: input.pts,
        bulges: input.bulges,
        holes: input.holes,
    })
}
