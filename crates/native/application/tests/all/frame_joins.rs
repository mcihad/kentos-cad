//! The frame's joins in the section model (docs/adr/8171 §2.1): no two of its columns, beams and slab structures hold the
//! same concrete. A slab's structure holds what lies in it, so a column or a beam stops under it where it covers them (a
//! part beyond its edge keeps its height); a column cuts a beam; where two beams cross, the one drawn first holds the
//! crossing. The volumes are worked by hand from the drawing's numbers.
use kentos_domain::Document;
use kentos_geometry_core::geom::arrangement::{Area, Ring, Rule};
use kentos_geometry_core::geom::overlay::overlay;
use kentos_geometry_core::geom::region::{area_source, intersect_area_sets, net_area};
use kentos_geometry_core::geom::section::Height;
use kentos_native_application::sections::{Model, model};
use serde_json::{Value, json};

fn building(entities: Value) -> (Document, Vec<String>) {
    let mut snapshot = json!({
        "format": "kentos.document", "version": 2, "name": "Çerçeve", "origin": {"x": 0.0, "y": 0.0},
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
            "plotScale": 50.0, "workspace": "cad",
            "arch": {"levels": [
                {"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
                {"id": "k1", "name": "1. kat", "elevation": 3.0, "height": 3.0}
            ]}},
        "layers": [{"id": "0", "name": "0", "type": "layer", "visible": true, "locked": false, "expanded": true,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}],
        "activeLayer": "0", "styles": {"items": [], "categories": []}, "uids": [], "entities": []
    });
    let mut uids = Vec::new();
    for (i, e) in entities.as_array().unwrap().iter().enumerate() {
        let mut e = e.clone();
        e["id"] = json!(i + 1);
        e["layerId"] = json!("0");
        e["attrs"] = json!({});
        snapshot["entities"].as_array_mut().unwrap().push(e);
        let uid = format!("0192f5a1-7242-7000-8000-0000000001{:02}", i + 10);
        snapshot["uids"].as_array_mut().unwrap().push(json!(uid));
        uids.push(uid);
    }
    (
        Document::from_snapshot_v2(serde_json::from_value(snapshot).unwrap()).unwrap(),
        uids,
    )
}

/// A solid's footprint as areas (its rings read even-odd).
fn footprint(m: &Model, i: usize) -> Vec<Area> {
    let sources: Vec<_> = m.solids[i]
        .rings
        .iter()
        .filter(|r| r.len() >= 3)
        .map(|r| {
            area_source(&[Area {
                outer: Ring {
                    pts: r.clone(),
                    bulges: None,
                },
                holes: Vec::new(),
            }])
        })
        .collect();
    overlay(&sources, Rule::Odd)
}

fn heights(m: &Model, i: usize) -> (f64, f64) {
    match m.solids[i].height {
        Height::Prism { bottom, top } => (bottom, top),
        _ => panic!("an upright solid"),
    }
}

/// The concrete an object's solids hold, cubic metres.
fn volume(m: &Model, owner: &str) -> f64 {
    (0..m.solids.len())
        .filter(|&i| m.owners[i].as_deref() == Some(owner))
        .map(|i| {
            let (b, t) = heights(m, i);
            footprint(m, i).iter().map(net_area).sum::<f64>() * (t - b)
        })
        .sum()
}

/// Every pair of the frame's solids (the columns', the beams' and the slabs' structures) that overlap upright share no
/// ground in plan: none holds concrete another holds.
fn shares_none(doc: &Document, m: &Model) {
    let frame: std::collections::HashSet<String> = doc
        .entities_with_uids()
        .filter(|(_, e)| {
            matches!(
                e,
                kentos_commands::Entity::Column(_) | kentos_commands::Entity::Beam(_)
            )
        })
        .map(|(u, _)| u.to_string())
        .collect();
    let solids: Vec<usize> = (0..m.solids.len())
        .filter(|&i| matches!(m.solids[i].height, Height::Prism { .. }) && m.parts[i].is_none())
        .filter(|&i| {
            m.structure.contains(&i) || m.owners[i].as_ref().is_some_and(|o| frame.contains(o))
        })
        .collect();
    assert!(solids.len() > 2, "a frame to join");
    for (n, &i) in solids.iter().enumerate() {
        for &j in &solids[n + 1..] {
            let ((bi, ti), (bj, tj)) = (heights(m, i), heights(m, j));
            if ti.min(tj) - bi.max(bj) <= 1e-6 {
                continue;
            }
            let shared: f64 = intersect_area_sets(&[footprint(m, i), footprint(m, j)])
                .iter()
                .map(net_area)
                .sum();
            assert!(
                shared < 1e-9,
                "{:?} and {:?} share {shared} m² between {}–{} and {}–{}",
                m.owners[i],
                m.owners[j],
                bi,
                ti,
                bj,
                tj
            );
        }
    }
}

#[test]
fn a_slab_holds_its_concrete_a_column_cuts_a_beam_and_the_first_beam_holds_a_crossing() {
    let (doc, uid) = building(json!([
        // 1. kat's 15 cm slab, its structure 2.85–3.00, over x −0.15…5.15.
        {"kind": "slab", "parts": [{"outer": {"pts": [{"x": -0.15, "y": -3.0}, {"x": 5.15, "y": -3.0},
            {"x": 5.15, "y": 3.0}, {"x": -0.15, "y": 3.0}]}}], "thickness": 0.15, "arch": {"level": "k1"}},
        {"kind": "column", "at": {"x": 0.0, "y": 0.0}, "width": 0.3, "depth": 0.3, "arch": {"level": "zemin"}},
        {"kind": "column", "at": {"x": 5.0, "y": 0.0}, "width": 0.3, "depth": 0.3, "arch": {"level": "zemin"}},
        // Column centre to column centre, then on past the slab's edge.
        {"kind": "beam", "pts": [{"x": 0.0, "y": 0.0}, {"x": 5.0, "y": 0.0}], "width": 0.25, "height": 0.5,
         "arch": {"level": "zemin"}},
        {"kind": "beam", "pts": [{"x": 5.0, "y": 0.0}, {"x": 8.0, "y": 0.0}], "width": 0.25, "height": 0.5,
         "arch": {"level": "zemin"}},
        // Drawn later, across the first beam.
        {"kind": "beam", "pts": [{"x": 2.5, "y": -1.0}, {"x": 2.5, "y": 1.0}], "width": 0.25, "height": 0.5,
         "arch": {"level": "zemin"}}
    ]));
    let m = model(&doc);
    shares_none(&doc, &m);
    let near = |owner: &str, want: f64| {
        let got = volume(&m, owner);
        assert!((got - want).abs() < 1e-9, "{owner}: {got} m³, want {want}");
    };
    // A column 0.30 × 0.30 from the floor to the slab's underside, 2.85.
    near(&uid[1], 0.09 * 2.85);
    near(&uid[2], 0.09 * 2.85);
    // Between the columns' faces (0.15…4.85) and under the slab: 4.70 × 0.25 × 0.35.
    near(&uid[3], 4.7 * 0.25 * 0.35);
    // From the column's face 5.15… and the slab's edge 5.15 on, nothing over it: 2.85 × 0.25 × 0.50.
    near(&uid[4], 2.85 * 0.25 * 0.5);
    // Two pieces of 0.875 under the slab, the crossing the first beam's.
    near(&uid[5], 2.0 * 0.875 * 0.25 * 0.35);
    // The slab whole: 5.30 × 6 × 0.15.
    near(&uid[0], 5.3 * 6.0 * 0.15);
}

#[test]
fn a_beam_over_a_slab_s_edge_keeps_its_depth_beyond_it() {
    let (doc, uid) = building(json!([
        // The slab ends at y 0: the beam along y 0 half under it.
        {"kind": "slab", "parts": [{"outer": {"pts": [{"x": -1.0, "y": -4.0}, {"x": 6.0, "y": -4.0},
            {"x": 6.0, "y": 0.0}, {"x": -1.0, "y": 0.0}]}}], "thickness": 0.15, "arch": {"level": "k1"}},
        {"kind": "beam", "pts": [{"x": 0.0, "y": 0.0}, {"x": 5.0, "y": 0.0}], "width": 0.25, "height": 0.5,
         "arch": {"level": "zemin"}}
    ]));
    let m = model(&doc);
    shares_none(&doc, &m);
    // Under the slab 5 × 0.125 × 0.35, beyond its edge 5 × 0.125 × 0.50.
    let got = volume(&m, &uid[1]);
    let want = 5.0 * 0.125 * 0.35 + 5.0 * 0.125 * 0.5;
    assert!((got - want).abs() < 1e-9, "{got} m³, want {want}");
}

#[test]
fn a_frame_with_no_slab_over_it_joins_its_beams_to_its_columns() {
    let (doc, uid) = building(json!([
        {"kind": "column", "at": {"x": 0.0, "y": 0.0}, "width": 0.3, "depth": 0.5, "arch": {"level": "k1"}},
        {"kind": "column", "at": {"x": 4.0, "y": 0.0}, "shape": "round", "width": 0.5, "arch": {"level": "k1"}},
        {"kind": "beam", "pts": [{"x": 0.0, "y": 0.0}, {"x": 4.0, "y": 0.0}], "width": 0.25, "height": 0.5,
         "arch": {"level": "k1"}}
    ]));
    let m = model(&doc);
    shares_none(&doc, &m);
    // The column's full height, the storey's: 0.15 m² × 3.
    let column = volume(&m, &uid[0]);
    assert!((column - 0.15 * 3.0).abs() < 1e-9, "{column}");
    // The beam starts at the rectangle's face (x 0.15) and ends on the round one's polygon, between the circle's
    // nearest point (x 3.75) and where it meets the beam's sides (4 − √(0.25² − 0.125²) = 3.7835).
    let beam = volume(&m, &uid[2]);
    assert!(beam > 3.6 * 0.125 && beam < 3.6335 * 0.125, "{beam}");
}

#[test]
fn the_section_building_s_frame_shares_no_concrete() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/arch/v1/section-building.kcad");
    let snapshot = kentos_kcad::decode(&std::fs::read(path).unwrap()).unwrap();
    let doc = Document::from_snapshot_v2(snapshot).unwrap();
    shares_none(&doc, &model(&doc));
}

/// Footings (docs/adr/8172): a pad footing under a column and a strip footing under a wall stand at their base under
/// the storey's floor, their height their thickness; nothing joins them to the column standing on the floor over them.
/// The IFC writes each an `IfcFooting` of its predefined type (EK-6 Tablo 6.34), named MM-TML-…, with its quantities.
#[test]
fn footings_stand_under_the_floor_and_are_the_ifc_s_footings() {
    let (doc, uid) = building(json!([
        {"kind": "column", "at": {"x": 0.0, "y": 0.0}, "width": 0.3, "depth": 0.3, "arch": {"level": "zemin"}},
        {"kind": "beam", "pts": [{"x": -0.75, "y": 0.0}, {"x": 0.75, "y": 0.0}], "width": 1.5, "height": 0.5,
         "role": "padFooting", "baseOffset": -0.8, "tag": "T101", "arch": {"level": "zemin"}},
        {"kind": "beam", "pts": [{"x": 0.75, "y": 0.0}, {"x": 6.0, "y": 0.0}], "width": 0.6, "height": 0.5,
         "role": "stripFooting", "baseOffset": -0.8, "arch": {"level": "zemin"}}
    ]));
    let m = model(&doc);
    shares_none(&doc, &m);
    let solid = |owner: &str| (0..m.solids.len()).find(|&i| m.owners[i].as_deref() == Some(owner)).expect("its solid");
    // Its base 0.80 under ±0.00, its top 0.30 under it.
    let (b, t) = heights(&m, solid(&uid[1]));
    assert!((b + 0.8).abs() < 1e-12 && (t + 0.3).abs() < 1e-12, "{b}–{t}");
    // Betonarme as the column and the slab: one section with them where a drawn section cuts (ISO 128-3, ADR 8041 §2).
    assert_eq!(m.looks[solid(&uid[1])].layer, "MM-TASI-BETONARME");
    // The pad 1.5 × 1.5 × 0.5 whole; the strip 5.25 × 0.6 × 0.5 less the pad's crossing (it was drawn after).
    assert!((volume(&m, &uid[1]) - 1.5 * 1.5 * 0.5).abs() < 1e-9);
    assert!((volume(&m, &uid[2]) - 5.25 * 0.6 * 0.5).abs() < 1e-9, "{}", volume(&m, &uid[2]));
    // The column over the pad stands on its top (ADR 8172 §3.2), whole: from −0.30 to its storey's top, 0.09 × 3.30.
    let (b, t) = heights(&m, solid(&uid[0]));
    assert!((b + 0.3).abs() < 1e-12 && (t - 3.0).abs() < 1e-12, "{b}–{t}");
    assert!((volume(&m, &uid[0]) - 0.09 * 3.3).abs() < 1e-9, "{}", volume(&m, &uid[0]));
    let (building, written) = kentos_native_application::arch_ifc::building(&doc, "", "KentOS Mimari");
    assert_eq!(written.footings, 2);
    let text = kentos_ifc::building::write(&building);
    let footings: Vec<&str> = text.lines().filter(|l| l.contains("=IFCFOOTING(")).collect();
    assert_eq!(footings.len(), 2, "{footings:?}");
    assert!(footings.iter().any(|l| l.ends_with(".PAD_FOOTING.);") && l.contains("'MM-TML-")), "{footings:?}");
    assert!(footings.iter().any(|l| l.ends_with(".STRIP_FOOTING.);")), "{footings:?}");
    assert_eq!(text.matches("Qto_FootingBaseQuantities").count(), 2);
}

#[test]
fn a_column_goes_down_to_its_own_storey_s_footing_only() {
    let (doc, uid) = building(json!([
        // Over the pad on its storey: down to the pad's top.
        {"kind": "column", "at": {"x": 0.0, "y": 0.0}, "width": 0.3, "depth": 0.3, "arch": {"level": "zemin"}},
        // Beside it, off the pad: on the floor.
        {"kind": "column", "at": {"x": 4.0, "y": 0.0}, "width": 0.3, "depth": 0.3, "arch": {"level": "zemin"}},
        // Over it a storey up: on its own floor.
        {"kind": "column", "at": {"x": 0.0, "y": 0.0}, "width": 0.3, "depth": 0.3, "arch": {"level": "k1"}},
        {"kind": "beam", "pts": [{"x": -0.75, "y": 0.0}, {"x": 0.75, "y": 0.0}], "width": 1.5, "height": 0.5,
         "role": "padFooting", "baseOffset": -0.8, "arch": {"level": "zemin"}}
    ]));
    let m = model(&doc);
    shares_none(&doc, &m);
    let solid = |owner: &str| (0..m.solids.len()).find(|&i| m.owners[i].as_deref() == Some(owner)).expect("its solid");
    let bottom = |owner: &str| heights(&m, solid(owner)).0;
    assert!((bottom(&uid[0]) + 0.3).abs() < 1e-12, "{}", bottom(&uid[0]));
    assert!(bottom(&uid[1]).abs() < 1e-12, "{}", bottom(&uid[1]));
    assert!((bottom(&uid[2]) - 3.0).abs() < 1e-12, "{}", bottom(&uid[2]));
}

#[test]
fn a_raft_is_the_ifc_s_base_slab_and_its_columns_stand_on_it() {
    let (doc, uid) = building(json!([
        {"kind": "column", "at": {"x": 0.0, "y": 0.0}, "width": 0.3, "depth": 0.3, "arch": {"level": "zemin"}},
        {"kind": "slab", "parts": [{"outer": {"pts": [{"x": -1.0, "y": -1.0}, {"x": 5.0, "y": -1.0},
            {"x": 5.0, "y": 4.0}, {"x": -1.0, "y": 4.0}]}}], "role": "raft", "thickness": 0.6, "offset": -0.3,
         "arch": {"level": "zemin"}},
        {"kind": "column", "at": {"x": 4.0, "y": 3.0}, "width": 0.3, "depth": 0.3, "arch": {"level": "zemin"}}
    ]));
    let m = model(&doc);
    shares_none(&doc, &m);
    let solid = |owner: &str| (0..m.solids.len()).find(|&i| m.owners[i].as_deref() == Some(owner)).expect("its solid");
    // The raft 0.90 to 0.30 under ±0.00, the column from its top to its storey's.
    let (b, t) = heights(&m, solid(&uid[1]));
    assert!((b + 0.9).abs() < 1e-12 && (t + 0.3).abs() < 1e-12, "{b}–{t}");
    let (b, t) = heights(&m, solid(&uid[0]));
    assert!((b + 0.3).abs() < 1e-12 && (t - 3.0).abs() < 1e-12, "{b}–{t}");
    let (building, written) = kentos_native_application::arch_ifc::building(&doc, "", "KentOS Mimari");
    assert_eq!((written.slabs, written.footings), (1, 0));
    let text = kentos_ifc::building::write(&building);
    let slabs: Vec<&str> = text.lines().filter(|l| l.contains("=IFCSLAB(")).collect();
    assert_eq!(slabs.len(), 1, "{slabs:?}");
    assert!(slabs[0].ends_with(".BASESLAB.);") && slabs[0].contains("'MM-DSM-Radye"), "{}", slabs[0]);
}

#[test]
fn a_profile_covers_a_wall_unjoined_and_is_the_ifc_s_covering() {
    let (doc, uid) = building(json!([
        // A parapet wall 1.00 m high and its coping: 5 cm past each face, 5 cm thick, on the wall's top.
        {"kind": "wall", "pts": [{"x": 0.0, "y": 0.0}, {"x": 6.0, "y": 0.0}], "width": 0.25, "height": 1.0,
         "arch": {"level": "zemin"}},
        {"kind": "beam", "pts": [{"x": 0.0, "y": 0.0}, {"x": 6.0, "y": 0.0}], "width": 0.35, "height": 0.05,
         "baseOffset": 1.0, "covering": {"predefined": "COPING"}, "arch": {"level": "zemin"}},
        // A facade moulding along the wall's face at 0.60 m, 5 cm out, through a column it meets.
        {"kind": "column", "at": {"x": 3.0, "y": -0.2}, "width": 0.3, "depth": 0.3, "arch": {"level": "zemin"}},
        {"kind": "beam", "pts": [{"x": 0.0, "y": -0.125}, {"x": 6.0, "y": -0.125}], "width": 0.05, "height": 0.2,
         "justify": "right", "baseOffset": 0.6, "covering": {"predefined": "MOLDING", "name": "Silme"},
         "material": "Prekast beton", "arch": {"level": "zemin"}}
    ]));
    let m = model(&doc);
    let solid = |owner: &str| (0..m.solids.len()).find(|&i| m.owners[i].as_deref() == Some(owner)).expect("its solid");
    // On EK-2's Harpuşta and Cephe, by their material; the coping on the wall's top.
    assert!(m.looks[solid(&uid[1])].layer.starts_with("MM-HARP-"), "{}", m.looks[solid(&uid[1])].layer);
    assert!(m.looks[solid(&uid[3])].layer.starts_with("MM-CEPH-"), "{}", m.looks[solid(&uid[3])].layer);
    let (b, t) = heights(&m, solid(&uid[1]));
    assert!((b - 1.0).abs() < 1e-12 && (t - 1.05).abs() < 1e-12, "{b}–{t}");
    // No structure: the column does not cut the moulding, the coping does not stop the wall.
    assert!((volume(&m, &uid[3]) - 6.0 * 0.05 * 0.2).abs() < 1e-9, "{}", volume(&m, &uid[3]));
    let Some(kentos_commands::Entity::Wall(w)) = doc.entities().find(|e| matches!(e, kentos_commands::Entity::Wall(_)))
    else {
        panic!("the wall")
    };
    assert_eq!(doc.wall_height(&w.wall), Some(1.0));
    let (building, written) = kentos_native_application::arch_ifc::building(&doc, "", "KentOS Mimari");
    assert_eq!((written.coverings, written.beams), (2, 0));
    let text = kentos_ifc::building::write(&building);
    let coverings: Vec<&str> = text.lines().filter(|l| l.contains("=IFCCOVERING(")).collect();
    assert!(coverings.iter().any(|l| l.ends_with(".COPING.);") && l.contains("'MM-KPL-Harpu")), "{coverings:?}");
    assert!(coverings.iter().any(|l| l.ends_with(".MOLDING.);") && l.contains("'MM-KPL-Silme")), "{coverings:?}");
}
