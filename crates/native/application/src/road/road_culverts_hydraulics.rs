//! `road.culverts.hydraulics` v1 (plan/road.md Y41, docs/adr/0947; ROAD-DRAIN-08): a culvert's headwater at a flow, by
//! inlet and outlet control as FHWA's HDS-5 writes them (`kentos-road`'s `culvert_hydraulics`), the road over it as a
//! weir whose crest is, unless given, the highest of the road's top over the barrel, and where the road is overtopped
//! the depth, speed and hazard class on its crest; with `curve`, the same at flows from 0 to 1.5 × the flow. Reads only.
//! The shared cases are `fixtures/commands/v1/road.culverts.hydraulics.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CulvertFlowRead, CulvertHydraulicsRead, CulvertInlet, CulvertOvertop,
    CulvertShapeDef, CulvertWeir, CulvertsHydraulics,
};
use kentos_domain::Document;
use kentos_road::culvert_hydraulics::{Control, Culvert, Section, Weir, hazard_words, inlet, perform};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_culverts as common;
use crate::road_terrains::fail;

/// The inlet's name among HDS-5's rows `kentos-road` keeps.
fn inlet_name(i: CulvertInlet) -> &'static str {
    match i {
        CulvertInlet::PipeSquareHeadwall => "pipe-square-headwall",
        CulvertInlet::PipeGrooveHeadwall => "pipe-groove-headwall",
        CulvertInlet::PipeGrooveProjecting => "pipe-groove-projecting",
        CulvertInlet::BoxWingwall30To75 => "box-wingwall-30-75",
        CulvertInlet::BoxWingwall90And15 => "box-wingwall-90-15",
        CulvertInlet::BoxWingwall0 => "box-wingwall-0",
        CulvertInlet::BoxHeadwallChamfer => "box-headwall-chamfer",
    }
}

pub(crate) fn bad(path: &str, words: &str) -> Stop {
    fail(codes::INVALID_CULVERT_HYDRAULICS, format!("Menfez hidroliği sorusu geçerli değil: {words}."), path)
}

pub(crate) fn within(v: f64, lo: f64, hi: f64) -> bool {
    v.is_finite() && v >= lo && v <= hi
}

/// One flow, read.
pub(crate) fn flow_read(culvert: &Culvert, weir: &Weir, q: f64, tw: f64, invert: f64) -> CulvertFlowRead {
    let p = perform(culvert, weir, q, tw);
    CulvertFlowRead {
        flow: q,
        headwater: p.headwater,
        level: invert + p.headwater,
        inlet_control: p.control == Control::Inlet,
        inlet_headwater: p.inlet,
        outlet_headwater: p.outlet,
        through: p.culvert,
        overtopping: p.overtopping,
        over: (p.hazard > 0).then(|| CulvertOvertop {
            head: p.head,
            depth: p.depth,
            speed: p.speed,
            unit: p.unit,
            hazard: p.hazard,
            words: hazard_words(p.hazard).into(),
        }),
    }
}

/// What a hydraulics question asks besides its flow, checked before the culvert is read.
pub(crate) struct Asked<'a> {
    pub(crate) tailwater: Option<f64>,
    pub(crate) inlet: CulvertInlet,
    pub(crate) entrance_loss: f64,
    pub(crate) roughness: Option<f64>,
    pub(crate) weir: Option<&'a CulvertWeir>,
}

/// The question's own rules: the tailwater, ke, n (the question's or the culvert's) and the weir. Its n.
pub(crate) fn check(def: &kentos_commands::CulvertDef, asked: &Asked<'_>) -> Result<f64, Stop> {
    if !within(asked.tailwater.unwrap_or(0.0), 0.0, 100.0) {
        return Err(bad("tailwater", "çıkıştaki su derinliği 0 ile 100 m arası olur"));
    }
    if !within(asked.entrance_loss, 0.0, 2.0) {
        return Err(bad("entranceLoss", "giriş kaybı katsayısı 0 ile 2 arası olur"));
    }
    let Some(n) = asked.roughness.or(def.roughness) else {
        return Err(bad("roughness", "menfezin Manning pürüzlülüğü yok; roughness ile verin"));
    };
    if !(n.is_finite() && n > 0.0 && n <= 0.1) {
        return Err(bad("roughness", "Manning pürüzlülüğü 0'dan büyük, en çok 0.1 olur"));
    }
    if let Some(w) = asked.weir {
        if !(w.cd.is_finite() && w.cd > 0.0 && w.cd <= 3.0) {
            return Err(bad("weir.cd", "savak katsayısı 0'dan büyük, en çok 3 olur"));
        }
        if !(w.length.is_finite() && w.length > 0.0 && w.length <= 10_000.0) {
            return Err(bad("weir.length", "savak boyu 0'dan büyük, en çok 10000 m olur"));
        }
        if w.crest.is_some_and(|c| !c.is_finite()) {
            return Err(bad("weir.crest", "savak kreti bir kot olur"));
        }
    }
    Ok(n)
}

/// A culvert read for its hydraulics: its model, the road as a weir (none: never overtopped), its inverts, slope,
/// cells and n, the crest (none without a weir) and the read's warnings.
pub(crate) struct Model {
    pub(crate) culvert: Culvert,
    pub(crate) weir: Weir,
    pub(crate) inlet_invert: f64,
    pub(crate) outlet_invert: f64,
    pub(crate) cells: u32,
    pub(crate) n: f64,
    pub(crate) crest: Option<f64>,
    pub(crate) warnings: Vec<CommandWarning>,
}

/// The culvert read now and modelled with `asked` and its n.
pub(crate) fn model(doc: &Document, def: &kentos_commands::CulvertDef, asked: &Asked<'_>, n: f64) -> Result<Model, Stop> {
    let read = common::read(doc, def)?;
    let r = &read.report;
    let end = |side: &str| r.ends.iter().find(|e| e.side == side).map_or(def.invert, |e| e.invert);
    let outlet_side = if r.inlet == "left" { "right" } else { "left" };
    let (inlet_invert, outlet_invert) = (end(&r.inlet), end(outlet_side));
    let slope = if r.length > 0.0 { (inlet_invert - outlet_invert) / r.length } else { 0.0 };
    let (section, cells) = match def.shape {
        CulvertShapeDef::Pipe { diameter, count, .. } => (Section::Circle { diameter }, count),
        CulvertShapeDef::Box { width, height, cells, .. } => (Section::Box { width, height }, cells),
    };
    let culvert = Culvert {
        section,
        cells,
        length: r.length,
        slope,
        n,
        inlet: inlet(inlet_name(asked.inlet)).expect("every named inlet is kept"),
        ke: asked.entrance_loss,
    };
    let crest = asked.weir.map(|w| w.crest.unwrap_or(read.crest));
    if let Some(c) = crest
        && c <= inlet_invert
    {
        return Err(bad("weir.crest", &format!("savak kreti ({c:.3}) menfezin giriş kotunun ({inlet_invert:.3}) üstünde olmalı")));
    }
    let weir = match (asked.weir, crest) {
        (Some(w), Some(c)) => Weir { crest: c - inlet_invert, length: w.length, cd: w.cd },
        _ => Weir { crest: f64::INFINITY, length: 0.0, cd: 0.0 },
    };
    Ok(Model { culvert, weir, inlet_invert, outlet_invert, cells, n, crest, warnings: read.warnings })
}

fn answer(doc: &Document, input: &CulvertsHydraulics) -> Result<(CulvertHydraulicsRead, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let (_, def) = common::find(doc.road_culverts(), &input.culvert, "culvert")?;
    if !(input.flow.is_finite() && input.flow > 0.0 && input.flow <= 10_000.0) {
        return Err(bad("flow", "debi 0'dan büyük, en çok 10000 m³/s olur"));
    }
    let asked = Asked {
        tailwater: input.tailwater,
        inlet: input.inlet,
        entrance_loss: input.entrance_loss,
        roughness: input.roughness,
        weir: input.weir.as_ref(),
    };
    let n = check(def, &asked)?;
    if input.curve.is_some_and(|c| !(2..=200).contains(&c)) {
        return Err(bad("curve", "eğrinin nokta sayısı 2 ile 200 arası olur"));
    }
    let m = model(doc, def, &asked, n)?;
    let tw = input.tailwater.unwrap_or(0.0);
    let at = flow_read(&m.culvert, &m.weir, input.flow, tw, m.inlet_invert);
    let curve = input.curve.map_or_else(Vec::new, |k| {
        let top = 1.5 * input.flow;
        (0..k).map(|i| flow_read(&m.culvert, &m.weir, top * f64::from(i) / f64::from(k - 1), tw, m.inlet_invert)).collect()
    });
    let mut warnings = m.warnings;
    if let Some(o) = &at.over {
        warnings.push(CommandWarning {
            code: codes::CULVERT_OVERTOPPED.into(),
            message: format!(
                "“{}” menfezinde {} m³/s debide su yolu aşıyor: yoldan {:.3} m³/s geçer, kret üstünde su {:.3} m; {}.",
                def.name,
                input.flow,
                at.overtopping,
                o.head,
                o.words
            ),
            path: Some("flow".into()),
        });
    }
    Ok((
        CulvertHydraulicsRead {
            culvert: def.id.clone(),
            inlet_invert: m.inlet_invert,
            outlet_invert: m.outlet_invert,
            length: m.culvert.length,
            slope: m.culvert.slope,
            cells: m.cells,
            roughness: m.n,
            crest: m.crest,
            at,
            curve,
        },
        warnings,
    ))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &CulvertsHydraulics) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &CulvertsHydraulics) -> CommandResult<CulvertHydraulicsRead> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: CulvertsHydraulics) -> CommandResult<CulvertHydraulicsRead> {
    plan(cx, &input)
}

/// The hydraulics for a window, or why they do not read.
pub fn preview(doc: &Document, input: &CulvertsHydraulics) -> Result<(CulvertHydraulicsRead, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}
