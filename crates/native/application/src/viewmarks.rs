//! `cad.viewmarks.create`, `.edit` and `.references` v1 (docs/adr/0345 §1–§3; plan/cad.md `CAD-ARCH-37`, `38`): section,
//! elevation and detail marks put, one undo step “Görünüm işareti”; changed, pointed elsewhere or let go, one step
//! “Görünüm işaretini düzenle”; the marks and the targets they may point at listed, read-only. The Kesit, Görünüş and
//! Detay tools preview the plan's marks and write through `execute`; Python and an agent give the same points and get
//! the same marks. The shared cases are `fixtures/commands/v1/cad.viewmarks.{create,edit,references}.json`.
//!
//! `cad.viewmarks.create`'s checks, in order (the first that fails answers): 1. at least one mark (`no_objects`), at
//! most `MAX_VIEWMARKS` (`invalid_viewmark`); 2. each mark in the input's order: its numbers finite (`not_finite`), its
//! fields (`invalid_viewmark`), its target in the project (`target_not_found`); 3. the line weight; 4. the expected
//! revision; 5. the layer. A mark's words are written from its target as the document writes them.

use kentos_contracts::{
    CommandResult, CommandWarning, Entity, EntityBase, MarkReference, NewViewMark, TargetReference,
    Vec2, ViewMarkEntity, ViewMarkFields, ViewMarkForm, ViewMarkLabel, ViewMarksCreate,
    ViewMarksCreated, ViewMarksEdit, ViewMarksEdited, ViewMarksOperation, ViewMarksPlan,
    ViewMarksReferenced, ViewMarksReferences, ViewTarget, view_key,
};
use kentos_domain::viewmark_follow::{SheetFrames, mark_settled, target_place};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

pub const STEP: &str = "Görünüm işareti";
pub const EDIT_STEP: &str = "Görünüm işaretini düzenle";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

fn invalid(words: String, path: &str) -> Stop {
    fail(codes::INVALID_VIEWMARK, words, path)
}

/// The refusal of a target the project does not have.
fn gone(t: &ViewTarget, path: &str) -> Stop {
    let words = match (&t.view, &t.sheet, &t.item) {
        (Some(v), _, _) => format!(
            "“{v}” kimlikli adlı görünüm projede yok. Görünümler penceresindeki bir görünümün kimliğini verin."
        ),
        (_, Some(s), Some(i)) => format!(
            "“{s}” paftasında “{i}” kimlikli görünüm penceresi yok. Pafta kitabındaki bir harita çerçevesini verin."
        ),
        _ => "İşaretin hedefi projede yok.".to_owned(),
    };
    fail(codes::VIEWMARK_TARGET_NOT_FOUND, words, path)
}

/// `mark`'s fields' problem at `path`, or, `targeted`, its target's absence (a mark whose target went stays editable).
fn holds(
    doc: &Document,
    sheets: &[SheetFrames],
    m: &ViewMarkFields,
    targeted: bool,
    path: impl Fn(&str) -> String,
) -> Result<(), Stop> {
    if let Some((field, words)) = m.problem() {
        return Err(invalid(words, &path(&field)));
    }
    if targeted
        && let Some(t) = &m.target
        && target_place(doc, sheets, t).is_none()
    {
        return Err(gone(t, &path("target")));
    }
    Ok(())
}

/// The numbers of a mark to put, finite.
fn finite(n: &NewViewMark, at: &str) -> Result<(), Stop> {
    for (k, p) in n.points.iter().enumerate() {
        checks::point(*p, "İşaretin noktasının", &format!("{at}.points[{k}]"))?;
    }
    if let Some(p) = n.at {
        checks::point(p, "Balonun yerinin", &format!("{at}.at"))?;
    }
    for (what, v, field) in [
        ("Yarıçap", n.radius, "radius"),
        ("Bakış yönü", n.angle, "angle"),
        ("Yazı yüksekliği", n.size, "size"),
    ] {
        if let Some(v) = v {
            checks::finite(v, what, "Sonlu bir sayı verin.", &format!("{at}.{field}"))?;
        }
    }
    Ok(())
}

fn fields_of(n: &NewViewMark) -> ViewMarkFields {
    ViewMarkFields {
        form: n.form,
        points: n.points.clone(),
        radius: n.radius,
        at: n.at,
        angle: n.angle,
        flip: n.flip,
        mark: n.mark.clone(),
        target: n.target.clone(),
        size: n.size,
        words: ViewMarkLabel::default(),
        broken: None,
    }
}

fn check(doc: &Document, input: &ViewMarksCreate) -> Result<(Vec<CommandWarning>, Vec<Entity>), Stop> {
    if input.marks.is_empty() {
        return Err(fail(
            codes::NO_OBJECTS,
            "Görünüm işareti verilmedi. En az bir işaret (marks) verin.".into(),
            "marks",
        ));
    }
    if input.marks.len() > kentos_contracts::MAX_VIEWMARKS {
        return Err(invalid(
            format!(
                "Bir çağrıda en çok {} işaret yazılır; {} verildi.",
                kentos_contracts::MAX_VIEWMARKS,
                input.marks.len()
            ),
            "marks",
        ));
    }
    let sheets = doc.sheet_frames();
    let mut made = Vec::with_capacity(input.marks.len());
    for (i, n) in input.marks.iter().enumerate() {
        let at = format!("marks[{i}]");
        finite(n, &at)?;
        let m = fields_of(n);
        holds(doc, &sheets, &m, true, |f| format!("{at}.{f}"))?;
        made.push(mark_settled(doc, &sheets, &m));
    }
    checks::line_weight(input.line_weight, "lineWeight")?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let warnings = checks::layer(doc, &input.layer_id)?;
    let entities = made
        .into_iter()
        .map(|mark| {
            Entity::ViewMark(ViewMarkEntity {
                base: EntityBase {
                    look: Default::default(),
                    id: 0,
                    layer_id: input.layer_id.clone(),
                    color: input.color.clone(),
                    attrs: Default::default(),
                    label: None,
                    symbol: None,
                    line_weight: input.line_weight,
                    label_pins: Vec::new(),
                    attachments: Vec::new(),
                },
                mark,
            })
        })
        .collect();
    Ok((warnings, entities))
}

pub mod create {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &ViewMarksCreate) -> CommandResult<()> {
        match check(cx.doc, input) {
            Ok((warnings, _)) => CommandResult::Completed {
                output: (),
                warnings,
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(cx: &ExecutionContext<'_>, input: &ViewMarksCreate) -> CommandResult<ViewMarksPlan> {
        plan_in(cx.doc, input)
    }

    /// [`plan`] on a document: what the Kesit, Görünüş and Detay tools draw as their preview.
    pub fn plan_in(doc: &Document, input: &ViewMarksCreate) -> CommandResult<ViewMarksPlan> {
        match check(doc, input) {
            Ok((warnings, marks)) => CommandResult::Completed {
                output: ViewMarksPlan {
                    marks,
                    revision: doc.revision().to_string(),
                },
                warnings,
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: ViewMarksCreate,
    ) -> CommandResult<ViewMarksCreated> {
        crate::call::recorded(
            cx,
            kentos_contracts::CAD_VIEWMARKS_CREATE,
            kentos_contracts::CAD_VIEWMARKS_CREATE_VERSION,
            input,
            write_execute,
        )
    }

    fn write_execute(
        cx: &mut ExecutionContext<'_>,
        input: ViewMarksCreate,
    ) -> CommandResult<ViewMarksCreated> {
        let (warnings, entities) = match check(cx.doc, &input) {
            Ok(checked) => checked,
            Err(stop) => return stop.into(),
        };
        let slots = match cx.doc.add_many(entities, STEP) {
            Ok(slots) => slots,
            Err(full) => {
                return Stop::failed(error(codes::SLOTS_EXHAUSTED, full.to_string(), None)).into();
            }
        };
        CommandResult::Completed {
            output: ViewMarksCreated {
                uids: slots
                    .iter()
                    .filter_map(|s| cx.doc.uid(*s))
                    .map(|u| u.to_string())
                    .collect(),
                ids: slots.iter().map(|s| s.0).collect(),
                revision: cx.doc.revision().to_string(),
            },
            warnings,
        }
    }
}

/// `cad.viewmarks.edit`'s checks, in its order; the marks as they would be.
fn edit_check(doc: &Document, input: &ViewMarksEdit) -> Result<Vec<(Slot, Entity)>, Stop> {
    checks::uids(&input.uids, "Düzenlenecek görünüm işareti verilmedi.")?;
    let found = checks::named(doc, &input.uids)?;
    for (at, _, e, _) in &found {
        if !matches!(e, Entity::ViewMark(_)) {
            return Err(fail(
                codes::NOT_A_VIEWMARK,
                "Nesne görünüm işareti değil; Görünüm işaretini düzenle yalnız kesit, görünüş ve detay işaretlerini değiştirir.".into(),
                &format!("uids[{at}]"),
            ));
        }
    }
    if let ViewMarksOperation::Set(c) = &input.operation {
        if c.mark.is_none()
            && c.size.is_none()
            && c.flip.is_none()
            && c.angle.is_none()
            && c.radius.is_none()
        {
            return Err(fail(
                codes::NOTHING_TO_SET,
                "Değiştirilecek bir alan verilmedi.".into(),
                "operation",
            ));
        }
        for (what, v, field) in [
            ("Yazı yüksekliği", c.size.flatten(), "size"),
            ("Bakış yönü", c.angle, "angle"),
            ("Yarıçap", c.radius, "radius"),
        ] {
            if let Some(v) = v {
                checks::finite(v, what, "Sonlu bir sayı verin.", &format!("operation.{field}"))?;
            }
        }
    }
    let sheets = doc.sheet_frames();
    let mut out = Vec::with_capacity(found.len());
    for (at, slot, e, _) in &found {
        let Entity::ViewMark(m) = e else { continue };
        let mut m = m.clone();
        match &input.operation {
            ViewMarksOperation::Set(c) => {
                if let Some(v) = &c.mark {
                    m.mark.mark = v.clone();
                }
                if let Some(v) = c.size {
                    m.mark.size = v;
                }
                if let Some(v) = c.flip {
                    m.mark.flip = v;
                }
                if let Some(v) = c.angle {
                    if m.mark.form != ViewMarkForm::Elevation {
                        return Err(invalid(
                            "Bakış yönü yalnız görünüş işaretinindir.".into(),
                            "operation.angle",
                        ));
                    }
                    m.mark.angle = Some(v);
                }
                if let Some(v) = c.radius {
                    m.mark.radius = Some(v);
                }
            }
            ViewMarksOperation::Retarget { target } => m.mark.target = Some(target.clone()),
            ViewMarksOperation::Detach => m.mark.target = None,
        }
        let at_input = |f: &str| match (&input.operation, f) {
            (ViewMarksOperation::Set(_), "mark" | "size" | "angle" | "radius") => {
                format!("operation.{f}")
            }
            (ViewMarksOperation::Retarget { .. }, "target") => "operation.target".to_owned(),
            _ => format!("uids[{at}]"),
        };
        let retarget = matches!(input.operation, ViewMarksOperation::Retarget { .. });
        holds(doc, &sheets, &m.mark, retarget, at_input)?;
        m.mark = mark_settled(doc, &sheets, &m.mark);
        out.push((*slot, Entity::ViewMark(m)));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    for (at, _, e, _) in &found {
        let layer = &e.base().layer_id;
        if doc.layers().is_locked(layer) {
            let name = doc
                .layers()
                .get(layer)
                .map_or(layer.as_str(), |n| n.name.as_str());
            return Err(fail(
                codes::LAYER_LOCKED,
                format!(
                    "“{name}” katmanı kilitli; üzerindeki işaret düzenlenemez. Kilidi Katmanlar panelinden açın."
                ),
                &format!("uids[{at}]"),
            ));
        }
    }
    Ok(out)
}

pub mod edit {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &ViewMarksEdit) -> CommandResult<()> {
        match edit_check(cx.doc, input) {
            Ok(_) => CommandResult::Completed {
                output: (),
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(cx: &ExecutionContext<'_>, input: &ViewMarksEdit) -> CommandResult<ViewMarksEdited> {
        plan_in(cx.doc, input)
    }

    /// [`plan`] on a document: what Öznitelikler and Başvurular preview.
    pub fn plan_in(doc: &Document, input: &ViewMarksEdit) -> CommandResult<ViewMarksEdited> {
        match edit_check(doc, input) {
            Ok(work) => CommandResult::Completed {
                output: ViewMarksEdited {
                    marks: work.into_iter().map(|(_, e)| e).collect(),
                    revision: doc.revision().to_string(),
                },
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: ViewMarksEdit,
    ) -> CommandResult<ViewMarksEdited> {
        crate::call::recorded(
            cx,
            kentos_contracts::CAD_VIEWMARKS_EDIT,
            kentos_contracts::CAD_VIEWMARKS_EDIT_VERSION,
            input,
            write_execute,
        )
    }

    fn write_execute(
        cx: &mut ExecutionContext<'_>,
        input: ViewMarksEdit,
    ) -> CommandResult<ViewMarksEdited> {
        let work = match edit_check(cx.doc, &input) {
            Ok(work) => work,
            Err(stop) => return stop.into(),
        };
        let marks = work.iter().map(|(_, e)| e.clone()).collect();
        cx.doc.update_many(work, EDIT_STEP);
        CommandResult::Completed {
            output: ViewMarksEdited {
                marks,
                revision: cx.doc.revision().to_string(),
            },
            warnings: Vec::new(),
        }
    }
}

/// The marks and the targets they may point at (docs/adr/0345 §3).
pub fn references_of(doc: &Document) -> ViewMarksReferenced {
    let sheets = doc.sheet_frames();
    let mut marks = Vec::new();
    for e in doc.entities() {
        let Entity::ViewMark(m) = e else { continue };
        let uid = doc
            .uid(Slot(m.base.id))
            .map(|u| u.to_string())
            .unwrap_or_default();
        marks.push(MarkReference {
            uid,
            form: m.mark.form,
            mark: m.mark.mark.clone(),
            target: m.mark.target.clone(),
            words: m.mark.words.clone(),
            broken: m.mark.broken.is_some(),
            at: m.mark.points.first().copied().unwrap_or(Vec2 { x: 0.0, y: 0.0 }),
        });
    }
    let pointing = |t: &ViewTarget| -> Vec<String> {
        marks
            .iter()
            .filter(|m| m.target.as_ref() == Some(t))
            .map(|m| m.uid.clone())
            .collect()
    };
    let mut targets = Vec::new();
    for v in &doc.settings().named_views {
        let target = ViewTarget::view(&view_key(v));
        targets.push(TargetReference {
            marks: pointing(&target),
            name: v.name.clone(),
            number: v.name.clone(),
            order: None,
            target,
        });
    }
    for s in &sheets {
        for (k, (item, name)) in s.frames.iter().enumerate() {
            let target = ViewTarget::sheet(&s.id, item);
            let frame = if name.is_empty() {
                format!("Çerçeve {}", k + 1)
            } else {
                name.clone()
            };
            targets.push(TargetReference {
                marks: pointing(&target),
                name: format!("{} › {frame}", s.name),
                number: s.name.clone(),
                order: Some(u32::try_from(k + 1).unwrap_or(u32::MAX)),
                target,
            });
        }
    }
    ViewMarksReferenced {
        marks,
        targets,
        revision: doc.revision().to_string(),
    }
}

pub mod references {
    use super::*;

    pub fn validate(_cx: &ExecutionContext<'_>, _input: &ViewMarksReferences) -> CommandResult<()> {
        CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        }
    }

    pub fn plan(
        cx: &ExecutionContext<'_>,
        _input: &ViewMarksReferences,
    ) -> CommandResult<ViewMarksReferenced> {
        CommandResult::Completed {
            output: references_of(cx.doc),
            warnings: Vec::new(),
        }
    }

    /// Read-only: the same answer as the plan.
    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: ViewMarksReferences,
    ) -> CommandResult<ViewMarksReferenced> {
        plan(cx, &input)
    }
}
