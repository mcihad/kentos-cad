//! The architectural adapter (docs/adr/0770 §4): the constraints that name a wall's line or an opening's jamb, read
//! from the drawing as the core's linear problem (`ops::arch_bonds`), solved, and written back as Kaydır writes (each
//! shifted edge moved square to itself by `ops::wall_shift`, the walls joined to it following, the openings of moved
//! walls read back from their jambs) and openings slid along their walls. `constraint_solve::solve_set` runs it beside
//! the general solver, which never sees these constraints; every command that solves goes through there.
//!
//! The shared cases are `fixtures/commands/v1/cad.constraints.edit.arch.json` (`scripts/fixtures/arch_bond_cases.py`).

use std::collections::{BTreeMap, HashMap};

use kentos_commands::{
    CadElement, ConstraintRule, ConstraintSet, ElementRef, Entity, EntityId, JambEnd,
    OpeningEntity, WallEntity, WallLine,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::Vec2 as P;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::opening;
use kentos_geometry_core::geom::wall::Spec;
use kentos_geometry_core::geom::wall_join::WallIn;
use kentos_geometry_core::ops::arch_bonds::{
    self as core, Bond, Item, Jamb, Line, LineKind, Outcome, Why,
};
use kentos_geometry_core::ops::wall_shift;

use crate::constraint_solve::Held;

/// A bond that held within this after its shifts were written, metres.
pub const SETTLED: f64 = 1e-6;

fn kind(line: WallLine) -> LineKind {
    match line {
        WallLine::LeftFace => LineKind::LeftFace,
        WallLine::RightFace => LineKind::RightFace,
        WallLine::Axis => LineKind::Axis,
        WallLine::LeftCore => LineKind::LeftCore,
        WallLine::RightCore => LineKind::RightCore,
    }
}

fn slot_of(doc: &Document, uid: EntityId) -> Option<Slot> {
    doc.slot_of(Uuid::from_bytes(uid.0))
}

/// The drawing as the adapter reads it: the document with the walls and openings it has moved so far.
pub(crate) struct View<'a> {
    doc: &'a Document,
    walls: HashMap<Slot, WallEntity>,
    openings: HashMap<Slot, OpeningEntity>,
}

/// A wall's path and fields as the core reads them.
struct CoreWall {
    pts: Vec<P>,
    bulges: Option<Vec<f64>>,
    spec: Spec,
}

impl CoreWall {
    fn of(w: &WallEntity) -> Option<CoreWall> {
        match crate::walls::core_shape(w) {
            Shape::Wall { pts, bulges, wall } => Some(CoreWall {
                pts,
                bulges,
                spec: wall,
            }),
            _ => None,
        }
    }

    fn input(&self) -> WallIn<'_> {
        WallIn {
            pts: &self.pts,
            bulges: self.bulges.as_deref(),
            spec: &self.spec,
        }
    }

    /// Straight edge `k`'s two points.
    fn edge(&self, k: usize) -> Option<(P, P)> {
        let w = self.input();
        if k >= wall_shift::edge_count(&w)
            || self
                .bulges
                .as_ref()
                .and_then(|b| b.get(k))
                .copied()
                .unwrap_or(0.0)
                != 0.0
        {
            return None;
        }
        let (a, b) = (self.pts[k], self.pts[(k + 1) % self.pts.len()]);
        ((b.x - a.x).hypot(b.y - a.y) > 0.0).then_some((a, b))
    }
}

fn unit(a: P, b: P) -> P {
    let l = (b.x - a.x).hypot(b.y - a.y);
    P::new((b.x - a.x) / l, (b.y - a.y) / l)
}

impl<'a> View<'a> {
    pub(crate) fn new(doc: &'a Document) -> View<'a> {
        View {
            doc,
            walls: HashMap::new(),
            openings: HashMap::new(),
        }
    }

    fn wall(&self, slot: Slot) -> Option<WallEntity> {
        if let Some(w) = self.walls.get(&slot) {
            return Some(w.clone());
        }
        match self.doc.get(slot)? {
            Entity::Wall(w) => Some(w.clone()),
            _ => None,
        }
    }

    fn opening(&self, slot: Slot) -> Option<OpeningEntity> {
        if let Some(o) = self.openings.get(&slot) {
            return Some(o.clone());
        }
        match self.doc.get(slot)? {
            Entity::Opening(o) => Some(o.clone()),
            _ => None,
        }
    }

    /// A wall's line: its edge's two points moved to the line, and the edge's direction.
    fn line(&self, uid: EntityId, edge: u32, line: WallLine) -> Option<(P, P)> {
        let w = CoreWall::of(&self.wall(slot_of(self.doc, uid)?)?)?;
        let (a, b) = w.edge(edge as usize)?;
        let u = unit(a, b);
        let o = core::line_offset(&w.spec, kind(line));
        Some((P::new(a.x - u.y * o, a.y + u.x * o), u))
    }

    /// An opening's jamb: the middle of its jamb line, the direction of its wall's edge there, its wall and that
    /// edge. None when its wall is gone, its interval falls off the wall or the edge is an arc.
    fn jamb(&self, uid: EntityId, end: JambEnd) -> Option<(P, P, Slot, usize)> {
        let o = self.opening(slot_of(self.doc, uid)?)?;
        let host = self.doc.slot_of(Uuid::parse_str(&o.opening.host).ok()?)?;
        let w = CoreWall::of(&self.wall(host)?)?;
        let path = opening::wall_path(&w.pts, w.bulges.as_deref(), w.spec.closed)?;
        let (s0, s1) = kentos_domain::opening_follow::span(&o.opening);
        let k = opening::edge_of(&path, s0, s1)?;
        let (a, b) = w.edge(k)?;
        let c = opening::jambs(&path, &w.spec, s0, s1)?;
        let at = match end {
            JambEnd::Start => P::new((c[0].x + c[1].x) / 2.0, (c[0].y + c[1].y) / 2.0),
            JambEnd::End => P::new((c[2].x + c[3].x) / 2.0, (c[2].y + c[3].y) / 2.0),
        };
        Some((at, unit(a, b), host, k))
    }
}

/// One shift of the problem.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Var {
    Edge(Slot, usize),
    Opening(Slot),
}

/// The constraints of a set that name architectural parts, read as the core's problem over a view.
struct Read {
    vars: Vec<Var>,
    /// Per bond: its constraint's index in the set.
    rules: Vec<usize>,
    bonds: Vec<Bond>,
    /// Constraints whose parts the drawing does not have (a wall line on no wall, an arc edge, a jamb off its wall).
    broken: Vec<usize>,
}

fn var(vars: &mut Vec<Var>, v: Var) -> usize {
    match vars.iter().position(|x| *x == v) {
        Some(i) => i,
        None => {
            vars.push(v);
            vars.len() - 1
        }
    }
}

fn item(view: &View<'_>, vars: &mut Vec<Var>, e: &ElementRef) -> Option<Item> {
    let slot = slot_of(view.doc, e.uid)?;
    match e.element {
        CadElement::Wall { edge, line } => {
            let (at, dir) = view.line(e.uid, edge, line)?;
            Some(Item::Line(Line {
                var: var(vars, Var::Edge(slot, edge as usize)),
                at,
                dir,
            }))
        }
        CadElement::Jamb { end } => {
            let (at, dir, host, k) = view.jamb(e.uid, end)?;
            let v = var(vars, Var::Opening(slot));
            let host = vars.iter().position(|x| *x == Var::Edge(host, k));
            Some(Item::Jamb(Jamb {
                var: v,
                at,
                dir,
                host,
            }))
        }
        _ => None,
    }
}

fn line_of(i: Item) -> Option<Line> {
    match i {
        Item::Line(l) => Some(l),
        Item::Jamb(_) => None,
    }
}

/// Reads the architectural constraints of `set` over `view`, each distance's value from `values` (metres).
fn read(view: &View<'_>, set: &ConstraintSet, values: &BTreeMap<String, f64>) -> Read {
    let mut out = Read {
        vars: Vec::new(),
        rules: Vec::new(),
        bonds: Vec::new(),
        broken: Vec::new(),
    };
    // Edges first, so a jamb finds its wall's edge when that edge is one of the problem's.
    for c in &set.constraints {
        if !kentos_commands::rule_is_architectural(&c.rule) {
            continue;
        }
        for e in c.rule.elements() {
            if let CadElement::Wall { edge, .. } = e.element
                && let Some(slot) = slot_of(view.doc, e.uid)
            {
                var(&mut out.vars, Var::Edge(slot, edge as usize));
            }
        }
    }
    for (i, c) in set.constraints.iter().enumerate() {
        if !kentos_commands::rule_is_architectural(&c.rule) {
            continue;
        }
        let mut get = |e: &ElementRef| item(view, &mut out.vars, e);
        let bond = match &c.rule {
            ConstraintRule::Distance {
                a,
                b,
                left,
                parameter,
                ..
            } => match (get(a), get(b)) {
                (Some(a), Some(b)) if c.reference => {
                    let _ = (a, b);
                    Some(Bond::Reference)
                }
                (Some(a), Some(b)) => values.get(parameter).map(|&value| Bond::Distance {
                    a,
                    b,
                    value,
                    left: *left,
                }),
                _ => None,
            },
            ConstraintRule::Parallel { a, b } => {
                match (get(a).and_then(line_of), get(b).and_then(line_of)) {
                    (Some(a), Some(b)) => Some(Bond::Parallel { a, b }),
                    _ => None,
                }
            }
            ConstraintRule::Symmetric { a, b, axis } => {
                match (
                    get(a).and_then(line_of),
                    get(b).and_then(line_of),
                    get(axis).and_then(line_of),
                ) {
                    (Some(a), Some(b), Some(axis)) => Some(Bond::Symmetric { a, b, axis }),
                    _ => None,
                }
            }
            ConstraintRule::Fix { a } => get(a).map(|a| Bond::Fix { a }),
            _ => None,
        };
        match bond {
            Some(b) => {
                out.rules.push(i);
                out.bonds.push(b);
            }
            None => out.broken.push(i),
        }
    }
    out
}

/// What the adapter made of a set.
#[derive(Debug, Default)]
pub(crate) struct Solved {
    /// The walls and openings it moved, as they would be written.
    pub changed: Vec<(Slot, Entity)>,
    /// The constraints (by index in the set) that cannot hold together, and why in Turkish.
    pub conflicting: Vec<usize>,
    pub why: Option<String>,
    /// The constraints whose parts the drawing does not have.
    pub broken: Vec<usize>,
    pub dof: usize,
    /// Whether the set names any architectural part.
    pub any: bool,
    /// Of `conflicting`, those whose taking away alone lets the write through (docs/adr/0780 §1).
    pub frees: Vec<usize>,
    /// The conflict came while the shifts were written (Kaydır refused one, a bond did not settle), not from the
    /// bonds' equations.
    pub late: bool,
}

/// Solves the architectural constraints of `set` (docs/adr/0770 §4) over the document as it is, with `values` its
/// parameters in metres; `before`: the shapes an edit's objects had before it (by id), whose moved parts are held.
/// On a conflict, each conflicting constraint is taken away alone and the set solved again: those that let the write
/// through are `frees` (docs/adr/0780 §1). A conflict of the bonds' equations is solved again as far as its equations
/// (the shifts not written: the budget, §4); one that came while the shifts were written, whole.
pub(crate) fn solve(
    doc: &Document,
    set: &ConstraintSet,
    values: &BTreeMap<String, f64>,
    holds: &[Held],
    before: Option<&HashMap<EntityId, Shape>>,
) -> Solved {
    let mut out = solve_once(doc, set, values, holds, before);
    if out.conflicting.is_empty() {
        return out;
    }
    if out.late {
        for &k in &out.conflicting {
            let mut without = set.clone();
            without.constraints.remove(k);
            if solve_once(doc, &without, values, holds, before)
                .conflicting
                .is_empty()
            {
                out.frees.push(k);
            }
        }
        return out;
    }
    // The equations' conflict: the set read once, each bond left out of its problem in turn.
    let view = View::new(doc);
    let read = read(&view, set, values);
    for &k in &out.conflicting {
        let Some(b) = read.rules.iter().position(|&r| r == k) else {
            continue;
        };
        let mut without = Read {
            vars: read.vars.clone(),
            rules: read.rules.clone(),
            bonds: read.bonds.clone(),
            broken: Vec::new(),
        };
        without.rules.remove(b);
        without.bonds.remove(b);
        if equations(doc, &without, &view, holds, before, false).is_ok() {
            out.frees.push(k);
        }
    }
    out
}

/// The bonds' equations over a read set solved, nothing written (docs/adr/0770 §4): the shifts and the freedom (none
/// unless `shifts`), or the bonds (by index in `read.bonds`) that conflict and why in Turkish.
fn equations(
    doc: &Document,
    read: &Read,
    view: &View<'_>,
    holds: &[Held],
    before: Option<&HashMap<EntityId, Shape>>,
    shifts: bool,
) -> Result<(Vec<f64>, usize), (Vec<usize>, String)> {
    let (mut hold, fixed_moved) = held(doc, read, view, holds, before);
    // Not an edit's follow (a new bond, a parameter's new value, a solve): a distance that does not hold now keeps its
    // `a` where it is; its `b` and what follows it move (docs/adr/0770 §4).
    if before.is_none() {
        for bond in &read.bonds {
            if matches!(bond, Bond::Distance { .. })
                && core::miss(bond).is_some_and(|m| m.abs() > SETTLED)
            {
                hold[bond_vars(bond)[0]] = true;
            }
        }
    }
    if !fixed_moved.is_empty() {
        return Err((
            fixed_moved,
            "Sabit bağlı bir duvar ya da açıklık oynatılamaz".into(),
        ));
    }
    // Asked only whether they hold together: eliminated, the least change not worked out (the budget, §4).
    let outcome = if shifts {
        core::solve(&hold, &read.bonds)
    } else {
        core::conflict(&hold, &read.bonds).unwrap_or(Outcome::Solved {
            shifts: Vec::new(),
            dof: 0,
        })
    };
    match outcome {
        Outcome::Conflict { bonds, why } => Err((bonds, why_words(why).into())),
        Outcome::Solved { shifts, dof } => Ok((shifts, dof)),
    }
}

/// What a write held: the shifts of the edges and openings it moved (`before`: their shapes before it, by id), and
/// the parts the caller holds; the objects on locked layers whole.
fn held(
    doc: &Document,
    read: &Read,
    view: &View<'_>,
    holds: &[Held],
    before: Option<&HashMap<EntityId, Shape>>,
) -> (Vec<bool>, Vec<usize>) {
    let mut out = vec![false; read.vars.len()];
    let mut fixed_moved = Vec::new();
    let uid = |s: Slot| doc.uid(s).map(|u| EntityId(u.into_bytes()));
    for (k, v) in read.vars.iter().enumerate() {
        let slot = match v {
            Var::Edge(s, _) | Var::Opening(s) => *s,
        };
        if doc
            .get(slot)
            .is_some_and(|e| doc.layers().is_locked(e.base().layer_id.as_str()))
        {
            out[k] = true;
        }
        let Some(id) = uid(slot) else { continue };
        for h in holds.iter().filter(|h| h.uid == id) {
            match (h.element, v) {
                (None, _) => out[k] = true,
                (Some(CadElement::Wall { edge, .. }), Var::Edge(_, e)) if edge as usize == *e => {
                    out[k] = true
                }
                (Some(CadElement::Jamb { .. }), Var::Opening(_)) => out[k] = true,
                _ => {}
            }
        }
        let Some(was) = before.and_then(|b| b.get(&id)) else {
            continue;
        };
        let moved = match (v, was) {
            (Var::Edge(_, e), Shape::Wall { pts, bulges, wall }) => {
                let then = CoreWall {
                    pts: pts.clone(),
                    bulges: bulges.clone(),
                    spec: wall.clone(),
                };
                let now = view.wall(slot).and_then(|w| CoreWall::of(&w));
                match (then.edge(*e), now.as_ref().and_then(|n| n.edge(*e))) {
                    (Some((a0, b0)), Some((a1, b1))) => {
                        let (u0, u1) = (unit(a0, b0), unit(a1, b1));
                        let turned = (u0.x * u1.y - u0.y * u1.x).abs() > core::PARALLEL;
                        let n = P::new(-u0.y, u0.x);
                        let across = n.x * (a1.x - a0.x) + n.y * (a1.y - a0.y);
                        turned || across.abs() > core::TIE
                    }
                    _ => true,
                }
            }
            // An opening moved along its wall in the world (not one whose `at` was read again on a moved wall).
            (Var::Opening(_), Shape::Opening { place, .. }) => {
                let then = (place.len() == 4).then(|| {
                    P::new(
                        (place[0].x + place[1].x) / 2.0,
                        (place[0].y + place[1].y) / 2.0,
                    )
                });
                match (then, view.jamb(id, JambEnd::Start)) {
                    (Some(p0), Some((p1, h, ..))) => {
                        (h.x * (p1.x - p0.x) + h.y * (p1.y - p0.y)).abs() > core::TIE
                    }
                    _ => true,
                }
            }
            _ => false,
        };
        if moved {
            out[k] = true;
        }
    }
    // Equal spacing keeps its two ends; its axis follows (docs/adr/0770 §4).
    for bond in &read.bonds {
        if let Bond::Symmetric { a, b, .. } = bond {
            out[a.var] = true;
            out[b.var] = true;
        }
    }
    // A fix whose part the write moved cannot hold.
    for (b, bond) in read.bonds.iter().enumerate() {
        if let Bond::Fix { a } = bond {
            let k = match a {
                Item::Line(l) => l.var,
                Item::Jamb(j) => j.var,
            };
            let slot = match read.vars[k] {
                Var::Edge(s, _) | Var::Opening(s) => s,
            };
            if out[k] && uid(slot).is_some_and(|id| before.is_some_and(|m| m.contains_key(&id))) {
                fixed_moved.push(b);
            }
        }
    }
    (out, fixed_moved)
}

fn why_words(why: Why) -> &'static str {
    match why {
        Why::NotParallel => "Bağlı duvar çizgileri artık paralel değil (bir duvar döndü)",
        Why::Crossing => "Pervazın bağlandığı çizgi açıklığın duvarını en az 30° ile kesmiyor",
        Why::Inconsistent => "Bağlar birlikte tutmuyor",
    }
}

/// [`solve`] once, without asking which bond frees a conflict.
fn solve_once(
    doc: &Document,
    set: &ConstraintSet,
    values: &BTreeMap<String, f64>,
    holds: &[Held],
    before: Option<&HashMap<EntityId, Shape>>,
) -> Solved {
    let mut out = Solved {
        any: set
            .constraints
            .iter()
            .any(|c| kentos_commands::rule_is_architectural(&c.rule)),
        ..Solved::default()
    };
    if !out.any {
        return out;
    }
    let mut view = View::new(doc);
    let read = read(&view, set, values);
    out.broken = read.broken.clone();
    if read.bonds.is_empty() {
        return out;
    }
    let conflict = |out: &mut Solved, bonds: Vec<usize>, words: String| {
        out.conflicting = bonds.into_iter().map(|b| read.rules[b]).collect();
        out.why = Some(words);
    };
    let shifts = match equations(doc, &read, &view, holds, before, true) {
        Err((bonds, words)) => {
            conflict(&mut out, bonds, words);
            return out;
        }
        Ok((shifts, dof)) => {
            out.dof = dof;
            shifts
        }
    };
    // From here a conflict comes while the shifts are written.
    out.late = true;
    // Write the shifts: edges as Kaydır does, in the shifts' order, then the openings along their walls.
    let mut moved_walls: Vec<Slot> = Vec::new();
    for (k, v) in read.vars.iter().enumerate() {
        let Var::Edge(slot, edge) = *v else { continue };
        if shifts[k] == 0.0 {
            continue;
        }
        match shift(&mut view, slot, edge, shifts[k]) {
            Ok(changed) => {
                for s in changed {
                    if !moved_walls.contains(&s) {
                        moved_walls.push(s);
                    }
                }
            }
            Err(words) => {
                let bonds: Vec<usize> = (0..read.bonds.len())
                    .filter(|&b| bond_vars(&read.bonds[b]).contains(&k))
                    .collect();
                conflict(&mut out, bonds, words);
                return out;
            }
        }
    }
    carry(&mut view, &moved_walls);
    for (k, v) in read.vars.iter().enumerate() {
        let Var::Opening(slot) = *v else { continue };
        if shifts[k] == 0.0 {
            continue;
        }
        if let Some(mut o) = view.opening(slot) {
            o.opening.at += shifts[k];
            view.openings.insert(slot, o);
        }
    }
    // Every bond is measured again on what was written.
    let again = self::read(&view, set, values);
    for (b, bond) in again.bonds.iter().enumerate() {
        if core::miss(bond).is_none_or(|m| m.abs() > SETTLED) {
            conflict(
                &mut out,
                vec![b],
                "Bağlar duvarların izleyebildiği kadar tutamadı (bağlı duvarlar birbirini engelliyor)".into(),
            );
            out.conflicting = vec![again.rules[b]];
            return out;
        }
    }
    let mut changed: Vec<(Slot, Entity)> = view
        .walls
        .into_iter()
        .map(|(s, w)| (s, Entity::Wall(w)))
        .chain(
            view.openings
                .into_iter()
                .map(|(s, o)| (s, Entity::Opening(o))),
        )
        .collect();
    changed.sort_by_key(|(s, _)| s.0);
    out.changed = changed;
    out
}

fn bond_vars(b: &Bond) -> Vec<usize> {
    let v = |i: &Item| match i {
        Item::Line(l) => l.var,
        Item::Jamb(j) => j.var,
    };
    match b {
        Bond::Distance { a, b, .. } => vec![v(a), v(b)],
        Bond::Parallel { a, b } => vec![a.var, b.var],
        Bond::Symmetric { a, b, axis } => vec![a.var, b.var, axis.var],
        Bond::Fix { a } => vec![v(a)],
        Bond::Reference => Vec::new(),
    }
}

/// Kaydır on the view: wall `slot`'s edge moved `by` to its left, the walls joined to it following (docs/adr/0710).
/// The walls it changed; why not, in Turkish (a locked layer among them too).
fn shift(view: &mut View<'_>, slot: Slot, edge: usize, by: f64) -> Result<Vec<Slot>, String> {
    let mut slots = vec![slot];
    let mut walls = vec![
        CoreWall::of(&view.wall(slot).ok_or("Bağlı duvar yok")?)
            .ok_or("Bağlı duvar çizilemiyor")?,
    ];
    for e in view.doc.entities() {
        let s = Slot(e.base().id);
        if s != slot
            && matches!(e, Entity::Wall(_))
            && let Some(w) = view.wall(s).and_then(|w| CoreWall::of(&w))
        {
            slots.push(s);
            walls.push(w);
        }
    }
    let ins: Vec<WallIn<'_>> = walls.iter().map(CoreWall::input).collect();
    let moved = wall_shift::shift(&ins, 0, edge, by)?;
    let mut out = Vec::new();
    for m in moved {
        let s = slots[m.wall];
        let Some(mut w) = view.wall(s) else { continue };
        if view.doc.layers().is_locked(&w.base.layer_id) {
            let id = &w.base.layer_id;
            let name = view
                .doc
                .layers()
                .get(id)
                .map_or(id.as_str(), |n| n.name.as_str());
            return Err(format!(
                "Bağ “{name}” katmanındaki bir duvarı kaydırmak istiyor ve o katman kilitli. Kilidi açın ya da bağı kaldırın."
            ));
        }
        w.wall.pts = m
            .pts
            .iter()
            .map(|p| kentos_commands::Vec2 { x: p.x, y: p.y })
            .collect();
        view.walls.insert(s, w);
        out.push(s);
    }
    Ok(out)
}

/// The openings of the walls moved, read back from their jambs onto their walls' new paths (docs/adr/0341 §3): their
/// `at` (their hand and side when the path turned round them), their place in the world kept.
fn carry(view: &mut View<'_>, walls: &[Slot]) {
    use kentos_domain::opening_follow::{anchor, wall_line};
    for &w in walls {
        let Some(wall) = view.wall(w) else { continue };
        let Some(line) = wall_line(&Entity::Wall(wall)) else {
            continue;
        };
        let Some(uid) = view.doc.uid(w).map(|u| u.to_string()) else {
            continue;
        };
        for (slot, _) in view.doc.openings_of(&uid) {
            let Some(mut e) = view.opening(slot) else {
                continue;
            };
            let o = &e.opening;
            let place: Vec<P> = o.place.iter().map(|q| P::new(q.x, q.y)).collect();
            if let Some(r) = opening::read_back(&line.path, &place, anchor(o), o.width) {
                let (hand, side) = (o.hand.other(), o.side.other());
                e.opening.at = r.at;
                if r.hand_turned {
                    e.opening.hand = hand;
                }
                if r.side_turned {
                    e.opening.side = side;
                }
                view.openings.insert(slot, e);
            }
        }
    }
}

/// What an architectural constraint measures now (metres): a distance its clear distance (along the wall for two
/// jambs); none for the others or when its parts do not fit.
pub(crate) fn measured(doc: &Document, rule: &ConstraintRule) -> Option<f64> {
    let view = View::new(doc);
    let mut vars = Vec::new();
    match rule {
        ConstraintRule::Distance { a, b, .. } => {
            let (a, b) = (item(&view, &mut vars, a)?, item(&view, &mut vars, b)?);
            let g = core::measure(&Bond::Distance {
                a,
                b,
                value: 0.0,
                left: true,
            })?;
            Some(match (a, b) {
                (Item::Jamb(_), Item::Jamb(_)) => g,
                _ => g.abs(),
            })
        }
        _ => None,
    }
}

/// The architectural constraints of `set` the drawing as it is does not meet (to [`SETTLED`]), or whose parts it does
/// not have; `values` the parameters in metres.
pub(crate) fn unmet(
    doc: &Document,
    set: &ConstraintSet,
    values: &BTreeMap<String, f64>,
) -> Vec<usize> {
    let view = View::new(doc);
    let read = read(&view, set, values);
    let mut out = read.broken.clone();
    for (b, bond) in read.bonds.iter().enumerate() {
        if core::miss(bond).is_none_or(|m| m.abs() > SETTLED) {
            out.push(read.rules[b]);
        }
    }
    out.sort_unstable();
    out
}

/// Which side of `a` `b` lies on now (docs/adr/0770 §4): the `left` a new distance takes from the drawing; none when
/// its parts do not fit (not parallel, a jamb's line along its wall).
pub(crate) fn side_now(doc: &Document, a: &ElementRef, b: &ElementRef) -> Option<bool> {
    let view = View::new(doc);
    let mut vars = Vec::new();
    let (a, b) = (item(&view, &mut vars, a)?, item(&view, &mut vars, b)?);
    let g = core::measure(&Bond::Distance {
        a,
        b,
        value: 0.0,
        left: true,
    })?;
    Some(g > 0.0)
}

/// Why an architectural constraint's parts do not fit the drawing (docs/adr/0770 §5): a wall line on no wall or on an
/// arc edge or an edge it has not, a jamb on no opening or off its wall, two jambs on different edges, a jamb's line
/// along its wall, lines a bond wants parallel that are not.
pub(crate) fn rule_problem(doc: &Document, rule: &ConstraintRule) -> Option<String> {
    let view = View::new(doc);
    let mut vars = Vec::new();
    let mut items = Vec::new();
    for e in rule.elements() {
        let what = match e.element {
            CadElement::Wall { edge, line } => match slot_of(doc, e.uid).and_then(|s| doc.get(s)) {
                Some(Entity::Wall(_)) => {
                    format!("duvarın {}. kenarının {}", edge + 1, line.label())
                }
                _ => return Some("duvar çizgisi yalnız bir duvarda olur".into()),
            },
            CadElement::Jamb { .. } => match slot_of(doc, e.uid).and_then(|s| doc.get(s)) {
                Some(Entity::Opening(_)) => "açıklığın pervazı".to_owned(),
                _ => return Some("pervaz yalnız bir açıklıkta olur".into()),
            },
            _ => return None,
        };
        match item(&view, &mut vars, e) {
            Some(i) => items.push(i),
            None => {
                return Some(format!(
                    "{what} bulunamadı: kenar yay ya da yok, ya da açıklık duvarının dışında"
                ));
            }
        }
    }
    let bond = match (rule, items.as_slice()) {
        (ConstraintRule::Distance { .. }, [Item::Jamb(a), Item::Jamb(b)]) => {
            let (ha, hb) = (vars_host(&view, rule, 0), vars_host(&view, rule, 1));
            if ha.is_none() || ha != hb {
                return Some("iki pervaz aynı duvarın aynı kenarında olmalı".into());
            }
            Bond::Distance {
                a: Item::Jamb(*a),
                b: Item::Jamb(*b),
                value: 0.0,
                left: false,
            }
        }
        (ConstraintRule::Distance { .. }, [a, b]) => Bond::Distance {
            a: *a,
            b: *b,
            value: 0.0,
            left: true,
        },
        (ConstraintRule::Parallel { .. }, [Item::Line(a), Item::Line(b)]) => {
            Bond::Parallel { a: *a, b: *b }
        }
        (ConstraintRule::Symmetric { .. }, [Item::Line(a), Item::Line(b), Item::Line(axis)]) => {
            Bond::Symmetric {
                a: *a,
                b: *b,
                axis: *axis,
            }
        }
        _ => return None,
    };
    match core::measure(&bond) {
        Some(_) => None,
        None => Some(match bond {
            Bond::Distance {
                a: Item::Jamb(_),
                b: Item::Line(_),
                ..
            } => {
                "pervazın bağlandığı duvar çizgisi açıklığın duvarını en az 30° ile kesmeli".into()
            }
            _ => "bağlanan duvar çizgileri paralel olmalı".into(),
        }),
    }
}

/// The wall and edge the `k`th element's jamb stands on.
fn vars_host(view: &View<'_>, rule: &ConstraintRule, k: usize) -> Option<(Slot, usize)> {
    let e = rule.elements().get(k).copied()?;
    let CadElement::Jamb { end } = e.element else {
        return None;
    };
    view.jamb(e.uid, end).map(|(_, _, h, edge)| (h, edge))
}

/// Where an architectural part is in the drawing (docs/adr/0770 §6): a wall line's middle and its edge moved to the
/// line, a jamb's point.
pub fn part_place(
    doc: &Document,
    uid: EntityId,
    element: &CadElement,
) -> Option<(P, Option<(P, P)>)> {
    let view = View::new(doc);
    match *element {
        CadElement::Wall { edge, line } => {
            let w = CoreWall::of(&view.wall(slot_of(doc, uid)?)?)?;
            let (a, b) = w.edge(edge as usize)?;
            let u = unit(a, b);
            let o = core::line_offset(&w.spec, kind(line));
            let (p, q) = (
                P::new(a.x - u.y * o, a.y + u.x * o),
                P::new(b.x - u.y * o, b.y + u.x * o),
            );
            Some((P::new((p.x + q.x) / 2.0, (p.y + q.y) / 2.0), Some((p, q))))
        }
        CadElement::Jamb { end } => view.jamb(uid, end).map(|(at, ..)| (at, None)),
        _ => None,
    }
}

/// A distance bond on one side of a wall's edge (docs/adr/0770 §3): the constraint's id, its parameter and the side.
#[derive(Clone, Debug, PartialEq)]
pub struct SideBond {
    pub id: Uuid,
    pub parameter: String,
    /// The edge's left side (its left face bonded).
    pub left: bool,
    /// Only measures: the side moves freely.
    pub reference: bool,
}

/// The distance bonds that hold edge `edge` of wall `uid` by one of its faces, in the set's order.
pub fn side_bonds(doc: &Document, uid: Uuid, edge: u32) -> Vec<SideBond> {
    let me = EntityId(uid.into_bytes());
    let face = |r: &ElementRef| match r.element {
        CadElement::Wall { edge: e, line } if r.uid == me && e == edge => match line {
            WallLine::LeftFace => Some(true),
            WallLine::RightFace => Some(false),
            _ => None,
        },
        _ => None,
    };
    doc.constraints()
        .constraints
        .iter()
        .filter_map(|c| match &c.rule {
            ConstraintRule::Distance {
                a, b, parameter, ..
            } => face(b).or_else(|| face(a)).map(|left| SideBond {
                id: Uuid::from_bytes(c.id.0),
                parameter: parameter.clone(),
                left,
                reference: c.reference,
            }),
            _ => None,
        })
        .collect()
}
