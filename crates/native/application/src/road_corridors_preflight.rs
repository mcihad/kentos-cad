//! `road.corridors.preflight` v1 (plan/road.md Y50, docs/adr/0956; ROAD-COR-13): a corridor checked before its
//! surface and volumes are trusted (`kentos_road::corridor_check`): each cross's margin `1 + κ·o` against folding,
//! the sampled sections that cross, the parts of the top surface over each other and the seams between its regions.
//! Reads only. The shared cases are `fixtures/commands/v1/road.corridors.preflight.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CorridorFold, CorridorOverlap, CorridorPreflight, CorridorSeam, CorridorTight, CorridorsPreflight,
};
use kentos_domain::Document;
use kentos_road::alignment::km_text;
use kentos_road::corridor_check::{margins, overlaps, seams};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_corridors as common;
use crate::road_terrains::fail;

/// Two crosses at a seam meet within this (m) where none is asked.
pub const SEAM: f64 = 1e-3;

/// A feature code in words: its side and point (“right.hinge” → “sağ şev başı”, “left.c2” → “sol 2. bileşen”,
/// “left.bench1Top” → “sol 1. basamak içi”, docs/adr/0958).
pub fn code_words(code: &str) -> String {
    let (side, point) = code.split_once('.').map_or(("", code), |(s, p)| (s, p));
    let side = match side {
        "left" => "sol ",
        "right" => "sağ ",
        _ => "",
    };
    let point = match point {
        "axis" => "eksen".to_owned(),
        "hinge" => "şev başı".to_owned(),
        "daylight" => "şev ucu".to_owned(),
        "ditchToe" => "hendek ayağı".to_owned(),
        "ditchBottom" => "hendek tabanı".to_owned(),
        p if p.starts_with('c') && p[1..].parse::<u32>().is_ok() => format!("{}. bileşen", &p[1..]),
        p if p.strip_prefix("bench").and_then(|r| r.strip_suffix("Top")).is_some_and(|k| k.parse::<u32>().is_ok()) => {
            format!("{}. basamak içi", &p[5..p.len() - 3])
        }
        p if p.strip_prefix("bench").and_then(|r| r.strip_suffix("End")).is_some_and(|k| k.parse::<u32>().is_ok()) => {
            format!("{}. basamak ucu", &p[5..p.len() - 3])
        }
        p => p.to_owned(),
    };
    format!("{side}{point}")
}

fn bad(words: String, path: &str) -> Stop {
    fail(codes::INVALID_PREFLIGHT, format!("Koridor ön denetimi sorusu geçerli değil: {words}"), path)
}

/// The check now, and its warnings.
pub(crate) fn answer(doc: &Document, input: &CorridorsPreflight) -> Result<(CorridorPreflight, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let (_, def) = common::find(doc.road_corridors(), &input.corridor, "corridor")?;
    if input.margin.is_some_and(|m| !(m.is_finite() && (0.0..1.0).contains(&m))) {
        return Err(bad("istenen pay 0 ile 1 arası olur (1: hiç daralmayan).".into(), "margin"));
    }
    if input.tolerance.is_some_and(|t| !(t.is_finite() && t > 0.0 && t <= 10.0)) {
        return Err(bad("dikiş toleransı 0'dan büyük, en çok 10 m olur.".into(), "tolerance"));
    }
    let made = common::make(doc, def, false)?;
    let al = &made.sampler.alignment;
    let text = |s: f64| km_text(al.km(s), 3);
    let curvature = |s: f64| al.core.at(s).map(|l| l.curvature);
    let limit = input.margin.unwrap_or(0.0);
    let tolerance = input.tolerance.unwrap_or(SEAM);
    // Margins: the least, and the stretches of consecutive crosses at or below the limit.
    let all = margins(&made.regions, &curvature, al.offset);
    let least = all.iter().min_by(|a, b| a.margin.total_cmp(&b.margin));
    let mut tight: Vec<CorridorTight> = Vec::new();
    let mut open = false;
    let mut prev_region = usize::MAX;
    for m in &all {
        if m.margin > limit {
            open = false;
            continue;
        }
        let radius = (m.curvature.abs() > 1e-12).then(|| 1.0 / m.curvature.abs());
        match tight.last_mut() {
            Some(t) if open && prev_region == m.region => {
                t.to = m.station;
                t.to_text = text(m.station);
                if m.margin < t.least {
                    t.least = m.margin;
                    t.at = m.station;
                    t.offset = m.offset;
                    t.radius = radius;
                }
            }
            _ => tight.push(CorridorTight {
                region: m.region as u32,
                from: m.station,
                to: m.station,
                from_text: text(m.station),
                to_text: text(m.station),
                least: m.margin,
                at: m.station,
                offset: m.offset,
                radius,
            }),
        }
        open = true;
        prev_region = m.region;
    }
    let folds: Vec<CorridorFold> = made
        .built
        .folded
        .iter()
        .map(|f| CorridorFold { region: f.0 as u32, from: f.1, to: f.2, from_text: text(f.1), to_text: text(f.2) })
        .collect();
    let over: Vec<CorridorOverlap> = overlaps(&made.built)
        .into_iter()
        .map(|o| CorridorOverlap {
            first: o.first,
            second: o.second,
            first_text: [text(o.first[0]), text(o.first[1])],
            second_text: [text(o.second[0]), text(o.second[1])],
            area: o.area,
            least: o.least,
            most: o.most,
        })
        .collect();
    let joins: Vec<CorridorSeam> = seams(&made.regions)
        .into_iter()
        .map(|s| CorridorSeam {
            station: s.station,
            km: text(s.station),
            before: s.before as u32,
            after: s.after as u32,
            shift: s.shift,
            shift_code: s.shift_code,
            step: s.step,
            step_code: s.step_code,
            meets: s.shift <= tolerance && s.step <= tolerance && s.only_before.is_empty() && s.only_after.is_empty(),
            only_before: s.only_before,
            only_after: s.only_after,
        })
        .collect();
    let mut warnings = made.warnings;
    let name = &def.name;
    if !tight.is_empty() {
        let words: Vec<String> = tight
            .iter()
            .take(6)
            .map(|t| {
                let km = if t.from == t.to { format!("km {}", t.from_text) } else { format!("km {} – {}", t.from_text, t.to_text) };
                let r = t.radius.map_or_else(String::new, |r| format!(", R {r:.1} m"));
                format!("{km} (pay {:.3}, {:.2} m ofsette{r})", t.least, t.offset)
            })
            .collect();
        let more = if tight.len() > 6 { format!(" ve {} aralık daha", tight.len() - 6) } else { String::new() };
        warnings.push(CommandWarning {
            code: codes::CORRIDOR_TIGHT.into(),
            message: format!(
                "“{name}” koridorunda kesit kurbun iç tarafında yarıçapa {}: {}{more}.",
                if limit > 0.0 { format!("yaklaşıyor (pay {limit:.2} altında)") } else { "ulaşıyor ya da onu aşıyor".into() },
                words.join("; ")
            ),
            path: Some("margin".into()),
        });
    }
    if !over.is_empty() {
        let words: Vec<String> = over
            .iter()
            .take(6)
            .map(|o| {
                format!(
                    "km {} – {} ile km {} – {} ({:.1} m², aradaki kot {:.2}–{:.2} m)",
                    o.first_text[0], o.first_text[1], o.second_text[0], o.second_text[1], o.area, o.least, o.most
                )
            })
            .collect();
        let more = if over.len() > 6 { format!(" ve {} yer daha", over.len() - 6) } else { String::new() };
        warnings.push(CommandWarning {
            code: codes::CORRIDOR_OVERLAP.into(),
            message: format!("“{name}” koridorunun üst yüzeyi kendi üstüne biniyor: {}{more}.", words.join("; ")),
            path: Some("corridor".into()),
        });
    }
    let apart: Vec<&CorridorSeam> = joins.iter().filter(|s| !s.meets).collect();
    if !apart.is_empty() {
        let words: Vec<String> = apart
            .iter()
            .map(|s| {
                let mut parts = Vec::new();
                if s.step > tolerance {
                    parts.push(format!("kotta {:.3} m ({})", s.step, code_words(s.step_code.as_deref().unwrap_or_default())));
                }
                if s.shift > tolerance {
                    parts.push(format!("planda {:.3} m ({})", s.shift, code_words(s.shift_code.as_deref().unwrap_or_default())));
                }
                if !s.only_before.is_empty() {
                    parts.push(format!("yalnız öncekinde {}", s.only_before.iter().map(|c| code_words(c)).collect::<Vec<_>>().join(", ")));
                }
                if !s.only_after.is_empty() {
                    parts.push(format!("yalnız sonrakinde {}", s.only_after.iter().map(|c| code_words(c)).collect::<Vec<_>>().join(", ")));
                }
                format!("km {} ({}. ve {}. bölge: {})", s.km, s.before + 1, s.after + 1, parts.join(", "))
            })
            .collect();
        warnings.push(CommandWarning {
            code: codes::CORRIDOR_SEAM.into(),
            message: format!("“{name}” koridorunun bölgeleri buluşmuyor: {}.", words.join("; ")),
            path: Some("regions".into()),
        });
    }
    let clear = tight.is_empty() && folds.is_empty() && over.is_empty() && apart.is_empty();
    let out = CorridorPreflight {
        corridor: def.id.clone(),
        crosses: made.regions.iter().map(|r| r.crosses.iter().filter(|c| c.section.is_some()).count()).sum::<usize>() as u32,
        strips: made.built.strips.len() as u32,
        triangles: made.built.triangles.len() as u32,
        least: least.map(|m| m.margin),
        least_at: least.map(|m| m.station),
        tight,
        folds,
        overlaps: over,
        seams: joins,
        clear,
    };
    Ok((out, warnings))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &CorridorsPreflight) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &CorridorsPreflight) -> CommandResult<CorridorPreflight> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: CorridorsPreflight) -> CommandResult<CorridorPreflight> {
    plan(cx, &input)
}

/// The check for a window, or why it does not read.
pub fn preview(doc: &Document, input: &CorridorsPreflight) -> Result<(CorridorPreflight, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(common::words)
}
