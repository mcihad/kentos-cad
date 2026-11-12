//! `road.stakeout.query` v1 (plan/road.md Y10, docs/adr/0908; ROAD-SEC-09): a corridor's coded section points at its
//! stations, or every `step` metres: station, km, region, code, side, offset, plan place and elevation; the offsets on
//! the grid or also as ground lengths. Reads only. The shared cases are `fixtures/commands/v1/road.stakeout.query.json`.

use kentos_commands::{
    CommandResult, CommandWarning, StakeRow, StakeSide, StakeoutDistances, StakeoutQueried, StakeoutQuery,
};
use kentos_domain::Document;
use kentos_geometry_core::crs::ground::{Grid, grid_factor};
use kentos_road::alignment::km_text;
use kentos_road::corridor::{Cross, stake_points};
use kentos_road::section::template::{Side, Template};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_corridors::{self as corridors, words};
use crate::road_terrains::fail;

/// The most stations, and points, a table reads.
pub const MOST_STATIONS: usize = 20_000;
pub const MOST_POINTS: usize = 200_000;

fn bad(path: &str, words: String) -> Stop {
    fail(codes::INVALID_STAKEOUT, words, path)
}

fn vec2(p: [f64; 2]) -> kentos_geometry_core::Vec2 {
    kentos_geometry_core::Vec2 { x: p[0], y: p[1] }
}

/// A number as Rust writes an f64.
fn num(v: f64) -> String {
    format!("{v}")
}

/// The code words a side of any of `templates` gives, in a fixed order.
fn side_words(templates: &[&Template], right: bool) -> Vec<String> {
    let sides: Vec<&Side> = templates.iter().map(|t| if right { &t.right } else { &t.left }).collect();
    let n = sides.iter().map(|s| s.components.len()).max().unwrap_or(0);
    let mut out: Vec<String> = (1..=n).map(|i| format!("c{i}")).collect();
    if sides.iter().any(|s| s.ditch.is_some()) {
        out.push("ditchToe".into());
    }
    if sides.iter().any(|s| s.ditch.is_some_and(|d| d.bottom > 0.0)) {
        out.push("ditchBottom".into());
    }
    out.extend(["hinge".to_owned(), "daylight".to_owned()]);
    out
}

/// The stations of a region between `lo` and `hi`: the range's ends and the multiples of `step` strictly between.
pub(crate) fn stepped(lo: f64, hi: f64, step: f64) -> Vec<f64> {
    let mut out = vec![lo];
    let mut k = (lo / step).floor();
    loop {
        let s = k * step;
        if s >= hi - 1e-9 {
            break;
        }
        if s > lo + 1e-9 {
            out.push(s);
        }
        k += 1.0;
    }
    if hi > lo + 1e-9 {
        out.push(hi);
    }
    out
}

fn answer(doc: &Document, input: &StakeoutQuery) -> Result<(StakeoutQueried, Vec<CommandWarning>), Stop> {
    table(doc, input).map(|(t, w, _)| (t, w))
}

/// The table, its warnings and the alignment the corridor lies on (built as the corridor reads it).
pub(crate) fn table(
    doc: &Document,
    input: &StakeoutQuery,
) -> Result<(StakeoutQueried, Vec<CommandWarning>, crate::road_alignments::Built), Stop> {
    table_told(doc, input, &|_, _| false)
}

/// [`table`] telling `tick` how far it has come (`done` of 2 × [`corridors::TICKS`], docs/adr/4860): its corridor's
/// sections the first half, the crosses read every `step` metres the second; `tick` answering true stops it there,
/// cancelled. The table is [`table`]'s.
pub(crate) fn table_told(
    doc: &Document,
    input: &StakeoutQuery,
    tick: &dyn Fn(u64, u64) -> bool,
) -> Result<(StakeoutQueried, Vec<CommandWarning>, crate::road_alignments::Built), Stop> {
    let whole = 2 * corridors::TICKS;
    corridors::road_project(doc)?;
    let (_, def) = corridors::find(doc.road_corridors(), &input.corridor, "corridor")?;
    let first_half = |done: u64, all: u64| tick(if all == 0 { 0 } else { done * corridors::TICKS / all }, whole);
    let made = corridors::make_ticked(doc, def, false, &first_half).map_err(|stop| match stop {
        Stop::Cancelled => Stop::Cancelled,
        stop => bad("corridor", format!("“{}” koridoru kurulamıyor: {}", def.name, words(stop))),
    })?;
    let first = def.regions.iter().map(|r| r.from).fold(f64::INFINITY, f64::min);
    let last = def.regions.iter().map(|r| r.to).fold(f64::NEG_INFINITY, f64::max);
    let from = input.from.unwrap_or(first);
    let to = input.to.unwrap_or(last);
    let span = format!("koridor {} ile {} m arası", num(first), num(last));
    if !from.is_finite() || from < first - 1e-9 || from > last + 1e-9 {
        return Err(bad("from", format!("{} m koridorun üstünde değil: {span}.", num(from))));
    }
    if !to.is_finite() || to < first - 1e-9 || to > last + 1e-9 {
        return Err(bad("to", format!("{} m koridorun üstünde değil: {span}.", num(to))));
    }
    if to < from {
        return Err(bad("to", "Aralığın sonu başından önce olamaz.".into()));
    }
    if let Some(step) = input.step
        && !(0.1..=10_000.0).contains(&step)
    {
        return Err(bad("step", "Aralık 0,1 ile 10 000 m arası olmalı.".into()));
    }
    let templates: Vec<&Template> = made.regions.iter().map(|r| &r.template).collect();
    let (left, right) = (side_words(&templates, false), side_words(&templates, true));
    let wanted = input.codes.clone().unwrap_or_default();
    for (i, q) in wanted.iter().enumerate() {
        let known = q == "axis"
            || left.contains(q)
            || right.contains(q)
            || q.strip_prefix("left.").is_some_and(|w| left.contains(&w.to_owned()))
            || q.strip_prefix("right.").is_some_and(|w| right.contains(&w.to_owned()));
        if !known {
            let all: Vec<String> = std::iter::once("axis".to_owned())
                .chain(left.iter().map(|w| format!("left.{w}")))
                .chain(right.iter().map(|w| format!("right.{w}")))
                .collect();
            return Err(bad(
                &format!("codes[{i}]"),
                format!("“{q}” kodunu koridorun hiçbir tip kesiti vermez; kodlar: {}.", all.join(", ")),
            ));
        }
    }
    let keep = |code: &str| {
        wanted.is_empty() || wanted.iter().any(|q| code == q || code.strip_suffix(q.as_str()).is_some_and(|h| h.ends_with('.')))
    };
    // The crosses read: each region's own in the range, or one every `step` metres (those told one at a time in the
    // second half, docs/adr/4860).
    let in_range = |r: &kentos_commands::CorridorRegionDef, step: f64| {
        let (lo, hi) = (from.max(r.from), to.min(r.to));
        if lo > hi + 1e-9 { 0 } else { stepped(lo, hi, step).len() as u64 }
    };
    let all = input.step.map_or(0, |step| def.regions.iter().map(|r| in_range(r, step)).sum::<u64>());
    let mut told = 0u64;
    let mut crosses: Vec<(usize, Cross)> = Vec::new();
    for (ri, (region, r)) in made.regions.iter().zip(&def.regions).enumerate() {
        match input.step {
            None => crosses.extend(
                region
                    .crosses
                    .iter()
                    .filter(|c| c.line.station >= from - 1e-9 && c.line.station <= to + 1e-9)
                    .map(|c| (ri, c.clone())),
            ),
            Some(step) => {
                let (lo, hi) = (from.max(r.from), to.min(r.to));
                if lo > hi + 1e-9 {
                    continue;
                }
                let stations = stepped(lo, hi, step);
                if crosses.len() + stations.len() > MOST_STATIONS {
                    return Err(bad("step", format!("Bu aralıkla {MOST_STATIONS}'den çok km çıkar; step'i büyütün.")));
                }
                for s in stations {
                    if tick(corridors::TICKS + told * corridors::TICKS / all.max(1), whole) {
                        return Err(Stop::Cancelled);
                    }
                    told += 1;
                    if let Some(c) = made.sampler.cross(&made.template_at(ri, s), s) {
                        crosses.push((ri, c));
                    }
                }
            }
        }
    }
    if crosses.len() > MOST_STATIONS {
        return Err(bad("from", format!("Bu aralıkta {MOST_STATIONS}'den çok km var; aralığı daraltın.")));
    }
    let height = doc.settings().ground_height();
    let grid = match input.distances {
        StakeoutDistances::Grid => None,
        StakeoutDistances::Ground => {
            let Some(h) = height else {
                return Err(fail(
                    codes::STAKEOUT_NO_GROUND,
                    "Zemin boyu için projenin ortalama elipsoit yüksekliği yazılmalı (Proje ayarları › Ölçme).".into(),
                    "distances",
                ));
            };
            let Some(system) = kentos_project::systems::own(doc.settings()).and_then(|n| n.system) else {
                return Err(fail(
                    codes::STAKEOUT_NO_GROUND,
                    "Projenin koordinat sistemi yok: zemin boyu hesaplanamaz.".into(),
                    "distances",
                ));
            };
            Some(Grid { system, height: h })
        }
    };
    let text = |s: f64| km_text(made.sampler.alignment.km(s), 3);
    let mut rows = Vec::new();
    let mut empty = Vec::new();
    for (ri, c) in &crosses {
        let s = c.line.station;
        if c.section.is_none() {
            empty.push(s);
            continue;
        }
        for p in stake_points(&made.regions[*ri].template, c) {
            if !keep(&p.code) {
                continue;
            }
            let side = match p.code.split_once('.') {
                Some(("left", _)) => StakeSide::Left,
                Some(("right", _)) => StakeSide::Right,
                _ => StakeSide::Axis,
            };
            let ground_offset = match &grid {
                None => None,
                Some(_) if p.offset == 0.0 => Some(0.0),
                Some(g) => {
                    let f = grid_factor(g, vec2(c.line.at), vec2(p.at))
                        .map_err(|why| fail(codes::STAKEOUT_NO_GROUND, why, "distances"))?;
                    Some(p.offset / f.combined())
                }
            };
            rows.push(StakeRow {
                station: s,
                km_text: text(s),
                region: *ri as u32,
                code: p.code,
                side,
                offset: p.offset,
                x: p.at[0],
                y: p.at[1],
                z: p.z,
                ground_offset,
            });
            if rows.len() > MOST_POINTS {
                return Err(bad("codes", format!("Cetvel {MOST_POINTS}'den çok nokta verir; aralığı ya da kodları daraltın.")));
            }
        }
    }
    let mut warnings = made.warnings;
    if !empty.is_empty() {
        let shown: Vec<String> = empty.iter().take(8).map(|s| format!("km {}", text(*s))).collect();
        let more = if empty.len() > 8 { format!(" ve {} km daha", empty.len() - 8) } else { String::new() };
        warnings.push(CommandWarning {
            code: codes::STAKEOUT_UNSOLVED.into(),
            message: format!(
                "“{}” koridorunun aplikasyon cetvelinde kesitsiz km'ler: {}{more}; oralarda nokta yok.",
                def.name,
                shown.join(", ")
            ),
            path: Some("corridor".into()),
        });
    }
    let method = match (&grid, height) {
        (Some(_), Some(h)) => format!(
            "Ötelemeler plan boyu; zemin boyu her ötelemenin eksenden noktaya çizginin ölçeği ve {} m ortalama elipsoit yüksekliğinin çarpanına bölünmesiyle.",
            num(h)
        ),
        _ => "Ötelemeler plan (ızgara) boyu.".into(),
    };
    Ok((
        StakeoutQueried {
            corridor: def.id.clone(),
            name: def.name.clone(),
            distances: input.distances,
            method,
            stations: crosses.len() as u32,
            rows,
            sources: sources_of(doc, def),
        },
        warnings,
        made.sampler.alignment,
    ))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &StakeoutQuery) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The table; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &StakeoutQuery) -> CommandResult<StakeoutQueried> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The table; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: StakeoutQuery) -> CommandResult<StakeoutQueried> {
    plan(cx, &input)
}

/// The table for a window: the answer or why there is none.
pub fn preview(doc: &Document, input: &StakeoutQuery) -> Result<(StakeoutQueried, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(words)
}

/// What the corridor's stakeout is computed from, each as it is now (docs/adr/1038, 1040): the corridor's chain and its
/// earthwork volumes.
fn sources_of(doc: &Document, def: &kentos_commands::CorridorDef) -> kentos_commands::StakeoutSources {
    let chain = crate::road_sources::corridor(doc, def);
    kentos_commands::StakeoutSources {
        corridor: chain.corridor,
        alignment: chain.alignment,
        profile: chain.profile,
        templates: chain.templates,
        superelevation: chain.superelevation,
        surface: Some(chain.surface),
        volumes: doc
            .road_volumes()
            .iter()
            .filter(|v| v.corridor == def.id)
            .map(|v| kentos_commands::StakeoutVolume { id: v.id.clone(), revision: v.revision, approved: v.approved.is_some() })
            .collect(),
    }
}

/// The stakeout's sources and when the table was written, as a sheet beside it (docs/adr/1038): a row a source.
pub fn sources_sheet(table: &StakeoutQueried, written: &str) -> kentos_commands::WorkbookSheet {
    use kentos_commands::{CellKind, WorkbookColumn};
    let column = |name: &str| WorkbookColumn { name: name.into(), kind: CellKind::Text, decimals: None };
    let s = &table.sources;
    let row = |what: &str, id: &str, revision: String| vec![what.to_owned(), id.to_owned(), revision];
    let mut rows = vec![
        row("Üretildi", written, String::new()),
        row("Koridor", &s.corridor.id, s.corridor.revision.to_string()),
        row("Aks", &s.alignment.id, s.alignment.revision.to_string()),
        row("Boykesit", &s.profile.id, s.profile.revision.to_string()),
    ];
    rows.extend(s.templates.iter().map(|t| row("Tip kesit", &t.id, t.revision.to_string())));
    if let Some(d) = &s.superelevation {
        rows.push(row("Dever", &d.id, d.revision.to_string()));
    }
    if let Some(g) = &s.surface {
        rows.push(row("Arazi", &g.id, crate::road_sources::surface_words(g)));
    }
    rows.extend(s.volumes.iter().map(|v| {
        row("Kübaj", &v.id, format!("{}{}", v.revision, if v.approved { " (onaylı)" } else { "" }))
    }));
    kentos_commands::WorkbookSheet {
        name: "Kaynaklar".into(),
        columns: vec![column("Kaynak"), column("Kimlik"), column("Sürüm")],
        rows,
    }
}

/// The stakeout table as a workbook's sheet (docs/adr/0223's writer): km, region, code, side, offset (and its ground
/// length where the table has it), Y (east), X (north) and Z, in the table's order.
pub fn sheet(table: &StakeoutQueried) -> kentos_commands::WorkbookSheet {
    use kentos_commands::{CellKind, WorkbookColumn};
    let column = |name: &str, kind: CellKind, decimals: Option<u32>| WorkbookColumn { name: name.into(), kind, decimals };
    let ground = table.distances == StakeoutDistances::Ground;
    let mut columns = vec![
        column("Km", CellKind::Text, None),
        column("Bölge", CellKind::Number, Some(0)),
        column("Kod", CellKind::Text, None),
        column("Yan", CellKind::Text, None),
        column("Öteleme (m)", CellKind::Number, Some(3)),
    ];
    if ground {
        columns.push(column("Zemin ötelemesi (m)", CellKind::Number, Some(3)));
    }
    columns.extend([
        column("Y (m)", CellKind::Number, Some(3)),
        column("X (m)", CellKind::Number, Some(3)),
        column("Z (m)", CellKind::Number, Some(3)),
    ]);
    let side = |s: StakeSide| match s {
        StakeSide::Left => "sol",
        StakeSide::Axis => "eksen",
        StakeSide::Right => "sağ",
    };
    let d = |v: f64| format!("{v:.3}");
    let rows = table
        .rows
        .iter()
        .map(|r| {
            let mut cells = vec![r.km_text.clone(), (r.region + 1).to_string(), r.code.clone(), side(r.side).into(), d(r.offset)];
            if ground {
                cells.push(r.ground_offset.map(d).unwrap_or_default());
            }
            cells.extend([d(r.x), d(r.y), d(r.z)]);
            cells
        })
        .collect();
    kentos_commands::WorkbookSheet { name: format!("{} aplikasyon", table.name), columns, rows }
}
