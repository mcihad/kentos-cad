//! `cad.pipes.check` v1 (plan/map-cad.md MAP-16, docs/adr/1640): Boru hattı denetimi, read only. The pipes are the
//! edges the project's utility network takes (its layers and filters, built by the host that has İşlemler's tools,
//! `processing_host::ToolHost::network`); each is checked as a gravity line:
//!
//! - its inverts: two attributes (linear between the drawn ends, by the length along the pipe) or its vertices'
//!   heights (linear between them); its flow is the way it is drawn unless the network's direction says against;
//! - its slope, (upstream − downstream) / length: rising, flat, below the least or above the greatest the user gave;
//! - its diameter, an attribute in a unit, and the catalogue's sizes; a size the catalogue has in another unit is a
//!   likely unit mistake;
//! - its cover over the crown (invert + diameter; the wall is not known and not added): from a terrain surface at every
//!   place the pipe crosses a triangle's edge and at its vertices (the ground and the crown are linear between them, so
//!   the least is exact), or from the structures' cover level at the ends; an invert that would fit the cover limits
//!   read in centimetres or millimetres, or as a depth, is a likely unit mistake;
//! - how it meets the others: a part not connected to the longest, an end at no structure (when the network has
//!   structures), an outlet higher than the lowest inlet at a node (Civil 3D's Pipe Drop Across Structure rule's
//!   question, with no drop asked);
//! - asked for, its clearances to another network's pipes (`pipes_clearance`, docs/adr/2360).
//!
//! Nothing is corrected or written, and no limit is the program's (CLAUDE.md: değer uydurulmaz). The shared cases are
//! `fixtures/commands/v1/cad.pipes.check.json`.

use std::collections::HashMap;

use kentos_commands::{
    CommandResult, Entity, NetworkDef, NetworkKind, PIPE_CATALOG_LIMIT, PIPE_FINDING_LIMIT, PIPE_FINDINGS, PIPE_LIMIT,
    PipeCheck, PipeCount, PipeDiameter, PipeFinding, PipeGround, PipeInverts, PipeUnit, PipesCheck, PipesChecked,
    network_caseless,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::ops::network::{Dir, Graph};
use kentos_road::surface::Surface;

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::pipes_clearance as clearance;
use crate::processing_host::host;
use crate::road_terrains as terrains;

pub(crate) fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// A plain decimal as an attribute holds it (an optional minus, digits, at most one point or comma, digits); none for
/// any other text.
pub fn plain_number(text: &str) -> Option<f64> {
    let t = text.trim();
    let body = t.strip_prefix('-').unwrap_or(t);
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let plain = match body.split_once(['.', ',']) {
        Some((whole, fraction)) => digits(whole) && digits(fraction),
        None => digits(body),
    };
    plain.then(|| t.replace(',', ".").parse::<f64>().ok()).flatten().filter(|v| v.is_finite())
}

pub(crate) fn attr(e: &Entity, field: &str) -> Option<f64> {
    e.base().attrs.get(field).and_then(|v| plain_number(v))
}

/// The question's own rules, before the drawing is read.
fn asked(input: &PipesCheck) -> Result<(), Stop> {
    asked_values(&input.inverts, input.diameter.as_ref(), input.ground.as_ref())?;
    limits_of(input)?;
    match &input.clearance {
        Some(c) => clearance::asked(c),
        None => Ok(()),
    }
}

/// The rules of where the values are read (`cad.pipes.check`'s and `cad.pipes.profile`'s): attributes named, a
/// catalogue of at most 200 sizes above zero, a surface or a cover level's attribute named.
pub(crate) fn asked_values(
    inverts: &PipeInverts,
    diameter: Option<&PipeDiameter>,
    ground: Option<&PipeGround>,
) -> Result<(), Stop> {
    let bad = |message: String, path: &str| Err(refuse(codes::INVALID_PIPE_CHECK, message, path));
    let named = |v: &str| !v.trim().is_empty();
    if let PipeInverts::Fields { start, end } = inverts {
        if !named(start) {
            return bad("Baş akar kotunun özniteliği boş; özniteliğin adını yazın.".into(), "inverts.start");
        }
        if !named(end) {
            return bad("Son akar kotunun özniteliği boş; özniteliğin adını yazın.".into(), "inverts.end");
        }
    }
    if let Some(d) = diameter {
        if !named(&d.field) {
            return bad("Çapın özniteliği boş; özniteliğin adını yazın.".into(), "diameter.field");
        }
        if d.catalog.len() > PIPE_CATALOG_LIMIT {
            return bad(format!("Çap kataloğunda en çok {PIPE_CATALOG_LIMIT} çap olur."), "diameter.catalog");
        }
        if let Some(i) = d.catalog.iter().position(|c| !(c.is_finite() && *c > 0.0)) {
            return bad("Katalogdaki çaplar 0'dan büyük olmalı.".into(), &format!("diameter.catalog[{i}]"));
        }
    }
    match ground {
        Some(PipeGround::Surface { surface }) if !named(surface) => {
            return bad("Zemin yüzeyi boş; yüzeyin adını ya da kimliğini yazın.".into(), "ground.surface");
        }
        Some(PipeGround::Structures { field }) if !named(field) => {
            return bad("Kapak kotunun özniteliği boş; özniteliğin adını yazın.".into(), "ground.field");
        }
        _ => {}
    }
    Ok(())
}

fn limits_of(input: &PipesCheck) -> Result<(), Stop> {
    let bad = |message: String, path: &str| Err(refuse(codes::INVALID_PIPE_CHECK, message, path));
    let limits = [
        (input.min_cover, "minCover", "En az örtü"),
        (input.max_cover, "maxCover", "En çok örtü"),
        (input.min_slope, "minSlope", "En az eğim"),
        (input.max_slope, "maxSlope", "En çok eğim"),
    ];
    for (v, path, words) in limits {
        if let Some(v) = v
            && !(v.is_finite() && v > 0.0)
        {
            return bad(format!("{words} 0'dan büyük bir sayı olmalı; denetlenmeyecekse boş bırakın."), path);
        }
    }
    if let (Some(a), Some(b)) = (input.min_cover, input.max_cover)
        && a > b
    {
        return bad("En az örtü en çok örtüden büyük olamaz.".into(), "maxCover");
    }
    if let (Some(a), Some(b)) = (input.min_slope, input.max_slope)
        && a > b
    {
        return bad("En az eğim en çok eğimden büyük olamaz.".into(), "maxSlope");
    }
    Ok(())
}

/// A pipe's plan path: its vertices, each segment's bulge and the stations (metres along it) of the vertices.
pub(crate) struct Path {
    pub(crate) pts: Vec<[f64; 2]>,
    pub(crate) stations: Vec<f64>,
    pub(crate) heights: Option<Vec<f64>>,
    pub(crate) curved: bool,
}

fn arc_length(chord: f64, bulge: f64) -> f64 {
    if bulge == 0.0 {
        return chord;
    }
    let theta = 4.0 * bulge.abs().atan();
    chord * theta / (2.0 * (theta / 2.0).sin())
}

pub(crate) fn path_of(e: Option<&Entity>, a: [f64; 2], b: [f64; 2]) -> Path {
    let (pts, bulges, heights, other) = match e {
        Some(Entity::Line(l)) => (
            vec![[l.a.x, l.a.y], [l.b.x, l.b.y]],
            vec![0.0],
            l.za.zip(l.zb).map(|(x, y)| vec![x, y]),
            false,
        ),
        Some(Entity::Polyline(p)) if p.pts.len() >= 2 => {
            let n = p.pts.len();
            let bulges: Vec<f64> =
                (0..n - 1).map(|i| p.bulges.as_ref().and_then(|b| b.get(i).copied()).unwrap_or(0.0)).collect();
            let heights = p
                .zs
                .as_ref()
                .filter(|zs| zs.len() == n)
                .and_then(|zs| zs.iter().copied().collect::<Option<Vec<f64>>>());
            (p.pts.iter().map(|v| [v.x, v.y]).collect(), bulges, heights, false)
        }
        _ => (vec![a, b], vec![0.0], None, true),
    };
    let mut stations = vec![0.0];
    for (i, w) in pts.windows(2).enumerate() {
        let chord = (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]);
        stations.push(stations[i] + arc_length(chord, bulges[i]));
    }
    let curved = other || bulges.iter().any(|b| *b != 0.0);
    Path { pts, stations, heights, curved }
}

impl Path {
    pub(crate) fn length(&self) -> f64 {
        *self.stations.last().unwrap_or(&0.0)
    }

    /// The invert at station `s`, by the ends' (fields) or the vertices' heights.
    pub(crate) fn invert(&self, ends: Option<(f64, f64)>, s: f64) -> Option<f64> {
        let l = self.length();
        if let Some((i0, i1)) = ends {
            return Some(if l > 0.0 { i0 + (i1 - i0) * s / l } else { i0 });
        }
        let h = self.heights.as_ref()?;
        let k = self.stations.windows(2).position(|w| s <= w[1]).unwrap_or(self.stations.len().saturating_sub(2));
        let (s0, s1) = (self.stations[k], self.stations[k + 1]);
        Some(if s1 > s0 { h[k] + (h[k + 1] - h[k]) * (s - s0) / (s1 - s0) } else { h[k] })
    }
}

/// A place of a pipe where the cover was measured: its station, place and ground height (none: off the surface).
struct Sample {
    station: f64,
    at: [f64; 2],
    ground: Option<f64>,
}

pub(crate) fn diameter_of(e: Option<&Entity>, d: &PipeDiameter) -> Result<f64, ()> {
    e.and_then(|e| attr(e, &d.field)).filter(|v| *v > 0.0).ok_or(())
}

fn in_catalog(v: f64, catalog: &[f64]) -> bool {
    catalog.iter().any(|c| (v - c).abs() <= 1e-9 * c.abs().max(1.0))
}

struct Ask<'a> {
    input: &'a PipesCheck,
    def: &'a NetworkDef,
    surface: Option<&'a Surface>,
    other: Option<&'a clearance::Other<'a>>,
}

struct Found {
    findings: Vec<(usize, PipeFinding)>,
}

impl Found {
    fn add(&mut self, kind: &str, uid: &str, at: [f64; 2], value: Option<f64>, unit: Option<&str>) {
        let order = PIPE_FINDINGS.iter().position(|k| *k == kind).unwrap_or(PIPE_FINDINGS.len());
        self.findings.push((
            order,
            PipeFinding { kind: kind.into(), uid: uid.into(), at, value, unit: unit.map(str::to_owned) },
        ));
    }
}

/// A utility network's definition by its id or name; refused (`network`) when there is none or it is a road network.
pub(crate) fn network_def(doc: &Document, network: &str) -> Result<NetworkDef, Stop> {
    let key = network_caseless(network);
    let def = doc
        .settings()
        .networks
        .iter()
        .find(|n| n.id == network)
        .or_else(|| doc.settings().networks.iter().find(|n| network_caseless(&n.name) == key))
        .ok_or_else(|| {
            refuse(
                codes::UNKNOWN_NETWORK,
                format!("“{network}” kimlikli ya da adlı ağ projede yok; Ağlar'da tanımlı bir ağı seçin."),
                "network",
            )
        })?
        .clone();
    if def.kind == NetworkKind::Road {
        return Err(refuse(
            codes::NOT_A_PIPE_NETWORK,
            format!("“{}” bir yol ağı; boru hattı denetimi altyapı ağlarını denetler.", def.name),
            "network",
        ));
    }
    Ok(def)
}

/// A utility network asked by its id or name, the terrain surface the ground is read from (made, when asked) and the
/// network built; refused as `cad.pipes.check`'s contract says.
pub(crate) fn load(
    doc: &Document,
    network: &str,
    ground: Option<&PipeGround>,
) -> Result<(NetworkDef, Option<terrains::Made>, kentos_geometry_core::ops::network::NetworkSession), Stop> {
    let def = network_def(doc, network)?;
    let made = match ground {
        Some(PipeGround::Surface { surface }) => {
            terrains::terrain_project(doc)?;
            let id = crate::road_profiles::surface_id(doc, surface, "ground.surface")?;
            let s = terrains::surfaces(doc).iter().find(|s| s.id == id).cloned().ok_or(Stop::Cancelled)?;
            let made = terrains::make(doc, &s);
            made.built()?;
            Some(made)
        }
        _ => None,
    };
    let (_, session) = host()
        .map_err(|e| Stop::failed(*e))
        .and_then(|h| h.network(doc, &def.id).map_err(|why| refuse(codes::INVALID_NETWORK, why, "network")))?;
    Ok((def, made, session))
}

fn run(doc: &Document, input: &PipesCheck) -> Result<PipesChecked, Stop> {
    asked(input)?;
    let (def, made, session) = load(doc, &input.network, input.ground.as_ref())?;
    let surface = match &made {
        Some(m) => Some(m.built()?),
        None => None,
    };
    let g = &session.graph;
    if g.edges.len() > PIPE_LIMIT {
        return Err(refuse(
            codes::INVALID_PIPE_CHECK,
            format!("Ağda {} boru var; en çok {PIPE_LIMIT} boru denetlenir. Ağı süzgeçle bölün.", g.edges.len()),
            "network",
        ));
    }
    let other = match &input.clearance {
        Some(c) => Some(clearance::load(doc, c, &def)?),
        None => None,
    };
    let ready = match (&input.clearance, &other) {
        (Some(c), Some((_, session))) => Some(clearance::Other::new(doc, c, session)),
        _ => None,
    };
    Ok(check(doc, g, &Ask { input, def: &def, surface, other: ready.as_ref() }))
}

fn root(parent: &mut [u32], mut x: u32) -> u32 {
    while parent[x as usize] != x {
        let up = parent[parent[x as usize] as usize];
        parent[x as usize] = up;
        x = up;
    }
    x
}

/// Each edge's part's root and length, and the longest part's root; the parts' count.
fn parts(g: &Graph) -> (Vec<Option<(u32, f64)>>, Option<u32>, usize) {
    let mut parent: Vec<u32> = (0..g.nodes.len() as u32).collect();
    for p in &g.pieces {
        let (a, b) = (root(&mut parent, p.from), root(&mut parent, p.to));
        if a != b {
            parent[a.max(b) as usize] = a.min(b);
        }
    }
    let mut length: HashMap<u32, (f64, usize)> = HashMap::new();
    let mut of_edge: Vec<Option<u32>> = vec![None; g.edges.len()];
    for (k, p) in g.pieces.iter().enumerate() {
        let r = root(&mut parent, p.from);
        let e = length.entry(r).or_insert((0.0, k));
        e.0 += p.len;
        of_edge[p.edge as usize].get_or_insert(r);
    }
    let longest = length.iter().max_by(|a, b| a.1.0.total_cmp(&b.1.0).then(b.1.1.cmp(&a.1.1))).map(|(r, _)| *r);
    let count = length.len();
    (of_edge.into_iter().map(|r| r.map(|r| (r, length[&r].0))).collect(), longest, count)
}

fn check(doc: &Document, g: &Graph, ask: &Ask<'_>) -> PipesChecked {
    let input = ask.input;
    let slot = |id: f64| Slot(id as u32);
    let (part_of, longest, part_count) = parts(g);
    let structures = !ask.def.junctions.is_empty();
    // The junctions at each node, in the graph's order.
    let mut at_node: HashMap<u32, Vec<usize>> = HashMap::new();
    for (k, j) in g.junctions.iter().enumerate() {
        if let Some(n) = j.node {
            at_node.entry(n).or_default().push(k);
        }
    }
    let cover_level = |node: Option<u32>, field: &str| -> Option<f64> {
        at_node.get(&node?)?.iter().find_map(|&k| doc.get(slot(g.junctions[k].id)).and_then(|e| attr(e, field)))
    };
    let mut found = Found { findings: Vec::new() };
    let mut pipes: Vec<PipeCheck> = Vec::new();
    // Each node's inlets and outlets: (pipe, invert there).
    let mut inlets: HashMap<u32, Vec<(usize, f64)>> = HashMap::new();
    let mut outlets: HashMap<u32, Vec<(usize, f64, [f64; 2])>> = HashMap::new();
    let mut total = 0.0;
    for (k, edge) in g.edges.iter().enumerate() {
        let e = doc.get(slot(edge.id));
        let uid = doc.uid(slot(edge.id)).map(|u| u.to_string()).unwrap_or_default();
        let layer = e.map(|e| e.base().layer_id.clone()).unwrap_or_default();
        let a = [edge.a.x, edge.a.y];
        let b = [edge.b.x, edge.b.y];
        let path = path_of(e, a, b);
        let l = path.length();
        total += l;
        let start = path.pts[0];
        let end = *path.pts.last().unwrap_or(&start);
        let reversed = edge.dir == Dir::Backward;
        let mut mark = |kind: &str, at: [f64; 2], value: Option<f64>, unit: Option<&str>| {
            found.add(kind, &uid, at, value, unit)
        };
        // Connection.
        if let (Some((r, len)), Some(top)) = (part_of[k], longest)
            && r != top
        {
            mark("detached", start, Some(len), None);
        }
        if structures {
            let (na, nb) = edge.nodes.unzip();
            for (node, place) in [(na, start), (nb, end)] {
                if node.is_none_or(|n| !at_node.contains_key(&n)) {
                    mark("noStructure", place, None, None);
                }
            }
        }
        // Inverts.
        let ends: Option<(f64, f64)> = match &input.inverts {
            PipeInverts::Fields { start: s, end: t } => e.and_then(|e| attr(e, s).zip(attr(e, t))),
            PipeInverts::Heights => None,
        };
        let known = match &input.inverts {
            PipeInverts::Fields { .. } => ends.is_some(),
            PipeInverts::Heights => path.heights.is_some(),
        };
        if !known {
            mark("noInvert", start, None, None);
        }
        let diameter = input.diameter.as_ref().map(|d| (d, diameter_of(e, d)));
        let d_m: Option<f64> = match diameter {
            Some((d, Ok(v))) => Some(v * d.unit.metres()),
            _ => None,
        };
        // The crown's offset over the invert: the diameter, none when one is asked and the pipe has none.
        let crown: Option<f64> = match diameter {
            None => Some(0.0),
            Some(_) => d_m,
        };
        // The ground at the ends (for the unit checks) and the samples for the cover.
        let mut samples: Vec<Sample> = Vec::new();
        let mut end_ground: [Option<f64>; 2] = [None, None];
        match (&input.ground, ask.surface) {
            (Some(PipeGround::Surface { .. }), Some(s)) => {
                let h = |p: [f64; 2]| s.height(p[0], p[1]).z;
                end_ground = [h(start), h(end)];
                if path.curved {
                    for (p, st) in path.pts.iter().zip(&path.stations) {
                        samples.push(Sample { station: *st, at: *p, ground: h(*p) });
                    }
                } else {
                    for q in s.section(&path.pts) {
                        samples.push(Sample { station: q.station, at: [q.x, q.y], ground: q.z });
                    }
                }
            }
            (Some(PipeGround::Structures { field }), _) => {
                let (na, nb) = edge.nodes.unzip();
                end_ground = [cover_level(na, field), cover_level(nb, field)];
                samples.push(Sample { station: 0.0, at: start, ground: end_ground[0] });
                samples.push(Sample { station: l, at: end, ground: end_ground[1] });
            }
            _ => {}
        }
        // A likely unit mistake in an invert: read as written it misses the cover limits, read in cm or mm (or as a
        // depth) it fits them.
        if let (Some((i0, i1)), Some(lo), Some(hi), Some(c)) = (ends, input.min_cover, input.max_cover, crown) {
            let fits = |cover: f64| (lo..=hi).contains(&cover);
            for (v, g0, place) in [(i0, end_ground[0], start), (i1, end_ground[1], end)] {
                let Some(g0) = g0 else { continue };
                if fits(g0 - (v + c)) {
                    continue;
                }
                if let Some(unit) = [(100.0, "cm"), (1000.0, "mm")].iter().find(|(k, _)| fits(g0 - (v / k + c))) {
                    mark("invertUnit", place, Some(v), Some(unit.1));
                } else if fits(v - c) {
                    mark("invertIsDepth", place, Some(v), None);
                }
            }
        }
        // Slope.
        let (inv0, inv1) = (path.invert(ends, 0.0), path.invert(ends, l));
        let (up, down) = if reversed { (inv1, inv0) } else { (inv0, inv1) };
        let slope = up.zip(down).filter(|_| l > 0.0).map(|(u, d)| (u - d) / l);
        if let Some(sl) = slope {
            if sl < 0.0 {
                mark("reverseSlope", start, Some(sl), None);
            } else if sl == 0.0 {
                mark("flat", start, Some(sl), None);
            } else if input.min_slope.is_some_and(|m| sl < m) {
                mark("lowSlope", start, Some(sl), None);
            }
            if input.max_slope.is_some_and(|m| sl > m) {
                mark("steepSlope", start, Some(sl), None);
            }
        }
        if let (Some((na, nb)), Some(u), Some(d)) = (edge.nodes, up, down) {
            let (from, to, from_at) = if reversed { (nb, na, end) } else { (na, nb, start) };
            outlets.entry(from).or_default().push((k, u, from_at));
            inlets.entry(to).or_default().push((k, d));
        }
        // Diameter.
        if let Some((d, read)) = diameter {
            match read {
                Err(()) => mark("noDiameter", start, None, None),
                Ok(v) if !d.catalog.is_empty() && !in_catalog(v, &d.catalog) => {
                    let other = PipeUnit::ALL
                        .iter()
                        .filter(|u| **u != d.unit)
                        .find(|u| in_catalog(v * u.metres() / d.unit.metres(), &d.catalog));
                    match other {
                        Some(u) => mark("diameterUnit", start, Some(v), Some(u.name())),
                        None => mark("notInCatalog", start, Some(v), None),
                    }
                }
                Ok(_) => {}
            }
        }
        // Cover.
        let mut min_cover: Option<(f64, [f64; 2])> = None;
        let mut max_cover: Option<(f64, [f64; 2])> = None;
        if input.ground.is_some() {
            let on: Vec<&Sample> = samples.iter().filter(|s| s.ground.is_some()).collect();
            if on.is_empty() {
                mark("noGround", start, None, None);
            } else if let Some(off) = samples.iter().find(|s| s.ground.is_none()) {
                let what = if matches!(input.ground, Some(PipeGround::Structures { .. })) { "noGround" } else { "offSurface" };
                mark(what, off.at, None, None);
            }
            if let Some(c) = crown {
                for s in &on {
                    let Some(inv) = path.invert(ends, s.station) else { continue };
                    let cover = s.ground.unwrap_or(f64::NAN) - (inv + c);
                    if min_cover.is_none_or(|m| cover < m.0) {
                        min_cover = Some((cover, s.at));
                    }
                    if max_cover.is_none_or(|m| cover > m.0) {
                        max_cover = Some((cover, s.at));
                    }
                }
            }
            if let Some((v, at)) = min_cover {
                if v < 0.0 {
                    mark("aboveGround", at, Some(v), None);
                } else if input.min_cover.is_some_and(|m| v < m) {
                    mark("shallow", at, Some(v), None);
                }
            }
            if let Some((v, at)) = max_cover
                && input.max_cover.is_some_and(|m| v > m)
            {
                mark("deep", at, Some(v), None);
            }
        }
        if path.curved && input.ground.is_some() {
            mark("curved", start, None, None);
        }
        pipes.push(PipeCheck {
            uid,
            layer,
            length: l,
            reversed,
            upstream: up,
            downstream: down,
            slope,
            diameter: d_m,
            min_cover: min_cover.map(|m| m.0),
            min_cover_at: min_cover.map(|m| m.1),
            max_cover: max_cover.map(|m| m.0),
            findings: Vec::new(),
        });
    }
    // An outlet higher than the lowest inlet at its node.
    let mut nodes: Vec<&u32> = outlets.keys().collect();
    nodes.sort();
    for n in nodes {
        let Some(low) = inlets.get(n).and_then(|list| list.iter().map(|x| x.1).reduce(f64::min)) else { continue };
        for &(k, o, at) in &outlets[n] {
            if o > low {
                let uid = pipes[k].uid.clone();
                found.add("outletAbove", &uid, at, Some(o - low), None);
            }
        }
    }
    // The clearances to the other network's pipes (docs/adr/2360).
    let measured = ask.other.map(|o| clearance::measure(doc, g, &input.inverts, input.diameter.as_ref(), o));
    if let Some(m) = &measured {
        for mark in &m.marks {
            found.add(mark.kind, &mark.uid, mark.at, mark.value, None);
        }
    }
    let (crossings, near) = measured.map(|m| (m.crossings, m.near)).unwrap_or_default();
    // Pipe by pipe, each pipe's in the kinds' order (a kind twice: in the order found).
    let index: HashMap<String, usize> = pipes.iter().enumerate().map(|(k, p)| (p.uid.clone(), k)).collect();
    let mut list = found.findings;
    list.sort_by_key(|(order, f)| (index.get(&f.uid).copied().unwrap_or(usize::MAX), *order));
    let mut counts: Vec<PipeCount> = Vec::new();
    for kind in PIPE_FINDINGS {
        let n = list.iter().filter(|(_, f)| f.kind == kind).count();
        if n > 0 {
            counts.push(PipeCount { kind: kind.into(), count: n.min(u32::MAX as usize) as u32 });
        }
    }
    for (_, f) in &list {
        if let Some(&k) = index.get(&f.uid)
            && !pipes[k].findings.contains(&f.kind)
        {
            pipes[k].findings.push(f.kind.clone());
        }
    }
    PipesChecked {
        network: ask.def.id.clone(),
        cover_to: if input.diameter.is_some() { "crown" } else { "invert" }.into(),
        pipes,
        findings: list.into_iter().map(|(_, f)| f).take(PIPE_FINDING_LIMIT).collect(),
        counts,
        parts: part_count.min(u32::MAX as usize) as u32,
        length: total,
        crossings,
        near,
    }
}

/// Checks the question and the network, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &PipesCheck) -> CommandResult<()> {
    match run(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The check; the same as execute.
pub fn plan(cx: &ExecutionContext<'_>, input: &PipesCheck) -> CommandResult<PipesChecked> {
    match run(cx.doc, input) {
        Ok(out) => CommandResult::Completed { output: out, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The check, read only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: PipesCheck) -> CommandResult<PipesChecked> {
    plan(cx, &input)
}
