//! `file.export.gltf`'s file (plan/road.md Y12.3, docs/adr/0910 §3): its shared cases' drawing written as GLB.
//! `KENTOS_WRITE_GLTF=1` writes it as fixtures/road/v1/gltf/out/yol.glb, which `scripts/fixtures/road_scene_cases.py
//! --written` checks against the references and with Khronos' glTF-Validator; without it the file goes to a folder of
//! the test's. Read back here, every vertex is its float32 plus the origin within the reported gap.

use std::path::PathBuf;

use kentos_commands::{CommandResult, ExportGltf};
use kentos_native_application::{ExecutionContext, road_scene};

#[test]
fn a_road_project_goes_out_as_gltf() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/file.export.gltf.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("setup");
    let mut doc = kentos_domain::Document::from_snapshot_v2(snapshot).expect("opens");
    let folder = if std::env::var_os("KENTOS_WRITE_GLTF").is_some() {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/road/v1/gltf/out")
    } else {
        let d = std::env::temp_dir().join(format!("kentos-gltf-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    };
    let path = folder.join("yol.glb");
    let input = ExportGltf { path: path.display().to_string(), surfaces: None, corridors: None, overwrite: true };
    let out = match road_scene::execute(&mut ExecutionContext::new(&mut doc), input) {
        CommandResult::Completed { output, .. } => output,
        other => panic!("writes: {other:?}"),
    };
    assert_eq!(out.meshes.len(), 3);
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(bytes.len() as u64, out.bytes);
    let (origin, meshes) = kentos_formats::gltf::read(&bytes).expect("reads back");
    assert_eq!(origin, out.origin);
    let (parts, _) = road_scene::scene(&doc);
    for (m, p) in meshes.iter().zip(&parts) {
        assert_eq!(m.triangles, p.triangles);
        for (f, v) in m.positions.iter().zip(&p.vertices) {
            for a in 0..3 {
                assert!((f64::from(f[a]) + origin[a] - v[a]).abs() <= out.error + 1e-9, "{} {v:?}", m.name);
            }
        }
    }
}

/// A scene of projected coordinates (TM, millions of metres) keeps its millimetres: the origin carries them.
#[test]
fn far_coordinates_keep_their_millimetres() {
    let parts = vec![road_scene::ScenePart {
        kind: kentos_commands::SceneMeshKind::Terrain,
        id: "t".into(),
        name: "Zemin".into(),
        vertices: vec![[512_345.678, 4_401_234.567, 1_234.5], [522_345.679, 4_401_234.568, 1_250.25], [512_345.678, 4_411_234.566, 1_240.0]],
        triangles: vec![[0, 1, 2]],
    }];
    let o = road_scene::origin_of(&parts);
    assert_eq!(o, [517_346.0, 4_406_235.0, 1_242.0]);
    let worst = parts[0]
        .vertices
        .iter()
        .flat_map(|v| (0..3).map(move |a| ((v[a] - o[a]) as f32 as f64 - (v[a] - o[a])).abs()))
        .fold(0.0, f64::max);
    assert!(worst < 2.5e-4, "{worst}");
}
