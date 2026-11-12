//! `cad.images.clip` v1 (docs/adr/0192 §5, 0622 §9): Resmi kırp. The picture's boundary is given in the drawing (a
//! rectangle by two corners, a polygon by its corners) or taken away; the shared core cuts it to the picture in the
//! picture's own fractions (`ops::image::clip_of`) and the picture is written as `cad.entities.edit`'s `imageClip`
//! writes it, one undo step “Resmi kırp”. Taking away the clip of a picture with none, or giving the clip it has,
//! writes nothing. The shared cases are `fixtures/commands/v1/cad.images.clip.json`
//! (`scripts/fixtures/image_clip_command_cases.py`).

use kentos_commands::{
    CommandResult, EditOperation, EntitiesEdit, EntitiesEditPlan, Entity, EntityEdit,
    EntityGeometry, ImageClipBy, ImagesClip, ImagesClipped, Vec2 as Wire,
};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::ops::image::clip_of;
use kentos_geometry_core::vec2::Vec2;

use crate::checks::{self, Stop, error, is_uid_text};
use crate::geometry::shape;
use crate::{ExecutionContext, codes, edit};

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

/// The boundary in the drawing, its corners checked; none: the clip taken away.
fn boundary(clip: &ImageClipBy) -> Result<Option<Vec<Vec2>>, Stop> {
    match clip {
        ImageClipBy::Rectangle { a, b } => {
            checks::point(*a, "Dikdörtgenin köşesi", "clip.a")?;
            checks::point(*b, "Dikdörtgenin köşesi", "clip.b")?;
            Ok(Some(vec![
                Vec2::new(a.x, a.y),
                Vec2::new(b.x, a.y),
                Vec2::new(b.x, b.y),
                Vec2::new(a.x, b.y),
            ]))
        }
        ImageClipBy::Polygon { points } => {
            for (i, p) in points.iter().enumerate() {
                checks::point(*p, "Çokgenin köşesi", &format!("clip.points[{i}]"))?;
            }
            if points.len() < 3 {
                return Err(refuse(
                    codes::TOO_FEW_POINTS,
                    format!(
                        "Kırpma çokgeninin en az üç köşesi olmalı; {} köşe verildi.",
                        points.len()
                    ),
                    "clip.points",
                ));
            }
            Ok(Some(points.iter().map(|p| Vec2::new(p.x, p.y)).collect()))
        }
        ImageClipBy::Remove => Ok(None),
    }
}

/// The picture: an image of the drawing, not on a locked layer.
fn picture<'d>(doc: &'d Document, uid: &str) -> Result<&'d Entity, Stop> {
    if !is_uid_text(uid) {
        return Err(refuse(
            codes::INVALID_UID,
            format!(
                "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
            ),
            "uid",
        ));
    }
    let entity = Uuid::parse_str(uid)
        .ok()
        .and_then(|u| doc.slot_of(u))
        .and_then(|s| doc.get(s))
        .ok_or_else(|| {
            refuse(
                codes::ENTITY_NOT_FOUND,
                format!(
                    "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir resmin kimliğini verin."
                ),
                "uid",
            )
        })?;
    if !matches!(entity, Entity::Image(_)) {
        return Err(refuse(
            codes::NOT_AN_IMAGE,
            "Verilen nesne bir resim değil; yalnız resim kırpılır. Bir resmin kimliğini verin.",
            "uid",
        ));
    }
    let layers = doc.layers();
    let layer = &entity.base().layer_id;
    if layers.is_locked(layer) {
        let name = layers
            .get(layer)
            .map_or(layer.as_str(), |n| n.name.as_str());
        return Err(refuse(
            codes::LAYER_LOCKED,
            format!(
                "“{name}” katmanı kilitli; üzerindeki resim kırpılamaz. Kilidi Katmanlar panelinden açın."
            ),
            "uid",
        ));
    }
    Ok(entity)
}

/// The clip in the picture's fractions, and the change that writes it; no change when the picture already shows it.
struct Prepared {
    clip: Option<Vec<Wire>>,
    change: Option<EntityEdit>,
}

fn prepare(doc: &Document, input: &ImagesClip) -> Result<Prepared, Stop> {
    let world = boundary(&input.clip)?;
    let entity = picture(doc, &input.uid)?;
    let Entity::Image(im) = entity else {
        return Err(refuse(
            codes::NOT_AN_IMAGE,
            "Verilen nesne bir resim değil.",
            "uid",
        ));
    };
    let clip = match world {
        Some(ring) => Some(
            clip_of(&shape(entity), &ring)
                .map_err(|why| refuse(codes::CLIP_OUTSIDE, why, "clip"))?
                .into_iter()
                .map(|q| Wire { x: q.x, y: q.y })
                .collect::<Vec<_>>(),
        ),
        None => None,
    };
    checks::revision(doc, input.expected_revision.as_deref())?;
    if clip == im.image.clip {
        return Ok(Prepared { clip, change: None });
    }
    let mut fields = im.image.clone();
    fields.clip = clip.clone();
    Ok(Prepared {
        clip,
        change: Some(EntityEdit::Update {
            uid: input.uid.clone(),
            geometry: EntityGeometry::Image(fields),
        }),
    })
}

fn edit_input(change: EntityEdit, input: &ImagesClip) -> EntitiesEdit {
    EntitiesEdit {
        operation: EditOperation::ImageClip,
        changes: vec![change],
        expected_revision: input.expected_revision.clone(),
    }
}

/// The boundary's checks and the writing command's.
pub fn validate(cx: &ExecutionContext<'_>, input: &ImagesClip) -> CommandResult<()> {
    match prepare(cx.doc, input) {
        Ok(Prepared {
            change: Some(change),
            ..
        }) => edit::validate(cx, &edit_input(change, input)),
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The picture as it would be written.
pub fn plan(cx: &ExecutionContext<'_>, input: &ImagesClip) -> CommandResult<EntitiesEditPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] over the document (the tool's preview).
pub fn plan_in(doc: &Document, input: &ImagesClip) -> CommandResult<EntitiesEditPlan> {
    match prepare(doc, input) {
        Ok(Prepared {
            change: Some(change),
            ..
        }) => edit::plan_in(doc, &edit_input(change, input)),
        Ok(_) => CommandResult::Completed {
            output: EntitiesEditPlan {
                changed: Vec::new(),
                created: Vec::new(),
                removed: Vec::new(),
                revision: doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// Clips the picture, one undo step “Resmi kırp”.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ImagesClip) -> CommandResult<ImagesClipped> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_IMAGES_CLIP,
        kentos_commands::CAD_IMAGES_CLIP_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: ImagesClip) -> CommandResult<ImagesClipped> {
    let Prepared { clip, change } = match prepare(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    let Some(change) = change else {
        return CommandResult::Completed {
            output: ImagesClipped {
                uid: input.uid,
                clip,
                changed: false,
                revision: cx.doc.revision().to_string(),
            },
            warnings: Vec::new(),
        };
    };
    match edit::write_in(cx, edit_input(change, &input)) {
        CommandResult::Completed { output, warnings } => CommandResult::Completed {
            output: ImagesClipped {
                uid: input.uid,
                clip,
                changed: true,
                revision: output.revision,
            },
            warnings,
        },
        other => crate::trim::relay(other),
    }
}
