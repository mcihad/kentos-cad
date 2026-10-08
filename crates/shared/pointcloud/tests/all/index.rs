//! The COPC index (docs/adr/0207 §4): node by node what the reference builds
//! (scripts/fixtures/pointcloud_index_cases.py), and a file every reader reads.

use kentos_pointcloud::copc::Key;
use kentos_pointcloud::index::{Plan, build_in_memory};
use kentos_pointcloud::record::{Layout, standard_len, wide_format, widen};
use kentos_pointcloud::source::Kind;

use crate::host::{fixtures, fnv, open, records};

fn index_cases() -> serde_json::Value {
    let text = std::fs::read_to_string(fixtures().join("index.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// The index of a fixture with these limits: the COPC's bytes.
fn build(
    name: &str,
    leaf_most: usize,
    bin_depth: i32,
) -> (Vec<u8>, Vec<kentos_pointcloud::copc::Node>) {
    let bytes = std::fs::read(fixtures().join("files").join(name)).unwrap();
    let cloud = open(&bytes).unwrap();
    let raw = records(&cloud, &bytes);
    let from = cloud.layout;
    let fmt = wide_format(from.format);
    let to = Layout::new(fmt, standard_len(fmt) + from.extra());
    let mut wide = vec![0u8; to.len * cloud.head.count as usize];
    for (i, r) in raw.chunks_exact(from.len).enumerate() {
        widen(&from, r, &to, &mut wide[i * to.len..(i + 1) * to.len]);
    }
    let mut plan = Plan::new(
        cloud.bounds(),
        cloud.head.count,
        cloud.head.scale,
        cloud.head.offset,
        fmt,
        from.extra() as u16,
    );
    plan.leaf_most = leaf_most;
    plan.bin_depth = bin_depth;
    build_in_memory(plan, &wide).unwrap()
}

#[test]
fn the_index_is_the_references_node_by_node() {
    let cases = index_cases();
    for c in cases["cases"].as_array().unwrap() {
        let name = c["file"].as_str().unwrap();
        let leaf = c["leafMost"].as_u64().unwrap() as usize;
        let bin = c["binDepth"].as_i64().unwrap() as i32;
        let (file, nodes) = build(name, leaf, bin);
        let want = c["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), want.len(), "{name} {leaf} {bin}: nodes");
        let copc = open(&file).unwrap();
        assert_eq!(copc.kind, Kind::Copc);
        let info = copc.info.unwrap();
        assert_eq!(info.halfsize, c["half"].as_f64().unwrap());
        assert_eq!(info.spacing, c["spacing"].as_f64().unwrap());
        for k in 0..3 {
            assert_eq!(info.center[k], c["center"][k].as_f64().unwrap());
        }
        assert_eq!(copc.head.count, c["count"].as_u64().unwrap());
        for (n, w) in nodes.iter().zip(want) {
            let key = Key {
                d: w["key"][0].as_i64().unwrap() as i32,
                x: w["key"][1].as_i64().unwrap() as i32,
                y: w["key"][2].as_i64().unwrap() as i32,
                z: w["key"][3].as_i64().unwrap() as i32,
            };
            assert_eq!(n.key, key, "{name}: node order");
            assert_eq!(
                n.count,
                w["count"].as_u64().unwrap(),
                "{name} {key:?}: count"
            );
            if let Some(run) = copc.node_run(key) {
                let a = run.need.offset as usize;
                let b = a + run.need.len as usize;
                let mut recs = Vec::new();
                copc.records(&run, &file[a..b], &mut recs).unwrap();
                assert_eq!(
                    fnv(&recs),
                    w["hash"].as_str().unwrap(),
                    "{name} {key:?}: records"
                );
            }
        }
        // As a plain LAZ too: every chunk in the table, every point once.
        let all = records(&copc, &file);
        assert_eq!(all.len() / copc.layout.len, copc.head.count as usize);
    }
}

#[test]
fn the_sample_index_is_the_committed_file() {
    let (file, _) = build("index-sample.laz", 400, 2);
    let path = fixtures().join("files/index-sample.copc.laz");
    if std::env::var_os("KENTOS_WRITE_COPC").is_some() {
        std::fs::write(&path, &file).unwrap();
    }
    let on_disk = std::fs::read(&path).unwrap();
    assert!(
        on_disk == file,
        "index-sample.copc.laz differs; KENTOS_WRITE_COPC=1 writes it, pointcloud_index_cases.py --verify checks it"
    );
}

/// A picture decodes only what its look reads (`chunks::selection_for`): for
/// every look's needs, a node's positions and returns and the classes,
/// intensities and colours the look reads are the same as from its whole
/// records, for every node of a COPC; what the look does not read is not held.
#[test]
fn a_pictures_records_hold_what_the_looks_read() {
    use kentos_pointcloud::nodes::{Needs, decode};
    let all_needs = [
        Needs::ALL,
        Needs::default(),
        Needs {
            rgb: true,
            ..Needs::default()
        },
        Needs {
            class: true,
            ..Needs::default()
        },
        Needs {
            intensity: true,
            ..Needs::default()
        },
    ];
    for name in ["index-sample.copc.laz"] {
        let bytes = std::fs::read(fixtures().join("files").join(name)).unwrap();
        let copc = open(&bytes).unwrap();
        let hierarchy = copc.hierarchy.as_ref().unwrap();
        for needs in all_needs {
            let mut checked = 0;
            for n in hierarchy.sorted() {
                let Some(run) = copc.node_run(n.key) else {
                    continue;
                };
                let raw =
                    &bytes[run.need.offset as usize..(run.need.offset + run.need.len) as usize];
                let (mut whole, mut view) = (Vec::new(), Vec::new());
                copc.records(&run, raw, &mut whole).unwrap();
                copc.view_records(&run, raw, &mut view, needs).unwrap();
                assert_eq!(whole.len(), view.len());
                let (s, o) = (copc.head.scale, copc.head.offset);
                let a = decode(&copc.layout, &whole, s, o, [0.0; 3], needs);
                let b = decode(&copc.layout, &view, s, o, [0.0; 3], needs);
                assert_eq!(a, b, "{name} {:?} {needs:?}", n.key);
                assert_eq!(a.class.is_empty(), !needs.class || a.is_empty());
                assert_eq!(a.intensity.is_empty(), !needs.intensity || a.is_empty());
                checked += a.len();
            }
            assert_eq!(checked as u64, copc.head.count, "{name}: every point");
        }
    }
}

/// What a look reads: classes by class or to hide some, intensities and colours by their looks.
#[test]
fn a_looks_needs() {
    use kentos_contracts::{CloudRender, PointCloudStyle, PointShape, PointSizeUnit};
    use kentos_pointcloud::nodes::Needs;
    let style = |render, hidden: Vec<u8>| PointCloudStyle {
        render,
        ramp: None,
        invert: false,
        min: Some(0.0),
        max: Some(1.0),
        hidden,
        rgb8: false,
        size: 2.0,
        size_unit: PointSizeUnit::Px,
        shape: PointShape::Round,
    };
    let n = |class, intensity, rgb| Needs {
        class,
        intensity,
        rgb,
    };
    assert_eq!(
        Needs::of(&style(CloudRender::Rgb, vec![])),
        n(false, false, true)
    );
    assert_eq!(
        Needs::of(&style(CloudRender::Rgb, vec![7])),
        n(true, false, true)
    );
    assert_eq!(
        Needs::of(&style(CloudRender::Classification, vec![])),
        n(true, false, false)
    );
    assert_eq!(
        Needs::of(&style(CloudRender::Elevation, vec![])),
        n(false, false, false)
    );
    assert_eq!(
        Needs::of(&style(CloudRender::Intensity, vec![18])),
        n(true, true, false)
    );
    assert_eq!(
        Needs::of(&style(CloudRender::Returns, vec![])),
        Needs::default()
    );
    assert_eq!(
        Needs::of(&style(CloudRender::Single, vec![])),
        Needs::default()
    );
    assert!(Needs::ALL.covers(n(true, true, true)));
    assert!(n(true, false, true).covers(n(false, false, true)));
    assert!(!n(false, false, true).covers(n(true, false, false)));
}
