//! A join view on a source's table, timed (docs/adr/0632 §5): 100 000 parcels in the drawing, a land registry table of
//! 300 000 owners (three per parcel) in a GeoPackage written once to `.run/perf/tapu-300000.gpkg`; the view read whole
//! with and without its summing columns, and the same view on a layer of the drawing.
//!
//! ```text
//! cargo test --release -p kentos-native-application --test all join_source_timing -- --ignored --nocapture
//! ```

use std::path::PathBuf;
use std::time::Instant;

use kentos_commands::{DocumentSnapshotV1, JoinAggregate, JoinHow, JoinKeep, JoinSource, JoinView};
use kentos_domain::Document;
use kentos_geopackage::{Column, ColumnKind, DataType, GeoPackage, NewRow, TableDef, Value};
use kentos_native_application::joins_query::read;
use serde_json::json;

const PARCELS: usize = 100_000;
const OWNERS: usize = 3;

fn write_registry(path: &std::path::Path) {
    let mut gp = GeoPackage::create(path).expect("creates");
    let column = |name: &str, kind: ColumnKind| Column {
        name: name.into(),
        decl: String::new(),
        kind,
        not_null: false,
        default: None,
    };
    let def = TableDef {
        name: "tapu".into(),
        data_type: DataType::Attributes,
        identifier: Some("tapu".into()),
        description: None,
        srs_id: None,
        geometry: None,
        columns: vec![
            column("parsel_no", ColumnKind::Text(None)),
            column("malik", ColumnKind::Text(None)),
            column("pay", ColumnKind::Real),
        ],
    };
    let rows: Vec<NewRow> = (0..PARCELS * OWNERS)
        .map(|i| NewRow {
            fid: None,
            geometry: None,
            values: vec![
                Value::Text(format!("{}/{}", 100 + i / OWNERS / 1000, i / OWNERS % 1000)),
                Value::Text(format!("Malik {i}")),
                Value::Real(1.0 / OWNERS as f64),
            ],
        })
        .collect();
    gp.write_table(&def, &rows, None).expect("written");
    gp.close().expect("closed");
}

/// The drawing: the parcels as points with their no, and the same owners as a layer.
fn drawing() -> Document {
    let point = |id: usize, layer: &str, attrs: serde_json::Value| {
        json!({"kind": "point", "id": id, "layerId": layer, "attrs": attrs,
               "p": {"x": 500_000.0 + (id % 1000) as f64, "y": 4_400_000.0 + (id / 1000) as f64}})
    };
    let mut entities: Vec<serde_json::Value> = (0..PARCELS)
        .map(|i| {
            point(
                i + 1,
                "parsel",
                json!({"no": format!("{}/{}", 100 + i / 1000, i % 1000)}),
            )
        })
        .collect();
    entities.extend((0..PARCELS * OWNERS).map(|i| {
        point(
            PARCELS + i + 1,
            "malik",
            json!({"parsel": format!("{}/{}", 100 + i / OWNERS / 1000, i / OWNERS % 1000),
                   "malik": format!("Malik {i}"), "pay": "0.333"}),
        )
    }));
    let layer = |id: &str| {
        json!({"id": id, "name": id, "type": "layer", "visible": true, "locked": false, "expanded": true,
               "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []})
    };
    let snapshot = json!({
        "format": "kentos.document", "version": 1, "name": "Tapu",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 1000},
        "origin": {"x": 500000, "y": 4400000},
        "layers": [layer("parsel"), layer("malik")], "activeLayer": "parsel", "entities": entities,
        "styles": {"items": [], "categories": []}
    });
    Document::from_snapshot(
        DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("a drawing"),
    )
    .expect("opens")
}

fn sums() -> Vec<JoinAggregate> {
    vec![
        JoinAggregate {
            name: "Malik sayısı".into(),
            how: JoinHow::Count,
            field: None,
        },
        JoinAggregate {
            name: "Pay toplamı".into(),
            how: JoinHow::Sum,
            field: Some("pay".into()),
        },
        JoinAggregate {
            name: "Ortalama pay".into(),
            how: JoinHow::Mean,
            field: Some("pay".into()),
        },
    ]
}

#[test]
#[ignore = "a measurement, run by hand"]
fn join_source_timing() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../.run/perf");
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join(format!("tapu-{}.gpkg", PARCELS * OWNERS));
    if !file.exists() {
        let t = Instant::now();
        write_registry(&file);
        println!("tapu yazıldı: {:.1} s", t.elapsed().as_secs_f64());
    }
    let doc = drawing();
    let view = |source: bool, aggregates: Vec<JoinAggregate>| JoinView {
        id: "v".into(),
        name: "V".into(),
        layer: "parsel".into(),
        key: "no".into(),
        with: if source {
            String::new()
        } else {
            "malik".into()
        },
        with_key: if source {
            "parsel_no".into()
        } else {
            "parsel".into()
        },
        keep: JoinKeep::All,
        fields: Vec::new(),
        source: source.then(|| JoinSource {
            path: file.to_string_lossy().into_owned(),
            table: "tapu".into(),
        }),
        aggregates,
    };
    for (what, v) in [
        ("kaynak tablosu, toplamasız", view(true, Vec::new())),
        ("kaynak tablosu, üç toplama sütunu", view(true, sums())),
        ("katman, toplamasız", view(false, Vec::new())),
        ("katman, üç toplama sütunu", view(false, sums())),
    ] {
        let mut best = f64::MAX;
        let mut rows = 0;
        for _ in 0..3 {
            let t = Instant::now();
            let r = read(&doc, &v);
            best = best.min(t.elapsed().as_secs_f64() * 1000.0);
            assert!(r.broken.is_none(), "{:?}", r.broken);
            rows = r.rows.len();
        }
        println!("{what:<36} {rows:>8} satır {best:>10.1} ms");
    }
}
