//! One immutable tag read shares room source networks; it never owns a cache across edits.
use std::cell::{OnceCell, RefCell};

use kentos_contracts::{Entity, SpaceFields, SpaceMethod, TagBreak, TagFields, TagPiece, Vec2};

use crate::{
    Document,
    arch_values::{ArchValue, core_areas},
    space_follow::SpaceSearch,
};

impl SpaceSearch<'_> {
    /// Inner-face area under this room's source and column policy, not a relabelled gross area.
    pub fn net_area(&mut self, room: &SpaceFields) -> Option<f64> {
        if room.broken.is_some() {
            return None;
        }
        if room.method == SpaceMethod::Net && room.seed.is_none() {
            return Some(kentos_geometry_core::geom::space::area(&core_areas(
                &room.parts,
            )));
        }
        room.seed?; // A drawn non-net outline does not tell us where the inner faces are.
        let mut net = room.clone();
        net.method = SpaceMethod::Net;
        let net = self.found_preserving_legacy(&net);
        net.broken
            .is_none()
            .then(|| kentos_geometry_core::geom::space::area(&core_areas(&net.parts)))
    }
}

/// Shared by the semantic preview, write follower and settings/history refresh.
pub struct TagRead<'a> {
    doc: &'a Document,
    search: RefCell<SpaceSearch<'a>>,
}

impl<'a> TagRead<'a> {
    pub fn new(doc: &'a Document) -> Self {
        Self {
            doc,
            search: RefCell::new(SpaceSearch::new(doc)),
        }
    }

    pub fn text(&self, entity: &Entity, content: Option<&str>) -> String {
        let template = content
            .or_else(|| kentos_contracts::default_template(entity.kind()))
            .unwrap_or("");
        let Ok(pieces) = kentos_contracts::tag_template(template) else {
            return String::new();
        };
        // A template may repeat the net-area token; query its source only once.
        let net = OnceCell::new();
        kentos_contracts::tag_fill(&pieces, &|piece| {
            let value = match piece {
                TagPiece::Place("netArea") => {
                    let Entity::Space(room) = entity else {
                        return None;
                    };
                    net.get_or_init(|| self.search.borrow_mut().net_area(&room.space))
                        .map(ArchValue::Area)
                }
                TagPiece::Place(name) => self.doc.arch_value(entity, name, None),
                TagPiece::Attr(name) => self.doc.arch_value(entity, "attr", Some(name)),
                TagPiece::Words(_) => None,
            }?;
            Some(self.doc.arch_value_text(&value, true))
        })
    }

    pub fn settled(&self, tag: &TagFields, follow: bool) -> TagFields {
        let mut out = tag.clone();
        match self.doc.tag_target(&tag.of) {
            Some(target) => {
                out.text = self.text(&target, tag.content.as_deref());
                let anchor = self.doc.tag_anchor(&target);
                if follow && let (Some(old), Some(new)) = (tag.anchor, anchor) {
                    out.at = Vec2 {
                        x: tag.at.x + (new.x - old.x),
                        y: tag.at.y + (new.y - old.y),
                    };
                }
                out.anchor = anchor;
                out.broken = None;
            }
            None => out.broken = Some(TagBreak::TargetGone),
        }
        out
    }
}
