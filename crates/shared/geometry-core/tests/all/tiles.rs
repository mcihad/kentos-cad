//! The map services' tiles (docs/adr/0208 §3) against PROJ in
//! `fixtures/services/v1/tiles.json` (`scripts/fixtures/service_tiles_cases.py`:
//! pyproj and the ADR's rules, no KentOS code): a view's box, centre and
//! pixel in a grid's system, the level and the tiles it shows in their
//! order, a tile's mesh in the project's system and Bing's quadkeys. Boxes,
//! centres and nodes within 1e-6 m (1e-11 degrees), the pixel (their
//! differences) within twice that, levels and tiles exactly.

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::crs::{System, transform};
use kentos_geometry_core::geom::tiles::{Grid, TileRef, mesh, quadkey, shown};
use serde_json::Value;

fn fixture() -> Value {
    let path = format!(
        "{}/../../../fixtures/services/v1/tiles.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.service-tiles");
    assert_eq!(file["version"], 1);
    file
}

fn num(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn system(file: &Value, srid: u64) -> System {
    let v = &file["systems"][srid.to_string()];
    System::from_json(&Json::parse(&v.to_string()).expect("the core reads the JSON"))
        .expect("a system")
}

fn grid(v: &Value) -> (Grid, u64) {
    let tile = v["tile"].as_u64().expect("a tile side") as u32;
    let max = v["max"].as_u64().expect("a level") as u32;
    match v["kind"].as_str() {
        Some("webMercator") => (Grid::web_mercator(tile, max), 3857),
        Some("geographic") => (Grid::geographic(tile, max), 4326),
        other => panic!("a grid kind: {other:?}"),
    }
}

/// A point of `from` in `to`, as the apps move one; the same system as it is.
fn mover(from: Option<System>, to: Option<System>) -> impl Fn(f64, f64) -> Option<(f64, f64)> {
    move |x, y| match (&from, &to) {
        (Some(a), Some(b)) => transform(a, b, Vec2::new(x, y)).map(|t| (t.point.x, t.point.y)),
        _ => Some((x, y)),
    }
}

fn tol(service: u64) -> f64 {
    if service == 4326 { 1e-11 } else { 1e-6 }
}

#[test]
fn a_view_shows_the_tiles_proj_and_the_rules_give_in_their_order() {
    let file = fixture();
    let views = file["views"].as_array().expect("views");
    assert!(views.len() >= 7);
    for v in views {
        let title = v["title"].as_str().expect("a title");
        let project = v["project"].as_u64().expect("a project");
        let (g, service) = grid(&v["grid"]);
        let (from, to) = if project == service {
            (None, None)
        } else {
            (Some(system(&file, project)), Some(system(&file, service)))
        };
        let b = &v["view"];
        let view = [num(&b[0]), num(&b[1]), num(&b[2]), num(&b[3])];
        let lo = v["levels"][0].as_u64().expect("a level") as u32;
        let hi = v["levels"][1].as_u64().expect("a level") as u32;
        let got = shown(&g, lo, hi, view, num(&v["pxPerUnit"]), mover(from, to))
            .unwrap_or_else(|| panic!("{title}: nothing shown"));
        let want = &v["expect"];
        let t = tol(service);
        for (i, w) in want["bbox"].as_array().expect("a box").iter().enumerate() {
            assert!(
                (got.view.bbox[i] - num(w)).abs() <= t,
                "{title}: bbox[{i}] {} ≠ {}",
                got.view.bbox[i],
                num(w)
            );
        }
        for (i, w) in want["center"]
            .as_array()
            .expect("a centre")
            .iter()
            .enumerate()
        {
            assert!(
                (got.view.center[i] - num(w)).abs() <= t,
                "{title}: centre[{i}] {} ≠ {}",
                got.view.center[i],
                num(w)
            );
        }
        // The pixel is the points' differences: within twice their tolerance.
        let upp = num(&want["unitsPerPx"]);
        assert!(
            (got.view.units_per_px - upp).abs() <= 2.0 * t,
            "{title}: pixel {} ≠ {upp}",
            got.view.units_per_px
        );
        assert_eq!(
            got.level as u64,
            want["level"].as_u64().expect("a level"),
            "{title}: level"
        );
        let tiles: Vec<[u64; 3]> = got
            .tiles
            .iter()
            .map(|t| [u64::from(t.level), t.col, t.row])
            .collect();
        let expected: Vec<[u64; 3]> = want["tiles"]
            .as_array()
            .expect("tiles")
            .iter()
            .map(|t| {
                [
                    t[0].as_u64().expect("z"),
                    t[1].as_u64().expect("x"),
                    t[2].as_u64().expect("y"),
                ]
            })
            .collect();
        assert_eq!(tiles, expected, "{title}: tiles");
    }
}

#[test]
fn a_tiles_mesh_lies_where_proj_puts_its_nodes() {
    let file = fixture();
    for m in file["meshes"].as_array().expect("meshes") {
        let title = m["title"].as_str().expect("a title");
        let project = m["project"].as_u64().expect("a project");
        let (g, service) = grid(&m["grid"]);
        let t = &m["tile"];
        let tile = TileRef {
            level: t[0].as_u64().expect("z") as u32,
            col: t[1].as_u64().expect("x"),
            row: t[2].as_u64().expect("y"),
        };
        let n = m["n"].as_u64().expect("cells") as u32;
        let mut out = Vec::new();
        mesh(
            &g,
            tile,
            n,
            mover(Some(system(&file, service)), Some(system(&file, project))),
            &mut out,
        );
        let want: Vec<f64> = m["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .map(num)
            .collect();
        assert_eq!(out.len(), want.len(), "{title}: nodes");
        for (i, (a, b)) in out.iter().zip(&want).enumerate() {
            assert!((a - b).abs() <= 1e-6, "{title}: node value {i}: {a} ≠ {b}");
        }
    }
}

#[test]
fn quadkeys_are_bings() {
    let file = fixture();
    for q in file["quadkeys"].as_array().expect("quadkeys") {
        let t = &q["tile"];
        let tile = TileRef {
            level: t[0].as_u64().expect("z") as u32,
            col: t[1].as_u64().expect("x"),
            row: t[2].as_u64().expect("y"),
        };
        assert_eq!(quadkey(tile), q["key"].as_str().expect("a key"), "{tile:?}");
    }
}
