//! `cad.annotations.scales` v1 (docs/adr/0323 §4): texts, dimensions and
//! leaders made annotative, their scales added and removed, one scale's own
//! values set and reset, as one undo step. The rules of a representation are
//! the contracts' (`kentos_commands::annotative`); the shared cases are
//! `fixtures/commands/v1/cad.annotations.scales.json`.
//!
//! The checks, in order: the ids, the operation's own input (`no_scales`,
//! `no_scale`, `invalid_scale`, `no_value`, `not_finite`), the revision, the
//! objects (`entity_not_found`), their layers (`layer_locked`), each object
//! (`not_annotative` when none is left, `own_scale`, `scale_not_taken`,
//! `invalid_annotative`).

use kentos_commands::{
    AnnotationsScaled, AnnotationsScales, AnnotationsScalesPlan, Annotative, AnnotativeOperation,
    AnnotativeScale, CommandResult, CommandWarning, Entity, annotative_field, annotative_of,
    annotative_problem, annotative_scale_holds, entry_problem, same_scale, why_not_annotative,
    with_scale, without_scale,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &AnnotationsScales) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed {
            output: (),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would change, and the revision to expect for exactly that.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &AnnotationsScales,
) -> CommandResult<AnnotationsScalesPlan> {
    match check(cx.doc, input) {
        Ok((changes, warnings)) => CommandResult::Completed {
            output: AnnotationsScalesPlan {
                changed: changes.into_iter().map(|c| c.entity).collect(),
                revision: cx.doc.revision().to_string(),
            },
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the objects as one undo step.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: AnnotationsScales,
) -> CommandResult<AnnotationsScaled> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_ANNOTATIONS_SCALES,
        kentos_commands::CAD_ANNOTATIONS_SCALES_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: AnnotationsScales,
) -> CommandResult<AnnotationsScaled> {
    let (changes, warnings) = match check(cx.doc, &input) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    let (uids, slots): (Vec<String>, Vec<Slot>) =
        changes.iter().map(|c| (c.uid.clone(), c.slot)).unzip();
    if !changes.is_empty() {
        let writes = changes.into_iter().map(|c| (c.slot, c.entity)).collect();
        cx.doc.update_many(writes, label(input.operation));
    }
    CommandResult::Completed {
        output: AnnotationsScaled {
            changed: uids,
            ids: slots.iter().map(|s| s.0).collect(),
            revision: cx.doc.revision().to_string(),
        },
        warnings,
    }
}

/// The undo step's name.
pub fn label(operation: AnnotativeOperation) -> &'static str {
    match operation {
        AnnotativeOperation::Make => "Ölçekli yap",
        AnnotativeOperation::Unmake => "Ölçekliliği kaldır",
        AnnotativeOperation::Add => "Ölçek ekle",
        AnnotativeOperation::Remove => "Ölçek kaldır",
        AnnotativeOperation::Set => "Ölçekteki yeri",
        AnnotativeOperation::Reset => "Ölçeği sıfırla",
    }
}

/// An object written: its slot, id and itself as it will be.
struct Change {
    slot: Slot,
    uid: String,
    entity: Entity,
}

fn refused(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(checks::error(code, message.into(), Some(path.into())))
}

/// A scale as the interface writes it: 1:100, 1:12.5.
fn ratio(scale: f64) -> String {
    format!(
        "1:{}",
        kentos_geometry_core::display::fixed(scale, 6)
            .trim_end_matches('0')
            .trim_end_matches('.')
    )
}

/// The operation's own input, before the document is read.
fn input_rule(input: &AnnotationsScales) -> Result<(), Stop> {
    use AnnotativeOperation as Op;
    if matches!(input.operation, Op::Add | Op::Remove) && input.scales.is_empty() {
        return Err(refused(
            codes::NO_SCALES,
            format!(
                "{} için ölçek verilmedi: scales'e 1:N ölçeklerinin N'lerini yazın (50, 100 gibi).",
                label(input.operation)
            ),
            "scales",
        ));
    }
    if input.operation == Op::Set && input.scale.is_none() {
        return Err(refused(
            codes::NO_SCALE,
            "Değerleri yazılacak ölçek verilmedi: scale'e o ölçeğin N'sini yazın (1:50 için 50).",
            "scale",
        ));
    }
    for (i, s) in input.scales.iter().enumerate() {
        if !annotative_scale_holds(*s) {
            return Err(refused(
                codes::INVALID_SCALE,
                format!(
                    "Ölçek {s} olamaz: sıfırdan büyük, en çok 1 000 000 olmalı (1:N'nin N'si)."
                ),
                &format!("scales[{i}]"),
            ));
        }
    }
    if let Some(s) = input.scale
        && !annotative_scale_holds(s)
    {
        return Err(refused(
            codes::INVALID_SCALE,
            format!("Ölçek {s} olamaz: sıfırdan büyük, en çok 1 000 000 olmalı (1:N'nin N'si)."),
            "scale",
        ));
    }
    if input.operation == Op::Set {
        if input.point.is_none()
            && input.rotation.is_none()
            && input.offset.is_none()
            && input.text_shift.is_none()
        {
            return Err(refused(
                codes::NO_VALUE,
                "Ölçekteki yeri için değer verilmedi: yazının ve kılavuzun point ya da rotation'ı, ölçünün offset ya da textShift'i.",
                "point",
            ));
        }
        if let Some(p) = input.point {
            checks::point(p, "Ölçekteki yer", "point")?;
        }
        if let Some(r) = input.rotation {
            checks::finite(r, "Doğrultu", "Sonlu bir açı verin.", "rotation")?;
        }
        if let Some(o) = input.offset {
            checks::finite(o, "Çizginin uzaklığı", "Sonlu bir uzaklık verin.", "offset")?;
        }
        if let Some(t) = input.text_shift {
            checks::finite(
                t.along,
                "Değerin kayması",
                "Sonlu bir uzaklık verin.",
                "textShift.along",
            )?;
            checks::finite(
                t.across,
                "Değerin kayması",
                "Sonlu bir uzaklık verin.",
                "textShift.across",
            )?;
        }
    }
    Ok(())
}

/// `e` with its scales as `a`; none (one height) when `a` is none.
fn with_scales(e: &Entity, a: Option<Annotative>) -> Entity {
    let mut out = e.clone();
    if let Some(field) = annotative_field(&mut out) {
        *field = a.map(Box::new);
    }
    out
}

/// One object after the operation; none when it does not change. `Err` for a refusal at `at`.
fn changed(
    e: &Entity,
    input: &AnnotationsScales,
    plot: f64,
    at: usize,
) -> Result<Option<Entity>, Stop> {
    use AnnotativeOperation as Op;
    let own = annotative_of(e);
    let next = match (input.operation, own) {
        (Op::Make, None) => {
            let mut scales: Vec<AnnotativeScale> = Vec::new();
            for s in &input.scales {
                if !same_scale(*s, plot) && !scales.iter().any(|x| same_scale(x.scale, *s)) {
                    scales.push(AnnotativeScale::plain(*s));
                }
            }
            scales.sort_by(|x, y| x.scale.total_cmp(&y.scale));
            Some(with_scales(e, Some(Annotative { at: plot, scales })))
        }
        (Op::Make | Op::Add, Some(_)) => {
            let mut out = e.clone();
            for s in &input.scales {
                if let Some(next) = with_scale(&out, *s) {
                    out = next;
                }
            }
            Some(out)
        }
        (Op::Unmake, Some(_)) => Some(with_scales(e, None)),
        (Op::Remove, Some(a)) => {
            if let Some((i, s)) = input
                .scales
                .iter()
                .enumerate()
                .find(|(_, s)| same_scale(**s, a.at))
            {
                return Err(refused(
                    codes::OWN_SCALE,
                    format!(
                        "{} nesnenin kendi ölçeği; kaldırılamaz. Nesnenin kendisini başka ölçekte istiyorsanız projenin çizim ölçeğini değiştirin ya da Ölçekliliği kaldır'ı kullanın.",
                        ratio(*s)
                    ),
                    &format!("scales[{i}]"),
                ));
            }
            let mut out = e.clone();
            for s in &input.scales {
                if let Some(next) = without_scale(&out, *s) {
                    out = next;
                }
            }
            Some(out)
        }
        (Op::Set, Some(a)) => {
            let scale = input.scale.unwrap_or(a.at);
            if same_scale(scale, a.at) {
                return Err(refused(
                    codes::OWN_SCALE,
                    format!(
                        "{} nesnenin kendi ölçeği: oradaki yeri nesnenin kendisidir. Nesneyi taşıyın ya da döndürün.",
                        ratio(scale)
                    ),
                    "scale",
                ));
            }
            let Some(entry) = a.entry(scale) else {
                return Err(refused(
                    codes::SCALE_NOT_TAKEN,
                    format!(
                        "Nesne {} ölçeğinde değil: önce Ölçek ekle ile ekleyin.",
                        ratio(scale)
                    ),
                    &format!("uids[{at}]"),
                ));
            };
            let mut entry = entry.clone();
            if input.point.is_some() {
                entry.point = input.point;
            }
            if input.rotation.is_some() {
                entry.rotation = input.rotation;
            }
            if input.offset.is_some() {
                entry.offset = input.offset;
            }
            if input.text_shift.is_some() {
                entry.text_shift = input.text_shift;
            }
            if let Some(words) = entry_problem(e, &entry) {
                return Err(refused(
                    codes::INVALID_ANNOTATIVE,
                    words,
                    &format!("uids[{at}]"),
                ));
            }
            let scales = a
                .scales
                .iter()
                .map(|s| {
                    if same_scale(s.scale, scale) {
                        entry.clone()
                    } else {
                        s.clone()
                    }
                })
                .collect();
            Some(with_scales(e, Some(Annotative { at: a.at, scales })))
        }
        (Op::Reset, Some(a)) => {
            let scales = a
                .scales
                .iter()
                .map(|s| {
                    if input.scales.is_empty()
                        || input.scales.iter().any(|x| same_scale(*x, s.scale))
                    {
                        AnnotativeScale::plain(s.scale)
                    } else {
                        s.clone()
                    }
                })
                .collect();
            Some(with_scales(e, Some(Annotative { at: a.at, scales })))
        }
        (_, None) => None,
    };
    let Some(next) = next else {
        return Ok(None);
    };
    if let Some(words) = annotative_problem(&next) {
        return Err(refused(
            codes::INVALID_ANNOTATIVE,
            words,
            &format!("uids[{at}]"),
        ));
    }
    Ok((next != *e).then_some(next))
}

fn check(
    doc: &Document,
    input: &AnnotationsScales,
) -> Result<(Vec<Change>, Vec<CommandWarning>), Stop> {
    checks::uids(&input.uids, "Ölçekleri değişecek nesne verilmedi.")?;
    input_rule(input)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let named = checks::named(doc, &input.uids)?;
    let (named, locked) = checks::skip_locked(doc, named)?;
    let plot = doc.settings().plot_scale;
    let mut warnings: Vec<CommandWarning> = locked.into_iter().collect();
    let mut left_out: Vec<(usize, &'static str)> = Vec::new();
    let mut out = Vec::new();
    let total = named.len();
    for (at, slot, entity, uid) in named {
        if let Some(why) = why_not_annotative(entity) {
            left_out.push((at, why));
            continue;
        }
        if input.operation != AnnotativeOperation::Make && annotative_of(entity).is_none() {
            left_out.push((at, "Nesne ölçekli değil; önce Ölçekli yap'ı kullanın."));
            continue;
        }
        if let Some(next) = changed(entity, input, plot, at)? {
            out.push(Change {
                slot,
                uid: uid.clone(),
                entity: next,
            });
        }
    }
    if let Some((at, why)) = left_out.first() {
        // Every object left out: the refusal, with the first one's reason.
        if left_out.len() == total {
            return Err(refused(
                codes::NOT_ANNOTATIVE,
                (*why).to_owned(),
                &format!("uids[{at}]"),
            ));
        }
        warnings.push(CommandWarning {
            code: codes::NOT_ANNOTATIVE.into(),
            message: format!(
                "{} nesne ölçekli olmadığı ya da olamadığı için atlandı (yalnız yazı, ölçü ve kılavuz; eğri boyunca ve bağlı yazı değil).",
                left_out.len()
            ),
            path: Some("uids".into()),
        });
    }
    Ok((out, warnings))
}
