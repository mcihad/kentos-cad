//! `cad.annotations.write`'s Etiketleri yazıya çevir (docs/adr/0175, 0212 §4, 0622 §9): the asked objects' labels
//! written as texts where the label engine places them at 1:N, as a sheet writes them (`Store::label_texts`), their
//! callouts as lines. The store is the host's when it has one (the desktop's keeps the labels in step), else one made
//! with what the engine reads (`labels_report::label_store`). The shared cases are
//! `scripts/fixtures/labels_to_text_cases.py`'s.

use kentos_commands::{
    Entity, EntityGeometry, LabelsMode, TextAlign, TextFace, TextPath, Vec2 as Wire,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::ops::label_text::LabelText;
use kentos_geometry_core::store::Store;
use kentos_geometry_core::vec2::Vec2;

use crate::checks::{self, Stop, error, is_uid_text};
use crate::codes;

const LABEL: &str = "Etiketleri yazıya çevir";

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

fn wire(p: Vec2) -> Wire {
    Wire { x: p.x, y: p.y }
}

/// What the labels become: the objects (each text with its object and the scale when linked), the labels written
/// and those left (with no free place, out of their scale range, too small).
pub(crate) struct Written {
    pub objects: Vec<(EntityGeometry, Option<(String, f64)>)>,
    pub count: usize,
    pub skipped: usize,
}

/// Whether the object's label may be written: not a text, a dimension or a leader, on a visible layer that labels,
/// without a text that writes its label already (docs/adr/0175 §4).
fn labelled(doc: &Document, e: &Entity) -> bool {
    let base = e.base();
    let layers = doc.layers();
    let off = layers
        .get(&base.layer_id)
        .and_then(|l| l.style.labels.as_ref())
        .is_some_and(|l| l.mode == LabelsMode::Off);
    !(matches!(
        e,
        Entity::Text(_) | Entity::Dimension(_) | Entity::Leader(_)
    ) || off
        || !layers.is_visible(&base.layer_id)
        || doc.has_linked_text(Slot(base.id)))
}

/// The objects whose labels are written, by slot: the ones `ids` names, else every object of the drawing.
fn wanted(doc: &Document, ids: Option<&[String]>) -> Result<Vec<f64>, Stop> {
    let mut out = Vec::new();
    match ids {
        None => out.extend(
            doc.entities()
                .filter(|e| labelled(doc, e))
                .map(|e| f64::from(e.base().id)),
        ),
        Some(ids) => {
            for (i, uid) in ids.iter().enumerate() {
                let path = format!("work.ids[{i}]");
                if !is_uid_text(uid) {
                    return Err(refuse(
                        codes::INVALID_UID,
                        format!(
                            "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
                        ),
                        &path,
                    ));
                }
                let e = Uuid::parse_str(uid)
                    .ok()
                    .and_then(|u| doc.slot_of(u))
                    .and_then(|s| doc.get(s))
                    .ok_or_else(|| {
                        refuse(
                            codes::ENTITY_NOT_FOUND,
                            format!(
                                "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
                            ),
                            &path,
                        )
                    })?;
                let id = f64::from(e.base().id);
                if labelled(doc, e) && !out.contains(&id) {
                    out.push(id);
                }
            }
        }
    }
    if out.is_empty() {
        let place = if ids.is_some() {
            "verilen nesnelerde"
        } else {
            "çizimde"
        };
        return Err(refuse(
            codes::NOTHING_DERIVED,
            format!("{LABEL}: {place} yazıya çevrilecek etiket yok."),
            "work.ids",
        ));
    }
    Ok(out)
}

/// A label's text's face: its class's weight and slant (its colour stays the layer's).
fn face_of(doc: &Document, wanted: &[f64], t: &LabelText) -> TextFace {
    let style = wanted
        .get(t.item)
        .and_then(|&id| doc.get(Slot(id as u32)))
        .and_then(|e| {
            let node = doc.layers().get(&e.base().layer_id)?;
            match node
                .style
                .labels
                .as_ref()
                .filter(|l| l.mode == LabelsMode::Rules)
            {
                Some(l) => l.classes.get(t.class as usize).map(|c| c.style.clone()),
                None => node.style.label.clone(),
            }
        });
    TextFace {
        bold: style
            .as_ref()
            .is_some_and(|s| s.weight.unwrap_or(500) >= 600),
        italic: style.as_ref().is_some_and(|s| s.italic == Some(true)),
        ..TextFace::default()
    }
}

/// The asked labels as objects at 1:`scale` (the project's when none); `every` writes the labels with no free place
/// too, `mask` masks the texts, `linked` ties each to its object.
#[allow(clippy::too_many_arguments)]
pub(crate) fn labels(
    doc: &Document,
    store: Option<&Store>,
    ids: Option<&[String]>,
    scale: Option<f64>,
    every: bool,
    mask: bool,
    linked: bool,
) -> Result<Written, Stop> {
    let scale = match scale {
        Some(s) => {
            checks::finite(s, "Ölçek", "Sonlu bir sayı verin.", "work.scale")?;
            if !(s > 0.0 && s <= 1_000_000.0) {
                return Err(refuse(
                    codes::INVALID_SCALE,
                    "Ölçek 1:N'nin N'si, sıfırdan büyük olmalı.",
                    "work.scale",
                ));
            }
            s
        }
        None => doc.settings().plot_scale,
    };
    let wanted = wanted(doc, ids)?;
    let own;
    let store = match store {
        Some(s) => s,
        None => {
            own = crate::labels_report::label_store(doc, None).0;
            &own
        }
    };
    let found = store.label_texts(&wanted, scale, every);
    if found.texts.is_empty() {
        return Err(refuse(
            codes::NOTHING_DERIVED,
            format!("{LABEL}: 1:{scale} ölçekte yazı olacak etiket yok; başka bir ölçek verin."),
            "work.scale",
        ));
    }
    let mut objects: Vec<(EntityGeometry, Option<(String, f64)>)> = found
        .texts
        .iter()
        .map(|t| {
            let link = linked
                .then(|| {
                    let slot = Slot(*wanted.get(t.item)? as u32);
                    Some((doc.uid(slot)?.to_string(), scale))
                })
                .flatten();
            let geometry = EntityGeometry::Text {
                layout: None,
                annotative: None,
                p: wire(t.p),
                text: t.text.clone(),
                height: t.height,
                rotation: t.rotation,
                align: TextAlign::ALL
                    .into_iter()
                    .find(|c| c.name() == t.align.name()),
                width_factor: None,
                mask,
                box_width: None,
                line_spacing: t.line_spacing,
                runs: Vec::new(),
                face: face_of(doc, &wanted, t),
                path: t.path.as_ref().map(|pts| TextPath {
                    pts: pts.iter().map(|p| wire(*p)).collect(),
                    bulges: None,
                }),
            };
            (geometry, link)
        })
        .collect();
    objects.extend(found.callouts.iter().map(|c| {
        (
            EntityGeometry::Line {
                ms: None,
                a: wire(c.from),
                b: wire(c.to),
                zs: None,
            },
            None,
        )
    }));
    Ok(Written {
        objects,
        count: found.texts.len(),
        skipped: found.overlapping + found.out_of_scale + found.small,
    })
}
