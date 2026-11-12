//! Explicit guide, observed fixed arc-length spacing, unchanged adoption validation.
use super::*;
use kentos_geometry_core::{
    entity::Shape,
    ops::{
        assoc_array::{Layout, Placement, item_placements},
        path::{Path, nearest_s, path_of, point_at_s},
    },
    vec2::Vec2,
};

pub(super) fn guide(doc: &Document, input: &ArraysFind) -> Result<(Shape, Path), Stop> {
    let text = input.path.as_deref().ok_or_else(|| {
        refuse(
            codes::INVALID_PATH,
            "Yol boyunca arama için mevcut bir 2B kılavuz seçin.",
            "path",
        )
    })?;
    let uid = named(doc, text, "path")?;
    let Some(e) = doc.slot_of(uid).and_then(|s| doc.get(s)) else {
        return Err(refuse(
            codes::ENTITY_NOT_FOUND,
            "Kılavuz çizimde yok.",
            "path",
        ));
    };
    let supported = match e {
        Entity::Line(_) | Entity::Arc(_) | Entity::Circle(_) => true,
        Entity::Polyline(p) => {
            p.pts.len() <= 4096
                && p.zs.as_ref().is_none_or(|v| v.iter().all(Option::is_none))
                && p.ms.as_ref().is_none_or(|v| v.iter().all(Option::is_none))
                && p.space_separator.is_none()
                && p.parts.as_ref().is_none_or(Vec::is_empty)
                && p.holes.as_ref().is_none_or(Vec::is_empty)
        }
        _ => false,
    };
    if !supported {
        return Err(refuse(
            codes::INVALID_PATH,
            "Kılavuz tek parçalı 2B çizgi, yay, çember veya çoklu çizgi olmalı; desteklenmeyen geometri atlanmadı.",
            "path",
        ));
    }
    let shape = crate::geometry::shape(e);
    let path = supported
        .then(|| path_of(&shape))
        .flatten()
        .filter(|p| p.length.is_finite() && p.length > 1e-9 && p.edges.len() <= 4096);
    path.map(|p|(shape,p)).ok_or_else(||refuse(codes::INVALID_PATH,
        "Kılavuz tek parçalı 2B çizgi, yay, çember veya en çok 4096 köşeli çoklu çizgi olmalı; Z/M ve mahal rolü atlanmaz.", "path"))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn find(
    doc: &Document,
    input: &ArraysFind,
    points: &[Point],
    seed: Point,
    tolerance: f64,
    limit: usize,
    budget: &mut Budget,
    stopped: &mut impl FnMut() -> bool,
) -> Result<Vec<ArrayPattern>, Stop> {
    let (shape, path) = guide(doc, input)?;
    let Some(path_uid) = input.path.as_ref() else {
        return Err(refuse(codes::INVALID_PATH, "Kılavuz seçin.", "path"));
    };
    let start = point_at_s(&path, 0.);
    if (seed.x - start.x).hypot(seed.y - start.y) > tolerance {
        return Err(refuse(
            codes::RECOGNITION_UNSUPPORTED,
            "Seçilen kaynak kılavuzun kayıtlı başlangıcında değil. Başlangıçtaki bloğu kaynak seçin; kılavuz veya blok taşınmadı.",
            "source",
        ));
    }
    let prototype = arrays_adopt::prototype(doc, Uuid::from_bytes(seed.uid.0), "source")?;
    let mut stations = Vec::new();
    for p in points {
        // nearest_s visits every analytic edge. Charge all of that work and check cancellation.
        for _ in &path.edges {
            if !budget.tick(stopped)? {
                return Ok(Vec::new());
            }
        }
        let s = if p.uid == seed.uid {
            0.
        } else {
            nearest_s(&path, Vec2::new(p.x, p.y))
        };
        let at = point_at_s(&path, s);
        if s.is_finite() && (p.x - at.x).hypot(p.y - at.y) <= tolerance {
            stations.push((s, *p));
        }
    }
    stations.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.uid.cmp(&b.1.uid)));
    let mut steps = Vec::new();
    for pair in stations.windows(2) {
        let step = pair[1].0 - pair[0].0;
        if step > 2. * tolerance && step > 1e-9 {
            steps.push(step);
        }
    }
    steps.sort_by(f64::total_cmp);
    steps.dedup_by(|a, b| a.to_bits() == b.to_bits());
    let mut seen = HashSet::new();
    let mut matches = Vec::new();
    'search: for step in steps {
        let mut slots = BTreeMap::new();
        for &(s, p) in &stations {
            if !budget.tick(stopped)? {
                break 'search;
            }
            let k = (s / step).round();
            if !(0. ..=9999.).contains(&k) || (s - k * step).abs() > tolerance {
                continue;
            }
            slots.entry(k as u32).or_insert(p);
        }
        if slots.len() < 3
            || slots.get(&0).is_none_or(|p| p.uid != seed.uid)
            || !slots.keys().any(|k| slots.contains_key(&(k + 1)))
        {
            continue;
        }
        let Some((&last, last_point)) = slots.last_key_value() else {
            continue;
        };
        let Some(last_station) = stations
            .iter()
            .find(|(_, p)| p.uid == last_point.uid)
            .map(|(s, _)| *s)
        else {
            continue;
        };
        let full = last_station / last as f64;
        for spacing in [full, step] {
            for align in [false, true] {
                if !budget.tick(stopped)? {
                    break 'search;
                }
                let count = last + 1;
                // The placement engine and final adoption gate each visit every slot too.
                for _ in 0..count.saturating_mul(3) {
                    if !budget.tick(stopped)? {
                        break 'search;
                    }
                }
                let Some(places) = item_placements(
                    Placement {
                        p: Vec2::new(seed.x, seed.y),
                        rotation: prototype.rotation,
                        scale: prototype.scale,
                        mirror: prototype.mirror,
                    },
                    &Layout::Path {
                        path: &shape,
                        count,
                        spacing: Some(spacing),
                        align,
                    },
                ) else {
                    continue;
                };
                let mut items = vec![ArrayItem::default(); count as usize];
                let mut ambiguous = false;
                for &(s, p) in &stations {
                    if !budget.tick(stopped)? {
                        break 'search;
                    }
                    let k = (s / spacing).round() as usize;
                    let Some(at) = places.get(k) else {
                        continue;
                    };
                    // `points` already passed the common static-template gate in this immutable document.
                    let Some(Entity::Insert(actual)) = doc
                        .slot_of(Uuid::from_bytes(p.uid.0))
                        .and_then(|s| doc.get(s))
                    else {
                        unreachable!()
                    };
                    let angle = (actual.rotation - at.rotation).rem_euclid(std::f64::consts::TAU);
                    if (actual.p.x - at.p.x).hypot(actual.p.y - at.p.y) > tolerance
                        || angle.min(std::f64::consts::TAU - angle) > 1e-10
                    {
                        continue;
                    }
                    if items[k].uid.replace(p.uid).is_some() {
                        ambiguous = true;
                    }
                }
                let present = items.iter().filter(|i| i.uid.is_some()).count();
                if ambiguous
                    || present < 3
                    || items[0].uid != Some(seed.uid)
                    || !items.windows(2).any(|p| p.iter().all(|i| i.uid.is_some()))
                {
                    continue;
                }
                // A wrong-angle root may have suggested a longer span. It is not evidence
                // of trailing empty slots once the actual poses have been checked.
                let Some(last) = items.iter().rposition(|i| i.uid.is_some()) else {
                    continue;
                };
                let count = last + 1;
                items.truncate(count);
                let layout = ArrayLayout::Path {
                    path: path_uid.clone(),
                    count: count as u32,
                    spacing: Some(spacing),
                    align,
                };
                let Some((adoption, plan)) =
                    review_layouts(doc, seed.uid, &items, [layout], tolerance, budget, stopped)?
                else {
                    continue;
                };
                let key: Vec<_> = items.iter().map(|i| i.uid).collect();
                if !seen.insert((align, key)) {
                    continue;
                }
                matches.push(ArrayPattern {
                    adoption,
                    present: present as u32,
                    missing: plan.missing,
                    deviation: plan.deviation,
                    overlaps: false,
                });
                if matches.len() > limit {
                    budget.cut = true;
                    break 'search;
                }
            }
        }
    }
    Ok(matches)
}
