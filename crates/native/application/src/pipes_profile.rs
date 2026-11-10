//! `cad.pipes.profile` v1 (plan/map-cad.md MAP-16, docs/adr/1860): Hat profili, a gravity line's longitudinal profile
//! (kanal boykesiti). The run starts at a pipe of a utility network and follows the flow (the way a pipe is drawn
//! unless the network's direction says against) through every node exactly one pipe leaves; it ends at the pipe asked,
//! where no pipe leaves (the outlet) or where more than one does (a split). The values are `cad.pipes.check`'s
//! (`pipes_check`'s inverts, diameter, ground and cover): the ground at every triangle edge a straight pipe crosses (a
//! curved pipe at its vertices) or the structures' cover levels at the nodes; each pipe's inverts up and down the flow,
//! its slope, diameter and least cover; at each node the structure, the ground, the run's inverts in and out and the
//! depth. It is drawn as `road.profiles.draw` draws a road's (docs/adr/0902 §8): the frame from the datum to the top,
//! the elevation grid with its heights on both sides, the ground (black), the inverts (red, 0.5) and the crowns (red,
//! 0.25), each structure from its lowest invert to the ground, and under the frame the bands (`PIPE_PROFILE_BANDS`): a
//! tick through them at every node, each node's values at it and each pipe's at its middle. Asked (docs/adr/2760),
//! another network's pipes crossing the run in plan (the clearances' crossings) are drawn where they cross: an ellipse
//! of the diameter, its height times the exaggeration, the name and invert above; with its wall thickness asked and read
//! (docs/adr/4360), its outer wall about the same centre and the ring between the two filled. The run's own pipes
//! whose wall is asked and known above zero (docs/adr/4780: their wall attribute or their outer diameter's, as the
//! check reads them) have their outer walls drawn too, thin red lines the wall under the invert and over the crown
//! (Civil 3D's “Outside Pipe Walls”), and their cover is over the top outside. One undo step “Hat profili” through
//! `cad.entities.create`'s write. The shared cases are `fixtures/commands/v1/cad.pipes.profile.json`.

use std::collections::{HashMap, HashSet};

use kentos_commands::{
    CommandResult, CommandWarning, CreateOperation, EntitiesCreate, EntityGeometry, NewObject, PIPE_PROFILE_BANDS,
    PIPE_LIMIT, PIPE_PROFILE_LIMIT, PipeClearanceAsk, PipeGround, PipeInverts, PipeProfileCrossing, PipeProfileGround,
    PipeProfilePipe, PipeProfileStructure, PipeRunEnd, PipesProfile, PipesProfiled, TextAlign, Vec2,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::ops::network::Dir;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::pipes_check::{self as pipes, attr, path_of, wall_of};
use crate::pipes_clearance;
use crate::road_profiles_draw::{grid_step, line, masked, turned};

/// The design's red (as `road.profiles.draw`'s).
const RED: &str = "#d62828";

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

struct Checked {
    out: PipesProfiled,
    /// The inverts' points along the run (station, invert, crown, wall above zero): each pipe's ends, or its vertices
    /// when its vertices' heights are the inverts; a pipe without inverts a gap.
    inverts: Vec<(f64, Option<f64>, Option<f64>, Option<f64>)>,
    /// Texts' paper height (mm).
    height: f64,
    warnings: Vec<CommandWarning>,
}

/// The question's own rules: where the values are read, and the drawing's sizes.
fn asked(input: &PipesProfile) -> Result<(), Stop> {
    pipes::asked_values(&input.inverts, input.diameter.as_ref(), input.ground.as_ref())?;
    // The crossing network's values, read as the run's (docs/adr/2760).
    if let Some(c) = &input.crossing {
        pipes::asked_values(&c.inverts, c.diameter.as_ref(), None).map_err(|stop| match stop {
            Stop::Failed(mut e) => {
                e.path = e.path.take().map(|p| format!("crossing.{p}"));
                Stop::Failed(e)
            }
            other => other,
        })?;
    }
    let bad = |message: &str, path: &str| Err(refuse(codes::INVALID_PIPE_CHECK, message.into(), path));
    if input.exaggeration.is_some_and(|e| !(e.is_finite() && e > 0.0)) {
        return bad("Düşey abartı 0'dan büyük bir sayı olmalı (yatay ölçeğin kaç katı).", "exaggeration");
    }
    if input.height.is_some_and(|h| !(h.is_finite() && h > 0.0)) {
        return bad("Yazı boyu 0'dan büyük bir sayı olmalı (kâğıtta mm).", "height");
    }
    if input.datum.is_some_and(|d| !d.is_finite()) {
        return bad("Kıyas kotu (datum) sonlu bir sayı olmalı.", "datum");
    }
    Ok(())
}

fn check(doc: &Document, input: &PipesProfile) -> Result<Checked, Stop> {
    asked(input)?;
    let (def, made, session) = pipes::load(doc, &input.network, input.ground.as_ref())?;
    let surface = match &made {
        Some(m) => Some(m.built()?),
        None => None,
    };
    // The crossing network (docs/adr/2760): its pipes made ready as the clearances take them.
    let crossing = match &input.crossing {
        Some(c) => Some((
            pipes_clearance::load_other(
                doc,
                &c.network,
                &def,
                "crossing.network",
                |name| format!("Kesişen borular başka bir ağdan okunur; “{name}” profili çizilen ağın kendisi."),
                |name, n| {
                    format!("“{name}” ağında {n} boru var; en çok {PIPE_LIMIT} borunun kesişimi aranır. Ağı süzgeçle bölün.")
                },
            )?,
            PipeClearanceAsk {
                network: c.network.clone(),
                inverts: c.inverts.clone(),
                diameter: c.diameter.clone(),
                vertical: None,
                horizontal: None,
                above: None,
            },
        )),
        None => None,
    };
    let other = crossing.as_ref().map(|((_, session), ask)| pipes_clearance::Other::new(doc, ask, session));
    let mut crossings: Vec<PipeProfileCrossing> = Vec::new();
    let g = &session.graph;
    let slot = |id: f64| Slot(id as u32);
    let uid_of = |k: usize| doc.uid(slot(g.edges[k].id)).map(|u| u.to_string()).unwrap_or_default();
    let broken = |message: String, path: &str| refuse(codes::INVALID_PIPE_RUN, message, path);
    let find = |uid: &str| (0..g.edges.len()).find(|&k| uid_of(k) == uid);
    let first = find(&input.start)
        .ok_or_else(|| broken(format!("“{}” “{}” ağının bir borusu değil.", input.start, def.name), "start"))?;
    let last = match &input.end {
        Some(e) => Some(find(e).ok_or_else(|| broken(format!("“{e}” “{}” ağının bir borusu değil.", def.name), "end"))?),
        None => None,
    };
    // Each pipe's nodes the way it flows; the pipes leaving each node.
    let flow = |k: usize| g.edges[k].nodes.map(|(a, b)| if g.edges[k].dir == Dir::Backward { (b, a) } else { (a, b) });
    let mut leaving: HashMap<u32, Vec<usize>> = HashMap::new();
    for k in 0..g.edges.len() {
        if let Some((a, _)) = flow(k) {
            leaving.entry(a).or_default().push(k);
        }
    }
    let mut run = vec![first];
    let mut seen = HashSet::from([first]);
    let ends = loop {
        let at = run[run.len() - 1];
        if Some(at) == last {
            break PipeRunEnd::End;
        }
        let Some((_, to)) = flow(at) else { break PipeRunEnd::Outlet };
        let next: Vec<usize> = leaving.get(&to).map_or_else(Vec::new, |v| v.iter().copied().filter(|k| !seen.contains(k)).collect());
        match next.as_slice() {
            [] => break PipeRunEnd::Outlet,
            [k] => {
                if run.len() >= PIPE_PROFILE_LIMIT {
                    return Err(broken(format!("Hat {PIPE_PROFILE_LIMIT} borudan uzun; bir bitiş borusu (end) verin."), "end"));
                }
                seen.insert(*k);
                run.push(*k);
            }
            _ => break PipeRunEnd::Split,
        }
    };
    if let (Some(l), Some(e)) = (last, &input.end)
        && run[run.len() - 1] != l
    {
        return Err(broken(format!("“{e}” borusu akış yönünde “{}” borusunun aşağısında değil.", input.start), "end"));
    }
    // The junctions at each node.
    let mut at_node: HashMap<u32, Vec<usize>> = HashMap::new();
    for (k, j) in g.junctions.iter().enumerate() {
        if let Some(n) = j.node {
            at_node.entry(n).or_default().push(k);
        }
    }
    let junction = |node: Option<u32>| node.and_then(|n| at_node.get(&n)).and_then(|v| v.first()).map(|&k| &g.junctions[k]);
    // Pipe by pipe, the way they flow.
    let mut out_pipes: Vec<PipeProfilePipe> = Vec::new();
    let mut ground: Vec<PipeProfileGround> = Vec::new();
    // Each node's place and node (first the run's start), and the invert polylines' points.
    let mut nodes: Vec<([f64; 2], Option<u32>)> = Vec::new();
    let mut inverts: Vec<(f64, Option<f64>, Option<f64>, Option<f64>)> = Vec::new();
    let mut s0 = 0.0;
    for (i, &k) in run.iter().enumerate() {
        let edge = &g.edges[k];
        let e = doc.get(slot(edge.id));
        let path = path_of(e, [edge.a.x, edge.a.y], [edge.b.x, edge.b.y]);
        let l = path.length();
        let reversed = edge.dir == Dir::Backward;
        let (na, nb) = edge.nodes.unzip();
        let (start, end) = (path.pts[0], *path.pts.last().unwrap_or(&path.pts[0]));
        let ((up_at, up_node), (down_at, down_node)) = if reversed { ((end, nb), (start, na)) } else { ((start, na), (end, nb)) };
        if i == 0 {
            nodes.push((up_at, up_node));
        }
        nodes.push((down_at, down_node));
        let ends_read: Option<(f64, f64)> = match &input.inverts {
            PipeInverts::Fields { start, end } => e.and_then(|e| attr(e, start).zip(attr(e, end))),
            PipeInverts::Heights => None,
        };
        // The station along the run of a station along the drawn pipe.
        let along = |st: f64| s0 + if reversed { l - st } else { st };
        if let Some(o) = &other {
            for x in pipes_clearance::crossing_pipes(e, &path, o) {
                crossings.push(PipeProfileCrossing {
                    uid: x.uid,
                    name: x.name,
                    station: along(x.station),
                    at: x.at,
                    invert: x.invert,
                    diameter: x.radius.filter(|r| *r > 0.0).map(|r| 2.0 * r),
                    wall: x.wall,
                });
            }
        }
        let (inv0, inv1) = (path.invert(ends_read, 0.0), path.invert(ends_read, l));
        let (up, down) = if reversed { (inv1, inv0) } else { (inv0, inv1) };
        let slope = up.zip(down).filter(|_| l > 0.0).map(|(u, d)| (u - d) / l);
        let diameter = input.diameter.as_ref().and_then(|d| pipes::diameter_of(e, d).ok().map(|v| v * d.unit.metres()));
        let crown: Option<f64> = match &input.diameter {
            None => Some(0.0),
            Some(_) => diameter,
        };
        // Docs/adr/4780: its wall as the check reads it (its attribute or its outer diameter's); the cover over its top
        // outside, its outer walls drawn when above zero.
        let wall = input.diameter.as_ref().and_then(|d| wall_of(e, d)).and_then(Result::ok);
        let top_outside = crown.map(|c| c + wall.unwrap_or(0.0));
        let drawn_wall = wall.filter(|w| *w > 0.0);
        // The inverts' points: the ends, and a polyline's vertices when its heights are the inverts.
        let mut points: Vec<(f64, f64)> = match (&input.inverts, &path.heights) {
            (PipeInverts::Heights, Some(h)) => path.stations.iter().zip(h).map(|(st, z)| (along(*st), *z)).collect(),
            _ => [(0.0, inv0), (l, inv1)].into_iter().filter_map(|(st, z)| z.map(|z| (along(st), z))).collect(),
        };
        points.sort_by(|a, b| a.0.total_cmp(&b.0));
        if points.is_empty() {
            inverts.push((s0, None, None, None));
        }
        for (s, z) in points {
            inverts.push((s, Some(z), crown.map(|c| z + c), drawn_wall));
        }
        // The ground along it, the way it flows, and the least cover over it.
        let mut samples: Vec<(f64, [f64; 2], Option<f64>)> = Vec::new();
        match (&input.ground, surface) {
            (Some(PipeGround::Surface { .. }), Some(s)) => {
                if path.curved {
                    for (p, st) in path.pts.iter().zip(&path.stations) {
                        samples.push((*st, *p, s.height(p[0], p[1]).z));
                    }
                } else {
                    for q in s.section(&path.pts) {
                        samples.push((q.station, [q.x, q.y], q.z));
                    }
                }
            }
            (Some(PipeGround::Structures { field }), _) => {
                let level = |node: Option<u32>| junction(node).and_then(|j| doc.get(slot(j.id))).and_then(|e| attr(e, field));
                samples.push((0.0, start, level(na)));
                samples.push((l, end, level(nb)));
            }
            _ => {}
        }
        let mut min_cover: Option<f64> = None;
        if let Some(c) = top_outside {
            for (st, _, z) in &samples {
                if let (Some(z), Some(inv)) = (z, path.invert(ends_read, *st)) {
                    let cover = z - (inv + c);
                    min_cover = Some(min_cover.map_or(cover, |m: f64| m.min(cover)));
                }
            }
        }
        if matches!(input.ground, Some(PipeGround::Surface { .. })) {
            let mut flowing: Vec<(f64, Option<f64>)> = samples.iter().map(|(st, _, z)| (along(*st), *z)).collect();
            flowing.sort_by(|a, b| a.0.total_cmp(&b.0));
            for (s, z) in flowing {
                if ground.last().is_none_or(|p| (p.station - s).abs() > 1e-9) {
                    ground.push(PipeProfileGround { station: s, z });
                }
            }
        }
        out_pipes.push(PipeProfilePipe {
            uid: uid_of(k),
            name: e.and_then(|e| e.base().attrs.get("Ad").cloned()),
            from: s0,
            to: s0 + l,
            length: l,
            reversed,
            upstream: up,
            downstream: down,
            slope,
            diameter,
            wall,
            min_cover,
        });
        s0 += l;
    }
    // The nodes: the structure there, the ground, the run's inverts in and out, the depth.
    let structures: Vec<PipeProfileStructure> = nodes
        .iter()
        .enumerate()
        .map(|(i, (place, node))| {
            let j = junction(*node);
            let je = j.and_then(|j| doc.get(slot(j.id)));
            let station = if i == 0 { 0.0 } else { out_pipes[i - 1].to };
            let ground = match (&input.ground, surface) {
                (Some(PipeGround::Surface { .. }), Some(s)) => s.height(place[0], place[1]).z,
                (Some(PipeGround::Structures { field }), _) => je.and_then(|e| attr(e, field)),
                _ => None,
            };
            let inlet = (i > 0).then(|| out_pipes[i - 1].downstream).flatten();
            let outlet = out_pipes.get(i).and_then(|p| p.upstream);
            let low = [inlet, outlet].into_iter().flatten().reduce(f64::min);
            PipeProfileStructure {
                station,
                uid: j.and_then(|j| doc.uid(slot(j.id))).map(|u| u.to_string()),
                name: je.and_then(|e| e.base().attrs.get("Ad").cloned()),
                ground,
                inlet,
                outlet,
                depth: ground.zip(low).map(|(g, l)| g - l),
            }
        })
        .collect();
    if matches!(input.ground, Some(PipeGround::Structures { .. })) {
        ground = structures.iter().map(|s| PipeProfileGround { station: s.station, z: s.ground }).collect();
    }
    // The crossing pipes along the run, then by their uid (docs/adr/2760).
    crossings.sort_by(|a, b| a.station.total_cmp(&b.station).then_with(|| a.uid.cmp(&b.uid)));
    // The elevations shown, the datum and the top: the crossing pipes' inverts and tops too.
    let mut zs: Vec<f64> = ground.iter().filter_map(|p| p.z).collect();
    zs.extend(inverts.iter().flat_map(|(_, i, c, _)| [*i, *c]).flatten());
    // Docs/adr/4780: the run's outer walls, the bottom outside and the top outside.
    for (_, i, c, w) in &inverts {
        if let (Some(i), Some(w)) = (i, w) {
            zs.push(i - w);
            zs.extend(c.map(|c| c + w));
        }
    }
    zs.extend(structures.iter().filter_map(|s| s.ground));
    for c in &crossings {
        if let Some(i) = c.invert {
            zs.push(i);
            zs.extend(c.diameter.map(|d| i + d));
            // Docs/adr/4360: the outer wall's bottom and top.
            if let (Some(d), Some(w)) = (c.diameter, c.wall) {
                zs.extend([i - w, i + d + w]);
            }
        }
    }
    let (lo, hi) = zs.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), z| (lo.min(*z), hi.max(*z)));
    if !lo.is_finite() {
        return Err(broken(
            "Hatta çizilecek kot yok: borularda akar kotu ve zemin okunamadı (öznitelik adlarını ya da zemini denetleyin).".into(),
            "inverts",
        ));
    }
    let datum = match input.datum {
        Some(d) if d > lo + 1e-9 => {
            return Err(refuse(
                codes::INVALID_PIPE_CHECK,
                format!("Kıyas kotu (datum) en alçak kottan ({lo:.2}) yüksek olamaz."),
                "datum",
            ));
        }
        Some(d) => d,
        None => lo.floor() - 1.0,
    };
    let warnings = checks::layer(doc, &input.layer_id)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Checked {
        out: PipesProfiled {
            network: def.id.clone(),
            pipes: out_pipes,
            structures,
            ground,
            length: s0,
            ends,
            datum,
            top: hi.ceil() + 1.0,
            exaggeration: input.exaggeration.unwrap_or(10.0),
            crossings,
            uids: Vec::new(),
            revision: doc.revision().to_string(),
        },
        inverts,
        height: input.height.unwrap_or(2.5),
        warnings,
    })
}

fn polyline(pts: &[[f64; 2]], closed: bool, color: Option<&str>, weight: f64) -> NewObject {
    NewObject {
        geometry: EntityGeometry::Polyline {
            pts: pts.iter().map(|p| Vec2 { x: p[0], y: p[1] }).collect(),
            bulges: None,
            zs: None,
            ms: None,
            parts: None,
            closed,
            widths: None,
        },
        color: color.map(str::to_owned),
        line_weight: Some(weight),
        line_type: None,
        line_pattern: None,
        attrs: None,
        label: None,
        label_of: None,
        label_scale: None,
        symbol: None,
        order: None,
        transparency: None,
    }
}

/// An object drawn as it is, the layer's look.
fn drawn(geometry: EntityGeometry) -> NewObject {
    NewObject {
        geometry,
        color: None,
        line_weight: None,
        line_type: None,
        line_pattern: None,
        attrs: None,
        label: None,
        label_of: None,
        label_scale: None,
        symbol: None,
        order: None,
        transparency: None,
    }
}

/// The runs of consecutive known points (each of two or more).
fn runs(points: impl Iterator<Item = (f64, Option<f64>)>) -> Vec<Vec<(f64, f64)>> {
    let mut out: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut open = false;
    for (s, z) in points {
        match z {
            Some(z) => {
                if !open {
                    out.push(Vec::new());
                    open = true;
                }
                if let Some(r) = out.last_mut() {
                    r.push((s, z));
                }
            }
            None => open = false,
        }
    }
    out.retain(|r| r.len() >= 2);
    out
}

/// The heights the grid marks: every step from the datum's up to the top.
pub(crate) fn grid_heights(datum: f64, top: f64) -> Vec<f64> {
    let g = grid_step(datum, top);
    let mut k = (datum / g).ceil();
    let mut out = Vec::new();
    while k * g <= top + 1e-9 {
        out.push(k * g);
        k += 1.0;
    }
    out
}

fn objects(doc: &Document, c: &Checked, at: [f64; 2], crowns: bool) -> Vec<NewObject> {
    let o = &c.out;
    let inverts = &c.inverts;
    let mm = doc.settings().plot_scale / 1000.0;
    let h = c.height * mm;
    let row = 2.4 * h;
    let names = 14.0 * h;
    let x = |s: f64| at[0] + s;
    let y = |z: f64| at[1] + (z - o.datum) * o.exaggeration;
    let (x0, x1, y0, y1) = (x(0.0), x(o.length), y(o.datum), y(o.top));
    let text = |p: [f64; 2], words: String, align: TextAlign| turned(p, words, h, align, None, 0.0);
    let mut out = vec![polyline(&[[x0, y0], [x1, y0], [x1, y1], [x0, y1]], true, None, 0.35)];
    // The grid and its heights on both sides.
    for v in grid_heights(o.datum, o.top) {
        let yv = y(v);
        if v > o.datum + 1e-9 && v < o.top - 1e-9 {
            out.push(line([x0, yv], [x1, yv], None, Some(0.13)));
        }
        out.push(text([x0 - h, yv], format!("{v:.2}"), TextAlign::MiddleRight));
        out.push(text([x1 + h, yv], format!("{v:.2}"), TextAlign::MiddleLeft));
    }
    // The ground, the inverts and the crowns.
    for r in runs(o.ground.iter().map(|p| (p.station, p.z))) {
        out.push(polyline(&r.iter().map(|(s, z)| [x(*s), y(*z)]).collect::<Vec<_>>(), false, None, 0.25));
    }
    for r in runs(inverts.iter().map(|(s, i, _, _)| (*s, *i))) {
        out.push(polyline(&r.iter().map(|(s, z)| [x(*s), y(*z)]).collect::<Vec<_>>(), false, Some(RED), 0.5));
    }
    if crowns {
        for r in runs(inverts.iter().map(|(s, _, c, _)| (*s, *c))) {
            out.push(polyline(&r.iter().map(|(s, z)| [x(*s), y(*z)]).collect::<Vec<_>>(), false, Some(RED), 0.25));
        }
        // Docs/adr/4780: the outer walls (Civil 3D's “Outside Pipe Walls”), the wall under the invert and over the
        // crown, thin; a pipe whose wall is not known above zero a gap.
        let bottom = runs(inverts.iter().map(|(s, i, _, w)| (*s, i.zip(*w).map(|(i, w)| i - w))));
        let top = runs(inverts.iter().map(|(s, _, c, w)| (*s, c.zip(*w).map(|(c, w)| c + w))));
        for r in bottom.into_iter().chain(top) {
            out.push(polyline(&r.iter().map(|(s, z)| [x(*s), y(*z)]).collect::<Vec<_>>(), false, Some(RED), 0.13));
        }
    }
    // Each structure from its lowest invert to the ground.
    for s in &o.structures {
        let low = [s.inlet, s.outlet].into_iter().flatten().reduce(f64::min);
        if let (Some(low), Some(g)) = (low, s.ground) {
            out.push(line([x(s.station), y(low)], [x(s.station), y(g)], None, Some(0.35)));
        }
    }
    // The crossing pipes (docs/adr/2760): as Civil 3D draws a crossing pipe in a profile view, an ellipse of its diameter
    // across and its diameter times the exaggeration high from its invert up (a point at its invert without a
    // diameter), its name and invert above it, masked over the lines.
    for c in &o.crossings {
        let Some(inv) = c.invert else { continue };
        let xs = x(c.station);
        let top = match c.diameter {
            Some(d) => {
                let centre = Vec2 { x: xs, y: y(inv) + d / 2.0 * o.exaggeration };
                let (major, ratio) = axes(d / 2.0, o.exaggeration);
                out.push(drawn(EntityGeometry::Ellipse { c: centre, major, ratio, t0: 0.0, t1: std::f64::consts::TAU }));
                match c.wall.filter(|w| *w > 0.0) {
                    // Docs/adr/4360: the outer wall about the same centre and the ring between the two filled.
                    Some(w) => {
                        let (outer, outer_ratio) = axes(d / 2.0 + w, o.exaggeration);
                        out.push(drawn(EntityGeometry::Ellipse { c: centre, major: outer, ratio: outer_ratio, t0: 0.0, t1: std::f64::consts::TAU }));
                        out.push(drawn(wall_ring(centre, (outer, outer_ratio), (major, ratio))));
                        centre.y + (d / 2.0 + w) * o.exaggeration
                    }
                    None => centre.y + d / 2.0 * o.exaggeration,
                }
            }
            None => {
                out.push(drawn(EntityGeometry::Point { p: Vec2 { x: xs, y: y(inv) }, z: None, parts: None }));
                y(inv)
            }
        };
        let words = match &c.name {
            Some(n) => format!("{n} {inv:.2}"),
            None => format!("{inv:.2}"),
        };
        out.push(masked(text([xs, top + 0.5 * h], words, TextAlign::BottomCenter)));
    }
    // The bands: their lines, a tick through them at every node, their names.
    let n = PIPE_PROFILE_BANDS.len() as f64;
    let bottom = y0 - n * row;
    for k in 0..=PIPE_PROFILE_BANDS.len() {
        let yk = y0 - row * k as f64;
        out.push(line([x0 - names, yk], [x1, yk], None, Some(0.25)));
    }
    for xv in [x0 - names, x0, x1] {
        out.push(line([xv, y0], [xv, bottom], None, Some(0.25)));
    }
    for s in &o.structures {
        out.push(line([x(s.station), y0], [x(s.station), bottom], None, Some(0.13)));
    }
    let middle = |k: usize| y0 - row * (k as f64 + 0.5);
    for (k, name) in PIPE_PROFILE_BANDS.iter().enumerate() {
        out.push(text([x0 - names + 0.5 * h, middle(k)], (*name).to_owned(), TextAlign::MiddleLeft));
    }
    // Each node's values at it (masked: the tick runs through them), each pipe's at its middle.
    for s in &o.structures {
        let cells: [Option<String>; 6] = [
            s.name.clone(),
            Some(format!("{:.2}", s.station)),
            s.ground.map(|v| format!("{v:.2}")),
            s.inlet.map(|v| format!("{v:.2}")),
            s.outlet.map(|v| format!("{v:.2}")),
            s.depth.map(|v| format!("{v:.2}")),
        ];
        for (k, words) in cells.into_iter().enumerate() {
            if let Some(words) = words {
                out.push(masked(text([x(s.station), middle(k)], words, TextAlign::MiddleCenter)));
            }
        }
    }
    for p in &o.pipes {
        let cells: [Option<String>; 3] = [
            p.slope.map(|v| format!("{:.2}", v * 1000.0)),
            p.diameter.map(|v| format!("{:.0}", v * 1000.0)),
            Some(format!("{:.2}", p.length)),
        ];
        for (k, words) in cells.into_iter().enumerate() {
            if let Some(words) = words {
                out.push(text([x(0.5 * (p.from + p.to)), middle(6 + k)], words, TextAlign::MiddleCenter));
            }
        }
    }
    out
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &PipesProfile) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: (), warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// The profile execute would draw; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &PipesProfile) -> CommandResult<PipesProfiled> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: c.out, warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// Writes the profile's objects in the step it runs in (`road.project.rebuild` draws a kept profile again with it).
pub(crate) fn draw(cx: &mut ExecutionContext<'_>, input: &PipesProfile) -> CommandResult<PipesProfiled> {
    let c = match check(cx.doc, input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let create = EntitiesCreate {
        layer_id: input.layer_id.clone(),
        objects: objects(cx.doc, &c, input.at, input.diameter.is_some()),
        operation: Some(CreateOperation::PipeProfile),
        expected_revision: None,
    };
    match crate::create::write_in(cx, create) {
        CommandResult::Completed { output, warnings } => CommandResult::Completed {
            output: PipesProfiled { uids: output.created, revision: output.revision, ..c.out },
            warnings,
        },
        other => crate::trim::relay(other),
    }
}

/// Draws the profile, one undo step “Hat profili”. In a Harita or Yol project the drawing is kept with its network
/// (docs/adr/3560; the surface its ground is read from and the crossing network too): the project's tree says when a
/// pipe or a structure changed since, and `road.project.rebuild` (or drawing again by itself) draws it again.
pub fn execute(cx: &mut ExecutionContext<'_>, input: PipesProfile) -> CommandResult<PipesProfiled> {
    crate::call::recorded(cx, kentos_commands::CAD_PIPES_PROFILE, kentos_commands::CAD_PIPES_PROFILE_VERSION, input, |cx, input| {
        let r = draw(cx, &input);
        if !crate::road_terrains::holds_surfaces(cx.doc.settings().cad_work) {
            return r;
        }
        crate::road_drawings::kept(
            cx,
            kentos_commands::CAD_PIPES_PROFILE,
            &input,
            (kentos_commands::RoadSourceKind::Network, "network"),
            r,
            |o| &o.uids,
        )
    })
}

/// An ellipse `r` across and `r · exaggeration` high as its major semi-axis and ratio (the longer axis the major).
fn axes(r: f64, exaggeration: f64) -> (Vec2, f64) {
    let (rx, ry) = (r, r * exaggeration);
    if ry >= rx { (Vec2 { x: 0.0, y: ry }, rx / ry) } else { (Vec2 { x: rx, y: 0.0 }, ry / rx) }
}

/// The ring between a crossing pipe's outer and inner walls, filled (docs/adr/4360): its boundary exact, the outer
/// ellipse then the inner one as a hole, each a loop of one point that is one closed curve.
fn wall_ring(c: Vec2, outer: (Vec2, f64), inner: (Vec2, f64)) -> EntityGeometry {
    let whole = |(major, ratio): (Vec2, f64)| kentos_commands::HatchLoop {
        pts: vec![Vec2 { x: c.x + major.x, y: c.y + major.y }],
        bulges: None,
        curves: Some(vec![Some(kentos_commands::HatchCurve::Ellipse { c, major, ratio, t0: 0.0, t1: std::f64::consts::TAU })]),
    };
    EntityGeometry::Hatch {
        ring: Vec::new(),
        holes: None,
        pattern: kentos_commands::HatchPattern::user(kentos_commands::HatchPatternType::Solid, 0.0, 1.0),
        assoc: None,
        boundary: Some(vec![whole(outer), whole(inner)]),
        annotative: None,
    }
}
