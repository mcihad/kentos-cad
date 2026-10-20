//! GML's write and read times (docs/adr/0545 Doğrulama): `cargo test --release -p kentos-native-application --test
//! all gml_timing -- --ignored --nocapture`.

use std::time::Instant;

use kentos_geopackage::{ColumnKind, Coord, Curve, Geometry, Value};
use kentos_native_application::gml_read;
use kentos_native_application::gml_write::{self, Notes, OutColumn, OutType};

fn c(x: f64, y: f64) -> Coord {
    Coord::xy(x, y)
}

#[test]
#[ignore = "timing, run by hand in release (docs/adr/0545)"]
fn gml_timing() {
    // 100 000 points with four properties, and 10 000 areas of 12 vertices and an arc: written and read.
    let columns = vec![
        OutColumn { name: "ad".into(), kind: ColumnKind::Text(None) },
        OutColumn { name: "no".into(), kind: ColumnKind::Integer },
        OutColumn { name: "kot".into(), kind: ColumnKind::Real },
        OutColumn { name: "var".into(), kind: ColumnKind::Boolean },
    ];
    let points = OutType {
        name: "nokta".into(),
        columns: columns.clone(),
        features: (0..100_000)
            .map(|i| {
                let z = 100.0 + i as f64 * 0.001;
                let p = Coord {
                    x: 487000.0 + i as f64 * 0.25,
                    y: 4420000.0 + i as f64 * 0.5,
                    z: Some(z),
                    m: None,
                };
                (
                    Some(Geometry::Point(Some(p))),
                    vec![
                        Value::Text(format!("N{i}")),
                        Value::Integer(i),
                        Value::Real(z),
                        Value::Integer(i % 2),
                    ],
                )
            })
            .collect(),
    };
    let areas = OutType {
        name: "alan".into(),
        columns,
        features: (0..10_000)
            .map(|i| {
                let x = 487000.0 + i as f64 * 30.0;
                let mut ring: Vec<Coord> = (0..11)
                    .map(|k| c(x + (k as f64).cos() * 10.0, 4420000.0 + (k as f64).sin() * 10.0))
                    .collect();
                ring.push(ring[0]);
                (
                    Some(Geometry::Polygon(vec![Curve::Compound(vec![
                        Curve::Line(ring),
                        Curve::Arc(vec![c(x, 4420000.0), c(x + 1.0, 4420005.0), c(x + 2.0, 4420000.0)]),
                    ])])),
                    vec![
                        Value::Text(format!("A{i}")),
                        Value::Integer(i),
                        Value::Real(i as f64),
                        Value::Null,
                    ],
                )
            })
            .collect(),
    };
    for (name, t) in [("100 000 nokta", points), ("10 000 alan", areas)] {
        let n = t.features.len();
        let started = Instant::now();
        let text = gml_write::write(std::slice::from_ref(&t), Some(5256), None, &mut Notes::default());
        let wrote = started.elapsed();
        let started = Instant::now();
        let parsed = gml_read::parse(&text, None, None).expect("reads");
        let read = started.elapsed();
        assert_eq!(parsed.tables[0].rows.len(), n);
        println!("{name}: yazma {wrote:?}, okuma {read:?} ({} MiB)", text.len() >> 20);
    }
}
