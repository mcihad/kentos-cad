//! `cad.walls.edit` v1 (docs/adr/0339 §5, 0340 §4): walls named by their persistent ids changed as one undo step
//! “Duvarı düzenle”, all or none. Değiştir sets their own fields (Öznitelikler writes through it); Böl, Birleştir,
//! Uzat, Buda and Çevir edit their paths; Stili değiştir gives the walls of one style another; Birleşim and
//! Birleşimleri sıfırla set how their ends join; Kaydır and Mesafe move one straight edge square to itself, what is
//! joined to it following (docs/adr/0710, `wall_distances`). The shared cases are
//! `fixtures/commands/v1/cad.walls.edit.json` and `cad.walls.edit.shift.json`.
//!
//! A style given takes its components, ends and fill (and its width when its components fix it); a style taken away
//! leaves one plain body. A width the components do not fit, a face a new width, offset or path cannot draw, a level
//! the project does not have: nothing is written. The joins with the walls round them follow in the same step (they
//! are drawn from the walls); what then does not join is said, with what resolves it.

use kentos_commands::{
    CommandResult, Entity, OpeningEntity, Vec2, WallChange, WallEnd, WallEntity, WallFields,
    WallJoin, WallJoins, WallJustify, WallStyleDef, WallsEdit, WallsEdited, WallsOperation,
    fixed_width,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::geom::wall_join::JOIN_TOLERANCE;
use kentos_geometry_core::ops::wall_edit::{self, SplitRefusal, WallPath};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::walls;

pub const STEP: &str = "Duvarı düzenle";

/// What an edit writes: the walls changed (each its slot), the new ones, the ones gone, KentOS's styles taken.
#[derive(Default)]
struct Work {
    updates: Vec<(Slot, WallEntity)>,
    added: Vec<WallEntity>,
    /// The wall each added one was split from.
    origins: Vec<Slot>,
    removed: Vec<Slot>,
    styles: Vec<WallStyleDef>,
    /// The openings the walls carry (docs/adr/0341 §3): each its slot, itself with its new place along its wall, and
    /// its wall: an updated one by its slot, or an added one by its place in `added`.
    openings: Vec<(Slot, OpeningEntity, Carrier)>,
    /// Kaydır: how many of `updates` are the walls named (the rest followed); none: all.
    named: Option<usize>,
    /// Kaydır: the locked layer of a wall that would follow, refused after the walls named.
    blocked: Option<String>,
    /// Kaydır, Mesafe and Boy: the edge moved (docs/adr/0770 §3).
    edge: Option<usize>,
    /// Mesafe's `keep`: the bond to write in the same step (docs/adr/0770 §5).
    keep: Option<kentos_commands::ConstraintRule>,
}

/// The wall an opening goes into.
#[derive(Clone, Copy)]
enum Carrier {
    Kept(Slot),
    Added(usize),
}

/// The openings of the walls a path edit changed, each read back from its jambs onto the new base line of its wall,
/// or of the part of it nearest its jambs (Böl), or of the wall it was joined into (Birleştir): its `at`, and its hand
/// and side when the part runs the other way. One that falls off its wall's new line breaks (`offWall`) in the step.
fn carry(doc: &Document, work: &Work) -> Vec<(Slot, OpeningEntity, Carrier)> {
    use kentos_domain::opening_follow::{WallLine, anchor, wall_line};
    use kentos_geometry_core::geom::opening::{centre, read_back};
    use kentos_geometry_core::ops::path::{nearest_s, point_at_s};
    let line = |w: &WallEntity| wall_line(&Entity::Wall(w.clone()));
    let kept: Vec<(Slot, Option<WallLine>)> =
        work.updates.iter().map(|(s, w)| (*s, line(w))).collect();
    let added: Vec<Option<WallLine>> = work.added.iter().map(line).collect();
    let mut out = Vec::new();
    let sources = work
        .updates
        .iter()
        .map(|(s, _)| *s)
        .chain(work.removed.iter().copied());
    for source in sources {
        let Some(uid) = doc.uid(source).map(|u| u.to_string()) else {
            continue;
        };
        let mut cands: Vec<(Carrier, &WallLine)> = Vec::new();
        match kept.iter().find(|(s, _)| *s == source) {
            Some((s, l)) => {
                cands.extend(l.as_ref().map(|l| (Carrier::Kept(*s), l)));
                for (k, o) in work.origins.iter().enumerate() {
                    if *o == source {
                        cands.extend(added[k].as_ref().map(|l| (Carrier::Added(k), l)));
                    }
                }
            }
            // A wall Birleştir removes: the openings go into the wall it was joined into.
            None => {
                if let Some((s, Some(l))) = kept.first() {
                    cands.push((Carrier::Kept(*s), l));
                }
            }
        }
        for (slot, o) in doc.openings_of(&uid) {
            let Some(Entity::Opening(e)) = doc.get(slot) else {
                continue;
            };
            let place: Vec<CoreVec2> = o.place.iter().map(|q| CoreVec2::new(q.x, q.y)).collect();
            let Some(m) = centre(&place) else { continue };
            let off = |l: &WallLine| {
                let p = point_at_s(&l.path, nearest_s(&l.path, m));
                (p.x - m.x).hypot(p.y - m.y)
            };
            let Some((carrier, l)) = cands
                .iter()
                .min_by(|a, b| off(a.1).total_cmp(&off(b.1)))
                .copied()
            else {
                continue;
            };
            let mut next = e.clone();
            if let Some(r) = read_back(&l.path, &place, anchor(&o), o.width) {
                next.opening.at = r.at;
                if r.hand_turned {
                    next.opening.hand = o.hand.other();
                }
                if r.side_turned {
                    next.opening.side = o.side.other();
                }
            }
            out.push((slot, next, carrier));
        }
    }
    out
}

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// `w` changed as `c` says, `style` the style a change names (found), `at` where the input names it.
pub(crate) fn changed(
    doc: &Document,
    w: &WallEntity,
    c: &WallChange,
    style: Option<&WallStyleDef>,
    at: &str,
) -> Result<WallEntity, Stop> {
    let mut out = w.clone();
    let f = &mut out.wall;
    match (&c.style, style) {
        (Some(Some(_)), Some(s)) => {
            f.style = Some(s.id.clone());
            f.components = s.components.clone();
            f.caps = s.caps;
            f.fill = s.fill.clone();
            if let Some(width) = fixed_width(&s.components) {
                f.width = width;
            }
        }
        (Some(None), _) => {
            f.style = None;
            f.components.clear();
        }
        _ => {}
    }
    if let Some(width) = c.width {
        f.width = width;
    }
    if let Some(j) = c.justify {
        f.justify = j;
    }
    if let Some(o) = c.offset {
        f.offset = o;
    }
    if let Some(caps) = c.caps {
        f.caps = caps;
    }
    if let Some(fill) = &c.fill {
        f.fill = fill.clone();
    }
    if let Some(h) = c.height {
        f.height = h;
    }
    if let Some(l) = &c.level {
        f.arch.level = l.clone();
    }
    if let Some(p) = c.phase {
        f.arch.phase = p;
    }
    if let Some(s) = c.status {
        f.arch.status = s;
    }
    if let Some(d) = c.discipline {
        f.arch.discipline = d;
    }
    walls::valid(&out.wall, "operation")?;
    walls::drawable(&out.wall, at)?;
    walls::level_known(doc, &out.wall.arch, "operation.level")?;
    Ok(out)
}

fn path_of(w: &WallFields) -> WallPath {
    WallPath::new(&walls::core_points(&w.pts), w.bulges.as_deref(), w.closed)
}

/// `w` along `p`: its points and bulges (none when every edge is straight).
fn with_path(w: &WallFields, p: &WallPath) -> WallFields {
    let mut out = w.clone();
    out.pts = p.pts.iter().map(|q| Vec2 { x: q.x, y: q.y }).collect();
    out.bulges = p.curved().then(|| p.bulges.clone());
    out.closed = p.closed;
    out
}

/// The wall the other way round with its body where it was: its path reversed, its justification and offset to the
/// other side, its components in the other order, its ends' joins swapped.
fn turned(w: &WallFields) -> WallFields {
    let mut out = with_path(w, &path_of(w).reversed());
    out.justify = match w.justify {
        WallJustify::Left => WallJustify::Right,
        WallJustify::Center => WallJustify::Center,
        WallJustify::Right => WallJustify::Left,
    };
    out.offset = -w.offset;
    out.components.reverse();
    out.joins = WallJoins {
        start: w.joins.end,
        end: w.joins.start,
    };
    out
}

/// Whether two walls are of one kind: all their fields but their paths and joins.
fn same_kind(a: &WallFields, b: &WallFields) -> bool {
    let strip = |w: &WallFields| {
        let mut x = w.clone();
        x.pts.clear();
        x.bulges = None;
        x.closed = false;
        x.joins = WallJoins::default();
        x
    };
    strip(a) == strip(b)
}

fn close(a: Vec2, b: Vec2) -> bool {
    (a.x - b.x).hypot(a.y - b.y) <= JOIN_TOLERANCE
}

/// `a` with `b` run on from its end (`b` starting where `a` ends).
fn appended(a: &WallFields, b: &WallFields) -> WallFields {
    let pa = path_of(a);
    let pb = path_of(b);
    let mut pts = pa.pts.clone();
    let mut bulges = pa.bulges.clone();
    if let Some(last) = bulges.last_mut() {
        *last = pb.bulges[0];
    }
    pts.extend_from_slice(&pb.pts[1..]);
    bulges.extend_from_slice(&pb.bulges[1..]);
    let mut out = with_path(a, &WallPath::new(&pts, Some(&bulges), false));
    out.joins.end = b.joins.end;
    out
}

/// The walls joined into one: chained from the first, each found by an end meeting the chain's (turned when the
/// other way round), of the first's kind; the chain closed when its ends meet.
fn joined(found: &[(usize, Slot, &WallEntity)]) -> Result<WallFields, Stop> {
    let mut chain = found[0].2.wall.clone();
    if chain.closed {
        return Err(fail(
            codes::WALL_CLOSED,
            "Kapalı duvar başka duvarla birleştirilmez.".into(),
            "uids[0]",
        ));
    }
    let mut left: Vec<(usize, &WallFields)> =
        found[1..].iter().map(|(at, _, w)| (*at, &w.wall)).collect();
    while !left.is_empty() {
        let (start, end) = (chain.pts[0], chain.pts[chain.pts.len() - 1]);
        let mut took = None;
        for (k, (at, w)) in left.iter().enumerate() {
            if w.closed {
                return Err(fail(
                    codes::WALL_CLOSED,
                    "Kapalı duvar başka duvarla birleştirilmez.".into(),
                    &format!("uids[{at}]"),
                ));
            }
            let (ws, we) = (w.pts[0], w.pts[w.pts.len() - 1]);
            let next = if close(ws, end) {
                Some((false, (*w).clone()))
            } else if close(we, end) {
                Some((false, turned(w)))
            } else if close(we, start) {
                Some((true, (*w).clone()))
            } else if close(ws, start) {
                Some((true, turned(w)))
            } else {
                None
            };
            if let Some((before, w)) = next {
                if !same_kind(&chain, &w) {
                    return Err(fail(
                        codes::WALLS_DIFFER,
                        "Duvarların genişliği, hizası, ötelemesi, stili, bileşenleri ya da diğer alanları farklı; tek duvar olmaz. Önce Stili değiştir ya da Değiştir ile eşitleyin.".into(),
                        &format!("uids[{at}]"),
                    ));
                }
                took = Some((k, before, w));
                break;
            }
        }
        let Some((k, before, w)) = took else {
            let at = left[0].0;
            return Err(fail(
                codes::WALLS_NOT_TOUCHING,
                "Duvarların uçları buluşmuyor: birleştirilecek duvarlar uç uca olmalı (1 mm içinde).".into(),
                &format!("uids[{at}]"),
            ));
        };
        chain = if before {
            appended(&w, &chain)
        } else {
            appended(&chain, &w)
        };
        left.remove(k);
    }
    let n = chain.pts.len();
    if n > 2 && close(chain.pts[0], chain.pts[n - 1]) {
        let p = path_of(&chain);
        let ring = WallPath::new(&p.pts[..n - 1], Some(&p.bulges[..n - 1]), true);
        let mut b = ring.bulges.clone();
        b[n - 2] = p.bulges[n - 2];
        chain = with_path(&chain, &WallPath::new(&ring.pts, Some(&b), true));
        chain.joins = WallJoins::default();
    }
    Ok(chain)
}

/// A wall named by `uid` at `path`, and its slot.
fn wall_at<'a>(doc: &'a Document, uid: &str, path: &str) -> Result<(Slot, &'a WallEntity), Stop> {
    let parsed = Uuid::parse_str(uid).map_err(|_| {
        fail(
            codes::INVALID_UID,
            format!("“{uid}” bir kalıcı kimlik (UUID) değil."),
            path,
        )
    })?;
    let slot = doc.slot_of(parsed).ok_or_else(|| {
        fail(
            codes::ENTITY_NOT_FOUND,
            format!("Çizimde “{uid}” kimlikli nesne yok."),
            path,
        )
    })?;
    match doc.get(slot) {
        Some(Entity::Wall(w)) => Ok((slot, w)),
        _ => Err(fail(
            codes::NOT_A_WALL,
            "Nesne duvar değil; duvarlar yalnız duvarlara uzatılır ve budanır.".into(),
            path,
        )),
    }
}

fn style_matches(doc: &Document, w: &WallFields, key: &str) -> bool {
    let Some(id) = &w.style else { return false };
    if id == key {
        return true;
    }
    walls::find_style(doc, key).is_some_and(|(s, _)| &s.id == id)
}

fn check(doc: &Document, input: &WallsEdit) -> Result<Work, Stop> {
    // Stili değiştir without ids: every wall of the style.
    let all_of_style =
        matches!(&input.operation, WallsOperation::ReplaceStyle { .. }) && input.uids.is_empty();
    let uids: Vec<String> = if all_of_style {
        let WallsOperation::ReplaceStyle { from, .. } = &input.operation else {
            unreachable!()
        };
        doc.entities()
            .filter(|e| matches!(e, Entity::Wall(w) if style_matches(doc, &w.wall, from)))
            .filter_map(|e| doc.uid(Slot(e.base().id)).map(|u| u.to_string()))
            .collect()
    } else {
        checks::uids(&input.uids, "Düzenlenecek duvar verilmedi.")?;
        input.uids.clone()
    };
    let found = checks::named(doc, &uids)?;
    for (at, _, e, _) in &found {
        if !matches!(e, Entity::Wall(_)) {
            return Err(fail(
                codes::NOT_A_WALL,
                "Nesne duvar değil; Duvarı düzenle yalnız duvarları değiştirir.".into(),
                &format!("uids[{at}]"),
            ));
        }
    }
    let walls_found: Vec<(usize, Slot, &WallEntity)> = found
        .iter()
        .filter_map(|(at, slot, e, _)| match e {
            Entity::Wall(w) => Some((*at, *slot, w)),
            _ => None,
        })
        .collect();
    let mut work = Work::default();
    let each = |work: &mut Work,
                f: &dyn Fn(usize, &WallEntity) -> Result<WallEntity, Stop>|
     -> Result<(), Stop> {
        for (at, slot, w) in &walls_found {
            work.updates.push((*slot, f(*at, w)?));
        }
        Ok(())
    };
    let checked = |w: WallEntity, at: usize| -> Result<WallEntity, Stop> {
        walls::valid(&w.wall, &format!("uids[{at}]"))?;
        walls::drawable(&w.wall, &format!("uids[{at}]"))?;
        Ok(w)
    };
    match &input.operation {
        WallsOperation::Set(c) => {
            if c == &WallChange::default() {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    "Değiştirilecek bir alan verilmedi: genişlik, hiza, öteleme, stil, uçlar, dolgu, yükseklik, kat, evre, durum ya da disiplin verin.".into(),
                    "operation",
                ));
            }
            for (v, what, name) in [
                (c.width, "width", "Genişlik"),
                (c.offset, "offset", "Öteleme"),
                (c.height.flatten(), "height", "Yükseklik"),
            ] {
                if let Some(x) = v {
                    checks::finite(
                        x,
                        name,
                        "Sonlu bir sayı verin.",
                        &format!("operation.{what}"),
                    )?;
                }
            }
            let style = match &c.style {
                Some(Some(key)) => {
                    let (s, builtin) = walls::find_style(doc, key)
                        .ok_or_else(|| walls::no_style(doc, key, "operation.style"))?;
                    if builtin {
                        work.styles.push(s.clone());
                    }
                    Some(s)
                }
                _ => None,
            };
            each(&mut work, &|at, w| {
                changed(doc, w, c, style.as_ref(), &format!("uids[{at}]"))
            })?;
        }
        WallsOperation::Split { at: point } => {
            checks::finite(
                point.x,
                "Bölme noktası",
                "Sonlu bir sayı verin.",
                "operation.at.x",
            )?;
            checks::finite(
                point.y,
                "Bölme noktası",
                "Sonlu bir sayı verin.",
                "operation.at.y",
            )?;
            for (at, slot, w) in &walls_found {
                let p = path_of(&w.wall);
                let (a, b, across) = wall_edit::split(&p, CoreVec2::new(point.x, point.y)).map_err(|SplitRefusal::AtEnd| {
                    fail(
                        codes::SPLIT_AT_END,
                        "Duvar ucundan bölünmez: bölme noktası duvarın iki ucunun arasında olmalı.".into(),
                        "operation.at",
                    )
                })?;
                let lf = kentos_geometry_core::geom::wall::left_face(&walls::core_spec(&w.wall));
                if across > lf + JOIN_TOLERANCE || across < lf - w.wall.width - JOIN_TOLERANCE {
                    return Err(fail(
                        codes::POINT_OFF_WALL,
                        format!(
                            "Nokta duvarın üzerinde değil: duvarın yolundan {:.3} m uzakta, iki yüzünün arasında olmalı.",
                            across.abs()
                        ),
                        "operation.at",
                    ));
                }
                let mut first = (*w).clone();
                first.wall = with_path(&w.wall, &a);
                match b {
                    Some(b) => {
                        first.wall.joins.end = WallJoin::Auto;
                        let mut second = (*w).clone();
                        second.wall = with_path(&w.wall, &b);
                        second.wall.joins.start = WallJoin::Auto;
                        // Its slot is the document's to give (0 in a plan).
                        second.base.id = 0;
                        second.base.label = None;
                        second.base.label_pins.clear();
                        second.base.attachments.clear();
                        work.added.push(checked(second, *at)?);
                        work.origins.push(*slot);
                    }
                    None => first.wall.joins = WallJoins::default(),
                }
                work.updates.push((*slot, checked(first, *at)?));
            }
        }
        WallsOperation::Join => {
            if walls_found.len() < 2 {
                return Err(fail(
                    codes::TOO_FEW_OBJECTS,
                    "Birleştirmek için uç uca iki ya da daha çok duvar verin.".into(),
                    "uids",
                ));
            }
            let one = joined(&walls_found)?;
            let (_, slot, first) = walls_found[0];
            let mut w = first.clone();
            w.wall = one;
            work.updates.push((slot, checked(w, 0)?));
            work.removed = walls_found[1..].iter().map(|(_, s, _)| *s).collect();
        }
        WallsOperation::Extend { to, end } => {
            let (target_slot, target) = wall_at(doc, to, "operation.to")?;
            let edges = wall_edit::edges(&path_of(&target.wall));
            for (at, slot, w) in &walls_found {
                if *slot == target_slot {
                    continue;
                }
                if w.wall.closed {
                    return Err(fail(
                        codes::WALL_CLOSED,
                        "Kapalı duvarın ucu yok; uzatılmaz.".into(),
                        &format!("uids[{at}]"),
                    ));
                }
                let p = path_of(&w.wall);
                let at_end = match end {
                    Some(e) => *e == WallEnd::End,
                    None => {
                        let tp = target
                            .wall
                            .pts
                            .iter()
                            .map(|q| CoreVec2::new(q.x, q.y))
                            .collect::<Vec<_>>();
                        let tpath =
                            WallPath::new(&tp, target.wall.bulges.as_deref(), target.wall.closed);
                        let d = |q: CoreVec2| {
                            tpath.path().map_or(f64::INFINITY, |tp| {
                                let s = kentos_geometry_core::ops::path::nearest_s(&tp, q);
                                let r = kentos_geometry_core::ops::path::point_at_s(&tp, s);
                                (r.x - q.x).hypot(r.y - q.y)
                            })
                        };
                        d(p.pts[p.pts.len() - 1]) <= d(p.pts[0])
                    }
                };
                let Some(moved) = wall_edit::extend(&p, at_end, &edges) else {
                    return Err(fail(
                        codes::WALL_MISSES,
                        "Duvar uzatınca diğer duvarın yoluna varmıyor: ucu ona doğru bakmalı."
                            .into(),
                        &format!("uids[{at}]"),
                    ));
                };
                let mut out = (*w).clone();
                out.wall = with_path(&w.wall, &moved);
                work.updates.push((*slot, checked(out, *at)?));
            }
        }
        WallsOperation::Trim { to, keep } => {
            let (target_slot, target) = wall_at(doc, to, "operation.to")?;
            let edges = wall_edit::edges(&path_of(&target.wall));
            let keep = keep.map(|k| CoreVec2::new(k.x, k.y));
            for (at, slot, w) in &walls_found {
                if *slot == target_slot {
                    continue;
                }
                if w.wall.closed {
                    return Err(fail(
                        codes::WALL_CLOSED,
                        "Kapalı duvarın ucu yok; budanmaz.".into(),
                        &format!("uids[{at}]"),
                    ));
                }
                let Some(cut) = wall_edit::trim(&path_of(&w.wall), &edges, keep) else {
                    return Err(fail(
                        codes::WALL_MISSES,
                        "Duvarın yolu diğer duvarın yolunu kesmiyor; budanacak yer yok.".into(),
                        &format!("uids[{at}]"),
                    ));
                };
                let mut out = (*w).clone();
                out.wall = with_path(&w.wall, &cut);
                work.updates.push((*slot, checked(out, *at)?));
            }
        }
        WallsOperation::Flip => {
            each(&mut work, &|at, w| {
                let mut out = w.clone();
                out.wall = with_path(&w.wall, &path_of(&w.wall).reversed());
                out.wall.joins = WallJoins {
                    start: w.wall.joins.end,
                    end: w.wall.joins.start,
                };
                checked(out, at)
            })?;
        }
        WallsOperation::ReplaceStyle { from, to } => {
            let (s, builtin) = walls::find_style(doc, to)
                .ok_or_else(|| walls::no_style(doc, to, "operation.to"))?;
            let of_style: Vec<&(usize, Slot, &WallEntity)> = walls_found
                .iter()
                .filter(|(_, _, w)| style_matches(doc, &w.wall, from))
                .collect();
            if of_style.is_empty() {
                return Err(fail(
                    codes::STYLE_NOT_USED,
                    format!("Verilen duvarların hiçbiri “{from}” stilinde değil."),
                    "operation.from",
                ));
            }
            if builtin {
                work.styles.push(s.clone());
            }
            let c = WallChange {
                style: Some(Some(to.clone())),
                ..WallChange::default()
            };
            for (at, slot, w) in of_style {
                work.updates.push((
                    *slot,
                    changed(doc, w, &c, Some(&s), &format!("uids[{at}]"))?,
                ));
            }
        }
        WallsOperation::Joins {
            start,
            end,
            cleanup,
        } => {
            if start.is_none() && end.is_none() && cleanup.is_none() {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    "Değiştirilecek bir alan verilmedi: başlangıcın ya da sonun birleşimini, ya da temizliği verin.".into(),
                    "operation",
                ));
            }
            each(&mut work, &|at, w| {
                if w.wall.closed && (start.is_some() || end.is_some()) {
                    return Err(fail(
                        codes::WALL_CLOSED,
                        "Kapalı duvarın ucu yok; uç birleşimi verilmez, yalnız temizlik.".into(),
                        &format!("uids[{at}]"),
                    ));
                }
                let mut out = w.clone();
                if let Some(m) = start {
                    out.wall.joins.start = *m;
                }
                if let Some(m) = end {
                    out.wall.joins.end = *m;
                }
                if let Some(c) = cleanup {
                    out.wall.cleanup = *c;
                }
                Ok(out)
            })?;
        }
        WallsOperation::ResetJoins => {
            each(&mut work, &|_, w| {
                let mut out = w.clone();
                out.wall.joins = WallJoins::default();
                out.wall.cleanup = true;
                Ok(out)
            })?;
        }
        WallsOperation::Shift { edge, by } => {
            checks::finite(*by, "Kaydırma", "Sonlu bir uzunluk verin.", "operation.by")?;
            shifted(doc, &walls_found, *edge, None, *by, false, &mut work)?;
        }
        WallsOperation::Distance {
            edge,
            side,
            distance,
            to,
            keep,
        } => {
            checks::finite(
                *distance,
                "Mesafe",
                "Sonlu bir uzunluk verin.",
                "operation.distance",
            )?;
            if *distance < 0.0 {
                return Err(fail(
                    codes::INVALID_WALL,
                    format!("Mesafe {distance}; sıfır ya da daha büyük olmalı."),
                    "operation.distance",
                ));
            }
            shifted(
                doc,
                &walls_found,
                *edge,
                Some((*side, *distance, *to)),
                0.0,
                *keep,
                &mut work,
            )?;
            if *keep {
                work.keep = Some(kept(doc, &walls_found, *edge, *side, *to)?);
            }
        }
        WallsOperation::Length {
            edge,
            length,
            anchor,
        } => {
            checks::finite(
                *length,
                "Boy",
                "Sonlu bir uzunluk verin.",
                "operation.length",
            )?;
            lengthened(doc, &walls_found, *edge, *length, *anchor, &mut work)?;
        }
    }
    // The openings go with the walls whose paths changed (docs/adr/0341 §3).
    if matches!(
        input.operation,
        WallsOperation::Split { .. }
            | WallsOperation::Join
            | WallsOperation::Extend { .. }
            | WallsOperation::Trim { .. }
            | WallsOperation::Flip
            | WallsOperation::Shift { .. }
            | WallsOperation::Distance { .. }
            | WallsOperation::Length { .. }
    ) {
        work.openings = carry(doc, &work);
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    // All or none: a wall on a locked layer refuses the whole change.
    for (at, _, e, _) in &found {
        let layer = &e.base().layer_id;
        if doc.layers().is_locked(layer) {
            let name = doc
                .layers()
                .get(layer)
                .map_or(layer.as_str(), |n| n.name.as_str());
            return Err(fail(
                codes::LAYER_LOCKED,
                format!(
                    "“{name}” katmanı kilitli; üzerindeki duvar düzenlenemez. Kilidi Katmanlar panelinden açın."
                ),
                &format!("uids[{at}]"),
            ));
        }
    }
    if let Some(name) = &work.blocked {
        let what = if matches!(input.operation, WallsOperation::Length { .. }) {
            "Boyun değişmesi"
        } else {
            "Kaydırma"
        };
        return Err(fail(
            codes::LAYER_LOCKED,
            format!(
                "{what} “{name}” katmanındaki bağlı bir duvarı da değiştirir ve o katman kilitli. Kilidi Katmanlar panelinden açın."
            ),
            "operation",
        ));
    }
    Ok(work)
}

/// Kaydır and Mesafe (docs/adr/0710): the one wall's edge moved `by`, or by as much as makes the gap on `target`'s side
/// its distance (to its face, or from its axis to a grid, docs/adr/0728 §1); the wall first in `work.updates`, the
/// walls that followed after it.
fn shifted(
    doc: &Document,
    found: &[(usize, Slot, &WallEntity)],
    edge: Option<u32>,
    target: Option<(
        kentos_commands::DimensionSetFace,
        f64,
        kentos_commands::DistanceTo,
    )>,
    by: f64,
    keep: bool,
    work: &mut Work,
) -> Result<(), Stop> {
    use kentos_geometry_core::ops::wall_shift;
    let [(at, slot, wall)] = found else {
        return Err(fail(
            codes::INVALID_WALL,
            "Kaydır ve Mesafe tek duvarla çalışır; bir duvar verin.".into(),
            "uids",
        ));
    };
    let walls = crate::wall_distances::Walls::of(doc, *slot, wall);
    let ins = walls.ins();
    let k = edge.unwrap_or(0) as usize;
    let gaps = wall_shift::gaps(&ins, 0, k)
        .map_err(|why| fail(codes::INVALID_WALL, why, "operation.edge"))?;
    let by = match target {
        None => by,
        Some((side, distance, to)) => {
            let left = side == kentos_commands::DimensionSetFace::Left;
            let gap = match to {
                kentos_commands::DistanceTo::Face => gaps[usize::from(!left)].map(|g| g.distance),
                kentos_commands::DistanceTo::Grid => {
                    let grids = crate::grid_refs::Grids::of(doc);
                    let (at, u) = wall_shift::axis_middle(&ins, 0, k)
                        .map_err(|why| fail(codes::INVALID_WALL, why, "operation.edge"))?;
                    kentos_geometry_core::ops::grid_ref::beside(
                        &grids.ins(),
                        wall_shift::level(&ins[0]),
                        at,
                        u,
                    )[usize::from(!left)]
                    .map(|g| g.distance)
                }
            };
            let Some(gap) = gap else {
                let what = match to {
                    kentos_commands::DistanceTo::Face => "duvar",
                    kentos_commands::DistanceTo::Grid => "aks",
                };
                return Err(fail(
                    codes::INVALID_WALL,
                    format!(
                        "Duvarın {} yanında ona paralel bir {what} yok; mesafe ölçülemiyor. Kaydır ile uzaklık verin.",
                        if left { "sol" } else { "sağ" }
                    ),
                    if to == kentos_commands::DistanceTo::Grid {
                        "operation.to"
                    } else {
                        "operation.side"
                    },
                ));
            };
            if left { gap - distance } else { distance - gap }
        }
    };
    if by.abs() <= wall_shift::TIE {
        // The distance kept as it is (docs/adr/0770 §3): only the bond is written.
        if keep {
            work.edge = Some(k);
            work.named = Some(0);
            return Ok(());
        }
        return Err(fail(
            codes::NOTHING_TO_SET,
            "Duvar zaten bu konumda; kaydırılacak bir şey yok.".into(),
            "operation",
        ));
    }
    let moved = wall_shift::shift(&ins, 0, k, by)
        .map_err(|why| fail(codes::INVALID_WALL, why, "operation"))?;
    work.edge = Some(k);
    for m in moved {
        let s = walls.slots[m.wall];
        let Some(Entity::Wall(w)) = doc.get(s) else {
            continue;
        };
        if m.wall != 0 && work.blocked.is_none() && doc.layers().is_locked(&w.base.layer_id) {
            let id = &w.base.layer_id;
            work.blocked = Some(
                doc.layers()
                    .get(id)
                    .map_or(id.as_str(), |n| n.name.as_str())
                    .to_owned(),
            );
        }
        let mut out = w.clone();
        out.wall.pts = m.pts.iter().map(|p| Vec2 { x: p.x, y: p.y }).collect();
        let path = format!("uids[{at}]");
        walls::valid(&out.wall, &path)?;
        walls::drawable(&out.wall, &path)?;
        work.updates.push((s, out));
    }
    work.named = Some(1);
    Ok(())
}

/// Boy (docs/adr/0728 §3): the one wall's edge made `length` long, `anchor` staying; the wall first in `work.updates`,
/// the walls that followed after it.
fn lengthened(
    doc: &Document,
    found: &[(usize, Slot, &WallEntity)],
    edge: Option<u32>,
    length: f64,
    anchor: kentos_commands::WallAnchor,
    work: &mut Work,
) -> Result<(), Stop> {
    use kentos_geometry_core::ops::wall_shift;
    let [(at, slot, wall)] = found else {
        return Err(fail(
            codes::INVALID_WALL,
            "Boy tek duvarla çalışır; bir duvar verin.".into(),
            "uids",
        ));
    };
    let walls = crate::wall_distances::Walls::of(doc, *slot, wall);
    let ins = walls.ins();
    let k = edge.unwrap_or(0) as usize;
    let now = wall_shift::edge_length(&ins, 0, k)
        .map_err(|why| fail(codes::INVALID_WALL, why, "operation.edge"))?;
    if (length - now).abs() <= wall_shift::TIE {
        return Err(fail(
            codes::NOTHING_TO_SET,
            "Kenar zaten bu boyda; değişecek bir şey yok.".into(),
            "operation.length",
        ));
    }
    let anchor = match anchor {
        kentos_commands::WallAnchor::Start => wall_shift::Anchor::Start,
        kentos_commands::WallAnchor::End => wall_shift::Anchor::End,
        kentos_commands::WallAnchor::Middle => wall_shift::Anchor::Middle,
    };
    let moved = wall_shift::lengthen(&ins, 0, k, length, anchor).map_err(|why| {
        let path = if why.starts_with("Boy ") {
            "operation.length"
        } else {
            "operation"
        };
        fail(codes::INVALID_WALL, why, path)
    })?;
    for m in moved {
        let s = walls.slots[m.wall];
        let Some(Entity::Wall(w)) = doc.get(s) else {
            continue;
        };
        if m.wall != 0 && work.blocked.is_none() && doc.layers().is_locked(&w.base.layer_id) {
            let id = &w.base.layer_id;
            work.blocked = Some(
                doc.layers()
                    .get(id)
                    .map_or(id.as_str(), |n| n.name.as_str())
                    .to_owned(),
            );
        }
        let mut out = w.clone();
        out.wall.pts = m.pts.iter().map(|p| Vec2 { x: p.x, y: p.y }).collect();
        let path = format!("uids[{at}]");
        walls::valid(&out.wall, &path)?;
        walls::drawable(&out.wall, &path)?;
        work.updates.push((s, out));
    }
    work.named = Some(1);
    work.edge = Some(k);
    Ok(())
}

fn warnings(doc: &Document, work: &Work) -> Vec<kentos_commands::CommandWarning> {
    let changed: Vec<(Option<Slot>, &WallEntity)> = work
        .updates
        .iter()
        .map(|(s, w)| (Some(*s), w))
        .chain(work.added.iter().map(|w| (None, w)))
        .collect();
    walls::join_warnings(doc, &changed, &work.removed)
}

fn removed_uids(doc: &Document, work: &Work) -> Vec<String> {
    work.removed
        .iter()
        .filter_map(|s| doc.uid(*s).map(|u| u.to_string()))
        .collect()
}

pub fn validate(cx: &ExecutionContext<'_>, input: &WallsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &WallsEdit) -> CommandResult<WallsEdited> {
    match check(cx.doc, input) {
        Ok(work) => {
            let warnings = warnings(cx.doc, &work);
            // The openings the walls carry with their new places (docs/adr/0341 §3), so that the preview of what
            // follows is the step's (an opening whose wall's start moved keeps its place).
            let updates: Vec<_> = work
                .updates
                .iter()
                .map(|(s, w)| (*s, Entity::Wall(w.clone())))
                .chain(work.openings.iter().filter_map(|(s, o, c)| match c {
                    Carrier::Kept(_) => Some((*s, Entity::Opening(o.clone()))),
                    Carrier::Added(_) => None,
                }))
                .collect();
            let adds: Vec<_> = work.added.iter().cloned().map(Entity::Wall).collect();
            let followers = crate::dimension_sets::derived(cx.doc, &updates, &adds, &work.removed);
            let effects = match effects(cx.doc, &work) {
                Ok(e) => e,
                Err(stop) => return stop.into(),
            };
            let named = work.named.unwrap_or(work.updates.len());
            CommandResult::Completed {
                output: WallsEdited {
                    followers,
                    walls: work.updates[..named]
                        .iter()
                        .map(|(_, w)| Entity::Wall(w.clone()))
                        .collect(),
                    joined: work.updates[named..]
                        .iter()
                        .map(|(_, w)| Entity::Wall(w.clone()))
                        .collect(),
                    added: work.added.iter().cloned().map(Entity::Wall).collect(),
                    removed: removed_uids(cx.doc, &work),
                    styles_added: work.styles.iter().map(|s| s.id.clone()).collect(),
                    effects,
                    bonds: Vec::new(),
                    revision: cx.doc.revision().to_string(),
                },
                warnings,
            }
        }
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: WallsEdit) -> CommandResult<WallsEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_WALLS_EDIT,
        kentos_commands::CAD_WALLS_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: WallsEdit) -> CommandResult<WallsEdited> {
    let work = match check(cx.doc, &input) {
        Ok(work) => work,
        Err(stop) => return stop.into(),
    };
    let warnings = warnings(cx.doc, &work);
    let removed = removed_uids(cx.doc, &work);
    let effects = match effects(cx.doc, &work) {
        Ok(e) => e,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    let mut entities: Vec<Entity> = work
        .updates
        .iter()
        .map(|(_, w)| Entity::Wall(w.clone()))
        .collect();
    let joined = entities.split_off(work.named.unwrap_or(entities.len()));
    let result = doc.transact(STEP, |d| {
        walls::take_styles(d, &work.styles);
        d.update_many(
            work.updates
                .iter()
                .map(|(s, w)| (*s, Entity::Wall(w.clone())))
                .collect(),
            STEP,
        );
        let slots = d.add_many(work.added.iter().cloned().map(Entity::Wall).collect(), STEP)?;
        // The openings into their walls, an added one's by the id it was just given.
        let carried: Vec<(Slot, Entity)> = work
            .openings
            .iter()
            .filter_map(|(s, o, c)| {
                let wall = match c {
                    Carrier::Kept(w) => *w,
                    Carrier::Added(k) => *slots.get(*k)?,
                };
                let mut o = o.clone();
                o.opening.host = d.uid(wall)?.to_string();
                Some((*s, Entity::Opening(o)))
            })
            .collect();
        if !carried.is_empty() {
            d.update_many(carried, STEP);
        }
        d.remove(&work.removed);
        Ok::<_, kentos_domain::SlotsExhausted>(slots)
    });
    // Bu ölçüyü koru: the bond and its parameter in the same step (docs/adr/0770 §5).
    let mut bonds = Vec::new();
    if result.is_ok()
        && let Some(rule) = &work.keep
    {
        match crate::constraints_edit::add_bond(doc, rule.clone(), STEP) {
            Ok(id) => bonds.push(id),
            Err(stop) => return stop.into(),
        }
    }
    let slots = match result {
        Ok(slots) => slots,
        Err(_) => {
            return CommandResult::Failed {
                error: error(
                    codes::SLOTS_EXHAUSTED,
                    "Çizimde yeni nesneye yer kalmadı; çizimi kaydedip yeniden açın.".into(),
                    None,
                ),
            };
        }
    };
    let added = slots.iter().filter_map(|s| doc.get(*s).cloned()).collect();
    CommandResult::Completed {
        output: WallsEdited {
            followers: Default::default(),
            walls: entities,
            joined,
            added,
            removed,
            styles_added: work.styles.into_iter().map(|s| s.id).collect(),
            effects,
            bonds,
            revision: doc.revision().to_string(),
        },
        warnings,
    }
}

/// Mesafe's `keep` (docs/adr/0770 §5): the distance bond from the wall across on `side` (its face towards this one,
/// `a`) to this wall's face on that side (`b`); faces only (a grid is not a wall).
fn kept(
    doc: &Document,
    found: &[(usize, Slot, &WallEntity)],
    edge: Option<u32>,
    side: kentos_commands::DimensionSetFace,
    to: kentos_commands::DistanceTo,
) -> Result<kentos_commands::ConstraintRule, Stop> {
    use kentos_commands::{
        CadElement, ConstraintRule, DimensionSetFace, ElementRef, EntityId, WallLine,
    };
    use kentos_geometry_core::ops::wall_shift;
    if to == kentos_commands::DistanceTo::Grid {
        return Err(fail(
            codes::INVALID_WALL,
            "Bu ölçüyü koru yalnız duvar yüzüne olan mesafede olur; aks bağlanmaz.".into(),
            "operation.keep",
        ));
    }
    let [(_, slot, wall)] = found else {
        return Err(fail(codes::INVALID_WALL, "Tek duvar verin.".into(), "uids"));
    };
    let walls = crate::wall_distances::Walls::of(doc, *slot, wall);
    let ins = walls.ins();
    let k = edge.unwrap_or(0) as usize;
    let gaps = wall_shift::gaps(&ins, 0, k)
        .map_err(|why| fail(codes::INVALID_WALL, why, "operation.edge"))?;
    let left = side == DimensionSetFace::Left;
    let Some(g) = gaps[usize::from(!left)] else {
        return Err(fail(
            codes::INVALID_WALL,
            "Bu yanda paralel bir duvar yok; bağlanacak yüz bulunamadı.".into(),
            "operation.side",
        ));
    };
    let uid = |s: Slot| doc.uid(s).map(|u| EntityId(u.into_bytes()));
    let (Some(across), Some(me)) = (uid(walls.slots[g.wall]), uid(*slot)) else {
        return Err(fail(
            codes::INVALID_WALL,
            "Duvarın kimliği yok.".into(),
            "uids",
        ));
    };
    let a = ElementRef {
        uid: across,
        element: CadElement::Wall {
            edge: g.edge as u32,
            line: if g.left_face {
                WallLine::LeftFace
            } else {
                WallLine::RightFace
            },
        },
    };
    let b = ElementRef {
        uid: me,
        element: CadElement::Wall {
            edge: k as u32,
            line: if left {
                WallLine::LeftFace
            } else {
                WallLine::RightFace
            },
        },
    };
    Ok(ConstraintRule::Distance {
        a,
        b,
        axis: kentos_commands::DistanceAxis::Aligned,
        left: false,
        parameter: String::new(),
    })
}

/// What a Kaydır, Mesafe or Boy does round it (docs/adr/0770 §3): the step run whole on a copy of the drawing (its
/// bonds, openings, spaces and dimensions following) and read against the drawing. The bonds' refusal when they
/// cannot follow. None for the other operations.
fn effects(doc: &Document, work: &Work) -> Result<Option<kentos_commands::WallEffects>, Stop> {
    use kentos_commands::{GapChange, SpaceAreaChange, WallEffects};
    use kentos_geometry_core::ops::wall_shift;
    let (Some(k), Some((first, _))) = (work.edge, work.updates.first()) else {
        return Ok(None);
    };
    let gaps = |d: &Document| -> [Option<f64>; 2] {
        let Some(Entity::Wall(w)) = d.get(*first) else {
            return [None, None];
        };
        let walls = crate::wall_distances::Walls::of(d, *first, w);
        match wall_shift::gaps(&walls.ins(), 0, k) {
            Ok(g) => [g[0].map(|g| g.distance), g[1].map(|g| g.distance)],
            Err(_) => [None, None],
        }
    };
    let before = gaps(doc);
    let mut copy = doc.reading_copy();
    let written: Result<(), Stop> = copy.transact(STEP, |d| {
        d.update_many(
            work.updates
                .iter()
                .map(|(s, w)| (*s, Entity::Wall(w.clone())))
                .chain(work.openings.iter().filter_map(|(s, o, c)| match c {
                    Carrier::Kept(_) => Some((*s, Entity::Opening(o.clone()))),
                    Carrier::Added(_) => None,
                }))
                .collect(),
            STEP,
        );
        crate::constraint_follow::follow(d, kentos_commands::CAD_WALLS_EDIT, false, &|| false)
            .map(|_| ())
    });
    written?;
    let after = gaps(&copy);
    let mut out = WallEffects {
        left: before[0].zip(after[0]).map(|(b, a)| GapChange {
            before: b,
            after: a,
        }),
        right: before[1].zip(after[1]).map(|(b, a)| GapChange {
            before: b,
            after: a,
        }),
        ..WallEffects::default()
    };
    let named: Vec<Slot> = work.updates.iter().map(|(s, _)| *s).collect();
    for e in doc.entities() {
        let slot = Slot(e.base().id);
        let Some(now) = copy.get(slot) else { continue };
        if std::ptr::eq(e, now) || !differs(e, now) {
            continue;
        }
        let Some(uid) = doc.uid(slot).map(|u| u.to_string()) else {
            continue;
        };
        match (e, now) {
            (Entity::Space(b), Entity::Space(a)) => out.spaces.push(SpaceAreaChange {
                uid,
                number: b.space.number.clone().unwrap_or_default(),
                name: b.space.name.clone().unwrap_or_default(),
                before: crate::spaces::area_of(&b.space.parts),
                after: crate::spaces::area_of(&a.space.parts),
                broken: a.space.broken.is_some(),
            }),
            (Entity::Opening(_), _) => out.openings.push(uid),
            (Entity::Dimension(_), _) => out.dimensions.push(uid),
            (Entity::Insert(_), _) => out.fixtures.push(uid),
            (Entity::Wall(_), _) if !named.contains(&slot) => out.bonded.push(uid),
            _ => {}
        }
    }
    Ok(Some(out))
}

/// Whether two objects differ by more than a nanometre in a number, or at all in anything else (docs/adr/0770 §3: a
/// follower written again where it was is not an effect).
fn differs(a: &Entity, b: &Entity) -> bool {
    fn apart(x: &serde_json::Value, y: &serde_json::Value) -> bool {
        use serde_json::Value as V;
        match (x, y) {
            (V::Number(p), V::Number(q)) => match (p.as_f64(), q.as_f64()) {
                (Some(p), Some(q)) => (p - q).abs() > 1e-9,
                _ => p != q,
            },
            (V::Array(p), V::Array(q)) => {
                p.len() != q.len() || p.iter().zip(q).any(|(p, q)| apart(p, q))
            }
            (V::Object(p), V::Object(q)) => {
                p.len() != q.len() || p.iter().any(|(k, v)| q.get(k).is_none_or(|w| apart(v, w)))
            }
            _ => x != y,
        }
    }
    match (serde_json::to_value(a), serde_json::to_value(b)) {
        (Ok(x), Ok(y)) => apart(&x, &y),
        _ => true,
    }
}
