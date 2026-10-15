//! Denetle's rules (docs/adr/0306; AutoCAD's AUDIT and PURGE): what a drawing carries broken, how each object is
//! repaired, and which named styles nothing uses. The reader already refuses an object on a layer the file does not
//! have and an insert of a block it does not define (`snapshot.rs`); what it lets through is found here:
//!
//! - a linked text whose object is gone (repair: the link goes, the text stays);
//! - an associative hatch one of whose boundary objects is gone (repair: it is no longer associative);
//! - an associative dimension's live reference to an object that is gone (repair: the reference breaks as a deletion
//!   breaks it, `object_gone`, the point keeping its place);
//! - a text or a table naming a text style the project does not have, a dimension naming a dimension style it does not
//!   have (repair: it follows Standart, its look kept).
//!
//! A style is used when an object of the drawing or of a block definition names it.

use std::collections::HashSet;

use kentos_contracts::Entity;

use crate::document::Document;
use crate::identity::{Slot, Uuid};

/// What is broken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditKind {
    BrokenLink,
    BrokenHatch,
    BrokenDimension,
    MissingTextStyle,
    MissingDimensionStyle,
}

/// A fault the drawing carries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditFault {
    pub kind: AuditKind,
    pub slot: Slot,
    /// Turkish, for people: what is broken and what the repair does.
    pub message: String,
}

impl Document {
    fn gone(&self, uid: [u8; 16]) -> bool {
        self.slot_of(Uuid::from_bytes(uid)).is_none()
    }

    fn text_style_known(&self, id: &str) -> bool {
        self.settings().text_styles.iter().any(|s| s.id == id)
    }

    fn dimension_style_known(&self, id: &str) -> bool {
        self.settings().dimension_styles.iter().any(|s| s.id == id)
    }

    /// The faults of one object, in the kinds' order.
    pub fn audit_object(&self, slot: Slot) -> Vec<AuditFault> {
        let Some(e) = self.get(slot) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let mut say = |kind: AuditKind, message: &str| {
            out.push(AuditFault {
                kind,
                slot,
                message: message.to_owned(),
            });
        };
        match e {
            Entity::Text(t) => {
                if t.label_of.is_some_and(|of| self.gone(of.0)) {
                    say(
                        AuditKind::BrokenLink,
                        "Bağlı yazının nesnesi çizimde yok; onarım bağı koparır, yazı kalır.",
                    );
                }
                if t.face
                    .text_style
                    .as_deref()
                    .is_some_and(|s| !self.text_style_known(s))
                {
                    say(
                        AuditKind::MissingTextStyle,
                        "Yazının stili projede yok; onarım yazıyı Standart'a bağlar, görünüşü kalır.",
                    );
                }
            }
            Entity::Table(t) => {
                if t.face
                    .text_style
                    .as_deref()
                    .is_some_and(|s| !self.text_style_known(s))
                {
                    say(
                        AuditKind::MissingTextStyle,
                        "Tablonun yazı stili projede yok; onarım tabloyu Standart'a bağlar, görünüşü kalır.",
                    );
                }
            }
            Entity::Hatch(h) => {
                if let Some(assoc) = &h.assoc {
                    let gone = std::iter::once(&assoc.outer)
                        .chain(&assoc.islands)
                        .chain(&assoc.cutouts)
                        .any(|id| self.gone(id.0));
                    if gone {
                        say(
                            AuditKind::BrokenHatch,
                            "İlişkili taramanın sınır nesnelerinden biri çizimde yok; onarım ilişkiyi koparır, tarama kalır.",
                        );
                    }
                }
            }
            Entity::Dimension(d) => {
                if d.refs
                    .as_ref()
                    .is_some_and(|r| r.each().any(|(_, r)| r.live() && self.gone(r.uid.0)))
                {
                    say(
                        AuditKind::BrokenDimension,
                        "İlişkili ölçünün bağlı olduğu nesne çizimde yok; onarım bağı “nesne yok” nedeniyle koparır, nokta yerinde kalır.",
                    );
                }
                if d.look
                    .dim_style
                    .as_deref()
                    .is_some_and(|s| !self.dimension_style_known(s))
                {
                    say(
                        AuditKind::MissingDimensionStyle,
                        "Ölçünün stili projede yok; onarım ölçüyü Standart'a bağlar, görünüşü kalır.",
                    );
                }
            }
            _ => {}
        }
        out
    }

    /// The object with its faults repaired; none when it has none.
    pub fn audit_repaired(&self, slot: Slot) -> Option<Entity> {
        let faults = self.audit_object(slot);
        if faults.is_empty() {
            return None;
        }
        let mut e = self.get(slot)?.clone();
        for f in &faults {
            match (&mut e, f.kind) {
                (Entity::Text(t), AuditKind::BrokenLink) => {
                    t.label_of = None;
                    t.label_scale = None;
                }
                (Entity::Text(t), AuditKind::MissingTextStyle) => t.face.text_style = None,
                (Entity::Table(t), AuditKind::MissingTextStyle) => t.face.text_style = None,
                (Entity::Hatch(h), AuditKind::BrokenHatch) => h.assoc = None,
                (Entity::Dimension(d), AuditKind::BrokenDimension) => {
                    if let Some(refs) = d.refs.as_mut() {
                        for r in [&mut refs.a, &mut refs.b, &mut refs.c]
                            .into_iter()
                            .flatten()
                        {
                            if r.live() && self.slot_of(Uuid::from_bytes(r.uid.0)).is_none() {
                                r.broken = Some("object_gone".into());
                            }
                        }
                    }
                }
                (Entity::Dimension(d), AuditKind::MissingDimensionStyle) => d.look.dim_style = None,
                _ => {}
            }
        }
        Some(e)
    }

    /// Every fault of the drawing, object by object in its order.
    pub fn audit(&self) -> Vec<AuditFault> {
        self.entities()
            .flat_map(|e| self.audit_object(Slot(e.base().id)))
            .collect()
    }

    /// The project's text and dimension styles nothing names (an object of the drawing or of a block definition),
    /// by their ids, in the tables' order.
    pub fn unused_styles(&self) -> (Vec<String>, Vec<String>) {
        let mut text: HashSet<&str> = HashSet::new();
        let mut dims: HashSet<&str> = HashSet::new();
        let defined = self.blocks().iter().flat_map(|b| b.entities.iter());
        for e in self.entities().chain(defined) {
            match e {
                Entity::Text(t) => text.extend(t.face.text_style.as_deref()),
                Entity::Table(t) => text.extend(t.face.text_style.as_deref()),
                Entity::Dimension(d) => dims.extend(d.look.dim_style.as_deref()),
                _ => {}
            }
        }
        let s = self.settings();
        (
            s.text_styles
                .iter()
                .filter(|t| !text.contains(t.id.as_str()))
                .map(|t| t.id.clone())
                .collect(),
            s.dimension_styles
                .iter()
                .filter(|d| !dims.contains(d.id.as_str()))
                .map(|d| d.id.clone())
                .collect(),
        )
    }
}
