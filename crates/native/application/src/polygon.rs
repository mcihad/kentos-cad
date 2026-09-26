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
//! so the conflict comes first.
//!
//! Nothing here is geometry: the checks are counts, finite numbers and the
//! layer tree's state. Geometric validity (a ring crossing itself, zero area,
//! a hole outside the ring) is not checked, as the drawing tools and the file
//! reader do not check it either.

use kentos_contracts::{
    CommandError, CommandResult, CommandWarning, Entity, EntityBase, LayerNodeType, PathEntity,
    PolygonCreate, PolygonCreated, PolygonPlan, Vec2,
};
use kentos_domain::Document;

use crate::ExecutionContext;

/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub mod codes {
    pub const TOO_FEW_CORNERS: &str = "too_few_corners";
    pub const NOT_FINITE: &str = "not_finite";
    pub const BULGE_COUNT: &str = "bulge_count";
    pub const INVALID_REVISION: &str = "invalid_revision";
    pub const REVISION_CONFLICT: &str = "revision_conflict";
    pub const LAYER_NOT_FOUND: &str = "layer_not_found";
    pub const NOT_A_LAYER: &str = "not_a_layer";
    pub const LAYER_LOCKED: &str = "layer_locked";
    /// The desktop only: every slot (`u32`) of the document has been given out.
    pub const SLOTS_EXHAUSTED: &str = "slots_exhausted";
    /// A warning: the layer is hidden, by itself or a group above it.
    pub const LAYER_HIDDEN: &str = "layer_hidden";
}

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
    let slot = match cx.doc.add(polygon(input, 0)) {
        Ok(slot) => slot,
        Err(full) => {
            return CommandResult::Failed {
                error: error(codes::SLOTS_EXHAUSTED, full.to_string(), None),
            };
        }
    };
    CommandResult::Completed {
        output: PolygonCreated {
            uid: cx.doc.uid(slot).map(|u| u.to_string()).unwrap_or_default(),
            id: slot.0,
            revision: cx.doc.revision().to_string(),
        },
        warnings,
    }
}

/// Why a call stops before anything is written.
enum Stop {
    Failed(CommandError),
    Conflict(CommandError),
}

impl<T> From<Stop> for CommandResult<T> {
    fn from(stop: Stop) -> Self {
        match stop {
            Stop::Failed(error) => CommandResult::Failed { error },
            Stop::Conflict(error) => CommandResult::Conflict { error },
        }
    }
}

/// The checks in the contract's order; the warnings when the polygon may be written.
fn check(doc: &Document, input: &PolygonCreate) -> Result<Vec<CommandWarning>, Stop> {
    check_ring(Ring::Outer, &input.pts, input.bulges.as_deref()).map_err(Stop::Failed)?;
    for (h, hole) in input.holes.iter().flatten().enumerate() {
        check_ring(Ring::Hole(h), &hole.pts, hole.bulges.as_deref()).map_err(Stop::Failed)?;
    }
    if let Some(expected) = &input.expected_revision {
        if !is_revision_text(expected) {
            return Err(Stop::Failed(error(
                codes::INVALID_REVISION,
                format!(
                    "Beklenen sürüm “{expected}” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."
                ),
                Some("expectedRevision".into()),
            )));
        }
        let current = doc.revision().to_string();
        if *expected != current {
            return Err(Stop::Conflict(CommandError {
                revision: Some(current),
                ..error(
                    codes::REVISION_CONFLICT,
                    "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.".into(),
                    Some("expectedRevision".into()),
                )
            }));
        }
    }
    let layers = doc.layers();
    let id = input.layer_id.as_str();
    let at = || Some("layerId".to_owned());
    let Some(node) = layers.get(id) else {
        return Err(Stop::Failed(error(
            codes::LAYER_NOT_FOUND,
            format!("“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."),
            at(),
        )));
    };
    let name = &node.name;
    if node.kind != LayerNodeType::Layer {
        return Err(Stop::Failed(error(
            codes::NOT_A_LAYER,
            format!(
                "“{name}” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin."
            ),
            at(),
        )));
    }
    // The closed-area tool's words (web `tools/targetLayer.ts`), kept since the tool writes through here.
    if layers.is_locked(id) {
        return Err(Stop::Failed(error(
            codes::LAYER_LOCKED,
            format!(
                "“{name}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin."
            ),
            at(),
        )));
    }
    let mut warnings = Vec::new();
    if !layers.is_visible(id) {
        warnings.push(CommandWarning {
            code: codes::LAYER_HIDDEN.into(),
            message: format!("“{name}” katmanı gizli; çizilen nesne görünmeyecek."),
            path: at(),
        });
    }
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

fn check_ring(ring: Ring, pts: &[Vec2], bulges: Option<&[f64]>) -> Result<(), CommandError> {
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
        return Err(error(
            codes::TOO_FEW_CORNERS,
            message,
            Some(ring.path("pts")),
        ));
    }
    for (i, p) in pts.iter().enumerate() {
        for (axis, value, name) in [("x", p.x, "doğu (Y)"), ("y", p.y, "kuzey (X)")] {
            if !value.is_finite() {
                return Err(error(
                    codes::NOT_FINITE,
                    format!(
                        "{} {name} değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.",
                        ring.corner(i)
                    ),
                    Some(format!("{}[{i}].{axis}", ring.path("pts"))),
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
        return Err(error(
            codes::BULGE_COUNT,
            format!(
                "{whose} köşe sayısı kadar olmalı, kapanış kenarı dahil her kenara bir değer: {n} köşe, {} yay değeri verildi. Eksik ya da fazla değerleri düzeltin.",
                bulges.len()
            ),
            Some(ring.path("bulges")),
        ));
    }
    for (i, b) in bulges.iter().enumerate() {
        if !b.is_finite() {
            return Err(error(
                codes::NOT_FINITE,
                format!(
                    "{} yay değeri sonlu bir sayı değil (NaN ya da sonsuz). Düz kenar için 0, yay için tan(açı/4) verin.",
                    ring.edge(i)
                ),
                Some(format!("{}[{i}]", ring.path("bulges"))),
            ));
        }
    }
    Ok(())
}

/// Decimal text of a whole number, no sign, no leading zero: `0`, `12`.
/// Any length: a revision the document never had is a conflict, not an error.
fn is_revision_text(text: &str) -> bool {
    let digits = text.bytes().all(|b| b.is_ascii_digit());
    digits && !text.is_empty() && (text == "0" || !text.starts_with('0'))
}

fn error(code: &str, message: String, path: Option<String>) -> CommandError {
    CommandError {
        code: code.to_owned(),
        message,
        path,
        revision: None,
    }
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
