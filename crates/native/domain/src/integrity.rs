//! The document's integrity after a step (docs/adr/0253 §8): what a write
//! must not leave behind. Checked over what the step touched, once its
//! objects followed (the outermost step), so a command or a change set that
//! breaks it is taken back whole:
//!
//! - an object on a layer the tree does not have, or on a group;
//! - an insert of a block the drawing does not define;
//! - a linked text whose object is gone;
//! - an associative hatch, or a dimension's live reference, to an object
//!   that is gone; or such a reference left to an object the step removed.
//!
//! Only what the step brought counts: an object the step did not move to
//! its layer, block or link is not refused for a fault it already had (a
//! file opened with it).

use kentos_contracts::{Entity, LayerNodeType};

use crate::document::Document;
use crate::identity::Uuid;
use crate::step::StepReport;

/// A fault a step would leave.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fault {
    /// The object it is about.
    pub uid: Uuid,
    /// Turkish, for people: what and how to fix it.
    pub message: String,
}

impl Document {
    /// What the step reported would leave broken; none when nothing.
    pub fn faults(&self, report: &StepReport) -> Vec<Fault> {
        let mut out = Vec::new();
        // A Veri girişi's project (docs/adr/0411 §5): existing records' attributes alone.
        if self.object_policy == crate::document::ObjectPolicy::AttributesOnly {
            for change in &report.objects {
                let attributes_only = match (change.before.as_deref(), change.after.as_deref()) {
                    (Some(before), Some(after)) => {
                        let mut probe = after.clone();
                        probe.base_mut().attrs = before.base().attrs.clone();
                        probe == *before
                    }
                    _ => false,
                };
                if !attributes_only {
                    out.push(Fault {
                        uid: change.uid,
                        message: "Bu projede Veri girişi rolündesiniz: yalnız kayıtların özniteliklerini formla değiştirebilirsiniz; nesne eklenmez, silinmez, şekli ve görünüşü değişmez.".into(),
                    });
                }
            }
            if !out.is_empty() {
                return out;
            }
        }
        for change in &report.objects {
            let Some(after) = &change.after else {
                // Removed: nothing may still follow it.
                let linked = self.store.linked_to(change.uid).next().is_some();
                let tied = self.store.tied_to(change.uid).next().is_some();
                if linked || tied {
                    out.push(Fault {
                        uid: change.uid,
                        message: "Silinen nesneye hâlâ bir yazı, tarama ya da ölçü bağlı.".into(),
                    });
                }
                continue;
            };
            let before = change.before.as_deref();
            let base = after.base();
            let moved = before.is_none_or(|b| b.base().layer_id != base.layer_id);
            if moved {
                match self.layers.get(&base.layer_id) {
                    None => out.push(Fault {
                        uid: change.uid,
                        message: format!("Nesne çizimde olmayan “{}” katmanında.", base.layer_id),
                    }),
                    Some(node) if node.kind == LayerNodeType::Group => out.push(Fault {
                        uid: change.uid,
                        message: format!(
                            "Nesne “{}” grubuna konmuş; nesne yalnız katmana konur.",
                            node.name
                        ),
                    }),
                    Some(_) => {}
                }
            }
            match &**after {
                Entity::Insert(insert) => {
                    let new = !matches!(before, Some(Entity::Insert(b)) if b.block == insert.block);
                    if new && self.block(insert.block).is_none() {
                        out.push(Fault {
                            uid: change.uid,
                            message: "Yerleştirmenin bloğu çizimde tanımlı değil.".into(),
                        });
                    }
                }
                Entity::Text(text) => {
                    let new =
                        !matches!(before, Some(Entity::Text(b)) if b.label_of == text.label_of);
                    if new
                        && let Some(of) = text.label_of
                        && self.store.slot_of(Uuid::from_bytes(of.0)).is_none()
                    {
                        out.push(Fault {
                            uid: change.uid,
                            message: "Bağlı yazının nesnesi çizimde yok.".into(),
                        });
                    }
                }
                Entity::Hatch(hatch) => {
                    let new = !matches!(before, Some(Entity::Hatch(b)) if b.assoc == hatch.assoc);
                    if new && let Some(assoc) = &hatch.assoc {
                        let gone = assoc
                            .objects()
                            .any(|id| self.store.slot_of(Uuid::from_bytes(id.0)).is_none());
                        if gone {
                            out.push(Fault {
                                uid: change.uid,
                                message: "İlişkili taramanın sınır nesnelerinden biri çizimde yok."
                                    .into(),
                            });
                        }
                    }
                }
                Entity::Dimension(dim) => {
                    let new = !matches!(before, Some(Entity::Dimension(b)) if b.refs == dim.refs);
                    if new && let Some(refs) = &dim.refs {
                        let gone = refs.each().any(|(_, r)| {
                            r.live() && self.store.slot_of(Uuid::from_bytes(r.uid.0)).is_none()
                        });
                        if gone {
                            out.push(Fault {
                                uid: change.uid,
                                message: "İlişkili ölçünün bağlı olduğu nesne çizimde yok.".into(),
                            });
                        }
                    }
                }
                _ => {}
            }
        }
        out
    }
}
