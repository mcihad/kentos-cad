//! The roles a selection's objects have in the project (plan/map-cad.md MAP-01, docs/adr/2480): what a contextual tab's
//! role panels ask (Civil 3D's contextual tabs open the selected object's tools; KentOS's objects are drawing objects,
//! their kind the layer's role in the project).
//!
//! - `surface`: a terrain surface's source: on a layer (or in a group) a surface takes its points or breaklines from,
//!   or named by its persistent id;
//! - `network`: a utility network's part: on a layer the network takes its pipes or structures from;
//! - `zoning`: a zoning plan's object: on the layer of `imar-plani`'s plan boundaries, decision areas or notes.
//!
//! A role holds for the selection when every selected object has it. Worked out again only when the selection or the
//! drawing changes; the windows a role panel opens start on the selection's own surface or network.

use kentos_commands::NetworkKind;
use kentos_domain::Document;
use kentos_native_application::zoning_check;

use crate::app::App;

/// What the roles are worked out for: the drawing's session, the selection's version and the drawing's generation.
type Key = (u64, u64, u64);

/// The roles worked out last.
#[derive(Clone, Debug, Default)]
pub struct RolesCache(std::cell::RefCell<Option<(Key, Roles)>>);

/// The selection's roles and the surface and network every selected object belongs to (the first such).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Roles {
    pub roles: Vec<&'static str>,
    pub surface: Option<String>,
    pub network: Option<String>,
}

/// The leaf layers of layer or group ids.
fn leaves(doc: &Document, ids: &[String]) -> Vec<String> {
    ids.iter().flat_map(|id| doc.layers().leaves_of(id).into_iter().map(|n| n.id.clone())).collect()
}

/// The roles of the objects selected (their layers and persistent ids).
fn roles_of(doc: &Document, picked: &[(String, String)]) -> Roles {
    if picked.is_empty() {
        return Roles::default();
    }
    let settings = doc.settings();
    // The surfaces every selected object is a source of.
    let surfaces: Vec<(String, Vec<String>, Vec<String>)> = settings
        .road
        .as_ref()
        .map(|r| {
            r.surfaces
                .iter()
                .map(|s| {
                    let mut layers = leaves(doc, &s.points.layers);
                    layers.extend(leaves(doc, &s.breaklines.layers));
                    let uids: Vec<String> = s.points.uids.iter().chain(&s.breaklines.uids).cloned().collect();
                    (s.id.clone(), layers, uids)
                })
                .collect()
        })
        .unwrap_or_default();
    let surface = surfaces
        .iter()
        .find(|(_, layers, uids)| picked.iter().all(|(layer, uid)| layers.contains(layer) || uids.contains(uid)))
        .map(|(id, ..)| id.clone());
    // The utility network every selected object is a part of.
    let network = settings
        .networks
        .iter()
        .filter(|n| n.kind == NetworkKind::Utility)
        .find(|n| {
            let layers: Vec<&str> =
                n.edges.iter().map(|l| l.layer.as_str()).chain(n.junctions.iter().map(|l| l.layer.as_str())).collect();
            picked.iter().all(|(layer, _)| layers.contains(&layer.as_str()))
        })
        .map(|n| n.id.clone());
    let plan = zoning_check::plan_layers(doc);
    let zoning = !plan.is_empty() && picked.iter().all(|(layer, _)| plan.contains(layer));
    let mut roles = Vec::new();
    if surface.is_some() {
        roles.push("surface");
    }
    if network.is_some() {
        roles.push("network");
    }
    if zoning {
        roles.push("zoning");
    }
    Roles { roles, surface, network }
}

impl App {
    /// The selection's roles (docs/adr/2480), worked out again only when the selection or the drawing changes.
    pub(crate) fn selection_roles(&self) -> Roles {
        let Some(doc) = self.document.as_ref() else { return Roles::default() };
        let key = (doc.session, self.selection.version(), doc.model.generation());
        if let Some((k, roles)) = self.selection_roles.0.borrow().as_ref()
            && *k == key
        {
            return roles.clone();
        }
        let picked: Vec<(String, String)> = self
            .selection
            .ids()
            .iter()
            .filter_map(|slot| {
                let e = doc.model.get(*slot)?;
                Some((e.base().layer_id.clone(), doc.model.uid(*slot).map(|u| u.to_string()).unwrap_or_default()))
            })
            .collect();
        let roles = roles_of(&doc.model, &picked);
        *self.selection_roles.0.borrow_mut() = Some((key, roles.clone()));
        roles
    }

    /// Whether a contextual panel that asks `roles` shows for the selection: every role it names held.
    pub(crate) fn roles_hold(&self, roles: &[&str]) -> bool {
        roles.is_empty() || {
            let have = self.selection_roles();
            roles.iter().all(|r| have.roles.contains(r))
        }
    }
}
