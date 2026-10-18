//! `cad.walls.convert` v1 (docs/adr/0339 §5, plan/cad.md `CAD-ARCH-09`): walls from the drawing's line work, one
//! undo step “Çizgiden duvar”. Tek tek: a wall along each line, polyline, area's ring, arc or circle. Çiftler: two
//! parallel straight lines, the nearest first, a wall between them. The sources a wall covers whole are taken away
//! unless kept. The shared cases are `fixtures/commands/v1/cad.walls.convert.json`.
//!
//! Pairing (`pairs`), worked out on the straight sources in the input's order: lines `i < j` pair when their
//! directions are within `angle` degrees (`atan2(|u×v|, |u·v|)`), `j`'s middle lies `d` from `i`'s line with
//! `0 < d ≤ maxWidth`, and `j`'s ends projected on `i` overlap `i` over more than 1 µm. The candidates are taken by `d`,
//! then the longer overlap, then `i`, then `j`; a line already taken is passed. The wall runs along `i` over the overlap,
//! moved `d / 2` towards `j`, `d` wide, centred.

use kentos_contracts::{
    CommandResult, CommandWarning, Entity, EntityBase, NewWall, Vec2, WallEntity, WallFields,
    WallJustify, WallStyleDef, WallsFromLines, WallsFromLinesDone, WallsFromLinesPlan,
    WallsFromMode,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::jsmath::{atan2, cos, js_hypot, sin, tan};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::walls;

pub const STEP: &str = "Çizgiden duvar";
/// Overlaps and covers are measured to a micrometre.
const EPS: f64 = 1e-6;

/// A source's path as a new wall takes it: its vertices, one bulge per edge, whether it closes.
type Path = (Vec<Vec2>, Option<Vec<f64>>, bool);

/// A source a wall is drawn along, or none for another kind.
pub fn path_of(e: &Entity) -> Option<Path> {
    let per_edge = |b: &Option<Vec<f64>>, n: usize| {
        b.as_ref()
            .filter(|b| b.iter().take(n).any(|x| *x != 0.0))
            .map(|b| {
                b.iter()
                    .copied()
                    .take(n)
                    .chain(std::iter::repeat(0.0))
                    .take(n)
                    .collect::<Vec<f64>>()
            })
    };
    match e {
        Entity::Line(l) => Some((vec![l.a, l.b], None, false)),
        Entity::Polyline(p) if p.parts.is_none() && p.pts.len() >= 2 => {
            let edges = if p.closed {
                p.pts.len()
            } else {
                p.pts.len() - 1
            };
            Some((p.pts.clone(), per_edge(&p.bulges, edges), p.closed))
        }
        Entity::Polygon(p) if p.parts.is_none() && p.pts.len() >= 2 => {
            Some((p.pts.clone(), per_edge(&p.bulges, p.pts.len()), true))
        }
        Entity::Arc(a) => {
            let sweep = kentos_geometry_core::geom::arc::sweep(a.a0, a.a1);
            let at = |t: f64| Vec2 {
                x: a.c.x + a.r * cos(t),
                y: a.c.y + a.r * sin(t),
            };
            Some((
                vec![at(a.a0), at(a.a1)],
                Some(vec![tan(sweep / 4.0)]),
                false,
            ))
        }
        Entity::Circle(c) => Some((
            vec![
                Vec2 {
                    x: c.c.x + c.r,
                    y: c.c.y,
                },
                Vec2 {
                    x: c.c.x - c.r,
                    y: c.c.y,
                },
            ],
            Some(vec![1.0, 1.0]),
            true,
        )),
        _ => None,
    }
}

/// A straight source: a line, or an open polyline of two vertices without an arc.
fn straight(e: &Entity) -> Option<(Vec2, Vec2)> {
    match e {
        Entity::Line(l) => Some((l.a, l.b)),
        Entity::Polyline(p)
            if p.parts.is_none()
                && !p.closed
                && p.pts.len() == 2
                && p.bulges
                    .as_ref()
                    .is_none_or(|b| b.iter().all(|x| *x == 0.0)) =>
        {
            Some((p.pts[0], p.pts[1]))
        }
        _ => None,
    }
}

/// A pair's wall: its path, width, and whether it covers each line whole.
struct Paired {
    i: usize,
    j: usize,
    path: [Vec2; 2],
    width: f64,
    covers: [bool; 2],
}

/// The pairs of `lines` (each its index among the sources and its ends), as the module's comment says.
fn pairs(lines: &[(usize, Vec2, Vec2)], max_width: f64, angle: f64) -> Vec<Paired> {
    struct Candidate {
        d: f64,
        overlap: f64,
        i: usize,
        j: usize,
        lo: f64,
        hi: f64,
        side: f64,
        j_in: bool,
        i_whole: bool,
    }
    let mut candidates = Vec::new();
    for (x, &(i, a, b)) in lines.iter().enumerate() {
        let len = js_hypot(b.x - a.x, b.y - a.y);
        if len <= EPS {
            continue;
        }
        let u = Vec2 {
            x: (b.x - a.x) / len,
            y: (b.y - a.y) / len,
        };
        for &(j, c, e) in &lines[x + 1..] {
            let lj = js_hypot(e.x - c.x, e.y - c.y);
            if lj <= EPS {
                continue;
            }
            let v = Vec2 {
                x: (e.x - c.x) / lj,
                y: (e.y - c.y) / lj,
            };
            let cross = u.x * v.y - u.y * v.x;
            let dot = u.x * v.x + u.y * v.y;
            if atan2(cross.abs(), dot.abs()) * 180.0 / std::f64::consts::PI > angle {
                continue;
            }
            let mid = Vec2 {
                x: (c.x + e.x) / 2.0,
                y: (c.y + e.y) / 2.0,
            };
            let across = u.x * (mid.y - a.y) - u.y * (mid.x - a.x);
            let d = across.abs();
            if !(d > EPS && d <= max_width) {
                continue;
            }
            let t = |p: Vec2| (p.x - a.x) * u.x + (p.y - a.y) * u.y;
            let (t1, t2) = (t(c), t(e));
            let (min_j, max_j) = if t1 <= t2 { (t1, t2) } else { (t2, t1) };
            let lo = if min_j > 0.0 { min_j } else { 0.0 };
            let hi = if max_j < len { max_j } else { len };
            if hi - lo <= EPS {
                continue;
            }
            candidates.push(Candidate {
                d,
                overlap: hi - lo,
                i,
                j,
                lo,
                hi,
                side: if across > 0.0 { 1.0 } else { -1.0 },
                j_in: min_j >= lo - EPS && max_j <= hi + EPS,
                i_whole: lo <= EPS && hi >= len - EPS,
            });
        }
    }
    candidates.sort_by(|p, q| {
        p.d.total_cmp(&q.d)
            .then(q.overlap.total_cmp(&p.overlap))
            .then(p.i.cmp(&q.i))
            .then(p.j.cmp(&q.j))
    });
    let mut taken = std::collections::HashSet::new();
    let mut out = Vec::new();
    for c in candidates {
        if taken.contains(&c.i) || taken.contains(&c.j) {
            continue;
        }
        taken.insert(c.i);
        taken.insert(c.j);
        let &(_, a, b) = lines.iter().find(|(k, ..)| *k == c.i).unwrap_or(&lines[0]);
        let len = js_hypot(b.x - a.x, b.y - a.y);
        let u = Vec2 {
            x: (b.x - a.x) / len,
            y: (b.y - a.y) / len,
        };
        let n = Vec2 { x: -u.y, y: u.x };
        let half = c.side * c.d / 2.0;
        let at = |t: f64| Vec2 {
            x: a.x + u.x * t + n.x * half,
            y: a.y + u.y * t + n.y * half,
        };
        out.push(Paired {
            i: c.i,
            j: c.j,
            path: [at(c.lo), at(c.hi)],
            width: c.d,
            covers: [c.i_whole, c.j_in],
        });
    }
    out
}

/// What the call does: the walls with their layers and where each goes among the sources, the sources taken away and
/// those left, KentOS's styles taken, the warnings.
struct Work {
    walls: Vec<(usize, String, WallFields)>,
    removed: Vec<String>,
    left: Vec<String>,
    added: Vec<WallStyleDef>,
    warnings: Vec<CommandWarning>,
}

fn check(doc: &Document, input: &WallsFromLines) -> Result<Work, Stop> {
    checks::uids(&input.uids, "Duvara çevrilecek çizgi verilmedi.")?;
    let found = checks::named(doc, &input.uids)?;
    let (found, locked) = checks::skip_locked(doc, found)?;
    let mut warnings: Vec<CommandWarning> = locked.into_iter().collect();
    for (v, what, name) in [
        (input.width, "width", "Genişlik"),
        (input.max_width, "maxWidth", "En büyük genişlik"),
        (input.angle, "angle", "Açı"),
    ] {
        if let Some(x) = v {
            checks::finite(x, name, "Sonlu bir sayı verin.", what)?;
        }
    }
    let layer_of = |e: &Entity| {
        input
            .layer_id
            .clone()
            .unwrap_or_else(|| e.base().layer_id.clone())
    };
    if let Some(l) = &input.layer_id {
        warnings.extend(checks::layer_at(doc, l, "layerId")?);
    }
    let mut added = Vec::new();
    let style_key = input.style.clone();
    // A pair's wall takes the style only when its components fit the pair's width: the lines are its faces.
    let style_fits = |width: f64| match &input.style {
        Some(key) => walls::find_style(doc, key).is_none_or(|(s, _)| {
            s.components.is_empty()
                || kentos_contracts::component_thicknesses(width, &s.components).is_some()
        }),
        None => true,
    };
    let wall_of = |path: Path,
                   width: Option<f64>,
                   justify: Option<WallJustify>,
                   style: Option<String>| NewWall {
        pts: path.0,
        bulges: path.1,
        closed: path.2,
        width,
        justify,
        offset: None,
        style,
        caps: None,
        height: None,
        arch: None,
        attrs: None,
        label: None,
    };
    let mut work = Work {
        walls: Vec::new(),
        removed: Vec::new(),
        left: Vec::new(),
        added: Vec::new(),
        warnings: Vec::new(),
    };
    let mut other = Vec::new();
    match input.mode {
        WallsFromMode::Each => {
            for (k, (at, _, e, uid)) in found.iter().enumerate() {
                let Some(path) = path_of(e) else {
                    other.push((*at, (*uid).clone()));
                    continue;
                };
                let given = wall_of(path, input.width, input.justify, style_key.clone());
                let wall = walls::build(doc, &given, &format!("uids[{at}]"), &mut added)?;
                work.walls.push((k, layer_of(e), wall));
                if !input.keep_sources {
                    work.removed.push((*uid).clone());
                }
            }
        }
        WallsFromMode::Pairs => {
            let mut lines = Vec::new();
            for (k, (at, _, e, uid)) in found.iter().enumerate() {
                match straight(e) {
                    Some((a, b)) => lines.push((k, a, b)),
                    None => other.push((*at, (*uid).clone())),
                }
            }
            let max_width = input
                .max_width
                .unwrap_or(kentos_contracts::DEFAULT_PAIR_WIDTH);
            let angle = input.angle.unwrap_or(kentos_contracts::DEFAULT_PAIR_ANGLE);
            let paired = pairs(&lines, max_width, angle);
            let mut kept = 0;
            for p in &paired {
                let (at, _, e, _) = &found[p.i];
                let style = style_key.clone().filter(|_| style_fits(p.width));
                let given = wall_of(
                    (p.path.to_vec(), None, false),
                    Some(p.width),
                    Some(WallJustify::Center),
                    style,
                );
                let wall = walls::build(doc, &given, &format!("uids[{at}]"), &mut added)?;
                work.walls.push((p.i, layer_of(e), wall));
                for (k, covers) in [(p.i, p.covers[0]), (p.j, p.covers[1])] {
                    if input.keep_sources {
                        continue;
                    }
                    if covers {
                        work.removed.push(found[k].3.clone());
                    } else {
                        kept += 1;
                    }
                }
            }
            if kept > 0 {
                work.warnings.push(CommandWarning {
                    code: codes::SOURCE_KEPT.into(),
                    message: format!(
                        "{kept} çizginin duvarın örtmediği parçası var; o çizgiler silinmedi."
                    ),
                    path: Some("uids".into()),
                });
            }
            let used: std::collections::HashSet<usize> =
                paired.iter().flat_map(|p| [p.i, p.j]).collect();
            let unpaired: Vec<(usize, String)> = lines
                .iter()
                .filter(|(k, ..)| !used.contains(k))
                .map(|(k, ..)| (found[*k].0, found[*k].3.clone()))
                .collect();
            if !unpaired.is_empty() {
                if paired.is_empty() && other.is_empty() {
                    return Err(Stop::failed(error(
                        codes::WALL_UNPAIRED,
                        format!(
                            "Hiçbir çizginin karşısında {max_width} m içinde paralel ve örtüşen bir çizgi yok. En büyük genişliği artırın ya da çizgileri Tek tek çevirin."
                        ),
                        Some("uids".into()),
                    )));
                }
                work.warnings.push(CommandWarning {
                    code: codes::WALL_UNPAIRED.into(),
                    message: format!(
                        "{} çizginin karşısında {max_width} m içinde paralel ve örtüşen bir çizgi yok; duvara çevrilmedi.",
                        unpaired.len()
                    ),
                    path: Some(format!("uids[{}]", unpaired[0].0)),
                });
                work.left.extend(unpaired.into_iter().map(|(_, u)| u));
            }
        }
    }
    if work.walls.is_empty() {
        let at = other.first().map_or(0, |o| o.0);
        return Err(Stop::failed(error(
            codes::NOT_A_WALL_PATH,
            match input.mode {
                WallsFromMode::Each => "Verilen nesnelerden duvar çizilmez: çizgi, çoklu çizgi, kapalı alan, yay ya da daire verin.".into(),
                WallsFromMode::Pairs => "Çiftler yalnız düz çizgileri (çizgi, iki köşeli çoklu çizgi) eşler; verilenlerin hiçbiri düz çizgi değil.".into(),
            },
            Some(format!("uids[{at}]")),
        )));
    }
    if !other.is_empty() {
        work.warnings.push(CommandWarning {
            code: codes::NOT_A_WALL_PATH.into(),
            message: format!(
                "{} nesneden duvar çizilmez ({}); bırakıldı.",
                other.len(),
                match input.mode {
                    WallsFromMode::Each => "çizgi, çoklu çizgi, kapalı alan, yay ya da daire değil",
                    WallsFromMode::Pairs => "düz çizgi değil",
                }
            ),
            path: Some(format!("uids[{}]", other[0].0)),
        });
        work.left.extend(other.into_iter().map(|(_, u)| u));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    // The walls' layers, each once: a source's own when none is given.
    let mut seen = std::collections::HashSet::new();
    for (_, layer, _) in &work.walls {
        if input.layer_id.is_none() && seen.insert(layer.clone()) {
            warnings.extend(checks::layer(doc, layer)?);
        }
    }
    work.walls.sort_by_key(|(k, ..)| *k);
    work.added = added;
    warnings.append(&mut work.warnings);
    work.warnings = warnings;
    Ok(work)
}

fn entity(layer: String, wall: WallFields) -> Entity {
    Entity::Wall(WallEntity {
        base: EntityBase {
            id: 0,
            layer_id: layer,
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
            look: Default::default(),
        },
        wall,
    })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &WallsFromLines) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(work) => CommandResult::Completed {
            output: (),
            warnings: work.warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &WallsFromLines,
) -> CommandResult<WallsFromLinesPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document: what Çizgiden duvar previews.
pub fn plan_in(doc: &Document, input: &WallsFromLines) -> CommandResult<WallsFromLinesPlan> {
    match check(doc, input) {
        Ok(work) => CommandResult::Completed {
            output: WallsFromLinesPlan {
                walls: work
                    .walls
                    .into_iter()
                    .map(|(_, l, w)| entity(l, w))
                    .collect(),
                removed: work.removed,
                left: work.left,
                styles_added: work.added,
                revision: doc.revision().to_string(),
            },
            warnings: work.warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: WallsFromLines,
) -> CommandResult<WallsFromLinesDone> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_WALLS_FROM_LINES,
        kentos_contracts::CAD_WALLS_FROM_LINES_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: WallsFromLines,
) -> CommandResult<WallsFromLinesDone> {
    let work = match check(cx.doc, &input) {
        Ok(work) => work,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    walls::take_styles(doc, &work.added);
    let entities: Vec<Entity> = work
        .walls
        .into_iter()
        .map(|(_, l, w)| entity(l, w))
        .collect();
    let slots = match doc.add_many(entities, STEP) {
        Ok(slots) => slots,
        Err(full) => {
            return Stop::failed(error(codes::SLOTS_EXHAUSTED, full.to_string(), None)).into();
        }
    };
    let gone: Vec<Slot> = work
        .removed
        .iter()
        .filter_map(|u| kentos_domain::Uuid::parse_str(u).ok())
        .filter_map(|u| doc.slot_of(u))
        .collect();
    if !gone.is_empty() {
        doc.remove(&gone);
    }
    doc.name_step(STEP);
    CommandResult::Completed {
        output: WallsFromLinesDone {
            uids: slots
                .iter()
                .filter_map(|s| doc.uid(*s))
                .map(|u| u.to_string())
                .collect(),
            removed: work.removed,
            left: work.left,
            revision: doc.revision().to_string(),
        },
        warnings: work.warnings,
    }
}
