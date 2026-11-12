//! `road.hauls.network` v1 (plan/road.md Y49, docs/adr/0955; ROAD-HAUL-06): a mass haul diagram's haul road network
//! read (`kentos_road::haul_network`): the line and polyline objects on its layers are its roads; where they meet the
//! alignment, how they join, what only crosses on the screen and does not join, the ends that join nothing; and how
//! each site off the road is reached. The diagram, its plan and this read place a site off the road the same way
//! (`placed`): at the station where its access meets the alignment, its access the extra distance every cubic metre
//! goes; a site the network does not reach is left out with a warning. Reads only. The shared cases are
//! `fixtures/commands/v1/road.hauls.network.json`.

use kentos_commands::{
    CommandResult, CommandWarning, Entity, HaulDef, HaulJunction, HaulNetworkIssue, HaulNetworkProblem, HaulNetworkRead,
    HaulRoadRead, HaulSiteAccess, HaulSiteKind, HaulsNetwork,
};
use kentos_domain::Document;
use kentos_road::alignment::km_text;
use kentos_road::haul_network::{Connection, Network, Problem, Road, flatten, network};
use kentos_road::haul_plan::PlanSite;

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_alignments::Built;
use crate::road_hauls as hauls;
use crate::road_terrains::fail;

/// A road as read: the core's road and its layer.
pub(crate) struct Read {
    pub roads: Vec<Road>,
    pub layers: Vec<String>,
    pub net: Network,
    pub connect: f64,
    pub reach: f64,
}

/// The lines and polylines on `layers`, each part a road, arcs flattened.
fn roads(doc: &Document, layers: &[String]) -> (Vec<Road>, Vec<String>) {
    let mut out = Vec::new();
    let mut on = Vec::new();
    let pts = |v: &[kentos_commands::Vec2]| v.iter().map(|p| [p.x, p.y]).collect::<Vec<[f64; 2]>>();
    for e in doc.entities() {
        match e {
            Entity::Line(l) if layers.contains(&l.base.layer_id) => {
                out.push(Road { id: u64::from(l.base.id), part: 0, points: vec![[l.a.x, l.a.y], [l.b.x, l.b.y]] });
                on.push(l.base.layer_id.clone());
            }
            Entity::Polyline(p) if layers.contains(&p.base.layer_id) => {
                let bulges = p.bulges.clone().unwrap_or_default();
                out.push(Road { id: u64::from(p.base.id), part: 0, points: flatten(&pts(&p.pts), &bulges, p.closed) });
                on.push(p.base.layer_id.clone());
                for (k, part) in p.parts.iter().flatten().enumerate() {
                    let b = part.bulges.clone().unwrap_or_default();
                    out.push(Road { id: u64::from(p.base.id), part: k as u32 + 1, points: flatten(&pts(&part.pts), &b, false) });
                    on.push(p.base.layer_id.clone());
                }
            }
            _ => {}
        }
    }
    (out, on)
}

/// The diagram's network against its alignment, if it has one; a layer not in the drawing is refused.
pub(crate) fn read(doc: &Document, def: &HaulDef, built: &Built) -> Result<Option<Read>, Stop> {
    let Some(n) = &def.network else { return Ok(None) };
    for (i, id) in n.layers.iter().enumerate() {
        if doc.layers().get(id).is_none() {
            return Err(fail(codes::INVALID_HAUL, format!("Taşıma ağının “{id}” kimlikli katmanı çizimde yok."), &format!("network.layers[{i}]")));
        }
    }
    let (roads, layers) = roads(doc, &n.layers);
    let reach = n.reach.unwrap_or(n.connect);
    let sites: Vec<[f64; 2]> = def.sites.iter().filter_map(|s| s.at.as_ref().map(|p| [p.x, p.y])).collect();
    let net = network(&built.core, built.offset, &roads, &sites, n.connect, reach);
    Ok(Some(Read { roads, layers, net, connect: n.connect, reach }))
}

/// Why a site does not reach the alignment, in words.
fn why(c: &Connection, connect: f64) -> Option<String> {
    match c {
        Connection::Joined { .. } => None,
        Connection::NoRoad { nearest: Some(d) } => {
            Some(format!("en yakın taşıma yolu {d:.2} m uzakta; bağlantı toleransı {connect:.2} m"))
        }
        Connection::NoRoad { nearest: None } => Some("taşıma ağının katmanlarında yol yok".into()),
        Connection::Cut { .. } => Some("bağlandığı yollar aksa ulaşmıyor".into()),
    }
}

/// The sites as a plan places them (docs/adr/0955): one on the road at its station; one off the road at the station
/// where its access meets the alignment, with that access; one the network does not reach with no capacity, and a
/// warning. Also each site's access (none on the road) and whether it is unconnected.
pub(crate) struct Placed {
    pub sites: Vec<PlanSite>,
    pub access: Vec<Option<f64>>,
    pub unconnected: Vec<bool>,
    pub warnings: Vec<CommandWarning>,
}

pub(crate) fn placed(doc: &Document, def: &HaulDef, built: &Built) -> Result<Placed, Stop> {
    let read = if def.sites.iter().any(|s| s.at.is_some()) { read(doc, def, built)? } else { None };
    let mut out = Placed { sites: Vec::new(), access: Vec::new(), unconnected: Vec::new(), warnings: Vec::new() };
    let mut k = 0;
    for (j, s) in def.sites.iter().enumerate() {
        let borrow = s.kind == HaulSiteKind::Borrow;
        let base = PlanSite { station: s.station, capacity: s.volume, borrow, access: 0.0 };
        if s.at.is_none() {
            out.sites.push(base);
            out.access.push(None);
            out.unconnected.push(false);
            continue;
        }
        let c = read.as_ref().and_then(|r| r.net.sites.get(k).cloned().map(|c| (c, r.connect)));
        k += 1;
        match c {
            Some((Connection::Joined { length, station, .. }, _)) => {
                out.sites.push(PlanSite { station, access: length, ..base });
                out.access.push(Some(length));
                out.unconnected.push(false);
            }
            other => {
                let words = other.as_ref().and_then(|(c, connect)| why(c, *connect)).unwrap_or_else(|| "taşıma ağı yok".into());
                out.warnings.push(CommandWarning {
                    code: codes::HAUL_SITE_UNCONNECTED.into(),
                    message: format!("“{}” alanı taşıma ağıyla aksa bağlanmıyor ({words}); hesaba katılmadı.", s.name),
                    path: Some(format!("sites[{j}]")),
                });
                out.sites.push(PlanSite { capacity: 0.0, ..base });
                out.access.push(None);
                out.unconnected.push(true);
            }
        }
    }
    Ok(out)
}

/// The network read now, and its warnings.
pub(crate) fn answer(doc: &Document, input: &HaulsNetwork) -> Result<(HaulNetworkRead, Vec<CommandWarning>), Stop> {
    hauls::road_project(doc)?;
    let (_, def) = hauls::find(doc.road_hauls(), &input.haul, "haul")?;
    if def.network.is_none() {
        return Err(fail(
            codes::INVALID_HAUL,
            format!("“{}” Brüknerinin taşıma ağı yok; önce ağın katmanlarını verin (road.hauls.edit network).", def.name),
            "haul",
        ));
    }
    let built = hauls::alignment_of(doc, def)?;
    let Some(r) = read(doc, def, &built)? else { return Err(Stop::Cancelled) };
    let km = |s: f64| km_text(built.km(s), 3);
    let index = |i: usize| i as u32;
    let mut warnings = Vec::new();
    let problems: Vec<HaulNetworkProblem> = r
        .net
        .problems
        .iter()
        .map(|p| match p {
            Problem::Crossing { a, b, at } => HaulNetworkProblem {
                kind: HaulNetworkIssue::Crossing,
                roads: vec![index(*a), index(*b)],
                x: at[0],
                y: at[1],
                gap: None,
                message: format!(
                    "{}. ve {}. yol ({:.2}, {:.2}) noktasında kesişiyor ama birleşmiyor: biri ötekinin üstünde bitmeli.",
                    a + 1,
                    b + 1,
                    at[0],
                    at[1]
                ),
            },
            Problem::CrossesAlignment { road, station, at } => HaulNetworkProblem {
                kind: HaulNetworkIssue::CrossesAlignment,
                roads: vec![index(*road)],
                x: at[0],
                y: at[1],
                gap: None,
                message: format!("{}. yol aksı {} km'sinde gövdesiyle kesiyor; aksa yalnız ucu bağlanır.", road + 1, km(*station)),
            },
            Problem::Loose { road, end, at, gap } => HaulNetworkProblem {
                kind: HaulNetworkIssue::Loose,
                roads: vec![index(*road)],
                x: at[0],
                y: at[1],
                gap: *gap,
                message: format!(
                    "{}. yolun {} hiçbir yere bağlanmıyor{}.",
                    road + 1,
                    if *end { "sonu" } else { "başı" },
                    gap.map_or_else(String::new, |g| format!(
                        " (en yakın yol ya da aks {g:.2} m uzakta; bağlantı toleransı {:.2} m)",
                        r.connect
                    ))
                ),
            },
        })
        .collect();
    if !problems.is_empty() {
        warnings.push(CommandWarning {
            code: codes::HAUL_NETWORK_GAP.into(),
            message: format!(
                "“{}” taşıma ağında birleşmeyen {} yer var: {}",
                def.name,
                problems.len(),
                problems.iter().map(|p| p.message.clone()).collect::<Vec<_>>().join(" ")
            ),
            path: Some("network".into()),
        });
    }
    let mut sites = Vec::new();
    let mut k = 0;
    for (j, s) in def.sites.iter().enumerate() {
        let Some(at) = &s.at else { continue };
        let c = &r.net.sites[k];
        k += 1;
        let mut row = HaulSiteAccess {
            name: s.name.clone(),
            kind: s.kind,
            x: at.x,
            y: at.y,
            joined: false,
            length: None,
            station: None,
            km: None,
            offset: None,
            roads: Vec::new(),
            why: why(c, r.connect),
        };
        if let Connection::Joined { length, station, offset, roads } = c {
            row = HaulSiteAccess {
                joined: true,
                length: Some(*length),
                station: Some(*station),
                km: Some(km(*station)),
                offset: Some(*offset),
                roads: roads.iter().map(|x| index(*x)).collect(),
                ..row
            };
        } else {
            warnings.push(CommandWarning {
                code: codes::HAUL_SITE_UNCONNECTED.into(),
                message: format!("“{}” alanı taşıma ağıyla aksa bağlanmıyor ({}); hesaba katılmadı.", s.name, row.why.clone().unwrap_or_default()),
                path: Some(format!("sites[{j}]")),
            });
        }
        sites.push(row);
    }
    let out = HaulNetworkRead {
        haul: def.id.clone(),
        connect: r.connect,
        reach: r.reach,
        roads: r
            .roads
            .iter()
            .zip(&r.layers)
            .zip(&r.net.lengths)
            .map(|((x, layer), length)| HaulRoadRead { entity: x.id, part: x.part, layer: layer.clone(), length: *length })
            .collect(),
        junctions: r
            .net
            .junctions
            .iter()
            .map(|j| HaulJunction { road: index(j.road), end: j.end, station: j.station, km: km(j.station), offset: j.offset })
            .collect(),
        joins: r.net.joins.len() as u32,
        problems,
        sites,
    };
    Ok((out, warnings))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &HaulsNetwork) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The read; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &HaulsNetwork) -> CommandResult<HaulNetworkRead> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The read, as execute; writes nothing.
pub fn execute(cx: &mut ExecutionContext<'_>, input: HaulsNetwork) -> CommandResult<HaulNetworkRead> {
    plan(cx, &input)
}

/// For the window: the read, or why not.
pub fn preview(doc: &Document, input: &HaulsNetwork) -> Result<(HaulNetworkRead, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}

/// What a window draws of a diagram's network (docs/adr/0955): the alignment's centre line every 2 m, each road's
/// points in plan (arcs flattened), in the order of `road.hauls.network`'s roads.
#[derive(Clone, Debug, Default)]
pub struct Drawn {
    pub axis: Vec<[f64; 2]>,
    pub roads: Vec<Vec<[f64; 2]>>,
}

/// The network's drawing for diagram `haul`, if it has a network that reads.
pub fn drawn(doc: &Document, haul: &str) -> Option<Drawn> {
    let (_, def) = hauls::find(doc.road_hauls(), haul, "haul").ok()?;
    let n = def.network.as_ref()?;
    let built = hauls::alignment_of(doc, def).ok()?;
    let length = built.core.length;
    let steps = ((length / 2.0).ceil() as usize).clamp(2, 20_000);
    let (sin_cos, shift) = (|t: f64| t.sin_cos(), built.offset);
    let axis = (0..=steps)
        .filter_map(|k| {
            let l = built.core.at(length * k as f64 / steps as f64)?;
            // An offset alignment's centre line lies `shift` to the right of the geometry's (offsets are right
            // positive).
            let (s, c) = sin_cos(l.direction);
            Some([l.x + shift * s, l.y - shift * c])
        })
        .collect();
    let (roads, _) = roads(doc, &n.layers);
    Some(Drawn { axis, roads: roads.into_iter().map(|r| r.points).collect() })
}
