//! Sizes and times of a large drawing, for docs/adr/0025 (TODOS.md FILE-24 is
//! the full measurement): 100 000 objects, parcels of 20 vertices with
//! attributes, written as KCAD v2 and as v1 JSON, read back. Not a test of
//! correctness and not run by default:
//!
//!     cargo test --release -p kentos-kcad --test measure -- --ignored --nocapture

use std::collections::BTreeMap;
use std::time::Instant;

use kentos_kcad::contracts::{
    AngleUnit, AreaUnit, DocumentSnapshotV1, DocumentSnapshotV2, Entity, EntityBase, EntityId,
    LayerNode, LayerNodeType, LayerStyle, LineType, PathEntity, ProjectSettings, ProjectStyles,
    Vec2,
};

fn drawing(n: usize) -> DocumentSnapshotV2 {
    let mut entities = Vec::with_capacity(n);
    let mut uids = Vec::with_capacity(n);
    for i in 0..n {
        let (x0, y0) = (
            486_000.0 + (i % 300) as f64 * 31.7,
            4_420_000.0 + (i / 300) as f64 * 27.3,
        );
        let pts = (0..20)
            .map(|k| {
                let a = k as f64 / 20.0 * std::f64::consts::TAU;
                Vec2 {
                    x: x0 + 12.5 + 11.0 * a.cos(),
                    y: y0 + 12.5 + 9.0 * a.sin(),
                }
            })
            .collect();
        let attrs = BTreeMap::from([
            ("Ada".to_owned(), format!("{}", 100 + i / 50)),
            ("Parsel".to_owned(), format!("{}", i % 50 + 1)),
            ("Nitelik".to_owned(), "Arsa".to_owned()),
        ]);
        entities.push(Entity::Polygon(PathEntity {
            base: EntityBase {
                id: i as u32 + 1,
                layer_id: "parsel".into(),
                color: None,
                attrs,
                label: Some(format!("{}/{}", 100 + i / 50, i % 50 + 1)),
                symbol: None,
            },
            pts,
            bulges: None,
            holes: None,
        }));
        // A v7-shaped id: a fixed time, the version, the variant, then the object's number.
        let mut id = [
            0x01, 0x92, 0xf5, 0xa0, 0x7c, 0x3e, 0x70, 0x00, 0x80, 0, 0, 0, 0, 0, 0, 0,
        ];
        id[10..].copy_from_slice(&(i as u64 + 1).to_be_bytes()[2..]);
        uids.push(EntityId(id));
    }
    DocumentSnapshotV2 {
        format: "kentos.document".into(),
        version: 2,
        name: "Ölçüm".into(),
        settings: ProjectSettings {
            srid: 5256,
            length_decimals: 3,
            area_decimals: 2,
            area_unit: AreaUnit::M2,
            angle_unit: AngleUnit::Grad,
            plot_scale: 1000.0,
            workspace: None,
            drawing_font: None,
        },
        origin: Vec2 {
            x: 486_000.0,
            y: 4_420_000.0,
        },
        home_view: None,
        layers: vec![LayerNode {
            id: "parsel".into(),
            name: "Parsel".into(),
            kind: LayerNodeType::Layer,
            visible: true,
            locked: false,
            expanded: true,
            style: LayerStyle {
                color: "#E06C75".into(),
                line_type: LineType::Continuous,
                line_weight: 0.35,
                fill: None,
                point: None,
                label: None,
                pick_interior: None,
                renderer: None,
            },
            children: Vec::new(),
        }],
        active_layer: "parsel".into(),
        entities,
        uids,
        styles: ProjectStyles::default(),
        project_id: None,
        migrated_from: None,
    }
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

#[test]
#[ignore = "a measurement, run by hand in release mode"]
fn a_large_drawing() {
    let doc = drawing(100_000);
    let t = Instant::now();
    let bytes = kentos_kcad::encode(&doc).expect("writes");
    let encode = ms(t);
    let t = Instant::now();
    let verified = kentos_kcad::encode_verified(&doc).expect("verifies");
    let encode_verified = ms(t);
    let t = Instant::now();
    let back = kentos_kcad::decode(&bytes).expect("reads");
    let decode = ms(t);
    assert_eq!(verified, bytes);
    assert_eq!(back.entities.len(), 100_000);
    let v1 = DocumentSnapshotV1 {
        format: "kentos.document".into(),
        version: 1,
        name: doc.name.clone(),
        settings: doc.settings.clone(),
        origin: doc.origin,
        home_view: None,
        layers: doc.layers.clone(),
        active_layer: doc.active_layer.clone(),
        entities: doc.entities.clone(),
        styles: doc.styles.clone(),
    };
    let t = Instant::now();
    let json = serde_json::to_vec(&v1).expect("v1 JSON");
    let json_write = ms(t);
    let t = Instant::now();
    let text = std::str::from_utf8(&json).expect("UTF-8");
    let _ = DocumentSnapshotV1::from_json(text).expect("v1 reads");
    let json_read = ms(t);
    println!(
        "100 000 nesne (20 köşeli alan, 3 öznitelik, etiket): KCAD v2 {} bayt, v1 JSON {} bayt",
        bytes.len(),
        json.len()
    );
    println!(
        "v2 yazma {encode:.0} ms, doğrulamalı yazma {encode_verified:.0} ms, okuma {decode:.0} ms; v1 JSON yazma {json_write:.0} ms, okuma {json_read:.0} ms"
    );
}
