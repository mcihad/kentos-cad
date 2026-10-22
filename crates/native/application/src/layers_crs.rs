//! `cad.layers.crs` and `cad.layers.reproject` v1 (docs/adr/0583 §2): a layer's source system recorded, nothing moved
//! (Sistem ata), and a layer's objects moved from a system into the project's, the record that system (Yeniden
//! izdüşür), each as one undo step. The shared cases are `fixtures/commands/v1/cad.layers.crs.json` and
//! `cad.layers.reproject.json` (scripts/fixtures/layer_crs_cases.py, with PROJ).

use kentos_commands::{
    CommandResult, LayerNode, LayerNodeType, LayersCrs, LayersCrsSet, LayersReproject,
    LayersReprojected,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::crs as core;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::sources::{Mover, mover_in};

/// The undo steps' names.
pub const ASSIGN: &str = "Sistem ata";
pub const REPROJECT: &str = "Yeniden izdüşür";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn layer_of<'a>(doc: &'a Document, id: &str) -> Result<&'a LayerNode, Stop> {
    let Some(node) = doc.layers().get(id) else {
        return Err(fail(
            codes::LAYER_NOT_FOUND,
            format!("“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."),
            "layer",
        ));
    };
    if node.kind != LayerNodeType::Layer {
        return Err(fail(
            codes::NOT_A_LAYER,
            format!(
                "“{}” bir katman grubu; kaynak sistemi yalnız katmanın olur. Grubun bir katmanını verin.",
                node.name
            ),
            "layer",
        ));
    }
    Ok(node)
}

/// The record of `srid`, or why not: a code the registry knows, not the local system.
fn record(srid: u32, path: &str) -> Result<kentos_commands::LayerSourceCrs, Stop> {
    kentos_project::wkt2::source_crs(srid).ok_or_else(|| {
        fail(
            codes::UNKNOWN_SRID,
            if srid == 0 {
                "Yerel sistem bir kaynak sistemi değildir; kaydı silmek için srid vermeyin.".to_owned()
            } else {
                format!("EPSG:{srid} koordinat sistemleri kaydında yok. Kayıttaki bir sistemin kodunu verin.")
            },
            path,
        )
    })
}

// ── cad.layers.crs ─────────────────────────────────────────────────────────

fn assign_check(
    doc: &Document,
    input: &LayersCrs,
) -> Result<(LayerNode, Option<kentos_commands::LayerSourceCrs>), Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    let node = layer_of(doc, &input.layer)?.clone();
    let next = match input.srid {
        None => None,
        Some(srid) => Some(record(srid, "srid")?),
    };
    Ok((node, next))
}

pub fn crs_validate(cx: &ExecutionContext<'_>, input: &LayersCrs) -> CommandResult<()> {
    match assign_check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn crs_plan(cx: &ExecutionContext<'_>, input: &LayersCrs) -> CommandResult<LayersCrsSet> {
    match assign_check(cx.doc, input) {
        Ok((node, next)) => CommandResult::Completed {
            output: LayersCrsSet {
                layer: node.id.clone(),
                changed: node.source_crs != next,
                crs: next,
                revision: cx.doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn crs_execute(cx: &mut ExecutionContext<'_>, input: LayersCrs) -> CommandResult<LayersCrsSet> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_LAYERS_CRS,
        kentos_commands::CAD_LAYERS_CRS_VERSION,
        input,
        crs_write,
    )
}

fn crs_write(cx: &mut ExecutionContext<'_>, input: LayersCrs) -> CommandResult<LayersCrsSet> {
    let (node, next) = match assign_check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    match cx.doc.set_layer_source_crs(&node.id, next.clone(), ASSIGN) {
        Ok(changed) => CommandResult::Completed {
            output: LayersCrsSet {
                layer: node.id,
                crs: next,
                changed,
                revision: cx.doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(refusal) => fail(codes::LAYER_REFUSED, refusal.0, "layer").into(),
    }
}

// ── cad.layers.reproject ───────────────────────────────────────────────────

/// What a reprojection writes: the objects moved, the system they came from, the way.
struct Moved {
    layer: String,
    from: u32,
    to: u32,
    changes: Vec<(Slot, kentos_commands::Entity)>,
    via: String,
    record: kentos_commands::LayerSourceCrs,
}

/// The way a point goes, as the transforms say it (“EPSG:1311, ±1 m”).
fn way(m: &Mover, at: kentos_geometry_core::Vec2) -> String {
    match core::transform_in(&m.source, &m.project, at, &m.choices) {
        Ok(t) => {
            let mut words = if t.via.is_empty() {
                "aynı datum".to_owned()
            } else {
                t.via.clone()
            };
            if let Some(a) = t.accuracy {
                words.push_str(&format!(", ±{a} m"));
            }
            if t.unofficial {
                words.push_str(", resmî dönüşüm değil");
            }
            words
        }
        Err(_) => String::new(),
    }
}

fn reproject_check(doc: &Document, input: &LayersReproject) -> Result<Moved, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    let node = layer_of(doc, &input.layer)?;
    let name = node.name.clone();
    if doc.layers().is_locked(&node.id) {
        return Err(fail(
            codes::LAYER_LOCKED,
            format!("“{name}” katmanı kilitli; kilidi Katmanlar panelinden açın."),
            "layer",
        ));
    }
    let from = match input
        .from
        .or_else(|| node.source_crs.as_ref().and_then(|c| c.srid))
    {
        Some(f) => f,
        None => {
            return Err(fail(
                codes::INVALID_INPUT,
                format!(
                    "“{name}” katmanının kaynak sistemi kaydı yok; nesnelerin şimdiki sistemini from ile verin."
                ),
                "from",
            ));
        }
    };
    let record = record(from, "from")?;
    let settings = doc.settings();
    if from == settings.srid {
        return Err(fail(
            codes::CRS_MISMATCH,
            format!(
                "Nesneler zaten projenin sisteminde (EPSG:{from}); çevrilecek bir şey yok. Yalnız kayıt için cad.layers.crs."
            ),
            "from",
        ));
    }
    let mover = match mover_in(settings, Some(from)) {
        Ok(Some(m)) => m,
        Ok(None) => unreachable!("from differs from the project's system"),
        Err(why) => {
            return Err(fail(
                codes::CRS_MISMATCH,
                format!("“{name}” katmanı çevrilemiyor: {why}."),
                "from",
            ));
        }
    };
    let mut changes = Vec::new();
    let mut unreached: Vec<Uuid> = Vec::new();
    let mut first: Option<kentos_geometry_core::Vec2> = None;
    for (uid, e) in doc.by_layer_with_uids(&node.id) {
        let mut moved = e.clone();
        if first.is_none() {
            let b = kentos_geometry_core::ops::topology_rules::incremental::shape_bounds(
                &crate::geometry::shape(e),
            );
            let c = kentos_geometry_core::Vec2::new(
                (b.min_x + b.max_x) / 2.0,
                (b.min_y + b.max_y) / 2.0,
            );
            first = (c.x.is_finite() && c.y.is_finite()).then_some(c);
        }
        match kentos_formats::reproject::move_entity(
            &mut moved,
            &mover.source,
            &mover.project,
            &mover.choices,
        ) {
            Ok(()) => changes.push((Slot(e.base().id), moved)),
            Err(_) => unreached.push(uid),
        }
    }
    if !unreached.is_empty() {
        let mut err = error(
            codes::CRS_UNREACHED,
            format!(
                "“{name}” katmanının {} nesnesi EPSG:{from}'den projenin sistemine çevrilemiyor (sistemin dışında); hiçbir şey yazılmadı. Doğru sistemi from ile verin.",
                unreached.len()
            ),
            Some("from".to_owned()),
        );
        err.uids = unreached.iter().map(Uuid::to_string).collect();
        return Err(Stop::failed(err));
    }
    let via = first.map(|p| way(&mover, p)).unwrap_or_default();
    Ok(Moved {
        layer: node.id.clone(),
        from,
        to: settings.srid,
        changes,
        via,
        record,
    })
}

fn reprojected(m: &Moved, revision: String) -> LayersReprojected {
    LayersReprojected {
        layer: m.layer.clone(),
        from: m.from,
        to: m.to,
        moved: m.changes.len() as u32,
        via: m.via.clone(),
        crs: m.record.clone(),
        revision,
    }
}

pub fn reproject_validate(cx: &ExecutionContext<'_>, input: &LayersReproject) -> CommandResult<()> {
    match reproject_check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn reproject_plan(
    cx: &ExecutionContext<'_>,
    input: &LayersReproject,
) -> CommandResult<LayersReprojected> {
    match reproject_check(cx.doc, input) {
        Ok(m) => CommandResult::Completed {
            output: reprojected(&m, cx.doc.revision().to_string()),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn reproject_execute(
    cx: &mut ExecutionContext<'_>,
    input: LayersReproject,
) -> CommandResult<LayersReprojected> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_LAYERS_REPROJECT,
        kentos_commands::CAD_LAYERS_REPROJECT_VERSION,
        input,
        reproject_write,
    )
}

fn reproject_write(
    cx: &mut ExecutionContext<'_>,
    input: LayersReproject,
) -> CommandResult<LayersReprojected> {
    let m = match reproject_check(cx.doc, &input) {
        Ok(m) => m,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    let written: Result<(), kentos_domain::Refusal> = doc.transact(REPROJECT, |doc| {
        doc.update_many(m.changes.clone(), REPROJECT);
        doc.set_layer_source_crs(&m.layer, Some(m.record.clone()), REPROJECT)
            .map(|_| ())
    });
    match written {
        Ok(()) => CommandResult::Completed {
            output: reprojected(&m, doc.revision().to_string()),
            warnings: Vec::new(),
        },
        Err(refusal) => fail(codes::LAYER_REFUSED, refusal.0, "layer").into(),
    }
}

/// `cad.layers.crs` under the names the registry calls.
pub mod assign {
    pub use super::{crs_execute as execute, crs_plan as plan, crs_validate as validate};
}

/// `cad.layers.reproject` under the names the registry calls.
pub mod reproject {
    pub use super::{
        reproject_execute as execute, reproject_plan as plan, reproject_validate as validate,
    };
}
