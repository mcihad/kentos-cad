//! A file read as the model viewer's input (feature `view`; docs/adr/8061 §3.2, docs/adr/8073 §2): the IFC code that
//! turns the reading face's objects and the engine's bodies into the viewer core's plain input, so the core knows no
//! IFC name. Each object's category comes from its class by the file's own schema's inheritance (an `IfcWallStandardCase`
//! is a wall; a spatial zone its own see-through volume, docs/adr/8123); its tree parent is what it is a part of, else the
//! structure that contains it, an opening's the body it cuts, a zone's the structure that references it; its storey the
//! nearest storey above it; its label its name (a storey's and a space's long name beside it), else its class.

use std::sync::Arc;

use kentos_model_view::{BodyIn, Category, Input, ObjectIn};

use super::bodies::Bodies;
use super::{Model, header};

/// The classes whose objects are which category, the first the object is a kind of winning.
const CATEGORIES: [(&str, Category); 27] = [
    ("IfcProject", Category::Project),
    ("IfcFeatureElementSubtraction", Category::Opening),
    ("IfcSpatialZone", Category::Zone),
    ("IfcSite", Category::Site),
    ("IfcBuilding", Category::Building),
    ("IfcFacility", Category::Building),
    ("IfcBuildingStorey", Category::Storey),
    ("IfcFacilityPart", Category::Storey),
    ("IfcSpace", Category::Space),
    ("IfcCurtainWall", Category::Window),
    ("IfcWall", Category::Wall),
    ("IfcSlab", Category::Slab),
    ("IfcCovering", Category::Slab),
    ("IfcPlate", Category::Slab),
    ("IfcColumn", Category::Column),
    ("IfcBeam", Category::Beam),
    ("IfcMember", Category::Beam),
    ("IfcFooting", Category::Footing),
    ("IfcRoof", Category::Roof),
    ("IfcStair", Category::Stair),
    ("IfcStairFlight", Category::Stair),
    ("IfcRamp", Category::Stair),
    ("IfcRampFlight", Category::Stair),
    ("IfcRailing", Category::Railing),
    ("IfcDoor", Category::Door),
    ("IfcWindow", Category::Window),
    ("IfcFurnishingElement", Category::Furnishing),
];

/// The viewer's input of `model` (read from a file named `title`) and its `bodies`, the openings' volumes among them
/// when the host made them ([`Model::opening_bodies`]).
pub fn input(title: &str, model: &Model, bodies: &Bodies) -> Input {
    let schema = header::version(model.summary.family);
    let category = |class: &str| {
        CATEGORIES
            .iter()
            .find(|(of, _)| schema.is_subtype_of(class, of))
            .map_or_else(|| furnishing(schema, class), |(_, c)| *c)
    };
    let mut objects: Vec<ObjectIn> = model
        .objects
        .iter()
        .map(|o| ObjectIn {
            key: o.global_id.clone(),
            class: o.class.clone(),
            label: label(o),
            // A base slab is the building's foundation (IFC 4.3's IfcSlabTypeEnum, ADR 8172 §6): drawn as Temel.
            category: match category(&o.class) {
                Category::Slab if o.predefined.as_deref() == Some("BASESLAB") => Category::Footing,
                c => c,
            },
            parent: o.parent.or(o.container),
            storey: None,
        })
        .collect();
    // An opening hangs under the body it cuts (`IfcRelVoidsElement`), not at the tree's root.
    for r in model
        .relations
        .iter()
        .filter(|r| schema.is_subtype_of(&r.class, "IfcRelVoidsElement"))
    {
        let host = r.relating.and_then(|h| model.index_of(h));
        for o in r.related.iter().filter_map(|&o| model.index_of(o)) {
            if objects[o].parent.is_none() && host != Some(o) {
                objects[o].parent = host;
            }
        }
    }
    // What a structure references without containing it (`IfcRelReferencedInSpatialStructure`: a storey's far zone,
    // docs/adr/8069) hangs under the first one when nothing else holds it (docs/adr/8119): the tree shows it there and
    // hiding the storey hides it. A structure under the object itself is passed over.
    for r in model
        .relations
        .iter()
        .filter(|r| schema.is_subtype_of(&r.class, "IfcRelReferencedInSpatialStructure"))
    {
        let Some(structure) = r.relating.and_then(|h| model.index_of(h)) else {
            continue;
        };
        for o in r.related.iter().filter_map(|&o| model.index_of(o)) {
            if objects[o].parent.is_none()
                && objects[o].category != Category::Project
                && !reaches(&objects, structure, o)
            {
                objects[o].parent = Some(structure);
            }
        }
    }
    let n = objects.len();
    for i in 0..n {
        let mut at = objects[i].parent;
        for _ in 0..n {
            let Some(p) = at.filter(|&p| p < n) else {
                break;
            };
            if objects[p].category == Category::Storey {
                objects[i].storey = Some(p);
                break;
            }
            at = objects[p].parent;
        }
    }
    Input {
        title: title.to_owned(),
        objects,
        bodies: bodies
            .bodies
            .iter()
            .map(|b| BodyIn {
                object: b.object,
                positions: Arc::new(b.positions.clone()),
                triangles: b.triangles.clone(),
            })
            .collect(),
        not_shown: bodies
            .not_shown
            .iter()
            .map(|n| (n.object, n.reason.clone()))
            .collect(),
        apart: None,
    }
}

/// Whether `to` is `from` or above it in the tree as it stands (a parent out of range ends the walk).
fn reaches(objects: &[ObjectIn], from: usize, to: usize) -> bool {
    let mut at = Some(from);
    for _ in 0..=objects.len() {
        match at {
            Some(p) if p == to => return true,
            Some(p) => at = objects.get(p).and_then(|o| o.parent),
            None => return false,
        }
    }
    true
}

/// The blocks that are building elements beside the furniture (a sanitary fixture, a lift): furnishings' colour.
fn furnishing(schema: ifc_lite_core::SchemaVersion, class: &str) -> Category {
    if [
        "IfcElectricAppliance",
        "IfcSanitaryTerminal",
        "IfcTransportElement",
        "IfcFurniture",
    ]
    .iter()
    .any(|of| schema.is_subtype_of(class, of))
    {
        Category::Furnishing
    } else {
        Category::Other
    }
}

/// What the tree says of an object: its name, a storey's and a space's long name beside it, else its class.
fn label(o: &super::Object) -> String {
    let name = o.name.as_deref().filter(|s| !s.trim().is_empty());
    let long = o
        .long_name
        .as_deref()
        .filter(|s| !s.trim().is_empty() && Some(*s) != name);
    match (name, long) {
        (Some(n), Some(l)) => format!("{n} — {l}"),
        (Some(n), None) => n.to_owned(),
        (None, Some(l)) => l.to_owned(),
        (None, None) => o.class.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{WRITTEN, never, read_ok};
    use super::*;

    #[test]
    fn the_section_building_becomes_the_viewer_s_input_whole() {
        let (name, bytes) = WRITTEN[0];
        let m = read_ok(bytes);
        let b = m.bodies(bytes, &never).expect("the bodies");
        let input = input(name, &m, &b);
        assert_eq!(input.objects.len(), m.objects.len());
        // 58 elements, 10 spaces and the 4 far zones (each storey's included and excluded area, docs/adr/8069; two
        // before the fixture was written again, docs/adr/8117).
        assert_eq!(input.bodies.len(), 72, "every body the file holds");
        let storeys: Vec<&str> = input
            .objects
            .iter()
            .filter(|o| o.category == Category::Storey)
            .map(|o| o.label.as_str())
            .collect();
        assert_eq!(storeys.len(), 2, "{storeys:?}");
        let view = kentos_model_view::View::new(input.clone());
        let roots: Vec<Category> = view
            .roots()
            .iter()
            .map(|&r| input.objects[r].category)
            .collect();
        assert!(roots.contains(&Category::Project), "{roots:?}");
        // Every wall is on a storey and has the wall's category.
        let walls: Vec<&ObjectIn> = input
            .objects
            .iter()
            .filter(|o| o.class == "IfcWall")
            .collect();
        assert!(
            !walls.is_empty()
                && walls
                    .iter()
                    .all(|w| w.category == Category::Wall && w.storey.is_some())
        );
        // The far zones each storey references (docs/adr/8069) hang under that storey, not at the root: two a storey.
        let zones: Vec<&ObjectIn> = input
            .objects
            .iter()
            .filter(|o| o.class == "IfcSpatialZone")
            .collect();
        assert_eq!(zones.len(), 4);
        for z in &zones {
            let storey = z.parent.map(|p| &input.objects[p]);
            assert!(
                storey.is_some_and(|s| s.category == Category::Storey) && z.storey == z.parent,
                "{z:?}"
            );
        }
        assert!(
            !roots.contains(&Category::Space),
            "no zone at the root: {roots:?}"
        );
    }

    #[test]
    fn a_base_slab_is_shown_as_a_foundation() {
        // A raft (docs/adr/8172 §6): IFC 4.3's slab foundation, an IfcSlab BASESLAB; a floor stays a slab.
        let bytes = b"ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
            FILE_NAME('radye.ifc','2026-11-27T12:00:00',(''),(''),'','','');\nFILE_SCHEMA(('IFC4X3_ADD2'));\nENDSEC;\n\
            DATA;\n#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'Proje',$,$,$,$,$,$);\n\
            #2=IFCSLAB('1YvctVUKr0kugbFTf53O9L',$,'MM-DSM-Radye',$,$,$,$,$,.BASESLAB.);\n\
            #3=IFCSLAB('2YvctVUKr0kugbFTf53O9L',$,'MM-DSM-D\\X2\\00F6\\X0\\\\X2\\015F\\X0\\eme',$,$,$,$,$,.FLOOR.);\n\
            ENDSEC;\nEND-ISO-10303-21;\n";
        let m = read_ok(bytes);
        let b = m.bodies(bytes, &never).expect("the bodies");
        let input = input("radye.ifc", &m, &b);
        let of = |name: &str| {
            input
                .objects
                .iter()
                .find(|o| o.label.contains(name))
                .map(|o| o.category)
        };
        assert_eq!(of("Radye"), Some(Category::Footing), "{:?}", input.objects);
        assert_eq!(
            input
                .objects
                .iter()
                .filter(|o| o.category == Category::Slab)
                .count(),
            1,
            "{:?}",
            input.objects
        );
    }
}
