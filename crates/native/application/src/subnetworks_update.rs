//! `cad.subnetworks.update` (docs/adr/0557 §2): the subnetworks' names written into the attribute the network names, on
//! every object of the network: joined with `; ` where an object is in two subnetworks, taken away where it is in none;
//! objects already right are left. Each layer's values are checked as `cad.fields.write` checks them (a locked layer,
//! or a typed layer without the attribute's field, is left and said); one undo step, “Alt ağları güncelle”.

use std::collections::BTreeMap;

use kentos_commands::{
    CommandResult, CommandWarning, Entity, FieldRow, FieldsWrite, FieldsWriteOp, SubnetworksFind,
    SubnetworksUpdate, SubnetworksUpdated, WritePolicy, codes,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::fields_write::{Write, run as plan_fields};
use crate::subnetworks::{Object, find_in};

/// What an update writes: the writes, the answer and its warnings.
struct Planned {
    writes: Vec<Write>,
    out: SubnetworksUpdated,
    warnings: Vec<CommandWarning>,
}

fn update_in(doc: &Document, input: &SubnetworksUpdate) -> Result<Planned, Stop> {
    let found = find_in(
        doc,
        &SubnetworksFind {
            network: input.network.clone(),
            members: false,
        },
    )?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let attribute = found.out.attribute.clone();
    // The objects to write, by layer in the order the layers first come.
    let mut layers: Vec<String> = Vec::new();
    let mut by_layer: BTreeMap<String, Vec<(Object, Option<String>)>> = BTreeMap::new();
    for (o, want) in found.wrong {
        if !by_layer.contains_key(&o.layer) {
            layers.push(o.layer.clone());
        }
        by_layer.entry(o.layer.clone()).or_default().push((o, want));
    }
    let mut writes: Vec<Write> = Vec::new();
    let mut warnings: Vec<CommandWarning> = Vec::new();
    let (mut skipped, mut invalid, mut written, mut cleared) = (0u32, 0u32, 0u32, 0u32);
    for layer in layers {
        let rows = by_layer.remove(&layer).unwrap_or_default();
        let name = doc
            .layers()
            .get(&layer)
            .map_or_else(|| layer.clone(), |n| n.name.clone());
        let why = match doc.layers().get(&layer) {
            None => Some("çizimde yok".to_owned()),
            Some(_) if doc.layers().is_locked(&layer) => Some("kilitli".to_owned()),
            Some(n) if !n.fields.is_empty() && !n.fields.iter().any(|f| f.name == attribute) => {
                Some(format!("“{attribute}” alanını taşımıyor"))
            }
            _ => None,
        };
        if let Some(why) = why {
            skipped += rows.len() as u32;
            warnings.push(CommandWarning {
                code: codes::SUBNETWORK_LAYER_SKIPPED.into(),
                message: format!(
                    "“{name}” katmanı {why}; {} nesnesine alt ağ adı yazılmadı.",
                    rows.len()
                ),
                path: None,
            });
            continue;
        }
        let write = FieldsWrite {
            layer: layer.clone(),
            write: FieldsWriteOp::Values {
                rows: rows
                    .iter()
                    .map(|(o, want)| FieldRow {
                        uid: o.uid.clone(),
                        values: BTreeMap::from([(attribute.clone(), want.clone())]),
                    })
                    .collect(),
            },
            policy: WritePolicy::Valid,
        };
        let planned = plan_fields(doc, &write)?;
        invalid += planned.out.invalid;
        for w in &planned.writes {
            if w.1.contains_key(&attribute) {
                written += 1;
            } else {
                cleared += 1;
            }
        }
        writes.extend(planned.writes);
    }
    if invalid > 0 {
        skipped += invalid;
        warnings.push(CommandWarning {
            code: codes::VALUES_SKIPPED.into(),
            message: format!(
                "{invalid} nesnenin alt ağ adı “{attribute}” alanının kurallarına uymadığı için yazılmadı."
            ),
            path: None,
        });
    }
    Ok(Planned {
        writes,
        out: SubnetworksUpdated {
            found: found.out,
            written,
            skipped,
            cleared,
            revision: doc.revision().to_string(),
        },
        warnings,
    })
}

/// Checks `input` against the drawing, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &SubnetworksUpdate) -> CommandResult<()> {
    match update_in(cx.doc, input) {
        Ok(p) => CommandResult::Completed {
            output: (),
            warnings: p.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What `update` would write now; writes nothing.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &SubnetworksUpdate,
) -> CommandResult<SubnetworksUpdated> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document alone (a window shows it before the write).
pub fn plan_in(doc: &Document, input: &SubnetworksUpdate) -> CommandResult<SubnetworksUpdated> {
    match update_in(doc, input) {
        Ok(p) => CommandResult::Completed {
            output: p.out,
            warnings: p.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Writes the subnetworks' names, one undo step “Alt ağları güncelle”.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: SubnetworksUpdate,
) -> CommandResult<SubnetworksUpdated> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_SUBNETWORKS_UPDATE,
        kentos_commands::CAD_SUBNETWORKS_UPDATE_VERSION,
        input,
        written_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step and journals it.
fn written_execute(
    cx: &mut ExecutionContext<'_>,
    input: SubnetworksUpdate,
) -> CommandResult<SubnetworksUpdated> {
    let planned = match update_in(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    if !planned.writes.is_empty() {
        let updates: Vec<(Slot, Entity)> = planned
            .writes
            .iter()
            .filter_map(|(slot, attrs, follows)| {
                let mut e = cx.doc.get(*slot)?.clone();
                let base = e.base_mut();
                base.attrs = attrs.clone();
                if let Some(l) = follows {
                    base.label = l.clone();
                }
                Some((*slot, e))
            })
            .collect();
        cx.doc.update_many(updates, "Alt ağları güncelle");
    }
    let mut out = planned.out;
    out.revision = cx.doc.revision().to_string();
    out.found.revision = out.revision.clone();
    CommandResult::Completed {
        output: out,
        warnings: planned.warnings,
    }
}
