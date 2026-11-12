//! `road.terrains.sectioncheck` v1 (plan/map-cad.md MAP-14, docs/adr/5120): Kesit kontrolü, BÖHHBÜY m. 93(1). The
//! section is the straight line between two control points of known height; each measured point is read at its foot on
//! that line, how far along it and how far off it. The contour lines (polylines whose every vertex has one same height;
//! an arc read as its chord) are cut with the section; a point's height from the contours is the straight interpolation
//! between the nearest crossing at or before it and the nearest at or after it (a crossing at the point itself is that
//! height). The RMS of the measured less the read heights is held to the package's 1.5 · KOH_Z; the section's places
//! (its ends and its points on it) to the package's spacing. Read only. The shared cases are
//! `fixtures/commands/v1/road.terrains.sectioncheck.json`.

use kentos_commands::{
    CommandResult, CommandWarning, ContourRules, Entity, SectionGap, SectionPointRow, TerrainsSectionCheck,
    TerrainsSectionChecked, contour_rules_default,
};
use kentos_domain::{Document, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

type P = [f64; 2];

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// Two crossings within this along the section, of one height, are one (a line through a vertex of a contour).
const SAME: f64 = 1e-9;

/// The point with its own height that `uid` names.
fn height_point(doc: &Document, uid: &str, path: &str) -> Result<(P, f64), Stop> {
    if !checks::is_uid_text(uid) {
        return Err(fail(
            codes::INVALID_UID,
            format!("“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir."),
            path,
        ));
    }
    let Some(e) = Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u)).and_then(|s| doc.get(s)) else {
        return Err(fail(codes::ENTITY_NOT_FOUND, format!("“{uid}” kimlikli nesne çizimde yok."), path));
    };
    match e {
        Entity::Point(p) if p.z.is_some_and(f64::is_finite) => Ok(([p.p.x, p.p.y], p.z.unwrap_or_default())),
        _ => Err(fail(
            codes::NOT_A_HEIGHT_POINT,
            format!(
                "“{uid}” kendi kotu olan bir nokta değil; kesitin uçları ve noktaları nivelmanla ölçülmüş kotu olan \
                 noktalardır."
            ),
            path,
        )),
    }
}

/// Where the section from `a` to `b` (length `l`) crosses the segment `p`–`q`: how far along the section; none when
/// they do not meet or run together.
fn cross(a: P, b: P, l: f64, p: P, q: P) -> Option<f64> {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let (ex, ey) = (q[0] - p[0], q[1] - p[1]);
    let den = dx * ey - dy * ex;
    if den == 0.0 {
        return None;
    }
    let (fx, fy) = (p[0] - a[0], p[1] - a[1]);
    let t = (fx * ey - fy * ex) / den;
    let u = (fx * dy - fy * dx) / den;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then_some(t * l)
}

struct Checked {
    out: TerrainsSectionChecked,
    warnings: Vec<CommandWarning>,
}

fn run(doc: &Document, input: &TerrainsSectionCheck) -> Result<Checked, Stop> {
    let (a, za) = height_point(doc, &input.from, "from")?;
    let (b, zb) = height_point(doc, &input.to, "to")?;
    let mut measured: Vec<(String, P, f64)> = Vec::new();
    for (i, uid) in input.points.iter().enumerate() {
        let path = format!("points[{i}]");
        let (at, z) = height_point(doc, uid, &path)?;
        if measured.iter().any(|m| m.0 == *uid) || *uid == input.from || *uid == input.to {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("“{uid}” noktası iki kez ya da kesitin ucu olarak verildi."),
                &path,
            ));
        }
        measured.push((uid.clone(), at, z));
    }
    if a == b {
        return Err(fail(
            codes::INVALID_INPUT,
            "Kesitin iki ucu aynı yerde; kesit yüksekliği bilinen iki ayrı kontrol noktası arasında alınır.".into(),
            "to",
        ));
    }
    if measured.is_empty() {
        return Err(fail(
            codes::INVALID_INPUT,
            "Kesitte ölçülmüş nokta yok; kesit doğrultusunda nivelmanla kotu taşınmış noktaları verin.".into(),
            "points",
        ));
    }
    if input.corridor.is_some_and(|c| !(c.is_finite() && c > 0.0)) {
        return Err(fail(codes::INVALID_INPUT, "Kesitten uzaklık sınırı 0'dan büyük olmalı.".into(), "corridor"));
    }
    if input.contours.is_empty() {
        return Err(fail(codes::LAYER_NOT_FOUND, "Eş yükselti eğrilerinin katmanını verin.".into(), "contours"));
    }
    for (i, l) in input.contours.iter().enumerate() {
        if doc.layers().get(l.trim()).is_none() {
            return Err(fail(codes::LAYER_NOT_FOUND, format!("“{}” katmanı çizimde yok.", l.trim()), &format!("contours[{i}]")));
        }
    }
    let scale = match input.scale {
        Some(0) => return Err(fail(codes::INVALID_CONTOURS, "Ölçek paydası 0 olamaz.".into(), "scale")),
        Some(s) => s,
        None => doc.settings().plot_scale.round().max(1.0) as u32,
    };
    let rules: ContourRules = match &input.rules {
        Some(r) => r.clone(),
        None => contour_rules_default()
            .ok_or_else(|| fail(codes::INVALID_CONTOURS, "Yerleşik eğri kural paketi okunamadı.".into(), "rules"))?,
    };
    let positive = |v: f64| v.is_finite() && v > 0.0;
    let Some(c) = rules.section_check.as_ref().filter(|c| {
        c.xy.iter().all(|v| v.is_finite()) && positive(c.z_factor) && positive(c.factor) && c.spacing.is_none_or(positive)
    }) else {
        return Err(fail(
            codes::INVALID_CONTOURS,
            format!(
                "{} paketi kesit kontrolünün sınırını (sectionCheck) vermiyor ya da değerleri sonlu ve 0'dan büyük değil.",
                rules.id
            ),
            "rules",
        ));
    };

    // The contour lines the section crosses, each crossing once, in order along it.
    let l = (b[0] - a[0]).hypot(b[1] - a[1]);
    let mut crossings: Vec<[f64; 2]> = Vec::new();
    let mut unlevelled = 0;
    // Each contour layer's objects from its index (a layer named twice read once).
    let mut read: Vec<&str> = Vec::new();
    for l in input.contours.iter().map(|l| l.trim()) {
        if !read.contains(&l) {
            read.push(l);
        }
    }
    for e in read.iter().flat_map(|l| doc.by_layer(l)) {
        let Entity::Polyline(pl) = e else { continue };
        let zs = pl.zs.as_ref();
        let level = zs.and_then(|zs| {
            let first = zs.first().copied().flatten()?;
            (zs.len() == pl.pts.len() && zs.iter().all(|z| *z == Some(first)) && first.is_finite()).then_some(first)
        });
        // A polyline with parts carries its height in each part's own vertices: read only one level throughout.
        let parts_level = pl.parts.iter().flatten().all(|part| {
            part.zs.as_ref().is_some_and(|zs| zs.len() == part.pts.len() && zs.iter().all(|z| *z == level))
        });
        let Some(z) = level.filter(|_| parts_level) else {
            unlevelled += 1;
            continue;
        };
        let rings = std::iter::once(&pl.pts).chain(pl.parts.iter().flatten().map(|p| &p.pts));
        for pts in rings {
            let n = pts.len();
            let edges = if pl.closed && n > 2 { n } else { n.saturating_sub(1) };
            for k in 0..edges {
                let (p, q) = (pts[k], pts[(k + 1) % n]);
                if let Some(s) = cross(a, b, l, [p.x, p.y], [q.x, q.y])
                    && !crossings.iter().any(|x| (x[0] - s).abs() <= SAME && x[1] == z)
                {
                    crossings.push([s, z]);
                }
            }
        }
    }
    crossings.sort_by(|x, y| x[0].total_cmp(&y[0]).then(x[1].total_cmp(&y[1])));

    let (ux, uy) = ((b[0] - a[0]) / l, (b[1] - a[1]) / l);
    let mut rows: Vec<SectionPointRow> = measured
        .iter()
        .map(|(uid, at, z)| {
            let (fx, fy) = (at[0] - a[0], at[1] - a[1]);
            let along = fx * ux + fy * uy;
            let offset = (fx * uy - fy * ux).abs();
            let mut row = SectionPointRow {
                uid: uid.clone(),
                at: *at,
                along,
                offset,
                measured: *z,
                contour: None,
                between: None,
                difference: None,
                state: String::new(),
            };
            row.state = if along < 0.0 || along > l {
                "outside".into()
            } else if input.corridor.is_some_and(|c| offset > c) {
                "offLine".into()
            } else {
                let before = crossings.iter().rev().find(|x| x[0] <= along);
                let after = crossings.iter().find(|x| x[0] >= along);
                match (before, after) {
                    (Some(p), Some(q)) => {
                        let h = if q[0] - p[0] == 0.0 { p[1] } else { p[1] + (q[1] - p[1]) * (along - p[0]) / (q[0] - p[0]) };
                        row.contour = Some(h);
                        row.between = Some([p[1], q[1]]);
                        row.difference = Some(z - h);
                        "compared".into()
                    }
                    _ => "noContour".into(),
                }
            };
            row
        })
        .collect();
    rows.sort_by(|x, y| x.along.total_cmp(&y.along));

    // The section's places along it: its ends and its points on it (not outside, not off the line).
    let mut places: Vec<f64> = vec![0.0, l];
    places.extend(rows.iter().filter(|r| r.state == "compared" || r.state == "noContour").map(|r| r.along));
    places.sort_by(f64::total_cmp);
    let gaps: Vec<SectionGap> = match c.spacing {
        Some(most) => places
            .windows(2)
            .filter(|w| w[1] - w[0] > most)
            .map(|w| SectionGap { from: w[0], to: w[1], length: w[1] - w[0] })
            .collect(),
        None => Vec::new(),
    };
    let diffs: Vec<f64> = rows.iter().filter_map(|r| r.difference).collect();
    let s = f64::from(scale);
    let koh_z = c.z_factor * (c.xy[0] * s * s + c.xy[1] * s + c.xy[2]) / 100.0;
    let limit = c.factor * koh_z;
    let rms = (!diffs.is_empty()).then(|| (diffs.iter().map(|d| d * d).sum::<f64>() / diffs.len() as f64).sqrt());
    let largest = diffs.iter().map(|d| d.abs()).reduce(f64::max);
    let within = rms.map(|r| r < limit);
    let mut warnings = Vec::new();
    if within == Some(false) {
        warnings.push(CommandWarning {
            code: codes::SECTION_CHECK_EXCEEDED.into(),
            message: format!(
                "Kesit noktalarının ölçülen kotlarıyla eğrilerden okunan kotlarının farklarının karesel ortalaması {:.3} m, \
                 m. 93(1)(c)'nin sınırını ({:.3} m) aşıyor; eğrileri ve noktaların kotlarını denetleyin.",
                rms.unwrap_or_default(),
                limit
            ),
            path: Some("points".into()),
        });
    }
    if let (Some(most), Some(g)) = (c.spacing, gaps.first()) {
        warnings.push(CommandWarning {
            code: codes::SECTION_SPACING_EXCEEDED.into(),
            message: format!(
                "Kesitte {} yerde iki komşu nokta {most} m'den uzak (ilki {:.2} m ile {:.2} m arasında, {:.2} m); m. 93(1)(b) \
                 noktaları en çok {most} m aralıkla ister.",
                gaps.len(),
                g.from,
                g.to,
                g.length
            ),
            path: Some("points".into()),
        });
    }
    Ok(Checked {
        out: TerrainsSectionChecked {
            length: l,
            ends: [za, zb],
            points: rows,
            crossings,
            unlevelled,
            gaps,
            scale,
            rules: format!("{} {}", rules.id, rules.release),
            compared: diffs.len() as u32,
            rms,
            largest,
            koh_z,
            limit,
            spacing: c.spacing,
            within,
        },
        warnings,
    })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &TerrainsSectionCheck) -> CommandResult<()> {
    match run(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &TerrainsSectionCheck) -> CommandResult<TerrainsSectionChecked> {
    match run(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: c.out, warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: TerrainsSectionCheck) -> CommandResult<TerrainsSectionChecked> {
    plan(cx, &input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_section_crosses_a_segment_where_they_meet() {
        assert_eq!(cross([0.0, 0.0], [10.0, 0.0], 10.0, [4.0, -1.0], [4.0, 1.0]), Some(4.0));
        assert_eq!(cross([0.0, 0.0], [10.0, 0.0], 10.0, [4.0, 1.0], [4.0, 2.0]), None);
        assert_eq!(cross([0.0, 0.0], [10.0, 0.0], 10.0, [0.0, 0.0], [5.0, 0.0]), None);
    }
}
