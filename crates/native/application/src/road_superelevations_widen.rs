//! `road.superelevations.widen` v1 (plan/road.md Y28, docs/adr/0932 §3): each curve's traveled way widening for a
//! design vehicle, suggested for a superelevation schedule on a template's lanes (`kentos_road::widening`), and with
//! `drive` the off-tracking the vehicle reaches driving the curve (`kentos_road::swept`). Reads only: the desktop's
//! Dever window writes a suggestion into the schedule through `road.superelevations.edit`. The shared cases are
//! `fixtures/commands/v1/road.superelevations.widen.json`.

use kentos_commands::{
    CommandResult, CommandWarning, SuperelevationMethod, SuperelevationsWiden, SuperelevationsWidened, WideningMethod,
    WideningRow,
};
use kentos_domain::Document;
use kentos_road::swept::{self, Trailer, Vehicle};
use kentos_road::widening::{self as wid, WideningError, WideningRoad, WideningVehicle};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_superelevations as common;
use crate::road_terrains::fail;

/// The drive's step along the alignment (m): the chords' sagitta at R 15 m is 0.5 mm.
const DRIVE_STEP: f64 = 0.25;

fn bad(path: &str, words: &str) -> Stop {
    fail(codes::INVALID_WIDENING, words.into(), path)
}

fn swept_vehicle(input: &SuperelevationsWiden) -> Vehicle {
    let v = &input.vehicle;
    Vehicle {
        wheelbase: v.wheelbase,
        front: v.front,
        rear: v.rear,
        width: v.width,
        trailer: v.trailer.as_ref().map(|t| Trailer {
            kingpin: t.kingpin,
            wheelbase: t.wheelbase,
            front: t.front,
            rear: t.rear,
            width: t.width,
        }),
    }
}

/// The input's own values judged, before the drawing is read.
fn judged(input: &SuperelevationsWiden) -> Result<(), Stop> {
    if swept::drive(&swept_vehicle(input), &[[0.0, 0.0], [1.0, 0.0]], 1.0).is_err() {
        return Err(bad(
            "vehicle",
            "Taşıtın dingil aralığı ve genişliği sıfırdan büyük, çıkıntıları eksi olmayan sonlu sayılar olmalı (römorkun da).",
        ));
    }
    let positive = |v: Option<f64>| v.is_none_or(|x| x.is_finite() && x > 0.0);
    if !positive(input.track) {
        return Err(bad("track", "İz genişliği (track) sıfırdan büyük olmalı (m)."));
    }
    if !(input.clearance.is_finite() && input.clearance >= 0.0) {
        return Err(bad("clearance", "Yanal açıklık (clearance) eksi olmayan bir sayı olmalı (m)."));
    }
    if !positive(input.speed) {
        return Err(bad("speed", "Proje hızı sıfırdan büyük olmalı (km/sa)."));
    }
    if !positive(input.step) {
        return Err(bad("step", "Yuvarlama adımı (step) sıfırdan büyük olmalı (m)."));
    }
    if !input.least.is_none_or(|x| x.is_finite() && x >= 0.0) {
        return Err(bad("least", "En az genişletme (least) eksi olmayan bir sayı olmalı (m)."));
    }
    Ok(())
}

/// The suggestion now, and its warnings.
pub(crate) fn answer(doc: &Document, input: &SuperelevationsWiden) -> Result<(SuperelevationsWidened, Vec<CommandWarning>), Stop> {
    judged(input)?;
    common::road_project(doc)?;
    let (_, def) = common::find(doc.road_superelevations(), &input.superelevation, "superelevation")?;
    widen_def(doc, def, input)
}

/// The suggestion for a schedule as typed (the desktop's Dever window, before it is saved).
pub fn preview(
    doc: &Document,
    def: &kentos_commands::SuperelevationDef,
    input: &SuperelevationsWiden,
) -> Result<(SuperelevationsWidened, Vec<CommandWarning>), String> {
    let words = |stop: Stop| match stop {
        Stop::Failed(e) | Stop::Conflict(e) => e.message,
        Stop::Cancelled => String::new(),
    };
    judged(input).map_err(words)?;
    common::road_project(doc).map_err(words)?;
    widen_def(doc, def, input).map_err(words)
}

fn widen_def(
    doc: &Document,
    def: &kentos_commands::SuperelevationDef,
    input: &SuperelevationsWiden,
) -> Result<(SuperelevationsWidened, Vec<CommandWarning>), Stop> {
    let template = doc
        .road_templates()
        .iter()
        .find(|t| t.id == input.template || t.name.to_lowercase() == input.template.to_lowercase())
        .ok_or_else(|| {
            fail(codes::UNKNOWN_ROAD_TEMPLATE, format!("“{}” kimlikli ya da adlı tip kesit projede yok.", input.template), "template")
        })?;
    let core = crate::road_sections::core_template(template);
    let lanes_of = |side: &kentos_road::section::template::Side| {
        side.components.iter().filter(|c| matches!(c, kentos_road::section::template::Component::Lane { .. })).count()
    };
    let lanes = (lanes_of(&core.left) + lanes_of(&core.right)) as u32;
    let (wl, wr) = crate::road_sections::lane_widths(&core);
    if lanes == 0 {
        return Err(bad("template", "Tip kesitte şerit yok: genişletme şeritlerin sayısından ve genişliğinden hesaplanır."));
    }
    let built = common::alignment_of(doc, def)?;
    let method = input.method.unwrap_or_default();
    let road = WideningRoad {
        lanes,
        tangent: wl + wr,
        clearance: input.clearance,
        speed: input.speed,
        method: match method {
            WideningMethod::Standard => wid::WideningMethod::Standard,
            WideningMethod::Simple => wid::WideningMethod::Simple,
        },
    };
    let v = &input.vehicle;
    let vehicle = WideningVehicle {
        track: input.track.unwrap_or(v.width),
        front: v.front,
        wheelbase: v.wheelbase,
        trailer: v.trailer.as_ref().map(|t| (t.kingpin, t.wheelbase)),
    };
    let current = |pi: &str| match &def.method {
        SuperelevationMethod::Curves { curves } => curves.iter().find(|c| c.pi == pi).map_or(0.0, |c| c.widening),
        _ => 0.0,
    };
    let al = &built.core;
    let mut curves = Vec::new();
    let mut unturnable = Vec::new();
    for c in &al.curves {
        if c.radius.is_nan() || c.radius <= 0.0 {
            continue;
        }
        let (from, to) = (c.stations[0], c.stations[3]);
        let w = match wid::widening(c.radius, &vehicle, &road) {
            Ok(w) => w,
            Err(WideningError::Unturnable) => {
                unturnable.push(c.pi.clone());
                continue;
            }
            Err(WideningError::Input) => return Err(bad("vehicle", "Taşıtın ölçüleri bu kurpta okunamadı.")),
        };
        let driven = input.drive.then(|| drive(al, &swept_vehicle(input), from, to)).flatten();
        curves.push(WideningRow {
            pi: c.pi.clone(),
            from,
            to,
            radius: c.radius,
            offtracking: w.offtracking,
            track: w.track,
            overhang: w.overhang,
            allowance: w.allowance,
            width: w.width,
            widening: w.widening,
            suggested: wid::rounded(w.widening, input.step, input.least),
            current: current(&c.pi),
            driven,
        });
    }
    let mut warnings = Vec::new();
    if !unturnable.is_empty() {
        warnings.push(CommandWarning {
            code: codes::WIDENING_UNTURNABLE.into(),
            message: format!(
                "Taşıt {} kurbunda kararlı dönemez: yarıçap dingil aralıklarının karelerinin toplamının kökünden küçük.",
                unturnable.join(", ")
            ),
            path: Some("vehicle".into()),
        });
    }
    if let Some(w) = common::stale_warning(doc, def) {
        warnings.push(w);
    }
    let output = SuperelevationsWidened { superelevation: def.id.clone(), lanes, tangent: wl + wr, method, curves, unturnable };
    Ok((output, warnings))
}

/// The largest off-tracking of the vehicle driven along the alignment over a curve (TS `from` to ST `to`): its front
/// axle on the alignment from far enough before the TS for the vehicle to be straight (twice its length, ten metres
/// more) to the ST; the rearmost axle's distance from the front's path.
fn drive(al: &kentos_road::alignment::chain::Alignment, vehicle: &Vehicle, from: f64, to: f64) -> Option<f64> {
    let long = vehicle.wheelbase + vehicle.trailer.map_or(0.0, |t| t.kingpin.abs() + t.wheelbase);
    let start = (from - 2.0 * long - 10.0).max(0.0);
    let end = to.min(al.length);
    let n = ((end - start) / DRIVE_STEP).ceil().max(1.0) as usize;
    let path: Vec<[f64; 2]> =
        (0..=n).filter_map(|i| al.at((start + i as f64 * DRIVE_STEP).min(end))).map(|p| [p.x, p.y]).collect();
    let swept = swept::drive(vehicle, &path, DRIVE_STEP).ok()?;
    Some(swept.trailer_offtracking.unwrap_or(swept.offtracking))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &SuperelevationsWiden) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &SuperelevationsWiden) -> CommandResult<SuperelevationsWidened> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: SuperelevationsWiden) -> CommandResult<SuperelevationsWidened> {
    plan(cx, &input)
}
