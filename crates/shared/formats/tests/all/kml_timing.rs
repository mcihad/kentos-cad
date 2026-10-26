//! KML, GPX and GeoJSON in WGS 84 and the import's move, on this machine,
//! release (docs/adr/0222 §9):
//!
//!     cargo test --release -p kentos-formats --test all kml_timing -- --ignored --nocapture
//!
//! 100 000 objects of a TUREF / TM36 drawing (40 000 parcels of five corners,
//! 30 000 three-corner paths, 30 000 points with elevations; 350 000
//! vertices): KML written (with each vertex moved into WGS 84) and read back,
//! the reading moved into the project's system, GPX and RFC 7946 GeoJSON
//! written. The web runs the same through WASM (apps/web/scripts/perf/kml.test.ts).

use std::time::Instant;

use kentos_contracts::{
    Entity, EntityBase, GeoJsonLayer, GeoJsonWriteInput, GpxWriteInput, KmlLayer, KmlWriteInput,
    PathEntity, PointEntity, Vec2,
};
use kentos_formats::{geojson, gnss::gpx_write, kml, reproject::reproject};
use kentos_geometry_core::crs::{Datum, System};

/// TUREF / TM36 (EPSG:5256).
fn tm36() -> System {
    System::Tm {
        datum: Datum::Turef,
        latitude_of_origin: None,
        central_meridian: 36.0,
        scale_factor: 1.0,
        false_easting: 500_000.0,
        false_northing: 0.0,
    }
}

fn base(id: u32, layer: &str) -> EntityBase {
    EntityBase {
        look: Default::default(),
        id,
        layer_id: layer.to_owned(),
        color: None,
        attrs: [("Ad".to_owned(), id.to_string())].into(),
        label: None,
        symbol: None,
        line_weight: None,
        label_pins: Vec::new(),
        attachments: Vec::new(),
    }
}

/// The scene: parcels, paths and points over a 10 km square.
fn scene() -> Vec<Entity> {
    let (e0, n0) = (480_000.0, 4_410_000.0);
    let mut out = Vec::with_capacity(100_000);
    let v = |x: f64, y: f64| Vec2 { x, y };
    for i in 0..40_000u32 {
        let (x, y) = (
            e0 + f64::from(i % 200) * 50.0,
            n0 + f64::from(i / 200) * 50.0,
        );
        out.push(Entity::Polygon(PathEntity {
            space_separator: None,
            ms: None,
            base: base(i + 1, "parsel"),
            pts: vec![
                v(x, y),
                v(x + 40.0, y),
                v(x + 45.0, y + 20.0),
                v(x + 40.0, y + 40.0),
                v(x, y + 40.0),
            ],
            bulges: None,
            holes: None,
            zs: None,
            parts: None,
            closed: false,
            widths: None,
        }));
    }
    for i in 0..30_000u32 {
        let (x, y) = (
            e0 + f64::from(i % 150) * 66.0,
            n0 + f64::from(i / 150) * 50.0 + 5.0,
        );
        out.push(Entity::Polyline(PathEntity {
            space_separator: None,
            ms: None,
            base: base(40_001 + i, "yol"),
            pts: vec![v(x, y), v(x + 30.0, y + 3.0), v(x + 60.0, y)],
            bulges: None,
            holes: None,
            zs: None,
            parts: None,
            closed: false,
            widths: None,
        }));
    }
    for i in 0..30_000u32 {
        let (x, y) = (
            e0 + f64::from(i % 150) * 66.0 + 3.0,
            n0 + f64::from(i / 150) * 50.0 + 25.0,
        );
        out.push(Entity::Point(PointEntity {
            base: base(70_001 + i, "nokta"),
            p: v(x, y),
            z: Some(850.0 + f64::from(i % 97)),
            parts: None,
        }));
    }
    out
}

fn time<T>(what: &str, budget_ms: f64, f: impl Fn() -> T) -> T {
    // One warm-up, then the best of three.
    let _ = f();
    let mut best = f64::INFINITY;
    let mut out = None;
    for _ in 0..3 {
        let t = Instant::now();
        let r = f();
        best = best.min(t.elapsed().as_secs_f64() * 1e3);
        out = Some(r);
    }
    println!("{what}: {best:.0} ms (bütçe {budget_ms:.0} ms)");
    assert!(
        best <= budget_ms,
        "{what}: {best:.0} ms > {budget_ms:.0} ms"
    );
    out.expect("ran")
}

#[test]
#[ignore = "timings, run by hand in release"]
fn kml_timing() {
    let entities = scene();
    let from = tm36();
    let layers = ["parsel", "yol", "nokta"]
        .map(|id| KmlLayer {
            id: id.to_owned(),
            name: id.to_owned(),
            path: vec!["Ölçüm".to_owned()],
            color: "#336699".to_owned(),
            line_weight: Some(0.25),
        })
        .to_vec();
    let input = KmlWriteInput {
        entities: entities.clone(),
        layers,
        name: "Ölçüm".to_owned(),
        blocks: Vec::new(),
        kmz: false,
    };
    let (bytes, _) = time(
        "KML yaz (100 000 nesne, 350 000 köşe WGS 84'e)",
        900.0,
        || kml::write(&input, &from, &[]),
    );
    println!("  KML: {:.1} MB", bytes.len() as f64 / 1e6);
    let kmz = KmlWriteInput {
        kmz: true,
        ..input.clone()
    };
    let (zipped, _) = time("KMZ yaz", 1600.0, || kml::write(&kmz, &from, &[]));
    println!("  KMZ: {:.1} MB", zipped.len() as f64 / 1e6);
    let read = time("KML oku", 900.0, || kml::read(&bytes, 0).expect("reads"));
    assert_eq!(read.entities.len(), 100_000);
    time("KMZ oku", 1200.0, || kml::read(&zipped, 0).expect("reads"));
    let wgs84 = System::Geographic {
        datum: Datum::Wgs84,
    };
    time("Projenin sistemine çevir (100 000 nesne)", 500.0, || {
        let mut r = read.clone();
        reproject(&mut r, &wgs84, &from, &[]);
        r
    });
    let gpx = GpxWriteInput {
        entities: entities.clone(),
        name: "Ölçüm".to_owned(),
        blocks: Vec::new(),
        routes: false,
    };
    time("GPX yaz", 900.0, || gpx_write::write(&gpx, &from, &[]));
    let geo = GeoJsonWriteInput {
        entities,
        layers: ["parsel", "yol", "nokta"]
            .map(|id| GeoJsonLayer {
                id: id.to_owned(),
                name: id.to_owned(),
                fields: Vec::new(),
            })
            .to_vec(),
        srid: 5256,
        name: "Ölçüm".to_owned(),
        blocks: Vec::new(),
    };
    time("GeoJSON yaz (WGS 84'e)", 900.0, || {
        geojson::write_wgs84(&geo, &from, &[])
    });
}
