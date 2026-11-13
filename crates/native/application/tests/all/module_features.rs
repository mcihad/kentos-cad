//! Which build compiles which module (docs/adr/8054 §11): a module in `arch/`, `map/`, `gis/` or `road/` is behind its
//! discipline's feature (a road module that reads a terrain surface behind `terrain`, which mapping also turns on),
//! unless the core reaches it. Those are listed here, by folder: a build of any module compiles them. The list only
//! shrinks: a module the core stops reaching goes behind its feature and leaves the list; a new one the core reaches
//! is a choice to write down here, not a side effect.

use std::collections::BTreeSet;

/// The discipline-folder modules every build compiles, 72 as of ADIM 11 (13 November 2026): the core reaches each
/// through a helper, a type or a check of its own (GIS's sources, fields, layers and file tables; arch's walls,
/// spaces and openings; map's survey, work and zoning). Their commands still answer only in their module's build.
const ALWAYS: &[(&str, &[&str])] = &[
    (
        "arch",
        &[
            "arch_bonds", "area_rules", "box_grid", "door_audit", "edge_shift_reference", "elevation", "elevations",
            "info_templates", "opening_types", "openings", "review_notes", "schedules", "spaces", "table_schedule",
            "walls",
        ],
    ),
    (
        "map",
        &[
            "grid_refs", "map_bonds", "map_survey", "map_survey_network", "map_survey_places", "map_survey_sessions",
            "map_work", "map_work_profile", "map_work_restore", "pointcloud_areas", "pointclouds", "project_grids",
            "zoning_check", "zoning_draft", "zoning_symbolize",
        ],
    ),
    (
        "gis",
        &[
            "csv_table", "datasets_edit", "event_follow", "fgb_table", "fields_edit", "fields_usage", "fields_write",
            "forms", "geopackage", "gis_derive", "gis_set", "gml_read", "gml_table", "gml_write", "gpkg_profile",
            "gpkg_tiles", "layers_indexes", "layers_labels", "layers_renderer", "layers_translate", "links", "lrs_events",
            "multidim_source", "network_ellipses", "parquet_table", "processing_host", "qml", "raster_files",
            "rasters_georef", "relations_edit", "remote_host", "shapefile", "sld", "source_bytes", "source_places",
            "sources", "subnetwork_follow", "subnetworks", "subnetworks_update", "time_export", "time_filter",
            "topology_follow",
        ],
    ),
];

/// Each `#[path = "…"] mod …;` of lib.rs: its folder, its name and the feature its `#[cfg(feature = "…")]` names.
fn modules() -> Vec<(String, String, Option<String>)> {
    let lib = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs")).expect("lib.rs");
    let lines: Vec<&str> = lib.lines().collect();
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let Some(path) = line.strip_prefix("#[path = \"").and_then(|p| p.strip_suffix("\"]")) else { continue };
        let Some((folder, _)) = path.split_once('/') else { continue };
        let mut feature = None;
        for above in lines[..i].iter().rev().take_while(|l| l.starts_with("#[") || l.starts_with("//")) {
            if let Some(f) = above.strip_prefix("#[cfg(feature = \"").and_then(|f| f.strip_suffix("\")]")) {
                feature = Some(f.to_owned());
            }
        }
        let Some(decl) = lines[i + 1..].iter().find(|l| !l.starts_with("#[")) else { continue };
        let name = decl.trim_start_matches("pub(crate) ").trim_start_matches("pub ");
        let Some(name) = name.strip_prefix("mod ").and_then(|n| n.strip_suffix(';')) else { continue };
        out.push((folder.to_owned(), name.to_owned(), feature));
    }
    out
}

#[test]
fn a_discipline_s_modules_are_behind_its_feature_but_those_the_core_reaches() {
    let mut wrong = Vec::new();
    let mut listed: BTreeSet<(String, String)> = ALWAYS
        .iter()
        .flat_map(|(folder, names)| names.iter().map(|n| ((*folder).to_owned(), (*n).to_owned())))
        .collect();
    for (folder, name, feature) in modules() {
        let own = format!("mod-{folder}");
        match (folder.as_str(), feature.as_deref()) {
            ("core", None) => {}
            ("core", Some("terrain")) => {}
            ("arch" | "map" | "gis" | "road", Some(f)) if f == own || (folder == "road" && f == "terrain") => {}
            ("arch" | "map" | "gis" | "road", None) if name.ends_with("_dispatch") => {}
            ("arch" | "map" | "gis" | "road", None) => {
                if !listed.remove(&(folder.clone(), name.clone())) {
                    wrong.push(format!("{folder}/{name}: in every build, not in ALWAYS (behind {own}, or listed)"));
                }
            }
            (_, f) => wrong.push(format!("{folder}/{name}: behind {f:?}")),
        }
    }
    for (folder, name) in listed {
        wrong.push(format!("{folder}/{name}: in ALWAYS but behind its feature or gone; take it off the list"));
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
