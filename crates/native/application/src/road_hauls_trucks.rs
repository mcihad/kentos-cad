//! `road.hauls.trucks` v1 (plan/road.md Y32.2, docs/adr/0936 §2; ROAD-HAUL-10): a haul fleet on a diagram's haul plan.
//! The plan's flows split into hauls (`kentos_road::haul_fleet::hauls`), each driven by the user's truck on the
//! road's own heights loaded and back empty (`run`), its cycle, trips and balance number; with a fleet the loader's
//! production deterministically and as the finite-source queue (`loader`), with the fuel figures the fuel and carbon.
//! Reads only. The shared cases are `fixtures/commands/v1/road.hauls.trucks.json`.

use kentos_commands::{
    CommandResult, CommandWarning, HaulTrack, HaulTrucked, HaulsPlan, HaulsTrucks, TruckFleet, TruckHaul, TruckTrace,
};
use kentos_domain::Document;
use kentos_road::alignment::km_text;
use kentos_road::haul_fleet::{Run, Truck, hauls, loader, reversed, run};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_hauls as common;
use crate::road_terrains::fail;

fn bad(path: &str, words: String) -> Stop {
    fail(codes::INVALID_HAUL_TRUCKS, words, path)
}

/// The input's own numbers judged.
fn judged(q: &HaulsTrucks) -> Result<(), Stop> {
    let t = &q.truck;
    let positive = |v: f64| v.is_finite() && v > 0.0;
    for (ok, path, words) in [
        (positive(t.empty), "truck.empty", "Kamyonun boş kütlesi 0'dan büyük olmalı (t)."),
        (t.gross.is_finite() && t.gross > t.empty, "truck.gross", "Kamyonun yüklü kütlesi boş kütlesinden büyük olmalı (t)."),
        (positive(t.power), "truck.power", "Motor gücü 0'dan büyük olmalı (kW)."),
        (positive(t.efficiency) && t.efficiency <= 1.0, "truck.efficiency", "Aktarma verimi 0 ile 1 arası olmalı."),
        (positive(t.top), "truck.top", "En yüksek hız 0'dan büyük olmalı (km/sa)."),
        (t.downhill.is_none_or(positive), "truck.downhill", "İniş hızı 0'dan büyük olmalı (km/sa)."),
        (positive(t.traction) && t.traction <= 2.0, "truck.traction", "Çekiş katsayısı 0 ile 2 arası olmalı."),
        (positive(t.drive) && t.drive <= 1.0, "truck.drive", "Tahrik dingilindeki ağırlık payı 0 ile 1 arası olmalı."),
        (positive(t.brake), "truck.brake", "Fren yavaşlaması 0'dan büyük olmalı (m/s²)."),
        (q.rolling.is_finite() && (0.0..50.0).contains(&q.rolling), "rolling", "Yuvarlanma direnci 0 ile %50 arası olmalı."),
        (positive(q.load), "load", "Kamyonun taşıdığı hacim 0'dan büyük olmalı (m³)."),
        (positive(q.loading), "loading", "Yükleme süresi 0'dan büyük olmalı (dk)."),
        (q.dumping.is_finite() && q.dumping >= 0.0, "dumping", "Boşaltma süresi negatif olamaz (dk)."),
        (q.spotting.is_none_or(|v| v.is_finite() && v >= 0.0), "spotting", "Manevra süresi negatif olamaz (dk)."),
        (q.trucks.is_none_or(|n| (1..=500).contains(&n)), "trucks", "Kamyon sayısı 1 ile 500 arası olmalı."),
        (q.hour.is_none_or(|h| positive(h) && h <= 60.0), "hour", "Saatte çalışılan dakika 0 ile 60 arası olmalı."),
        (q.fuel.is_none_or(positive), "fuel", "Özgül yakıt tüketimi 0'dan büyük olmalı (L/kWh)."),
        (q.idle.is_none_or(|v| v.is_finite() && v >= 0.0), "idle", "Rölanti yakıtı negatif olamaz (L/sa)."),
        (q.co2.is_none_or(positive), "co2", "Yakıtın CO₂ salımı 0'dan büyük olmalı (kg/L)."),
    ] {
        if !ok {
            return Err(bad(path, words.into()));
        }
    }
    if q.idle.is_some() && q.fuel.is_none() {
        return Err(bad("idle", "Rölanti yakıtı özgül yakıt tüketimiyle (fuel) birlikte verilir.".into()));
    }
    if q.co2.is_some() && q.fuel.is_none() {
        return Err(bad("co2", "CO₂ yakıttan hesaplanır: özgül yakıt tüketimini (fuel) de verin.".into()));
    }
    Ok(())
}

/// The road's heights along the alignment: the profile's at a station, or the ground's points.
enum Heights<'a> {
    Profile(&'a kentos_road::profile::Vertical, Vec<f64>),
    Ground(Vec<kentos_road::profile::ProfilePoint>),
}

impl Heights<'_> {
    /// The path from station `a` to `b` (`[distance from a, height]`), or the first station it does not cover.
    fn path(&self, a: f64, b: f64) -> Result<Vec<[f64; 2]>, f64> {
        let (lo, hi) = (a.min(b), a.max(b));
        let mut pts: Vec<(f64, f64)> = Vec::new();
        match self {
            Heights::Profile(v, breaks) => {
                let mut stations: Vec<f64> = vec![lo, hi];
                // Every metre and the profile's breaks: its grades exact, its curves by 1 m chords. With 5 m chords
                // the grade crossed the downhill speed's threshold (−c_r) a few metres off on a crest and a run of
                // 18 s came 47 ms (0,26 %) off the reference's; the threshold makes the time sensitive to it.
                let mut s = lo.floor() + 1.0;
                while s < hi {
                    stations.push(s);
                    s += 1.0;
                }
                stations.extend(breaks.iter().copied().filter(|s| *s > lo && *s < hi));
                stations.sort_by(f64::total_cmp);
                stations.dedup_by(|x, y| (*x - *y).abs() < 1e-9);
                for s in stations {
                    pts.push((s, v.at(s).ok_or(s)?.0));
                }
            }
            Heights::Ground(points) => {
                let at = |s: f64| -> Result<f64, f64> {
                    let i = points.partition_point(|p| p.station < s);
                    let (p0, p1) = (points.get(i.saturating_sub(1)).ok_or(s)?, points.get(i).ok_or(s)?);
                    if (p1.station - s).abs() < 1e-9 {
                        return p1.z.ok_or(s);
                    }
                    if p1.gap {
                        return Err(s);
                    }
                    let (z0, z1) = (p0.z.ok_or(s)?, p1.z.ok_or(s)?);
                    let t = if p1.station > p0.station { (s - p0.station) / (p1.station - p0.station) } else { 0.0 };
                    Ok(z0 + t * (z1 - z0))
                };
                pts.push((lo, at(lo)?));
                for p in points.iter().filter(|p| p.station > lo && p.station < hi) {
                    if p.gap {
                        return Err(p.station);
                    }
                    pts.push((p.station, p.z.ok_or(p.station)?));
                }
                pts.push((hi, at(hi)?));
            }
        }
        let path: Vec<[f64; 2]> = pts.iter().map(|&(s, z)| [s - lo, z]).collect();
        // From `a`: forward as it is, backward the other way.
        Ok(if a <= b { path } else { reversed(&path) })
    }
}

/// The answer now and its warnings.
pub(crate) fn answer(doc: &Document, q: &HaulsTrucks) -> Result<(HaulTrucked, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    common::find(doc.road_hauls(), &q.haul, "haul")?;
    judged(q)?;
    let (plan, warnings) = crate::road_hauls_plan::answer(doc, &HaulsPlan { haul: q.haul.clone(), objective: q.objective })?;
    let (_, def) = common::find(doc.road_hauls(), &q.haul, "haul")?;
    let vdef = doc.road_volumes().iter().find(|v| v.id == def.volume).ok_or_else(|| bad("haul", "Brüknerin kübajı yok.".into()))?;
    let cdef = doc
        .road_corridors()
        .iter()
        .find(|c| c.id == vdef.corridor)
        .ok_or_else(|| bad("haul", "Kübajın koridoru projede yok.".into()))?;
    let pdef = crate::road_profiles::profiles(doc)
        .iter()
        .find(|p| p.id == cdef.profile)
        .ok_or_else(|| bad("track", format!("“{}” koridorunun boykesiti projede yok.", cdef.name)))?;
    let built = crate::road_profiles::build(doc, pdef)?;
    let heights = match q.track {
        HaulTrack::Profile => {
            let mut breaks: Vec<f64> = pdef.pvis.iter().map(|p| p.station).collect();
            breaks.extend(built.vertical.curves.iter().flat_map(|c| [c.start[0], c.end[0]]));
            Heights::Profile(&built.vertical, breaks)
        }
        HaulTrack::Ground => {
            let sdef = crate::road_terrains::surfaces(doc)
                .iter()
                .find(|s| s.id == cdef.surface)
                .ok_or_else(|| bad("track", format!("“{}” koridorunun yüzeyi projede yok.", cdef.name)))?;
            let surface = crate::road_terrains::make(doc, sdef)
                .surface
                .map_err(|_| bad("track", format!("“{}” yüzeyi kurulamıyor.", sdef.name)))?;
            Heights::Ground(kentos_road::profile::terrain(&surface, &built.alignment.core))
        }
    };
    let km = |s: f64| km_text(built.alignment.km(s), 3);
    let truck = Truck {
        empty: q.truck.empty * 1000.0,
        gross: q.truck.gross * 1000.0,
        power: q.truck.power * 1000.0,
        efficiency: q.truck.efficiency,
        top: q.truck.top / 3.6,
        downhill: q.truck.downhill.map(|d| d / 3.6),
        traction: q.truck.traction,
        drive: q.truck.drive,
        brake: q.truck.brake,
    };
    let rolling = q.rolling / 100.0;
    let fixed = 60.0 * (q.loading + q.dumping + q.spotting.unwrap_or(0.0));
    let split = hauls(&plan.segments.iter().map(|s| (s.from, s.to, s.flow)).collect::<Vec<_>>());
    if let Some(i) = q.trace
        && i as usize >= split.len()
    {
        return Err(bad("trace", format!("Planda {} taşıma var; {} sıralı taşıma yok.", split.len(), i)));
    }
    let mut out_hauls = Vec::new();
    let mut runs: Vec<(Run, Run)> = Vec::new();
    for (i, h) in split.iter().enumerate() {
        let path = heights.path(h.from, h.to).map_err(|s| match q.track {
            HaulTrack::Profile => bad(
                "track",
                format!("“{}” boykesiti km {} noktasını kapsamıyor; taşıma {} – {} arası.", pdef.name, km(s), km(h.from), km(h.to)),
            ),
            HaulTrack::Ground => bad(
                "track",
                format!("Arazi km {} noktasında yok (yüzeyin dışı ya da boşluğu); taşıma {} – {} arası.", km(s), km(h.from), km(h.to)),
            ),
        })?;
        let sample = (q.trace == Some(i as u32)).then(|| (path.last().map_or(0.0, |p| p[0]) / 200.0).max(2.0));
        // Where it stalls depends on the steps; the haul it stalls on does not.
        let stalls = |loaded: bool| {
            fail(
                codes::HAUL_TRUCK_STALLS,
                format!(
                    "Kamyon {} {} → {} taşımasında bir eğimi çıkamıyor: tahrik tekerleklerinin tutunması (çekiş × pay = {:.3}) yuvarlanma ile eğimin direncinden az. Çekiş katsayısını ya da tahrik payını gözden geçirin.",
                    if loaded { "yüklü" } else { "boş dönerken" },
                    km(if loaded { h.from } else { h.to }),
                    km(if loaded { h.to } else { h.from }),
                    truck.traction * truck.drive
                ),
                "truck.traction",
            )
        };
        let loaded = run(&path, truck.gross, &truck, rolling, sample).map_err(|_| stalls(true))?;
        let empty = run(&reversed(&path), truck.empty, &truck, rolling, sample).map_err(|_| stalls(false))?;
        let cycle = loaded.time + empty.time + fixed;
        let trips = (h.volume / q.load - 1e-9).ceil().max(1.0);
        let fuel = q.fuel.map(|f| {
            let engine = (loaded.work + empty.work) / truck.efficiency / 3.6e6;
            f * engine + q.idle.unwrap_or(0.0) * (loaded.idle + empty.idle + fixed) / 3600.0
        });
        let (z0, z1) = (path.first().map_or(0.0, |p| p[1]), path.last().map_or(0.0, |p| p[1]));
        out_hauls.push(TruckHaul {
            from: h.from,
            to: h.to,
            from_text: km(h.from),
            to_text: km(h.to),
            volume: h.volume,
            distance: (h.to - h.from).abs(),
            rise: z1 - z0,
            loaded: loaded.time,
            empty: empty.time,
            cycle,
            trips,
            balance: cycle / (60.0 * q.loading),
            fuel,
        });
        runs.push((loaded, empty));
    }
    let trips: f64 = out_hauls.iter().map(|h| h.trips).sum();
    let cycle = (trips > 0.0).then(|| out_hauls.iter().map(|h| h.trips * h.cycle).sum::<f64>() / trips);
    let energy: f64 =
        out_hauls.iter().zip(&runs).map(|(h, (l, e))| h.trips * (l.work + e.work) / truck.efficiency / 3.6e6).sum();
    let service = 60.0 * q.loading;
    let hour = q.hour.unwrap_or(60.0);
    let fleet = q.trucks.filter(|_| trips > 0.0).map(|n| {
        let (mut steady_s, mut queued_s, mut busy_s, mut waits) = (0.0, 0.0, 0.0, 0.0);
        for h in &out_hauls {
            let l = loader(n, h.cycle - service, service);
            steady_s += h.trips / l.steady;
            queued_s += h.trips / l.queued;
            busy_s += h.trips / l.queued * l.busy;
            waits += h.trips * l.wait;
        }
        // Working seconds to hours on the clock: `hour` minutes worked in each.
        let hours = |s: f64| s / 3600.0 * 60.0 / hour;
        let volume: f64 = out_hauls.iter().map(|h| h.volume).sum();
        TruckFleet {
            trucks: n,
            match_factor: f64::from(n) * service / cycle.unwrap_or(f64::INFINITY),
            steady: volume / hours(steady_s),
            queued: volume / hours(queued_s),
            busy: busy_s / queued_s,
            wait: waits / trips,
            hours_steady: hours(steady_s),
            hours_queued: hours(queued_s),
        }
    });
    let fuel = q.fuel.map(|_| {
        let trips_fuel: f64 = out_hauls.iter().map(|h| h.trips * h.fuel.unwrap_or(0.0)).sum();
        trips_fuel + fleet.as_ref().map_or(0.0, |f| q.idle.unwrap_or(0.0) * f.wait * trips / 3600.0)
    });
    let trace = q.trace.map(|i| {
        let (h, (l, e)) = (&split[i as usize], &runs[i as usize]);
        let sign = if h.to >= h.from { 1.0 } else { -1.0 };
        TruckTrace {
            haul: i,
            loaded: l.speeds.iter().map(|p| [h.from + sign * p[0], p[1] * 3.6]).collect(),
            empty: e.speeds.iter().map(|p| [h.to - sign * p[0], p[1] * 3.6]).collect(),
        }
    });
    Ok((
        HaulTrucked {
            haul: plan.haul,
            track: q.track,
            hauls: out_hauls,
            trips,
            cycle,
            balance: cycle.map(|c| c / service),
            energy,
            fleet,
            co2: q.co2.zip(fuel).map(|(c, f)| c * f),
            fuel,
            trace,
        },
        warnings,
    ))
}

/// Checks `q` against the document.
pub fn validate(cx: &ExecutionContext<'_>, q: &HaulsTrucks) -> CommandResult<()> {
    match answer(cx.doc, q) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, q: &HaulsTrucks) -> CommandResult<HaulTrucked> {
    match answer(cx.doc, q) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, q: HaulsTrucks) -> CommandResult<HaulTrucked> {
    plan(cx, &q)
}

/// The answer for a window, or why it does not read.
pub fn preview(doc: &Document, q: &HaulsTrucks) -> Result<(HaulTrucked, Vec<CommandWarning>), String> {
    answer(doc, q).map_err(crate::road_corridors::words)
}
