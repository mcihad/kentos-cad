//! How long a million parcels take through the GeoPackage driver (docs/adr/9100 §9): adding the layer, a street, a
//! district and the whole city drawn, a filtered count, a table page and a quick search. The file is written once by
//! the GeoPackage crate in one transaction; run in release:
//! `cargo test --release -p kentos-data-gpkg --test perf -- --ignored --nocapture`.

use std::path::PathBuf;
use std::time::Instant;

use kentos_data::{Cancel, Condition, Coord, Curve, Driver, Geometry, Order, Request, Source};
use kentos_data_gpkg::Gpkg;
use kentos_geopackage::{
    Column, ColumnKind, DataType, GeoPackage, GeometryColumn, NewRow, TableDef, Value,
};

const SIDE: usize = 1000;
const CELL: f64 = 30.0;
const X0: f64 = 587_000.0;
const Y0: f64 = 4_400_000.0;

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn view(source: &dyn Source, r: &Request) -> (u64, f64, f64) {
    let t = Instant::now();
    let mut first = None;
    let mut n = 0u64;
    source
        .features("parseller", r, &Cancel::new(), &mut |part| {
            if first.is_none() {
                first = Some(ms(t));
            }
            n += part.len() as u64;
            true
        })
        .expect("features");
    (n, first.unwrap_or(0.0), ms(t))
}

#[test]
#[ignore]
fn a_million_parcels() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("kentos-data-gpkg");
    std::fs::create_dir_all(&dir).expect("dir");
    let path = dir.join("milyon.gpkg");
    if !path.exists() {
        let t = Instant::now();
        let mut gp = GeoPackage::create(&path).expect("create");
        let col = |name: &str, kind: ColumnKind| Column {
            name: name.into(),
            decl: String::new(),
            kind,
            not_null: false,
            default: None,
        };
        let def = TableDef {
            name: "parseller".into(),
            data_type: DataType::Features,
            identifier: None,
            description: None,
            srs_id: Some(4326),
            geometry: Some(GeometryColumn {
                name: "geom".into(),
                type_name: "POLYGON".into(),
                z: 0,
                m: 0,
            }),
            columns: vec![
                col("ada", ColumnKind::Integer),
                col("parsel", ColumnKind::Integer),
                col("mahalle", ColumnKind::Text(None)),
                col("alan", ColumnKind::Real),
                col("kullanim", ColumnKind::Text(None)),
            ],
        };
        let kinds = ["Konut", "Ticaret", "Park", "Eğitim"];
        let mut rows = Vec::with_capacity(SIDE * SIDE);
        for i in 0..SIDE {
            for j in 0..SIDE {
                let (x, y) = (X0 + j as f64 * CELL, Y0 + i as f64 * CELL);
                let ring = vec![
                    Coord::xy(x + 2.0, y + 2.0),
                    Coord::xy(x + CELL - 2.0, y + 2.0),
                    Coord::xy(x + CELL - 2.0, y + CELL / 2.0),
                    Coord::xy(x + CELL - 2.0, y + CELL - 2.0),
                    Coord::xy(x + CELL / 2.0, y + CELL - 2.0),
                    Coord::xy(x + 2.0, y + CELL - 2.0),
                    Coord::xy(x + 2.0, y + 2.0),
                ];
                rows.push(NewRow {
                    fid: None,
                    geometry: Some(Geometry::Polygon(vec![Curve::Line(ring)])),
                    values: vec![
                        Value::Integer(100 + i as i64),
                        Value::Integer(1 + j as i64),
                        Value::Text(format!("Mahalle {}", i / 50)),
                        Value::Real((CELL - 4.0) * (CELL - 4.0) - ((i * 7 + j) % 300) as f64),
                        Value::Text(kinds[(i * 31 + j) % 4].into()),
                    ],
                });
            }
        }
        gp.write_table(&def, &rows, None).expect("write");
        gp.close().expect("close");
        println!("dosya yazıldı: {:.0} ms", ms(t));
    }
    let t = Instant::now();
    let source = Gpkg.open(path.to_str().expect("utf-8")).expect("open");
    println!("açılış: {:.1} ms", ms(t));
    let t = Instant::now();
    let tables = source.tables().expect("tables");
    println!(
        "katalog ({} tablo, sayılarıyla): {:.1} ms",
        tables.len(),
        ms(t)
    );
    let t = Instant::now();
    let schema = source.schema("parseller").expect("schema");
    println!("şema ({} alan): {:.1} ms", schema.fields.len(), ms(t));
    for (name, w, h, tol) in [
        ("sokak 400 × 300 m", 400.0, 300.0, 0.3),
        ("mahalle 3 × 2 km", 3000.0, 2000.0, 2.0),
        ("ilçe 12 × 8 km", 12_000.0, 8000.0, 8.0),
        ("bütün kent 30 × 30 km", 30_000.0, 30_000.0, 20.0),
    ] {
        let b = [X0 + 9000.0, Y0 + 9000.0, X0 + 9000.0 + w, Y0 + 9000.0 + h];
        let b = if w >= 30_000.0 {
            [X0, Y0, X0 + w, Y0 + h]
        } else {
            b
        };
        let r = Request::view(b, vec!["kullanim".into()], Some(tol));
        let (rows, first, all) = view(source.as_ref(), &r);
        println!("{name}: {rows} satır, ilk parça {first:.1} ms, tamamı {all:.0} ms");
    }
    let t = Instant::now();
    let n = source
        .count(
            "parseller",
            &Request {
                condition: Some(Condition::eq("kullanim", "Park")),
                ..Request::all()
            },
            false,
        )
        .expect("count");
    println!("süzgeçli sayım (kullanım = Park) {n}: {:.0} ms", ms(t));
    for offset in [0u64, 500_000] {
        let r = Request {
            order: vec![Order {
                field: "alan".into(),
                descending: true,
            }],
            limit: Some(50),
            offset,
            geometry: false,
            ..Request::all()
        };
        let (rows, _, all) = view(source.as_ref(), &r);
        println!("tablo sayfası (alana göre, {offset}. satırdan) {rows} satır: {all:.0} ms");
    }
    let r = Request {
        condition: Some(Condition::Contains {
            fields: vec!["mahalle".into(), "kullanim".into()],
            text: "EĞİT".into(),
        }),
        limit: Some(50),
        geometry: false,
        ..Request::all()
    };
    let (rows, _, all) = view(source.as_ref(), &r);
    println!("hızlı arama (“EĞİT”, ilk 50) {rows} satır: {all:.0} ms");
}
