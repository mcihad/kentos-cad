//! The objects a building's file is expected to hold (docs/adr/8074 §2; plan/ifc-uyum.md IFC-08): each by the GlobalId
//! it is written with, the KentOS object it comes from and its IFC class, from the building to write (the export's
//! mapping of the drawing), not from the file. The model quality control form compares them with the file read back:
//! an object of this list the file lacks is EK-9/8's finding, one of another class EK-9/17's, whatever the file's other
//! checks say.
//!
//! Only the objects with a KentOS id are listed: a part the export makes without one (its GlobalId derived from its
//! owner and role, `crate::step::derived_id`, docs/adr/8117) has no object of the drawing to name.

use crate::building::Building;
use crate::step::guid;

/// An object the file is expected to hold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expected {
    /// The GlobalId it is written with.
    pub global_id: String,
    /// The KentOS object it comes from (a UUID's 128 bits): its own id, or the one it shows (a roof's chimney stack:
    /// the roof).
    pub object: u128,
    /// Its IFC class, as STEP spells it (`IFCWALL`).
    pub class: &'static str,
}

/// The building's objects with a KentOS id, in the file's order of kinds: the project, the site, the building, the
/// storeys, the elements, the openings, the spaces, the zones, the groups and the grids.
pub fn of(b: &Building) -> Vec<Expected> {
    let mut out = Vec::new();
    let mut put = |id: Option<u128>, object: Option<u128>, class: &'static str| {
        if let Some(id) = id.filter(|&id| !crate::step::is_derived(id)) {
            out.push(Expected { global_id: guid(id), object: object.unwrap_or(id), class });
        }
    };
    put(b.project.id, None, "IFCPROJECT");
    put(b.site.id, None, "IFCSITE");
    put(b.building.id, None, "IFCBUILDING");
    for s in &b.storeys {
        put(s.id, None, "IFCBUILDINGSTOREY");
    }
    for e in &b.elements {
        put(e.id, e.object, e.class.entity());
    }
    for o in &b.openings {
        put(o.id, None, "IFCOPENINGELEMENT");
    }
    for s in &b.spaces {
        put(s.id, None, "IFCSPACE");
    }
    for z in &b.zones {
        put(z.id, None, "IFCSPATIALZONE");
    }
    for g in &b.groups {
        put(g.id, None, "IFCZONE");
    }
    for g in &b.grids {
        put(g.id, None, "IFCGRID");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::building::{Class, Element, Place, Storey};

    fn element(class: Class, id: Option<u128>, object: Option<u128>) -> Element {
        Element {
            class,
            id,
            object,
            name: String::new(),
            description: None,
            tag: None,
            predefined: "NOTDEFINED",
            object_type: None,
            part_of: None,
            storey: 0,
            shapes: Vec::new(),
            overall: None,
            material: None,
            psets: Vec::new(),
            qtos: Vec::new(),
        }
    }

    #[test]
    fn an_object_is_named_by_its_global_id_and_the_object_it_shows() {
        let roof = 0x0123_4567_89ab_cdef_0123_4567_89ab_cdef_u128;
        let storey = Storey { id: Some(8), name: String::new(), long_name: None, elevation: 0.0, psets: Vec::new(), qtos: Vec::new() };
        let b = Building {
            project: Place { id: Some(7), ..Place::default() },
            storeys: vec![storey],
            elements: vec![
                element(Class::Roof, Some(roof), None),
                // The roof's chimney stack: its own GlobalId, the roof its object.
                element(Class::Chimney, Some(roof + 1), Some(roof)),
                // A part without a KentOS id is not listed.
                element(Class::Member, None, None),
            ],
            ..Building::default()
        };
        let e = of(&b);
        assert_eq!(e.iter().map(|x| x.class).collect::<Vec<_>>(), ["IFCPROJECT", "IFCBUILDINGSTOREY", "IFCROOF", "IFCCHIMNEY"]);
        assert_eq!(e[2].global_id, guid(roof));
        assert_eq!((e[3].global_id.clone(), e[3].object), (guid(roof + 1), roof));
    }
}
