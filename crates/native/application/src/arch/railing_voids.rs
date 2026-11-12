//! Döşeme boşluğunun korkuluk isteyen kenarları (docs/adr/8045 §3, §4): a slab's void's sides a railing guards, read the
//! same by Korkuluk's Boşluktan and by Denetim › Mevzuat (PAİY 38/1: "galeri boşluğu … korkuluk yapılması mecburidir").
//!
//! - Its free sides: those no wall's face runs along, as a balcony's railing (docs/adr/0722 §4); the void is read as a
//!   slab of its own, its runs counter-clockwise, the void to their left.
//! - Open where a stair from the storey below arrives: the top riser's line of every straight-ended stair on that
//!   storey, the band's end across its walking line's last point, within 2 cm of a side.
//! - No stretch shorter than 0.10 m: PAİY 38/4's 0.10 m passes through no such gap.

use kentos_commands::{Entity, RAILING_GAP, SlabFields, SpacePart, SpaceRing};
use kentos_domain::Document;
use kentos_geometry_core::Vec2 as V;

/// A line and whether it closes.
pub type Piece = (Vec<V>, bool);

/// How close a stair's top riser runs to a void's side to leave it open, metres.
pub const ARRIVAL: f64 = 0.02;

/// The top riser's line of every straight-ended stair on the storey under `level`: where it arrives on `level`'s floor.
pub fn arrivals(doc: &Document, level: Option<&str>) -> Vec<(V, V)> {
    let Some(active) = level else {
        return Vec::new();
    };
    let levels = doc
        .settings()
        .arch
        .as_ref()
        .map(|a| a.levels.clone())
        .unwrap_or_default();
    let Some(me) = levels.iter().find(|l| l.id == active) else {
        return Vec::new();
    };
    let Some(below) = levels
        .iter()
        .filter(|l| l.elevation < me.elevation)
        .max_by(|a, b| a.elevation.total_cmp(&b.elevation))
    else {
        return Vec::new();
    };
    doc.entities()
        .filter_map(|e| match e {
            Entity::Stair(s)
                if s.stair.arch.level.as_deref() == Some(below.id.as_str())
                    && s.stair
                        .bulges
                        .as_ref()
                        .is_none_or(|b| b.iter().all(|x| *x == 0.0)) =>
            {
                let p = &s.stair.pts;
                let (a, b) = (p.get(p.len().checked_sub(2)?)?, p.last()?);
                let l = (b.x - a.x).hypot(b.y - a.y);
                (l > 0.0).then(|| {
                    let (nx, ny) = (-(b.y - a.y) / l, (b.x - a.x) / l);
                    let h = s.stair.width / 2.0;
                    (
                        V::new(b.x - nx * h, b.y - ny * h),
                        V::new(b.x + nx * h, b.y + ny * h),
                    )
                })
            }
            _ => None,
        })
        .collect()
}

/// `run` (`closed`: round to its first point) without the stretches along any of `cuts` (within [`ARRIVAL`] of them,
/// both ways): the pieces left, of two points or more; a closed run cut is open pieces.
pub fn cut_run(run: &[V], closed: bool, cuts: &[(V, V)]) -> Vec<Piece> {
    let n = run.len();
    let count = if closed { n } else { n.saturating_sub(1) };
    // Each segment's stretches cut away, along it from its start.
    let removed: Vec<Vec<(f64, f64)>> = (0..count)
        .map(|i| {
            let (a, b) = (run[i], run[(i + 1) % n]);
            let l = (b.x - a.x).hypot(b.y - a.y);
            if l <= 0.0 {
                return Vec::new();
            }
            let (ux, uy) = ((b.x - a.x) / l, (b.y - a.y) / l);
            let along = |p: V| (p.x - a.x) * ux + (p.y - a.y) * uy;
            let off = |p: V| ((p.x - a.x) * uy - (p.y - a.y) * ux).abs();
            let mut out: Vec<(f64, f64)> = cuts
                .iter()
                .filter(|(p, q)| off(*p) <= ARRIVAL && off(*q) <= ARRIVAL)
                .filter_map(|(p, q)| {
                    let (s0, s1) = (
                        along(*p).min(along(*q)).max(0.0),
                        along(*p).max(along(*q)).min(l),
                    );
                    (s1 - s0 > ARRIVAL).then_some((s0, s1))
                })
                .collect();
            out.sort_by(|x, y| x.0.total_cmp(&y.0));
            out
        })
        .collect();
    if removed.iter().all(Vec::is_empty) {
        return vec![(run.to_vec(), closed)];
    }
    // Walk it, a closed one from the start of a segment with a cut so no piece wraps unseen.
    let start = if closed {
        removed.iter().position(|r| !r.is_empty()).unwrap_or(0)
    } else {
        0
    };
    let mut pieces: Vec<Vec<V>> = Vec::new();
    let mut now: Vec<V> = Vec::new();
    for k in 0..count {
        let i = (start + k) % count.max(1);
        let (a, b) = (run[i], run[(i + 1) % n]);
        let l = (b.x - a.x).hypot(b.y - a.y);
        let at = |t: f64| V::new(a.x + (b.x - a.x) * t / l, a.y + (b.y - a.y) * t / l);
        let mut from = 0.0;
        for &(s0, s1) in &removed[i] {
            if s0 > from {
                if now.is_empty() {
                    now.push(at(from));
                }
                now.push(at(s0));
            }
            if now.len() >= 2 {
                pieces.push(std::mem::take(&mut now));
            }
            now.clear();
            from = f64::max(from, s1);
        }
        if from < l {
            if now.is_empty() {
                now.push(at(from));
            }
            now.push(b);
        }
    }
    if now.len() >= 2 {
        pieces.push(now);
    }
    pieces.into_iter().map(|p| (p, false)).collect()
}

fn length(line: &[V]) -> f64 {
    line.windows(2)
        .map(|w| (w[1].x - w[0].x).hypot(w[1].y - w[0].y))
        .sum()
}

/// The sides of `slab`'s void `hole` a railing guards, the slab on `level`: its free sides, open where a stair from the
/// storey below arrives, no stretch under 0.10 m; each counter-clockwise round the void (the void to its left). None for
/// a void with arcs (its sides are drawn by hand).
pub fn void_sides(
    doc: &Document,
    slab: &SlabFields,
    hole: &SpaceRing,
    level: Option<&str>,
) -> Vec<Piece> {
    if hole
        .bulges
        .as_ref()
        .is_some_and(|b| b.iter().any(|x| *x != 0.0))
    {
        return Vec::new();
    }
    let probe = SlabFields {
        parts: vec![SpacePart {
            outer: hole.clone(),
            holes: Vec::new(),
        }],
        arch: slab.arch.clone(),
        ..Default::default()
    };
    let cuts = arrivals(doc, level);
    let mut out: Vec<Piece> = Vec::new();
    for run in doc.slab_runs(&probe) {
        let mut pts: Vec<V> = run.pts.iter().map(|p| V::new(p.x, p.y)).collect();
        let closed = pts.len() > 3 && pts.first() == pts.last();
        if closed {
            pts.pop();
        }
        for (piece, shut) in cut_run(&pts, closed, &cuts) {
            if shut || length(&piece) >= RAILING_GAP {
                out.push((piece, shut));
            }
        }
    }
    out
}

/// [`void_sides`] as railings' lines, the middle of a top rail `width` wide: each moved half its width onto the slab
/// (right of its run), its face on the void's edge.
pub fn void_railings(
    doc: &Document,
    slab: &SlabFields,
    hole: &SpaceRing,
    level: Option<&str>,
    width: f64,
) -> Vec<Piece> {
    void_sides(doc, slab, hole, level)
        .into_iter()
        .filter_map(|(piece, shut)| {
            let line = kentos_geometry_core::geom::offset::offset_path(&piece, -width / 2.0, shut);
            (line.len() >= if shut { 3 } else { 2 }).then_some((line, shut))
        })
        .collect()
}
