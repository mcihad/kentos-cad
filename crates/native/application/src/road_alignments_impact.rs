//! `road.alignments.impact` v1 (plan/road.md Y57, docs/adr/0963; ROAD-ALIGN-10): what an alignment's new PIs would do
//! to what hangs on its stations, before they are written (`kentos_road::alignment::impact`): the old stretches off
//! the new line, the new ones off the old, the runs between and their station shift; every station a profile, a
//! section set, a corridor or a superelevation schedule names, mapped or unresolved; the parallel axes that follow;
//! the volumes and haul plans on the corridors it touches. Reads only. The shared cases are
//! `fixtures/commands/v1/road.alignments.impact.json`.

use kentos_commands::{
    AlignmentFollower, AlignmentImpact, AlignmentRun, AlignmentStretch, AlignmentsImpact, CommandResult, CommandWarning,
    ImpactDerived, SectionStations, StationOwner, StationRef, SuperelevationMethod,
};
use kentos_domain::Document;
use kentos_road::alignment::impact;
use kentos_road::alignment::km_text;

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_alignments as common;
use crate::road_terrains::fail;

/// The tolerance where none is asked (m).
pub const TOLERANCE: f64 = 1e-3;
/// A station moves when it shifts by more than this (m).
pub const SHIFT: f64 = 1e-6;

/// A named station's item in words: “from” → “başı”, “region 2 to” → “2. bölge sonu”, “station 3” → “3. km”, “row 1”
/// → “1. satır”, “curve p1” → “p1 kurbu”, a PVI's id → “PVI v2”.
pub fn item_words(owner: StationOwner, item: &str) -> String {
    let n = |rest: &str| rest.split_whitespace().next().unwrap_or("").to_owned();
    if let Some(rest) = item.strip_prefix("region ") {
        let end = if rest.ends_with(" from") { "başı" } else { "sonu" };
        return format!("{}. bölge {end}", n(rest));
    }
    if let Some(rest) = item.strip_prefix("station ") {
        return format!("{}. km", n(rest));
    }
    if let Some(rest) = item.strip_prefix("row ") {
        return format!("{}. satır", n(rest));
    }
    if let Some(rest) = item.strip_prefix("curve ") {
        return format!("{rest} kurbu");
    }
    match (owner, item) {
        (StationOwner::Profile, _) => format!("PVI {item}"),
        (_, "from") => "başı".into(),
        (_, "to") => "sonu".into(),
        _ => item.to_owned(),
    }
}

fn answer(doc: &Document, input: &AlignmentsImpact) -> Result<(AlignmentImpact, Vec<CommandWarning>), Stop> {
    crate::road_profiles::road_project(doc)?;
    let own = doc.road_alignments();
    let (at, old) = common::find(own, &input.alignment, "alignment")?;
    if old.offset_of.is_some() {
        return Err(fail(
            codes::INVALID_ALIGNMENT,
            "Paralel aksın PI'ı olmaz: kaynağını izler; kaynağı düzenleyin.".into(),
            "alignment",
        ));
    }
    let tolerance = input.tolerance.unwrap_or(TOLERANCE);
    if !(tolerance.is_finite() && tolerance > 0.0 && tolerance <= 1.0) {
        return Err(fail(codes::INVALID_ALIGNMENT_IMPACT, "Etki toleransı 0'dan büyük, en çok 1 m olur.".into(), "tolerance"));
    }
    // The new PIs judged and built as road.alignments.edit would.
    let before = common::build(own, old, "")?;
    let mut list = own.to_vec();
    list[at].pis = input.pis.clone();
    common::judged(&list, at, "")?;
    let after = common::build(&list, &list[at], "")?;
    let (b, a) = (&before.core, &after.core);
    let moved = impact::moved(b, a, tolerance);
    let old_text = |s: f64| km_text(before.km(s), 3);
    let new_text = |s: f64| km_text(after.km(s), 3);
    let stretch = |st: &impact::Stretch, text: &dyn Fn(f64) -> String| AlignmentStretch {
        from: st.from,
        to: st.to,
        from_text: text(st.from),
        to_text: text(st.to),
        most: st.most,
    };
    let removed: Vec<AlignmentStretch> = moved.removed.iter().map(|s| stretch(s, &old_text)).collect();
    let added: Vec<AlignmentStretch> = moved.added.iter().map(|s| stretch(s, &new_text)).collect();
    let runs: Vec<AlignmentRun> = moved
        .runs
        .iter()
        .map(|r| AlignmentRun {
            from: r.from,
            to: r.to,
            from_text: old_text(r.from),
            to_text: old_text(r.to),
            shift_from: r.shift_from,
            shift_to: r.shift_to,
        })
        .collect();
    // Every station named, kept where it lands on the same station.
    let mut refs: Vec<StationRef> = Vec::new();
    let name = |refs: &mut Vec<StationRef>, owner: StationOwner, id: &str, nm: &str, item: String, s: f64| {
        let to = impact::map(b, a, s, tolerance);
        if to.is_none_or(|t| (t - s).abs() > SHIFT) {
            refs.push(StationRef { owner, id: id.to_owned(), name: nm.to_owned(), item, station: s, to });
        }
    };
    for p in doc.road_profiles().iter().filter(|p| p.alignment == old.id) {
        for v in &p.pvis {
            name(&mut refs, StationOwner::Profile, &p.id, &p.name, v.id.clone(), v.station);
        }
    }
    for d in doc.road_sections().iter().filter(|d| d.alignment == old.id) {
        match &d.stations {
            SectionStations::Range { from, to, .. } | SectionStations::Breaks { from, to, .. } => {
                name(&mut refs, StationOwner::SectionSet, &d.id, &d.name, "from".into(), *from);
                name(&mut refs, StationOwner::SectionSet, &d.id, &d.name, "to".into(), *to);
            }
            SectionStations::Stations { stations } => {
                for (k, s) in stations.iter().enumerate() {
                    name(&mut refs, StationOwner::SectionSet, &d.id, &d.name, format!("station {}", k + 1), *s);
                }
            }
            // Where drawn lines cross the alignment: they cross the new line where they cross it, no station is named.
            SectionStations::Lines { .. } => {}
        }
    }
    let mut touched: Vec<String> = Vec::new();
    for c in doc.road_corridors().iter().filter(|c| c.alignment == old.id) {
        let n = refs.len();
        for (k, r) in c.regions.iter().enumerate() {
            name(&mut refs, StationOwner::Corridor, &c.id, &c.name, format!("region {} from", k + 1), r.from);
            name(&mut refs, StationOwner::Corridor, &c.id, &c.name, format!("region {} to", k + 1), r.to);
        }
        for (k, s) in c.stations.iter().enumerate() {
            name(&mut refs, StationOwner::Corridor, &c.id, &c.name, format!("station {}", k + 1), *s);
        }
        let over = moved.removed.iter().any(|st| c.regions.iter().any(|r| st.from < r.to && st.to > r.from));
        if over || refs.len() > n {
            touched.push(c.id.clone());
        }
    }
    for d in doc.road_superelevations().iter().filter(|d| d.alignment == old.id) {
        match &d.method {
            SuperelevationMethod::Stations { rows } => {
                for (k, r) in rows.iter().enumerate() {
                    name(&mut refs, StationOwner::Superelevation, &d.id, &d.name, format!("row {}", k + 1), r.station);
                }
            }
            // A curve's rule follows its PI: told only where the PI had a curve and has none now (a rule on a PI
            // that never had one does not build before the edit either).
            SuperelevationMethod::Curves { curves } => {
                for c in curves {
                    if let Some(was) = b.curves.iter().find(|x| x.pi == c.pi)
                        && !a.curves.iter().any(|x| x.pi == c.pi)
                    {
                        let s = was.stations[0];
                        refs.push(StationRef {
                            owner: StationOwner::Superelevation,
                            id: d.id.clone(),
                            name: d.name.clone(),
                            item: format!("curve {}", c.pi),
                            station: s,
                            to: None,
                        });
                    }
                }
            }
        }
    }
    let parallels: Vec<AlignmentFollower> = own
        .iter()
        .filter(|d| d.offset_of.as_ref().is_some_and(|o| o.alignment == old.id))
        .map(|d| AlignmentFollower { id: d.id.clone(), name: d.name.clone() })
        .collect();
    let volumes: Vec<ImpactDerived> = doc
        .road_volumes()
        .iter()
        .filter(|v| touched.contains(&v.corridor))
        .map(|v| ImpactDerived { id: v.id.clone(), name: v.name.clone(), approved: v.approved.is_some() })
        .collect();
    let hauls: Vec<ImpactDerived> = doc
        .road_hauls()
        .iter()
        .filter(|h| volumes.iter().any(|v| v.id == h.volume))
        .map(|h| ImpactDerived { id: h.id.clone(), name: h.name.clone(), approved: h.approved.is_some() })
        .collect();
    let mut warnings = Vec::new();
    let lost: Vec<String> =
        refs.iter().filter(|r| r.to.is_none()).map(|r| format!("{} {}", r.name, item_words(r.owner, &r.item))).collect();
    if !lost.is_empty() {
        let shown: Vec<String> = lost.iter().take(8).cloned().collect();
        let more = if lost.len() > 8 { format!(" ve {} yer daha", lost.len() - 8) } else { String::new() };
        warnings.push(CommandWarning {
            code: codes::ALIGNMENT_IMPACT_UNRESOLVED.into(),
            message: format!(
                "“{}” aksının yeni PI'larıyla şu km'lerin yeri aksın üstünde kalmıyor: {}{more}. Kaydedilirse bunlar elle yeniden verilmeli.",
                old.name,
                shown.join(", ")
            ),
            path: Some("pis".into()),
        });
    }
    let approved: Vec<String> = volumes
        .iter()
        .filter(|v| v.approved)
        .map(|v| format!("“{}” kübajı", v.name))
        .chain(hauls.iter().filter(|h| h.approved).map(|h| format!("“{}” taşıma planı", h.name)))
        .collect();
    if !approved.is_empty() {
        warnings.push(CommandWarning {
            code: codes::ALIGNMENT_IMPACT_APPROVED.into(),
            message: format!(
                "“{}” aksının yeni PI'ları onaylı hesapları değiştirir: {}. Yazılınca onayları eskir; yeni bir hesap onaylanmalı.",
                old.name,
                approved.join(", ")
            ),
            path: Some("pis".into()),
        });
    }
    let clear = removed.is_empty() && added.is_empty() && refs.is_empty();
    let out = AlignmentImpact {
        alignment: old.id.clone(),
        mapping: common::mapping_rows(Some(b), Some(a)),
        removed,
        added,
        runs,
        refs,
        parallels,
        volumes,
        hauls,
        clear,
    };
    Ok((out, warnings))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &AlignmentsImpact) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &AlignmentsImpact) -> CommandResult<AlignmentImpact> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: AlignmentsImpact) -> CommandResult<AlignmentImpact> {
    plan(cx, &input)
}

/// The impact of the PIs a window is typing, or why it does not read.
pub fn preview(doc: &Document, input: &AlignmentsImpact) -> Result<(AlignmentImpact, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}
