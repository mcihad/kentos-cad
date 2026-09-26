//! `cad.entities.array` v1 (docs/adr/0047): copies of objects named by their
//! persistent ids laid out in rows and columns, or around a centre, as one
//! undo step. The desktop's handler over the native document; the web's is
//! `apps/web/src/product/entitiesArray.ts`. Both pass the shared cases in
//! `fixtures/commands/v1/cad.entities.array.json`.
//!
//! The array tools (Dizi, Kutupsal dizi) make the selection explicit here
//! (TODOS.md CMD-07): they give the selected objects' ids. The layout is the
//! shared core's, as on the web: the copies' affines (`array_transforms`,
//! a polar array's middle measured in the drawing's typeface) and what each
//! does to every kind of object (`transform_shape`); nothing is computed here.
//!
//! The checks, in order (the first that fails answers):
//! 1. at least one id; every id lowercase UUID text with hyphens (in order);
//! 2. the layout's numbers finite, in their order; the counts in their
//!    ranges; a spacing for each grid direction with more than one place, a
//!    polar fill that is not zero and not past a full turn;
//! 3. the expected revision (every command's, `checks.rs`);
//! 4. every id names an object of the document (in order);
//! 5. not every object on a locked layer (with others, a warning);
//! 6. no copy carried past the largest float64.
//!
//! Objects on a locked layer are not copied (docs/adr/0037). The web's
//! tools used to copy them onto their locked layer; ADR 0047 records the
//! change for the arrays.

use kentos_contracts::{
    ArrayLayout, CommandError, CommandResult, CommandWarning, EntitiesArray, EntitiesArrayPlan,
    EntitiesArrayed, Entity,
};
use kentos_domain::Document;
use kentos_geometry_core::ops::transform::transform_shape;
use kentos_geometry_core::tools::editing::array_transforms;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;
use crate::geometry::{drawing_font, shape, with_shape};
use crate::transform::{base_mut, finite_shape};

/// Less than this far apart, a spacing or a fill angle is none (the web's
/// tools took a nanometre, or a nano-degree, as zero).
const NONE: f64 = 1e-9;

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesArray) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(checked) => CommandResult::Completed {
            output: (),
            warnings: checked.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now: the copies as they would be, and the
/// revision to expect for exactly that; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &EntitiesArray) -> CommandResult<EntitiesArrayPlan> {
    match check(cx.doc, input) {
        Ok(checked) => CommandResult::Completed {
            output: EntitiesArrayPlan {
                sources: checked.sources,
                entities: checked.copies,
                locked: checked.locked,
                revision: cx.doc.revision().to_string(),
            },
            warnings: checked.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and adds the copies as one undo step named after the
/// tool, into the open transaction or group if one is. Nothing is written
/// when the document has no slot left for them.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: EntitiesArray,
) -> CommandResult<EntitiesArrayed> {
    let checked = match check(cx.doc, &input) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    let slots = match cx.doc.add_many(checked.copies, label(&input.layout)) {
        Ok(slots) => slots,
        Err(full) => {
            return CommandResult::Failed {
                error: CommandError {
                    code: codes::SLOTS_EXHAUSTED.into(),
                    message: full.to_string(),
                    path: None,
                    revision: None,
                },
            };
        }
    };
    let doc = &*cx.doc;
    let created = slots
        .iter()
        .filter_map(|slot| doc.uid(*slot))
        .map(|uid| uid.to_string())
        .collect();
    CommandResult::Completed {
        output: EntitiesArrayed {
            created,
            locked: checked.locked,
            revision: doc.revision().to_string(),
        },
        warnings: checked.warnings,
    }
}

/// The undo step's name: the tool's (docs/adr/0047).
pub fn label(layout: &ArrayLayout) -> &'static str {
    match layout {
        ArrayLayout::Grid { .. } => "Dizi",
        ArrayLayout::Polar { .. } => "Kutupsal dizi",
    }
}

/// The layout as the core's `array_transforms` takes it: its kind and numbers.
pub fn array_numbers(layout: &ArrayLayout) -> (&'static str, Vec<f64>) {
    match *layout {
        ArrayLayout::Grid { rows, cols, dx, dy } => {
            ("grid", vec![f64::from(rows), f64::from(cols), dx, dy])
        }
        ArrayLayout::Polar {
            center,
            count,
            fill,
            rotate,
        } => (
            "polar",
            vec![
                center.x,
                center.y,
                f64::from(count),
                fill,
                if rotate { 1.0 } else { 0.0 },
            ],
        ),
    }
}

/// What may be written: the ids of the objects copied, the copies (slot 0)
/// place after place, those left on locked layers and the warning about them.
struct Checked {
    sources: Vec<String>,
    copies: Vec<Entity>,
    locked: Vec<String>,
    warnings: Vec<CommandWarning>,
}

fn refuse(code: &str, message: &str, path: &str) -> Stop {
    Stop::Failed(checks::error(code, message.into(), Some(path.into())))
}

fn locked_message(n: usize) -> String {
    format!(
        "{n} nesne kilitli katmanda olduğu için atlandı. Kopyalamak için katmanın kilidini Katmanlar panelinden açın."
    )
}

/// The layout's own checks, in its fields' order: finite numbers, then the
/// counts, then the spacing or the fill.
fn check_layout(layout: &ArrayLayout) -> Result<(), Stop> {
    match *layout {
        ArrayLayout::Grid { rows, cols, dx, dy } => {
            let fix = "Aralığı sonlu bir sayıyla verin.";
            checks::finite(dx, "Doğu (Y) yönündeki aralık", fix, "layout.dx")?;
            checks::finite(dy, "Kuzey (X) yönündeki aralık", fix, "layout.dy")?;
            let count = "Satır × sütun 2 ile 10 000 arasında olmalı; satır ve sütun en az 1'dir. Başka bir satır ve sütun sayısı verin.";
            if rows == 0 {
                return Err(refuse(codes::INVALID_COUNT, count, "layout.rows"));
            }
            if cols == 0 {
                return Err(refuse(codes::INVALID_COUNT, count, "layout.cols"));
            }
            let places = u64::from(rows) * u64::from(cols);
            if !(2..=10_000).contains(&places) {
                return Err(refuse(codes::INVALID_COUNT, count, "layout"));
            }
            if cols > 1 && dx.abs() < NONE {
                return Err(refuse(
                    codes::INVALID_SPACING,
                    "Sütunlar arasındaki aralık (dY) sıfır; kopyalar üst üste düşer. Sıfırdan farklı bir aralık verin.",
                    "layout.dx",
                ));
            }
            if rows > 1 && dy.abs() < NONE {
                return Err(refuse(
                    codes::INVALID_SPACING,
                    "Satırlar arasındaki aralık (dX) sıfır; kopyalar üst üste düşer. Sıfırdan farklı bir aralık verin.",
                    "layout.dy",
                ));
            }
        }
        ArrayLayout::Polar {
            center,
            count,
            fill,
            ..
        } => {
            checks::point(center, "Merkezin", "layout.center")?;
            checks::finite(
                fill,
                "Doldurma açısı",
                "Açıyı sonlu bir sayıyla verin.",
                "layout.fill",
            )?;
            if !(2..=1_000).contains(&count) {
                return Err(refuse(
                    codes::INVALID_COUNT,
                    "Adet 2 ile 1000 arasında bir tam sayı olmalı. Başka bir adet verin.",
                    "layout.count",
                ));
            }
            if fill.abs() < NONE || fill.abs() > 360.0 {
                return Err(refuse(
                    codes::INVALID_FILL,
                    "Doldurma açısı 0 ile ±360 derece arasında olmalı; 0 olamaz. Başka bir açı verin.",
                    "layout.fill",
                ));
            }
        }
    }
    Ok(())
}

/// The checks in the contract's order.
fn check(doc: &Document, input: &EntitiesArray) -> Result<Checked, Stop> {
    checks::uids(&input.uids, "Diziye alınacak nesne verilmedi.")?;
    check_layout(&input.layout)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let mut sources = Vec::new();
    let mut originals = Vec::new();
    let mut locked = Vec::new();
    for (_, entity, uid) in checks::objects(doc, &input.uids)? {
        if doc.layers().is_locked(&entity.base().layer_id) {
            locked.push(uid.clone());
        } else {
            sources.push(uid.clone());
            originals.push((entity, shape(entity)));
        }
    }
    if sources.is_empty() {
        return Err(refuse(
            codes::LAYER_LOCKED,
            &locked_message(locked.len()),
            "uids",
        ));
    }
    let shapes: Vec<_> = originals.iter().map(|(_, s)| s.clone()).collect();
    let (kind, numbers) = array_numbers(&input.layout);
    let font = drawing_font(doc.settings().drawing_font);
    let Some(affines) = array_transforms(kind, &numbers, &shapes, font) else {
        return Err(refuse(codes::NOT_FINITE, "Dizi kurulamadı.", "layout"));
    };
    let mut copies = Vec::with_capacity(affines.len() * originals.len());
    let mut overflow = false;
    for m in &affines {
        for (entity, before) in &originals {
            let after = transform_shape(before, m);
            overflow |= finite_shape(before) && !finite_shape(&after);
            // The core keeps an object's kind; a shape of another kind would be its fault.
            let Some(mut copy) = with_shape(entity, after) else {
                return Err(refuse(codes::NOT_FINITE, "Nesne kopyalanamadı.", "layout"));
            };
            // A copy's slot is given when it is written.
            base_mut(&mut copy).id = 0;
            copies.push(copy);
        }
    }
    if overflow {
        return Err(refuse(
            codes::NOT_FINITE,
            "Dönüşüm sonucunda sonlu olmayan bir değer çıktı (sayı taşması). Daha küçük bir değer verin.",
            "layout",
        ));
    }
    let mut warnings = Vec::new();
    if !locked.is_empty() {
        warnings.push(CommandWarning {
            code: codes::LAYER_LOCKED.into(),
            message: locked_message(locked.len()),
            path: Some("uids".into()),
        });
    }
    Ok(Checked {
        sources,
        copies,
        locked,
        warnings,
    })
}
