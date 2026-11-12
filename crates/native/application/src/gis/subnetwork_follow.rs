//! Subnetwork names kept current (docs/adr/0562 §3): run by `call::recorded` after a command's write, in the same
//! step, so what a write does to a network's objects and the names it makes the subnetwork attribute hold are one undo
//! step (the document's other followers: constraints, linked texts, hatches).
//!
//! A network whose subnetworks say `keepCurrent` is followed when the write touched objects of its layers (added,
//! removed or changed one: its edges, its junctions, its turn lines): the subnetworks are found on the drawing as the
//! write left it and the attribute is rewritten where it is not what they say, as `cad.subnetworks.update` writes it. A
//! write is never refused for it: where the attribute cannot be brought up to date (the network cannot be built, a layer
//! is locked or lacks the attribute's field, a value breaks the field's rules) the answer carries `subnetwork_dirty`,
//! and `Alt ağları güncelle` does it when the cause is gone. The `cad.subnetworks.update` command itself is not
//! followed (it is the update).

use kentos_commands::{CAD_SUBNETWORKS_UPDATE, CommandWarning, SubnetworksUpdate, codes};
use kentos_domain::Document;

use crate::subnetworks_update::{update_in, write_planned};

fn dirty(name: &str, why: &str) -> CommandWarning {
    CommandWarning {
        code: codes::SUBNETWORK_DIRTY.into(),
        message: format!(
            "“{name}” ağı alt ağ adlarını güncel tutuyor ama bu yazma onları güncelleyemedi: {why} Alt ağları güncelle ile yenileyin."
        ),
        path: None,
    }
}

/// Follows the open step's write: see the module. Inside no step, or for networks that keep nothing current, it does
/// nothing.
pub(crate) fn follow(doc: &mut Document, command: &str) -> Vec<CommandWarning> {
    let mut out = Vec::new();
    if command == CAD_SUBNETWORKS_UPDATE || !doc.in_step() {
        return out;
    }
    let keepers: Vec<(String, String, Vec<String>)> = doc
        .settings()
        .networks
        .iter()
        .filter(|n| n.subnetworks.as_ref().is_some_and(|s| s.keep_current))
        .map(|n| {
            (
                n.id.clone(),
                n.name.clone(),
                n.layers().into_iter().map(str::to_owned).collect(),
            )
        })
        .collect();
    if keepers.is_empty() {
        return out;
    }
    let touched = doc.step_layers();
    for (id, name, layers) in keepers {
        if !layers.iter().any(|l| touched.contains(l)) {
            continue;
        }
        let input = SubnetworksUpdate {
            network: id,
            expected_revision: None,
        };
        match update_in(doc, &input) {
            Ok(planned) => {
                let left = planned.out.skipped;
                write_planned(doc, planned.writes);
                out.extend(planned.warnings);
                if left > 0 {
                    out.push(dirty(&name, &format!("{left} nesnenin değeri yazılmadı.")));
                }
            }
            Err(stop) => out.push(dirty(&name, &stop.into_error().message)),
        }
    }
    out
}
