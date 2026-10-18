//! `cad.tags.create` and `cad.tags.check` v1 (docs/adr/0344 §1, §2, §4; plan/cad.md `CAD-ARCH-20`, `39`): tags put on
//! objects, one undo step “Etiket”, and the tags and numbers checked, read-only. The Etiket tool previews the plan's
//! tags and writes through `execute`; Python and an agent give the same objects and get the same tags. The shared cases
//! are `fixtures/commands/v1/cad.tags.{create,check}.json`.
//!
//! `cad.tags.create`'s checks, in order (the first that fails answers): 1. at least one tag (`no_objects`), at most
//! `MAX_TAGS` (`invalid_tag`); 2. each tag in the input's order: its object's id (`invalid_uid`), its place
//! (`not_finite`), its object (`entity_not_found`, `not_taggable`), its fields (`invalid_tag`); 3. `all`: its kinds,
//! layers (`layer_not_found`) and fields (`invalid_tag`), and at least one object it takes when no tag is given
//! (`no_objects`); 4. the line weight; 5. the expected revision; 6. the layer.
//!
//! A tag's words and the point it ties to are written as the document writes them; its place, when not given, is its
//! object's point for a space, else 4 times its words' height above it.

use std::collections::BTreeSet;

use kentos_contracts::{
    CommandResult, CommandWarning, Entity, EntityBase, NewTag, TagEntity, TagFields, TagFinding,
    TagShape, TaggedKind, TagsCheck, TagsChecked, TagsCreate, TagsCreated, TagsEdit, TagsEdited,
    TagsPlan, Vec2,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

pub const STEP: &str = "Etiket";
pub const EDIT_STEP: &str = "Etiketi düzenle";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

type Checked = (Vec<CommandWarning>, Vec<Entity>);

/// The objects the drawing's tags tag, by their persistent ids.
fn tagged(doc: &Document) -> BTreeSet<String> {
    doc.entities()
        .filter_map(|e| match e {
            Entity::Tag(t) => Some(t.tag.of.clone()),
            _ => None,
        })
        .collect()
}

/// A tag's words' height on paper at the project's plot scale, metres.
fn words_height(doc: &Document, size: Option<f64>) -> f64 {
    size.unwrap_or(kentos_contracts::TAG_SIZE_MM) * doc.settings().plot_scale / 1000.0
}

/// A tag of `target` from what was given, its words and point written as the document writes them.
fn tag_of(doc: &Document, of: &str, target: &Entity, n: &NewTag) -> Option<TagFields> {
    let anchor = doc.tag_anchor(target);
    let at = match (n.at, anchor) {
        (Some(at), _) => at,
        (None, Some(a)) if target.kind() == "space" => a,
        (None, Some(a)) => Vec2 {
            x: a.x,
            y: a.y + 4.0 * words_height(doc, n.size),
        },
        (None, None) => return None,
    };
    Some(TagFields {
        of: of.to_owned(),
        at,
        content: n.content.clone(),
        shape: n
            .shape
            .unwrap_or_else(|| kentos_contracts::default_shape(target.kind())),
        size: n.size,
        leader: n.leader.unwrap_or(false),
        text: doc.tag_text(target, n.content.as_deref()),
        anchor,
        broken: None,
    })
}

fn invalid(words: String, path: &str) -> Stop {
    fail(codes::INVALID_TAG, words, path)
}

fn check(doc: &Document, input: &TagsCreate) -> Result<Checked, Stop> {
    if input.tags.is_empty() && input.all.is_none() {
        return Err(fail(
            codes::NO_OBJECTS,
            "Etiketlenecek nesne verilmedi. Etiketler (tags) ya da türler (all) verin.".into(),
            "tags",
        ));
    }
    if input.tags.len() > kentos_contracts::MAX_TAGS {
        return Err(invalid(
            format!(
                "Bir çağrıda en çok {} etiket yazılır; {} verildi.",
                kentos_contracts::MAX_TAGS,
                input.tags.len()
            ),
            "tags",
        ));
    }
    let mut made: Vec<TagFields> = Vec::new();
    let mut done: BTreeSet<String> = tagged(doc);
    for (i, n) in input.tags.iter().enumerate() {
        let at = format!("tags[{i}]");
        if !checks::is_uid_text(&n.of) {
            return Err(fail(
                codes::INVALID_UID,
                format!(
                    "“{}” geçerli bir nesne kimliği değil; küçük harfli, tireli bir UUID verin.",
                    n.of
                ),
                &format!("{at}.of"),
            ));
        }
        if let Some(p) = n.at {
            checks::point(p, "Etiketin yerinin", &format!("{at}.at"))?;
        }
        if let Some(s) = n.size {
            checks::finite(
                s,
                "Yazı yüksekliği",
                "Sonlu bir sayı verin.",
                &format!("{at}.size"),
            )?;
        }
        let Some(target) = doc.tag_target(&n.of) else {
            return Err(fail(
                codes::ENTITY_NOT_FOUND,
                format!(
                    "“{}” kimlikli nesne çizimde yok. Var olan bir nesnenin kimliğini verin.",
                    n.of
                ),
                &format!("{at}.of"),
            ));
        };
        if TaggedKind::of_kind(target.kind()).is_none() {
            return Err(fail(
                codes::NOT_TAGGABLE,
                "Bu nesne etiketlenmez: açıklık, mahal, kolon, aks, duvar, merdiven, rampa ya da blok etiketlenir.".into(),
                &format!("{at}.of"),
            ));
        }
        let Some(t) = tag_of(doc, &n.of, &target, n) else {
            return Err(invalid(
                "Nesnenin bağ noktası bulunamadı; etiketin yerini (at) verin.".into(),
                &format!("{at}.at"),
            ));
        };
        if let Some((field, words)) = t.problem() {
            return Err(invalid(words, &format!("{at}.{field}")));
        }
        done.insert(n.of.clone());
        made.push(t);
    }
    if let Some(all) = &input.all {
        if all.kinds.is_empty() {
            return Err(fail(
                codes::NO_OBJECTS,
                "Etiketlenecek tür verilmedi; en az bir tür verin.".into(),
                "all.kinds",
            ));
        }
        for (k, l) in all.layers.iter().flatten().enumerate() {
            if doc.layers().get(l).is_none() {
                return Err(fail(
                    codes::LAYER_NOT_FOUND,
                    format!(
                        "“{l}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
                    ),
                    &format!("all.layers[{k}]"),
                ));
            }
        }
        let probe = TagFields {
            of: "-".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
            content: all.content.clone(),
            shape: all.shape.unwrap_or_default(),
            size: all.size,
            leader: false,
            text: String::new(),
            anchor: None,
            broken: None,
        };
        if let Some((field, words)) = probe.problem() {
            return Err(invalid(words, &format!("all.{field}")));
        }
        let kinds: Vec<&str> = all.kinds.iter().map(|k| k.kind()).collect();
        let n = NewTag {
            of: String::new(),
            at: None,
            content: all.content.clone(),
            shape: all.shape,
            size: all.size,
            leader: None,
        };
        let mut found = 0usize;
        for e in doc.entities() {
            let layer = &e.base().layer_id;
            if !kinds.contains(&e.kind())
                || !doc.layers().is_visible(layer)
                || all.layers.as_ref().is_some_and(|ls| !ls.contains(layer))
            {
                continue;
            }
            let Some(uid) = doc.uid(Slot(e.base().id)).map(|u| u.to_string()) else {
                continue;
            };
            if done.contains(&uid) {
                continue;
            }
            if let Some(t) = tag_of(doc, &uid, e, &n) {
                done.insert(uid);
                made.push(t);
                found += 1;
            }
        }
        if found == 0 && input.tags.is_empty() {
            return Err(fail(
                codes::NO_OBJECTS,
                "Verilen türlerde etiketsiz ve görünen nesne yok.".into(),
                "all.kinds",
            ));
        }
        if made.len() > kentos_contracts::MAX_TAGS {
            return Err(invalid(
                format!(
                    "Bir çağrıda en çok {} etiket yazılır.",
                    kentos_contracts::MAX_TAGS
                ),
                "all",
            ));
        }
    }
    checks::line_weight(input.line_weight, "lineWeight")?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let warnings = checks::layer(doc, &input.layer_id)?;
    let entities = made
        .into_iter()
        .map(|tag| {
            Entity::Tag(TagEntity {
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
                tag,
            })
        })
        .collect();
    Ok((warnings, entities))
}

pub mod create {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &TagsCreate) -> CommandResult<()> {
        match check(cx.doc, input) {
            Ok((warnings, _)) => CommandResult::Completed {
                output: (),
                warnings,
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(cx: &ExecutionContext<'_>, input: &TagsCreate) -> CommandResult<TagsPlan> {
        plan_in(cx.doc, input)
    }

    /// [`plan`] on a document: what the Etiket tool draws as its preview.
    pub fn plan_in(doc: &Document, input: &TagsCreate) -> CommandResult<TagsPlan> {
        match check(doc, input) {
            Ok((warnings, tags)) => CommandResult::Completed {
                output: TagsPlan {
                    tags,
                    revision: doc.revision().to_string(),
                },
                warnings,
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn execute(cx: &mut ExecutionContext<'_>, input: TagsCreate) -> CommandResult<TagsCreated> {
        crate::call::recorded(
            cx,
            kentos_contracts::CAD_TAGS_CREATE,
            kentos_contracts::CAD_TAGS_CREATE_VERSION,
            input,
            write_execute,
        )
    }

    fn write_execute(
        cx: &mut ExecutionContext<'_>,
        input: TagsCreate,
    ) -> CommandResult<TagsCreated> {
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
            output: TagsCreated {
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

/// `cad.tags.edit`'s checks, in its order; the tags as they would be.
fn edit_check(doc: &Document, input: &TagsEdit) -> Result<Vec<(Slot, Entity)>, Stop> {
    checks::uids(&input.uids, "Düzenlenecek etiket verilmedi.")?;
    let found = checks::named(doc, &input.uids)?;
    for (at, _, e, _) in &found {
        if !matches!(e, Entity::Tag(_)) {
            return Err(fail(
                codes::NOT_A_TAG,
                "Nesne etiket değil; Etiketi düzenle yalnız etiketleri değiştirir.".into(),
                &format!("uids[{at}]"),
            ));
        }
    }
    let c = &input.change;
    if c.content.is_none() && c.shape.is_none() && c.size.is_none() && c.leader.is_none() {
        return Err(fail(
            codes::NOTHING_TO_SET,
            "Değiştirilecek bir alan verilmedi.".into(),
            "change",
        ));
    }
    if let Some(Some(s)) = c.size {
        checks::finite(s, "Yazı yüksekliği", "Sonlu bir sayı verin.", "change.size")?;
    }
    let mut out = Vec::with_capacity(found.len());
    for (at, slot, e, _) in &found {
        let Entity::Tag(t) = e else { continue };
        let mut t = t.clone();
        if let Some(v) = &c.content {
            t.tag.content = v.clone();
        }
        if let Some(v) = c.shape {
            t.tag.shape = v;
        }
        if let Some(v) = c.size {
            t.tag.size = v;
        }
        if let Some(v) = c.leader {
            t.tag.leader = v;
        }
        if let Some((field, words)) = t.tag.problem() {
            let path = if field == "content" || field == "size" {
                format!("change.{field}")
            } else {
                format!("uids[{at}]")
            };
            return Err(invalid(words, &path));
        }
        t.tag = doc.tag_settled(&t.tag, false);
        out.push((*slot, Entity::Tag(t)));
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
                    "“{name}” katmanı kilitli; üzerindeki etiket düzenlenemez. Kilidi Katmanlar panelinden açın."
                ),
                &format!("uids[{at}]"),
            ));
        }
    }
    Ok(out)
}

pub mod edit {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &TagsEdit) -> CommandResult<()> {
        match edit_check(cx.doc, input) {
            Ok(_) => CommandResult::Completed {
                output: (),
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(cx: &ExecutionContext<'_>, input: &TagsEdit) -> CommandResult<TagsEdited> {
        plan_in(cx.doc, input)
    }

    /// [`plan`] on a document: what Öznitelikler previews.
    pub fn plan_in(doc: &Document, input: &TagsEdit) -> CommandResult<TagsEdited> {
        match edit_check(doc, input) {
            Ok(work) => CommandResult::Completed {
                output: TagsEdited {
                    tags: work.into_iter().map(|(_, e)| e).collect(),
                    revision: doc.revision().to_string(),
                },
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn execute(cx: &mut ExecutionContext<'_>, input: TagsEdit) -> CommandResult<TagsEdited> {
        crate::call::recorded(
            cx,
            kentos_contracts::CAD_TAGS_EDIT,
            kentos_contracts::CAD_TAGS_EDIT_VERSION,
            input,
            write_execute,
        )
    }

    fn write_execute(cx: &mut ExecutionContext<'_>, input: TagsEdit) -> CommandResult<TagsEdited> {
        let work = match edit_check(cx.doc, &input) {
            Ok(work) => work,
            Err(stop) => return stop.into(),
        };
        let tags = work.iter().map(|(_, e)| e.clone()).collect();
        cx.doc.update_many(work, EDIT_STEP);
        CommandResult::Completed {
            output: TagsEdited {
                tags,
                revision: cx.doc.revision().to_string(),
            },
            warnings: Vec::new(),
        }
    }
}

/// What `cad.tags.check` finds (docs/adr/0344 §2).
fn findings(doc: &Document, input: &TagsCheck) -> Result<Vec<TagFinding>, Stop> {
    for (k, l) in input.layers.iter().flatten().enumerate() {
        if doc.layers().get(l).is_none() {
            return Err(fail(
                codes::LAYER_NOT_FOUND,
                format!(
                    "“{l}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
                ),
                &format!("layers[{k}]"),
            ));
        }
    }
    let takes = |e: &Entity| {
        input
            .layers
            .as_ref()
            .is_none_or(|ls| ls.contains(&e.base().layer_id))
    };
    let uid = |e: &Entity| {
        doc.uid(Slot(e.base().id))
            .map(|u| u.to_string())
            .unwrap_or_default()
    };
    let point = |e: &Entity| match e {
        Entity::Tag(t) => t.tag.at,
        _ => doc.tag_anchor(e).unwrap_or(Vec2 { x: 0.0, y: 0.0 }),
    };
    let tags = tagged(doc);
    let mut duplicates: Vec<(String, Vec<&Entity>)> = Vec::new();
    let (mut no_type, mut off, mut gone, mut untagged) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for e in doc.entities() {
        if !takes(e) {
            continue;
        }
        let number = match e {
            Entity::Opening(o) => o.opening.number.as_ref().map(|n| ("opening", n)),
            Entity::Space(s) => s.space.number.as_ref().map(|n| ("space", n)),
            _ => None,
        };
        if let Some((kind, n)) = number.filter(|(_, n)| !n.is_empty()) {
            let key = format!("{kind}\u{0}{n}");
            match duplicates.iter_mut().find(|(k, _)| *k == key) {
                Some((_, list)) => list.push(e),
                None => duplicates.push((key, vec![e])),
            }
        }
        match e {
            Entity::Opening(o) => {
                if o.opening.style.as_deref().is_none_or(str::is_empty) {
                    no_type.push(e);
                }
                if o.opening.broken.is_some() {
                    off.push(e);
                }
            }
            Entity::Tag(t) if doc.tag_target(&t.tag.of).is_none() => gone.push(e),
            _ => {}
        }
        if matches!(e, Entity::Opening(_) | Entity::Space(_)) && !tags.contains(&uid(e)) {
            untagged.push(e);
        }
    }
    let mut out = Vec::new();
    let mut push = |code: &str, message: String, list: Vec<&Entity>| {
        out.push(TagFinding {
            code: code.into(),
            message,
            uids: list.iter().map(|e| uid(e)).collect(),
            at: list.first().map_or(Vec2 { x: 0.0, y: 0.0 }, |e| point(e)),
        });
    };
    for (key, list) in duplicates {
        if list.len() > 1 {
            let (kind, n) = key.split_once('\u{0}').unwrap_or(("", ""));
            let what = if kind == "opening" {
                "açıklıkta"
            } else {
                "mahalde"
            };
            push(
                codes::TAG_DUPLICATE_NUMBER,
                format!("“{n}” numarası {} {what}.", list.len()),
                list,
            );
        }
    }
    for e in no_type {
        let n = match e {
            Entity::Opening(o) => o.opening.number.clone().unwrap_or_default(),
            _ => String::new(),
        };
        let words = if n.is_empty() {
            "Numarasız açıklığın türü yok.".to_owned()
        } else {
            format!("“{n}” açıklığının türü yok.")
        };
        push(codes::TAG_MISSING_TYPE, words, vec![e]);
    }
    for e in off {
        let why = match e {
            Entity::Opening(o) => o.opening.broken.map_or("", |b| b.words()),
            _ => "",
        };
        push(codes::TAG_HOST_BROKEN, why.to_owned(), vec![e]);
    }
    for e in gone {
        let words = match e {
            Entity::Tag(t) if !t.tag.text.is_empty() => {
                format!(
                    "“{}” etiketinin nesnesi çizimde yok.",
                    t.tag.text.replace('\n', " ")
                )
            }
            _ => "Etiketin nesnesi çizimde yok.".to_owned(),
        };
        push(codes::TAG_TARGET_GONE, words, vec![e]);
    }
    for e in untagged {
        let (what, n) = match e {
            Entity::Opening(o) => ("açıklık", o.opening.number.clone()),
            Entity::Space(s) => ("mahal", s.space.number.clone()),
            _ => ("", None),
        };
        let words = match n.filter(|n| !n.is_empty()) {
            Some(n) => format!("“{n}” {what} etiketsiz."),
            None => format!("Numarasız {what} etiketsiz."),
        };
        push(codes::TAG_UNTAGGED, words, vec![e]);
    }
    Ok(out)
}

pub mod check_tags {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &TagsCheck) -> CommandResult<()> {
        match findings(cx.doc, input) {
            Ok(_) => CommandResult::Completed {
                output: (),
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(cx: &ExecutionContext<'_>, input: &TagsCheck) -> CommandResult<TagsChecked> {
        plan_in(cx.doc, input)
    }

    pub fn plan_in(doc: &Document, input: &TagsCheck) -> CommandResult<TagsChecked> {
        match findings(doc, input) {
            Ok(findings) => CommandResult::Completed {
                output: TagsChecked {
                    findings,
                    revision: doc.revision().to_string(),
                },
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    /// Read-only: the same answer as the plan.
    pub fn execute(cx: &mut ExecutionContext<'_>, input: TagsCheck) -> CommandResult<TagsChecked> {
        plan_in(cx.doc, &input)
    }
}

/// The frame a new tag of `kind` takes when the tool names none.
pub fn shape_for(kind: &str) -> TagShape {
    kentos_contracts::default_shape(kind)
}
