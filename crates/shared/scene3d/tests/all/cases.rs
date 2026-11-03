//! Every case of fixtures/scene3d/v1/cases.json: the camera's matrix, frustum, pixels, rays and offsets; the
//! selection's tiles drawn and asked for; the chunking's tiles and contents, bit for bit where they are float32.

use kentos_scene3d::{
    Aabb, Camera, ContentRef, Exaggeration, Hierarchy, Limits, Projection, Refine, Settings, SourceMesh, Tile, TileKey,
    Viewport, Visibility, chunk, select,
};
use serde_json::Value;

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../../../fixtures/scene3d/v1/cases.json")).expect("the cases read")
}

fn f(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn v3(v: &Value) -> [f64; 3] {
    [f(&v[0]), f(&v[1]), f(&v[2])]
}

fn close(a: f64, b: f64, what: &str) {
    let tol = 1e-9 * a.abs().max(b.abs()).max(1.0);
    assert!((a - b).abs() <= tol, "{what}: {a} ≠ {b}");
}

fn camera(v: &Value) -> Camera {
    let p = &v["projection"];
    let projection = if p["kind"] == "perspective" {
        Projection::Perspective { fov_y: f(&p["fovY"]), near: f(&p["near"]) }
    } else {
        Projection::Orthographic { height: f(&p["height"]), depth: f(&p["depth"]) }
    };
    Camera { eye: v3(&v["eye"]), target: v3(&v["target"]), up: v3(&v["up"]), projection }
}

fn viewport(v: &Value) -> Viewport {
    Viewport { width: f(&v["width"]), height: f(&v["height"]) }
}

fn exaggeration(v: &Value) -> Exaggeration {
    Exaggeration { factor: f(&v["factor"]), datum: f(&v["datum"]) }
}

fn aabb(v: &Value) -> Aabb {
    Aabb { min: v3(&v[0]), max: v3(&v[1]) }
}

#[test]
fn the_camera_cases() {
    let all = cases();
    for c in all["cameras"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let cam = camera(&c["camera"]);
        let vp = viewport(&c["viewport"]);
        let ex = exaggeration(&c["exaggeration"]);
        let m = cam.rotation_projection(vp.aspect());
        for (k, want) in c["matrix"].as_array().unwrap().iter().enumerate() {
            close(m.0[k], f(want), &format!("{name}: matrix[{k}]"));
        }
        let frustum = cam.frustum(vp.aspect());
        for b in c["boxes"].as_array().unwrap() {
            let got = match frustum.test(&aabb(&b["box"])) {
                Visibility::Outside => "outside",
                Visibility::Intersecting => "intersecting",
                Visibility::Inside => "inside",
            };
            assert_eq!(got, b["visibility"], "{name}: {}", b["box"]);
        }
        for p in c["points"].as_array().unwrap() {
            let got = cam.project(v3(&p["point"]), &vp);
            match (&got, p["pixel"].as_array()) {
                (None, None) => {}
                (Some((x, y, z)), Some(w)) => {
                    close(*x, f(&w[0]), &format!("{name}: x"));
                    close(*y, f(&w[1]), &format!("{name}: y"));
                    close(*z, f(&w[2]), &format!("{name}: depth"));
                }
                _ => panic!("{name}: {} → {got:?}, not {}", p["point"], p["pixel"]),
            }
        }
        for r in c["rays"].as_array().unwrap() {
            let (o, d) = cam.ray(f(&r["pixel"][0]), f(&r["pixel"][1]), &vp);
            for a in 0..3 {
                close(o[a], f(&r["origin"][a]), &format!("{name}: ray origin"));
                close(d[a], f(&r["direction"][a]), &format!("{name}: ray direction"));
            }
        }
        for o in c["offsets"].as_array().unwrap() {
            let got = cam.offset(v3(&o["origin"]), &ex);
            for a in 0..3 {
                assert_eq!(f64::from(got[a]), f(&o["offset"][a]), "{name}: offset {}", o["origin"]);
            }
        }
    }
}

fn hierarchy(v: &Value) -> Hierarchy {
    let tiles = v["tiles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| Tile {
            key: TileKey(t["key"].as_u64().unwrap()),
            bounds: aabb(&t["bounds"]),
            geometric_error: f(&t["geometricError"]),
            refine: if t["refine"] == "add" { Refine::Add } else { Refine::Replace },
            content: t["content"].as_object().map(|c| ContentRef { id: c["id"].as_u64().unwrap(), bytes: c["bytes"].as_u64().unwrap() }),
            children: t["children"].as_array().unwrap().iter().map(|c| c.as_u64().unwrap() as u32).collect(),
        })
        .collect();
    let roots = v["roots"].as_array().unwrap().iter().map(|c| c.as_u64().unwrap() as u32).collect();
    Hierarchy { tiles, roots }
}

fn indices(v: &Value) -> Vec<u32> {
    v.as_array().unwrap().iter().map(|c| c.as_u64().unwrap() as u32).collect()
}

#[test]
fn the_selection_cases() {
    let all = cases();
    for s in all["selections"].as_array().unwrap() {
        let name = s["name"].as_str().unwrap();
        let h = hierarchy(&s["hierarchy"]);
        assert_eq!(h.problem(), None, "{name}");
        let ready: Vec<u64> = s["ready"].as_array().unwrap().iter().map(|c| c.as_u64().unwrap()).collect();
        let got = select(
            &h,
            &camera(&s["camera"]),
            &viewport(&s["viewport"]),
            &exaggeration(&s["exaggeration"]),
            Settings { max_screen_error: f(&s["maxScreenError"]) },
            |c| ready.contains(&c.id),
        );
        let want = &s["expect"];
        assert_eq!(got.draw, indices(&want["draw"]), "{name}: drawn");
        assert_eq!(got.wanted, indices(&want["wanted"]), "{name}: asked");
        assert_eq!(got.visited as u64, want["visited"].as_u64().unwrap(), "{name}: visited");
        assert_eq!(got.culled as u64, want["culled"].as_u64().unwrap(), "{name}: culled");
    }
}

/// A mesh read from a case: its part, vertices and triangles.
type Owned = (u32, Vec<[f64; 3]>, Vec<[u32; 3]>);

#[test]
fn the_chunk_cases() {
    let all = cases();
    for c in all["chunks"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let owned: Vec<Owned> = c["meshes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| {
                let vs = m["vertices"].as_array().unwrap().iter().map(v3).collect();
                let ts = m["triangles"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|t| [0, 1, 2].map(|k| t[k].as_u64().unwrap() as u32))
                    .collect();
                (m["part"].as_u64().unwrap() as u32, vs, ts)
            })
            .collect();
        let meshes: Vec<SourceMesh> =
            owned.iter().map(|(part, vs, ts)| SourceMesh { part: *part, vertices: vs, triangles: ts }).collect();
        let limits = Limits {
            max_triangles: c["limits"]["maxTriangles"].as_u64().unwrap() as usize,
            max_extent: f(&c["limits"]["maxExtent"]),
        };
        let got = chunk(&meshes, limits, c["firstId"].as_u64().unwrap());
        let want_h = hierarchy(&c["expect"]["hierarchy"]);
        assert_eq!(got.hierarchy.problem(), None, "{name}");
        assert_eq!(got.hierarchy.roots, want_h.roots, "{name}: roots");
        assert_eq!(got.hierarchy.tiles.len(), want_h.tiles.len(), "{name}: tiles");
        for (g, w) in got.hierarchy.tiles.iter().zip(&want_h.tiles) {
            assert_eq!((g.key, &g.children, g.content, g.refine), (w.key, &w.children, w.content, w.refine), "{name}");
            for a in 0..3 {
                close(g.bounds.min[a], w.bounds.min[a], name);
                close(g.bounds.max[a], w.bounds.max[a], name);
            }
            close(g.geometric_error, w.geometric_error, name);
        }
        let want_c = c["expect"]["contents"].as_array().unwrap();
        assert_eq!(got.contents.len(), want_c.len(), "{name}: contents");
        for (g, w) in got.contents.iter().zip(want_c) {
            assert_eq!(g.id, w["id"].as_u64().unwrap(), "{name}");
            for a in 0..3 {
                close(g.origin[a], f(&w["origin"][a]), name);
            }
            let positions: Vec<[f64; 3]> = g.positions.iter().map(|p| p.map(f64::from)).collect();
            let want_p: Vec<[f64; 3]> = w["positions"].as_array().unwrap().iter().map(v3).collect();
            assert_eq!(positions, want_p, "{name}: positions");
            assert_eq!(g.parts, indices(&w["parts"]), "{name}: parts");
            assert_eq!(g.indices, indices(&w["indices"]), "{name}: indices");
            let ranges: Vec<[u32; 3]> = g.ranges.iter().map(|(p, r)| [*p, r.start, r.end]).collect();
            let want_r: Vec<[u32; 3]> =
                w["ranges"].as_array().unwrap().iter().map(|r| [0, 1, 2].map(|k| r[k].as_u64().unwrap() as u32)).collect();
            assert_eq!(ranges, want_r, "{name}: ranges");
            let sources: Vec<[u32; 2]> = g.sources.iter().map(|(a, b)| [*a, *b]).collect();
            let want_s: Vec<[u32; 2]> =
                w["sources"].as_array().unwrap().iter().map(|r| [r[0].as_u64().unwrap() as u32, r[1].as_u64().unwrap() as u32]).collect();
            assert_eq!(sources, want_s, "{name}: sources");
            assert_eq!(g.encoding_error, f(&w["encodingError"]), "{name}: encoding error");
        }
    }
}
