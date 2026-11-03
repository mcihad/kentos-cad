//! A road project's junctions (plan/road.md Y12.2, docs/adr/0910 §2): found by id or name, read now from their legs'
//! alignments: the crossing (`kentos-road`'s `junction::crossings`), the legs' stations and directions there, the
//! angle between them, and at grade each quadrant's curb return (`junction::fillet`, the legs taken along their
//! tangents). Nothing read is kept: every read computes again.

use kentos_commands::{
    CommandWarning, JunctionDef, JunctionKind, JunctionLeg, JunctionListed, JunctionPlace, JunctionReport, JunctionReturn,
};
use kentos_domain::Document;
use kentos_road::alignment::km_text;
use kentos_road::junction::{Edge, crossings, fillet};

use crate::checks::Stop;
use crate::codes;
use crate::road_alignments as alignments;
use crate::road_terrains::fail;

/// The quadrants in order: the first leg's side and the second's.
pub(crate) const QUADRANTS: [(&str, &str); 4] = [("right", "right"), ("right", "left"), ("left", "left"), ("left", "right")];

/// Only a road project has junctions.
pub(crate) fn road_project(doc: &Document) -> Result<(), Stop> {
    if doc.settings().cad_work == Some(kentos_commands::CadWork::Road) {
        Ok(())
    } else {
        Err(fail(codes::NOT_A_ROAD_PROJECT, "Kavşak yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.".into(), ""))
    }
}

/// The junction of `name`: by its id, else by its name (case aside).
pub(crate) fn find<'a>(list: &'a [JunctionDef], name: &str, path: &str) -> Result<(usize, &'a JunctionDef), Stop> {
    list.iter()
        .position(|c| c.id == name)
        .or_else(|| list.iter().position(|c| c.name.to_lowercase() == name.to_lowercase()))
        .map(|i| (i, &list[i]))
        .ok_or_else(|| fail(codes::UNKNOWN_JUNCTION, format!("“{name}” kimlikli ya da adlı kavşak projede yok."), path))
}

pub(crate) fn listed(list: &[JunctionDef]) -> Vec<JunctionListed> {
    list.iter().map(|j| JunctionListed { id: j.id.clone(), name: j.name.clone(), kind: j.kind, revision: j.revision }).collect()
}

/// The junction read now, and its warnings.
pub(crate) fn read(doc: &Document, def: &JunctionDef) -> Result<(JunctionReport, Vec<CommandWarning>), Stop> {
    let list = alignments::alignments(doc);
    let bad = |words: String, path: &str| fail(codes::INVALID_JUNCTION, words, path);
    let mut built = Vec::new();
    for leg in &def.legs {
        let (_, a) = alignments::find(list, &leg.alignment, "legs")?;
        let b = alignments::build(list, a, "")
            .map_err(|_| bad(format!("“{}” kavşağının “{}” aksı kurulamıyor.", def.name, a.name), "legs"))?;
        built.push((a, b));
    }
    let ((da, ba), (db, bb)) = (&built[0], &built[1]);
    let found = crossings(&ba.core, &bb.core, 1.0);
    if found.is_empty() {
        return Err(bad(format!("“{}” kavşağının kolları (“{}” ve “{}”) kesişmiyor.", def.name, da.name, db.name), "legs"));
    }
    let Some(&(sa, sb)) = found.get(def.at as usize) else {
        return Err(bad(
            format!("“{}” kavşağının kolları {} yerde kesişiyor; {}. kesişim yok.", def.name, found.len(), def.at + 1),
            "at",
        ));
    };
    let (Some(pa), Some(pb)) = (ba.core.at(sa), bb.core.at(sb)) else {
        return Err(bad(format!("“{}” kavşağının kesişimi aksların üstünde değil.", def.name), "legs"));
    };
    let (ta, tb) = ([pa.direction.cos(), pa.direction.sin()], [pb.direction.cos(), pb.direction.sin()]);
    if (ta[0] * tb[1] - ta[1] * tb[0]).abs() < 1e-9 {
        return Err(bad(format!("“{}” kavşağının kolları kesişimde paralel.", def.name), "legs"));
    }
    let angle = (ta[0] * tb[0] + ta[1] * tb[1]).clamp(-1.0, 1.0).acos().to_degrees();
    let p = [pa.x, pa.y];
    let mut warnings = Vec::new();
    let mut stale = false;
    let legs: Vec<JunctionLeg> = [(da, ba, sa, pa.direction, &def.legs[0]), (db, bb, sb, pb.direction, &def.legs[1])]
        .into_iter()
        .map(|(d, b, s, dir, leg)| {
            let changed = d.revision != leg.alignment_revision;
            if changed {
                stale = true;
                warnings.push(CommandWarning {
                    code: codes::JUNCTION_STALE.into(),
                    message: format!("“{}” kavşağının “{}” aksı değişti; kavşağı denetleyip yeniden bağlayın.", def.name, d.name),
                    path: Some("junction".into()),
                });
            }
            JunctionLeg { alignment: d.id.clone(), station: s, km_text: km_text(b.km(s), 3), direction: dir.to_degrees(), stale: changed }
        })
        .collect();
    let mut returns = Vec::new();
    if def.kind == JunctionKind::AtGrade {
        let side = |w: &str, leg: &kentos_commands::JunctionLegDef| if w == "right" { (1.0, leg.right) } else { (-1.0, -leg.left) };
        let mut reach: [Vec<f64>; 2] = [vec![sa], vec![sb]];
        for (k, (wa, wb)) in QUADRANTS.iter().enumerate() {
            let r = def.radii.as_ref().map_or(def.radius, |v| v[k]);
            let ((ka, oa), (kb, ob)) = (side(wa, &def.legs[0]), side(wb, &def.legs[1]));
            let Some(f) = fillet(p, Edge { tangent: ta, offset: oa, side: ka }, Edge { tangent: tb, offset: ob, side: kb }, r)
            else {
                return Err(bad(format!("“{}” kavşağının kolları kesişimde paralel.", def.name), "legs"));
            };
            let along = |q: [f64; 2], t: [f64; 2], s: f64| s + (q[0] - p[0]) * t[0] + (q[1] - p[1]) * t[1];
            let (qa, qb) = (along(f.on_a, ta, sa), along(f.on_b, tb, sb));
            reach[0].push(qa);
            reach[1].push(qb);
            returns.push(JunctionReturn {
                a: (*wa).into(),
                b: (*wb).into(),
                radius: r,
                cx: f.center[0],
                cy: f.center[1],
                on_a: JunctionPlace { x: f.on_a[0], y: f.on_a[1], station: qa },
                on_b: JunctionPlace { x: f.on_b[0], y: f.on_b[1], station: qb },
                sweep: f.sweep.to_degrees(),
                length: f.length,
            });
        }
        for ((d, b), stations) in built.iter().zip(&reach) {
            let (lo, hi) = stations.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), s| (l.min(*s), h.max(*s)));
            let curved = b
                .core
                .elements
                .iter()
                .any(|e| e.station < hi && e.station + e.length > lo && b.core.at(e.station + 0.5 * e.length).is_some_and(|q| q.curvature.abs() > 1e-12));
            if curved {
                warnings.push(CommandWarning {
                    code: codes::JUNCTION_ON_CURVE.into(),
                    message: format!(
                        "“{}” kavşağının “{}” kolu dönüş yaylarının erimi içinde kurpta; yaylar kolu düz kabul eder.",
                        def.name, d.name
                    ),
                    path: Some("legs".into()),
                });
            }
        }
    }
    Ok((
        JunctionReport {
            id: def.id.clone(),
            name: def.name.clone(),
            revision: def.revision,
            kind: def.kind,
            connected: def.kind == JunctionKind::AtGrade,
            stale,
            x: p[0],
            y: p[1],
            legs,
            angle,
            returns,
        },
        warnings,
    ))
}

/// A junction that does not read: its names and nothing else.
pub(crate) fn broken(def: &JunctionDef) -> JunctionReport {
    JunctionReport {
        id: def.id.clone(),
        name: def.name.clone(),
        revision: def.revision,
        kind: def.kind,
        connected: def.kind == JunctionKind::AtGrade,
        stale: false,
        x: 0.0,
        y: 0.0,
        legs: Vec::new(),
        angle: 0.0,
        returns: Vec::new(),
    }
}
