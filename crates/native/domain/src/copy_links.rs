//! The links among objects copied together (docs/adr/0793 §2): a copy, an array's items and a paste write each copy's
//! link to an object copied with it onto that object's copy, as a room's sources and an opening's wall go (docs/adr/0730
//! §4). A tag copied with its object tags the copy (Revit's tag goes with its element); a label, an associative
//! hatch, a coordinate, area or attribute table and an associative dimension the paste keeps (docs/adr/0309 §1) tie
//! to the copies. A link to an object not copied stays as it was; a view mark names a view, not an object.

use std::collections::HashMap;

use kentos_contracts::{Entity, EntityId, TableSource};
use uuid::Uuid;

/// The copies' ids by their originals' (16 bytes each).
pub type Copies = HashMap<[u8; 16], [u8; 16]>;

fn moved(id: &mut EntityId, map: &Copies) {
    if let Some(to) = map.get(&id.0) {
        id.0 = *to;
    }
}

/// Whether `e` links to one of `originals`.
pub fn links_to(e: &Entity, originals: &Copies) -> bool {
    let named = |id: &EntityId| originals.contains_key(&id.0);
    match e {
        Entity::Tag(t) => Uuid::parse_str(&t.tag.of).is_ok_and(|u| originals.contains_key(u.as_bytes())),
        Entity::Text(t) => t.label_of.as_ref().is_some_and(named),
        Entity::Hatch(h) => h.assoc.as_ref().is_some_and(|a| a.objects().any(named)),
        Entity::Table(t) => t.source.as_ref().is_some_and(|s| s.objects().iter().any(named)),
        Entity::Dimension(d) => d
            .refs
            .as_ref()
            .is_some_and(|r| [&r.a, &r.b, &r.c].into_iter().flatten().any(|s| named(&s.uid))),
        _ => false,
    }
}

/// `e`'s links to an original moved onto its copy.
pub fn relink(e: &mut Entity, map: &Copies) {
    match e {
        Entity::Tag(t) => {
            if let Some(to) = Uuid::parse_str(&t.tag.of).ok().and_then(|u| map.get(u.as_bytes())) {
                t.tag.of = Uuid::from_bytes(*to).to_string();
            }
        }
        Entity::Text(t) => {
            if let Some(id) = &mut t.label_of {
                moved(id, map);
            }
        }
        Entity::Hatch(h) => {
            if let Some(a) = &mut h.assoc {
                if let Some(id) = &mut a.outer {
                    moved(id, map);
                }
                for id in a.bounds.iter_mut().chain(&mut a.islands).chain(&mut a.cutouts) {
                    moved(id, map);
                }
            }
        }
        Entity::Table(t) => {
            if let Some(
                TableSource::Coordinates { objects }
                | TableSource::Areas { objects }
                | TableSource::Attributes { objects },
            ) = &mut t.source
            {
                for id in objects {
                    moved(id, map);
                }
            }
        }
        Entity::Dimension(d) => {
            if let Some(r) = &mut d.refs {
                for s in [&mut r.a, &mut r.b, &mut r.c].into_iter().flatten() {
                    moved(&mut s.uid, map);
                }
            }
        }
        _ => {}
    }
}
