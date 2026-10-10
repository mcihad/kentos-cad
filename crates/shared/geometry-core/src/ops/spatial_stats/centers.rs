//! Ortalama ve ortanca merkez, Standart uzaklık, Yön dağılımı (docs/adr/0238
//! §3–§5): one result a group, the groups by Özet istatistik's rule (their
//! texts trimmed, in their natural order, the empty group “(boş)” last).

use super::{NewObject, Placed, StatsRun, number_of, pair, text, unplaced_note, unread_note};
use crate::entity::Shape;
use crate::jsmath::{PI, TAU, atan2, cos, sin};
use crate::ops::point_editor::natural_order;
use crate::ops::statistics::{EMPTY_GROUP, figures};
use crate::tools::point_text::js_trim;
use crate::vec2::Vec2;

/// Which centre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CenterKind {
    Mean,
    Median,
    Distance,
    Ellipse,
}

impl CenterKind {
    pub fn from_key(key: &str) -> Option<CenterKind> {
        Some(match key {
            "mean" => CenterKind::Mean,
            "median" => CenterKind::Median,
            "distance" => CenterKind::Distance,
            "ellipse" => CenterKind::Ellipse,
            _ => return None,
        })
    }

    /// What the summary calls the result: “… ortalama merkezi yazıldı”.
    fn noun(self) -> &'static str {
        match self {
            CenterKind::Mean => "ortalama merkezi",
            CenterKind::Median => "ortanca merkezi",
            CenterKind::Distance => "standart uzaklığı",
            CenterKind::Ellipse => "yön dağılımı elipsi",
        }
    }
}

/// The inputs: each object's weight text (none: no weight field) and group
/// text (none: no group field), the weight field's name for the warnings,
/// and the multiple of the standard distance (§4).
pub struct CenterInput<'a> {
    pub weights: Option<&'a [Option<String>]>,
    pub groups: Option<&'a [Option<String>]>,
    pub weight_field: &'a str,
    pub k: f64,
}

/// A group's members: places (worked), weights and the weights' texts.
#[derive(Default)]
struct Group {
    pts: Vec<Vec2>,
    w: Vec<f64>,
    texts: Vec<String>,
}

/// Most Weiszfeld steps, the step that ends them (m), and how near a data
/// place counts as on it (m).
const MOST_STEPS: usize = 10_000;
const STEP_DONE: f64 = 1e-10;
const ON_PLACE: f64 = 1e-12;

/// The weighted mean of the places.
fn mean_of(g: &Group, total: f64) -> Vec2 {
    let (mut sx, mut sy) = (0.0, 0.0);
    for (p, w) in g.pts.iter().zip(&g.w) {
        sx += w * p.x;
        sy += w * p.y;
    }
    Vec2::new(sx / total, sy / total)
}

/// The geometric median (§3): Weiszfeld's steps from the mean, Vardi and
/// Zhang's when the estimate sits on data places; whether the steps ended.
fn median_of(g: &Group, start: Vec2) -> (Vec2, bool) {
    let mut p = start;
    for _ in 0..MOST_STEPS {
        let (mut nx, mut ny, mut den, mut on, mut rx, mut ry) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        for (q, &w) in g.pts.iter().zip(&g.w) {
            if w == 0.0 {
                continue;
            }
            let (dx, dy) = (q.x - p.x, q.y - p.y);
            let d = (dx * dx + dy * dy).sqrt();
            if d <= ON_PLACE {
                on += w;
                continue;
            }
            nx += w * q.x / d;
            ny += w * q.y / d;
            den += w / d;
            rx += w * dx / d;
            ry += w * dy / d;
        }
        if den == 0.0 {
            return (p, true);
        }
        let t = Vec2::new(nx / den, ny / den);
        let next = if on > 0.0 {
            let r = (rx * rx + ry * ry).sqrt();
            if r <= on {
                return (p, true);
            }
            let a = on / r;
            Vec2::new((1.0 - a) * t.x + a * p.x, (1.0 - a) * t.y + a * p.y)
        } else {
            t
        };
        let (sx, sy) = (next.x - p.x, next.y - p.y);
        p = next;
        if (sx * sx + sy * sy).sqrt() <= STEP_DONE {
            return (p, true);
        }
    }
    (p, false)
}

/// Every group's centre, standard distance or ellipse.
pub fn centers(
    shapes: &[Shape],
    kind: CenterKind,
    input: &CenterInput,
) -> Result<StatsRun, String> {
    let mut run = StatsRun::default();
    let (mut unread, mut negative) = (0usize, 0usize);
    let mut weights: Vec<Option<(f64, String)>> = vec![None; shapes.len()];
    let placed = Placed::of(shapes, |i| match input.weights {
        None => {
            weights[i] = Some((1.0, String::new()));
            true
        }
        Some(list) => {
            let t = list.get(i).and_then(|v| v.as_deref());
            match number_of(t) {
                None => {
                    unread += 1;
                    false
                }
                Some(w) if w < 0.0 => {
                    negative += 1;
                    false
                }
                Some(w) => {
                    weights[i] = Some((w, js_trim(t.unwrap_or("")).to_owned()));
                    true
                }
            }
        }
    });
    run.warn_if(placed.unplaced, unplaced_note);
    run.warn_if(unread, |n| unread_note(n, "ağırlığı", input.weight_field));
    run.warn_if(negative, |n| {
        format!("{n} nesnenin ağırlığı eksi; alınmadı.")
    });
    if placed.pts.is_empty() {
        return Err(if input.weights.is_some() {
            "Yeri ve ağırlığı okunan nesne yok.".into()
        } else {
            "Yeri bulunan nesne yok.".into()
        });
    }
    // The groups by their trimmed texts; the empty one apart.
    let mut names: Vec<String> = Vec::new();
    let mut groups: Vec<Group> = Vec::new();
    let mut empty = Group::default();
    for (k, &i) in placed.index.iter().enumerate() {
        let (w, t) = weights[i].clone().unwrap_or((1.0, String::new()));
        let key = input
            .groups
            .and_then(|g| g.get(i).and_then(|v| v.as_deref()))
            .map(js_trim)
            .filter(|t| !t.is_empty());
        let g = match key {
            None if input.groups.is_some() => &mut empty,
            None => {
                if groups.is_empty() {
                    names.push(String::new());
                    groups.push(Group::default());
                }
                &mut groups[0]
            }
            Some(name) => match names.iter().position(|n| n == name) {
                Some(at) => &mut groups[at],
                None => {
                    names.push(name.to_owned());
                    groups.push(Group::default());
                    let last = groups.len() - 1;
                    &mut groups[last]
                }
            },
        };
        g.pts.push(placed.pts[k]);
        g.w.push(w);
        g.texts.push(t);
    }
    let mut order: Vec<(Option<String>, Group)> = Vec::new();
    let mut taken: Vec<Option<Group>> = groups.into_iter().map(Some).collect();
    for at in natural_order(&names) {
        let at = at as usize;
        if let Some(g) = taken[at].take() {
            let name = input.groups.map(|_| names[at].clone());
            order.push((name, g));
        }
    }
    if !empty.pts.is_empty() {
        order.push((Some(EMPTY_GROUP.to_owned()), empty));
    }
    let (mut weightless, mut flat, mut unsettled, mut used) = (0usize, 0usize, 0usize, 0usize);
    for (name, g) in &order {
        let total: f64 = g.w.iter().sum();
        if total.is_nan() || total <= 0.0 {
            weightless += 1;
            continue;
        }
        let mean = mean_of(g, total);
        let mut attrs = Vec::new();
        if let Some(n) = name {
            attrs.push(pair("Grup", n.as_str()));
        }
        attrs.push(pair("Nesne sayısı", g.pts.len().to_string()));
        if input.weights.is_some() {
            let texts: Vec<Option<&str>> = g.texts.iter().map(|t| Some(t.as_str())).collect();
            if let Some(sum) = figures(&texts, Some(0)).ok().and_then(|f| f.sum) {
                attrs.push(pair("Ağırlık toplamı", sum.text()));
            }
        }
        let shape = match kind {
            CenterKind::Mean => point(placed.absolute(mean)),
            CenterKind::Median => {
                let (m, settled) = median_of(g, mean);
                if !settled {
                    unsettled += 1;
                }
                point(placed.absolute(m))
            }
            CenterKind::Distance => {
                let mut s = 0.0;
                for (p, w) in g.pts.iter().zip(&g.w) {
                    let (dx, dy) = (p.x - mean.x, p.y - mean.y);
                    s += w * (dx * dx + dy * dy);
                }
                let sd = (s / total).sqrt();
                if sd.is_nan() || sd <= 0.0 {
                    flat += 1;
                    continue;
                }
                attrs.push(pair("Standart uzaklık", text(sd, 3)));
                attrs.push(pair("Kat", text(input.k, 0)));
                Shape::Circle {
                    c: placed.absolute(mean),
                    r: input.k * sd,
                }
            }
            CenterKind::Ellipse => {
                let Some(e) = ellipse_of(g, mean, total, input.k) else {
                    flat += 1;
                    continue;
                };
                attrs.push(pair("Büyük yarı eksen", text(e.a, 3)));
                attrs.push(pair("Küçük yarı eksen", text(e.b, 3)));
                attrs.push(pair("Doğrultu", text(e.bearing, 2)));
                attrs.push(pair("Kat", text(input.k, 0)));
                Shape::Ellipse {
                    c: placed.absolute(mean),
                    major: Vec2::new(e.a * cos(e.phi), e.a * sin(e.phi)),
                    ratio: e.b / e.a,
                    t0: 0.0,
                    t1: TAU,
                }
            }
        };
        used += g.pts.len();
        run.objects.push(NewObject { shape, attrs });
    }
    run.warn_if(weightless, |n| {
        format!("{n} grubun ağırlıklarının toplamı sıfır; yazılmadı.")
    });
    run.warn_if(flat, |n| match kind {
        CenterKind::Ellipse => {
            format!("{n} grubun yerleri bir doğru üzerinde ya da çakışık; elips yazılmadı.")
        }
        _ => {
            format!("{n} grubun standart uzaklığı sıfır (tek yer ya da çakışık yerler); yazılmadı.")
        }
    });
    run.warn_if(unsettled, |n| {
        format!("{n} grubun ortanca merkezi {MOST_STEPS} adımda yakınsamadı; son adımı yazıldı.")
    });
    if run.objects.is_empty() {
        return Err(match kind {
            CenterKind::Ellipse => {
                "Yazılacak elips yok: yerler bir doğru üzerinde ya da çakışık.".into()
            }
            CenterKind::Distance => "Yazılacak daire yok: yerler çakışık ya da tek.".into(),
            _ => "Yazılacak merkez yok: ağırlıkların toplamı sıfır.".into(),
        });
    }
    run.summary = if input.groups.is_some() {
        format!(
            "{} grubun {} yazıldı ({used} nesne).",
            run.objects.len(),
            kind.noun()
        )
    } else {
        let tail = match (kind, run.objects[0].attrs.as_slice()) {
            (CenterKind::Distance, a) => format!(": {} m", a[a.len() - 2][1]),
            (CenterKind::Ellipse, a) => {
                let n = a.len();
                format!(
                    ": büyük yarı eksen {} m, küçük {} m, doğrultu {}°",
                    a[n - 4][1],
                    a[n - 3][1],
                    a[n - 2][1]
                )
            }
            _ => String::new(),
        };
        format!("{used} nesnenin {} yazıldı{tail}.", kind.noun())
    };
    Ok(run)
}

fn point(p: Vec2) -> Shape {
    Shape::Point {
        p,
        z: None,
        parts: None,
    }
}

/// A standard deviational ellipse: semi-axes, the major axis's angle from x
/// (anticlockwise) and its bearing (from north, clockwise, degrees in [0, 180)).
struct Deviational {
    a: f64,
    b: f64,
    phi: f64,
    bearing: f64,
}

fn ellipse_of(g: &Group, mean: Vec2, total: f64, k: f64) -> Option<Deviational> {
    let (mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0);
    for (p, w) in g.pts.iter().zip(&g.w) {
        let (x, y) = (p.x - mean.x, p.y - mean.y);
        sxx += w * x * x;
        syy += w * y * y;
        sxy += w * x * y;
    }
    let t = (sxx + syy) / 2.0;
    let half = (sxx - syy) / 2.0;
    let h = (half * half + sxy * sxy).sqrt();
    let l1 = (t + h) / total;
    if l1.is_nan() || l1 <= 0.0 {
        return None;
    }
    let l2 = (sxx * syy - sxy * sxy) / (total * total * l1);
    if l2.is_nan() || l2 <= 0.0 {
        return None;
    }
    let root2 = std::f64::consts::SQRT_2;
    let a = k * root2 * l1.sqrt();
    let b = k * root2 * l2.sqrt();
    // The major axis's angle in (−π/2, π/2]: atan2's −π (a minus zero sxy) is the same axis as π.
    let mut phi = atan2(2.0 * sxy, sxx - syy) / 2.0;
    if phi <= -PI / 2.0 {
        phi += PI;
    }
    // The major axis's bearing from the direction (cos φ, sin φ): from north, clockwise, in [0°, 180°).
    let mut bearing = atan2(cos(phi), sin(phi)) * (180.0 / PI);
    if bearing >= 180.0 {
        bearing -= 180.0;
    }
    Some(Deviational { a, b, phi, bearing })
}
