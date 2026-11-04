//! Water on a corridor's platform (docs/adr/0923; ROAD-DEVER-09): the top surface's lane and shoulder triangles flatter
//! than the user's least slope (where the grade and the crossfall leave water standing: the superelevation
//! transition's flat spot), the stations where a side's first lane's crossfall passes through zero, and the low
//! points along the platform's edges (where water gathers and an inlet is wanted). A triangle's slope is its plane's
//! steepest, |∇z|, from its corners; the crossfall is the axis point's to the side's `c1` point's elevation over their
//! plan distance (rising outward positive), linear between stations; a low point is a feature point lower than both
//! its neighbours. The least slope is the user's; KentOS gives none.

use crate::corridor::{Built, Feature, Role};

/// A stretch of flat platform: its region, its first and last station, the plan area flatter than the least, and the
/// least slope in it.
#[derive(Clone, Debug, PartialEq)]
pub struct FlatStretch {
    pub region: usize,
    pub from: f64,
    pub to: f64,
    pub area: f64,
    pub least: f64,
}

/// A station where a side's first lane's crossfall is zero.
#[derive(Clone, Debug, PartialEq)]
pub struct ZeroCrossfall {
    pub region: usize,
    pub right: bool,
    pub station: f64,
}

/// A low point on a platform edge: its code, region and place (station, x, y, z).
#[derive(Clone, Debug, PartialEq)]
pub struct LowPoint {
    pub code: String,
    pub region: usize,
    pub at: [f64; 4],
}

/// What water does on the platform.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlatformWater {
    pub flats: Vec<FlatStretch>,
    pub flat_triangles: usize,
    pub zeros: Vec<ZeroCrossfall>,
    pub lows: Vec<LowPoint>,
}

/// A triangle's steepest slope (rise over run).
pub fn slope(t: &[[f64; 3]; 3]) -> f64 {
    let (u, v) = ([t[1][0] - t[0][0], t[1][1] - t[0][1], t[1][2] - t[0][2]], [t[2][0] - t[0][0], t[2][1] - t[0][1], t[2][2] - t[0][2]]);
    let n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
    n[0].hypot(n[1]) / n[2].abs()
}

fn plan_area(t: &[[f64; 3]; 3]) -> f64 {
    0.5 * ((t[1][0] - t[0][0]) * (t[2][1] - t[0][1]) - (t[2][0] - t[0][0]) * (t[1][1] - t[0][1])).abs()
}

/// Whether `code` is a platform edge: the axis or a component's edge (`left.c1`, `right.c2` …).
pub fn platform_edge(code: &str) -> bool {
    code == "axis"
        || code
            .split_once('.')
            .is_some_and(|(side, rest)| (side == "left" || side == "right") && rest.strip_prefix('c').is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())))
}

fn crossfalls(features: &[Feature], region: usize, right: bool) -> Vec<(f64, f64)> {
    let edge = if right { "right.c1" } else { "left.c1" };
    let axis: Vec<&[f64; 4]> = features.iter().filter(|f| f.region == region && f.code == "axis").flat_map(|f| f.points.iter()).collect();
    let mut out: Vec<(f64, f64)> = features
        .iter()
        .filter(|f| f.region == region && f.code == edge)
        .flat_map(|f| f.points.iter())
        .filter_map(|p| {
            let a = axis.iter().find(|a| a[0] == p[0])?;
            let run = (p[1] - a[1]).hypot(p[2] - a[2]);
            (run > 0.0).then(|| (p[0], (p[3] - a[3]) / run))
        })
        .collect();
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

/// What water does on `built`'s platform, `least` the least slope (rise over run).
pub fn water(built: &Built, least: f64) -> PlatformWater {
    let mut out = PlatformWater::default();
    for (region, from, to, range) in &built.strips {
        let mut area = 0.0;
        let mut low = f64::INFINITY;
        for i in range.clone() {
            if !matches!(built.roles[i], Role::Lane | Role::Shoulder) {
                continue;
            }
            let s = slope(&built.triangles[i]);
            if s < least {
                area += plan_area(&built.triangles[i]);
                low = low.min(s);
                out.flat_triangles += 1;
            }
        }
        if low.is_finite() {
            match out.flats.last_mut() {
                Some(last) if last.region == *region && last.to == *from => {
                    last.to = *to;
                    last.area += area;
                    last.least = last.least.min(low);
                }
                _ => out.flats.push(FlatStretch { region: *region, from: *from, to: *to, area, least: low }),
            }
        }
    }
    let regions = built.features.iter().map(|f| f.region + 1).max().unwrap_or(0);
    for region in 0..regions {
        for right in [false, true] {
            let cf = crossfalls(&built.features, region, right);
            for (k, &(s, c)) in cf.iter().enumerate() {
                if c == 0.0 {
                    out.zeros.push(ZeroCrossfall { region, right, station: s });
                }
                if let Some(&(s1, c1)) = cf.get(k + 1)
                    && c * c1 < 0.0
                {
                    out.zeros.push(ZeroCrossfall { region, right, station: s + (s1 - s) * c / (c - c1) });
                }
            }
        }
    }
    for f in built.features.iter().filter(|f| platform_edge(&f.code)) {
        for w in f.points.windows(3) {
            if w[1][3] < w[0][3] && w[1][3] < w[2][3] {
                out.lows.push(LowPoint { code: f.code.clone(), region: f.region, at: w[1] });
            }
        }
    }
    out.zeros.sort_by(|a, b| (a.region, a.right).cmp(&(b.region, b.right)).then(a.station.total_cmp(&b.station)));
    out.lows.sort_by(|a, b| a.region.cmp(&b.region).then(a.at[0].total_cmp(&b.at[0])).then(a.code.cmp(&b.code)));
    out
}
