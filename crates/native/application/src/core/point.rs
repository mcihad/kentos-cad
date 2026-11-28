//! `cad.point.create` v1 (docs/adr/0032): one point object on a named layer,
//! as one undo step. The desktop's handler over the native document; the
//! web's is `apps/web/src/product/pointCreate.ts`. Both pass the shared cases
//! in `fixtures/commands/v1/cad.point.create.json`.
//!
//! The point tool calls it once per point it places. Its elevation and the
//! text beside it are optional (the spot elevation tool writes both). With
//! `nameText` its name is written as a text object too, linked to the point
//! (docs/adr/9314): the document keeps it with the point from then on.
//!
//! The checks, in order (the first that fails answers):
//! 1. the point finite, x before y, then the elevation when given;
//! 2. the expected revision, 3. the layer: every create command's
//!    (`checks.rs`), for the reasons `polygon.rs` gives.

use kentos_commands::{
    AnnotationKind, CommandResult, CommandWarning, DrawingFont, Entity, EntityBase, EntityId,
    PointCreate, PointCreated, PointEntity, PointPlan, TextEntity, Vec2,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &PointCreate) -> CommandResult<()> {
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
pub fn plan(cx: &ExecutionContext<'_>, input: &PointCreate) -> CommandResult<PointPlan> {
    match check(cx.doc, input) {
        Ok(warnings) => CommandResult::Completed {
            output: PointPlan {
                entity: point(settled(cx.doc, input.clone()), 0),
                text: name_text(cx.doc, &input, None),
                revision: cx.doc.revision().to_string(),
            },
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the point as one undo step (“Ekle”, the
/// document's own `add`): into the open transaction or group, if one is.
pub fn execute(cx: &mut ExecutionContext<'_>, input: PointCreate) -> CommandResult<PointCreated> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_POINT_CREATE,
        kentos_commands::CAD_POINT_CREATE_VERSION,
        input,
        write_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step of the
/// document, all or nothing, and journals it (docs/adr/0253 §1, §3).
fn write_execute(cx: &mut ExecutionContext<'_>, input: PointCreate) -> CommandResult<PointCreated> {
    let warnings = match check(cx.doc, &input) {
        Ok(warnings) => warnings,
        Err(stop) => return stop.into(),
    };
    let named = input.clone();
    let (id, uid, revision) = match checks::add(cx.doc, point(settled(cx.doc, input), 0)) {
        Ok(added) => added,
        Err(stop) => return stop.into(),
    };
    // Its name as a text linked to it, in the same step (docs/adr/9314).
    let mut output = PointCreated {
        uid,
        id,
        revision,
        text_uid: None,
        text_id: None,
    };
    if let Some(text) = name_text(cx.doc, &named, EntityId::parse(&output.uid)) {
        match checks::add(cx.doc, text) {
            Ok((text_id, text_uid, revision)) => {
                output.text_id = Some(text_id);
                output.text_uid = Some(text_uid);
                output.revision = revision;
            }
            Err(stop) => return stop.into(),
        }
    }
    CommandResult::Completed { output, warnings }
}

/// Whether a point's name is written as a text that follows it (`nameText`): in a map's and a road's drawing, which
/// have no labels (docs/adr/9314, 9316); a CBS shows its points' names with labels. The Nokta tool, Noktalar and
/// Öznitelikler ask it.
pub fn names_as_texts(doc: &Document) -> bool {
    matches!(
        doc.settings().cad_work(),
        Some(kentos_commands::CadWork::Map | kentos_commands::CadWork::Road)
    )
}

/// The point's name as a text (`nameText`, docs/adr/9314), linked to the point `of` (none in a plan). None without
/// `nameText` or a name.
fn name_text(doc: &Document, input: &PointCreate, of: Option<EntityId>) -> Option<Entity> {
    if !input.name_text {
        return None;
    }
    let name = input.label.as_deref().filter(|l| !l.trim().is_empty())?;
    Some(name_text_at(
        doc,
        &input.layer_id,
        input.color.as_ref(),
        input.p,
        name,
        of,
    ))
}

/// A point's name `name` as a text (docs/adr/9314), linked to the point `of` (none in a plan) without a scale: up and
/// right of the point at `p` by the corners' rule, at the project's measure text height on paper, in its typeface,
/// upright, on the layer `layer_id` in the colour `color`. `cad.point.create`'s `nameText` writes it, and
/// `cad.entities.set`'s for a point named later (docs/adr/9316).
pub(crate) fn name_text_at(
    doc: &Document,
    layer_id: &str,
    color: Option<&String>,
    p: Vec2,
    name: &str,
    of: Option<EntityId>,
) -> Entity {
    let settings = doc.settings();
    let height = settings.annotation_height(AnnotationKind::Measure);
    let font = kentos_geometry_core::text::Font::from_id(DrawingFont::id_of(settings.drawing_font));
    let out = kentos_geometry_core::vec2::Vec2::new(
        std::f64::consts::FRAC_1_SQRT_2,
        std::f64::consts::FRAC_1_SQRT_2,
    );
    let at = kentos_geometry_core::processing::numbering::corner_text_at(
        kentos_geometry_core::vec2::Vec2::new(p.x, p.y),
        out,
        name,
        height,
        font,
    );
    Entity::Text(TextEntity {
        annotative: None,
        base: EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: layer_id.to_owned(),
            color: color.cloned(),
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        p: Vec2 { x: at.x, y: at.y },
        text: name.to_owned(),
        height,
        rotation: 0.0,
        align: None,
        width_factor: None,
        mask: false,
        label_of: of,
        label_scale: None,
        paragraph: Default::default(),
        face: Default::default(),
        path: None,
    })
}

/// The checks in the contract's order; the warnings when the point may be written.
fn check(doc: &Document, input: &PointCreate) -> Result<Vec<CommandWarning>, Stop> {
    checks::point(input.p, "Noktanın", "p")?;
    if let Some(z) = input.z {
        checks::finite(z, "Kot", "Kotu sonlu bir sayıyla verin.", "z")?;
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let warnings = checks::layer(doc, &input.layer_id)?;
    checks::object_attrs(doc, &input.layer_id, input.attrs.as_ref(), "attrs")?;
    Ok(warnings)
}

/// The point `input` describes, as the document stores it; `id` is the
/// document's to give (0 in a plan).
fn point(input: PointCreate, id: u32) -> Entity {
    Entity::Point(PointEntity {
        base: EntityBase {
            look: Default::default(),
            id,
            layer_id: input.layer_id,
            color: input.color,
            attrs: input.attrs.unwrap_or_default(),
            label: input.label,
            symbol: input.symbol,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        p: input.p,
        z: input.z,
        parts: None,
    })
}

/// `input` with the attributes the object is written with: canonical, the fields' defaults filled
/// (`checks::object_attrs`; the checks refused what it refuses).
fn settled(doc: &Document, mut input: PointCreate) -> PointCreate {
    input.attrs = checks::object_attrs(doc, &input.layer_id, input.attrs.as_ref(), "attrs")
        .ok()
        .flatten();
    input
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_domain::Slot;

    /// A named point with `nameText` writes its name up and right of it, at the measure text height at the plot scale
    /// (2 mm at 1:1000: 2 m), linked to it without a scale, in its own step (docs/adr/9314); plan says so, writing
    /// nothing.
    #[test]
    fn a_name_is_written_up_and_right_of_its_point_and_linked_to_it() {
        let cases: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../fixtures/commands/v1/cad.point.create.json"
        ))
        .expect("cases");
        let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&cases["setup"].to_string())
            .expect("a drawing");
        let mut doc = Document::from_snapshot(snapshot).expect("opens");
        let mut input: PointCreate =
            serde_json::from_value(cases["cases"][0]["steps"][0]["input"].clone())
                .expect("an input");
        input.label = Some("P1".to_owned());
        input.name_text = true;
        let CommandResult::Completed { output: plan, .. } =
            super::plan(&ExecutionContext::new(&mut doc), &input)
        else {
            panic!("a plan")
        };
        assert!(
            matches!(&plan.text, Some(Entity::Text(t)) if t.text == "P1" && t.label_of.is_none())
        );
        let CommandResult::Completed { output, .. } =
            execute(&mut ExecutionContext::new(&mut doc), input.clone())
        else {
            panic!("written")
        };
        let text_id = output.text_id.expect("a text");
        let Some(Entity::Text(t)) = doc.get(Slot(text_id)) else {
            panic!("the text")
        };
        assert!(
            t.p.x > input.p.x && t.p.y > input.p.y,
            "up and right: {:?}",
            t.p
        );
        assert_eq!(
            (t.text.as_str(), t.height, t.label_scale),
            ("P1", 2.0, None)
        );
        let point = doc.uid(Slot(output.id)).map(|u| *u.as_bytes());
        assert_eq!(t.label_of.map(|of| of.0), point);
        assert_eq!(
            output.text_uid,
            doc.uid(Slot(text_id)).map(|u| u.to_string())
        );
        assert_eq!(doc.undo().as_deref(), Some("Ekle"), "one step");
        assert!(doc.get(Slot(output.id)).is_none() && doc.get(Slot(text_id)).is_none());
    }
}
