//! `cad.sections.edit` v1 (docs/adr/8042 §5): a section mark's section drawn into its block and placed (`derive`), or
//! drawn again from the model as it is now (`refresh`). Each is one undo step, “Kesit çıkar” or “Kesiti yeniden çiz”;
//! the plan is the tool's preview. Patlat turns a placement into the drawing's own objects. The layers the block's objects name (EK-4) are opened in the same step when the drawing
//! has none.
//!
//! Checks, in order: the mark (its id, a section mark, two points), its block (none for derive, one for the others),
//! the origin, the expected revision, a locked layer.

use std::collections::BTreeMap;

use kentos_commands::{
    BlockDefinition, BuildingSectionLeft, BuildingSectionLeftOut, BuildingSectionsEdit,
    BuildingSectionsEdited, BuildingSectionsOperation, CommandResult, Entity, SECTION_DERIVE_STEP,
    SECTION_REFRESH_STEP, Vec2, ViewMarkForm,
};
use kentos_domain::{Document, NewLayer, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::sections::{self, Drawn, Left};

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// The mark: its id, its fields; refused when it is not a two-point section mark.
pub(crate) fn mark_of(
    doc: &Document,
    uid: &str,
    path: &str,
) -> Result<(Uuid, kentos_commands::ViewMarkFields), Stop> {
    if !checks::is_uid_text(uid) {
        return Err(fail(
            codes::INVALID_UID,
            format!(
                "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir."
            ),
            path,
        ));
    }
    let u = Uuid::parse_str(uid).map_err(|_| {
        fail(
            codes::INVALID_UID,
            format!("“{uid}” geçerli bir nesne kimliği değil."),
            path,
        )
    })?;
    let Some(e) = doc.slot_of(u).and_then(|s| doc.get(s)) else {
        return Err(fail(
            codes::ENTITY_NOT_FOUND,
            format!(
                "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir."
            ),
            path,
        ));
    };
    let Entity::ViewMark(v) = e else {
        return Err(fail(
            codes::NOT_A_SECTION_MARK,
            "Nesne kesit işareti değil; kesit, planda çizilmiş bir kesit işaretinden çıkarılır."
                .into(),
            path,
        ));
    };
    if v.mark.form != ViewMarkForm::Section {
        return Err(fail(
            codes::NOT_A_SECTION_MARK,
            "İşaret görünüş ya da detay işareti; kesit yalnız kesit işaretinden çıkarılır.".into(),
            path,
        ));
    }
    if v.mark.points.len() != 2 {
        return Err(fail(
            codes::SECTION_JOGGED,
            format!(
                "Kesit çizgisinin {} noktası var; kırıklı kesit henüz çıkarılmıyor. İki noktalı bir kesit çizgisi çizin.",
                v.mark.points.len()
            ),
            path,
        ));
    }
    Ok((u, v.mark.clone()))
}

/// The layers' ids the block's objects need: an EK-4 name's layer as the drawing has it (by its id, the name in lower
/// case, or by its name), else a new one; and whether each is new.
fn layer_ids(
    doc: &Document,
    names: &std::collections::BTreeSet<String>,
) -> BTreeMap<String, (String, bool)> {
    let layers = doc.layers();
    names
        .iter()
        .map(|name| {
            let id = name.to_ascii_lowercase();
            let found = if layers.get(&id).is_some() {
                Some(id.clone())
            } else {
                layers
                    .leaves()
                    .into_iter()
                    .find(|l| l.name.eq_ignore_ascii_case(name))
                    .map(|l| l.id.clone())
            };
            match found {
                Some(f) => (name.clone(), (f, false)),
                None => (name.clone(), (id, true)),
            }
        })
        .collect()
}

/// The block's objects on their layers' ids.
fn placed(drawn: &Drawn, ids: &BTreeMap<String, (String, bool)>) -> Vec<Entity> {
    drawn
        .entities
        .iter()
        .cloned()
        .map(|mut e| {
            let name = e.base().layer_id.clone();
            if let Some((id, _)) = ids.get(&name) {
                e.base_mut().layer_id = id.clone();
            }
            e
        })
        .collect()
}

fn left_out(drawn: &Drawn) -> Vec<BuildingSectionLeftOut> {
    drawn
        .left
        .iter()
        .map(|(why, uid)| BuildingSectionLeftOut {
            why: match why {
                Left::Roof => BuildingSectionLeft::Roof,
                Left::NaturalGround => BuildingSectionLeft::NaturalGround,
                Left::Curved => BuildingSectionLeft::Curved,
                Left::Ramp => BuildingSectionLeft::Ramp,
                Left::NoHeight => BuildingSectionLeft::NoHeight,
                Left::HeadUnknown => BuildingSectionLeft::HeadUnknown,
                Left::StairWaist => BuildingSectionLeft::StairWaist,
                Left::Recess => BuildingSectionLeft::Recess,
            },
            uid: uid.clone(),
        })
        .collect()
}

/// The block a section is drawn into.
pub(crate) fn block_of(mark: Uuid, words: &str, entities: Vec<Entity>) -> BlockDefinition {
    BlockDefinition {
        id: sections::block_id(mark),
        name: sections::block_name(words),
        base: Vec2 { x: 0.0, y: 0.0 },
        entities,
        attributes: Vec::new(),
        description: Some(format!(
            "{} kesiti: KentOS kesit işaretinden çıkarır; plan değişince eskir, Yeniden çiz ile güncellenir.",
            sections::label(words)
        )),
        units: Some(kentos_commands::BlockUnits::M),
        card: None,
        dynamic: None,
        xref: None,
    }
}

/// A block's objects as compared: their ids aside (a drawing written again is the same drawing).
pub(crate) fn bare(entities: &[Entity]) -> Vec<Entity> {
    entities
        .iter()
        .cloned()
        .map(|mut e| {
            e.base_mut().id = 0;
            e
        })
        .collect()
}

enum Checked {
    Derive {
        mark: Uuid,
        words: String,
        drawn: Drawn,
        ids: BTreeMap<String, (String, bool)>,
        at: Vec2,
    },
    Refresh {
        mark: Uuid,
        words: String,
        drawn: Drawn,
        ids: BTreeMap<String, (String, bool)>,
        changed: bool,
    },
}

/// Where a section goes when no origin is given: 5 m right of the drawing's objects, at their bottom.
fn beside(doc: &Document) -> Vec2 {
    match crate::sheets_fit::extent_of(doc, doc.entities()) {
        Some([_, min_y, max_x, _]) => Vec2 {
            x: max_x + 5.0,
            y: min_y,
        },
        None => Vec2 { x: 0.0, y: 0.0 },
    }
}

fn check(doc: &Document, input: &BuildingSectionsEdit) -> Result<Checked, Stop> {
    let path = "operation.mark";
    match &input.operation {
        BuildingSectionsOperation::Derive { mark, at } => {
            let (u, m) = mark_of(doc, mark, path)?;
            if doc.block(sections::block_id(u)).is_some() {
                return Err(fail(
                    codes::SECTION_EXISTS,
                    format!(
                        "{} kesiti zaten çıkarılmış; güncellemek için Yeniden çiz'i kullanın.",
                        sections::label(&m.mark)
                    ),
                    path,
                ));
            }
            if let Some(p) = at {
                checks::point(*p, "Kesitin yerinin", "operation.at")?;
            }
            let frame = sections::frame_of(&m).ok_or_else(|| {
                fail(
                    codes::SECTION_JOGGED,
                    "Kesit çizgisi bir milimetreden kısa; çizilemez.".into(),
                    path,
                )
            })?;
            let drawn = sections::draw(doc, &frame, &m.mark);
            let ids = layer_ids(doc, &drawn.layers);
            checks::revision(doc, input.expected_revision.as_deref())?;
            Ok(Checked::Derive {
                mark: u,
                words: m.mark.clone(),
                at: at.unwrap_or_else(|| beside(doc)),
                drawn,
                ids,
            })
        }
        BuildingSectionsOperation::Refresh { mark } => {
            let (u, m) = mark_of(doc, mark, path)?;
            let Some(block) = doc.block(sections::block_id(u)) else {
                return Err(fail(
                    codes::SECTION_NOT_FOUND,
                    format!(
                        "{} kesiti çıkarılmamış; önce Kesit çıkar.",
                        sections::label(&m.mark)
                    ),
                    path,
                ));
            };
            let frame = sections::frame_of(&m).ok_or_else(|| {
                fail(
                    codes::SECTION_JOGGED,
                    "Kesit çizgisi bir milimetreden kısa; çizilemez.".into(),
                    path,
                )
            })?;
            let drawn = sections::draw(doc, &frame, &m.mark);
            let ids = layer_ids(doc, &drawn.layers);
            let changed = bare(&block.entities) != bare(&placed(&drawn, &ids));
            checks::revision(doc, input.expected_revision.as_deref())?;
            if !changed {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    format!(
                        "{} kesiti güncel: model çizdiğinden beri değişmedi.",
                        sections::label(&m.mark)
                    ),
                    path,
                ));
            }
            Ok(Checked::Refresh {
                mark: u,
                words: m.mark.clone(),
                drawn,
                ids,
                changed,
            })
        }
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &BuildingSectionsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &BuildingSectionsEdit,
) -> CommandResult<BuildingSectionsEdited> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document: what the Kesit çıkar tool draws as its preview.
pub fn plan_in(
    doc: &Document,
    input: &BuildingSectionsEdit,
) -> CommandResult<BuildingSectionsEdited> {
    let c = match check(doc, input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let revision = doc.revision().to_string();
    let output = match c {
        Checked::Derive {
            mark,
            words,
            drawn,
            ids,
            ..
        } => BuildingSectionsEdited {
            mark: mark.to_string(),
            name: format!("{} kesiti", sections::label(&words)),
            block: sections::block_id(mark).to_string(),
            inserts: Vec::new(),
            drawn: placed(&drawn, &ids),
            changed: true,
            left_out: left_out(&drawn),
            revision,
        },
        Checked::Refresh {
            mark,
            words,
            drawn,
            ids,
            changed,
        } => BuildingSectionsEdited {
            mark: mark.to_string(),
            name: format!("{} kesiti", sections::label(&words)),
            block: sections::block_id(mark).to_string(),
            inserts: inserts_of(doc, mark),
            drawn: placed(&drawn, &ids),
            changed,
            left_out: left_out(&drawn),
            revision,
        },
    };
    CommandResult::Completed {
        output,
        warnings: Vec::new(),
    }
}

/// The persistent ids of the mark's section's placements.
pub fn inserts_of(doc: &Document, mark: Uuid) -> Vec<String> {
    let id = sections::block_id(mark);
    doc.entities_with_uids()
        .filter(|(_, e)| matches!(e, Entity::Insert(i) if i.block == id))
        .map(|(u, _)| u.to_string())
        .collect()
}

pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: BuildingSectionsEdit,
) -> CommandResult<BuildingSectionsEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_SECTIONS_EDIT,
        kentos_commands::CAD_SECTIONS_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn refused(
    doc: &mut Document,
    group: kentos_domain::Group,
    why: String,
) -> CommandResult<BuildingSectionsEdited> {
    doc.cancel_group(group);
    Stop::failed(error(codes::LAYER_REFUSED, why, Some("operation".into()))).into()
}

/// Opens the layers the block's objects need that the drawing has none of.
fn open_layers(doc: &mut Document, ids: &BTreeMap<String, (String, bool)>) -> Result<(), String> {
    for (name, (id, new)) in ids {
        if *new {
            let mut l = NewLayer::layer(name.clone());
            l.id = Some(id.clone());
            doc.add_layer(l, None, false).map_err(|r| r.to_string())?;
        }
    }
    Ok(())
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: BuildingSectionsEdit,
) -> CommandResult<BuildingSectionsEdited> {
    let c = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    match c {
        Checked::Derive {
            mark,
            words,
            drawn,
            ids,
            at,
        } => {
            let group = doc.begin_group(SECTION_DERIVE_STEP);
            if let Err(why) = open_layers(doc, &ids) {
                return refused(doc, group, why);
            }
            let block = block_of(mark, &words, placed(&drawn, &ids));
            if let Err(r) = doc.add_block(block) {
                return refused(doc, group, r.to_string());
            }
            // Its placement on a layer of its own (EK-4's form, the general element: MM-XXXX-KESIT), so a set's plan and
            // site maps leave the sections drawn beside the building out (docs/adr/8042 §6).
            let mut one = BTreeMap::new();
            one.insert(
                "MM-XXXX-KESIT".to_owned(),
                layer_ids(doc, &std::iter::once("MM-XXXX-KESIT".to_owned()).collect())
                    .remove("MM-XXXX-KESIT")
                    .unwrap_or(("mm-xxxx-kesit".into(), true)),
            );
            if let Err(why) = open_layers(doc, &one) {
                return refused(doc, group, why);
            }
            let layer = one["MM-XXXX-KESIT"].0.clone();
            let insert: Entity = match serde_json::from_value(serde_json::json!({
                "kind": "insert", "id": 0, "layerId": layer, "attrs": {},
                "block": sections::block_id(mark).to_string(), "p": {"x": at.x, "y": at.y}, "scale": 1.0, "rotation": 0.0
            })) {
                Ok(e) => e,
                Err(why) => return refused(doc, group, why.to_string()),
            };
            let slots = match doc.add_many(vec![insert], SECTION_DERIVE_STEP) {
                Ok(s) => s,
                Err(full) => {
                    doc.cancel_group(group);
                    return Stop::failed(error(codes::SLOTS_EXHAUSTED, full.to_string(), None))
                        .into();
                }
            };
            doc.end_group(group);
            let inserts: Vec<String> = slots
                .iter()
                .filter_map(|s| doc.uid(*s))
                .map(|u| u.to_string())
                .collect();
            CommandResult::Completed {
                output: BuildingSectionsEdited {
                    mark: mark.to_string(),
                    name: format!("{} kesiti", sections::label(&words)),
                    block: sections::block_id(mark).to_string(),
                    inserts,
                    drawn: doc
                        .block(sections::block_id(mark))
                        .map(|b| b.entities.clone())
                        .unwrap_or_default(),
                    changed: true,
                    left_out: left_out(&drawn),
                    revision: doc.revision().to_string(),
                },
                warnings: Vec::new(),
            }
        }
        Checked::Refresh {
            mark,
            words,
            drawn,
            ids,
            changed,
        } => {
            let group = doc.begin_group(SECTION_REFRESH_STEP);
            if let Err(why) = open_layers(doc, &ids) {
                return refused(doc, group, why);
            }
            let block = block_of(mark, &words, placed(&drawn, &ids));
            if let Err(r) = doc.update_block(block) {
                return refused(doc, group, r.to_string());
            }
            doc.end_group(group);
            CommandResult::Completed {
                output: BuildingSectionsEdited {
                    mark: mark.to_string(),
                    name: format!("{} kesiti", sections::label(&words)),
                    block: sections::block_id(mark).to_string(),
                    inserts: inserts_of(doc, mark),
                    drawn: doc
                        .block(sections::block_id(mark))
                        .map(|b| b.entities.clone())
                        .unwrap_or_default(),
                    changed,
                    left_out: left_out(&drawn),
                    revision: doc.revision().to_string(),
                },
                warnings: Vec::new(),
            }
        }
    }
}
