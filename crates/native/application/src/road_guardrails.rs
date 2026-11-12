//! A road project's guardrails (plan/road.md Y11.1, docs/adr/0909 §1): found by id or name, read now from their
//! corridor's crosses every `step` metres (`road_stakeout::table`'s hinge and daylight points): each side's fill height
//! (the hinge above the daylight point) and slope (their run per that rise), read by `kentos-road`'s
//! `guardrail::side_runs` against the project's rule. Nothing read is kept: every read computes again (ROAD-MOD-06).

use kentos_commands::{
    CommandWarning, GuardrailDef, GuardrailListed, GuardrailReport, GuardrailRun, StakeoutDistances, StakeoutQuery,
};
use kentos_domain::Document;
use kentos_road::alignment::km_text;
use kentos_road::guardrail::{Rule, Run, Sample, Warrant, side_runs};

use crate::checks::Stop;
use crate::codes;
use crate::road_corridors::{self as corridors, words};
use crate::road_terrains::fail;

/// Only a road project has guardrails.
pub(crate) fn road_project(doc: &Document) -> Result<(), Stop> {
    if doc.settings().cad_work == Some(kentos_commands::CadWork::Road) {
        Ok(())
    } else {
        Err(fail(
            codes::NOT_A_ROAD_PROJECT,
            "Otokorkuluk yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.".into(),
            "",
        ))
    }
}

/// The guardrail of `name`: by its id, else by its name (case aside).
pub(crate) fn find<'a>(list: &'a [GuardrailDef], name: &str, path: &str) -> Result<(usize, &'a GuardrailDef), Stop> {
    list.iter()
        .position(|g| g.id == name)
        .or_else(|| list.iter().position(|g| g.name.to_lowercase() == name.to_lowercase()))
        .map(|i| (i, &list[i]))
        .ok_or_else(|| fail(codes::UNKNOWN_GUARDRAIL, format!("“{name}” kimlikli ya da adlı otokorkuluk projede yok."), path))
}

pub(crate) fn listed(list: &[GuardrailDef]) -> Vec<GuardrailListed> {
    list.iter()
        .map(|g| GuardrailListed { id: g.id.clone(), name: g.name.clone(), corridor: g.corridor.clone(), revision: g.revision })
        .collect()
}

/// Whether the guardrail's corridor changed since it was made or rebound, and its present revision.
pub(crate) fn stale(doc: &Document, def: &GuardrailDef) -> (bool, u32) {
    let now = doc.road_corridors().iter().find(|c| c.id == def.corridor).map_or(0, |c| c.revision);
    (now != def.corridor_revision, now)
}

/// A side of a cross as read: its station, its hinge's offset (right positive) and its fill.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Side {
    pub(crate) station: f64,
    pub(crate) hinge: f64,
    pub(crate) fill: Option<(f64, f64)>,
}

/// A guardrail as read: its report, its warnings, each side's crosses (left, right) and the corridor's alignment.
pub(crate) struct Read {
    pub(crate) report: GuardrailReport,
    pub(crate) warnings: Vec<CommandWarning>,
    pub(crate) sides: [Vec<Side>; 2],
    pub(crate) runs: Vec<Run>,
    pub(crate) alignment: crate::road_alignments::Built,
}

/// The guardrail read now.
pub(crate) fn read(doc: &Document, def: &GuardrailDef) -> Result<Read, Stop> {
    read_told(doc, def, &|_, _| false)
}

/// [`read`] telling `tick` how far it has come (docs/adr/4860): its corridor's sections, then its crosses every `step`
/// metres (`road_stakeout::table_told`); `tick` answering true stops it there, cancelled.
pub(crate) fn read_told(doc: &Document, def: &GuardrailDef, tick: &dyn Fn(u64, u64) -> bool) -> Result<Read, Stop> {
    let (_, corridor) = corridors::find(doc.road_corridors(), &def.corridor, "corridor")?;
    let query = StakeoutQuery {
        corridor: corridor.id.clone(),
        from: None,
        to: None,
        step: Some(def.step),
        codes: Some(["left.hinge", "left.daylight", "right.hinge", "right.daylight"].map(str::to_owned).to_vec()),
        distances: StakeoutDistances::Grid,
    };
    let (table, mut warnings, alignment) = crate::road_stakeout::table_told(doc, &query, tick).map_err(|stop| match stop {
        Stop::Cancelled => Stop::Cancelled,
        stop => fail(
            codes::INVALID_GUARDRAIL,
            format!("“{}” otokorkuluğunun koridoru okunamıyor: {}", def.name, words(stop)),
            "corridor",
        ),
    })?;
    // A cross without a section reads as no fill on either side; the stakeout's own words for it are not this table's.
    warnings.retain(|w| w.code != codes::STAKEOUT_UNSOLVED);
    // Each station's side once (a station two regions share, its first), its hinge and daylight.
    let mut sides: [Vec<Side>; 2] = [Vec::new(), Vec::new()];
    let mut stations: Vec<f64> = Vec::new();
    for r in &table.rows {
        if stations.last() != Some(&r.station) {
            stations.push(r.station);
        }
    }
    for (k, word) in ["left", "right"].into_iter().enumerate() {
        for &s in &stations {
            if sides[k].last().is_some_and(|x| x.station == s) {
                continue;
            }
            let at = |code: &str| table.rows.iter().find(|r| r.station == s && r.code == format!("{word}.{code}"));
            let Some(h) = at("hinge") else { continue };
            let fill = at("daylight").and_then(|d| {
                let rise = h.z - d.z;
                (rise > 1e-9).then(|| (rise, (d.offset - h.offset).abs() / rise))
            });
            sides[k].push(Side { station: s, hinge: h.offset, fill });
        }
    }
    let rule = Rule {
        warrants: def.rule.warrants.iter().map(|w| Warrant { height: w.height, slope: w.slope }).collect(),
        approach: def.rule.approach,
        leaving: def.rule.leaving,
        gap: def.rule.gap,
    };
    let first = corridor.regions.iter().map(|r| r.from).fold(f64::INFINITY, f64::min);
    let last = corridor.regions.iter().map(|r| r.to).fold(f64::NEG_INFINITY, f64::max);
    let mut runs = Vec::new();
    for (k, right) in [(0, false), (1, true)] {
        let samples: Vec<Sample> = sides[k].iter().map(|s| Sample { station: s.station, fill: s.fill }).collect();
        runs.extend(side_runs(&samples, &rule, right, [first, last]));
    }
    let text = |s: f64| km_text(alignment.km(s), 3);
    let rows: Vec<GuardrailRun> = runs
        .iter()
        .map(|r| GuardrailRun {
            side: if r.right { "right" } else { "left" }.into(),
            from: r.from,
            to: r.to,
            from_text: text(r.from),
            to_text: text(r.to),
            length: r.to - r.from,
            warranted: r.warranted,
            height: r.height,
            slope: r.slope,
        })
        .collect();
    let total = |right: bool| rows.iter().filter(|r| (r.side == "right") == right).map(|r| r.length).sum();
    let (is_stale, _) = stale(doc, def);
    if is_stale {
        warnings.push(CommandWarning {
            code: codes::GUARDRAIL_STALE.into(),
            message: format!(
                "“{}” otokorkuluğunun koridoru değişti; korkulukları denetleyip koridora yeniden bağlayın.",
                def.name
            ),
            path: Some("guardrail".into()),
        });
    }
    let report = GuardrailReport {
        id: def.id.clone(),
        name: def.name.clone(),
        corridor: def.corridor.clone(),
        revision: def.revision,
        stale: is_stale,
        source: def.rule.source.clone(),
        crosses: stations.len() as u32,
        left: total(false),
        right: total(true),
        runs: rows,
    };
    Ok(Read { report, warnings, sides, runs, alignment })
}

/// A guardrail whose corridor does not read: its names and nothing else.
pub(crate) fn broken(def: &GuardrailDef) -> GuardrailReport {
    GuardrailReport {
        id: def.id.clone(),
        name: def.name.clone(),
        corridor: def.corridor.clone(),
        revision: def.revision,
        stale: false,
        source: def.rule.source.clone(),
        crosses: 0,
        runs: Vec::new(),
        left: 0.0,
        right: 0.0,
    }
}
