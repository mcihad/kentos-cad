use crate::{Spatial, Vec2};
use kentos_domain::{Document, Slot};
use kentos_commands::{DocumentSnapshotV1, CallOp};
use kentos_geometry_core::store::snap::{SnapExtras, SnapKind};
use serde_json::{Value, json};
#[test]
fn twenty_viewports_reuse_original_objects_for_select_snap_measure_and_incremental_sync() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/sheet/v1/viewport-identities.json"
    ))
    .unwrap();
    for c in data["cases"].as_array().unwrap() {
        let snapshot: DocumentSnapshotV1 = serde_json::from_value(c["model"].clone()).unwrap();
        let mut doc = Document::from_snapshot(snapshot).unwrap();
        let r = kentos_native_application::registry::run(
            &mut kentos_native_application::ExecutionContext::new(&mut doc),
            "cad.sheets.edit",
            CallOp::Execute,
            json!({"ops":[{"op":"addSheet","sheet":c["sheet"]},c["operation"]]}),
        )
        .unwrap();
        assert_eq!(r["status"], "completed");
        let scope = kentos_native_application::viewport_scope::resolve(
            &doc,
            &kentos_commands::SheetModelScope {
                sheet: "page".into(),
                item: "viewport".into(),
            },
        )
        .unwrap();
        let origin = Vec2::new(
            c["origin"]["x"].as_f64().unwrap(),
            c["origin"]["y"].as_f64().unwrap(),
        );
        let before = doc.to_snapshot_v2();
        let mut spatial = Spatial::of(&doc);
        spatial.set_sheet_view(&doc, Some(scope.clone()));
        let reloads = spatial.reloads();
        let inside = spatial.in_rect(
            Vec2::new(origin.x - 1000., origin.y - 1000.),
            Vec2::new(origin.x + 1000., origin.y + 1000.),
            true,
        );
        let expected = c["expected"]["selectedSlots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| Slot(n.as_u64().unwrap() as u32))
            .collect::<Vec<_>>();
        assert_eq!(inside, expected, "{}", c["name"]);
        let at = &c["snap"]["at"];
        let at = Vec2::new(at["x"].as_f64().unwrap(), at["y"].as_f64().unwrap());
        let tol = c["snap"]["aperture"].as_f64().unwrap();
        let hits =
            spatial.snap_candidates(at, tol, SnapKind::Node.bit(), None, &SnapExtras::default());
        assert_eq!(hits[0].id, 5., "{}", c["name"]);
        assert_eq!(
            spatial.snap_ex(at, tol, SnapKind::Node.bit(), None, &SnapExtras::default()),
            Some(hits[0])
        );
        for _ in 0..25 {
            spatial.set_sheet_view(&doc, Some(scope.clone()));
            spatial.sync(&doc);
        }
        assert_eq!(
            spatial.reloads(),
            reloads,
            "pointer motion cannot rebuild model geometry"
        );
        assert_eq!(doc.to_snapshot_v2(), before);
        assert_eq!(doc.entities().count(), 5);
        assert_eq!(
            crate::spatial::measures(doc.get(Slot(1)).unwrap())
                .1
                .unwrap(),
            5.
        );
        let made = kentos_native_application::registry::run(
            &mut kentos_native_application::ExecutionContext::new(&mut doc),
            "cad.point.create",
            CallOp::Execute,
            json!({"layerId":"geom","p":{"x":origin.x,"y":origin.y-0.03*scope.denominator}}),
        )
        .unwrap();
        assert_eq!(made["status"], "completed");
        let uid = made["output"]["uid"].as_str().unwrap();
        let slot = doc.slot_of(uid.parse().unwrap()).unwrap();
        spatial.sync(&doc);
        assert_eq!(spatial.reloads(), reloads);
        assert_eq!(
            spatial.pick(
                Vec2::new(origin.x, origin.y - 0.03 * scope.denominator),
                1e-7
            ),
            Some(slot)
        );
        doc.undo();
        spatial.sync(&doc);
        assert!(spatial.store().get(f64::from(slot.0)).is_none());
        assert_eq!(spatial.reloads(), reloads);
        spatial.set_sheet_view(&doc, None);
        assert_eq!(
            spatial
                .in_rect(
                    Vec2::new(origin.x - 1000., origin.y - 1000.),
                    Vec2::new(origin.x + 1000., origin.y + 1000.),
                    true
                )
                .len(),
            5
        );
    }
}
