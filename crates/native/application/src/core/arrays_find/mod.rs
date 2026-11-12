//! Cancellable, bounded line/grid/full-circle pattern proposals. No objects, selection or array records are written.
use crate::{
    ExecutionContext, arrays_adopt,
    checks::{self, Stop, error},
};
use kentos_commands::{
    ARRAY_FIND_SCOPE_MAX, ArrayFindKind, ArrayItem, ArrayLayout, ArrayPattern, ArraysAdopt,
    ArraysAdoptPlan, ArraysFind, ArraysFound, CommandResult, CommandWarning, Entity, EntityId,
    InsertEntity, codes,
};
use kentos_domain::{Document, Uuid};
use std::collections::{BTreeMap, BTreeSet, HashSet};

mod grid;
mod lattice;
mod polar;
mod path;

const WORK_MAX: u64 = 2_000_000;

fn refuse(code: &str, text: &str, path: &str) -> Stop {
    Stop::failed(error(code, text.into(), Some(path.into())))
}
fn named(doc: &Document, text: &str, path: &str) -> Result<Uuid, Stop> {
    if !checks::is_uid_text(text) {
        return Err(refuse(
            codes::INVALID_UID,
            "Nesne kimliği küçük harfli, tireli UUID olmalı.",
            path,
        ));
    }
    let uid =
        Uuid::parse_str(text).map_err(|_| refuse(codes::INVALID_UID, "Kimlik okunamadı.", path))?;
    if doc.slot_of(uid).and_then(|s| doc.get(s)).is_none() {
        return Err(refuse(
            codes::ENTITY_NOT_FOUND,
            "Nesne çizimde yok; kaynağı ve kapsamı yeniden seçin.",
            path,
        ));
    }
    Ok(uid)
}
fn compatible(a: &InsertEntity, b: &InsertEntity, polar: bool) -> bool {
    let angle = (a.rotation - b.rotation).rem_euclid(std::f64::consts::TAU);
    arrays_adopt::plain(b)
        && arrays_adopt::same_appearance(a, b)
        && a.block == b.block
        && a.mirror == b.mirror
        && b.rotation.is_finite()
        && (polar || angle.min(std::f64::consts::TAU - angle) <= 1e-10)
        && b.scale.is_finite()
        && b.scale > 0.
        && (a.scale - b.scale).abs() <= a.scale.abs() * 1e-12
        && b.p.x.is_finite()
        && b.p.y.is_finite()
}

struct Checked {
    seed: Uuid,
    scope: BTreeSet<Uuid>,
    tolerance: f64,
    limit: usize,
}
fn check(
    doc: &Document,
    input: &ArraysFind,
    stopped: &mut impl FnMut() -> bool,
) -> Result<Checked, Stop> {
    if stopped() {
        return Err(Stop::Cancelled);
    }
    let tolerance = input.tolerance.unwrap_or(1e-6);
    if !tolerance.is_finite() || tolerance <= 0. || tolerance > 0.001 {
        return Err(refuse(
            codes::INVALID_TOLERANCE,
            "Tolerans sıfırdan büyük ve en çok 0,001 metre olmalı.",
            "tolerance",
        ));
    }
    let limit = input.limit.unwrap_or(100);
    if !(1..=100).contains(&limit) {
        return Err(refuse(
            codes::INVALID_LIMIT,
            "Aday sınırı 1 ile 100 arasında olmalı.",
            "limit",
        ));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let seed = named(doc, &input.source, "source")?;
    arrays_adopt::prototype(doc, seed, "source")?;
    if input.kind == Some(ArrayFindKind::Path) {
        path::guide(doc, input)?;
    } else if input.path.is_some() {
        return Err(refuse(codes::INVALID_PATH, "Kılavuz yolu yalnız Yol boyunca düzeniyle verin.", "path"));
    }
    for array in doc.arrays() {
        for item in &array.items {
            if stopped() {
                return Err(Stop::Cancelled);
            }
            if item.uid == Some(EntityId(seed.into_bytes())) {
                return Err(refuse(
                    codes::ARRAY_ADOPTION_REFUSED,
                    "Kaynak zaten bir ilişkili dizinin öğesi.",
                    "source",
                ));
            }
        }
    }
    let mut scope = BTreeSet::new();
    match &input.uids {
        Some(ids) => {
            if ids.len() > ARRAY_FIND_SCOPE_MAX {
                return Err(scope_limit());
            }
            for (i, text) in ids.iter().enumerate() {
                if stopped() {
                    return Err(Stop::Cancelled);
                }
                scope.insert(named(doc, text, &format!("uids[{i}]"))?);
            }
        }
        None => {
            for (uid, _) in doc.entities_with_uids() {
                if stopped() {
                    return Err(Stop::Cancelled);
                }
                if scope.len() >= ARRAY_FIND_SCOPE_MAX {
                    return Err(scope_limit());
                }
                scope.insert(uid);
            }
        }
    }
    scope.insert(seed);
    if scope.len() > ARRAY_FIND_SCOPE_MAX {
        return Err(scope_limit());
    }
    Ok(Checked {
        seed,
        scope,
        tolerance,
        limit: limit as usize,
    })
}
fn scope_limit() -> Stop {
    refuse(
        codes::RECOGNITION_SCOPE_LIMIT,
        "Bu dizi örüntü araması en çok 2 048 nesne alır; kapsamı daraltın.",
        "uids",
    )
}

#[derive(Clone, Copy)]
struct Point {
    uid: EntityId,
    x: f64,
    y: f64,
}
struct Budget {
    work: u64,
    cut: bool,
}
impl Budget {
    fn tick(&mut self, stopped: &mut impl FnMut() -> bool) -> Result<bool, Stop> {
        if stopped() {
            return Err(Stop::Cancelled);
        }
        if self.work >= WORK_MAX {
            self.cut = true;
            return Ok(false);
        }
        self.work += 1;
        Ok(true)
    }
}
// The slot topology comes from observed adjacent intervals. Refine its displacement from
// the full observed span, then try the original local interval as a fallback. World-space
// rounding of one short gap must not get accumulated over every slot. The unchanged adoption
// validator remains the only acceptance gate, at the caller's original absolute tolerance.
fn review_layouts(
    doc: &Document,
    source: EntityId,
    items: &[ArrayItem],
    layouts: impl IntoIterator<Item = ArrayLayout>,
    tolerance: f64,
    budget: &mut Budget,
    stopped: &mut impl FnMut() -> bool,
) -> Result<Option<(ArraysAdopt, ArraysAdoptPlan)>, Stop> {
    for layout in layouts {
        if !budget.tick(stopped)? {
            return Ok(None);
        }
        let adoption = ArraysAdopt {
            source: source.to_text(),
            layout,
            items: items.to_vec(),
            tolerance: Some(tolerance),
            expected_revision: Some(doc.revision().to_string()),
        };
        match arrays_adopt::inspect(doc, &adoption, &mut *stopped) {
            CommandResult::Completed { output, .. } => return Ok(Some((adoption, output))),
            CommandResult::Cancelled => return Err(Stop::Cancelled),
            _ => {}
        }
    }
    Ok(None)
}

fn search(
    doc: &Document,
    input: &ArraysFind,
    mut stopped: impl FnMut() -> bool,
) -> Result<ArraysFound, Stop> {
    let checked = check(doc, input, &mut stopped)?;
    let source = arrays_adopt::prototype(doc, checked.seed, "source")?;
    let mut owned = HashSet::new();
    for a in doc.arrays() {
        for item in &a.items {
            if stopped() {
                return Err(Stop::Cancelled);
            }
            if let Some(uid) = item
                .uid
                .filter(|id| checked.scope.contains(&Uuid::from_bytes(id.0)))
            {
                owned.insert(uid);
            }
        }
    }
    let mut points = Vec::new();
    let mut excluded = BTreeMap::new();
    for uid in checked.scope {
        if stopped() {
            return Err(Stop::Cancelled);
        }
        let e = doc
            .slot_of(uid)
            .and_then(|slot| doc.get(slot))
            .ok_or_else(|| {
                refuse(
                    codes::ENTITY_NOT_FOUND,
                    "Kapsam nesnesi çizimde yok; aramayı yenileyin.",
                    "uids",
                )
            })?;
        let id = EntityId(uid.into_bytes());
        let reason = match e {
            Entity::Insert(_) if owned.contains(&id) => Some("array_member"),
            Entity::Insert(i) if doc.layers().is_locked(&i.base.layer_id) => Some("layer_locked"),
            Entity::Insert(i) if compatible(source, i, matches!(input.kind, Some(ArrayFindKind::Polar | ArrayFindKind::Path))) => {
                points.push(Point {
                    uid: id,
                    x: i.p.x,
                    y: i.p.y,
                });
                None
            }
            Entity::Insert(_) => Some("template_differs"),
            _ => Some("not_insert"),
        };
        if let Some(reason) = reason {
            excluded.insert(uid.to_string(), reason.into());
        }
    }
    let seed = Point {
        uid: EntityId(checked.seed.into_bytes()),
        x: source.p.x,
        y: source.p.y,
    };
    let tolerance = checked.tolerance;
    let mut budget = Budget {
        work: 0,
        cut: false,
    };
    let mut matches: Vec<ArrayPattern> = Vec::new();
    if input.kind == Some(ArrayFindKind::Path) {
        matches = path::find(doc, input, &points, seed, tolerance, checked.limit, &mut budget, &mut stopped)?;
    } else if input.kind == Some(ArrayFindKind::Polar) {
        matches = polar::find(doc, &points, seed, tolerance, checked.limit, &mut budget, &mut stopped)?;
    } else if input.kind == Some(ArrayFindKind::Lattice) {
        matches = lattice::find(doc,&points,seed,tolerance,checked.limit,&mut budget,&mut stopped)?;
    } else if input.kind == Some(ArrayFindKind::Grid) {
        matches = grid::find(
            doc,
            &points,
            seed,
            tolerance,
            checked.limit,
            &mut budget,
            &mut stopped,
        )?;
    } else {
        let by_uid: BTreeMap<_, _> = points.iter().map(|p| (p.uid, *p)).collect();
        let mut directions: Vec<(f64, f64)> = Vec::new();
        let mut seen = HashSet::new();
        // All arithmetic is Float64 relative to the seed. A direction only groups points;
        // the exact existing placement engine validates every returned proposal again.
        'outer: for other in &points {
            if !budget.tick(&mut stopped)? {
                break;
            }
            let (dx, dy) = (other.x - seed.x, other.y - seed.y);
            let length = dx.hypot(dy);
            if !length.is_finite() || length <= tolerance {
                continue;
            }
            let (mut ux, mut uy) = (dx / length, dy / length);
            if ux < 0. || (ux == 0. && uy < 0.) {
                ux = -ux;
                uy = -uy;
            }
            // Only exact unit directions are coalesced. Tolerance bucketing could suppress
            // a second real line at large coordinates or with a different length.
            let mut duplicate = false;
            for &(x, y) in &directions {
                if !budget.tick(&mut stopped)? {
                    break 'outer;
                }
                if ux.to_bits() == x.to_bits() && uy.to_bits() == y.to_bits() {
                    duplicate = true;
                    break;
                }
            }
            if duplicate {
                continue;
            }
            directions.push((ux, uy));
            let mut line = Vec::new();
            for point in &points {
                if !budget.tick(&mut stopped)? {
                    break 'outer;
                }
                let (x, y) = (point.x - seed.x, point.y - seed.y);
                let along = x * ux + y * uy;
                let across = x * uy - y * ux;
                if along.is_finite() && across.is_finite() && across.abs() <= tolerance {
                    line.push((along, *point));
                }
            }
            if line.len() < 3 {
                continue;
            }
            line.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.uid.cmp(&b.1.uid)));
            let mut steps = Vec::<f64>::new();
            for pair in line.windows(2) {
                let step = pair[1].0 - pair[0].0;
                if !step.is_finite() || step <= tolerance * 2. || step <= 1e-9 {
                    continue;
                }
                let mut duplicate = false;
                for old in &steps {
                    if !budget.tick(&mut stopped)? {
                        break 'outer;
                    }
                    if old.to_bits() == step.to_bits() {
                        duplicate = true;
                        break;
                    }
                }
                if !duplicate {
                    steps.push(step);
                }
            }
            let mut step_cursor = 0;
            while step_cursor < steps.len() {
                let step = steps[step_cursor];
                step_cursor += 1;
                let mut places = BTreeMap::<i32, EntityId>::new();
                let mut collision = false;
                for &(along, point) in &line {
                    if !budget.tick(&mut stopped)? {
                        break 'outer;
                    }
                    let k = (along / step).round();
                    if !k.is_finite() || k.abs() > 9999. {
                        continue;
                    }
                    let (x, y) = (seed.x + k * step * ux, seed.y + k * step * uy);
                    if (point.x - x).hypot(point.y - y) > tolerance {
                        continue;
                    }
                    if places.insert(k as i32, point.uid).is_some() {
                        collision = true;
                        break;
                    }
                }
                if collision || places.len() < 3 || !places.values().any(|u| *u == seed.uid) {
                    continue;
                }
                let (Some((&min, &first)), Some((&max, &last))) =
                    (places.first_key_value(), places.last_key_value())
                else {
                    continue;
                };
                if max - min >= 10000 || !places.keys().any(|k| places.contains_key(&(k + 1))) {
                    continue;
                }
                let (Some(start), Some(end)) = (by_uid.get(&first), by_uid.get(&last)) else {
                    continue;
                };
                let span = (max - min) as f64;
                let refined = ((end.x - start.x) * ux + (end.y - start.y) * uy) / span;
                if refined.is_finite() && refined > tolerance * 2. && refined > 1e-9 {
                    let mut known = false;
                    for old in &steps {
                        if !budget.tick(&mut stopped)? {
                            break 'outer;
                        }
                        if old.to_bits() == refined.to_bits() {
                            known = true;
                            break;
                        }
                    }
                    // Recheck the whole line with the refined pitch. An endpoint that was
                    // outside the first rounded short-gap estimate may now be a valid member.
                    if !known {
                        steps.push(refined);
                    }
                }
                let items: Vec<_> = (min..=max)
                    .map(|k| ArrayItem {
                        uid: places.get(&k).copied(),
                        kept: false,
                    })
                    .collect();
                let key: Vec<_> = items.iter().map(|i| i.uid).collect();
                if seen.contains(&key) {
                    continue;
                }
                let count = items.len() as u32;
                let layouts = [
                    ArrayLayout::Line {
                        count,
                        dx: (end.x - start.x) / span,
                        dy: (end.y - start.y) / span,
                        fit: false,
                    },
                    ArrayLayout::Line {
                        count,
                        dx: step * ux,
                        dy: step * uy,
                        fit: false,
                    },
                ];
                let Some((adoption, plan)) = review_layouts(
                    doc,
                    first,
                    &items,
                    layouts,
                    tolerance,
                    &mut budget,
                    &mut stopped,
                )?
                else {
                    continue;
                };
                // Only an accepted proposal suppresses a later proposal with the same members.
                seen.insert(key);
                matches.push(ArrayPattern {
                    present: places.len() as u32,
                    missing: plan.missing,
                    deviation: plan.deviation,
                    adoption,
                    overlaps: false,
                });
                if matches.len() > checked.limit {
                    budget.cut = true;
                    break 'outer;
                }
            }
        }
    }
    // Prefer the fullest explanation. Suppress strict subsets of a returned pattern;
    // crossing lines and mutually exclusive alternatives remain explicit.
    matches.sort_by(|a, b| {
        b.present
            .cmp(&a.present)
            .then(a.missing.cmp(&b.missing))
            .then(a.adoption.source.cmp(&b.adoption.source))
    });
    let mut maximal: Vec<ArrayPattern> = Vec::new();
    for candidate in matches {
        let ids: HashSet<_> = candidate
            .adoption
            .items
            .iter()
            .filter_map(|i| i.uid)
            .collect();
        if maximal.iter().any(|m| {
            let prior: HashSet<_> = m.adoption.items.iter().filter_map(|i| i.uid).collect();
            ids.is_subset(&prior)
        }) {
            continue;
        }
        maximal.push(candidate);
    }
    maximal.truncate(checked.limit);
    let mut uses = BTreeMap::new();
    for m in &maximal {
        for id in m.adoption.items.iter().filter_map(|i| i.uid) {
            *uses.entry(id).or_insert(0) += 1;
        }
    }
    for m in &mut maximal {
        m.overlaps = m
            .adoption
            .items
            .iter()
            .filter_map(|i| i.uid)
            .any(|id| uses.get(&id).is_some_and(|n| *n > 1));
    }
    Ok(ArraysFound {
        matches: maximal,
        eligible: points.iter().map(|p| p.uid.to_text()).collect(),
        excluded,
        truncated: budget.cut,
        comparisons: budget.work,
        revision: doc.revision().to_string(),
    })
}

pub fn inspect(
    doc: &Document,
    input: &ArraysFind,
    stopped: impl FnMut() -> bool,
) -> CommandResult<ArraysFound> {
    match search(doc, input, stopped) {
        Ok(output) => {
            let warnings = if output.truncated {
                vec![CommandWarning {
                    code: codes::RECOGNITION_TRUNCATED.into(),
                    message:
                        "Arama sınırına ulaşıldı; sonuçlar kısmi. Kapsamı daraltıp yeniden arayın."
                            .into(),
                    path: Some("uids".into()),
                }]
            } else {
                Vec::new()
            };
            CommandResult::Completed { output, warnings }
        }
        Err(e) => e.into(),
    }
}
pub fn validate(cx: &ExecutionContext<'_>, input: &ArraysFind) -> CommandResult<()> {
    match check(cx.doc, input, &mut || cx.stopped()) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(e) => e.into(),
    }
}
pub fn plan(cx: &ExecutionContext<'_>, input: &ArraysFind) -> CommandResult<ArraysFound> {
    inspect(cx.doc, input, || cx.stopped())
}
pub fn execute(cx: &mut ExecutionContext<'_>, input: ArraysFind) -> CommandResult<ArraysFound> {
    plan(cx, &input)
}
