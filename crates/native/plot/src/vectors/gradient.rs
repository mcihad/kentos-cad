//! A gradient fill as vectors (docs/adr/0333 §1): bands of one colour each, cut to the batch's triangles, coloured as
//! the renderers' `gradientFs` colours their middle. Linear and cylinder gradients are cut along their direction into
//! `BANDS` stripes of the gradient's parameter (the first and last reach on, as the shader clamps); a spherical one into
//! rings around its centre (between two regular `RING`-gons, in convex sectors) and what lies past its radius. Every
//! part is convex and a fill of its own: no part is a hole. A band's colour differs from
//! the shader's at any of its points by at most half a band's step (1/128 of the two colours' difference; a cylinder has
//! twice the stripes, its parameter running twice as fast).

use kentos_sheet::pdf::MapStroke;
use kentos_sheet::style::{LineCap, LineJoin};

use super::{Part, Rule, Shape, color};

/// The edge an opaque band is drawn with in its own colour, paper mm: it covers the hairline seams a viewer's
/// anti-aliasing leaves between pieces that meet (a band's colour on its neighbour's side by half of it).
const SEAM_MM: f64 = 0.08;

/// The bands a gradient is cut into.
pub(super) const BANDS: usize = 64;
/// The corners of a ring's circles.
const RING: usize = 128;

/// The convex polygon `p` where `a·q + b ≥ 0` (Sutherland–Hodgman).
fn keep(p: &[[f64; 2]], a: [f64; 2], b: f64) -> Vec<[f64; 2]> {
    let side = |q: [f64; 2]| a[0] * q[0] + a[1] * q[1] + b;
    let mut out = Vec::with_capacity(p.len() + 2);
    for i in 0..p.len() {
        let (q, r) = (p[i], p[(i + 1) % p.len()]);
        let (sq, sr) = (side(q), side(r));
        if sq >= 0.0 {
            out.push(q);
        }
        if (sq >= 0.0) != (sr >= 0.0) {
            let t = sq / (sq - sr);
            out.push([q[0] + t * (r[0] - q[0]), q[1] + t * (r[1] - q[1])]);
        }
    }
    out
}

/// The convex polygon `p` inside the convex polygon `clip` (counter-clockwise; edges of no length are passed over).
fn within(p: &[[f64; 2]], clip: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut out = p.to_vec();
    for i in 0..clip.len() {
        if out.len() < 3 {
            return Vec::new();
        }
        let (q, r) = (clip[i], clip[(i + 1) % clip.len()]);
        let a = [q[1] - r[1], r[0] - q[0]];
        if a[0] == 0.0 && a[1] == 0.0 {
            continue;
        }
        out = keep(&out, a, -(a[0] * q[0] + a[1] * q[1]));
    }
    out
}

/// Whether two boxes `[min x, min y, max x, max y]` meet.
fn meet(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3]
}

fn bounds(p: &[[f64; 2]]) -> [f64; 4] {
    p.iter().fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |b, q| {
            [
                b[0].min(q[0]),
                b[1].min(q[1]),
                b[2].max(q[0]),
                b[3].max(q[1]),
            ]
        },
    )
}

/// Twice an outline's area, unsigned.
fn twice_area(p: &[[f64; 2]]) -> f64 {
    super::signed(p).abs()
}

/// The colour at the gradient's parameter `t` (0 the first colour): the shader's `mix`.
fn mix(a: [f64; 4], b: [f64; 4], t: f64) -> [f64; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

/// What a gradient is.
pub(super) struct Gradient {
    pub color: [f64; 4],
    pub color2: [f64; 4],
    /// 0 linear, 1 cylinder, 2 spherical.
    pub shape: u32,
    pub inverted: bool,
    /// Radians.
    pub dir: f64,
    pub from: f64,
    pub to: f64,
    pub centre: [f64; 2],
    pub radius: f64,
}

/// The gradient over the triangles `t` (the batch's numbers) as filled bands, moved by `(bx, by)`.
pub(super) fn bands(t: &[f32], g: &Gradient, bx: f64, by: f64) -> Vec<Shape> {
    let triangles: Vec<[[f64; 2]; 3]> = t
        .chunks_exact(6)
        .map(|c| {
            [
                [f64::from(c[0]), f64::from(c[1])],
                [f64::from(c[2]), f64::from(c[3])],
                [f64::from(c[4]), f64::from(c[5])],
            ]
        })
        .collect();
    let paint = |t: f64| {
        let t = if g.inverted { 1.0 - t } else { t };
        let c = mix(g.color, g.color2, t);
        (color(c, c[3]), Rule::EvenOdd)
    };
    // Only an opaque band takes its seam edge: a see-through one would be darker along it.
    let seam = |fill: &(String, Rule)| {
        (g.color[3] >= 0.999 && g.color2[3] >= 0.999).then(|| MapStroke {
            color: fill.0.clone(),
            width: SEAM_MM,
            dash: Vec::new(),
            cap: LineCap::Butt,
            join: LineJoin::Miter,
        })
    };
    let moved = |p: Vec<[f64; 2]>| Part {
        points: p.into_iter().map(|q| [q[0] + bx, q[1] + by]).collect(),
        closed: true,
    };
    let mut out = Vec::with_capacity(2 * BANDS + 1);
    if g.shape == 2 {
        // Rings: t = 1 − r/R, then 0 past R. Each ring is cut into the RING sectors of its two regular RING-gons
        // (convex pieces, so no part is a hole); past R, the sectors reach out beyond every triangle.
        let r = g.radius.max(1e-9);
        let c = g.centre;
        let all = triangles.iter().fold(
            [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ],
            |b, t| {
                let tb = bounds(t);
                [
                    b[0].min(tb[0]),
                    b[1].min(tb[1]),
                    b[2].max(tb[2]),
                    b[3].max(tb[3]),
                ]
            },
        );
        let reach = (all[2] - c[0]).abs().max((all[0] - c[0]).abs())
            + (all[3] - c[1]).abs().max((all[1] - c[1]).abs())
            + r;
        let corner = |k: usize, radius: f64| {
            let angle = k as f64 * std::f64::consts::TAU / RING as f64;
            [c[0] + radius * angle.cos(), c[1] + radius * angle.sin()]
        };
        let boxes: Vec<[f64; 4]> = triangles.iter().map(|t| bounds(t)).collect();
        for k in 0..=BANDS {
            let (r0, r1) = if k == BANDS {
                (r, 2.0 * reach)
            } else {
                (
                    r * k as f64 / BANDS as f64,
                    r * (k + 1) as f64 / BANDS as f64,
                )
            };
            let mut parts = Vec::new();
            for s in 0..RING {
                let piece = [
                    corner(s, r1),
                    corner(s + 1, r1),
                    corner(s + 1, r0),
                    corner(s, r0),
                ];
                let pb = bounds(&piece);
                for (tri, tb) in triangles.iter().zip(&boxes) {
                    if !meet(pb, *tb) {
                        continue;
                    }
                    let p = within(tri, &piece);
                    if p.len() >= 3 && twice_area(&p) > 0.0 {
                        parts.push(moved(p));
                    }
                }
            }
            if !parts.is_empty() {
                let t = if k == BANDS {
                    0.0
                } else {
                    1.0 - (k as f64 + 0.5) / BANDS as f64
                };
                let fill = paint(t);
                out.push(Shape {
                    parts,
                    stroke: seam(&fill),
                    fill: Some(fill),
                });
            }
        }
        return out;
    }
    // Stripes of u = (d·p − from) / max(to − from, 1e-9), clamped (the shader's): t = u, or 1 − |2u − 1| along a
    // cylinder.
    let den = (g.to - g.from).max(1e-9);
    let d = [g.dir.cos(), g.dir.sin()];
    let (a, b) = ([d[0] / den, d[1] / den], -g.from / den);
    // Along a cylinder t runs twice as fast: twice the stripes keep each band's step.
    let n = if g.shape == 1 { 2 * BANDS } else { BANDS };
    for k in 0..n {
        let (lo, hi) = (k as f64 / n as f64, (k + 1) as f64 / n as f64);
        let mut parts = Vec::new();
        for tri in &triangles {
            let mut p = tri.to_vec();
            if k > 0 {
                p = keep(&p, a, b - lo);
            }
            if k + 1 < n && p.len() >= 3 {
                p = keep(&p, [-a[0], -a[1]], hi - b);
            }
            if p.len() >= 3 && twice_area(&p) > 0.0 {
                parts.push(moved(p));
            }
        }
        if !parts.is_empty() {
            let u = (k as f64 + 0.5) / n as f64;
            let t = if g.shape == 1 {
                1.0 - (2.0 * u - 1.0).abs()
            } else {
                u
            };
            let fill = paint(t);
            out.push(Shape {
                parts,
                stroke: seam(&fill),
                fill: Some(fill),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 100 m square as two triangles, its numbers as a batch holds them.
    const SQUARE: [f32; 12] = [
        0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 0.0, 100.0, 100.0, 0.0, 100.0,
    ];

    fn gradient(shape: u32, inverted: bool) -> Gradient {
        Gradient {
            color: [0.0, 0.0, 0.0, 1.0],
            color2: [1.0, 1.0, 1.0, 1.0],
            shape,
            inverted,
            dir: 0.0,
            from: 0.0,
            to: 100.0,
            centre: [50.0, 50.0],
            radius: 40.0,
        }
    }

    /// The renderers' `gradientFs` at a point: the grey it gives.
    fn shader(g: &Gradient, p: [f64; 2]) -> f64 {
        let t = if g.shape == 2 {
            let d = ((p[0] - g.centre[0]).powi(2) + (p[1] - g.centre[1]).powi(2)).sqrt();
            1.0 - (d / g.radius.max(1e-9)).clamp(0.0, 1.0)
        } else {
            let u = ((p[0] * g.dir.cos() + p[1] * g.dir.sin() - g.from)
                / (g.to - g.from).max(1e-9))
            .clamp(0.0, 1.0);
            if g.shape == 1 {
                1.0 - (2.0 * u - 1.0).abs()
            } else {
                u
            }
        };
        if g.inverted { 1.0 - t } else { t }
    }

    fn grey(fill: &str) -> f64 {
        f64::from(u8::from_str_radix(&fill[1..3], 16).expect("hex")) / 255.0
    }

    fn contains(p: &[[f64; 2]], q: [f64; 2]) -> bool {
        super::super::inside(q[0], q[1], p)
    }

    /// Every band of every shape covers the square once, and at each sample point the band's grey is the shader's
    /// within half a band's step and the colour's rounding.
    #[test]
    fn the_bands_are_the_shader_s_colours_and_cover_the_region_once() {
        for (shape, inverted) in [(0, false), (1, false), (2, false), (2, true), (0, true)] {
            let g = gradient(shape, inverted);
            let shapes = bands(&SQUARE, &g, 0.0, 0.0);
            let area: f64 = shapes
                .iter()
                .flat_map(|s| &s.parts)
                .map(|p| twice_area(&p.points) / 2.0)
                .sum();
            assert!((area - 10_000.0).abs() < 1e-6, "shape {shape}: area {area}");
            for i in 0..20 {
                for j in 0..20 {
                    // Off the sectors' edges (the diagonals are edges of a 128-gon's sectors).
                    let q = [2.5137 + 5.0 * f64::from(i), 2.5071 + 5.0 * f64::from(j)];
                    let hit: Vec<&Shape> = shapes
                        .iter()
                        .filter(|s| s.parts.iter().any(|p| contains(&p.points, q)))
                        .collect();
                    assert_eq!(hit.len(), 1, "shape {shape}: {q:?} in {} bands", hit.len());
                    let (fill, _) = hit[0].fill.as_ref().expect("filled");
                    let want = shader(&g, q);
                    assert!(
                        (grey(fill) - want).abs() <= 0.5 / BANDS as f64 + 0.5 / 255.0 + 1e-9,
                        "shape {shape}, inverted {inverted}: {q:?} {} against {want}",
                        grey(fill)
                    );
                }
            }
        }
    }

    /// The batch's tile moves every point.
    #[test]
    fn the_bands_are_moved_with_the_batch() {
        let shapes = bands(&SQUARE, &gradient(0, false), 1000.0, 2000.0);
        assert!(
            shapes
                .iter()
                .flat_map(|s| &s.parts)
                .flat_map(|p| &p.points)
                .all(|q| q[0] >= 1000.0 && q[1] >= 2000.0)
        );
    }
}
