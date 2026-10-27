//! `cad.revisions.mark` v1 (“Revizyon işaretle”, docs/adr/0721 §4): a revision cloud and its delta in one undo step
//! “Revizyon”. The cloud is `cad.revcloud.create`'s, its attribute `Revizyon` the revision's number (and `Tarih`,
//! `Açıklama` when given); the delta is KentOS's library block “Revizyon” (the drawing's own when it has one, else
//! added in the same step), its `REV` the number, sized on paper at the drawing's scale. Revizyon bulutu writes
//! through it when a revision's number is set; the headless server, Python and MCP run it from the catalog. The
//! shared cases are `fixtures/commands/v1/cad.revisions.mark.json` (`scripts/fixtures/revision_cases.py`).
//!
//! The checks, in order (the first that fails answers): the number, the date and the description (`invalid_mark`);
//! `at` and the size finite; the size above 0 and at most 100 mm (`invalid_scale`); then the cloud's frame and arcs,
//! the expected revision and the layer, as `cad.revcloud.create`'s.

use std::collections::BTreeMap;

use kentos_commands::{
    BlockDefinition, BlockId, CommandResult, CommandWarning, Entity, EntityBase, InsertEntity, MAX_REVISION_DATE,
    MAX_REVISION_DESCRIPTION, MAX_REVISION_NUMBER, REVISION_BLOCK, REVISION_SIZE, REVISION_TAG, RevcloudCreate,
    RevisionMarked, RevisionPlan, RevisionsMark, Vec2 as Wire,
};
use kentos_domain::{Document, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::library_symbols;

/// The undo step's name.
pub const STEP: &str = "Revizyon";
/// The most a delta may be across on paper, millimetres.
pub const MAX_SIZE: f64 = 100.0;

/// What may be written.
struct Checked {
    cloud: crate::polygon::Ruled,
    tag: Entity,
    block: Option<BlockDefinition>,
}

fn invalid(words: &str, path: &str) -> Stop {
    Stop::failed(error(codes::INVALID_MARK, words.into(), Some(path.into())))
}

/// A text the mark writes: no control character, no space at its edges, at most `most` characters.
fn plain(text: &str, most: usize) -> bool {
    text.trim() == text && !text.chars().any(char::is_control) && text.chars().count() <= most
}

/// The cloud `cad.revcloud.create` draws for the mark, its attributes the revision's.
fn cloud_input(input: &RevisionsMark) -> RevcloudCreate {
    let mut attrs = BTreeMap::new();
    attrs.insert("Revizyon".to_owned(), input.number.clone());
    if let Some(d) = &input.date {
        attrs.insert(kentos_commands::REVISION_DATE.to_owned(), d.clone());
    }
    if let Some(d) = &input.description {
        attrs.insert(kentos_commands::REVISION_DESCRIPTION.to_owned(), d.clone());
    }
    RevcloudCreate {
        layer_id: input.layer_id.clone(),
        frame: input.frame.clone(),
        arc: input.arc,
        color: input.color.clone(),
        line_weight: input.line_weight,
        line_type: None,
        attrs: Some(attrs),
        label: None,
        symbol: None,
        rules: None,
        expected_revision: input.expected_revision.clone(),
    }
}

fn check(
    doc: &Document,
    store: Option<&kentos_geometry_core::store::Store>,
    input: &RevisionsMark,
    fresh: impl FnOnce() -> BlockId,
) -> Result<Checked, Stop> {
    if input.number.is_empty() || !plain(&input.number, MAX_REVISION_NUMBER) {
        return Err(invalid(
            "Revizyonun numarası boş olmayan, kenarı boşluksuz, denetim karaktersiz ve en çok 16 karakter olmalı (1, A, B2 gibi).",
            "number",
        ));
    }
    if input.date.as_deref().is_some_and(|d| !plain(d, MAX_REVISION_DATE)) {
        return Err(invalid("Revizyonun tarihi kenarı boşluksuz, denetim karaktersiz ve en çok 32 karakter olmalı.", "date"));
    }
    if input.description.as_deref().is_some_and(|d| !plain(d, MAX_REVISION_DESCRIPTION)) {
        return Err(invalid(
            "Revizyonun açıklaması kenarı boşluksuz, denetim karaktersiz ve en çok 240 karakter olmalı.",
            "description",
        ));
    }
    if let Some(at) = input.at {
        checks::point(at, "Revizyon üçgeninin yerinin", "at")?;
    }
    let size = input.size.unwrap_or(REVISION_SIZE);
    checks::finite(size, "Revizyon üçgeninin boyu", "Boyu kâğıt milimetresi olarak sonlu bir sayıyla verin.", "size")?;
    if !(size > 0.0 && size <= MAX_SIZE) {
        return Err(Stop::failed(error(
            codes::INVALID_SCALE,
            format!(
                "Revizyon üçgeninin boyu sıfırdan büyük, en çok {MAX_SIZE} mm olmalı; {size} verildi. Kâğıttaki boyunu milimetre olarak verin."
            ),
            Some("size".into()),
        )));
    }
    let cloud_in = cloud_input(input);
    let ring = crate::shapes::cloud_ring_in(Some(doc), &cloud_in)?;
    // Off the cloud's top right corner: out by half an arc and half the delta, both ways.
    let (mut east, mut north) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in &ring.0 {
        east = east.max(p.x);
        north = north.max(p.y);
    }
    let cloud = crate::polygon::ruled(doc, store, &crate::shapes::revcloud_polygon(&cloud_in, ring))?;
    let Some(symbol) = library_symbols::symbol(doc, REVISION_BLOCK, fresh) else {
        return Err(Stop::failed(error(
            codes::UNKNOWN_BLOCK,
            "KentOS blok kitaplığında “Revizyon” yok.".into(),
            None,
        )));
    };
    let scale = library_symbols::scale_for(doc, &symbol.def, size);
    let half = size / 1000.0 * doc.settings().plot_scale / 2.0;
    let at = input.at.unwrap_or(Wire {
        x: east + input.arc / 2.0 + half,
        y: north + input.arc / 2.0 + half,
    });
    // The delta's other data: the number is its attribute's value, the date and description plain data.
    let mut attrs = BTreeMap::new();
    attrs.insert(REVISION_TAG.to_owned(), input.number.clone());
    if let Some(d) = &input.date {
        attrs.insert(kentos_commands::REVISION_DATE.to_owned(), d.clone());
    }
    if let Some(d) = &input.description {
        attrs.insert(kentos_commands::REVISION_DESCRIPTION.to_owned(), d.clone());
    }
    let tag = Entity::Insert(InsertEntity {
        clipping: None,
        parameters: Default::default(),
        attribute_places: Default::default(),
        base: EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: input.layer_id.clone(),
            color: input.color.clone(),
            attrs,
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        block: symbol.def.id,
        p: at,
        scale,
        y_scale: None,
        rotation: 0.0,
        mirror: false,
    });
    Ok(Checked {
        cloud,
        tag,
        block: symbol.added.then_some(symbol.def),
    })
}

fn warnings_of(c: &Checked) -> Vec<CommandWarning> {
    c.cloud.0.clone()
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &RevisionsMark) -> CommandResult<()> {
    match check(cx.doc, cx.store, input, || BlockId([0; 16])) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: warnings_of(&c),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &RevisionsMark) -> CommandResult<RevisionPlan> {
    plan_in(cx.doc, cx.store, input)
}

/// [`plan`] over a document (Revizyon işaretle's ghost).
pub fn plan_in(
    doc: &Document,
    store: Option<&kentos_geometry_core::store::Store>,
    input: &RevisionsMark,
) -> CommandResult<RevisionPlan> {
    match check(doc, store, input, || BlockId([0; 16])) {
        Ok(c) => CommandResult::Completed {
            warnings: warnings_of(&c),
            output: RevisionPlan {
                cloud: c.cloud.1,
                tag: c.tag,
                block: c.block,
                revision: doc.revision().to_string(),
            },
        },
        Err(stop) => stop.into(),
    }
}

/// The revision's number the drawing's last delta shows (the one a new mark takes by default); none without one.
pub fn last_number(doc: &Document) -> Option<String> {
    let block = doc.block_named(kentos_commands::REVISION_BLOCK)?.id;
    doc.entities()
        .filter_map(|e| match e {
            Entity::Insert(i) if i.block == block => i.base.attrs.get(REVISION_TAG).cloned(),
            _ => None,
        })
        .last()
}

/// The date and the description the drawing's deltas of revision `number` give (each the first that has one, in
/// the drawing's order): what another mark of the same revision takes by default.
pub fn data_of(doc: &Document, number: &str) -> (Option<String>, Option<String>) {
    let Some(block) = doc.block_named(kentos_commands::REVISION_BLOCK).map(|b| b.id) else {
        return (None, None);
    };
    let deltas = || {
        doc.entities().filter_map(move |e| match e {
            Entity::Insert(i) if i.block == block && i.base.attrs.get(REVISION_TAG).map(String::as_str) == Some(number) => {
                Some(&i.base.attrs)
            }
            _ => None,
        })
    };
    let first = |key: &str| deltas().find_map(|a| a.get(key).filter(|v| !v.is_empty()).cloned());
    (first(kentos_commands::REVISION_DATE), first(kentos_commands::REVISION_DESCRIPTION))
}

/// Checks `input` again and writes the cloud and its delta (the library's block first when the drawing lacks it) as
/// one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: RevisionsMark) -> CommandResult<RevisionMarked> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_REVISIONS_MARK,
        kentos_commands::CAD_REVISIONS_MARK_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: RevisionsMark) -> CommandResult<RevisionMarked> {
    let c = match check(cx.doc, cx.store, &input, || BlockId(*Uuid::now_v7().as_bytes())) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    cx.doc.name_step(STEP);
    let added = c.block.is_some();
    if let Some(def) = c.block
        && let Err(refused) = cx.doc.add_block(def)
    {
        return Stop::failed(error(codes::BLOCK_REFUSED, refused.to_string(), None)).into();
    }
    let Entity::Insert(i) = &c.tag else {
        unreachable!("the delta is an insert");
    };
    let block = i.block;
    let (cloud, cloud_id, warnings) = match crate::polygon::write_ruled(cx, c.cloud) {
        CommandResult::Completed { output, warnings } => (output.uid, output.id, warnings),
        CommandResult::Failed { error } => return CommandResult::Failed { error },
        CommandResult::Conflict { error } => return CommandResult::Conflict { error },
        CommandResult::NeedsInput { error } => return CommandResult::NeedsInput { error },
        CommandResult::Queued { job_id } => return CommandResult::Queued { job_id },
        CommandResult::Cancelled => return CommandResult::Cancelled,
    };
    match checks::add(cx.doc, c.tag) {
        Ok((tag_id, tag, revision)) => CommandResult::Completed {
            output: RevisionMarked {
                cloud,
                cloud_id,
                tag,
                tag_id,
                block,
                added,
                revision,
            },
            warnings,
        },
        Err(stop) => stop.into(),
    }
}
