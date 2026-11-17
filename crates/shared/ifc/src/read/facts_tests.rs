//! The written file's facts (docs/adr/8074 §2) on the files KentOS writes: every object and its place, EK-7's records,
//! the catalog's gaps by GlobalId, EK-5's codes; the bodies' heights, the spaces closed and apart.

use serde_json::Value as Json;

use super::tests::{WRITTEN, never, read_ok};

/// A file KentOS wrote with one of its doors and the records only it names taken out by another program
/// (scripts/fixtures/ifc_missing_door_case.py): EK-9/8's acceptance case (docs/adr/8074 §7).
const MISSING_DOOR: (&str, &[u8]) =
    ("kesit-binasi-kapi-eksik", include_bytes!("../../../../../fixtures/ifc/v1/eksik/kesit-binasi-kapi-eksik.ifc"));

fn facts_of(name: &str) -> (Json, Json) {
    let (_, bytes) = WRITTEN.iter().chain([&MISSING_DOOR]).find(|(n, _)| *n == name).copied().expect("a written file");
    let m = read_ok(bytes);
    let facts = m.facts(bytes, &format!("{name}.ifc")).expect("the facts");
    let bodies = m.bodies(bytes, &never).expect("the bodies");
    (facts, m.body_facts(&bodies))
}

#[test]
fn the_section_building_s_facts_name_every_object_and_its_place() {
    let (f, _) = facts_of("kesit-binasi");
    assert_eq!((f["format"].as_str(), f["version"].as_u64()), (Some("kentos.ifcFacts"), Some(1)));
    assert_eq!(f["file"]["extension"], "ifc");
    assert_eq!(f["file"]["schema"], "IFC4X3_ADD2");
    let objects = f["objects"].as_array().expect("objects");
    let wall = objects.iter().find(|o| o["globalId"] == "01alMXGa9m080000000001").expect("the outer wall");
    assert_eq!(wall["class"], "IfcWall");
    assert_eq!(wall["sets"]["Pset_WallCommon"]["IsExternal"], true);
    assert_eq!(wall["quantities"]["Qto_WallBaseQuantities"]["Length"], 12.0);
    assert_eq!(wall["materials"][0], "Tuğla");
    let storey = |gid: &Json| objects.iter().find(|o| &o["globalId"] == gid).expect("its storey");
    assert_eq!(storey(&wall["storey"])["class"], "IfcBuildingStorey");
    assert!(storey(&wall["storey"])["elevation"].is_number(), "a storey's elevation");
    // EK-5's category code of a wall.
    assert_eq!(f["classCodes"]["IfcWall"], "DVR");
    // EK-7's rows are there for the project's information.
    assert!(f["rows"].as_array().is_some_and(|r| r.iter().any(|r| r["class"] == "IfcPerson")));
    // A gap names its objects by GlobalId: no fire rating on the section building's walls (docs/adr/8068).
    let fire = f["gaps"].as_array().expect("gaps").iter().find(|g| g["path"] == "Pset_WallCommon.FireRating");
    let fire = fire.expect("the walls' fire rating is missing");
    // KentOS's standard sets are the IFC 4.3 dictionary's, on the classes they are for.
    assert_eq!(f["setChecks"].as_array().map(Vec::len), Some(0), "{}", f["setChecks"]);
    assert_eq!(f["profile"]["schema"], "IFC4X3_ADD2");
    assert!(fire["missing"].as_array().is_some_and(|m| m.iter().any(|g| g == "01alMXGa9m080000000001")));
}

#[test]
fn the_kunye_is_in_ek7_s_records() {
    let (f, _) = facts_of("kesit-binasi-kunye");
    let person = &f["records"]["IfcPerson"][0]["attributes"];
    assert!(person["FamilyName"].is_string() && person["Identification"].is_string(), "{person}");
    let org = &f["records"]["IfcOrganization"][0]["attributes"];
    assert!(org["Name"].is_string(), "{org}");
    assert!(f["records"]["IfcActorRole"][0]["attributes"]["Role"].is_string());
}

#[test]
fn the_spaces_are_closed_with_their_footprints_and_none_overlaps() {
    let (_, b) = facts_of("kesit-binasi");
    let spaces = b["spaces"].as_array().expect("spaces");
    assert_eq!(spaces.len(), 10);
    for s in spaces {
        assert_eq!(s["closed"], true, "{s}");
        assert!(s["area"].as_f64().is_some_and(|a| a > 1.0), "{s}");
        assert!(s["storey"].is_string(), "{s}");
    }
    assert_eq!(b["overlaps"].as_array().map(Vec::len), Some(0));
    // Every body's heights with their bound: the outer wall from under the ground floor's slab to under the next.
    let wall = b["bodies"].as_array().expect("bodies").iter().find(|x| x["globalId"] == "01alMXGa9m080000000001").expect("the wall");
    let (lo, hi, bound) = (wall["zMin"].as_f64().unwrap_or(0.0), wall["zMax"].as_f64().unwrap_or(0.0), wall["rounding"].as_f64().unwrap_or(0.0));
    assert!((lo + 0.13).abs() <= bound && (hi - 2.43).abs() <= bound, "{wall}");
}

/// The facts the standards package's form is tried on (catalog/standards/dijital-proje/tests/test_form.py), the
/// bodies' under `bodies`: written by `KENTOS_WRITE_IFC_FACTS=1`, else compared.
#[test]
fn the_facts_the_form_is_tried_on_are_current() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/ifc/v1/olgular");
    for name in ["kesit-binasi", "kesit-binasi-kunye", "kesit-binasi-tam", "yerel-sistem", "kesit-binasi-kapi-eksik"] {
        let (mut f, b) = facts_of(name);
        f["bodies"] = b;
        let text = serde_json::to_string_pretty(&f).expect("the facts as text") + "\n";
        let path = dir.join(format!("{name}.json"));
        if std::env::var_os("KENTOS_WRITE_IFC_FACTS").is_some() {
            std::fs::create_dir_all(&dir).expect("the folder");
            std::fs::write(&path, &text).expect("writes the facts");
        } else {
            let kept = std::fs::read_to_string(&path).unwrap_or_default();
            assert!(kept == text, "{}: not current; KENTOS_WRITE_IFC_FACTS=1 writes it, read the difference", path.display());
        }
    }
}
