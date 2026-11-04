//! A corridor's adaptive sampling (docs/adr/0939, plan/road.md Y33; ROAD-COR-08, 09): how far the straight join of
//! two sections strays from the section built between them, and the stations added until it strays no more than the
//! tolerances allow.
//!
//! Between two stations the corridor joins each feature line's points straight and its top surface by a strip of
//! triangles (`crate::corridor`). Where the road curves, its profile bends, its crossfall turns or the terrain the
//! slopes meet is uneven, the section that would be built in between is not on that join. Two ways bound it:
//!
//! - **Beforehand, the profile:** on a vertical curve whose second derivative is at most `k` in size the straight join
//!   over a horizontal step `h` strays at most `k·h²/8` (the interpolation error of a line; exact on a parabola,
//!   whose `k` is constant), so steps of `√(8t/k)` keep it within `t` (OpenRoads's "Densify Vertical", Civil 3D's
//!   profile curve increment). [`vertical_step`].
//! - **Afterwards, everything:** the section in the middle of each interval is built and its coded points compared
//!   with the straight join of the ends' ([`stray`]): the plan distance to the chord and the height off it. Over a
//!   tolerance the interval is halved, down to the least step ([`refine`]). A section whose points differ in kind
//!   from its neighbours' (cut to fill, a slope that meets nothing, no section) halves it too, so the change is found
//!   within the least step. This measures the terrain the slopes meet, which no rule beforehand can: the middle is
//!   measured, not bounded, and a bump narrower than the halved step can still hide between two measured places.

/// The least a step is cut to.
pub const LEAST_STEP: f64 = 0.01;

/// The horizontal step that keeps the straight join of a vertical curve within `tolerance` (m) of it, where its
/// second derivative is at most `bend` in size (1/m): `√(8t/k)`; infinite on a straight grade.
pub fn vertical_step(tolerance: f64, bend: f64) -> f64 {
    if bend.abs() <= 0.0 { f64::INFINITY } else { (8.0 * tolerance / bend.abs()).sqrt() }
}

/// How far a section strays from the straight join of its neighbours: the largest plan distance of a coded point
/// from the chord between the same code's points (and that code) and the largest height off the chord at the point's
/// place along it (and that code); `changed` when the three sections do not have the same codes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stray {
    pub plan: f64,
    pub plan_code: Option<String>,
    pub height: f64,
    pub height_code: Option<String>,
    pub changed: bool,
}

/// A coded section: its points (code, x, y, z), none where there is no section.
pub type Coded = Option<Vec<(String, [f64; 3])>>;

/// How far `middle` strays from the straight join of `a` and `b` (see [`Stray`]). A code's place along the chord is
/// its plan projection on it, kept within the chord's ends.
pub fn stray(a: &Coded, middle: &Coded, b: &Coded) -> Stray {
    let (Some(a), Some(m), Some(b)) = (a, middle, b) else {
        return Stray { changed: !(a.is_none() && middle.is_none() && b.is_none()), ..Stray::default() };
    };
    fn codes(v: &[(String, [f64; 3])]) -> Vec<&str> {
        let mut c: Vec<&str> = v.iter().map(|p| p.0.as_str()).collect();
        c.sort_unstable();
        c
    }
    let mut out = Stray { changed: codes(a) != codes(m) || codes(m) != codes(b), ..Stray::default() };
    for (code, p) in m {
        let (Some(pa), Some(pb)) = (a.iter().find(|x| &x.0 == code), b.iter().find(|x| &x.0 == code)) else { continue };
        let (pa, pb) = (pa.1, pb.1);
        let (dx, dy) = (pb[0] - pa[0], pb[1] - pa[1]);
        let len2 = dx * dx + dy * dy;
        let t = if len2 > 0.0 { (((p[0] - pa[0]) * dx + (p[1] - pa[1]) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
        let (cx, cy, cz) = (pa[0] + t * dx, pa[1] + t * dy, pa[2] + t * (pb[2] - pa[2]));
        let plan = ((p[0] - cx).powi(2) + (p[1] - cy).powi(2)).sqrt();
        let height = (p[2] - cz).abs();
        if plan > out.plan {
            out.plan = plan;
            out.plan_code = Some(code.clone());
        }
        if height > out.height {
            out.height = height;
            out.height_code = Some(code.clone());
        }
    }
    out
}

/// The tolerances a corridor's sampling is checked against: the plan distance and the height a section may stray
/// from the join of its neighbours (m; none: not checked), and the least step an interval is halved to (m).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tolerance {
    pub plan: Option<f64>,
    pub height: Option<f64>,
    pub least: f64,
}

/// An interval as it was measured: its ends, how far its middle strayed, and whether that is within the tolerances.
#[derive(Clone, Debug, PartialEq)]
pub struct Measured {
    pub from: f64,
    pub to: f64,
    pub stray: Stray,
    pub within: bool,
}

/// Why an interval was not refined although it strays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Left {
    /// Halving it would go under the least step.
    Least,
    /// The stations allowed are used up.
    Budget,
}

/// What [`refine`] did: the stations it added (in order) with what was built there; each first interval's middle as it
/// was measured before refining (`before`); the intervals at the end found within the tolerances (`after`, in order);
/// those left straying and why: halved down to the least step (its middle added), or not measured for the budget.
pub struct Refined<T> {
    pub added: Vec<(f64, T)>,
    pub before: Vec<Measured>,
    pub after: Vec<Measured>,
    pub left: Vec<(Measured, Left)>,
}

/// Whether a stray is within the tolerances: a changed section never is.
pub fn within(s: &Stray, tol: &Tolerance) -> bool {
    !s.changed && tol.plan.is_none_or(|t| s.plan <= t) && tol.height.is_none_or(|t| s.height <= t)
}

/// Refines `stations` (in order, each with what was built there): each interval's middle is built (`make`) and its
/// coded points (`coded`) measured against the ends'; one straying beyond `tol` is halved and both halves measured in
/// turn, until each is within or halving would make a step under `tol.least`, or `budget` middles have been built.
/// Intervals shorter than twice the least step are not measured.
pub fn refine<T>(
    stations: &[(f64, T)],
    tol: &Tolerance,
    budget: usize,
    mut make: impl FnMut(f64) -> T,
    coded: impl Fn(&T) -> Coded,
) -> Refined<T> {
    let mut out = Refined { added: Vec::new(), before: Vec::new(), after: Vec::new(), left: Vec::new() };
    let mut built = 0usize;
    for w in stations.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        // Depth first, left half before right, so the leaves come out in order.
        let mut stack: Vec<(f64, Coded, f64, Coded, bool)> = vec![(a.0, coded(&a.1), b.0, coded(&b.1), true)];
        while let Some((s0, c0, s1, c1, first)) = stack.pop() {
            let half = (s1 - s0) / 2.0;
            if half < tol.least {
                continue;
            }
            if built >= budget {
                out.left.push((Measured { from: s0, to: s1, stray: Stray::default(), within: false }, Left::Budget));
                continue;
            }
            let sm = s0 + half;
            let m = make(sm);
            built += 1;
            let cm = coded(&m);
            let stray = stray(&c0, &cm, &c1);
            let ok = within(&stray, tol);
            let measured = Measured { from: s0, to: s1, stray, within: ok };
            if first {
                out.before.push(measured.clone());
            }
            if ok {
                out.after.push(measured);
            } else if half / 2.0 < tol.least {
                // Its halves are too short to measure again: its middle is added and it is left at the least step
                // (a change of section found within two least steps, or a stray the least step does not resolve).
                out.added.push((sm, m));
                out.left.push((measured, Left::Least));
            } else {
                out.added.push((sm, m));
                stack.push((sm, cm.clone(), s1, c1, false));
                stack.push((s0, c0, sm, cm, false));
            }
        }
    }
    out.added.sort_by(|x, y| x.0.total_cmp(&y.0));
    out.after.sort_by(|x, y| x.from.total_cmp(&y.from));
    out.left.sort_by(|x, y| x.0.from.total_cmp(&y.0.from));
    out
}

/// The stations a profile's vertical curves are divided at so their straight joins stay within `tolerance` (m) of
/// them (§1): each parabola or circle piece over `region` divided equally into the fewest steps of at most
/// [`vertical_step`] at its largest second derivative (a parabola's constant one; a circle's at its end farther from
/// its centre), the divisions inside the piece and the region; grades are not divided.
pub fn vertical_divisions(pieces: &[crate::profile::vertical::Piece], tolerance: f64, region: [f64; 2]) -> Vec<f64> {
    use crate::profile::vertical::Piece;
    let mut out = Vec::new();
    for p in pieces {
        let (s0, s1) = p.range();
        let bend = match p {
            Piece::Grade { .. } => continue,
            Piece::Parabola { r, .. } => r.abs(),
            Piece::Circle { .. } => p.at(s0).2.abs().max(p.at(s1).2.abs()),
        };
        let step = vertical_step(tolerance, bend);
        if !step.is_finite() || s1 <= s0 {
            continue;
        }
        let n = ((s1 - s0) / step - 1e-9).ceil().max(1.0) as usize;
        for i in 1..n {
            let s = s0 + (s1 - s0) * i as f64 / n as f64;
            if s > region[0] && s < region[1] {
                out.push(s);
            }
        }
    }
    out
}
