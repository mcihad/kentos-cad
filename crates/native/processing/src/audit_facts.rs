//! The facts an extension's audit rule reads beside the layers (`facts.project`, `kentos.auditFacts` 1; plan/eklenti.md
//! ADIM 20, docs/adr/8057 §36), measured here from the drawing with the geometry core. Their shape, order, rounding
//! and bounds are `kentos_extensions::audit_facts`'s; this module only measures.
//!
//! - **What:** every layer holding objects, in the tree's order: its objects counted by geometry, its areas' area and
//!   its lines' length (arcs exact), and what three of the Topoloji tab's rules find on it: Geçerli olmalı (each
//!   problem of its lines and areas), Çakışmamalı (each pair of its areas covering each other) and Boşluk olmamalı
//!   (what its areas' union closes in), at the project's tolerance and without its exceptions. The facts are the
//!   drawing's; what they mean is the rule's Python to say.
//! - **The same everywhere:** the objects go to the core by persistent id, not in the drawing's order, so the same
//!   objects give the same bytes whatever order they were drawn or read in; past `most` of them (by id) the rest are
//!   left out and `cut` says so. The desktop and a cloud job pass the same [`MOST_OBJECTS`].
//! - **Stoppable:** the check asks `stopped` as it goes (the audit's Durdur) and, stopped, gives nothing.
//! - **No attribute is read:** the facts are shapes and counts, so a field the drawing hides is never in them.

use std::collections::BTreeMap;

use kentos_commands::{Entity, LayerNode, TOPOLOGY_TOLERANCE};
use kentos_domain::Document;
use kentos_extensions::audit_facts::{
    Gap, InvalidGeometry, LayerFacts, ObjectCounts, Overlap, ProjectAuditFacts, Totals,
    ValidityProblem,
};
use kentos_geometry_core::geom::arrangement::edge_len;
use kentos_geometry_core::geom::region::net_area;
use kentos_geometry_core::ops::geoprocess::{Class, class_of};
use kentos_geometry_core::ops::topology_rules::{Kind, Objects, Rule, Watch, check_watched};
use kentos_native_application::geometry::shape;

/// The most objects measured: past it the facts are cut (`cut`). The desktop's audit and a cloud job pass this one, so
/// that a large drawing gives the same facts on both. Boşluk olmamalı's overlay of a whole layer takes about 5 µs a
/// vertex (docs/adr/0756): 200 000 parcels of 8 corners, some 8 s on the audit's own thread.
pub const MOST_OBJECTS: usize = 200_000;

/// The facts of `doc`'s objects, at most `most` of them by persistent id; none when `stopped` said to stop.
pub fn project_facts(
    doc: &Document,
    most: usize,
    stopped: &dyn Fn() -> bool,
) -> Option<ProjectAuditFacts> {
    let tolerance = doc
        .settings()
        .topology
        .as_ref()
        .map_or(TOPOLOGY_TOLERANCE, |t| t.tolerance());
    let leaves = doc.layers().leaves();
    let leaf: BTreeMap<&str, usize> = leaves
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();
    let mut found: Vec<(String, usize, &Entity)> = doc
        .entities_with_uids()
        .filter_map(|(uid, e)| Some((uid.to_string(), *leaf.get(e.base().layer_id.as_str())?, e)))
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    let cut = found.len() > most;
    found.truncate(most);
    if stopped() {
        return None;
    }
    let mut facts: Vec<Option<LayerFacts>> = vec![None; leaves.len()];
    let (mut shapes, mut layers, mut uids) = (Vec::new(), Vec::new(), Vec::new());
    for (uid, at, e) in found {
        let s = shape(e);
        let Some(l) = facts.get_mut(at) else {
            continue;
        };
        let l = l.get_or_insert_with(|| empty(leaves[at]));
        match class_of(&s) {
            Class::Points(p) if !p.is_empty() => l.objects.points += 1,
            Class::Paths(p) if !p.is_empty() => {
                l.objects.lines += 1;
                l.length += p.iter().flatten().map(edge_len).sum::<f64>();
            }
            Class::Areas(a) if !a.is_empty() => {
                l.objects.areas += 1;
                l.area += a.iter().map(net_area).sum::<f64>();
            }
            _ => l.objects.other += 1,
        }
        shapes.push(s);
        layers.push(e.base().layer_id.clone());
        uids.push(uid);
    }
    let (rules, of_rule) = rules(&facts);
    let objects = Objects {
        shapes: &shapes,
        layers: &layers,
        uids: &uids,
    };
    let checked = check_watched(&objects, &rules, tolerance, &[], &Asked(stopped))?;
    let uid = |i: usize| uids.get(i).cloned().unwrap_or_default();
    for f in checked.findings {
        let (Some(rule), Some(&at)) = (rules.get(f.rule), of_rule.get(f.rule)) else {
            continue;
        };
        let Some(Some(l)) = facts.get_mut(at) else {
            continue;
        };
        let xy = [f.at.x, f.at.y];
        match (rule.kind, f.objects.as_slice()) {
            (Kind::MustBeValid, &[i, ..]) => {
                if let Some(problem) = ValidityProblem::of_key(f.problem) {
                    l.invalid.push(InvalidGeometry {
                        uid: uid(i),
                        problem,
                        at: xy,
                    });
                }
            }
            (Kind::MustNotOverlap, &[i, j]) => l.overlaps.push(Overlap {
                a: uid(i),
                b: uid(j),
                area: f.measure.unwrap_or_default(),
                at: xy,
            }),
            (Kind::MustNotHaveGaps, around) => l.gaps.push(Gap {
                area: f.measure.unwrap_or_default(),
                at: xy,
                around: around.iter().map(|&i| uid(i)).collect(),
            }),
            _ => {}
        }
    }
    Some(ProjectAuditFacts::new(
        tolerance,
        facts.into_iter().flatten().collect(),
        cut,
    ))
}

/// The rules each layer is measured with, and each rule's layer (its place among the leaves): Geçerli olmalı where it
/// has lines or areas, Çakışmamalı where it has two areas, Boşluk olmamalı where it has one.
fn rules(facts: &[Option<LayerFacts>]) -> (Vec<Rule>, Vec<usize>) {
    let mut rules = Vec::new();
    let mut of_rule = Vec::new();
    for (at, l) in facts.iter().enumerate() {
        let Some(l) = l else {
            continue;
        };
        let n = l.objects;
        for (kind, takes) in [
            (Kind::MustBeValid, n.lines + n.areas > 0),
            (Kind::MustNotOverlap, n.areas > 1),
            (Kind::MustNotHaveGaps, n.areas > 0),
        ] {
            if takes {
                rules.push(Rule {
                    id: format!("{}:{}", l.id, kind.key()),
                    kind,
                    layer: l.id.clone(),
                    other: None,
                    value: None,
                });
                of_rule.push(at);
            }
        }
    }
    (rules, of_rule)
}

fn empty(n: &LayerNode) -> LayerFacts {
    LayerFacts {
        id: n.id.clone(),
        name: n.name.clone(),
        objects: ObjectCounts::default(),
        area: 0.0,
        length: 0.0,
        totals: Totals::default(),
        invalid: Vec::new(),
        overlaps: Vec::new(),
        gaps: Vec::new(),
    }
}

/// The audit's Durdur, as the check asks it.
struct Asked<'a>(&'a dyn Fn() -> bool);

impl Watch for Asked<'_> {
    fn progress(&self, _: f64) {}

    fn stopped(&self) -> bool {
        (self.0)()
    }
}

#[cfg(test)]
mod tests;
