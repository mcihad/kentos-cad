//! `cad.polyline.create` v1 (docs/adr/0027): one open polyline on a named
//! layer, as one undo step. The desktop's handler over the native document;
//! the web's is `apps/web/src/product/polylineCreate.ts`. Both pass the shared
//! cases in `fixtures/commands/v1/cad.polyline.create.json`.
//!
//! The checks, in order (the first that fails answers):
//! 1. at least 2 points, every coordinate finite (x before y), one bulge per
//!    edge when bulges are given (one fewer than the points), every bulge
//!    finite;
//! 2. the expected revision, 3. the layer: every create command's
//!    (`checks.rs`), for the reasons `polygon.rs` gives.
//!
//! Bulges are given one per edge; the document keeps one per point, as DXF's
//! LWPOLYLINE does, so the polyline is stored with a 0 after them for the
//! closing edge it does not have: what the polyline tool always wrote.
//! Otherwise the input is stored as given (zero bulges too).

use kentos_contracts::{
    CommandResult, CommandWarning, Entity, EntityBase, PathEntity, PolylineCreate, PolylineCreated,
    PolylinePlan,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;

/// Points a polyline needs.
const MIN_POINTS: usize = 2;

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &PolylineCreate) -> CommandResult<()> {
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
pub fn plan(cx: &ExecutionContext<'_>, input: &PolylineCreate) -> CommandResult<PolylinePlan> {
    match check(cx.doc, input) {
        Ok(warnings) => CommandResult::Completed {
            output: PolylinePlan {
                entity: polyline(input.clone(), 0),
                revision: cx.doc.revision().to_string(),
            },
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the polyline as one undo step (“Ekle”,
/// the document's own `add`): into the open transaction or group, if one is.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: PolylineCreate,
) -> CommandResult<PolylineCreated> {
    let warnings = match check(cx.doc, &input) {
        Ok(warnings) => warnings,
        Err(stop) => return stop.into(),
    };
    match checks::add(cx.doc, polyline(input, 0)) {
        Ok((id, uid, revision)) => CommandResult::Completed {
            output: PolylineCreated { uid, id, revision },
            warnings,
        },
        Err(error) => CommandResult::Failed { error },
    }
}

/// The checks in the contract's order; the warnings when the polyline may be written.
fn check(doc: &Document, input: &PolylineCreate) -> Result<Vec<CommandWarning>, Stop> {
    let n = input.pts.len();
    if n < MIN_POINTS {
        return Err(Stop::Failed(error(
            codes::TOO_FEW_POINTS,
            format!(
                "Çoklu çizginin en az 2 noktası olmalı; {n} nokta verildi. Eksik noktaları ekleyin."
            ),
            Some("pts".into()),
        )));
    }
    for (i, p) in input.pts.iter().enumerate() {
        for (axis, value) in [("x", p.x), ("y", p.y)] {
            if !value.is_finite() {
                return Err(checks::not_finite(
                    &format!("{}. noktanın", i + 1),
                    axis,
                    format!("pts[{i}].{axis}"),
                ));
            }
        }
    }
    if let Some(bulges) = &input.bulges {
        let edges = n - 1;
        if bulges.len() != edges {
            return Err(Stop::Failed(error(
                codes::BULGE_COUNT,
                format!(
                    "Yay değerleri kenar sayısı kadar olmalı, her kenara bir değer: {n} nokta, {edges} kenar, {} yay değeri verildi. Eksik ya da fazla değerleri düzeltin.",
                    bulges.len()
                ),
                Some("bulges".into()),
            )));
        }
        for (i, b) in bulges.iter().enumerate() {
            if !b.is_finite() {
                return Err(Stop::Failed(error(
                    codes::NOT_FINITE,
                    format!(
                        "{}. kenarın yay değeri sonlu bir sayı değil (NaN ya da sonsuz). Düz kenar için 0, yay için tan(açı/4) verin.",
                        i + 1
                    ),
                    Some(format!("bulges[{i}]")),
                )));
            }
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    checks::layer(doc, &input.layer_id)
}

/// The polyline `input` describes, as the document stores it (bulges one per
/// point, see the module); `id` is the document's to give (0 in a plan).
fn polyline(input: PolylineCreate, id: u32) -> Entity {
    Entity::Polyline(PathEntity {
        base: EntityBase {
            id,
            layer_id: input.layer_id,
            color: input.color,
            attrs: input.attrs.unwrap_or_default(),
            label: None,
            symbol: None,
        },
        pts: input.pts,
        bulges: input.bulges.map(|mut bulges| {
            bulges.push(0.0);
            bulges
        }),
        holes: None,
    })
}
