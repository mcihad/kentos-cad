//! The local copy of a large project, measured (docs/adr/0043): keeping it
//! after an online opening (`Replica::reset`: the verified KCAD v2 bytes and
//! the versions, flushed), adding steps (`append`, each flushed), opening it
//! without a connection (`load`: decode, the steps, the document) and
//! rewriting it from the sync's base (`compact`). Drawings of parcels with
//! 20 vertices, three attributes and a label, as the desktop's own save and
//! open are measured (apps/desktop/src/perf.rs). Not a correctness test and
//! not run by default:
//!
//! ```text
//! cargo test --release -p kentos-cloud --test replica_perf -- --ignored --nocapture
//! ```
//!
//! `KENTOS_PERF_SIZES` (parcels, comma separated) changes what is measured.
//! The copies go to `KENTOS_PERF_DIR` (default the system's temporary folder)
//! and are removed. Measure on a real disk: a temporary folder in memory
//! (tmpfs) makes every flush free.

use std::collections::BTreeMap;
use std::time::Instant;

use kentos_cloud::{BaseObject, BaseStep, Opened, ProjectSync, ReplicaStore, Source};
use kentos_contracts::{
    AccessSource, AngleUnit, AreaUnit, DocumentSnapshotV2, Entity, EntityBase, EntityId, LayerNode,
    LayerNodeType, LayerStyle, LineType, PathEntity, ProjectAccessView, ProjectInfo,
    ProjectPermission, ProjectRole, ProjectSettings, ProjectState, ProjectStorage, ProjectStyles,
    TenantKind, Vec2,
};
use kentos_domain::Document;
use uuid::Uuid;

fn parcel(i: usize, shift: f64) -> Entity {
    let (x0, y0) = (
        486_000.0 + (i % 300) as f64 * 31.7 + shift,
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
    Entity::Polygon(PathEntity {
        base: EntityBase {
            id: i as u32 + 1,
            layer_id: "parsel".into(),
            color: None,
            attrs: BTreeMap::from([
                ("Ada".to_owned(), format!("{}", 100 + i / 50)),
                ("Parsel".to_owned(), format!("{}", i % 50 + 1)),
                ("Nitelik".to_owned(), "Arsa".to_owned()),
            ]),
            label: Some(format!("{}/{}", 100 + i / 50, i % 50 + 1)),
            symbol: None,
        },
        pts,
        bulges: None,
        holes: None,
    })
}

fn uid(i: usize) -> Uuid {
    let mut id = [
        0x01, 0x92, 0xf5, 0xa0, 0x7c, 0x3e, 0x70, 0x00, 0x80, 0, 0, 0, 0, 0, 0, 0,
    ];
    id[10..].copy_from_slice(&(i as u64 + 1).to_be_bytes()[2..]);
    Uuid::from_bytes(id)
}

fn opened(n: usize) -> Opened {
    let settings = ProjectSettings {
        srid: 5256,
        length_decimals: 3,
        area_decimals: 2,
        area_unit: AreaUnit::M2,
        angle_unit: AngleUnit::Grad,
        plot_scale: 1000.0,
        workspace: None,
        drawing_font: None,
    };
    let layers = vec![LayerNode {
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
    }];
    let project = Uuid::now_v7();
    let snapshot = DocumentSnapshotV2 {
        format: "kentos.document".into(),
        version: 2,
        name: format!("olcum-{n}"),
        settings: settings.clone(),
        origin: Vec2 {
            x: 486_000.0,
            y: 4_420_000.0,
        },
        home_view: None,
        layers: layers.clone(),
        active_layer: "parsel".into(),
        entities: (0..n).map(|i| parcel(i, 0.0)).collect(),
        uids: (0..n).map(|i| EntityId(uid(i).into_bytes())).collect(),
        styles: ProjectStyles::default(),
        project_id: Some(kentos_contracts::ProjectId(project.into_bytes())),
        migrated_from: None,
    };
    let document = Document::from_snapshot_v2(snapshot).expect("the drawing opens");
    let tenant = Uuid::now_v7();
    Opened {
        tenant,
        project,
        info: ProjectInfo {
            id: project.to_string(),
            tenant_id: tenant.to_string(),
            tenant_name: "Ölçüm".into(),
            tenant_kind: TenantKind::Organization,
            access: ProjectAccessView {
                role: ProjectRole::Editor,
                via: AccessSource::Grant,
                permissions: vec![ProjectPermission::Read, ProjectPermission::FeatureWrite],
            },
            state: ProjectState::Active,
            name: format!("olcum-{n}"),
            settings,
            origin: Vec2 {
                x: 486_000.0,
                y: 4_420_000.0,
            },
            home_view: None,
            layers,
            active_layer: "parsel".into(),
            styles: ProjectStyles::default(),
            meta_version: "1".into(),
            data_revision: "1".into(),
            feature_count: n.to_string(),
            event_cursor: "1".into(),
            storage: ProjectStorage::Database,
        },
        document,
        source: Source::Database {
            versions: (0..n).map(|i| (uid(i), "1".to_string())).collect(),
        },
    }
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

#[test]
#[ignore = "a measurement, run by hand in release mode"]
fn the_local_copy_of_a_large_project() {
    let sizes: Vec<usize> = std::env::var("KENTOS_PERF_SIZES")
        .ok()
        .map(|s| s.split(',').filter_map(|n| n.trim().parse().ok()).collect())
        .unwrap_or_else(|| vec![10_000, 100_000]);
    println!(
        "| Parsel | Kopyala (reset) | 100 adım (append) | Aç (load) | Topla (compact) | Kopya boyu |"
    );
    println!("|---:|---:|---:|---:|---:|---:|");
    for n in sizes {
        let root = std::env::var_os("KENTOS_PERF_DIR")
            .map_or_else(std::env::temp_dir, std::path::PathBuf::from)
            .join(format!("kentos-replica-perf-{}", Uuid::now_v7()));
        let store = ReplicaStore::new(&root);
        let o = opened(n);
        let mut replica = store
            .open("https://kentos.olcum", "olcum", o.tenant, o.project)
            .expect("the copy opens");
        let t = Instant::now();
        replica.reset(&o).expect("kept");
        let reset = ms(t);
        // 100 steps of 10 changed parcels each, every one flushed to the disk.
        let t = Instant::now();
        for s in 0..100usize {
            let put = (0..10)
                .map(|k| {
                    let i = (s * 10 + k) % n;
                    BaseObject {
                        id: uid(i).to_string(),
                        version: format!("{}", s + 2),
                        entity: parcel(i, 0.5),
                    }
                })
                .collect();
            replica
                .append(&BaseStep {
                    cursor: Some(format!("{}", s + 2)),
                    put,
                    ..BaseStep::default()
                })
                .expect("appended");
        }
        let append = ms(t);
        let t = Instant::now();
        let back = replica.load().expect("loads").expect("kept");
        let load = ms(t);
        assert_eq!(back.document.len(), n);
        let sync = ProjectSync::new(&back).expect("a database project");
        let t = Instant::now();
        replica
            .compact(&sync.base(&back.document))
            .expect("compacted");
        let compact = ms(t);
        let size: u64 = std::fs::read_dir(
            std::fs::read_dir(root.join("kentos.olcum").join("olcum"))
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path(),
        )
        .unwrap()
        .filter_map(Result::ok)
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum();
        println!(
            "| {n} | {reset:.0} ms | {append:.0} ms | {load:.0} ms | {compact:.0} ms | {:.1} MB |",
            size as f64 / 1e6
        );
        drop(replica);
        let _ = std::fs::remove_dir_all(root);
    }
}
