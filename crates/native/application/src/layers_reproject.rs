//! Yeniden izdüşür's preview (plan/map-cad.md MAP-02, docs/adr/2500): the layer's objects moved as
//! `cad.layers.reproject` writes them, and what the move shows before it is written — every object's shift (its first
//! point now and moved, the smallest and largest shift of its points), what its points fall outside of (EPSG's areas
//! of use of the source system, of the datum transformation, of the project's system; IOGP GN7-1 §6.5 asks whether
//! a transformation's area is consistent with the data's), the shifts summed up and the chain's verdict for the way
//! as `cad.crs.transform` gives it (docs/adr/1230 §3). A layer some of whose objects do not move is refused with each
//! reason counted (a place the system does not reach, a grid, a kind that is not points and straight paths).

use kentos_commands::{
    ChainStatus, CommandResult, CommandWarning, CrsPoint, CrsSide, CrsTransform, Entity,
    REPROJECT_LISTED, REPROJECT_NOTES, ReprojectShift, Vec2,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::crs as core;

use crate::checks::{Stop, error};
use crate::codes;
use crate::sources::Mover;

/// The move's preview: every point's shift summed up, the objects with a note, the chain's verdict for the way, the
/// objects' shifts (those with a note first) and the warnings they give.
#[derive(Default)]
pub(crate) struct Preview {
    pub(crate) points: u32,
    pub(crate) mean: Option<(f64, f64)>,
    pub(crate) least: Option<f64>,
    pub(crate) most: Option<f64>,
    pub(crate) flagged: u32,
    pub(crate) status: Option<ChainStatus>,
    pub(crate) reasons: Vec<String>,
    pub(crate) shifts: Vec<ReprojectShift>,
    pub(crate) warnings: Vec<CommandWarning>,
}

/// The way a point goes, as the transforms say it (“EPSG:1311, ±1 m”).
fn way(m: &Mover, at: kentos_geometry_core::Vec2) -> String {
    match core::transform_in(&m.source, &m.project, at, &m.choices) {
        Ok(t) => {
            let mut words = if t.via.is_empty() {
                "aynı datum".to_owned()
            } else {
                t.via.clone()
            };
            if let Some(a) = t.accuracy {
                words.push_str(&format!(", ±{a} m"));
            }
            if t.unofficial {
                words.push_str(", resmî dönüşüm değil");
            }
            words
        }
        Err(_) => String::new(),
    }
}

/// Why an object does not move, as the refusal says it: a vertex's reason, or none for a kind that is not points and
/// straight paths (docs/adr/2500 §5).
fn unreached_words(why: Option<core::Unreached>) -> &'static str {
    match why {
        Some(core::Unreached::Outside) => "sistemin dışında",
        Some(core::Unreached::OutsideGrid) => "datum dönüşümünün ızgarasının dışında",
        Some(core::Unreached::NoGrid) => "datum dönüşümünün ızgarası bu cihazda yok",
        Some(core::Unreached::NoLink) => "datumlardan birinin WGS 84'e dönüşümü yok",
        None => "yay ya da eğrili çizgi, daire, yazı, blok gibi tam çevrilemeyen türden",
    }
}

/// The refusal of a layer some of whose objects do not move (docs/adr/2500 §5): how many and why, each reason with
/// its count when there are several, and what to do; the objects' ids.
fn unreached_refusal(name: &str, from: u32, unreached: &[(Uuid, Option<core::Unreached>)]) -> Stop {
    let mut reasons: Vec<(&'static str, u32)> = Vec::new();
    for (_, why) in unreached {
        let words = unreached_words(*why);
        match reasons.iter_mut().find(|(w, _)| *w == words) {
            Some((_, n)) => *n += 1,
            None => reasons.push((words, 1)),
        }
    }
    let said = match reasons.as_slice() {
        [(words, _)] => (*words).to_owned(),
        many => many
            .iter()
            .map(|(words, n)| format!("{n} nesne {words}"))
            .collect::<Vec<_>>()
            .join(", "),
    };
    let kinds = unreached.iter().any(|(_, why)| why.is_none());
    let places = unreached.iter().any(|(_, why)| why.is_some());
    let advice = match (places, kinds) {
        (true, false) => "Doğru sistemi from ile verin.",
        (false, _) => {
            "Yalnız noktalar ve düz kenarlı çizgiler çevrilir; bu nesneleri başka bir katmana alın."
        }
        (true, true) => {
            "Doğru sistemi from ile verin; yalnız noktalar ve düz kenarlı çizgiler çevrilir, öbürlerini başka bir katmana alın."
        }
    };
    let mut err = error(
        codes::CRS_UNREACHED,
        format!(
            "“{name}” katmanının {} nesnesi EPSG:{from}'den projenin sistemine çevrilemiyor ({said}); hiçbir şey yazılmadı. {advice}",
            unreached.len()
        ),
        Some("from".to_owned()),
    );
    err.uids = unreached.iter().map(|(uid, _)| uid.to_string()).collect();
    Stop::failed(err)
}

/// What a note says in the warning (docs/adr/2500 §3).
fn note_words(note: &str) -> &'static str {
    match note {
        "outside_source_area" => {
            "kaynak sistemin kullanım alanının dışında (nesneler başka bir sistemde olabilir)"
        }
        "outside_operation_area" => {
            "datum dönüşümünün kullanım alanının dışında (dönüşümün doğruluğu orada geçerli değil)"
        }
        _ => "projenin sisteminin kullanım alanının dışında",
    }
}

/// The boxes (degrees west, south, east, north) a moved object's points are read against (docs/adr/2500 §3): EPSG's
/// area of use of the source system, of the datum transformation's EPSG operation (none when the datum stays or the
/// way is the project's own), of the project's system (none for a definition of the project's).
struct Areas {
    source: Option<[f64; 4]>,
    operation: Option<[f64; 4]>,
    project: Option<[f64; 4]>,
}

fn bbox(a: &kentos_project::crs::AreaOfUse) -> [f64; 4] {
    [a.west, a.south, a.east, a.north]
}

/// One moved object's shift: its first point now and moved, every point's shift length and its notes. A point is
/// read against the source's boxes at its place on the source's datum, against the project's where it lands.
fn shift_of(
    uid: &Uuid,
    e: &Entity,
    before: &[Vec2],
    after: &[Vec2],
    mover: &Mover,
    areas: &Areas,
) -> ReprojectShift {
    let mut outside = [false; 3];
    let held = |b: Option<[f64; 4]>, at: Option<(f64, f64)>| match (b, at) {
        (Some(b), Some((lat, lon))) => kentos_project::crs::box_holds(b, lon, lat),
        _ => true,
    };
    let (mut least, mut most) = (f64::INFINITY, 0.0_f64);
    for (p, q) in before.iter().zip(after) {
        let d = (q.x - p.x).hypot(q.y - p.y);
        least = least.min(d);
        most = most.max(d);
        let here = mover
            .source
            .geographic_of(kentos_geometry_core::Vec2::new(p.x, p.y));
        outside[0] |= !held(areas.source, here);
        outside[1] |= !held(areas.operation, here);
        let there = mover
            .project
            .geographic_of(kentos_geometry_core::Vec2::new(q.x, q.y));
        outside[2] |= !held(areas.project, there);
    }
    let (p, q) = (before[0], after[0]);
    ReprojectShift {
        uid: uid.to_string(),
        kind: e.kind().to_owned(),
        points: before.len() as u32,
        before: p,
        after: q,
        dx: q.x - p.x,
        dy: q.y - p.y,
        shift: (q.x - p.x).hypot(q.y - p.y),
        least,
        most,
        notes: REPROJECT_NOTES
            .iter()
            .zip(outside)
            .filter(|(_, o)| *o)
            .map(|(n, _)| (*n).to_owned())
            .collect(),
    }
}

/// The chain's verdict for the way, as `cad.crs.transform` gives it for the same two systems at `at` (docs/adr/1230
/// §3): the horizontal way only, as heights stay as they are (docs/adr/0583 §2). None when it does not answer.
fn verdict(doc: &Document, from: u32, at: kentos_geometry_core::Vec2) -> Option<(ChainStatus, Vec<String>)> {
    let input = CrsTransform {
        points: vec![CrsPoint {
            x: at.x,
            y: at.y,
            z: None,
        }],
        from: Some(CrsSide {
            srid: Some(from),
            heights: None,
            epoch: None,
        }),
        to: None,
        uncertainty: None,
        velocity: None,
    };
    match crate::crs_transform::answer(doc, &input) {
        CommandResult::Completed { output, .. } => {
            Some((output.chain.status, output.chain.reasons))
        }
        _ => None,
    }
}

/// The layer's objects moved from `from` into the project's system, the way they go and the move's preview; the
/// refusal when one of them does not move.
pub(crate) fn preview(
    doc: &Document,
    layer: &str,
    name: &str,
    from: u32,
    mover: &Mover,
) -> Result<(Vec<(Slot, Entity)>, String, Preview), Stop> {
    let settings = doc.settings();
    let areas = Areas {
        source: kentos_project::crs::area(from).map(bbox),
        operation: core::operation::operation(&mover.source, &mover.project, &mover.choices, false)
            .ok()
            .and_then(|o| o.area)
            .map(|(_, b)| b),
        project: kentos_project::crs::area(settings.srid).map(bbox),
    };
    let mut changes = Vec::new();
    let mut unreached: Vec<(Uuid, Option<core::Unreached>)> = Vec::new();
    let mut first: Option<kentos_geometry_core::Vec2> = None;
    let mut preview = Preview::default();
    let (mut sum_x, mut sum_y) = (0.0, 0.0);
    let (mut noted, mut plain): (Vec<ReprojectShift>, Vec<ReprojectShift>) = (Vec::new(), Vec::new());
    for (uid, e) in doc.by_layer_with_uids(layer) {
        let mut moved = e.clone();
        if first.is_none() {
            let b = kentos_geometry_core::ops::topology_rules::incremental::shape_bounds(
                &crate::geometry::shape(e),
            );
            let c = kentos_geometry_core::Vec2::new(
                (b.min_x + b.max_x) / 2.0,
                (b.min_y + b.max_y) / 2.0,
            );
            first = (c.x.is_finite() && c.y.is_finite()).then_some(c);
        }
        match kentos_formats::reproject::move_entity(
            &mut moved,
            &mover.source,
            &mover.project,
            &mover.choices,
        ) {
            Ok(()) => {
                let before = kentos_formats::reproject::points_of(e).unwrap_or_default();
                let after = kentos_formats::reproject::points_of(&moved).unwrap_or_default();
                if !before.is_empty() && before.len() == after.len() {
                    for (p, q) in before.iter().zip(&after) {
                        let (dx, dy) = (q.x - p.x, q.y - p.y);
                        let d = dx.hypot(dy);
                        sum_x += dx;
                        sum_y += dy;
                        preview.least = Some(preview.least.map_or(d, |l| l.min(d)));
                        preview.most = Some(preview.most.map_or(d, |m| m.max(d)));
                    }
                    preview.points += before.len() as u32;
                    let row = shift_of(&uid, e, &before, &after, mover, &areas);
                    if row.notes.is_empty() {
                        plain.push(row);
                    } else {
                        preview.flagged += 1;
                        noted.push(row);
                    }
                }
                changes.push((Slot(e.base().id), moved));
            }
            Err(why) => unreached.push((uid, why)),
        }
    }
    if !unreached.is_empty() {
        return Err(unreached_refusal(name, from, &unreached));
    }
    if preview.points > 0 {
        let n = f64::from(preview.points);
        preview.mean = Some((sum_x / n, sum_y / n));
    }
    // How many objects each note has, for the warning; then every object with a note listed first.
    let counts: Vec<(&str, usize)> = REPROJECT_NOTES
        .iter()
        .map(|n| (*n, noted.iter().filter(|r| r.notes.iter().any(|m| m == n)).count()))
        .filter(|(_, k)| *k > 0)
        .collect();
    preview.shifts = noted;
    preview.shifts.extend(plain);
    preview.shifts.truncate(REPROJECT_LISTED);
    if let Some(at) = first
        && let Some((status, reasons)) = verdict(doc, from, at)
    {
        if status == ChainStatus::Limited {
            preview.warnings.push(CommandWarning {
                code: codes::CHAIN_LIMITED.into(),
                message: format!(
                    "Dönüşüm zinciri sınırlı, hassas dönüşüm sayılmaz: {}.",
                    reasons
                        .iter()
                        .map(|r| crate::crs_transform::reason_text(r))
                        .collect::<Vec<_>>()
                        .join("; ")
                ),
                path: Some("from".into()),
            });
        }
        preview.status = Some(status);
        preview.reasons = reasons;
    }
    if preview.flagged > 0 {
        preview.warnings.push(CommandWarning {
            code: codes::OUTSIDE_AREA.into(),
            message: format!(
                "{} nesnenin noktaları EPSG'nin kullanım alanlarından birinin dışında: {}. Önizlemedeki kaymaları denetleyin.",
                preview.flagged,
                counts
                    .iter()
                    .map(|(n, k)| format!("{k} nesne {}", note_words(n)))
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
            path: Some("from".into()),
        });
    }
    let via = first.map(|p| way(mover, p)).unwrap_or_default();
    Ok((changes, via, preview))
}
