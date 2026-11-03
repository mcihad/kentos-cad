//! World bounds are independent of drawing units and insertion grips.
use kentos_commands::{Bounds, DocumentSnapshotV1};
use kentos_domain::{Document, Slot};
use kentos_headless::{Method, Session};
use serde_json::{Value, json};

fn model() -> Value {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/commands/v1/cad.blocks.find.json"
    ))
    .unwrap();
    let mut raw = fixture["setup"].clone();
    let inner = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4001";
    let outer = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4002";
    let mut hidden = raw["layers"][0].clone();
    hidden["id"] = json!("hidden");
    hidden["name"] = json!("Hidden");
    hidden["visible"] = json!(false);
    raw["layers"].as_array_mut().unwrap().push(hidden);
    raw["blocks"] = json!([
      {"id":inner,"name":"Inner","base":{"x":0.,"y":0.},"units":"mm","entities":[
        {"kind":"line","id":1,"layerId":"","attrs":{},"a":{"x":1.,"y":2.},"b":{"x":3.,"y":4.}},
        {"kind":"line","id":2,"layerId":"hidden","attrs":{},"a":{"x":-1000.,"y":-1000.},"b":{"x":1000.,"y":1000.}}]},
      {"id":outer,"name":"Outer","base":{"x":0.,"y":0.},"units":"m","entities":[
        {"kind":"insert","id":1,"layerId":"","attrs":{},"block":inner,"p":{"x":10.,"y":20.},"scale":2.,"rotation":0.}]}
    ]);
    raw["entities"] = json!([{ "kind":"insert","id":1,"layerId":"a","attrs":{},"block":outer,
        "p":{"x":100.,"y":200.},"scale":2.,"yScale":3.,"rotation":std::f64::consts::FRAC_PI_2,"mirror":true }]);
    raw
}

fn close(b: &Bounds, expected: [f64; 4]) {
    for (a, e) in [b.min_x, b.min_y, b.max_x, b.max_y]
        .into_iter()
        .zip(expected)
    {
        assert!((a - e).abs() < 1e-9, "{b:?} != {expected:?}");
    }
}

#[test]
fn nested_mirrored_nonuniform_blocks_measure_visible_pieces_and_bbox_search_reaches_them() {
    for unit in ["mm", "m"] {
        let mut raw = model();
        raw["settings"]["drawingUnit"] = json!(unit);
        let session = Session::from_bytes(&serde_json::to_vec(&raw).unwrap()).unwrap();
        let uid = session
            .entities(None, None, None, None, None)
            .unwrap()
            .items[0]
            .uid
            .clone();
        // Inner canonical metres -> nested (12,24)-(16,28), mirrored Y, X*2/Y*3,
        // quarter turn and translation -> (172,224)-(184,232). No second mm conversion.
        let measured = session.measure(&uid).unwrap();
        close(&measured.bounds, [172., 224., 184., 232.]);
        assert_eq!(measured.area, None);
        assert_eq!(measured.length, None);
        let batch = session
            .measure_many(&[uid.clone(), uid.clone()], Method::Plane)
            .unwrap();
        assert_eq!(batch.items[0].bounds, measured.bounds);
        let near = Bounds {
            min_x: 175.,
            min_y: 225.,
            max_x: 176.,
            max_y: 226.,
        };
        assert_eq!(
            session
                .entities(None, None, Some(near), None, None)
                .unwrap()
                .items
                .len(),
            1
        );
        let at_grip = Bounds {
            min_x: 99.,
            min_y: 199.,
            max_x: 101.,
            max_y: 201.,
        };
        assert!(
            session
                .entities(None, None, Some(at_grip), None, None)
                .unwrap()
                .items
                .is_empty()
        );
        // The same placed-box contract is used by the desktop inspector and matches visible renderer pieces.
        let doc =
            Document::from_snapshot(serde_json::from_value::<DocumentSnapshotV1>(raw).unwrap())
                .unwrap();
        let store = kentos_native_application::geometry_store::store_of(&doc);
        let boxes: Vec<_> = store
            .get(1.)
            .unwrap()
            .shapes()
            .map(kentos_geometry_core::entity::entity_bounds)
            .collect();
        let b = kentos_geometry_core::ops::arrange::union(&boxes).unwrap();
        close(
            &Bounds {
                min_x: b.min_x,
                min_y: b.min_y,
                max_x: b.max_x,
                max_y: b.max_y,
            },
            [172., 224., 184., 232.],
        );
    }
}

#[test]
fn clipped_block_bounds_exclude_content_beyond_its_boundary() {
    let mut raw = model();
    raw["entities"][0]["rotation"] = json!(0.);
    raw["entities"][0]["mirror"] = json!(false);
    raw["entities"][0]["scale"] = json!(1.);
    raw["entities"][0]["yScale"] = json!(1.);
    raw["entities"][0]["clipping"] = json!({"boundary":[{"x":13.,"y":20.},{"x":15.,"y":20.},{"x":15.,"y":30.},{"x":13.,"y":30.}]});
    let session = Session::from_bytes(&serde_json::to_vec(&raw).unwrap()).unwrap();
    let uid = session.document().uid(Slot(1)).unwrap().to_string();
    close(
        &session.measure(&uid).unwrap().bounds,
        [113., 225., 115., 227.],
    );
}

#[test]
fn active_dynamic_variant_changes_measured_bounds_and_one_undo_restores_them() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/commands/v1/cad.blocks.proxy.json"
    ))
    .unwrap();
    let mut session = Session::from_bytes(&serde_json::to_vec(&fixture["setup"]).unwrap()).unwrap();
    let uid = session.document().uid(Slot(4)).unwrap().to_string();
    // Shelf: default one 0.4 m segment and circle at (0,.5), radius .05.
    close(
        &session.measure(&uid).unwrap().bounds,
        [-0.05, 5., 0.4, 5.55],
    );
    let changed = session
        .run(
            "cad.blocks.parameters",
            None,
            kentos_headless::Op::Execute,
            json!({"operation":"set","uids":[uid],"values":{"Uzunluk":1.5}}),
        )
        .unwrap();
    assert_eq!(changed["status"], "completed", "{changed}");
    // One extra copy over the definition default, .5 m apart.
    close(
        &session.measure(&uid).unwrap().bounds,
        [-0.05, 5., 0.9, 5.55],
    );
    assert!(session.undo().is_some());
    close(
        &session.measure(&uid).unwrap().bounds,
        [-0.05, 5., 0.4, 5.55],
    );
}
