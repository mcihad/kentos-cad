//! The fixes (docs/adr/0202 §4): what each writes, object by object. A fix
//! that would leave nothing of an object it keeps is refused with the reason.

use super::areas::part_area;
use super::{Finding, Objects, Rule, Spot, is_areas, taken, written};
use crate::entity::{PointPart, Shape, area_parts, join_parts};
use crate::geom::arrangement::Area;
use crate::geom::bulge::bulge_of_sweep;
use crate::geom::intersect::{Edge, closest_on_edge};
use crate::ops::geoprocess::clip::within;
use crate::ops::geoprocess::validity::repair;
use crate::ops::geoprocess::{Class, class_of, shape_of, subtract_all, union_all};
use crate::vec2::Vec2;

/// One object a fix changes: its new shape, or none when it is deleted.
#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    pub object: usize,
    pub shape: Option<Shape>,
}

fn areas(s: &Shape) -> Vec<Area> {
    match class_of(s) {
        Class::Areas(a) => a,
        _ => Vec::new(),
    }
}

fn object(objects: &Objects<'_>, i: Option<usize>) -> Result<usize, String> {
    i.filter(|&i| i < objects.shapes.len())
        .ok_or_else(|| "Bulgunun nesnesi çizimde yok; yeniden denetleyin.".to_owned())
}

/// An object without part `k` (none when it was its only part).
fn without_part(s: &Shape, k: usize) -> Option<Shape> {
    let mut parts: Vec<Shape> = area_parts(s).into_owned();
    if k < parts.len() {
        parts.remove(k);
    }
    join_parts(&parts)
}

/// The object with ring or path `spot.ring` of part `spot.part` written anew.
fn with_ring(s: &Shape, spot: Spot, pts: Vec<Vec2>, bulges: Vec<f64>) -> Option<Shape> {
    let bulges = bulges.iter().any(|b| *b != 0.0).then_some(bulges);
    if let Shape::Line { .. } = s {
        return match pts.as_slice() {
            [a, b] => Some(Shape::Line { a: *a, b: *b }),
            _ => None,
        };
    }
    let mut parts: Vec<Shape> = area_parts(s).into_owned();
    let part = parts.get_mut(spot.part)?;
    match part {
        Shape::Polyline {
            pts: p, bulges: b, ..
        } => {
            *p = pts;
            *b = bulges;
        }
        Shape::Polygon {
            pts: p,
            bulges: b,
            holes,
            ..
        } => {
            if spot.ring == 0 {
                *p = pts;
                *b = bulges;
            } else {
                let h = holes.as_mut()?.get_mut(spot.ring - 1)?;
                h.pts = pts;
                h.bulges = bulges;
            }
        }
        _ => return None,
    }
    join_parts(&parts)
}

/// Vertex `spot.index` of the ring or path removed; its two edges become one straight edge.
fn remove_vertex(s: &Shape, spot: Spot) -> Result<Shape, String> {
    let w = written(s)
        .into_iter()
        .find(|w| w.part == spot.part && w.ring == spot.ring)
        .ok_or("Bulgunun köşesi nesnede yok; yeniden denetleyin.")?;
    let n = w.pts.len();
    let k = spot.index;
    if k >= n || n < if w.closed { 4 } else { 3 } {
        return Err("Köşe silinirse nesne kalmaz.".to_owned());
    }
    let mut pts = w.pts.clone();
    let mut bulges = w.bulges.clone();
    pts.remove(k);
    if w.closed {
        bulges[(k + n - 1) % n] = 0.0;
        bulges.remove(k);
    } else if k == 0 {
        bulges.remove(0);
    } else if k == n - 1 {
        bulges.remove(n - 1);
        bulges[n - 2] = 0.0;
    } else {
        bulges[k - 1] = 0.0;
        bulges.remove(k);
    }
    with_ring(s, spot, pts, bulges).ok_or_else(|| "Köşe silinemedi.".to_owned())
}

/// `v` added on edge `spot.index`: an arc edge is cut in two on its circle.
fn add_vertex(s: &Shape, spot: Spot, v: Vec2) -> Result<Shape, String> {
    let w = written(s)
        .into_iter()
        .find(|w| w.part == spot.part && w.ring == spot.ring)
        .ok_or("Komşunun kenarı nesnede yok; yeniden denetleyin.")?;
    let k = spot.index;
    if k >= w.edge_count() {
        return Err("Komşunun kenarı nesnede yok; yeniden denetleyin.".to_owned());
    }
    let halves = match w.edge(k) {
        e @ Edge::Arc { sweep, .. } => {
            let t = closest_on_edge(&e, v).t;
            [bulge_of_sweep(sweep * t), bulge_of_sweep(sweep * (1.0 - t))]
        }
        Edge::Seg { .. } => [0.0, 0.0],
    };
    let mut pts = w.pts.clone();
    let mut bulges = w.bulges.clone();
    pts.insert(k + 1, v);
    bulges.splice(k..=k, halves);
    with_ring(s, spot, pts, bulges).ok_or_else(|| "Köşe eklenemedi.".to_owned())
}

/// A line's or a polyline's start (`index` 0) or end (1) moved to `q`.
fn move_end(s: &Shape, spot: Spot, q: Vec2) -> Result<Shape, String> {
    let w = written(s)
        .into_iter()
        .find(|w| w.part == spot.part && w.ring == 0)
        .ok_or("Çizginin ucu nesnede yok; yeniden denetleyin.")?;
    let mut pts = w.pts.clone();
    let n = pts.len();
    let k = if spot.index == 0 { 0 } else { n - 1 };
    pts[k] = q;
    let other = if k == 0 { pts[n - 1] } else { pts[0] };
    if n == 2 && super::dist(q, other) <= crate::geom::arrangement::TOL {
        return Err("Uç taşınırsa çizgi sıfır uzunlukta kalır.".to_owned());
    }
    with_ring(s, spot, pts, w.bulges).ok_or_else(|| "Uç taşınamadı.".to_owned())
}

/// Point `spot.part` of a (multi-)point object moved to `q`, its elevation kept.
fn move_point(s: &Shape, spot: Spot, q: Vec2) -> Result<Shape, String> {
    let Shape::Point { p, z, parts } = s else {
        return Err("Nokta değil.".to_owned());
    };
    let mut parts: Option<Vec<PointPart>> = parts.clone();
    let mut p = *p;
    if spot.part == 0 {
        p = q;
    } else {
        let part = parts
            .as_mut()
            .and_then(|v| v.get_mut(spot.part - 1))
            .ok_or("Noktanın parçası yok; yeniden denetleyin.")?;
        part.p = q;
    }
    Ok(Shape::Point { p, z: *z, parts })
}

/// What a fix writes (docs/adr/0202 §4).
pub fn fix(
    objects: &Objects<'_>,
    rules: &[Rule],
    finding: &Finding,
    key: &str,
) -> Result<Vec<Change>, String> {
    if !finding.fixes.contains(&key) {
        return Err("Bu bulgu bu düzeltmeyi sunmuyor.".to_owned());
    }
    let first = object(objects, finding.objects.first().copied())?;
    let shapes = objects.shapes;
    let update = |object: usize, shape: Shape| {
        Ok(vec![Change {
            object,
            shape: Some(shape),
        }])
    };
    match key {
        "subtractFirst" | "subtractSecond" => {
            let second = object(objects, finding.objects.get(1).copied())?;
            let (from, cut) = if key == "subtractFirst" {
                (first, second)
            } else {
                (second, first)
            };
            let left = subtract_all(&areas(&shapes[from]), &areas(&shapes[cut]));
            let shape =
                shape_of(&Class::Areas(left)).ok_or("Çıkarınca nesneden bir şey kalmıyor.")?;
            update(from, shape)
        }
        "mergeNeighbour" => {
            let into = object(objects, finding.subject)?;
            let mut joined = areas(&shapes[into]);
            if finding.problem == "gap" {
                let gap = finding
                    .regions
                    .first()
                    .ok_or("Boşluğun bölgesi yok; yeniden denetleyin.")?;
                joined.push(gap.clone());
                let shape = shape_of(&Class::Areas(union_all(&joined))).ok_or("Birleşim boş.")?;
                return update(into, shape);
            }
            let spot = finding
                .spot
                .ok_or("İnce alanın parçası yok; yeniden denetleyin.")?;
            let part = part_area(&shapes[first], spot.part)
                .ok_or("İnce alanın parçası yok; yeniden denetleyin.")?;
            joined.push(part);
            let shape = shape_of(&Class::Areas(union_all(&joined))).ok_or("Birleşim boş.")?;
            let mut out = vec![
                Change {
                    object: into,
                    shape: Some(shape),
                },
                Change {
                    object: first,
                    shape: without_part(&shapes[first], spot.part),
                },
            ];
            out.sort_by_key(|c| c.object);
            Ok(out)
        }
        "deletePart" => {
            let spot = finding
                .spot
                .ok_or("İnce alanın parçası yok; yeniden denetleyin.")?;
            Ok(vec![Change {
                object: first,
                shape: without_part(&shapes[first], spot.part),
            }])
        }
        "deleteDuplicate" => Ok(vec![Change {
            object: object(objects, finding.subject)?,
            shape: None,
        }]),
        "deleteObject" => Ok(vec![Change {
            object: first,
            shape: None,
        }]),
        "snapEnd" => {
            let spot = finding
                .spot
                .ok_or("Çizginin ucu yok; yeniden denetleyin.")?;
            let q = finding.target.ok_or("Taşınacak yer yok.")?;
            update(first, move_end(&shapes[first], spot, q)?)
        }
        "removeVertex" => {
            let spot = finding
                .spot
                .ok_or("Bulgunun köşesi yok; yeniden denetleyin.")?;
            update(first, remove_vertex(&shapes[first], spot)?)
        }
        "addVertex" => {
            let spot = finding
                .spot
                .ok_or("Komşunun kenarı yok; yeniden denetleyin.")?;
            let v = finding.target.ok_or("Eklenecek köşe yok.")?;
            update(first, add_vertex(&shapes[first], spot, v)?)
        }
        "repair" => {
            let shape = repair(&shapes[first])
                .shape
                .ok_or("Onarınca nesneden bir şey kalmıyor.")?;
            update(first, shape)
        }
        "clipOutside" => {
            let rule = rules
                .get(finding.rule)
                .ok_or("Bulgunun kuralı yok; yeniden denetleyin.")?;
            let other = rule.other.as_deref().unwrap_or("");
            let cover: Vec<Area> = (0..shapes.len())
                .filter(|&j| objects.layers[j] == other)
                .map(|j| taken(&shapes[j]))
                .filter(|t| is_areas(&t.class))
                .flat_map(|t| match t.class {
                    Class::Areas(a) => a,
                    _ => Vec::new(),
                })
                .collect();
            let kept = within(&class_of(&shapes[first]), &union_all(&cover), true);
            let shape = shape_of(&kept)
                .ok_or("Nesnenin hiçbir yeri öbür katmanın alanlarının içinde değil.")?;
            update(first, shape)
        }
        "snapToEnd" => {
            let spot = finding.spot.ok_or("Nokta yok; yeniden denetleyin.")?;
            let q = finding.target.ok_or("Taşınacak uç yok.")?;
            update(first, move_point(&shapes[first], spot, q)?)
        }
        other => Err(format!("Bilinmeyen düzeltme “{other}”.")),
    }
}
