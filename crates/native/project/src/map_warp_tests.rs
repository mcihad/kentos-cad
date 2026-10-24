//! Pictures, rasters and point clouds on the map's system (docs/adr/0630): the core's meshes against PROJ's, from
//! `fixtures/map-warp/v1/cases.json` (`scripts/fixtures/map_warp_cases.py`, no KentOS code). A TUREF / TM36 project
//! seen in Web Mercator, WGS 84 degrees, TM30, UTM 36N and ED50 / TM30: a raster tile's cells and its grid's nodes, a
//! picture's halvings and its first small triangles, a node's square, the map's linear part and the box a view shows.
//! Cells and levels exactly; points within 1e-3 m, 1e-8° of PROJ's (KentOS and PROJ differ by 0.1 mm on TUREF).

use kentos_geometry_core::Vec2;
use kentos_geometry_core::crs::map::MapProjection;
use serde_json::Value;

use crate::new_project::default_settings;
use crate::systems::map_projection;

fn cases() -> Value {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../fixtures/map-warp/v1/cases.json"
    ))
    .expect("the cases (scripts/fixtures/map_warp_cases.py)");
    serde_json::from_str(&text).expect("JSON")
}

fn map(project: u32, srid: u32) -> &'static MapProjection {
    let mut settings = default_settings(project);
    settings.map_srid = Some(srid);
    map_projection(&settings).expect("a map of another system")
}

fn pt(v: &Value) -> Vec2 {
    Vec2::new(v[0].as_f64().expect("x"), v[1].as_f64().expect("y"))
}

fn near(got: Vec2, want: &Value, degrees: bool, case: &str) {
    let tol = if degrees { 1e-8 } else { 1e-3 };
    let w = pt(want);
    assert!(
        (got.x - w.x).abs() < tol && (got.y - w.y).abs() < tol,
        "{case}: {got:?} ≠ {w:?}"
    );
}

#[test]
fn the_meshes_are_proj_s() {
    let all = cases();
    let project = all["project"].as_u64().expect("the project's system") as u32;
    let list = all["cases"].as_array().expect("cases");
    assert!(list.len() >= 20, "{}", list.len());
    for c in list {
        let name = c["name"].as_str().expect("a name");
        let srid = c["map"].as_u64().expect("a map") as u32;
        let m = map(project, srid);
        let degrees = srid == 4326;
        let tolerance = c["tolerance"].as_f64().unwrap_or(0.0);
        match c["kind"].as_str().expect("a kind") {
            "grid" | "node" => {
                let corners: Vec<Vec2> = c["corners"]
                    .as_array()
                    .expect("corners")
                    .iter()
                    .map(pt)
                    .collect();
                let corners = [corners[0], corners[1], corners[2], corners[3]];
                let most = c["most"].as_u64().expect("most") as usize;
                let n = m.grid_cells(corners, tolerance, most);
                assert_eq!(n as u64, c["cells"].as_u64().expect("cells"), "{name}");
                if let Some(want) = c["nodes"].as_array() {
                    let got = m.grid(corners, n).expect("reached");
                    assert_eq!(got.len(), want.len(), "{name}");
                    for (g, w) in got.iter().zip(want) {
                        near(*g, w, degrees, name);
                    }
                }
            }
            "mesh" => {
                let tris: Vec<[Vec2; 3]> = c["triangles"]
                    .as_array()
                    .expect("triangles")
                    .iter()
                    .map(|t| [pt(&t[0]), pt(&t[1]), pt(&t[2])])
                    .collect();
                let most = c["most"].as_u64().expect("most") as u32;
                let level = m.mesh_level(&tris, tolerance, most);
                assert_eq!(
                    u64::from(level),
                    c["level"].as_u64().expect("level"),
                    "{name}"
                );
                let mut out = Vec::new();
                for t in &tris {
                    m.mesh(*t, level, &mut out);
                }
                assert_eq!(
                    (out.len() / 3) as u64,
                    c["count"].as_u64().expect("count"),
                    "{name}"
                );
                for (k, tri) in c["first"].as_array().expect("first").iter().enumerate() {
                    for (i, w) in tri.as_array().expect("corners").iter().enumerate() {
                        near(out[k * 3 + i][1], w, degrees, name);
                    }
                }
            }
            "linear" => {
                let (centre, j) = m
                    .linear_at(pt(&c["p"]), c["h"].as_f64().expect("h"))
                    .expect("reached");
                near(centre, &c["centre"], degrees, name);
                for (g, w) in j.iter().zip(c["j"].as_array().expect("j")) {
                    let w = w.as_f64().expect("a number");
                    assert!((g - w).abs() < 1e-6 * w.abs() + 1e-12, "{name}: {g} ≠ {w}");
                }
            }
            "box" => {
                let b: Vec<f64> = c["box"]
                    .as_array()
                    .expect("box")
                    .iter()
                    .map(|v| v.as_f64().expect("n"))
                    .collect();
                let got = m.box_back([b[0], b[1], b[2], b[3]]).expect("reached");
                for (g, w) in got.iter().zip(c["back"].as_array().expect("back")) {
                    let w = w.as_f64().expect("a number");
                    assert!((g - w).abs() < 1e-3, "{name}: {got:?}");
                }
            }
            k => panic!("{name}: an unknown kind {k}"),
        }
    }
}

/// The meshes' times (docs/adr/0630 Doğrulama), on Web Mercator and WGS 84:
///
/// ```text
/// cargo test --release -p kentos-project --lib map_warp_timing -- --ignored --nocapture
/// ```
#[test]
#[ignore = "a timing, run on purpose"]
fn map_warp_timing() {
    use std::time::Instant;
    let (e, n) = (487_200.0, 4_420_600.0);
    for srid in [3857, 4326] {
        let m = map(5256, srid);
        let degrees = srid == 4326;
        let px = if degrees { 0.5 / 111_320.0 } else { 0.5 };
        // A view's tiles: 40 photograph tiles of 256 m at half a pixel (a metre a pixel).
        let t = Instant::now();
        let mut nodes = 0;
        for k in 0..40 {
            let (x, y) = (e + f64::from(k % 8) * 256.0, n + f64::from(k / 8) * 256.0);
            let corners = [
                Vec2::new(x, y + 256.0),
                Vec2::new(x + 256.0, y + 256.0),
                Vec2::new(x + 256.0, y),
                Vec2::new(x, y),
            ];
            let cells = m.grid_cells(corners, px, 32);
            nodes += m.grid(corners, cells).expect("reached").len();
        }
        println!(
            "EPSG:{srid}: 40 karonun ağı {:.2} ms ({nodes} düğüm)",
            t.elapsed().as_secs_f64() * 1e3
        );
        // A scanned sheet of 6 × 4 km at the map's own tolerance.
        let r = [
            Vec2::new(e, n),
            Vec2::new(e + 6_000.0, n),
            Vec2::new(e + 6_000.0, n + 4_000.0),
            Vec2::new(e, n + 4_000.0),
        ];
        let tris = [[r[0], r[1], r[2]], [r[0], r[2], r[3]]];
        let t = Instant::now();
        let level = m.mesh_level(&tris, m.tolerance, 8);
        let mut out = Vec::new();
        for tri in tris {
            m.mesh(tri, level, &mut out);
        }
        println!(
            "EPSG:{srid}: 6 × 4 km paftanın ağı {:.2} ms ({} üçgen)",
            t.elapsed().as_secs_f64() * 1e3,
            out.len() / 3
        );
        // A 2 km root node of 100 000 points: its grid and every point through its cell.
        let t = Instant::now();
        let half = 1_000.0;
        let corners = [
            Vec2::new(e - half, n + half),
            Vec2::new(e + half, n + half),
            Vec2::new(e + half, n - half),
            Vec2::new(e - half, n - half),
        ];
        let k = m.grid_cells(corners, m.tolerance / 10.0, 64);
        let g = m.grid(corners, k).expect("reached");
        let mut sum = 0.0;
        for i in 0..100_000u32 {
            let (fu, fv) = (
                f64::from(i % 317) / 317.0 * k as f64,
                f64::from(i % 251) / 251.0 * k as f64,
            );
            let (a, b) = ((fu as usize).min(k - 1), (fv as usize).min(k - 1));
            let (u, v) = (fu - a as f64, fv - b as f64);
            let at = |x: usize, y: usize| g[y * (k + 1) + x];
            let top = at(a, b).x + (at(a + 1, b).x - at(a, b).x) * u;
            let low = at(a, b + 1).x + (at(a + 1, b + 1).x - at(a, b + 1).x) * u;
            sum += top + (low - top) * v;
        }
        println!(
            "EPSG:{srid}: 2 km düğüm, 100 000 nokta {:.2} ms ({k} × {k} hücre; {})",
            t.elapsed().as_secs_f64() * 1e3,
            sum.is_finite()
        );
        let t = Instant::now();
        let c = m.forward(Vec2::new(e, n)).expect("on the map");
        let d = if degrees { 0.02 } else { 2_000.0 };
        let b = m.box_back([c.x - d, c.y - d, c.x + d, c.y + d]);
        println!(
            "EPSG:{srid}: görünümün kutusu geri {:.3} ms ({})",
            t.elapsed().as_secs_f64() * 1e3,
            b.is_some()
        );
    }
}
