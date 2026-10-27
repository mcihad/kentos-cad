//! `cad.legends.materials` v1 (“Malzeme lejandı”, docs/adr/0721 §5): a legend of the materials the walls are drawn
//! with, as the block “Malzeme lejandı”: a title, then for each material a swatch hatched as the walls hatch it and
//! its name beside, at the drawing's scale. Made again in its place when the drawing has it (every insert follows);
//! `at` places one more. One undo step “Malzeme lejandı”. Malzeme lejandı writes through it; the headless server,
//! Python and MCP run it from the catalog. The shared cases are `fixtures/commands/v1/cad.legends.materials.json`
//! (`scripts/fixtures/legend_cases.py`).
//!
//! A row is a wall's component (walls in the drawing's order, each one's components from its left face) whose name
//! (its material, else its own name) or hatch differs from every row before; its count is the walls that have it.
//! The layout, in the text's height `h` on paper times the drawing's scale (`H = h·N/1000` metres): the title's top
//! left at the base point; row `i`'s swatch `6H` wide and `3H` high with its top `2H + 4H·i` below the base point, its
//! name `H` high, its left middle `7H` right of the base point, level with the swatch's middle.
//!
//! The checks, in order (the first that fails answers): `at` and the height finite; the height above 0 and at most
//! 50 mm (`invalid_height`); the expected revision; a material (`no_materials`); `at` or a legend to make again
//! (`no_legend`); with `at`, the layer, as every create command's.

use kentos_commands::{
    BlockDefinition, BlockId, CommandResult, CommandWarning, Entity, LEGEND_BLOCK, LegendMade, LegendMaterial,
    LegendPlan, LegendsMaterials,
};
use kentos_domain::{Document, Uuid};
use serde_json::json;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

/// The undo step's name.
pub const STEP: &str = "Malzeme lejandı";
/// The legend's title.
pub const TITLE: &str = "Malzeme lejandı";
/// A legend's text height on paper when none is given, and the most, millimetres.
pub const HEIGHT: f64 = 2.5;
pub const MAX_HEIGHT: f64 = 50.0;

/// One row: its name, its component's hatch, how many walls have it.
struct Row {
    name: String,
    hatch: Option<kentos_commands::ComponentHatch>,
    walls: u32,
}

/// The rows of the drawing's walls, in their order.
fn rows(doc: &Document) -> Vec<Row> {
    let mut out: Vec<Row> = Vec::new();
    for e in doc.entities() {
        let Entity::Wall(w) = e else {
            continue;
        };
        let mut seen: Vec<usize> = Vec::new();
        for c in &w.wall.components {
            let name = c
                .material
                .as_deref()
                .filter(|m| !m.trim().is_empty())
                .unwrap_or(&c.name)
                .to_owned();
            let at = match out.iter().position(|r| r.name == name && r.hatch == c.hatch) {
                Some(at) => at,
                None => {
                    out.push(Row {
                        name,
                        hatch: c.hatch.clone(),
                        walls: 0,
                    });
                    out.len() - 1
                }
            };
            if !seen.contains(&at) {
                seen.push(at);
                out[at].walls += 1;
            }
        }
    }
    out
}

/// The block's objects: the title, then each row's swatch, its hatch and its name.
fn entities(doc: &Document, rows: &[Row], h: f64) -> Result<Vec<Entity>, Stop> {
    let plot = doc.settings().plot_scale;
    let big = h * plot / 1000.0;
    let mut out = Vec::new();
    let push = |mut v: serde_json::Value, out: &mut Vec<Entity>| -> Result<(), Stop> {
        v["id"] = json!(out.len() + 1);
        v["layerId"] = json!("");
        v["attrs"] = json!({});
        let e = serde_json::from_value(v)
            .map_err(|e| Stop::failed(error(codes::BLOCK_REFUSED, format!("Lejandın nesnesi yazılamadı: {e}"), None)))?;
        out.push(e);
        Ok(())
    };
    let p = |x: f64, y: f64| json!({ "x": x, "y": y });
    push(
        json!({ "kind": "text", "p": p(0.0, 0.0), "text": TITLE, "height": big, "rotation": 0.0, "align": "topLeft" }),
        &mut out,
    )?;
    for (i, r) in rows.iter().enumerate() {
        let top = -(2.0 * big + 4.0 * big * i as f64);
        let bottom = top - 3.0 * big;
        let ring = json!([p(0.0, bottom), p(6.0 * big, bottom), p(6.0 * big, top), p(0.0, top)]);
        push(json!({ "kind": "polyline", "pts": ring, "closed": true }), &mut out)?;
        let pattern = r.hatch.as_ref().and_then(|c| {
            kentos_geometry_core::geom::wall::component_hatch(&c.pattern, c.scale, c.angle, plot)
                .and_then(crate::geometry::contract_pattern)
                .map(|pattern| (pattern, c.color.clone()))
        });
        if let Some((pattern, color)) = pattern {
            let mut v = json!({ "kind": "hatch", "ring": ring, "pattern": pattern });
            if let Some(c) = color {
                v["color"] = json!(c);
            }
            push(v, &mut out)?;
        }
        push(
            json!({ "kind": "text", "p": p(7.0 * big, top - 1.5 * big), "text": r.name, "height": big, "rotation": 0.0,
                    "align": "middleLeft" }),
            &mut out,
        )?;
    }
    Ok(out)
}

/// What may be written.
struct Checked {
    materials: Vec<LegendMaterial>,
    block: BlockDefinition,
    remade: bool,
    insert: Option<Entity>,
    warnings: Vec<CommandWarning>,
}

fn check(doc: &Document, input: &LegendsMaterials, fresh: impl FnOnce() -> BlockId) -> Result<Checked, Stop> {
    if let Some(at) = input.at {
        checks::point(at, "Lejandın yerinin", "at")?;
    }
    let h = input.height.unwrap_or(HEIGHT);
    checks::finite(h, "Lejandın yazı yüksekliği", "Yüksekliği kâğıt milimetresi olarak sonlu bir sayıyla verin.", "height")?;
    if !(h > 0.0 && h <= MAX_HEIGHT) {
        return Err(Stop::failed(error(
            codes::INVALID_HEIGHT,
            format!("Lejandın yazı yüksekliği sıfırdan büyük, en çok {MAX_HEIGHT} mm olmalı; {h} verildi."),
            Some("height".into()),
        )));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let rows = rows(doc);
    if rows.is_empty() {
        return Err(Stop::failed(error(
            codes::NO_MATERIALS,
            "Çizimde bileşenli duvar yok: lejanda yazılacak malzeme yok. Duvarlara bileşenli bir duvar stili verin.".into(),
            None,
        )));
    }
    let had = doc.block_named(LEGEND_BLOCK);
    if input.at.is_none() && had.is_none() {
        return Err(Stop::failed(error(
            codes::NO_LEGEND,
            "Çizimde malzeme lejandı yok: yeniden yapılacak bir şey yok. Lejandın yerini (at) verin.".into(),
            Some("at".into()),
        )));
    }
    let warnings = match input.at {
        Some(_) => checks::layer(doc, &input.layer_id)?,
        None => Vec::new(),
    };
    let block = BlockDefinition {
        id: had.map_or_else(fresh, |b| b.id),
        name: had.map_or_else(|| LEGEND_BLOCK.to_owned(), |b| b.name.clone()),
        base: kentos_commands::Vec2 { x: 0.0, y: 0.0 },
        entities: entities(doc, &rows, h)?,
        attributes: Vec::new(),
        description: Some("Duvarların malzemeleri: her birinin taraması ve adı (docs/adr/0721).".into()),
        units: None,
        card: None,
        dynamic: None,
        xref: None,
    };
    let insert = match input.at {
        Some(at) => Some(
            serde_json::from_value::<Entity>(json!({
                "kind": "insert", "id": 0, "layerId": input.layer_id, "attrs": {}, "block": block.id,
                "p": at, "scale": 1.0, "rotation": 0.0,
            }))
            .map_err(|e| Stop::failed(error(codes::BLOCK_REFUSED, format!("Lejandın yerleştirmesi yazılamadı: {e}"), None)))?,
        ),
        None => None,
    };
    Ok(Checked {
        materials: rows
            .iter()
            .map(|r| LegendMaterial {
                name: r.name.clone(),
                pattern: r.hatch.as_ref().map(|c| c.pattern.clone()),
                walls: r.walls,
            })
            .collect(),
        block,
        remade: had.is_some(),
        insert,
        warnings,
    })
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &LegendsMaterials) -> CommandResult<()> {
    match check(cx.doc, input, || BlockId([0; 16])) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &LegendsMaterials) -> CommandResult<LegendPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] over a document (Malzeme lejandı's ghost).
pub fn plan_in(doc: &Document, input: &LegendsMaterials) -> CommandResult<LegendPlan> {
    match check(doc, input, || BlockId([0; 16])) {
        Ok(c) => CommandResult::Completed {
            output: LegendPlan {
                materials: c.materials,
                block: c.block,
                remade: c.remade,
                insert: c.insert,
                revision: doc.revision().to_string(),
            },
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the legend's block (made again or added) and the new insert as one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: LegendsMaterials) -> CommandResult<LegendMade> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_LEGENDS_MATERIALS,
        kentos_commands::CAD_LEGENDS_MATERIALS_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: LegendsMaterials) -> CommandResult<LegendMade> {
    let c = match check(cx.doc, &input, || BlockId(*Uuid::now_v7().as_bytes())) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    cx.doc.name_step(STEP);
    let block = c.block.id;
    // Made again only when it differs: the same legend writes nothing of its block.
    let written = if c.remade {
        if cx.doc.block(block).is_some_and(|b| *b == c.block) {
            Ok(())
        } else {
            cx.doc.update_block(c.block).map(|_| ())
        }
    } else {
        cx.doc.add_block(c.block)
    };
    if let Err(refused) = written {
        return Stop::failed(error(codes::BLOCK_REFUSED, refused.to_string(), None)).into();
    }
    let (uid, id) = match c.insert {
        Some(insert) => match checks::add(cx.doc, insert) {
            Ok((id, uid, _)) => (Some(uid), Some(id)),
            Err(stop) => return stop.into(),
        },
        None => (None, None),
    };
    CommandResult::Completed {
        output: LegendMade {
            materials: c.materials,
            block,
            remade: c.remade,
            uid,
            id,
            revision: cx.doc.revision().to_string(),
        },
        warnings: c.warnings,
    }
}
