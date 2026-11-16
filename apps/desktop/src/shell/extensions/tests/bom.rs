//! Malzeme listesi… (docs/adr/8057 §91): an installed extension's Python packages, from its installed lock, as
//! CycloneDX; an extension without Python is told it has none.

use super::*;

#[test]
fn an_installed_extensions_bill_of_materials_lists_its_locked_packages() {
    let dir = temp("malzeme");
    let mut gis = app(&dir, Some(Module::Gis));
    install(&mut gis, "vitrin.kek", super::demo::vitrin_package());
    let (text, file) = gis.extension_bom("tr.ornek.vitrin").expect("a list");
    assert!(
        file.starts_with("tr.ornek.vitrin-") && file.ends_with(".cdx.json"),
        "{file}"
    );
    let bom: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(bom["metadata"]["component"]["name"], "tr.ornek.vitrin");
    let names: Vec<&str> = bom["components"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c["name"].as_str())
        .collect();
    assert_eq!(names, ["numpy", "shapely"]);
    // Not installed, said so.
    assert!(gis.extension_bom("tr.ornek.yok").is_err());
}
