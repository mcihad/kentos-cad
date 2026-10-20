//! `cad.blocks.insert` v1 (docs/adr/0315 §4; plan/cad.md CAD-BLOCK-04): a block placed, named by its id or its
//! name, as one undo step (“Ekle”). Blok ekle writes through it; the headless server, Python and MCP run it from
//! the catalog. The shared cases are `fixtures/commands/v1/cad.blocks.insert.json`
//! (`scripts/fixtures/block_insert_cases.py`).
//!
//! The insert's factors are the input's times the unit's: a unitless block takes the drawing's unit (its numbers
//! are drawing units, a 1 a millimetre in a millimetre drawing); a real-size block keeps its size, and one drawn in
//! another unit than the drawing's is said so (`insert_units_differ`).
//!
//! The checks, in order (the first that fails answers):
//! 1. a block named (`no_block`); `at` finite, then the turn; the factors positive and finite (`invalid_scale`);
//! 2. the expected revision;
//! 3. the block is the drawing's (`unknown_block`); every value's tag is one of its attributes (`unknown_tag`);
//! 4. the layer: every create command's (`checks.rs`).

use std::collections::BTreeMap;

use kentos_commands::blocks::name_key;
use kentos_commands::units::Scale;
use kentos_commands::{
    BlockDefinition, BlockInserted, BlockUnits, BlocksInsert, BlocksInsertPlan, CommandResult,
    CommandWarning, Entity, EntityBase, InsertEntity,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &BlocksInsert) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &BlocksInsert) -> CommandResult<BlocksInsertPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] over a document (Blok ekle's ghost and its report).
pub fn plan_in(doc: &Document, input: &BlocksInsert) -> CommandResult<BlocksInsertPlan> {
    match check(doc, input) {
        Ok(c) => CommandResult::Completed {
            output: BlocksInsertPlan {
                insert: c.insert,
                unit_factor: c.factor,
                revision: doc.revision().to_string(),
            },
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the insert as one undo step (“Ekle”), into the open transaction or group, if
/// one is.
pub fn execute(cx: &mut ExecutionContext<'_>, input: BlocksInsert) -> CommandResult<BlockInserted> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_BLOCKS_INSERT,
        kentos_commands::CAD_BLOCKS_INSERT_VERSION,
        input,
        write_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step of the document, all or nothing, and journals it
/// (docs/adr/0253 §1, §3).
fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: BlocksInsert,
) -> CommandResult<BlockInserted> {
    let c = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let Entity::Insert(i) = &c.insert else {
        unreachable!("the plan is an insert");
    };
    let block = i.block;
    match checks::add(cx.doc, c.insert) {
        Ok((id, uid, revision)) => CommandResult::Completed {
            output: BlockInserted {
                uid,
                id,
                block,
                unit_factor: c.factor,
                revision,
            },
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What may be written: the insert (`id` 0), the unit's factor and the warnings.
struct Checked {
    insert: Entity,
    factor: f64,
    warnings: Vec<CommandWarning>,
}

fn invalid_scale(what: &str, path: &str) -> Stop {
    Stop::failed(error(
        codes::INVALID_SCALE,
        format!("{what} sıfırdan büyük, sonlu bir sayı olmalı. Pozitif bir ölçek verin."),
        Some(path.into()),
    ))
}

/// The block an input names: by id, else by name (Turkish case folded).
pub fn named_block<'d>(doc: &'d Document, input: &BlocksInsert) -> Option<&'d BlockDefinition> {
    match (input.block, &input.name) {
        (Some(id), _) => doc.block(id),
        (None, Some(name)) => {
            let key = name_key(name);
            doc.blocks()
                .iter()
                .find(|b| name_key(&b.name) == key)
                .map(|b| &**b)
        }
        (None, None) => None,
    }
}

/// The unit's factor of an insert of `block` in `doc` (docs/adr/0315 §4): a unitless block's numbers are the
/// drawing's units, a real-size block's metres.
pub fn unit_factor(doc: &Document, block: &BlockDefinition) -> f64 {
    match block.units {
        Some(BlockUnits::Unitless) => {
            Scale::from_unit(crate::units_change::unit_of(doc)).apply(1.0)
        }
        _ => 1.0,
    }
}

/// The checks in the contract's order.
fn check(doc: &Document, input: &BlocksInsert) -> Result<Checked, Stop> {
    if input.block.is_none() && input.name.as_deref().is_none_or(checks::is_blank) {
        return Err(Stop::failed(error(
            codes::NO_BLOCK,
            "Yerleştirilecek blok verilmedi. Bloğu kimliğiyle (block) ya da adıyla (name) verin.".into(),
            Some("block".into()),
        )));
    }
    checks::point(input.at, "Yerleştirme noktasının", "at")?;
    let rotation = input.rotation.unwrap_or(0.0);
    checks::finite(
        rotation,
        "Dönüş",
        "Dönüşü radyan olarak sonlu bir sayıyla verin.",
        "rotation",
    )?;
    let scale = input.scale.unwrap_or(1.0);
    if !kentos_commands::blocks::scale_ok(scale) {
        return Err(invalid_scale("Ölçek", "scale"));
    }
    if let Some(y) = input.y_scale
        && !kentos_commands::blocks::scale_ok(y)
    {
        return Err(invalid_scale("Y ölçeği", "yScale"));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let Some(block) = named_block(doc, input) else {
        let what = match (&input.block, &input.name) {
            (Some(id), _) => format!("“{}” kimlikli", id.to_text()),
            (None, Some(name)) => format!("“{name}” adlı"),
            (None, None) => String::new(),
        };
        return Err(Stop::failed(error(
            codes::UNKNOWN_BLOCK,
            format!("{what} blok çizimde tanımlı değil. Bloklar panelindeki bir bloğun adını ya da kimliğini verin."),
            Some(if input.block.is_some() { "block" } else { "name" }.into()),
        )));
    };
    let values: BTreeMap<String, String> = input.attributes.clone().unwrap_or_default();
    if let Some(tag) = values
        .keys()
        .find(|t| !block.attributes.iter().any(|a| &a.tag == *t))
    {
        return Err(Stop::failed(error(
            codes::UNKNOWN_TAG,
            format!(
                "“{}” bloğunda “{tag}” etiketli öznitelik yok. Bloğun etiketlerinden birini verin.",
                block.name
            ),
            Some(format!("attributes.{tag}")),
        )));
    }
    // A constant attribute's value is its definition's (docs/adr/0317 §1).
    if let Some(tag) = values.keys().find(|t| {
        block
            .attributes
            .iter()
            .any(|a| &a.tag == *t && a.options.constant)
    }) {
        return Err(Stop::failed(error(
            codes::CONSTANT_ATTRIBUTE,
            format!(
                "“{}” bloğunun “{tag}” özniteliği sabit: değeri tanımındakidir, yerleştirmede verilmez.",
                block.name
            ),
            Some(format!("attributes.{tag}")),
        )));
    }
    let fields = crate::blocks_sync::fields_of(doc, &block.attributes, &|_| {
        if input.block.is_some() { "block" } else { "name" }.to_owned()
    })?;
    let mut warnings = checks::layer(doc, &input.layer_id)?;
    let factor = unit_factor(doc, block);
    let drawing = crate::units_change::unit_of(doc);
    if let Some(u) = block.units.and_then(BlockUnits::unit)
        && u != drawing
    {
        warnings.push(CommandWarning {
            code: codes::INSERT_UNITS_DIFFER.into(),
            message: format!(
                "“{}” bloğu {} biriminde çizilmiş, çizim {}; blok gerçek boyutuyla yerleşti.",
                block.name,
                u.mark(),
                drawing.mark()
            ),
            path: Some("block".into()),
        });
    }
    let sx = scale * factor;
    let sy = input.y_scale.map(|y| y * factor).filter(|y| *y != sx);
    // A dynamic block's values given, resolved as `cad.blocks.parameters` does (docs/adr/0318 §3).
    let parameters = match (&input.parameters, &block.dynamic) {
        (None, _) => Default::default(),
        (Some(given), Some(d)) => crate::block_params::resolve(d, &Default::default(), given).map_err(
            |(code, path, words)| {
                Stop::failed(error(code, words, Some(path.replacen("values", "parameters", 1))))
            },
        )?,
        (Some(_), None) => {
            return Err(Stop::failed(error(
                codes::NOT_DYNAMIC,
                format!("“{}” bloğunun parametresi yok; dinamik değil. parameters'ı vermeyin.", block.name),
                Some("parameters".into()),
            )));
        }
    };
    let insert = Entity::Insert(InsertEntity {
        clipping: None,
        parameters,
        attribute_places: Default::default(),
        base: EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: input.layer_id.clone(),
            color: input.color.clone(),
            // Its other data, the attribute values over them.
            attrs: input
                .attrs
                .clone()
                .unwrap_or_default()
                .into_iter()
                .chain(values)
                .collect(),
            label: input.label.clone().filter(|l| !l.is_empty()),
            symbol: input.symbol.clone(),
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        block: block.id,
        p: input.at,
        scale: sx,
        y_scale: sy,
        rotation,
        mirror: input.mirror == Some(true),
    });
    // Its fields filled with its values and place, those not given (docs/adr/0317 §1).
    let mut insert = insert;
    if !fields.is_empty() {
        let shapes = [crate::geometry::shape(&insert)];
        let layer_name = |id: &str| {
            doc.layers()
                .get(id)
                .map_or_else(|| id.to_owned(), |n| n.name.clone())
        };
        let mut filled = Vec::new();
        for (tag, template) in &fields {
            if input.attributes.as_ref().is_some_and(|v| v.contains_key(tag)) {
                continue;
            }
            let text = template
                .texts(&[&insert], &layer_name, &|k| shapes.get(k), &|_| None)
                .pop()
                .unwrap_or_default();
            filled.push((tag.clone(), text));
        }
        if let Entity::Insert(i) = &mut insert {
            i.base.attrs.extend(filled);
        }
    }
    Ok(Checked {
        insert,
        factor,
        warnings,
    })
}
