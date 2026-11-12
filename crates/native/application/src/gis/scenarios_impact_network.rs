//! Ağ üzerinden erişim (docs/adr/5060; plan/map-cad.md MAP-10): `cad.scenarios.impact`'s access along a network of the
//! project instead of straight. The network is the project's own (Ağlar: its layers, tolerance, connection and
//! direction), built by the host that has İşlemler's tools (`processing_host::ToolHost::network_of`); a layer of it the
//! scenario stands in for is the scenario's on the alternative's side. Each area's centroid joins the network at its
//! nearest place (`Graph::locate`) when that is no farther than the user's `connect`, else the area is off the network;
//! a `from` area reaches when its leg, the shortest way along the network (cost 0, the length) and the `to` area's leg
//! come to the distance at most; of equal ways the first `to` area in the drawing's order. The legs are counted, not
//! snapped away, and said.

use kentos_commands::{ImpactAccess, ImpactRoute, NetworkDef, network_caseless};
use kentos_domain::Document;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::centroid::shape_centroid;
use kentos_geometry_core::ops::network::closest::{Nearest, nearest};
use kentos_geometry_core::ops::network::search::{polyline_of, spans_edges};
use kentos_geometry_core::ops::network::{Graph, Location, Searcher};

use crate::checks::{Stop, error};
use crate::codes;
use crate::processing_host::host;

/// An area as the measures read it: its shape, use, area and persistent id.
pub(crate) type MeasuredArea = (Shape, String, f64, String);

/// What the access along the network found on a side.
#[derive(Default)]
pub(crate) struct Along {
    pub got: f64,
    pub whole: f64,
    pub reached: Vec<String>,
    pub unreached: Vec<String>,
    pub targets: Vec<String>,
    pub routes: Vec<ImpactRoute>,
    /// The `from` areas off the network, and how many `to` areas are.
    pub off: Vec<String>,
    pub off_targets: usize,
}

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The project's network by its id or name (any kind: a road's, a footpath's or a utility's).
fn definition(doc: &Document, name: &str) -> Result<NetworkDef, Stop> {
    let nets = &doc.settings().networks;
    let key = network_caseless(name);
    nets.iter()
        .find(|n| n.id == name.trim())
        .or_else(|| nets.iter().find(|n| network_caseless(&n.name) == key))
        .cloned()
        .ok_or_else(|| {
            refuse(
                codes::UNKNOWN_NETWORK,
                format!("“{}” kimlikli ya da adlı ağ projede yok; Ağlar'da tanımlı bir ağı seçin.", name.trim()),
                "access.network",
            )
        })
}

/// The network's name and its graph on each side: the definition as it is on the field state's side; on the
/// scenario's, every layer of it the scenario stands in for swapped for the scenario's layer.
pub(crate) fn graphs(doc: &Document, pairs: &[(String, String)], name: &str) -> Result<(String, Graph, Graph), Stop> {
    let base = definition(doc, name)?;
    let mut alt = base.clone();
    let swap = |l: &mut String| {
        if let Some((_, theirs)) = pairs.iter().find(|(b, _)| b == l) {
            *l = theirs.clone();
        }
    };
    alt.edges.iter_mut().for_each(|e| swap(&mut e.layer));
    alt.junctions.iter_mut().for_each(|j| swap(&mut j.layer));
    let h = host().map_err(|e| Stop::failed(*e))?;
    let build = |def: &NetworkDef| {
        h.network_of(doc, def).map(|s| s.graph).map_err(|why| {
            refuse(codes::INVALID_NETWORK, format!("“{}” ağı kurulamadı: {why}", def.name), "access.network")
        })
    };
    Ok((base.name.clone(), build(&base)?, build(&alt)?))
}

/// The access along `g` on a side whose areas are `areas`.
pub(crate) fn along(g: &Graph, areas: &[MeasuredArea], access: &ImpactAccess, connect: f64) -> Along {
    let mut r = Along::default();
    let place = |sh: &Shape| shape_centroid(sh).map(|c| (c, g.locate(c, connect)));
    let mut ends: Vec<(String, kentos_geometry_core::Vec2, Location)> = Vec::new();
    for (sh, _, _, uid) in areas.iter().filter(|(_, u, _, _)| access.to.contains(u)) {
        r.targets.push(uid.clone());
        match place(sh) {
            Some((c, Some(at))) => ends.push((uid.clone(), c, at)),
            _ => r.off_targets += 1,
        }
    }
    let targets: Vec<Location> = ends.iter().map(|(_, _, at)| *at).collect();
    let mut searcher = Searcher::new();
    for (sh, _, area, uid) in areas.iter().filter(|(_, u, _, _)| access.from.contains(u)) {
        r.whole += area;
        let Some((c, Some(at))) = place(sh) else {
            r.off.push(uid.clone());
            r.unreached.push(uid.clone());
            continue;
        };
        let found = if targets.is_empty() {
            Vec::new()
        } else {
            let q = Nearest {
                origins: std::slice::from_ref(&at),
                targets: &targets,
                k: None,
                cutoff: None,
                cost: 0,
                reverse: false,
                barriers: &[],
                at: None,
            };
            nearest(g, &mut searcher, &q, true).into_iter().next().unwrap_or_default()
        };
        // The nearest in all, the legs counted; of equals the first in the drawing's order.
        let best = found
            .into_iter()
            .map(|(f, path)| (at.d + f.cost + ends[f.target].2.d, f.target, path))
            .min_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)));
        match best {
            Some((total, k, path)) => {
                let (tid, tc, tat) = &ends[k];
                let mut pts = vec![[c.x, c.y], [at.p.x, at.p.y]];
                if let Some(p) = path {
                    pts.extend(polyline_of(&spans_edges(g, &p.spans)).0.iter().map(|q| [q.x, q.y]));
                }
                pts.extend([[tat.p.x, tat.p.y], [tc.x, tc.y]]);
                pts.dedup();
                r.routes.push(ImpactRoute { area: uid.clone(), target: tid.clone(), distance: total, legs: [at.d, tat.d], path: pts });
                if total <= access.distance {
                    r.got += area;
                    r.reached.push(uid.clone());
                } else {
                    r.unreached.push(uid.clone());
                }
            }
            None => r.unreached.push(uid.clone()),
        }
    }
    r
}
