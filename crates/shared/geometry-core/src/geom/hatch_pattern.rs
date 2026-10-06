//! Hatch patterns (docs/adr/0186): the library's patterns, a pattern's line
//! families as the style engine's hatch paints, its lines and dots cut to a
//! region (explode, previews) and a pattern carried through a turn, a scale
//! or a reflection. A family's definition is in its pattern's units (paper
//! millimetres for the library's), unturned; the hatch's `scale` makes them
//! metres and its `angle` turns them. Every family is anchored at the
//! world's origin, so neighbouring hatches line up across shared edges.

use std::sync::OnceLock;

use crate::api::Op;
use crate::entity::HatchPattern;
use crate::geom::hatch::MAX_HATCH_LINES;
use crate::jsmath::{PI, cos, js_cmp, js_max, js_min, sin, stable_sort};
use crate::op;
use crate::vec2::Vec2;

/// One family of a pattern's lines: lines at `angle` (degrees) through
/// `origin`, each the next `offset` on (`[along, across]` the line), drawn
/// as `dashes` say (plus drawn, minus a gap, 0 a dot; none: whole).
#[derive(Clone, Debug, PartialEq)]
pub struct PatternLine {
    pub angle: f64,
    pub origin: [f64; 2],
    pub offset: [f64; 2],
    pub dashes: Option<Vec<f64>>,
}

crate::json_struct!(PatternLine {
    angle,
    origin,
    offset,
    dashes
});

/// A gradient's shape (`linear`, `cylinder`, `spherical`), whether it runs
/// the other way and its second colour (`#RRGGBB`).
#[derive(Clone, Debug, PartialEq)]
pub struct Gradient {
    pub shape: String,
    pub inverted: Option<bool>,
    pub color2: String,
}

crate::json_struct!(Gradient {
    shape,
    inverted,
    color2
});

/// A pattern of the library: its name, its group (`ansi`, `iso`,
/// `general`), what it is drawn for and its families in paper millimetres.
#[derive(Clone, Debug, PartialEq)]
pub struct LibraryPattern {
    pub name: &'static str,
    pub group: &'static str,
    pub description: &'static str,
    pub lines: Vec<PatternLine>,
}

crate::json_struct!(out LibraryPattern { name, group, description, lines });

/// 1/8 inch, the ANSI patterns' spacing (mm).
const EIGHTH: f64 = 3.175;
/// The ISO patterns' spacing (mm).
const ISO_SPACING: f64 = 5.0;

fn family(angle: f64, origin: [f64; 2], offset: [f64; 2], dashes: &[f64]) -> PatternLine {
    PatternLine {
        angle,
        origin,
        offset,
        dashes: (!dashes.is_empty()).then(|| dashes.to_vec()),
    }
}

/// An ISO 128 line type as a horizontal family (W100: d = 1 mm).
fn iso(name: &'static str, description: &'static str, dashes: &[f64]) -> LibraryPattern {
    LibraryPattern {
        name,
        group: "iso",
        description,
        lines: vec![family(0.0, [0.0, 0.0], [0.0, ISO_SPACING], dashes)],
    }
}

fn ansi(name: &'static str, description: &'static str, lines: Vec<PatternLine>) -> LibraryPattern {
    LibraryPattern {
        name,
        group: "ansi",
        description,
        lines,
    }
}

fn general(
    name: &'static str,
    description: &'static str,
    lines: Vec<PatternLine>,
) -> LibraryPattern {
    LibraryPattern {
        name,
        group: "general",
        description,
        lines,
    }
}

/// The library (docs/adr/0186 §2), in the order the tool offers it.
pub fn library() -> &'static [LibraryPattern] {
    static LIBRARY: OnceLock<Vec<LibraryPattern>> = OnceLock::new();
    LIBRARY.get_or_init(|| {
        let e = EIGHTH;
        // A 45° line through (k·e·√2, 0) lies k·e under the one through the origin.
        let below = |k: f64| [k * e * std::f64::consts::SQRT_2, 0.0];
        vec![
            ansi(
                "ANSI31",
                "Demir, tuğla, taş duvar",
                vec![family(45.0, [0.0, 0.0], [0.0, e], &[])],
            ),
            ansi(
                "ANSI32",
                "Çelik",
                vec![
                    family(45.0, [0.0, 0.0], [0.0, 3.0 * e], &[]),
                    family(45.0, below(1.0), [0.0, 3.0 * e], &[]),
                ],
            ),
            ansi(
                "ANSI33",
                "Bronz, pirinç, bakır",
                vec![
                    family(45.0, [0.0, 0.0], [0.0, 2.0 * e], &[]),
                    family(45.0, below(1.0), [0.0, 2.0 * e], &[e, -e / 2.0]),
                ],
            ),
            ansi(
                "ANSI34",
                "Plastik, kauçuk",
                vec![
                    family(45.0, [0.0, 0.0], [0.0, 6.0 * e], &[]),
                    family(45.0, below(1.0), [0.0, 6.0 * e], &[]),
                    family(45.0, below(2.0), [0.0, 6.0 * e], &[]),
                    family(45.0, below(3.0), [0.0, 6.0 * e], &[]),
                ],
            ),
            ansi(
                "ANSI35",
                "Ateş tuğlası, refrakter malzeme",
                vec![
                    family(45.0, [0.0, 0.0], [0.0, 2.0 * e], &[]),
                    family(
                        45.0,
                        below(1.0),
                        [0.0, 2.0 * e],
                        &[2.5 * e, -e / 2.0, 0.0, -e / 2.0],
                    ),
                ],
            ),
            ansi(
                "ANSI36",
                "Mermer, arduvaz, cam",
                vec![family(
                    45.0,
                    [0.0, 0.0],
                    [1.75 * e, e],
                    &[2.5 * e, -e / 2.0, 0.0, -e / 2.0],
                )],
            ),
            ansi(
                "ANSI37",
                "Kurşun, çinko, magnezyum, yalıtım",
                vec![
                    family(45.0, [0.0, 0.0], [0.0, e], &[]),
                    family(135.0, [0.0, 0.0], [0.0, e], &[]),
                ],
            ),
            ansi(
                "ANSI38",
                "Alüminyum",
                vec![
                    family(45.0, [0.0, 0.0], [0.0, e], &[]),
                    family(135.0, [0.0, 0.0], [2.0 * e, e], &[2.5 * e, -1.5 * e]),
                ],
            ),
            iso("ISO02W100", "Kesikli", &[12.0, -3.0]),
            iso("ISO03W100", "Aralıklı kesikli", &[12.0, -18.0]),
            iso(
                "ISO04W100",
                "Uzun kesikli noktalı",
                &[24.0, -3.0, 0.5, -3.0],
            ),
            iso(
                "ISO05W100",
                "Uzun kesikli iki noktalı",
                &[24.0, -3.0, 0.5, -3.0, 0.5, -3.0],
            ),
            iso(
                "ISO06W100",
                "Uzun kesikli üç noktalı",
                &[24.0, -3.0, 0.5, -3.0, 0.5, -3.0, 0.5, -3.0],
            ),
            iso("ISO07W100", "Noktalı", &[0.5, -3.0]),
            iso(
                "ISO08W100",
                "Uzun ve kısa kesikli",
                &[24.0, -3.0, 6.0, -3.0],
            ),
            iso(
                "ISO09W100",
                "Uzun ve iki kısa kesikli",
                &[24.0, -3.0, 6.0, -3.0, 6.0, -3.0],
            ),
            iso("ISO10W100", "Kesikli noktalı", &[12.0, -3.0, 0.5, -3.0]),
            iso(
                "ISO11W100",
                "İki kesikli noktalı",
                &[12.0, -3.0, 12.0, -3.0, 0.5, -3.0],
            ),
            iso(
                "ISO12W100",
                "Kesikli iki noktalı",
                &[12.0, -3.0, 0.5, -3.0, 0.5, -3.0],
            ),
            iso(
                "ISO13W100",
                "İki kesikli iki noktalı",
                &[12.0, -3.0, 12.0, -3.0, 0.5, -3.0, 0.5, -3.0],
            ),
            iso(
                "ISO14W100",
                "Kesikli üç noktalı",
                &[12.0, -3.0, 0.5, -3.0, 0.5, -3.0, 0.5, -3.0],
            ),
            general(
                "LINE",
                "Yatay çizgiler",
                vec![family(0.0, [0.0, 0.0], [0.0, e], &[])],
            ),
            general(
                "NET",
                "Kare ızgara",
                vec![
                    family(0.0, [0.0, 0.0], [0.0, e], &[]),
                    family(90.0, [0.0, 0.0], [0.0, e], &[]),
                ],
            ),
            general(
                "NET3",
                "Üçgen ızgara",
                vec![
                    family(0.0, [0.0, 0.0], [0.0, e], &[]),
                    family(60.0, [0.0, 0.0], [0.0, e], &[]),
                    family(120.0, [0.0, 0.0], [0.0, e], &[]),
                ],
            ),
            general(
                "DASH",
                "Kesikli çizgiler",
                vec![family(0.0, [0.0, 0.0], [e, e], &[e, -e])],
            ),
            general(
                "DOTS",
                "Noktalar",
                vec![family(
                    0.0,
                    [0.0, 0.0],
                    [e / 4.0, e / 2.0],
                    &[0.0, -e / 2.0],
                )],
            ),
            general(
                "BRICK",
                "Tuğla örgüsü",
                vec![
                    family(0.0, [0.0, 0.0], [0.0, 2.0 * e], &[]),
                    family(90.0, [0.0, 0.0], [2.0 * e, 2.0 * e], &[2.0 * e, -2.0 * e]),
                    family(
                        90.0,
                        [2.0 * e, 0.0],
                        [2.0 * e, 2.0 * e],
                        &[-2.0 * e, 2.0 * e],
                    ),
                ],
            ),
            general(
                "CROSS",
                "Artılar",
                vec![
                    family(0.0, [0.0, 0.0], [2.0 * e, 2.0 * e], &[e, -3.0 * e]),
                    family(
                        90.0,
                        [e / 2.0, -e / 2.0],
                        [2.0 * e, 2.0 * e],
                        &[e, -3.0 * e],
                    ),
                ],
            ),
        ]
    })
}

/// The library's pattern of this name, its case aside.
pub fn library_pattern(name: &str) -> Option<&'static LibraryPattern> {
    library().iter().find(|p| p.name.eq_ignore_ascii_case(name))
}

/// A family's dashes as drawn: whole, never (gaps only) or alternating
/// lengths starting with a drawn one (a dot is drawn, 0 long), and how far
/// the pattern's start moved to make it so.
#[derive(Clone, Debug, PartialEq)]
pub enum Dashes {
    Whole,
    Never,
    Runs { runs: Vec<f64>, shift: f64 },
}

/// The most dashes the drawing draws of a family (the shader's eight).
pub const DRAWN_DASHES: usize = 8;

/// A family's signed dashes as runs (docs/adr/0186 §3): neighbours of a kind
/// joined, a run across the pattern's end joined with the first, a gap
/// first moved to the end; `shift` is what the dash phase gains for it.
/// The first eight runs are kept.
pub fn dash_runs(signed: &[f64]) -> Dashes {
    if signed.is_empty() {
        return Dashes::Whole;
    }
    let mut runs: Vec<(bool, f64)> = Vec::with_capacity(signed.len());
    for &x in signed {
        let on = x >= 0.0;
        match runs.last_mut() {
            Some((kind, len)) if *kind == on => *len += x.abs(),
            _ => runs.push((on, x.abs())),
        }
    }
    if runs.iter().all(|(on, _)| *on) {
        return Dashes::Whole;
    }
    if runs.iter().all(|(on, _)| !*on) {
        return Dashes::Never;
    }
    let mut shift = 0.0;
    // The run across the end joins the first: the pattern starts that much earlier.
    if runs.len() > 1 && runs[0].0 == runs[runs.len() - 1].0 {
        let (_, len) = runs.pop().unwrap_or((true, 0.0));
        runs[0].1 += len;
        shift += len;
    }
    // A gap first goes to the end: the pattern starts after it.
    if !runs[0].0 {
        let gap = runs.remove(0);
        shift -= gap.1;
        runs.push(gap);
    }
    let mut lens: Vec<f64> = runs.into_iter().map(|(_, len)| len).collect();
    lens.truncate(DRAWN_DASHES);
    Dashes::Runs { runs: lens, shift }
}

/// A family as the style engine's hatch paint: lines at `angle` (degrees)
/// `spacing` apart, the one through the anchor `offset` across; a line
/// `stagger` further along than the one before; `dash` alternating drawn
/// and gap lengths, its phase `dashOffset` (all metres).
#[derive(Clone, Debug, PartialEq)]
pub struct FamilyPaint {
    pub angle: f64,
    pub spacing: f64,
    pub offset: f64,
    pub stagger: f64,
    pub dash: Option<Vec<f64>>,
    pub dash_offset: f64,
}

crate::json_struct!(out FamilyPaint { angle, spacing, offset, stagger, dash, dash_offset => "dashOffset" });

fn user_family(angle: f64, spacing: f64) -> FamilyPaint {
    FamilyPaint {
        angle,
        spacing,
        offset: 0.0,
        stagger: 0.0,
        dash: None,
        dash_offset: 0.0,
    }
}

/// A pattern's families as hatch paints (docs/adr/0186 §3), in metres and
/// turned; none for a solid or a gradient, nor for a pattern without a scale.
pub fn paints(p: &HatchPattern) -> Vec<FamilyPaint> {
    match p.kind.as_str() {
        "lines" if p.spacing > 0.0 => vec![user_family(p.angle, p.spacing)],
        "cross" if p.spacing > 0.0 => vec![
            user_family(p.angle, p.spacing),
            user_family(p.angle + 90.0, p.spacing),
        ],
        "pattern" => {
            let scale = p.scale.unwrap_or(0.0);
            if !(scale > 0.0 && scale.is_finite()) {
                return Vec::new();
            }
            let turn = p.angle * PI / 180.0;
            let (tc, ts) = (cos(turn), sin(turn));
            let mut out = Vec::new();
            for l in p.lines.as_deref().unwrap_or_default() {
                let dy = l.offset[1] * scale;
                if !(dy.abs() > 0.0) {
                    continue;
                }
                let angle = l.angle + p.angle;
                let r = angle * PI / 180.0;
                let (c, s) = (cos(r), sin(r));
                let (ox, oy) = (l.origin[0] * scale, l.origin[1] * scale);
                let o = Vec2::new(ox * tc - oy * ts, ox * ts + oy * tc);
                let offset = -s * o.x + c * o.y;
                let scaled: Vec<f64> = l
                    .dashes
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .map(|d| d * scale)
                    .collect();
                let paint = match dash_runs(&scaled) {
                    Dashes::Never => continue,
                    Dashes::Whole => FamilyPaint {
                        angle,
                        spacing: dy.abs(),
                        offset,
                        stagger: 0.0,
                        dash: None,
                        dash_offset: 0.0,
                    },
                    Dashes::Runs { runs, shift } => FamilyPaint {
                        angle,
                        spacing: dy.abs(),
                        offset,
                        stagger: if dy < 0.0 { -1.0 } else { 1.0 } * l.offset[0] * scale,
                        dash: Some(runs),
                        dash_offset: -(c * o.x + s * o.y) + shift,
                    },
                };
                out.push(paint);
            }
            out
        }
        _ => Vec::new(),
    }
}

/// A pattern's lines and dots cut to a region: what Patlat writes and a
/// preview draws. `capped`: more than the budget, none given.
#[derive(Clone, Debug, PartialEq)]
pub struct HatchPieces {
    pub segments: Vec<[Vec2; 2]>,
    pub dots: Vec<Vec2>,
    pub capped: bool,
}

crate::json_struct!(out HatchPieces { segments, dots, capped });

#[derive(Clone, Copy)]
struct Local {
    u: f64,
    v: f64,
}

/// The pieces of one family inside the rings (even–odd), appended; false
/// when they pass `budget` in all.
fn family_pieces(rings: &[&[Vec2]], f: &FamilyPaint, budget: usize, out: &mut HatchPieces) -> bool {
    if !(f.spacing > 0.0) {
        return true;
    }
    let r = f.angle * PI / 180.0;
    let (c, s) = (cos(r), sin(r));
    let to_local = |p: &Vec2| Local {
        u: p.x * c + p.y * s,
        v: -p.x * s + p.y * c,
    };
    let to_world = |u: f64, v: f64| Vec2::new(u * c - v * s, u * s + v * c);
    let local: Vec<Vec<Local>> = rings
        .iter()
        .filter(|r| r.len() >= 3)
        .map(|r| r.iter().map(to_local).collect())
        .collect();
    let Some(outer) = local.first() else {
        return true;
    };
    let (mut min_v, mut max_v) = (f64::INFINITY, f64::NEG_INFINITY);
    for p in outer {
        min_v = js_min(min_v, p.v);
        max_v = js_max(max_v, p.v);
    }
    let first = ((min_v - f.offset) / f.spacing).ceil();
    let last = ((max_v - f.offset) / f.spacing).floor();
    if last - first + 1.0 > MAX_HATCH_LINES {
        return false;
    }
    let period: f64 = f.dash.as_deref().map_or(0.0, |d| d.iter().sum());
    let mut xs: Vec<f64> = Vec::new();
    let mut k = first;
    while k <= last {
        let v = f.offset + k * f.spacing;
        xs.clear();
        for ring in &local {
            let mut j = ring.len() - 1;
            for i in 0..ring.len() {
                let (a, b) = (ring[j], ring[i]);
                // Half-open rule so a line through a vertex is counted once.
                if (a.v <= v) != (b.v <= v) {
                    xs.push(a.u + ((v - a.v) / (b.v - a.v)) * (b.u - a.u));
                }
                j = i;
            }
        }
        stable_sort(&mut xs, &mut |p, q| js_cmp(*p - *q, 0.0));
        let mut i = 0;
        while i + 1 < xs.len() {
            let (u0, u1) = (xs[i], xs[i + 1]);
            i += 2;
            if u1 - u0 <= 1e-9 {
                continue;
            }
            let Some(dash) = f.dash.as_deref().filter(|_| period > 0.0) else {
                out.segments.push([to_world(u0, v), to_world(u1, v)]);
                if out.segments.len() + out.dots.len() > budget {
                    return false;
                }
                continue;
            };
            // The pattern's phase on this line: t = u + dashOffset − k·stagger.
            let shift = f.dash_offset - k * f.stagger;
            let mut j = ((u0 + shift) / period).floor();
            while j * period - shift < u1 {
                let mut at = j * period - shift;
                for (n, &len) in dash.iter().enumerate() {
                    if n % 2 == 0 {
                        let (a, b) = (at, at + len);
                        if len == 0.0 {
                            if a >= u0 && a <= u1 {
                                out.dots.push(to_world(a, v));
                            }
                        } else if b > u0 && a < u1 {
                            let (a, b) = (js_max(a, u0), js_min(b, u1));
                            if b - a > 1e-9 {
                                out.segments.push([to_world(a, v), to_world(b, v)]);
                            }
                        }
                        if out.segments.len() + out.dots.len() > budget {
                            return false;
                        }
                    }
                    at += len;
                }
                j += 1.0;
            }
        }
        k += 1.0;
    }
    true
}

/// A pattern's lines and dots inside a ring and its holes (even–odd), at
/// most `budget` of them.
pub fn pattern_pieces(
    ring: &[Vec2],
    holes: &[Vec<Vec2>],
    p: &HatchPattern,
    budget: usize,
) -> HatchPieces {
    let mut out = HatchPieces {
        segments: Vec::new(),
        dots: Vec::new(),
        capped: false,
    };
    let rings: Vec<&[Vec2]> = std::iter::once(ring)
        .chain(holes.iter().map(Vec::as_slice))
        .collect();
    for f in paints(p) {
        if !family_pieces(&rings, &f, budget, &mut out) {
            return HatchPieces {
                segments: Vec::new(),
                dots: Vec::new(),
                capped: true,
            };
        }
    }
    out
}

/// Whether a pattern's lines over a ring are too many to draw or explode
/// (more than `MAX_HATCH_LINES` in a family; the tools refuse it, as ADR 0062's).
pub fn too_dense(ring: &[Vec2], p: &HatchPattern) -> bool {
    paints(p).iter().any(|f| {
        let r = f.angle * PI / 180.0;
        let (c, s) = (cos(r), sin(r));
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for q in ring {
            let v = -q.x * s + q.y * c;
            lo = js_min(lo, v);
            hi = js_max(hi, v);
        }
        f.spacing > 0.0 && (hi - lo) / f.spacing + 1.0 > MAX_HATCH_LINES
    })
}

/// An angle in degrees from 0 up to 360.
pub fn turn_of(a: f64) -> f64 {
    let t = a % 360.0;
    if t < 0.0 { t + 360.0 } else { t }
}

/// A pattern's family reflected in its own x axis (docs/adr/0186 §8).
fn reflected(l: &PatternLine) -> PatternLine {
    PatternLine {
        angle: turn_of(-l.angle),
        origin: [l.origin[0], -l.origin[1]],
        offset: [l.offset[0], -l.offset[1]],
        dashes: l.dashes.clone(),
    }
}

/// A pattern or a gradient carried through a similarity (docs/adr/0186 §8):
/// turned by `turn` degrees, its lengths times `scale`, reflected first in
/// the x axis when `reflect`. A user-defined pattern's angle and spacing are
/// the caller's (they follow the lines' own direction).
pub fn carried(p: &HatchPattern, turn: f64, scale: f64, reflect: bool) -> HatchPattern {
    let mut out = p.clone();
    match p.kind.as_str() {
        "pattern" => {
            out.angle = turn_of(if reflect {
                turn - p.angle
            } else {
                turn + p.angle
            });
            out.scale = p.scale.map(|s| s * scale);
            if reflect {
                out.lines = p
                    .lines
                    .as_ref()
                    .map(|ls| ls.iter().map(reflected).collect());
            }
        }
        "gradient" => {
            out.angle = turn_of(if reflect {
                turn - p.angle
            } else {
                turn + p.angle
            });
        }
        _ => {}
    }
    out
}

pub(crate) static OPS: &[Op] = &[
    op!("hatchPatterns", || library().to_vec()),
    op!("hatchPaints", |pattern: HatchPattern| paints(&pattern)),
    op!("hatchTooDense", |ring: Vec<Vec2>, pattern: HatchPattern| {
        too_dense(&ring, &pattern)
    }),
    op!(
        "hatchPatternPieces",
        |ring: Vec<Vec2>, holes: Option<Vec<Vec<Vec2>>>, pattern: HatchPattern, budget: f64| {
            pattern_pieces(
                &ring,
                &holes.unwrap_or_default(),
                &pattern,
                if budget.is_finite() && budget > 0.0 {
                    budget as usize
                } else {
                    0
                },
            )
        }
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dash_runs_start_with_a_drawn_run() {
        assert_eq!(dash_runs(&[]), Dashes::Whole);
        assert_eq!(dash_runs(&[2.0, 1.0]), Dashes::Whole);
        assert_eq!(dash_runs(&[-2.0, -1.0]), Dashes::Never);
        assert_eq!(
            dash_runs(&[12.0, -3.0]),
            Dashes::Runs {
                runs: vec![12.0, 3.0],
                shift: 0.0
            }
        );
        // A gap first: it goes to the end, the pattern starts after it.
        assert_eq!(
            dash_runs(&[-2.0, 2.0]),
            Dashes::Runs {
                runs: vec![2.0, 2.0],
                shift: -2.0
            }
        );
        // The run across the end joins the first.
        assert_eq!(
            dash_runs(&[5.0, -2.0, 3.0]),
            Dashes::Runs {
                runs: vec![8.0, 2.0],
                shift: 3.0
            }
        );
        // A dot is drawn, 0 long.
        assert_eq!(
            dash_runs(&[0.0, -1.5]),
            Dashes::Runs {
                runs: vec![0.0, 1.5],
                shift: 0.0
            }
        );
    }

    #[test]
    fn the_library_holds_its_names_once() {
        let names: std::collections::HashSet<&str> = library().iter().map(|p| p.name).collect();
        assert_eq!(names.len(), library().len());
        assert_eq!(library().len(), 28);
        assert!(library_pattern("ansi31").is_some());
        for p in library() {
            for l in &p.lines {
                assert!(l.offset[1] != 0.0, "{}", p.name);
                assert!(
                    l.dashes.as_ref().is_none_or(|d| d.len() <= DRAWN_DASHES),
                    "{}",
                    p.name
                );
            }
        }
    }
}
