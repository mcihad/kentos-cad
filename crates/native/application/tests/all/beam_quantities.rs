//! Metraj › Kirişler (docs/adr/3041 §3): a beam's concrete is what hangs under the slab it carries, so the slab's volume
//! and the beam's are not the same concrete twice; a lintel's and a bond beam's, in their walls, is the whole.
use super::wall_contours::completed;
use kentos_domain::Document;
use serde_json::{Value, json};

fn building(entities: Value) -> Document {
    let mut snapshot = json!({
        "format": "kentos.document", "version": 2, "name": "Kirişler", "origin": {"x": 0.0, "y": 0.0},
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
    for (i, e) in entities.as_array().unwrap().iter().enumerate() {
        let mut e = e.clone();
        e["id"] = json!(i + 1);
        e["layerId"] = json!("0");
        e["attrs"] = json!({});
        snapshot["entities"].as_array_mut().unwrap().push(e);
        snapshot["uids"].as_array_mut().unwrap().push(json!(format!(
            "0192f5a1-7242-7000-8000-0000000000{:02}",
            i + 10
        )));
    }
    Document::from_snapshot_v2(serde_json::from_value(snapshot).unwrap()).unwrap()
}

#[test]
fn a_beam_counts_its_concrete_under_its_slab_and_a_lintel_the_whole() {
    let doc = building(json!([
        // 1. kat's 15 cm slab over the ground storey.
        {"kind": "slab", "parts": [{"outer": {"pts": [{"x": -1.0, "y": -1.0}, {"x": 10.0, "y": -1.0},
            {"x": 10.0, "y": 6.0}, {"x": -1.0, "y": 6.0}]}}], "thickness": 0.15, "arch": {"level": "k1"}},
        {"kind": "beam", "pts": [{"x": 0.0, "y": 0.0}, {"x": 5.0, "y": 0.0}], "width": 0.25, "height": 0.5,
         "tag": "K101", "arch": {"level": "zemin"}},
        {"kind": "beam", "pts": [{"x": 6.0, "y": 0.0}, {"x": 7.4, "y": 0.0}], "width": 0.2, "height": 0.2,
         "role": "lintel", "baseOffset": 2.1, "tag": "L101", "arch": {"level": "zemin"}}
    ]));
    let q = completed(kentos_native_application::spaces_quantities::plan_in(
        &doc,
        &Default::default(),
    ));
    assert_eq!(q.beams.len(), 2);
    let (beam, lintel) = (&q.beams[0], &q.beams[1]);
    // 0.25 × (0.50 − 0.15) × 5 = 0.4375 m³: the 15 cm in the slab is the slab's.
    assert_eq!(beam.slab, Some(0.15));
    assert!((beam.volume - 0.4375).abs() < 1e-12, "{}", beam.volume);
    assert!((beam.length - 5.0).abs() < 1e-12);
    assert!(beam.rule.contains("döşemenin altında"), "{}", beam.rule);
    // 0.20 × 0.20 × 1.40 = 0.056 m³, the whole.
    assert_eq!(lintel.role, kentos_commands::BeamRole::Lintel);
    assert_eq!(lintel.slab, None);
    assert!((lintel.volume - 0.056).abs() < 1e-12, "{}", lintel.volume);
    assert!(
        (q.totals.beam_volume - 0.4935).abs() < 1e-12,
        "{}",
        q.totals.beam_volume
    );
    // The level filter: none on 1. kat.
    let only = completed(kentos_native_application::spaces_quantities::plan_in(
        &doc,
        &serde_json::from_value(json!({"levels": ["k1"]})).unwrap(),
    ));
    assert!(only.beams.is_empty());
}
