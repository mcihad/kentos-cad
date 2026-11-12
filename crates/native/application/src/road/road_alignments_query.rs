//! `road.alignments.query` v1 (plan/road.md Y3, docs/adr/0901 §5–§7): the road project's alignments as they build now,
//! or one's places at stations, at kms and every round km, and points' stations and offsets. Reads only. The shared
//! cases are `fixtures/commands/v1/road.alignments.query.json`.

use kentos_commands::{
    AlignmentFoot, AlignmentsQueried, AlignmentsQuery, CommandResult, KmPlaces, MAX_ALIGNMENT_PLACES, PointStation,
};
use kentos_road::alignment::{End, Stationing};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_alignments::{self as common, Built};
use crate::road_terrains::fail;

fn place_at(built: &Built, station: f64, path: &str) -> Result<kentos_commands::AlignmentPlace, Stop> {
    built.place(station).ok_or_else(|| {
        fail(
            codes::STATION_OUTSIDE,
            format!("{station} m aksın üstünde değil (aks {:.3} m).", built.core.length),
            path,
        )
    })
}

fn answer(doc: &kentos_domain::Document, input: &AlignmentsQuery) -> Result<AlignmentsQueried, Stop> {
    common::road_project(doc)?;
    let own = common::alignments(doc);
    let finite = input.stations.iter().all(|s| s.is_finite())
        && input.kms.iter().all(|k| k.km.is_finite())
        && input.points.iter().all(|p| p[0].is_finite() && p[1].is_finite())
        && input.every.is_none_or(|e| e.is_finite() && e > 0.0);
    if !finite {
        return Err(fail(
            codes::INVALID_PLACES,
            "Km'ler ve yerler sonlu sayılar, every 0'dan büyük olmalı.".into(),
            "stations",
        ));
    }
    if input.stations.len() + input.kms.len() + input.points.len() > MAX_ALIGNMENT_PLACES {
        return Err(fail(
            codes::INVALID_PLACES,
            format!("Bir soruda en çok {MAX_ALIGNMENT_PLACES} yer olur."),
            "stations",
        ));
    }
    let asks = !input.stations.is_empty() || !input.kms.is_empty() || !input.points.is_empty() || input.every.is_some();
    let Some(name) = input.alignment.as_deref() else {
        if asks {
            return Err(fail(
                codes::INVALID_PLACES,
                "Km ve yer bir aksta sorulur: aksın kimliğini ya da adını (alignment) verin.".into(),
                "alignment",
            ));
        }
        let mut alignments = Vec::new();
        for def in own {
            let built = common::build(own, def, "")?;
            alignments.push(common::report(doc, def, &built));
        }
        return Ok(AlignmentsQueried {
            alignments,
            at: vec![],
            kms: vec![],
            ticks: vec![],
            points: vec![],
        });
    };
    let (_, def) = common::find(own, name, "alignment")?;
    let built = common::build(own, def, "")?;
    let report = common::report(doc, def, &built);
    let at = input
        .stations
        .iter()
        .enumerate()
        .map(|(i, s)| place_at(&built, *s, &format!("stations[{i}]")))
        .collect::<Result<Vec<_>, _>>()?;
    let mut kms = Vec::new();
    for (i, k) in input.kms.iter().enumerate() {
        let path = format!("kms[{i}]");
        let found = Stationing::stations(&built.regions, k.km);
        let chosen: Vec<_> = match k.region {
            Some(r) => found.into_iter().filter(|(region, _)| *region as u32 == r).collect(),
            None if found.len() > 1 => {
                let regions: Vec<String> = found.iter().map(|(r, _)| r.to_string()).collect();
                return Err(fail(
                    codes::STATION_AMBIGUOUS,
                    format!(
                        "{} km'si aksın {} yerine düşüyor (bölgeler {}); hangisi olduğunu region ile verin.",
                        kentos_road::alignment::km_text(k.km, 3),
                        found.len(),
                        regions.join(", ")
                    ),
                    &path,
                ));
            }
            None => found,
        };
        if chosen.is_empty() {
            return Err(fail(
                codes::STATION_OUTSIDE,
                format!("{} km'si aksın üstünde değil.", kentos_road::alignment::km_text(k.km, 3)),
                &path,
            ));
        }
        let places = chosen.iter().map(|(_, s)| place_at(&built, *s, &path)).collect::<Result<Vec<_>, _>>()?;
        kms.push(KmPlaces { km: k.km, places });
    }
    let mut ticks = Vec::new();
    if let Some(step) = input.every {
        for s in common::tick_stations(&built, step)? {
            ticks.push(place_at(&built, s, "every")?);
        }
    }
    let points = input
        .points
        .iter()
        .map(|p| {
            // A parallel axis's points are read off its source, moved by the offset.
            let d = built.offset;
            let found = built.core.locate(*p);
            let foot = |f: &kentos_road::alignment::Foot| -> Option<AlignmentFoot> {
                Some(AlignmentFoot {
                    at: built.place(f.at.station)?,
                    offset: f.offset - d,
                    corner: f.corner,
                })
            };
            let feet: Vec<AlignmentFoot> = found.feet.iter().filter_map(foot).collect();
            PointStation {
                point: *p,
                nearest: feet.first().cloned(),
                feet,
                beyond: found.beyond.map(|(e, _)| match e {
                    End::Start => "start".to_owned(),
                    End::End => "end".to_owned(),
                }),
            }
        })
        .collect();
    Ok(AlignmentsQueried {
        alignments: vec![report],
        at,
        kms,
        ticks,
        points,
    })
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &AlignmentsQuery) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: vec![],
        },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &AlignmentsQuery) -> CommandResult<AlignmentsQueried> {
    match answer(cx.doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: vec![],
        },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: AlignmentsQuery) -> CommandResult<AlignmentsQueried> {
    plan(cx, &input)
}
