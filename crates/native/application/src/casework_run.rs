//! `cad.casework.run` v1 (“Mutfak dizisi”, docs/adr/0706): a straight run of kitchen base cabinets along a line, its
//! sink and cooktop modules, a countertop over the line and, if asked, wall cabinets above, as one undo step. The tool
//! previews the plan and writes through `execute`; Python and an agent give the same line and values. The shared cases
//! are `fixtures/commands/v1/cad.casework.run.json` (`scripts/fixtures/casework_cases.py`).
//!
//! The checks, in order (the first that fails answers): 1. the numbers (`not_finite`: `from`, `to`, `module`,
//! `depth`); 2. the run (`invalid_casework`): a line with a length, a depth from 0.30 to 1.20 m, the blocks (the
//! drawing's by name, else KentOS's library's) with their width parameter, the module's and the sink module's widths
//! among the cabinet's, at least one module, the sink's and the cooktop's modules in the run and apart; 3. the line
//! weight; 4. the expected revision; 5. the layer.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use kentos_commands::block_dynamic::{BlockParameter, ParamValue, same_param_value, value_problem};
use kentos_commands::{
    BlockDefinition, BlockId, CASEWORK_SINK_MODULE, CaseworkBuilt, CaseworkModule,
    CaseworkModuleKind, CaseworkPlan, CaseworkRun, CaseworkSide, CommandResult, CommandWarning,
    DEFAULT_CASEWORK_DEPTH, DEFAULT_CASEWORK_MODULE, Entity, EntityBase, InsertEntity, PathEntity,
    Vec2,
};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::geom::casework::{Frame, module_count, module_spans};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

/// The undo step's name.
pub const STEP: &str = "Mutfak dizisi";
const BASE: &str = "Alt dolap";
const SINK: &str = "Evye";
const COOKTOP: &str = "Ocak";
const UPPER: &str = "Üst dolap";
/// The cabinets' width parameter.
const WIDTH: &str = "Genişlik";
/// The sink's back and ends in from its module's; the cooktop's back from the line (docs/adr/0706 §1).
const SINK_INSET: f64 = 0.05;
const COOKTOP_INSET: f64 = 0.04;
/// The countertop's depth's range, metres.
const DEPTHS: (f64, f64) = (0.30, 1.20);
/// A filler narrower than this is no filler: the run's modules fill its line.
const NO_FILLER: f64 = 1e-3;

/// KentOS's block library, read once.
fn library() -> &'static [BlockDefinition] {
    static LIB: OnceLock<Vec<BlockDefinition>> = OnceLock::new();
    LIB.get_or_init(|| {
        let text = crate::blocks_library::BUILTIN_LIBRARIES[0].1;
        serde_json::from_str::<serde_json::Value>(text)
            .ok()
            .and_then(|file| file.get("blocks").cloned())
            .and_then(|blocks| serde_json::from_value(blocks).ok())
            .unwrap_or_default()
    })
}

struct Checked {
    entities: Vec<Entity>,
    added: Vec<BlockDefinition>,
    modules: Vec<CaseworkModule>,
    filler: f64,
    warnings: Vec<CommandWarning>,
}

fn invalid(words: String, path: &str) -> Stop {
    Stop::failed(error(codes::INVALID_CASEWORK, words, Some(path.into())))
}

/// The definition named `name`: the drawing's, else the library's, which the run adds (its id kept unless taken).
fn definition(
    doc: &Document,
    name: &str,
    added: &mut Vec<BlockDefinition>,
    fresh: &mut impl FnMut() -> BlockId,
) -> Result<BlockDefinition, Stop> {
    if let Some(b) = doc.block_named(name) {
        return Ok(b.clone());
    }
    if let Some(b) = added.iter().find(|b| b.name == name) {
        return Ok(b.clone());
    }
    let Some(mut b) = library().iter().find(|b| b.name == name).cloned() else {
        return Err(invalid(
            format!("“{name}” bloğu ne çizimde ne KentOS kitaplığında var."),
            "layerId",
        ));
    };
    if doc.block(b.id).is_some() {
        b.id = fresh();
    }
    added.push(b.clone());
    Ok(b)
}

/// The width parameter of a cabinet block, else why the run cannot use it.
fn width_of<'b>(b: &'b BlockDefinition, path: &str) -> Result<&'b BlockParameter, Stop> {
    b.dynamic
        .as_ref()
        .and_then(|d| {
            d.parameters
                .iter()
                .find(|p| matches!(p, BlockParameter::Linear { name, .. } if name == WIDTH))
        })
        .ok_or_else(|| {
            invalid(
                format!(
                    "“{}” bloğunun “{WIDTH}” parametresi yok; dizi modülün enini bu parametreyle verir.",
                    b.name
                ),
                path,
            )
        })
}

/// A cabinet's parameters for `width`: none when it is the block's own.
fn width_values(p: &BlockParameter, width: f64) -> BTreeMap<String, ParamValue> {
    let v = ParamValue::Number(width);
    match p.default_value() {
        Some(d) if same_param_value(&d, &v) => BTreeMap::new(),
        _ => BTreeMap::from([(WIDTH.to_owned(), v)]),
    }
}

fn insert(
    b: &BlockDefinition,
    base: &EntityBase,
    p: Vec2,
    frame: &Frame,
    mirror: bool,
    parameters: BTreeMap<String, ParamValue>,
) -> Entity {
    Entity::Insert(InsertEntity {
        base: base.clone(),
        block: b.id,
        p,
        scale: 1.0,
        y_scale: None,
        rotation: frame.rotation(),
        mirror,
        attribute_places: BTreeMap::new(),
        parameters,
        clipping: None,
    })
}

fn rectangle(base: &EntityBase, pts: Vec<Vec2>, area: bool) -> Entity {
    let path = PathEntity {
        base: base.clone(),
        pts,
        bulges: None,
        holes: None,
        zs: None,
        ms: None,
        parts: None,
        closed: !area,
        widths: None,
        space_separator: None,
    };
    if area {
        Entity::Polygon(path)
    } else {
        Entity::Polyline(path)
    }
}

fn wire(p: kentos_geometry_core::vec2::Vec2) -> Vec2 {
    Vec2 { x: p.x, y: p.y }
}

/// The counter-clockwise rectangle `a`..`b` along the line, `0`..`depth` into its side.
fn band(frame: &Frame, a: f64, b: f64, depth: f64, left: bool) -> Vec<Vec2> {
    let pts = [
        frame.at(a, 0.0),
        frame.at(b, 0.0),
        frame.at(b, depth),
        frame.at(a, depth),
    ];
    let mut out: Vec<Vec2> = pts.into_iter().map(wire).collect();
    if !left {
        out.reverse();
    }
    out
}

fn check(
    doc: &Document,
    input: &CaseworkRun,
    mut fresh: impl FnMut() -> BlockId,
) -> Result<Checked, Stop> {
    checks::point(input.from, "Dizinin başının", "from")?;
    checks::point(input.to, "Dizinin sonunun", "to")?;
    for (v, what, path) in [
        (input.module, "Modül eni", "module"),
        (input.depth, "Tezgâh derinliği", "depth"),
    ] {
        if let Some(x) = v {
            checks::finite(x, what, "Sonlu bir sayı verin.", path)?;
        }
    }
    let left = input.side.unwrap_or_default() == CaseworkSide::Left;
    let from = kentos_geometry_core::vec2::Vec2::new(input.from.x, input.from.y);
    let to = kentos_geometry_core::vec2::Vec2::new(input.to.x, input.to.y);
    let Some(frame) = Frame::of(from, to, left) else {
        return Err(invalid(
            "Dizinin başı ile sonu aynı nokta; dolapların sırtının dayandığı çizgiyi verin.".into(),
            "to",
        ));
    };
    let module = input.module.unwrap_or(DEFAULT_CASEWORK_MODULE);
    let depth = input.depth.unwrap_or(DEFAULT_CASEWORK_DEPTH);
    if !(DEPTHS.0 - 1e-9..=DEPTHS.1 + 1e-9).contains(&depth) {
        return Err(invalid(
            format!("Tezgâh derinliği {depth} m; 0,30 ile 1,20 m arasında olmalı."),
            "depth",
        ));
    }
    let mut added = Vec::new();
    let cabinet = definition(doc, BASE, &mut added, &mut fresh)?;
    let width = width_of(&cabinet, "module")?;
    if let Some(why) = value_problem(width, &ParamValue::Number(module)) {
        return Err(invalid(
            format!("Modül eni {module} m; “{BASE}” bloğunun eni için {why}."),
            "module",
        ));
    }
    let sink_width = input.sink.map(|_| CASEWORK_SINK_MODULE);
    if let Some(w) = sink_width
        && let Some(why) = value_problem(width, &ParamValue::Number(w))
    {
        return Err(invalid(
            format!("Evye modülü {w} m; “{BASE}” bloğunun eni için {why}."),
            "sink",
        ));
    }
    let count = module_count(frame.length, module, sink_width);
    if count == 0 {
        return Err(invalid(
            format!(
                "Dizinin çizgisi {} m; bir modüle ({} m) yetmiyor. Çizgiyi uzatın ya da modülü daraltın.",
                kentos_geometry_core::display::fixed(frame.length, 3),
                kentos_geometry_core::display::fixed(sink_width.unwrap_or(module), 2)
            ),
            "to",
        ));
    }
    for (v, path, whose) in [(input.sink, "sink", "Evye"), (input.cooktop, "cooktop", "Ocak")] {
        if let Some(k) = v
            && !(1..=count).contains(&(k as usize))
        {
            return Err(invalid(
                format!("{whose} {k}. modülde; dizi {count} modül (1 ile {count} arası)."),
                path,
            ));
        }
    }
    if input.sink.is_some() && input.sink == input.cooktop {
        return Err(invalid(
            "Evye ve ocak aynı modülde olamaz; ocağı başka bir modüle koyun.".into(),
            "cooktop",
        ));
    }
    let sink_block = match input.sink {
        Some(_) => Some(definition(doc, SINK, &mut added, &mut fresh)?),
        None => None,
    };
    let cooktop_block = match input.cooktop {
        Some(_) => Some(definition(doc, COOKTOP, &mut added, &mut fresh)?),
        None => None,
    };
    let upper = match input.upper {
        Some(true) => {
            let b = definition(doc, UPPER, &mut added, &mut fresh)?;
            let p = width_of(&b, "upper")?.clone();
            for w in [Some(module), sink_width].into_iter().flatten() {
                if let Some(why) = value_problem(&p, &ParamValue::Number(w)) {
                    return Err(invalid(
                        format!("Üst dolap {w} m; “{UPPER}” bloğunun eni için {why}."),
                        "upper",
                    ));
                }
            }
            Some((b, p))
        }
        _ => None,
    };
    checks::line_weight(input.line_weight, "lineWeight")?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let mut warnings = checks::layer(doc, &input.layer_id)?;

    let base = EntityBase {
        look: Default::default(),
        id: 0,
        layer_id: input.layer_id.clone(),
        color: input.color.clone(),
        attrs: BTreeMap::new(),
        label: None,
        symbol: None,
        line_weight: input.line_weight,
        label_pins: Vec::new(),
        attachments: Vec::new(),
    };
    let sink_at = input.sink.map(|k| k as usize);
    let cooktop_at = input.cooktop.map(|k| k as usize);
    let spans = module_spans(count, module, sink_at.zip(sink_width));
    let mut entities = Vec::new();
    let mut modules = Vec::with_capacity(count);
    for (i, &(start, w)) in spans.iter().enumerate() {
        let k = i + 1;
        entities.push(insert(
            &cabinet,
            &base,
            wire(frame.at(start, 0.0)),
            &frame,
            !left,
            width_values(width, w),
        ));
        let kind = if sink_at == Some(k) {
            CaseworkModuleKind::Sink
        } else if cooktop_at == Some(k) {
            CaseworkModuleKind::Cooktop
        } else {
            CaseworkModuleKind::Base
        };
        modules.push(CaseworkModule {
            kind,
            start,
            width: w,
        });
    }
    let end = spans.last().map_or(0.0, |&(s, w)| s + w);
    let filler = frame.length - end;
    if let (Some(b), Some(k)) = (&sink_block, sink_at) {
        let (start, _) = spans[k - 1];
        entities.push(insert(
            b,
            &base,
            wire(frame.at(start + SINK_INSET, SINK_INSET)),
            &frame,
            !left,
            BTreeMap::new(),
        ));
    }
    if let (Some(b), Some(k)) = (&cooktop_block, cooktop_at) {
        let (start, w) = spans[k - 1];
        entities.push(insert(
            b,
            &base,
            wire(frame.at(start + w / 2.0, COOKTOP_INSET)),
            &frame,
            !left,
            BTreeMap::new(),
        ));
    }
    if let Some((b, p)) = &upper {
        for &(start, w) in &spans {
            entities.push(insert(
                b,
                &base,
                wire(frame.at(start, 0.0)),
                &frame,
                !left,
                width_values(p, w),
            ));
        }
    }
    if filler > NO_FILLER {
        entities.push(rectangle(
            &base,
            band(&frame, end, frame.length, DEFAULT_CASEWORK_DEPTH, left),
            false,
        ));
        warnings.push(CommandWarning {
            code: codes::CASEWORK_FILLER.into(),
            message: format!(
                "{count} modülden sonra dizinin sonunda {} m dolgu kalıyor.",
                kentos_geometry_core::display::fixed(filler, 3)
            ),
            path: Some("to".into()),
        });
    }
    entities.push(rectangle(
        &base,
        band(&frame, 0.0, frame.length, depth, left),
        true,
    ));
    Ok(Checked {
        entities,
        added,
        modules,
        filler: if filler > NO_FILLER { filler } else { 0.0 },
        warnings,
    })
}

fn fresh_id() -> BlockId {
    BlockId(*Uuid::now_v7().as_bytes())
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &CaseworkRun) -> CommandResult<()> {
    match check(cx.doc, input, fresh_id) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &CaseworkRun) -> CommandResult<CaseworkPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] over a document: what the tool draws as its preview.
pub fn plan_in(doc: &Document, input: &CaseworkRun) -> CommandResult<CaseworkPlan> {
    match check(doc, input, fresh_id) {
        Ok(c) => CommandResult::Completed {
            output: CaseworkPlan {
                entities: c.entities,
                blocks: c.added.iter().map(|b| b.name.clone()).collect(),
                modules: c.modules,
                filler: c.filler,
                revision: doc.revision().to_string(),
            },
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// The run as the tool previews it: the plan and the shapes its objects draw in the drawing, its inserts opened
/// through their blocks (the drawing's, or the library's it would bring) with their widths; each with whether it
/// draws dashed (a wall cabinet's).
pub struct CaseworkDrawn {
    pub plan: CaseworkPlan,
    pub shapes: Vec<(kentos_geometry_core::entity::Shape, bool)>,
}

/// [`plan_in`] with what it draws: the tool's preview; writes nothing.
pub fn drawn_in(doc: &Document, input: &CaseworkRun) -> CommandResult<CaseworkDrawn> {
    let c = match check(doc, input, fresh_id) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let mut defs: Vec<std::sync::Arc<BlockDefinition>> = doc.blocks().to_vec();
    defs.extend(c.added.iter().cloned().map(std::sync::Arc::new));
    let blocks = crate::blocks::core_blocks(&defs);
    let mut shapes = Vec::new();
    for e in &c.entities {
        let shape = crate::geometry::shape(e);
        if matches!(e, Entity::Insert(_)) {
            shapes.extend(
                blocks
                    .expand(&shape)
                    .into_iter()
                    .filter(|piece| !kentos_geometry_core::block::shows_nothing(&piece.shape))
                    .map(|piece| {
                        let dashed = piece.stroke.line_type.as_deref() == Some("dashed");
                        (piece.shape, dashed)
                    }),
            );
        } else {
            shapes.push((shape, false));
        }
    }
    CommandResult::Completed {
        output: CaseworkDrawn {
            plan: CaseworkPlan {
                entities: c.entities,
                blocks: c.added.iter().map(|b| b.name.clone()).collect(),
                modules: c.modules,
                filler: c.filler,
                revision: doc.revision().to_string(),
            },
            shapes,
        },
        warnings: c.warnings,
    }
}

/// Checks `input` again and writes the run, with the library blocks it brings, as one undo step (“Mutfak dizisi”).
pub fn execute(cx: &mut ExecutionContext<'_>, input: CaseworkRun) -> CommandResult<CaseworkBuilt> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_CASEWORK_RUN,
        kentos_commands::CAD_CASEWORK_RUN_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: CaseworkRun) -> CommandResult<CaseworkBuilt> {
    let c = match check(cx.doc, &input, fresh_id) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let names: Vec<String> = c.added.iter().map(|b| b.name.clone()).collect();
    let written = cx.doc.transact(STEP, |doc| {
        for b in c.added {
            doc.add_block(b).map_err(|r| {
                Stop::failed(error(codes::INVALID_CASEWORK, r.to_string(), Some("layerId".into())))
            })?;
        }
        doc.add_many(c.entities, STEP).map_err(|full| {
            Stop::failed(error(codes::SLOTS_EXHAUSTED, full.to_string(), None))
        })
    });
    let slots = match written {
        Ok(slots) => slots,
        Err(stop) => return stop.into(),
    };
    let uids = slots
        .iter()
        .filter_map(|s| cx.doc.uid(*s))
        .map(|u| u.to_string())
        .collect();
    CommandResult::Completed {
        output: CaseworkBuilt {
            uids,
            ids: slots.iter().map(|s| s.0).collect(),
            blocks: names,
            modules: c.modules,
            filler: c.filler,
            revision: cx.doc.revision().to_string(),
        },
        warnings: c.warnings,
    }
}
