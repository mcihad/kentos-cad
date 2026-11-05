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
/// → “1. satır”, “curve p1” → “p1 kurbu”, a PVI's id → “PVI v2”, “crossing Köprü” → “Köprü kesişmesi”, “factor 1 to” →
/// “1. katsayı satırı sonu”, “strip 2 from” → “2. sıyırma aralığı başı”, “site D1” → “D1 alanı”, “unsuitable 1 from” →
/// “1. uygunsuz aralık başı”, “barrier Dere” → “Dere engeli”, a culvert's “station” → “km”.
pub fn item_words(owner: StationOwner, item: &str) -> String {
    let n = |rest: &str| rest.split_whitespace().next().unwrap_or("").to_owned();
    let end = |rest: &str| if rest.ends_with(" from") { "başı" } else { "sonu" };
    for (key, words) in [
        ("region ", "bölge"),
        ("factor ", "katsayı satırı"),
        ("strip ", "sıyırma aralığı"),
        ("unsuitable ", "uygunsuz aralık"),
    ] {
        if let Some(rest) = item.strip_prefix(key) {
            return format!("{}. {words} {}", n(rest), end(rest));
        }
    }
    if let Some(rest) = item.strip_prefix("station ") {
        return format!("{}. km", n(rest));
    }
    if let Some(rest) = item.strip_prefix("row ") {
        return format!("{}. satır", n(rest));
    }
    for (key, words) in [("curve ", "kurbu"), ("crossing ", "kesişmesi"), ("site ", "alanı"), ("barrier ", "engeli")] {
        if let Some(rest) = item.strip_prefix(key) {
            return format!("{rest} {words}");
        }
    }
    match (owner, item) {
        (StationOwner::Profile, _) => format!("PVI {item}"),
        (_, "from") => "başı".into(),
        (_, "to") => "sonu".into(),
        (_, "station") => "km".into(),
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
    // The new PIs, or the alignment turned round, judged and built as road.alignments.edit would.
    let before = common::build(own, old, "")?;
    let mut list = own.to_vec();
    if input.reverse {
        if !input.pis.is_empty() {
            return Err(fail(codes::INVALID_ALIGNMENT_IMPACT, "Ya yeni PI'lar ya ters çevirme sorulur; ikisi birden değil.".into(), "pis"));
        }
        let def = &mut list[at];
        def.pis.reverse();
        for p in &mut def.pis {
            std::mem::swap(&mut p.spiral_in, &mut p.spiral_out);
        }
        if let Some(st) = &mut def.stationing {
            st.equations.clear();
        }
    } else {
        list[at].pis = input.pis.clone();
    }
    common::judged(&list, at, "")?;
    let after = common::build(&list, &list[at], "")?;
    let (b, a) = (&before.core, &after.core);
    let length = b.length;
    let old_text = |s: f64| km_text(before.km(s), 3);
    let new_text = |s: f64| km_text(after.km(s), 3);
    let stretch = |st: &impact::Stretch, text: &dyn Fn(f64) -> String| AlignmentStretch {
        from: st.from,
        to: st.to,
        from_text: text(st.from),
        to_text: text(st.to),
        most: st.most,
    };
    // Turned round, the line is where it was: every station s is at L − s (docs/adr/0965), nothing leaves the line.
    let (removed, added, runs, mapping) = if input.reverse {
        let run = AlignmentRun { from: 0.0, to: length, from_text: old_text(0.0), to_text: old_text(length), shift_from: length, shift_to: -length };
        let rows = a
            .elements
            .iter()
            .map(|e| kentos_commands::StationMapRow {
                element: e.id.clone(),
                change: "reversed".into(),
                before: Some([length - e.end_station(), length - e.station]),
                after: Some([e.station, e.end_station()]),
            })
            .collect();
        (Vec::new(), Vec::new(), vec![run], rows)
    } else {
        let moved = impact::moved(b, a, tolerance);
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
        (removed, added, runs, common::mapping_rows(Some(b), Some(a)))
    };
    let lands = |s: f64| if input.reverse { Some(length - s) } else { impact::map(b, a, s, tolerance) };
    // Every station named, kept where it lands on the same station; a reversal writes the profiles' and the
    // schedules' itself (carried).
    let mut refs: Vec<StationRef> = Vec::new();
    let name = |refs: &mut Vec<StationRef>, owner: StationOwner, id: &str, nm: &str, item: String, s: f64| {
        let to = lands(s);
        if to.is_none_or(|t| (t - s).abs() > SHIFT) {
            let carried = input.reverse && matches!(owner, StationOwner::Profile | StationOwner::Superelevation);
            refs.push(StationRef { owner, id: id.to_owned(), name: nm.to_owned(), item, station: s, to, carried });
        }
    };
    for p in doc.road_profiles().iter().filter(|p| p.alignment == old.id) {
        for v in &p.pvis {
            name(&mut refs, StationOwner::Profile, &p.id, &p.name, v.id.clone(), v.station);
        }
        for c in &p.crossings {
            name(&mut refs, StationOwner::Profile, &p.id, &p.name, format!("crossing {}", c.name), c.station);
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
    let mine: Vec<&kentos_commands::CorridorDef> = doc.road_corridors().iter().filter(|c| c.alignment == old.id).collect();
    for c in &mine {
        let n = refs.len();
        for (k, r) in c.regions.iter().enumerate() {
            name(&mut refs, StationOwner::Corridor, &c.id, &c.name, format!("region {} from", k + 1), r.from);
            name(&mut refs, StationOwner::Corridor, &c.id, &c.name, format!("region {} to", k + 1), r.to);
        }
        for (k, s) in c.stations.iter().enumerate() {
            name(&mut refs, StationOwner::Corridor, &c.id, &c.name, format!("station {}", k + 1), *s);
        }
        let over = removed.iter().any(|st| c.regions.iter().any(|r| st.from < r.to && st.to > r.from));
        if over || refs.len() > n {
            touched.push(c.id.clone());
        }
    }
    // What lies on those corridors by station: culverts, walls, volumes' rows and their haul diagrams'
    // sites, unsuitable stretches and barriers (docs/adr/0965).
    let on_mine = |corridor: &str| mine.iter().any(|c| c.id == corridor);
    for d in doc.road_culverts().iter().filter(|d| on_mine(&d.corridor)) {
        name(&mut refs, StationOwner::Culvert, &d.id, &d.name, "station".into(), d.station);
    }
    for d in doc.road_walls().iter().filter(|d| on_mine(&d.corridor)) {
        name(&mut refs, StationOwner::Wall, &d.id, &d.name, "from".into(), d.from);
        name(&mut refs, StationOwner::Wall, &d.id, &d.name, "to".into(), d.to);
    }
    let volumes_here: Vec<&kentos_commands::VolumeDef> = doc.road_volumes().iter().filter(|v| on_mine(&v.corridor)).collect();
    for v in &volumes_here {
        for (k, f) in v.factors.iter().enumerate() {
            name(&mut refs, StationOwner::Volume, &v.id, &v.name, format!("factor {} from", k + 1), f.from);
            name(&mut refs, StationOwner::Volume, &v.id, &v.name, format!("factor {} to", k + 1), f.to);
        }
        for (k, f) in v.strip.iter().enumerate() {
            name(&mut refs, StationOwner::Volume, &v.id, &v.name, format!("strip {} from", k + 1), f.from);
            name(&mut refs, StationOwner::Volume, &v.id, &v.name, format!("strip {} to", k + 1), f.to);
        }
    }
    for h in doc.road_hauls().iter().filter(|h| volumes_here.iter().any(|v| v.id == h.volume)) {
        for s in &h.sites {
            name(&mut refs, StationOwner::Haul, &h.id, &h.name, format!("site {}", s.name), s.station);
        }
        for (k, u) in h.unsuitable.iter().enumerate() {
            name(&mut refs, StationOwner::Haul, &h.id, &h.name, format!("unsuitable {} from", k + 1), u.from);
            name(&mut refs, StationOwner::Haul, &h.id, &h.name, format!("unsuitable {} to", k + 1), u.to);
        }
        for x in &h.barriers {
            name(&mut refs, StationOwner::Haul, &h.id, &h.name, format!("barrier {}", x.name), x.station);
        }
    }
    for d in doc.road_sheet_sets().iter().filter(|d| d.alignment == old.id) {
        name(&mut refs, StationOwner::SheetSet, &d.id, &d.name, "from".into(), d.from);
        name(&mut refs, StationOwner::SheetSet, &d.id, &d.name, "to".into(), d.to);
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
                        refs.push(StationRef {
                            owner: StationOwner::Superelevation,
                            id: d.id.clone(),
                            name: d.name.clone(),
                            item: format!("curve {}", c.pi),
                            station: was.stations[0],
                            to: None,
                            carried: false,
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
    // Turned round, what is not carried goes stale where it is: a corridor's or a section set's template has a left and
    // a right side, and turning them would put the left on the physical right (docs/adr/0965).
    if input.reverse {
        let mut stale: Vec<String> = Vec::new();
        for r in refs.iter().filter(|r| !r.carried) {
            if !stale.contains(&r.name) {
                stale.push(r.name.clone());
            }
        }
        if !stale.is_empty() {
            warnings.push(CommandWarning {
                code: codes::ALIGNMENT_REVERSE_STALE.into(),
                message: format!(
                    "“{}” aksı ters çevrilince boykesitler ve dever çizelgeleri onunla döner; şunlar dönmez, km'leri eski yönde kalır ve eskir: {}.",
                    old.name,
                    stale.join(", ")
                ),
                path: Some("alignment".into()),
            });
        }
    }
    let clear = removed.is_empty() && added.is_empty() && refs.is_empty();
    let out = AlignmentImpact {
        alignment: old.id.clone(),
        mapping,
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
