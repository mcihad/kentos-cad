//! What every import says besides its objects (docs/adr/0138): each layer's
//! objects by kind and the box they span, so a window counts and places what
//! a choice of layers brings without walking hundreds of thousands of
//! objects, and where the view shows the objects once they are in, with a
//! note on the far strays it leaves out. The desktop and the web read these
//! from the result, so the two place an import alike (the web's `viewOf` is
//! [`view_of`] on four numbers). And how many objects came with elevations
//! (docs/adr/0142), a fact of the file every reader says the same way.

use std::collections::{BTreeMap, HashMap};

use kentos_contracts::{Bounds, Entity, ImportResult, ReportItem, SourceFact, Vec2};

use crate::math::hypot;

/// The box an object's defining points span (a text's is its insertion
/// point, a circle's its square): where it is, cheaply, before it is drawn.
pub fn defining_bounds(e: &Entity) -> Option<Bounds> {
    let mut b: Option<Bounds> = None;
    let mut add = |p: &Vec2| {
        if !(p.x.is_finite() && p.y.is_finite()) {
            return;
        }
        let o = b.get_or_insert(Bounds {
            min_x: p.x,
            min_y: p.y,
            max_x: p.x,
            max_y: p.y,
        });
        o.min_x = o.min_x.min(p.x);
        o.min_y = o.min_y.min(p.y);
        o.max_x = o.max_x.max(p.x);
        o.max_y = o.max_y.max(p.y);
    };
    let square = |c: &Vec2, r: f64| {
        [
            Vec2 {
                x: c.x - r,
                y: c.y - r,
            },
            Vec2 {
                x: c.x + r,
                y: c.y + r,
            },
        ]
    };
    match e {
        Entity::Point(e) => add(&e.p),
        Entity::Line(e) => {
            add(&e.a);
            add(&e.b);
        }
        Entity::Polyline(e) | Entity::Polygon(e) => {
            e.pts.iter().for_each(&mut add);
            // Every part of a multi-part area (docs/adr/0143).
            e.parts
                .iter()
                .flatten()
                .for_each(|p| p.pts.iter().for_each(&mut add));
        }
        Entity::Circle(e) => square(&e.c, e.r).iter().for_each(&mut add),
        Entity::Arc(e) => square(&e.c, e.r).iter().for_each(&mut add),
        Entity::Ellipse(e) => square(&e.c, hypot(e.major.x, e.major.y))
            .iter()
            .for_each(&mut add),
        Entity::Spline(e) => e.pts.iter().for_each(&mut add),
        Entity::Xline(e) | Entity::Ray(e) => add(&e.p),
        Entity::Text(e) => add(&e.p),
        Entity::Dimension(e) => {
            add(&e.a);
            add(&e.b);
        }
        Entity::Hatch(e) => e.ring.iter().for_each(&mut add),
        // Its insertion point: the extent of its definition is the store's (docs/adr/0144).
        Entity::Insert(e) => add(&e.p),
        Entity::Leader(e) => e.pts.iter().for_each(&mut add),
        Entity::Table(e) => crate::blocks::table_corners(e).iter().for_each(&mut add),
        Entity::Image(e) => crate::blocks::image_corners(e).iter().for_each(&mut add),
        Entity::Raster(e) => crate::blocks::raster_corners(e).iter().for_each(&mut add),
    }
    b
}

/// Whether an object has an elevation: a point's, an end of a line, a vertex
/// of a path, or of a hole or a part of an area (docs/adr/0142, 0143).
pub fn has_elevation(e: &Entity) -> bool {
    let some = |zs: &Option<Vec<Option<f64>>>| zs.iter().flatten().any(Option::is_some);
    match e {
        Entity::Point(p) => p.z.is_some(),
        Entity::Line(l) => l.za.is_some() || l.zb.is_some(),
        Entity::Polyline(p) | Entity::Polygon(p) => {
            some(&p.zs)
                || p.holes.iter().flatten().any(|h| some(&h.zs))
                || p.parts
                    .iter()
                    .flatten()
                    .any(|part| some(&part.zs) || part.holes.iter().flatten().any(|h| some(&h.zs)))
        }
        _ => false,
    }
}

fn union<'a>(it: impl Iterator<Item = &'a Bounds>) -> Option<Bounds> {
    it.fold(None::<Bounds>, |acc, b| {
        Some(match acc {
            None => *b,
            Some(o) => Bounds {
                min_x: o.min_x.min(b.min_x),
                min_y: o.min_y.min(b.min_y),
                max_x: o.max_x.max(b.max_x),
                max_y: o.max_y.max(b.max_y),
            },
        })
    })
}

fn centre(b: &Bounds) -> (f64, f64) {
    ((b.min_x + b.max_x) / 2.0, (b.min_y + b.max_y) / 2.0)
}

/// The boxes that lie far from the rest of a file, by index. A real plan
/// held two copies of a small outline drawn at (0, 0), a slip in its
/// source: fitted to everything, the city was a dot in a 4 400 km view. The
/// bulk is the box of the centres' 2nd to 98th percentiles; a box whose
/// centre lies beyond it grown by twenty times its size (and at least
/// 100 km) is a stray. None among fewer than 100 boxes, and none when one in
/// a hundred or more is out there: the file is spread out, not slipped.
fn strays(boxes: &[Bounds]) -> Vec<usize> {
    if boxes.len() < 100 {
        return Vec::new();
    }
    let pick = |mut v: Vec<f64>, q: f64| {
        let at = (q * (v.len() - 1) as f64) as usize;
        v.select_nth_unstable_by(at, f64::total_cmp);
        v[at]
    };
    let xs: Vec<f64> = boxes.iter().map(|b| centre(b).0).collect();
    let ys: Vec<f64> = boxes.iter().map(|b| centre(b).1).collect();
    let (x0, x1) = (pick(xs.clone(), 0.02), pick(xs, 0.98));
    let (y0, y1) = (pick(ys.clone(), 0.02), pick(ys, 0.98));
    let grow = (20.0 * (x1 - x0).max(y1 - y0)).max(100_000.0);
    let far: Vec<usize> = boxes
        .iter()
        .enumerate()
        .filter(|(_, b)| {
            let (x, y) = centre(b);
            !(x >= x0 - grow && x <= x1 + grow && y >= y0 - grow && y <= y1 + grow)
        })
        .map(|(i, _)| i)
        .collect();
    if far.len() * 100 > boxes.len() {
        return Vec::new();
    }
    far
}

/// The boxes' extent without those `far` names.
fn union_but(boxes: &[Bounds], far: &[usize]) -> Option<Bounds> {
    let mut far = far.iter().copied().peekable();
    union(boxes.iter().enumerate().filter_map(|(i, b)| {
        if far.peek() == Some(&i) {
            far.next();
            None
        } else {
            Some(b)
        }
    }))
}

/// Where the view shows what was imported: the extent of `boxes` without
/// the far strays a file can hold (`strays`).
pub fn view_bounds(boxes: &[Bounds]) -> Option<Bounds> {
    union_but(boxes, &strays(boxes))
}

/// Where the view shows the chosen layers (their boxes, `ImportLayer.bounds`):
/// their extent inside the import's view, so a stray on a chosen layer is
/// left out as it is from the whole; their whole extent when the chosen
/// layers hold only what lies beyond it.
pub fn view_of<'a>(
    view: Option<&Bounds>,
    chosen: impl Iterator<Item = &'a Bounds>,
) -> Option<Bounds> {
    let all = union(chosen)?;
    let Some(v) = view else { return Some(all) };
    let inside = Bounds {
        min_x: all.min_x.max(v.min_x),
        min_y: all.min_y.max(v.min_y),
        max_x: all.max_x.min(v.max_x),
        max_y: all.max_y.min(v.max_y),
    };
    Some(
        if inside.min_x <= inside.max_x && inside.min_y <= inside.max_y {
            inside
        } else {
            all
        },
    )
}

/// Fills in each layer's objects by kind and their box, and where the view
/// shows the objects (`ImportLayer.kinds`, `ImportLayer.bounds`,
/// `ImportResult.view`).
pub fn summarise(result: &mut ImportResult) {
    #[derive(Default)]
    struct Layer {
        kinds: HashMap<&'static str, u32>,
        bounds: Option<Bounds>,
    }
    let mut per: HashMap<&str, Layer> = HashMap::new();
    let mut boxes = Vec::with_capacity(result.entities.len());
    let mut on = Vec::with_capacity(result.entities.len());
    for e in &result.entities {
        let name = e.base().layer_id.as_str();
        let layer = per.entry(name).or_default();
        *layer.kinds.entry(e.kind()).or_default() += 1;
        if let Some(b) = defining_bounds(e) {
            layer.bounds = union([layer.bounds.as_ref(), Some(&b)].into_iter().flatten());
            boxes.push(b);
            on.push(name);
        }
    }
    let far = strays(&boxes);
    let note = stray_note(&boxes, &on, &far);
    let mut per: HashMap<String, Layer> = per.into_iter().map(|(l, v)| (l.to_owned(), v)).collect();
    for layer in &mut result.layers {
        if let Some(l) = per.remove(&layer.name) {
            layer.kinds = l
                .kinds
                .into_iter()
                .map(|(k, n)| (k.to_owned(), n))
                .collect();
            layer.bounds = l.bounds;
        }
    }
    result.view = union_but(&boxes, &far);
    result.report.notes.extend(note);
    let elevated = result.entities.iter().filter(|e| has_elevation(e)).count();
    if elevated > 0 {
        result.report.source.push(SourceFact {
            label: "Kotlu nesne".into(),
            value: elevated.to_string(),
        });
    }
}

/// The strays, said: how many, on which layers, and where the first is
/// (Y to the right, X up, as a Turkish plan writes them).
fn stray_note(boxes: &[Bounds], on: &[&str], far: &[usize]) -> Option<ReportItem> {
    let first = boxes.get(*far.first()?)?;
    let mut per: BTreeMap<&str, u32> = BTreeMap::new();
    for &i in far {
        *per.entry(on[i]).or_default() += 1;
    }
    let mut layers: Vec<String> = per
        .iter()
        .take(3)
        .map(|(n, k)| format!("{n} {k}"))
        .collect();
    if per.len() > 3 {
        layers.push("…".into());
    }
    let (x, y) = centre(first);
    Some(ReportItem {
        what: "Uzaktaki nesne".into(),
        count: u32::try_from(far.len()).unwrap_or(u32::MAX),
        // `+ 0.0` writes a negative zero as 0.
        reason: format!(
            "çizimin geri kalanından çok uzakta ({}; ilki Y {:.0}, X {:.0} yakınında); görünüm onları dışarıda bırakır, kaynakta yanlış yere düşmüşlerse silin",
            layers.join(", "),
            x + 0.0,
            y + 0.0
        ),
        lines: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: f64, y: f64) -> Bounds {
        Bounds {
            min_x: x,
            min_y: y,
            max_x: x + 1.0,
            max_y: y + 1.0,
        }
    }

    #[test]
    fn a_stray_far_away_is_left_out_of_the_view() {
        let mut boxes: Vec<Bounds> = (0..500)
            .map(|i| at(585_000.0 + f64::from(i), 4_400_000.0))
            .collect();
        boxes.push(at(0.0, 0.0));
        let v = view_bounds(&boxes).expect("a view");
        assert_eq!((v.min_x, v.max_x), (585_000.0, 585_500.0));
        // Many far away are not strays: the file is spread out.
        for i in 0..20 {
            boxes.push(at(f64::from(i), 0.0));
        }
        assert_eq!(view_bounds(&boxes).expect("a view").min_x, 0.0);
    }

    #[test]
    fn a_stray_is_said_with_its_layer_and_where_it_is() {
        let mut boxes: Vec<Bounds> = (0..500)
            .map(|i| at(585_000.0 + f64::from(i), 4_400_000.0))
            .collect();
        let mut on = vec!["PARSEL"; 500];
        boxes.push(at(-0.5, -0.5));
        on.push("TRAFO");
        let far = strays(&boxes);
        assert_eq!(far, [500]);
        let note = stray_note(&boxes, &on, &far).expect("a note");
        assert_eq!((note.what.as_str(), note.count), ("Uzaktaki nesne", 1));
        assert!(
            note.reason.starts_with(
                "çizimin geri kalanından çok uzakta (TRAFO 1; ilki Y 0, X 0 yakınında)"
            ),
            "{}",
            note.reason
        );
        assert!(
            !note.reason.ends_with('.'),
            "the report line puts the full stop"
        );
        assert!(stray_note(&boxes[..500], &on[..500], &strays(&boxes[..500])).is_none());
    }

    #[test]
    fn chosen_layers_are_shown_inside_the_view_unless_they_lie_beyond_it() {
        let view = Bounds {
            min_x: 100.0,
            min_y: 100.0,
            max_x: 200.0,
            max_y: 200.0,
        };
        // A layer in the city with a stray at 0, 0: the city part.
        let spread = Bounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 150.0,
            max_y: 150.0,
        };
        let v = view_of(Some(&view), [&spread].into_iter()).expect("a view");
        assert_eq!(
            (v.min_x, v.min_y, v.max_x, v.max_y),
            (100.0, 100.0, 150.0, 150.0)
        );
        // A layer that is only the stray: shown where it is.
        let v = view_of(Some(&view), [&at(0.0, 0.0)].into_iter()).expect("a view");
        assert_eq!((v.min_x, v.max_x), (0.0, 1.0));
        assert!(view_of(Some(&view), std::iter::empty()).is_none());
    }
}
