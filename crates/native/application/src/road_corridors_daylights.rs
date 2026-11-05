//! `road.corridors.daylights` v1 (plan/road.md Y45, docs/adr/0951; ROAD-COR-10): how each of a corridor's sections
//! meets the terrain as built now (`kentos_road::section::template::daylights`): each side's daylight with the slope's
//! crossings of the terrain and the daylight's sensitivity to the terrain's height, or why it has none — no terrain at
//! the hinge, the terrain stopping first, the swath ending first — apart where the corridor's report said only “şev
//! buluşmuyor”. Reads only. The shared cases are `fixtures/commands/v1/road.corridors.daylights.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CorridorDaylights, CorridorsDaylights, DaylightCounts, DaylightSide, DaylightStation,
    DaylightWorst,
};
use kentos_domain::Document;
use kentos_road::alignment::km_text;
use kentos_road::section::template::{Daylight, daylights};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_corridors as common;
use crate::road_terrains::fail;

fn side_of(d: &Daylight, sign: f64) -> DaylightSide {
    let none = DaylightSide { kind: String::new(), offset: None, cut: None, crossings: None, next: None, sensitivity: None };
    match *d {
        Daylight::Hit { offset, crossings, next, sensitivity, cut } => DaylightSide {
            kind: "hit".into(),
            offset: Some(sign * offset),
            cut: Some(cut),
            crossings: Some(crossings),
            next,
            sensitivity: sensitivity.is_finite().then_some(sensitivity),
        },
        Daylight::Hinge => DaylightSide { kind: "hinge".into(), ..none },
        Daylight::Gap { at } => DaylightSide { kind: "gap".into(), offset: Some(sign * at), ..none },
        Daylight::Swath { end } => DaylightSide { kind: "swath".into(), offset: Some(sign * end), ..none },
    }
}

/// The daylights now, and their warnings.
pub(crate) fn answer(doc: &Document, input: &CorridorsDaylights) -> Result<(CorridorDaylights, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let (_, def) = common::find(doc.road_corridors(), &input.corridor, "corridor")?;
    if input.sensitive.is_some_and(|s| !(s.is_finite() && s > 0.0)) {
        return Err(fail(
            codes::INVALID_DAYLIGHT_CHECK,
            "Şev ucu denetimi sorusu geçerli değil: duyarlılık sınırı (sensitive) 0'dan büyük olmalı.".into(),
            "sensitive",
        ));
    }
    daylights_of(doc, def, input.sensitive)
}

/// The daylights of `def` (saved or as typed) and their warnings.
fn daylights_of(
    doc: &Document,
    def: &kentos_commands::CorridorDef,
    sensitive_at: Option<f64>,
) -> Result<(CorridorDaylights, Vec<CommandWarning>), Stop> {
    let input = CorridorsDaylights { corridor: def.id.clone(), sensitive: sensitive_at };
    let made = common::make(doc, def, false)?;
    let text = |s: f64| km_text(made.sampler.alignment.km(s), 3);
    let mut stations = Vec::new();
    let mut counts = DaylightCounts { hit: 0, hinge: 0, gap: 0, swath: 0, multiple: 0, sensitive: 0, unbuilt: 0 };
    let mut worst: Option<DaylightWorst> = None;
    let (mut multiple, mut sensitive): (Vec<f64>, Vec<f64>) = (Vec::new(), Vec::new());
    for (ri, region) in made.regions.iter().enumerate() {
        for cross in &region.crosses {
            let s = cross.line.station;
            let Some(sec) = &cross.section else {
                counts.unbuilt += 1;
                continue;
            };
            let Some((_, ground)) = made.sampler.sample(&region.template, s) else {
                counts.unbuilt += 1;
                continue;
            };
            let template = made.sampler.template_at(&region.template, s);
            let both = daylights(&template, sec.axis, &ground);
            for (k, d) in both.iter().enumerate() {
                match *d {
                    Daylight::Hit { crossings, sensitivity, .. } => {
                        counts.hit += 1;
                        if crossings > 1 {
                            counts.multiple += 1;
                            multiple.push(s);
                        }
                        if input.sensitive.is_some_and(|t| sensitivity > t) {
                            counts.sensitive += 1;
                            sensitive.push(s);
                        }
                        if sensitivity.is_finite() && worst.as_ref().is_none_or(|w| sensitivity > w.sensitivity) {
                            worst = Some(DaylightWorst {
                                station: s,
                                km_text: text(s),
                                side: if k == 0 { "left" } else { "right" }.into(),
                                sensitivity,
                            });
                        }
                    }
                    Daylight::Hinge => counts.hinge += 1,
                    Daylight::Gap { .. } => counts.gap += 1,
                    Daylight::Swath { .. } => counts.swath += 1,
                }
            }
            stations.push(DaylightStation {
                station: s,
                km_text: text(s),
                region: ri as u32,
                left: side_of(&both[0], -1.0),
                right: side_of(&both[1], 1.0),
            });
        }
    }
    let mut warnings = made.warnings;
    let listed = |list: &[f64]| {
        let mut l: Vec<f64> = list.to_vec();
        l.dedup();
        let words: Vec<String> = l.iter().take(8).map(|s| text(*s)).collect();
        let more = if l.len() > 8 { format!(" ve {} kesit daha", l.len() - 8) } else { String::new() };
        format!("{}{more}", words.join(", "))
    };
    if !multiple.is_empty() {
        warnings.push(CommandWarning {
            code: codes::CORRIDOR_DAYLIGHT_CROSSINGS.into(),
            message: format!(
                "“{}” koridorunda {} yanda şev araziyi birden çok kez kesiyor (şev ucu ilk kesişme): {}.",
                def.name,
                multiple.len(),
                listed(&multiple)
            ),
            path: Some("corridor".into()),
        });
    }
    if let Some(t) = input.sensitive
        && !sensitive.is_empty()
    {
        warnings.push(CommandWarning {
            code: codes::CORRIDOR_DAYLIGHT_SENSITIVE.into(),
            message: format!(
                "“{}” koridorunda {} yanda şev ucu arazinin her metre kot hatasıyla {} m'den çok kayıyor: {}.",
                def.name,
                sensitive.len(),
                t,
                listed(&sensitive)
            ),
            path: Some("sensitive".into()),
        });
    }
    Ok((CorridorDaylights { corridor: def.id.clone(), sensitive: input.sensitive, stations, counts, worst }, warnings))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &CorridorsDaylights) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &CorridorsDaylights) -> CommandResult<CorridorDaylights> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: CorridorsDaylights) -> CommandResult<CorridorDaylights> {
    plan(cx, &input)
}

/// The daylights for a window, or why they do not read.
pub fn preview(doc: &Document, input: &CorridorsDaylights) -> Result<(CorridorDaylights, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(common::words)
}

/// The daylights of a corridor as typed (the desktop's Koridor window, before it is saved).
pub fn preview_of(
    doc: &Document,
    def: &kentos_commands::CorridorDef,
    sensitive: Option<f64>,
) -> Result<(CorridorDaylights, Vec<CommandWarning>), String> {
    if sensitive.is_some_and(|s| !(s.is_finite() && s > 0.0)) {
        return Err("Duyarlılık sınırı 0'dan büyük olmalı.".into());
    }
    daylights_of(doc, def, sensitive).map_err(common::words)
}
