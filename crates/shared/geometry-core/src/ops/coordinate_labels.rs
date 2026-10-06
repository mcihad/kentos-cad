//! Koordinat yaz (docs/adr/0185): where coordinate labels go and what they
//! say. The places are the coordinate schedule's (docs/adr/0184 §3, §2
//! here): a point object's points named by its label, the vertices of
//! lines, polylines and areas, one per 1 µm, the unnamed numbered past the
//! names; each remembers the middle of the box of the vertices of the object
//! it first came from. A label's lines are its template's parts filled with
//! the place's values (§3); its leader and lines stand as §4 says. Both
//! platforms' tools call it (`coordinatePlaces`, `coordinateLabels`); the
//! independent reference is scripts/fixtures/coordinate_label_cases.py
//! (fixtures/coordinate-labels/v1/cases.json).

use std::collections::HashSet;

use crate::api::Op;
use crate::api::json::{FromJson, Json, read_field};
use crate::display::fixed;
use crate::entity::Shape;
use crate::jsmath::{js_max, js_min};
use crate::op;
use crate::ops::table::Listed;
use crate::ops::vertex_points::Grid;
use crate::text::{Font, TextAlign, width_em_in};
use crate::vec2::Vec2;

/// A place a label is written at: where it is, its elevation, its name and
/// the middle of the box of the vertices of the object it first came from
/// (none: a place given by itself, a clicked one).
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub p: Vec2,
    pub z: Option<f64>,
    pub name: Option<String>,
    pub centre: Option<Vec2>,
}

crate::json_struct!(Place { p, z, name, centre });

/// An object's places: where, the elevation and the name, in order: a
/// point object's points (its own, then its other parts', the k-th past the
/// first named “label (k+1)”), the vertices of its paths.
fn vertices(o: &Listed) -> Vec<(Vec2, Option<f64>, Option<String>)> {
    match &o.shape {
        Shape::Point { p, z, parts } => {
            let label = o.name();
            std::iter::once((*p, *z))
                .chain(parts.iter().flatten().map(|q| (q.p, q.z)))
                .enumerate()
                .map(|(k, (p, z))| {
                    let name = label.map(|l| {
                        if k == 0 {
                            l.to_owned()
                        } else {
                            format!("{l} ({})", k + 1)
                        }
                    });
                    (p, z, name)
                })
                .collect()
        }
        _ => o
            .paths
            .iter()
            .flat_map(|path| {
                path.pts
                    .iter()
                    .enumerate()
                    .map(|(k, &p)| (p, path.zs.get(k).copied().flatten(), None))
            })
            .collect(),
    }
}

/// The places of `objects`, in their order (§2): one per 1 µm, the first
/// keeping its place and box, taking a later one's elevation and name when
/// it has none; the unnamed named 1, 2, … past the names the places have.
pub fn places(objects: &[Listed]) -> Vec<Place> {
    let mut out: Vec<Place> = Vec::new();
    let mut grid = Grid::default();
    for o in objects {
        let found = vertices(o);
        let Some(first) = found.first().map(|v| v.0) else {
            continue;
        };
        let (lo, hi) = found.iter().fold((first, first), |(lo, hi), v| {
            (
                Vec2::new(js_min(lo.x, v.0.x), js_min(lo.y, v.0.y)),
                Vec2::new(js_max(hi.x, v.0.x), js_max(hi.y, v.0.y)),
            )
        });
        let centre = Vec2::new((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0);
        for (p, z, name) in found {
            if let Some(i) = grid.near(p, |i| out[i].p) {
                let place = &mut out[i];
                if place.z.is_none() {
                    place.z = z;
                }
                if place.name.is_none() {
                    place.name = name;
                }
                continue;
            }
            grid.put(p, out.len());
            out.push(Place {
                p,
                z,
                name,
                centre: Some(centre),
            });
        }
    }
    let taken: HashSet<String> = out.iter().filter_map(|p| p.name.clone()).collect();
    let mut next: u64 = 1;
    for place in &mut out {
        if place.name.is_some() {
            continue;
        }
        loop {
            let n = next.to_string();
            next += 1;
            if !taken.contains(&n) {
                place.name = Some(n);
                break;
            }
        }
    }
    out
}

/// The project's settings a label writes its numbers by: a CAD project's
/// axes (X east, Y north) or a CBS project's (Y east, X north), and the
/// drawing unit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabelUnits {
    pub cad: bool,
    /// The unit's count in a metre: 1 (m), 100 (cm), 1000 (mm).
    pub per_metre: f64,
}

impl FromJson for LabelUnits {
    fn from_json(v: &Json) -> Result<LabelUnits, String> {
        let axes: String = read_field(v, "axes")?;
        let unit: String = read_field(v, "unit")?;
        Ok(LabelUnits {
            cad: axes == "cad",
            per_metre: match unit.as_str() {
                "cm" => 100.0,
                "mm" => 1000.0,
                _ => 1.0,
            },
        })
    }
}

/// Where a label goes from its place (§4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// Away from the middle of its object's box; north-east without one.
    Auto,
    NorthEast,
    NorthWest,
    SouthWest,
    SouthEast,
}

impl FromJson for Direction {
    fn from_json(v: &Json) -> Result<Direction, String> {
        match String::from_json(v)?.as_str() {
            "auto" => Ok(Direction::Auto),
            "ne" => Ok(Direction::NorthEast),
            "nw" => Ok(Direction::NorthWest),
            "sw" => Ok(Direction::SouthWest),
            "se" => Ok(Direction::SouthEast),
            other => Err(format!("“{other}” diye bir yön yok")),
        }
    }
}

impl Direction {
    /// Its signs east and north (each 1 or −1) for `place`: from the middle
    /// of the place's box to it, east and north when they are equal.
    pub fn signs(self, place: &Place) -> (f64, f64) {
        match self {
            Direction::NorthEast => (1.0, 1.0),
            Direction::NorthWest => (-1.0, 1.0),
            Direction::SouthWest => (-1.0, -1.0),
            Direction::SouthEast => (1.0, -1.0),
            Direction::Auto => match place.centre {
                None => (1.0, 1.0),
                Some(c) => (
                    if place.p.x >= c.x { 1.0 } else { -1.0 },
                    if place.p.y >= c.y { 1.0 } else { -1.0 },
                ),
            },
        }
    }
}

/// How labels are written: the template, the decimals, the height (m),
/// whether a leader carries them, where they go, and the face their widths
/// are measured in (typeface, bold, width factor).
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    pub template: String,
    pub decimals: usize,
    pub height: f64,
    pub leader: bool,
    pub direction: Direction,
    pub font: Font,
    pub bold: bool,
    pub width_factor: f64,
}

/// The most decimals a coordinate is written with.
pub const MAX_DECIMALS: usize = 12;

impl FromJson for Options {
    fn from_json(v: &Json) -> Result<Options, String> {
        let decimals: f64 = read_field(v, "decimals")?;
        let font: String = read_field(v, "font")?;
        let bold: Option<bool> = read_field(v, "bold")?;
        let width_factor: Option<f64> = read_field(v, "widthFactor")?;
        Ok(Options {
            template: read_field(v, "template")?,
            decimals: if decimals.is_finite() && decimals > 0.0 {
                (decimals as usize).min(MAX_DECIMALS)
            } else {
                0
            },
            height: read_field(v, "height")?,
            leader: read_field(v, "leader")?,
            direction: read_field(v, "direction")?,
            font: Font::from_id(&font),
            bold: bold.unwrap_or(false),
            width_factor: width_factor.unwrap_or(1.0),
        })
    }
}

/// The value a stand-in names, if it names one: `Some(None)` names a value
/// the place has not.
fn stand_in(
    key: &str,
    place: &Place,
    units: &LabelUnits,
    decimals: usize,
) -> Option<Option<String>> {
    let value = |v: f64| fixed(v * units.per_metre, decimals);
    let (east, north) = if units.cad { ("x", "y") } else { ("y", "x") };
    let key = key.to_ascii_lowercase();
    if key == east {
        Some(Some(value(place.p.x)))
    } else if key == north {
        Some(Some(value(place.p.y)))
    } else if key == "z" {
        Some(place.z.map(value))
    } else if key == "ad" {
        Some(place.name.clone())
    } else {
        None
    }
}

/// A label's lines (§3): the template's parts between `|`, each `{Y}`,
/// `{X}`, `{Z}` and `{ad}` (in either case) the place's value, any other
/// `{…}` as written, spaces round the part cut; a part with a stand-in that
/// has no value, or with nothing left, is left out.
pub fn lines(template: &str, place: &Place, units: &LabelUnits, decimals: usize) -> Vec<String> {
    let mut out = Vec::new();
    for part in template.split('|') {
        let mut line = String::new();
        let mut missing = false;
        let mut rest = part;
        while let Some(open) = rest.find('{') {
            line.push_str(&rest[..open]);
            let after = &rest[open + 1..];
            match after.find(['{', '}']) {
                Some(close) if after[close..].starts_with('}') => {
                    let key = &after[..close];
                    match stand_in(key, place, units, decimals) {
                        Some(Some(v)) => line.push_str(&v),
                        Some(None) => missing = true,
                        None => {
                            line.push('{');
                            line.push_str(key);
                            line.push('}');
                        }
                    }
                    rest = &after[close + 1..];
                }
                _ => {
                    line.push('{');
                    rest = after;
                }
            }
        }
        line.push_str(rest);
        let line = line.trim_matches(' ');
        if missing || line.is_empty() {
            continue;
        }
        out.push(line.to_owned());
    }
    out
}

/// One line of a label: where its alignment puts it, its words, and its
/// alignment (none: the left of its baseline).
#[derive(Clone, Debug, PartialEq)]
pub struct LabelText {
    pub p: Vec2,
    pub text: String,
    pub align: Option<TextAlign>,
}

crate::json_struct!(out LabelText { p, text, align });

/// A label: its leader (the place, the elbow, the bar's end; none without
/// one) and its lines.
#[derive(Clone, Debug, PartialEq)]
pub struct Label {
    pub leader: Option<Vec<Vec2>>,
    pub texts: Vec<LabelText>,
}

crate::json_struct!(out Label { leader, texts });

/// The labels of some places, and how many got none (no line left).
#[derive(Clone, Debug, PartialEq)]
pub struct Labels {
    pub labels: Vec<Label>,
    pub skipped: usize,
}

crate::json_struct!(out Labels { labels, skipped });

/// The label of `place` (§4), none when its template leaves it no line.
pub fn label(place: &Place, o: &Options, units: &LabelUnits) -> Option<Label> {
    let words = lines(&o.template, place, units, o.decimals);
    if words.is_empty() {
        return None;
    }
    let h = o.height;
    let (sx, sy) = o.direction.signs(place);
    let pitch = 5.0 / 3.0 * h;
    let east = sx > 0.0;
    let (baseline, top) = if east {
        (None, Some(TextAlign::TopLeft))
    } else {
        (Some(TextAlign::BaselineRight), Some(TextAlign::TopRight))
    };
    let p = place.p;
    let mut texts = Vec::with_capacity(words.len());
    if o.leader {
        let e = Vec2::new(p.x + 3.0 * h * sx, p.y + 3.0 * h * sy);
        let widest = words
            .iter()
            .map(|t| width_em_in(t, o.font, o.bold) * h * o.width_factor)
            .fold(0.0, js_max);
        let w = widest + h;
        let x = e.x + h / 2.0 * sx;
        for (k, text) in words.into_iter().enumerate() {
            let (y, align) = if k == 0 {
                (e.y + 0.4 * h, baseline)
            } else {
                (e.y - 0.4 * h - (k - 1) as f64 * pitch, top)
            };
            texts.push(LabelText {
                p: Vec2::new(x, y),
                text,
                align,
            });
        }
        return Some(Label {
            leader: Some(vec![p, e, Vec2::new(e.x + w * sx, e.y)]),
            texts,
        });
    }
    let a = Vec2::new(p.x + h / 2.0 * sx, p.y + h / 2.0 * sy);
    let n = words.len();
    for (k, text) in words.into_iter().enumerate() {
        let (y, align) = if sy > 0.0 {
            (a.y + (n - 1 - k) as f64 * pitch, baseline)
        } else {
            (a.y - k as f64 * pitch, top)
        };
        texts.push(LabelText {
            p: Vec2::new(a.x, y),
            text,
            align,
        });
    }
    Some(Label {
        leader: None,
        texts,
    })
}

/// The labels of `places`, in order, and how many got none.
pub fn labels(places: &[Place], o: &Options, units: &LabelUnits) -> Labels {
    let mut out = Vec::with_capacity(places.len());
    let mut skipped = 0;
    for place in places {
        match label(place, o, units) {
            Some(l) => out.push(l),
            None => skipped += 1,
        }
    }
    Labels {
        labels: out,
        skipped,
    }
}

pub(crate) static OPS: &[Op] = &[
    op!("coordinatePlaces", |objects: Vec<Listed>| places(&objects)),
    op!(
        "coordinateLabels",
        |places: Vec<Place>, options: Options, units: LabelUnits| labels(&places, &options, &units)
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::json::to_string;

    fn file() -> Json {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../fixtures/coordinate-labels/v1/cases.json"
        );
        Json::parse(&std::fs::read_to_string(path).expect("the cases")).expect("JSON")
    }

    fn list<'a>(f: &'a Json, k: &str) -> &'a [Json] {
        match f.get(k) {
            Json::Arr(a) => a,
            _ => panic!("{k}"),
        }
    }

    /// `got` is `want`: the same fields in any order, the same words, numbers
    /// within 0.1 µm (the cases are exact; f64 places them a few ulps off).
    fn same(got: &Json, want: &Json, at: &str) {
        match (got, want) {
            (Json::Num(a), Json::Num(b)) => {
                assert!((a - b).abs() <= 1e-7, "{at}: {a} ≠ {b}");
            }
            (Json::Arr(a), Json::Arr(b)) => {
                assert_eq!(a.len(), b.len(), "{at}: {got:?} ≠ {want:?}");
                for (k, (x, y)) in a.iter().zip(b).enumerate() {
                    same(x, y, &format!("{at}[{k}]"));
                }
            }
            (Json::Obj(a), Json::Obj(b)) => {
                let keys = |o: &Vec<(String, Json)>| {
                    let mut k: Vec<String> = o.iter().map(|(k, _)| k.clone()).collect();
                    k.sort();
                    k
                };
                assert_eq!(keys(a), keys(b), "{at}");
                for (k, v) in a {
                    same(v, want.get(k), &format!("{at}.{k}"));
                }
            }
            _ => assert_eq!(got, want, "{at}"),
        }
    }

    /// The shared cases (fixtures/coordinate-labels/v1/cases.json), written
    /// by scripts/fixtures/coordinate_label_cases.py from docs/adr/0185, not
    /// from this code; the web runs them through WASM
    /// (model/ops/coordinateLabels.test.ts).
    #[test]
    fn places_and_labels_are_as_the_shared_cases_say() {
        let f = file();
        for c in list(&f, "places") {
            let name = c.get("name");
            let objects: Vec<Listed> = FromJson::from_json(c.get("objects")).expect("objects");
            let got = Json::parse(&to_string(&places(&objects))).expect("JSON");
            same(&got, c.get("want"), &format!("{name:?}"));
        }
        for c in list(&f, "labels") {
            let name = c.get("name");
            let ps: Vec<Place> = FromJson::from_json(c.get("places")).expect("places");
            let o = Options::from_json(c.get("options")).expect("options");
            let u = LabelUnits::from_json(c.get("units")).expect("units");
            let got = Json::parse(&to_string(&labels(&ps, &o, &u))).expect("JSON");
            same(&got, c.get("want"), &format!("{name:?}"));
        }
    }
}
