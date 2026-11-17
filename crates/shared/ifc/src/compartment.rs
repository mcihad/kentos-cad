//! Yangın kompartımanları as IFC groups (docs/adr/8088; Binaların Yangından Korunması Hakkında Yönetmelik 4/tt, 24;
//! EK-6 Tablo 6.68): each compartment is an `IfcZone` of its rooms' spaces (`IfcRelAssignsToGroup`), named by EK-5
//! (`MM-ZON-Yangın_kompartımanı_N`), its long name "Yangın kompartımanı N", its object type "Yangın kompartımanı". The
//! compartments are not found here: the rule is the standards package's (plan/bim-plan.md K10); this writes what it is
//! given.

use crate::building::{Building, Group};
use crate::naming::name;
use crate::step::fnv;

/// The object type every compartment's zone carries.
pub const OBJECT_TYPE: &str = "Yangın kompartımanı";

/// The compartments' zones added to `b`, numbered from 1 in the order given: each the spaces of its rooms (their
/// persistent ids' bits) the file holds. A compartment none of whose rooms the file holds (another building's, in one
/// building's file) is left out and takes no number. Its GlobalId comes from its first room's id, the same in every
/// file written while that room leads it. How many were added.
pub fn add(b: &mut Building, compartments: &[Vec<u128>]) -> usize {
    let mut n = 0;
    for rooms in compartments {
        let spaces: Vec<usize> = b
            .spaces
            .iter()
            .enumerate()
            .filter(|(_, s)| s.id.is_some_and(|id| rooms.contains(&id)))
            .map(|(i, _)| i)
            .collect();
        let Some(first) = rooms.iter().min() else { continue };
        if spaces.is_empty() {
            continue;
        }
        n += 1;
        let words = format!("{OBJECT_TYPE} {n}");
        b.groups.push(Group {
            id: Some(fnv(&format!("kentos-kompartiman:{first:032x}"), 0)),
            name: name("ZON", &words),
            long_name: Some(words),
            object_type: Some(OBJECT_TYPE.into()),
            spaces,
            psets: Vec::new(),
        });
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::building::Space;

    fn space(id: u128) -> Space {
        Space {
            id: Some(id),
            name: format!("MM-MHL-{id}"),
            long_name: None,
            storey: 0,
            rings: vec![vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]],
            bulges: Vec::new(),
            bottom: 0.0,
            top: 2.8,
            predefined: "INTERNAL",
            psets: Vec::new(),
            qtos: Vec::new(),
            body: Vec::new(),
        }
    }

    #[test]
    fn each_compartment_is_a_zone_of_the_spaces_the_file_holds() {
        let mut b = Building { spaces: vec![space(1), space(2), space(3)], ..Building::default() };
        // The second compartment's rooms are another building's: no zone, no number.
        let n = add(&mut b, &[vec![2, 1], vec![9], vec![3, 8], Vec::new()]);
        assert_eq!(n, 2);
        let zones: Vec<(&str, Option<&str>, Option<&str>, &[usize])> = b
            .groups
            .iter()
            .map(|g| (g.name.as_str(), g.long_name.as_deref(), g.object_type.as_deref(), g.spaces.as_slice()))
            .collect();
        assert_eq!(
            zones,
            [
                ("MM-ZON-Yangın_kompartımanı_1", Some("Yangın kompartımanı 1"), Some("Yangın kompartımanı"), &[0, 1][..]),
                ("MM-ZON-Yangın_kompartımanı_2", Some("Yangın kompartımanı 2"), Some("Yangın kompartımanı"), &[2][..]),
            ]
        );
        // The same first room, the same GlobalId; another, another.
        assert_eq!(b.groups[0].id, Some(fnv("kentos-kompartiman:00000000000000000000000000000001", 0)));
        assert_ne!(b.groups[0].id, b.groups[1].id);
    }
}
