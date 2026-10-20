//! `cad.subnetworks.find` and `.update` (docs/adr/0557 §2, plan/gis.md CBS-27): the subnetworks of a utility network.
//! The network's sources are the controllers; each is named (the network's name field, else its label, else its `Ad`,
//! else `Kaynak <order>`), controllers of one name (Turkish case aside) are one subnetwork, and a subnetwork is what its
//! controllers reach by the network's rules (the core's `ops::network::subnetworks`; the graph is built by the host
//! that has İşlemler's tools, `processing_host::ToolHost::network`). `find` says what each holds, what no controller
//! reaches, what two reach, and what the attribute the network names says differently; `update` writes the names into
//! that attribute on every object of the network (joined with `; ` where an object is in two subnetworks, taken away
//! where it is in none), checked as `cad.fields.write` checks a value, in one undo step.

use std::collections::{HashMap, HashSet};

use kentos_commands::{
    CommandResult, Entity, NetworkDef, SUBNETWORK_UIDS_LISTED, Subnetwork, SubnetworkObjects,
    SubnetworkReach, SubnetworkStale, SubnetworksFind, SubnetworksFound, codes, network_caseless,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::ops::network::{Reach, Subnet};

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::processing_host::host;

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// A controller's name: the network's name field, else its label, else its `Ad`, else `Kaynak <order>`.
fn controller_name(e: Option<&Entity>, field: Option<&str>, order: usize) -> String {
    let given = |v: Option<&String>| v.map(|t| t.trim().to_owned()).filter(|t| !t.is_empty());
    e.and_then(|e| {
        field
            .and_then(|f| given(e.base().attrs.get(f)))
            .or_else(|| given(e.base().label.as_ref()))
            .or_else(|| given(e.base().attrs.get("Ad")))
    })
    .unwrap_or_else(|| format!("Kaynak {order}"))
}

/// One object of the network the attribute is wrong on: its persistent id (as text) and its layer.
pub(crate) struct Object {
    pub(crate) uid: String,
    pub(crate) layer: String,
}

/// What a find knows: the answer and what an update needs to write it.
pub(crate) struct Found {
    pub(crate) out: SubnetworksFound,
    /// The objects whose value is not what the subnetworks say: their slot, uid, layer and the value they should have.
    pub(crate) wrong: Vec<(Object, Option<String>)>,
}

fn id_slot(id: f64) -> Slot {
    Slot(id as u32)
}

fn objects(list: Vec<String>) -> SubnetworkObjects {
    SubnetworkObjects {
        count: list.len() as u32,
        uids: list.into_iter().take(SUBNETWORK_UIDS_LISTED).collect(),
    }
}

pub(crate) fn find_in(doc: &Document, input: &SubnetworksFind) -> Result<Found, Stop> {
    let def: &NetworkDef = doc
        .settings()
        .networks
        .iter()
        .find(|n| n.id == input.network)
        .ok_or_else(|| {
            refuse(
                codes::UNKNOWN_NETWORK,
                format!(
                    "“{}” kimlikli ağ projede yok. Var olan bir ağın kimliğini verin.",
                    input.network
                ),
                "network",
            )
        })?;
    let Some(subs) = def.subnetworks.as_ref() else {
        return Err(refuse(
            codes::NO_SUBNETWORKS,
            format!(
                "“{}” ağının alt ağları tanımlı değil; ağın subnetworks alanını yazın (cad.network.define).",
                def.name
            ),
            "network",
        ));
    };
    let (def, session) = host().map_err(|e| Stop::failed(*e)).and_then(|h| {
        h.network(doc, &input.network)
            .map_err(|why| refuse(codes::INVALID_NETWORK, why, "network"))
    })?;
    let controllers = session.controllers();
    if controllers.is_empty() {
        return Err(refuse(
            codes::NO_CONTROLLERS,
            format!(
                "“{}” ağının çizimde Kaynak düğümü yok; alt ağ denetleyicisi bulunamadı.",
                def.name
            ),
            "network",
        ));
    }
    let g = &session.graph;
    // The controllers' names, and the controllers of one name together, in the order the names first come.
    let mut names: Vec<String> = Vec::new();
    let mut keys: Vec<String> = Vec::new();
    let mut groups: Vec<Vec<u32>> = Vec::new();
    for (order, &c) in controllers.iter().enumerate() {
        let e = doc.get(id_slot(g.junctions[c as usize].id));
        let name = controller_name(e, subs.name.as_deref(), order + 1);
        let key = network_caseless(&name);
        match keys.iter().position(|k| *k == key) {
            Some(i) => groups[i].push(c),
            None => {
                keys.push(key);
                names.push(name);
                groups.push(vec![c]);
            }
        }
    }
    let reach = match subs.reach {
        SubnetworkReach::Downstream => Reach::Downstream,
        SubnetworkReach::Connected => Reach::Connected,
    };
    let subnets: Vec<Subnet> = session.subnets(&groups, reach);
    // Every object the network took, once, edges then junctions.
    let mut seen: HashSet<u64> = HashSet::new();
    let mut universe: Vec<f64> = Vec::new();
    for id in g
        .edges
        .iter()
        .map(|e| e.id)
        .chain(g.junctions.iter().map(|j| j.id))
    {
        if seen.insert(id.to_bits()) {
            universe.push(id);
        }
    }
    let mut member_of: HashMap<u64, Vec<usize>> = HashMap::new();
    for (k, s) in subnets.iter().enumerate() {
        for id in s.edges.iter().chain(&s.junctions) {
            let list = member_of.entry(id.to_bits()).or_default();
            if list.last() != Some(&k) {
                list.push(k);
            }
        }
    }
    let uid_of = |id: f64| doc.uid(id_slot(id)).map(|u| u.to_string());
    let attribute = subs.attribute.as_str();
    let mut orphans: Vec<String> = Vec::new();
    let mut shared: Vec<String> = Vec::new();
    let mut stale = (Vec::new(), Vec::new(), Vec::new());
    let mut wrong: Vec<(Object, Option<String>)> = Vec::new();
    for &id in &universe {
        let (Some(uid), Some(e)) = (uid_of(id), doc.get(id_slot(id))) else {
            continue;
        };
        let want: Option<String> = member_of.get(&id.to_bits()).map(|list| {
            list.iter()
                .map(|&k| names[k].as_str())
                .collect::<Vec<_>>()
                .join("; ")
        });
        let now = e
            .base()
            .attrs
            .get(attribute)
            .map(|v| v.trim().to_owned())
            .filter(|v| !v.is_empty());
        match member_of.get(&id.to_bits()) {
            None => orphans.push(uid.clone()),
            Some(list) if list.len() > 1 => shared.push(uid.clone()),
            _ => {}
        }
        let bucket = match (&want, &now) {
            (Some(w), Some(n)) if w != n => Some(&mut stale.0),
            (Some(_), None) => Some(&mut stale.1),
            (None, Some(_)) => Some(&mut stale.2),
            _ => None,
        };
        if let Some(b) = bucket {
            b.push(uid.clone());
            wrong.push((
                Object {
                    uid,
                    layer: e.base().layer_id.clone(),
                },
                want,
            ));
        }
    }
    let listed: Vec<Subnetwork> = subnets
        .iter()
        .enumerate()
        .map(|(k, s)| {
            let mut controllers_uids: Vec<String> = Vec::new();
            for c in &s.controllers {
                if let Some(u) = uid_of(*c)
                    && !controllers_uids.contains(&u)
                {
                    controllers_uids.push(u);
                }
            }
            let members = if input.members {
                s.edges
                    .iter()
                    .chain(&s.junctions)
                    .filter_map(|id| uid_of(*id))
                    .take(SUBNETWORK_UIDS_LISTED)
                    .collect()
            } else {
                Vec::new()
            };
            Subnetwork {
                name: names[k].clone(),
                controllers: controllers_uids,
                edges: s.edges.len() as u32,
                junctions: s.junctions.len() as u32,
                length: s.length,
                unverified: s.unverified as u32,
                members,
            }
        })
        .collect();
    let controller_count = controllers
        .iter()
        .map(|&c| g.junctions[c as usize].id.to_bits())
        .collect::<HashSet<_>>()
        .len() as u32;
    Ok(Found {
        out: SubnetworksFound {
            network: input.network.clone(),
            attribute: subs.attribute.clone(),
            reach: subs.reach,
            controllers: controller_count,
            subnetworks: listed,
            orphans: objects(orphans),
            shared: objects(shared),
            stale: SubnetworkStale {
                changed: objects(stale.0),
                missing: objects(stale.1),
                left_over: objects(stale.2),
            },
            revision: doc.revision().to_string(),
        },
        wrong,
    })
}

/// Checks `input` against the drawing, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &SubnetworksFind) -> CommandResult<()> {
    match find_in(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The subnetworks and what the attribute says of them.
pub fn plan(cx: &ExecutionContext<'_>, input: &SubnetworksFind) -> CommandResult<SubnetworksFound> {
    run(cx.doc, input)
}

/// [`plan`] on a document alone (a window reads it again after every write).
pub fn run(doc: &Document, input: &SubnetworksFind) -> CommandResult<SubnetworksFound> {
    match find_in(doc, input) {
        Ok(f) => CommandResult::Completed {
            output: f.out,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The same as [`plan`]: a find reads.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: SubnetworksFind,
) -> CommandResult<SubnetworksFound> {
    plan(cx, &input)
}
