//! The document's integrity after a step (docs/adr/0253 §8): what a write
//! must not leave behind. Checked over what the step touched, once its
//! objects followed (the outermost step), so a command or a change set that
//! breaks it is taken back whole:
//!
//! - an object on a layer the tree does not have, or on a group;
//! - an insert of a block the drawing does not define;
//! - a linked text whose object is gone;
//! - an associative hatch, or a dimension's live reference, to an object
//!   that is gone; or such a reference left to an object the step removed;
//! - an object of a kind its layer does not hold (a layer's type, docs/adr/0622 §5):
//!   one the step put on the layer, or whose kind it changed.
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
    /// Its code when it has its own (`layer_geometry_mismatch`); none: the integrity's.
    pub code: Option<&'static str>,
}

impl Document {
    /// What the step reported would leave broken; none when nothing.
    pub fn faults(&self, report: &StepReport) -> Vec<Fault> {
        let mut out = self.dimension_set_faults(report);
        // A kitchen run's objects are its own (docs/adr/0727 §5).
        out.extend(self.casework_faults(report));
        let cached: std::collections::HashSet<_> =
            report.dimension_set_cached.iter().copied().collect();
        // A Veri girişi's project (docs/adr/0411 §5): existing records' attributes alone.
        if self.object_policy == crate::document::ObjectPolicy::AttributesOnly {
            for change in &report.objects {
                let attributes_only = match (change.before.as_deref(), change.after.as_deref()) {
                    (Some(before), Some(after)) => {
                        let mut probe = after.clone();
                        probe.base_mut().attrs = before.base().attrs.clone();
                        if cached.contains(&change.uid)
                            && let (Entity::Dimension(p), Entity::Dimension(b)) =
                                (&mut probe, before)
                        {
                            p.offset = b.offset;
                        }
                        probe == *before
                    }
                    _ => false,
                };
                if !attributes_only {
                    out.push(Fault {
                        uid: change.uid,
                        message: "Bu projede Veri girişi rolündesiniz: yalnız kayıtların özniteliklerini formla değiştirebilirsiniz; nesne eklenmez, silinmez, şekli ve görünüşü değişmez.".into(),
                        code: None,
                    });
                }
            }
            // Through the layer's live form, the fields it shows (docs/adr/0764 §3): the server's rule.
            if out.is_empty() {
                out.extend(self.entry_write_faults(report));
            }
            if !out.is_empty() {
                return out;
            }
        }
        for change in &report.objects {
            let Some(after) = &change.after else {
                // Removed: nothing may still follow it.
                let linked = self.store.linked_to(change.uid).next().is_some();
                // An opening stays with its wall gone, broken (`hostGone`, docs/adr/0341 §3); a column with its grid gone
                // (`gridGone`, docs/adr/0343 §3); a tag with its object gone (`targetGone`, docs/adr/0344 §1).
                let tied = self.store.tied_to(change.uid).any(|slot| {
                    !matches!(
                        self.store.get(slot).map(|s| &*s.entity),
                        Some(
                            kentos_contracts::Entity::Opening(_)
                                | kentos_contracts::Entity::Column(_)
                                | kentos_contracts::Entity::Tag(_)
                        )
                    )
                });
                if linked || tied {
                    out.push(Fault {
                        uid: change.uid,
                        message: "Silinen nesneye hâlâ bir yazı, tarama ya da ölçü bağlı.".into(),
                        code: None,
                    });
                }
                continue;
            };
            let before = change.before.as_deref();
            let base = after.base();
            let moved = before.is_none_or(|b| b.base().layer_id != base.layer_id);
            // The layer's type (docs/adr/0622 §5): an object the step put on it, or whose kind it changed.
            let retyped = before.is_none_or(|b| b.kind() != after.kind());
            if (moved || retyped)
                && let Some(node) = self.layers.get(&base.layer_id)
                && let Some(holds) = node.geometry
                && !holds.admits(after.kind())
            {
                out.push(Fault {
                    uid: change.uid,
                    message: format!(
                        "“{}” bir {}; {} bu katmana yazılmaz. Nesneyi türüne uyan bir katmana yazın ya da katmanın türünü Karışık yapın.",
                        node.name,
                        holds.layer_words(),
                        kentos_contracts::kind_words(after.kind())
                    ),
                    code: Some(kentos_contracts::codes::LAYER_GEOMETRY_MISMATCH),
                });
            }
            if moved {
                match self.layers.get(&base.layer_id) {
                    None => out.push(Fault {
                        uid: change.uid,
                        message: format!("Nesne çizimde olmayan “{}” katmanında.", base.layer_id),
                        code: None,
                    }),
                    Some(node) if node.kind == LayerNodeType::Group => out.push(Fault {
                        uid: change.uid,
                        message: format!(
                            "Nesne “{}” grubuna konmuş; nesne yalnız katmana konur.",
                            node.name
                        ),
                        code: None,
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
                            code: None,
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
                            code: None,
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
                                code: None,
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
                                code: None,
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

impl Document {
    /// A Veri girişi's writes its forms do not let (docs/adr/0764 §3): a record of a layer no live form
    /// writes, a field its layer's live forms do not show.
    fn entry_write_faults(&self, report: &StepReport) -> Vec<Fault> {
        let mut reach = None;
        let mut out = Vec::new();
        for change in &report.objects {
            let (Some(before), Some(after)) = (change.before.as_deref(), change.after.as_deref())
            else {
                continue;
            };
            if before.base().attrs == after.base().attrs {
                continue;
            }
            let reach = reach.get_or_insert_with(|| {
                kentos_forms::reach::Reach::new(
                    &self.settings.forms,
                    |l| self.layers.get(l).map_or(&[][..], |n| n.fields.as_slice()),
                    &[],
                    kentos_expression::Unit::per_metre(self.settings.unit().per_metre()),
                    kentos_forms::reach::Caller::through_forms(),
                )
            });
            let layer = &before.base().layer_id;
            if let Some(why) = kentos_forms::entry_write_refusal(
                reach,
                layer,
                &before.base().attrs,
                &after.base().attrs,
            ) {
                let name = self
                    .layers
                    .get(layer)
                    .map_or_else(|| layer.clone(), |n| n.name.clone());
                out.push(Fault {
                    uid: change.uid,
                    message: why.message(&name),
                    code: None,
                });
            }
        }
        out
    }
}
