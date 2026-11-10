//! Kaçış yolları (docs/adr/8035; ADV-23, M35): the walking routes Binaların Yangından Korunması Hakkında Yönetmelik
//! (BYKHY) measures — 4/u (a storey's farthest point to its nearest storey exit), 32/4, 32/6, 33/6-b, 48/2, 48/4.
//!
//! - **Rooms** are a storey's spaces that are neither voids nor outdoor; their net outlines (inner faces) bound the walk.
//! - **Passages** join them: a door or a gap without a sill (docs/adr/0722 §5), its two sides 10 cm beyond its wall's
//!   faces; a space separator, crossed at points a metre apart along it.
//! - **Exits (kat çıkışı):**
//!   - an escape stair's room: a room named or used as Merdiven (Merdiven evi, Yangın merdiveni …) that is no dwelling
//!     unit's own; reaching it is reaching the exit;
//!   - an open stair: a stair's foot on its own storey, its head on the storey above, standing in a room that is no
//!     dwelling unit's own;
//!   - a passage to the outside on a storey at or below ±0.00 (the building's final exit).
//! - **Who walks where (33/6-b):** a room's route crosses its own unit's rooms and the common ones (no group, a common
//!   area, an outdoor area), never another unit's.
//! - **A room's farthest point** is taken 40 cm in front of its walls (32/6): each corner moved in along its bisector
//!   until it stands 0.40 m from both walls; the farthest by the walk is the one kept.
//! - **Round a room's inner corners** a route keeps the same 0.40 m: the regulation gives the start's distance only.
//! - **One way (tek yön):** the part of the route to the nearest exit it shares with the route to the nearest other
//!   exit; all of it when the storey has one exit.
//!
//! A route is the drawing's measure, not an approval.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use kentos_contracts::{Entity, SpaceFields, SpaceMethod, SpacePart, UnitKind};
use kentos_geometry_core::Vec2 as V;
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::region::inside_area;

use crate::Document;
use crate::identity::Slot;
use crate::space_follow::SpaceSearch;

/// BYKHY 32/6: the farthest point stands 40 cm in front of the walls round the room; a route keeps it round corners.
pub const WALL_CLEAR: f64 = 0.40;
/// How far beyond its wall's face a passage's side point stands, m.
const SIDE: f64 = 0.10;
/// The spacing of the crossing points along a space separator, m.
const SEPARATOR_STEP: f64 = 1.0;

/// A walk from a start to a target: its points, its length (m).
#[derive(Clone, Debug, PartialEq)]
pub struct Route {
    pub path: Vec<V>,
    pub length: f64,
}

/// A walk to the nearest storey exit.
#[derive(Clone, Debug, PartialEq)]
pub struct ExitRoute {
    pub route: Route,
    /// The part it shares with the walk to the nearest other exit (tek yön), m; all of it with one exit.
    pub one_way: f64,
    /// How many exits the start reaches.
    pub exits: usize,
}

/// A room's escape (docs/adr/8035).
#[derive(Clone, Debug, PartialEq)]
pub struct RoomEscape {
    pub room: Slot,
    pub level: Option<String>,
    /// Its program group, by its id.
    pub unit: Option<String>,
    /// From its farthest point to the nearest storey exit; none when it reaches none.
    pub exit: Option<ExitRoute>,
    /// Whether its storey has an exit it may walk to.
    pub storey_exits: bool,
    /// For a dwelling unit's room: from its farthest point to its unit's entrance (48/2); none for another room or a
    /// unit without an entrance on the storey.
    pub entrance: Option<Route>,
    /// Whether its unit has an entrance on the storey (a dwelling unit's room only).
    pub has_entrance: bool,
    /// Its passages (doors, gaps without a sill), counted: its exits for 39/2.
    pub passages: usize,
}

/// A dwelling unit's entrance: from its door's outer side to the nearest storey exit (48/4).
#[derive(Clone, Debug, PartialEq)]
pub struct DoorEscape {
    pub unit: String,
    pub level: Option<String>,
    /// The door, by its object.
    pub door: Slot,
    pub at: V,
    pub exit: Option<ExitRoute>,
}

/// The storeys' escape routes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Escape {
    pub rooms: Vec<RoomEscape>,
    pub doors: Vec<DoorEscape>,
    /// Each storey's exits, counted.
    pub exits: BTreeMap<Option<String>, usize>,
}

pub use crate::room_names::{dwelling_nature, stair_name};

/// A room as the routes read it.
struct Room {
    slot: Slot,
    level: Option<String>,
    unit: Option<String>,
    /// Its walk's owner: its unit when that is an independent unit or an annex; none for a common room.
    owner: Option<String>,
    /// A dwelling unit's own room.
    dwelling: bool,
    /// An escape stair's room.
    stair: bool,
    /// Its rings, arcs as segments, each without its first point repeated: the outer first.
    rings: Vec<Vec<V>>,
    areas: Vec<Area>,
    label: V,
}

impl Room {
    fn holds(&self, p: V) -> bool {
        self.areas.iter().any(|a| inside_area(a, p))
    }

    /// Whether the straight walk from `a` to `b` stays inside it.
    fn sees(&self, a: V, b: V) -> bool {
        if (a.x - b.x).hypot(a.y - b.y) < 1e-9 {
            return true;
        }
        for ring in &self.rings {
            let n = ring.len();
            for i in 0..n {
                if crosses(a, b, ring[i], ring[(i + 1) % n]) {
                    return false;
                }
            }
        }
        self.holds(V::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0))
    }

    /// Its corners as a walk reads them: each convex corner moved in to stand `WALL_CLEAR` from both walls (the
    /// farthest points' candidates), each inner corner moved out to stand `WALL_CLEAR` from it (a route's turn).
    fn corners(&self) -> (Vec<V>, Vec<V>) {
        let (mut convex, mut inner) = (Vec::new(), Vec::new());
        for (k, ring) in self.rings.iter().enumerate() {
            let n = ring.len();
            if n < 3 {
                continue;
            }
            let orient = signed_area(ring).signum();
            let hole = if k == 0 { 1.0 } else { -1.0 };
            for i in 0..n {
                let (p, v, q) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
                let u1 = unit(V::new(p.x - v.x, p.y - v.y));
                let u2 = unit(V::new(q.x - v.x, q.y - v.y));
                let (Some(u1), Some(u2)) = (u1, u2) else {
                    continue;
                };
                let turn = (v.x - p.x) * (q.y - v.y) - (v.y - p.y) * (q.x - v.x);
                let Some(bis) = unit(V::new(u1.x + u2.x, u1.y + u2.y)) else {
                    continue;
                };
                if turn * orient * hole > 1e-12 {
                    // Convex: the point on the bisector `WALL_CLEAR` from both walls' lines.
                    let half = (u1.x * u2.x + u1.y * u2.y).clamp(-1.0, 1.0).acos() / 2.0;
                    let s = half.sin();
                    if s > 1e-6 {
                        let at = V::new(v.x + bis.x * WALL_CLEAR / s, v.y + bis.y * WALL_CLEAR / s);
                        if self.holds(at) && self.clear_of_walls(at) {
                            convex.push(at);
                        }
                    }
                } else if turn * orient * hole < -1e-12 {
                    // Inner: out from the corner, away from the walls.
                    let at = V::new(v.x - bis.x * WALL_CLEAR, v.y - bis.y * WALL_CLEAR);
                    if self.holds(at) {
                        inner.push(at);
                    }
                }
            }
        }
        (convex, inner)
    }

    /// Whether a point stands at least nearly `WALL_CLEAR` from every wall (a room narrower than 0.80 m has none).
    fn clear_of_walls(&self, p: V) -> bool {
        self.rings.iter().all(|ring| {
            let n = ring.len();
            (0..n).all(|i| {
                distance_to_segment(p, ring[i], ring[(i + 1) % n]) + 1e-6 >= WALL_CLEAR * 0.99
            })
        })
    }
}

fn unit(v: V) -> Option<V> {
    let l = v.x.hypot(v.y);
    (l > 1e-12).then(|| V::new(v.x / l, v.y / l))
}

fn signed_area(ring: &[V]) -> f64 {
    let n = ring.len();
    (0..n)
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum::<f64>()
        / 2.0
}

fn orient(a: V, b: V, c: V) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

/// Whether two segments cross at a point inside both (touching or running along does not count).
fn crosses(a: V, b: V, c: V, d: V) -> bool {
    let (o1, o2) = (orient(a, b, c), orient(a, b, d));
    let (o3, o4) = (orient(c, d, a), orient(c, d, b));
    o1 * o2 < -1e-12 && o3 * o4 < -1e-12
}

fn distance_to_segment(p: V, a: V, b: V) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l2 = dx * dx + dy * dy;
    let t = if l2 < 1e-18 {
        0.0
    } else {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0)
    };
    (p.x - a.x - t * dx).hypot(p.y - a.y - t * dy)
}

fn dist(a: V, b: V) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

/// A room's net outline: its inner faces found again from its seed, else its own.
fn net_parts(search: &mut SpaceSearch<'_>, s: &SpaceFields) -> Vec<SpacePart> {
    if s.method != SpaceMethod::Net && s.seed.is_some() {
        let mut net = s.clone();
        net.method = SpaceMethod::Net;
        let found = search.found_preserving_legacy(&net);
        if found.broken.is_none() && !found.parts.is_empty() {
            return found.parts;
        }
    }
    s.parts.clone()
}

/// A part's rings as straight segments.
fn rings_of(parts: &[SpacePart]) -> Vec<Vec<V>> {
    use kentos_geometry_core::tessellate::bulge_path;
    let mut out = Vec::new();
    for p in parts {
        for r in std::iter::once(&p.outer).chain(&p.holes) {
            let pts: Vec<V> = r.pts.iter().map(|q| V::new(q.x, q.y)).collect();
            let mut ring = bulge_path(
                &pts,
                r.bulges.as_deref(),
                true,
                kentos_geometry_core::geom::curve_outline::CURVE_TOL,
            );
            if ring.len() > 1 && dist(ring[0], ring[ring.len() - 1]) < 1e-9 {
                ring.pop();
            }
            if ring.len() >= 3 {
                out.push(ring);
            }
        }
    }
    out
}

#[derive(Clone, Copy, PartialEq)]
struct Queued(f64, usize);

impl Eq for Queued {}

impl Ord for Queued {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.total_cmp(&self.0).then(other.1.cmp(&self.1))
    }
}

impl PartialOrd for Queued {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A storey's walk: its nodes (each in a room) and their edges.
struct Graph {
    at: Vec<V>,
    room: Vec<usize>,
    edges: Vec<Vec<(usize, f64)>>,
}

impl Graph {
    fn node(&mut self, at: V, room: usize) -> usize {
        self.at.push(at);
        self.room.push(room);
        self.edges.push(Vec::new());
        self.at.len() - 1
    }

    fn join(&mut self, a: usize, b: usize, w: f64) {
        self.edges[a].push((b, w));
        self.edges[b].push((a, w));
    }

    /// Each node's distance from `sources` and the next node towards them, through the rooms `walk` lets in.
    fn reach(
        &self,
        sources: &[usize],
        walk: impl Fn(usize) -> bool,
    ) -> (Vec<f64>, Vec<Option<usize>>) {
        let n = self.at.len();
        let (mut d, mut next) = (vec![f64::INFINITY; n], vec![None; n]);
        let mut heap = BinaryHeap::new();
        for &s in sources {
            if walk(self.room[s]) {
                d[s] = 0.0;
                heap.push(Queued(0.0, s));
            }
        }
        while let Some(Queued(du, u)) = heap.pop() {
            if du > d[u] {
                continue;
            }
            for &(v, w) in &self.edges[u] {
                if !walk(self.room[v]) {
                    continue;
                }
                let dv = du + w;
                if dv + 1e-12 < d[v] {
                    d[v] = dv;
                    next[v] = Some(u);
                    heap.push(Queued(dv, v));
                }
            }
        }
        (d, next)
    }
}

/// From a start in a room: the best first node and the walk's length to the sources `d` measures.
fn best(g: &Graph, room: &Room, room_nodes: &[usize], d: &[f64], p: V) -> Option<(usize, f64)> {
    room_nodes
        .iter()
        .filter(|&&k| d[k].is_finite() && room.sees(p, g.at[k]))
        .map(|&k| (k, dist(p, g.at[k]) + d[k]))
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

/// The walk from a start through `first` along `next` to the sources: its node ids (the start not among them).
fn chain(first: usize, next: &[Option<usize>]) -> Vec<usize> {
    let mut out = vec![first];
    let mut at = first;
    while let Some(n) = next[at] {
        out.push(n);
        at = n;
        if out.len() > next.len() {
            break;
        }
    }
    out
}

fn length(path: &[V]) -> f64 {
    path.windows(2).map(|w| dist(w[0], w[1])).sum()
}

/// The storeys' escape routes (docs/adr/8035).
pub fn escape(doc: &Document) -> Escape {
    let program = doc.program();
    let mut search = SpaceSearch::new(doc);
    let levels: Vec<kentos_contracts::ArchLevel> = {
        let mut l = doc
            .settings()
            .arch
            .as_ref()
            .map(|a| a.levels.clone())
            .unwrap_or_default();
        l.sort_by(|a, b| a.elevation.total_cmp(&b.elevation));
        l
    };
    let elevation = |level: &Option<String>| {
        level
            .as_deref()
            .and_then(|id| levels.iter().find(|l| l.id == id))
            .map(|l| l.elevation)
    };
    let above = |level: Option<&str>| -> Option<String> {
        let i = levels.iter().position(|l| Some(l.id.as_str()) == level)?;
        levels.get(i + 1).map(|l| l.id.clone())
    };
    // The rooms.
    let mut rooms: Vec<Room> = Vec::new();
    for e in doc.entities() {
        let Entity::Space(s) = e else { continue };
        let x = &s.space;
        let outdoor = x
            .usage
            .as_deref()
            .and_then(|u| program.brief(u))
            .is_some_and(|b| b.outdoor);
        if x.void || outdoor || x.broken.is_some() {
            continue;
        }
        let group = x.unit.as_deref().and_then(|u| program.unit(u));
        let owner = group
            .filter(|u| matches!(u.kind, UnitKind::Unit | UnitKind::Annex))
            .map(|u| u.id.clone());
        let dwelling =
            group.is_some_and(|u| u.kind == UnitKind::Unit && dwelling_nature(u.nature.as_deref()));
        let parts = net_parts(&mut search, x);
        let rings = rings_of(&parts);
        if rings.is_empty() {
            continue;
        }
        let areas = crate::arch_values::core_areas(&parts);
        let label = kentos_geometry_core::geom::space::label_at(&areas).unwrap_or(rings[0][0]);
        rooms.push(Room {
            slot: Slot(s.base.id),
            level: e.own_level().map(str::to_owned),
            unit: x.unit.clone(),
            owner,
            dwelling,
            stair: !dwelling && stair_name(x.name.as_deref(), x.usage.as_deref()),
            rings,
            areas,
            label,
        });
    }
    let room_at =
        |level: &Option<String>, p: V| rooms.iter().position(|r| &r.level == level && r.holds(p));
    // The passages, the separators and the stairs, by storey.
    let passages = crate::passages::passages(doc);
    let mut out = Escape::default();
    let storeys: BTreeSet<Option<String>> = rooms.iter().map(|r| r.level.clone()).collect();
    for level in storeys {
        let ground = elevation(&level).is_none_or(|z| z <= 0.01);
        let mut g = Graph {
            at: Vec::new(),
            room: Vec::new(),
            edges: Vec::new(),
        };
        let mut groups: Vec<Vec<usize>> = Vec::new();
        let mut stair_groups: BTreeMap<usize, usize> = BTreeMap::new();
        // A unit's entrances: its room's side node, the other side's node (none: the outside), the door.
        let mut entrances: Vec<(String, usize, Option<usize>, Slot, V)> = Vec::new();
        let mut doors_of: BTreeMap<usize, usize> = BTreeMap::new();
        let cross = |g: &mut Graph, a: V, b: V| -> (Option<usize>, Option<usize>) {
            let (ra, rb) = (room_at(&level, a), room_at(&level, b));
            let na = ra.map(|r| g.node(a, r));
            let nb = rb.map(|r| g.node(b, r));
            if let (Some(x), Some(y)) = (na, nb) {
                g.join(x, y, dist(a, b));
            }
            (na, nb)
        };
        for p in passages.iter().filter(|p| p.level == level) {
            let c = V::new((p.a.x + p.b.x) / 2.0, (p.a.y + p.b.y) / 2.0);
            let Some(axis) = unit(V::new(p.b.x - p.a.x, p.b.y - p.a.y)) else {
                continue;
            };
            let n = V::new(-axis.y, axis.x);
            let off = p.half + SIDE;
            let (s1, s2) = (
                V::new(c.x + n.x * off, c.y + n.y * off),
                V::new(c.x - n.x * off, c.y - n.y * off),
            );
            let (n1, n2) = cross(&mut g, s1, s2);
            for n in [n1, n2].into_iter().flatten() {
                *doors_of.entry(g.room[n]).or_default() += 1;
            }
            let door = crate::identity::Uuid::parse_str(&p.uid)
                .ok()
                .and_then(|u| doc.slot_of(u))
                .unwrap_or(Slot(0));
            // The building's final exit: a passage to the outside at or below ±0.00.
            match (n1, n2) {
                (Some(x), None) | (None, Some(x)) if ground => groups.push(vec![x]),
                _ => {}
            }
            // A unit's entrance: one side its room, the other side not.
            for (inner, outer) in [(n1, n2), (n2, n1)] {
                let Some(i) = inner else { continue };
                let r = &rooms[g.room[i]];
                if !r.dwelling {
                    continue;
                }
                let theirs = outer.is_some_and(|o| rooms[g.room[o]].unit == r.unit);
                if !theirs && let Some(u) = r.unit.clone() {
                    entrances.push((u, i, outer, door, c));
                }
            }
        }
        for e in doc.entities() {
            let Entity::Polyline(path) = e else { continue };
            if path.space_separator.is_none() || e.own_level().map(str::to_owned) != level {
                continue;
            }
            for w in path.pts.windows(2) {
                let (a, b) = (V::new(w[0].x, w[0].y), V::new(w[1].x, w[1].y));
                let len = dist(a, b);
                let Some(d) = unit(V::new(b.x - a.x, b.y - a.y)) else {
                    continue;
                };
                let n = V::new(-d.y, d.x);
                let count = (len / SEPARATOR_STEP).floor().max(1.0) as usize;
                for k in 0..count {
                    let t = (k as f64 + 0.5) / count as f64 * len;
                    let q = V::new(a.x + d.x * t, a.y + d.y * t);
                    cross(
                        &mut g,
                        V::new(q.x + n.x * SIDE, q.y + n.y * SIDE),
                        V::new(q.x - n.x * SIDE, q.y - n.y * SIDE),
                    );
                }
            }
        }
        // Open stairs: a stair's foot on its storey, its head on the one above, in a room no dwelling unit owns.
        for e in doc.entities() {
            let Entity::Stair(st) = e else { continue };
            let pts = &st.stair.pts;
            let (Some(foot), Some(head)) = (pts.first(), pts.last()) else {
                continue;
            };
            let own = e.own_level().map(str::to_owned);
            for (p, on) in [(foot, own.clone()), (head, above(own.as_deref()))] {
                if on != level {
                    continue;
                }
                let at = V::new(p.x, p.y);
                if let Some(r) = room_at(&level, at)
                    && !rooms[r].dwelling
                    && !rooms[r].stair
                {
                    let k = g.node(at, r);
                    groups.push(vec![k]);
                }
            }
        }
        // Inner corners: a route's turns.
        let on: Vec<usize> = (0..rooms.len())
            .filter(|&i| rooms[i].level == level)
            .collect();
        let mut convex: BTreeMap<usize, Vec<V>> = BTreeMap::new();
        for &i in &on {
            let (c, inner) = rooms[i].corners();
            for at in inner {
                g.node(at, i);
            }
            convex.insert(i, c);
        }
        // An escape stair's room: every node in it is its exit.
        for k in 0..g.at.len() {
            if rooms[g.room[k]].stair {
                let next = groups.len();
                let gi = *stair_groups.entry(g.room[k]).or_insert(next);
                if gi == groups.len() {
                    groups.push(Vec::new());
                }
                groups[gi].push(k);
            }
        }
        // Each room's nodes see each other.
        let mut room_nodes: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for k in 0..g.at.len() {
            room_nodes.entry(g.room[k]).or_default().push(k);
        }
        for (&r, nodes) in &room_nodes {
            for (i, &a) in nodes.iter().enumerate() {
                for &b in &nodes[i + 1..] {
                    if rooms[r].sees(g.at[a], g.at[b]) {
                        g.join(a, b, dist(g.at[a], g.at[b]));
                    }
                }
            }
        }
        out.exits.insert(level.clone(), groups.len());
        // Who walks where: a room's own unit and the common rooms.
        let walk_for = |owner: &Option<String>| {
            let owner = owner.clone();
            let rooms = &rooms;
            move |r: usize| rooms[r].owner.is_none() || rooms[r].owner == owner
        };
        let owners: BTreeSet<Option<String>> = on
            .iter()
            .map(|&i| rooms[i].owner.clone())
            .chain(std::iter::once(None))
            .collect();
        let mut reached: BTreeMap<Option<String>, Vec<(Vec<f64>, Vec<Option<usize>>)>> =
            BTreeMap::new();
        for o in owners {
            let walk = walk_for(&o);
            let per: Vec<_> = groups.iter().map(|s| g.reach(s, &walk)).collect();
            reached.insert(o, per);
        }
        // A start's walk to the nearest exit and the part it shares with the walk to the nearest other.
        let exit_route =
            |room: &Room, nodes: &[usize], per: &[(Vec<f64>, Vec<Option<usize>>)], p: V| {
                let mut ways: Vec<(usize, usize, f64)> = per
                    .iter()
                    .enumerate()
                    .filter_map(|(gi, (d, _))| best(&g, room, nodes, d, p).map(|(k, l)| (gi, k, l)))
                    .collect();
                ways.sort_by(|a, b| a.2.total_cmp(&b.2));
                let &(g1, k1, l1) = ways.first()?;
                let ids1 = chain(k1, &per[g1].1);
                let mut path = vec![p];
                path.extend(ids1.iter().map(|&k| g.at[k]));
                let one_way = match ways.get(1) {
                    None => l1,
                    Some(&(g2, k2, _)) => {
                        let ids2 = chain(k2, &per[g2].1);
                        let shared = ids1.iter().zip(&ids2).take_while(|(a, b)| a == b).count();
                        if shared == 0 {
                            0.0
                        } else {
                            length(&path[..=shared])
                        }
                    }
                };
                Some(ExitRoute {
                    route: Route { length: l1, path },
                    one_way,
                    exits: ways.len(),
                })
            };
        let empty = Vec::new();
        for &i in &on {
            let room = &rooms[i];
            if room.stair {
                continue;
            }
            let nodes = room_nodes.get(&i).unwrap_or(&empty);
            let per = &reached[&room.owner];
            let starts = match convex.get(&i) {
                Some(c) if !c.is_empty() => c.clone(),
                _ => vec![room.label],
            };
            // The farthest start by the walk to the nearest exit.
            let exit = starts
                .iter()
                .filter_map(|&p| exit_route(room, nodes, per, p))
                .max_by(|a, b| a.route.length.total_cmp(&b.route.length));
            // A dwelling unit's room: to its unit's entrance, through its own rooms (48/2).
            let doors: Vec<usize> = entrances
                .iter()
                .filter(|(u, ..)| room.dwelling && room.unit.as_deref() == Some(u.as_str()))
                .map(|&(_, k, ..)| k)
                .collect();
            let entrance = (!doors.is_empty())
                .then(|| {
                    let unit = room.unit.clone();
                    let (d, next) = g.reach(&doors, |r| rooms[r].dwelling && rooms[r].unit == unit);
                    starts
                        .iter()
                        .filter_map(|&p| {
                            best(&g, room, nodes, &d, p).map(|(k, l)| {
                                let mut path = vec![p];
                                path.extend(chain(k, &next).iter().map(|&n| g.at[n]));
                                Route { path, length: l }
                            })
                        })
                        .max_by(|a, b| a.length.total_cmp(&b.length))
                })
                .flatten();
            out.rooms.push(RoomEscape {
                room: room.slot,
                level: level.clone(),
                unit: room.unit.clone(),
                exit,
                storey_exits: per.iter().any(|(d, _)| d.iter().any(|v| v.is_finite())),
                entrance,
                has_entrance: !doors.is_empty(),
                passages: doors_of.get(&i).copied().unwrap_or(0),
            });
        }
        // Each dwelling unit's entrance: from its door's outer side to the nearest exit (48/4).
        for (u, _, outer, door, c) in &entrances {
            let Some(o) = *outer else { continue };
            let room = &rooms[g.room[o]];
            if room.owner.is_some() {
                continue;
            }
            let nodes = room_nodes.get(&g.room[o]).unwrap_or(&empty);
            let per = &reached[&None];
            out.doors.push(DoorEscape {
                unit: u.clone(),
                level: level.clone(),
                door: *door,
                at: *c,
                exit: if room.stair {
                    None
                } else {
                    exit_route(room, nodes, per, g.at[o])
                },
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> Room {
        let rings = vec![vec![
            V::new(x0, y0),
            V::new(x1, y0),
            V::new(x1, y1),
            V::new(x0, y1),
        ]];
        let areas = vec![Area {
            outer: kentos_geometry_core::geom::arrangement::Ring {
                pts: rings[0].clone(),
                bulges: None,
            },
            holes: Vec::new(),
        }];
        Room {
            slot: Slot(1),
            level: None,
            unit: None,
            owner: None,
            dwelling: false,
            stair: false,
            rings,
            areas,
            label: V::new((x0 + x1) / 2.0, (y0 + y1) / 2.0),
        }
    }

    #[test]
    fn a_room_s_farthest_points_stand_40_cm_from_its_walls() {
        let r = square(0.0, 0.0, 4.0, 3.0);
        let (convex, inner) = r.corners();
        assert!(inner.is_empty());
        assert_eq!(convex.len(), 4);
        for p in &convex {
            assert!(
                (p.x - 0.4).abs() < 1e-9 || (p.x - 3.6).abs() < 1e-9,
                "{p:?}"
            );
            assert!(
                (p.y - 0.4).abs() < 1e-9 || (p.y - 2.6).abs() < 1e-9,
                "{p:?}"
            );
        }
        // A room narrower than 0.80 m has none.
        assert!(square(0.0, 0.0, 4.0, 0.7).corners().0.is_empty());
    }

    #[test]
    fn an_l_shaped_room_turns_its_walk_40_cm_from_its_inner_corner() {
        // An L: 0..4 × 0..1.2 and 0..1.2 × 0..4; the inner corner at (1.2, 1.2).
        let pts = vec![
            V::new(0.0, 0.0),
            V::new(4.0, 0.0),
            V::new(4.0, 1.2),
            V::new(1.2, 1.2),
            V::new(1.2, 4.0),
            V::new(0.0, 4.0),
        ];
        let mut r = square(0.0, 0.0, 1.0, 1.0);
        r.areas[0].outer.pts = pts.clone();
        r.rings = vec![pts];
        let (_, inner) = r.corners();
        assert_eq!(inner.len(), 1);
        let p = inner[0];
        assert!(
            (dist(p, V::new(1.2, 1.2)) - WALL_CLEAR).abs() < 1e-9,
            "{p:?}"
        );
        assert!(p.x < 1.2 && p.y < 1.2);
        // The far ends do not see each other across the inner corner; each sees the turn.
        let (a, b) = (V::new(3.6, 0.6), V::new(0.6, 3.6));
        assert!(!r.sees(a, b));
        assert!(r.sees(a, p) && r.sees(p, b));
        assert!(
            dwelling_nature(Some("Daire"))
                && !dwelling_nature(Some("Büro"))
                && dwelling_nature(None)
        );
        assert!(
            stair_name(Some("Merdiven evi"), None) && stair_name(Some("Yangın merdiveni"), None)
        );
        assert!(!stair_name(Some("Merdiven altı deposu"), None));
    }
}
