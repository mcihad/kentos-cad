//! The unit applied once (plan/map-cad.md MAP-02, docs/adr/2100): DXF files in millimetres, centimetres, inches, feet
//! and metres read into a metre project put every point, height, radius and text size where ezdxf's own block
//! decomposition puts it, times the unit's length (`scripts/fixtures/dxf_units_once_cases.py`, no KentOS code). A
//! unit taken twice — a block's points, a nested insert's offset, a height or a text's size scaled again — would put a
//! value off by the unit's factor. Read flat (blocks exploded) every object is compared with its heights. Read with
//! its blocks, a block whose objects carry heights inserted raised or scaled in Z is opened, its heights where the file
//! puts them (a kept insert has no height); a flat symbol raised to a point's height stays a block, its insert's Z said
//! in the report.
use kentos_contracts::{BlockDefinition, DrawingUnit, DxfReadOptions, Entity, ImportResult, Vec2};
use kentos_formats::dxf;
use serde_json::Value;

const NEAR: f64 = 1e-9;

fn cases() -> (std::path::PathBuf, Value) {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/formats/v1/units-once");
    let all: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("cases.json")).unwrap()).unwrap();
    (dir, all)
}

fn read(dir: &std::path::Path, file: &str, explode: bool) -> ImportResult {
    let bytes = std::fs::read(dir.join(file)).unwrap();
    let options = DxfReadOptions { unit: Some(DrawingUnit::M), explode_blocks: explode, ..Default::default() };
    dxf::read_checked(&bytes, &options).unwrap()
}

/// An object as the fixture writes it: kind, layer, points (x, y, z: none where KentOS keeps no height), size.
#[derive(Debug, Clone, PartialEq)]
struct Got {
    kind: &'static str,
    layer: String,
    points: Vec<[Option<f64>; 3]>,
    size: Option<f64>,
}

fn got(e: &Entity) -> Option<Got> {
    let at = |p: Vec2, z: Option<f64>| [Some(p.x), Some(p.y), z];
    let layer = e.base().layer_id.clone();
    Some(match e {
        Entity::Line(l) => Got { kind: "line", layer, points: vec![at(l.a, l.za), at(l.b, l.zb)], size: None },
        Entity::Point(p) => Got { kind: "point", layer, points: vec![at(p.p, p.z)], size: None },
        Entity::Polyline(p) => Got {
            kind: "polyline",
            layer,
            points: p.pts.iter().enumerate().map(|(i, q)| at(*q, p.zs.as_ref().and_then(|z| z[i]))).collect(),
            size: None,
        },
        Entity::Circle(c) => Got { kind: "circle", layer, points: vec![at(c.c, None)], size: Some(c.r) },
        Entity::Arc(a) => Got { kind: "arc", layer, points: vec![at(a.c, None)], size: Some(a.r) },
        Entity::Text(t) => Got { kind: "text", layer, points: vec![at(t.p, None)], size: Some(t.height) },
        _ => return None,
    })
}

/// Whether `g` is the fixture's object `w`: same kind (an MTEXT is a text), layer, plane points and size within a
/// nanometre, and every height KentOS keeps within a nanometre too (the ones it does not keep are not compared:
/// a circle's, an arc's, a text's).
fn same(g: &Got, w: &Value) -> bool {
    let kind = match w["kind"].as_str().unwrap() {
        "mtext" => "text",
        k => k,
    };
    if g.kind != kind || g.layer != w["layer"].as_str().unwrap() {
        return false;
    }
    let want = w["points"].as_array().unwrap();
    if want.len() != g.points.len() {
        return false;
    }
    let close = |a: f64, b: &Value| (a - b.as_f64().unwrap()).abs() < NEAR;
    let points = g.points.iter().zip(want).all(|(p, q)| {
        close(p[0].unwrap(), &q[0]) && close(p[1].unwrap(), &q[1]) && p[2].is_none_or(|z| close(z, &q[2]))
    });
    let size = match (g.size, w.get("radius").or(w.get("height"))) {
        (Some(s), Some(v)) => close(s, v),
        (None, None) => true,
        _ => false,
    };
    points && size
}

#[test]
fn read_flat_every_point_height_and_size_is_ezdxfs_times_the_unit() {
    let (dir, all) = cases();
    for case in all["cases"].as_array().unwrap() {
        let file = case["file"].as_str().unwrap();
        let read = read(&dir, file, true);
        let got: Vec<Got> = read.entities.iter().filter_map(got).collect();
        let want = case["objects"].as_array().unwrap();
        assert_eq!(got.len(), want.len(), "{file}: {got:#?}");
        for w in want {
            assert!(got.iter().any(|g| same(g, w)), "{file}: nothing like {w} in {got:#?}");
        }
        // The heights KentOS keeps (lines, points, polylines) are all there: none dropped, none scaled twice.
        let heights = got.iter().filter(|g| matches!(g.kind, "line" | "point" | "polyline"));
        assert_eq!(heights.clone().count(), 6, "{file}");
        assert!(heights.flat_map(|g| g.points.iter()).all(|p| p[2].is_some()), "{file}: a height dropped");
    }
}

/// An insert's objects placed in the plane, its nested inserts too: base off, scaled, turned, moved.
fn placed(e: &Entity, blocks: &[BlockDefinition]) -> Vec<Got> {
    match e {
        Entity::Insert(i) => {
            let b = blocks.iter().find(|b| b.id == i.block).unwrap();
            let (s, c) = i.rotation.sin_cos(); // radians, as the contract keeps it
            let sy = i.y_scale.unwrap_or(i.scale);
            let place = |p: Vec2| {
                let (x, y) = ((p.x - b.base.x) * i.scale, (p.y - b.base.y) * sy);
                Vec2 { x: i.p.x + x * c - y * s, y: i.p.y + x * s + y * c }
            };
            b.entities
                .iter()
                .flat_map(|e| placed(e, blocks))
                .map(|mut g| {
                    for p in &mut g.points {
                        let q = place(Vec2 { x: p[0].unwrap(), y: p[1].unwrap() });
                        // Heights are not placed by a plane insert: left out of this comparison.
                        *p = [Some(q.x), Some(q.y), None];
                    }
                    g.size = g.size.map(|v| v * i.scale.abs());
                    g
                })
                .collect()
        }
        e => got(e).into_iter().collect(),
    }
}

#[test]
fn read_with_its_blocks_the_inserts_place_the_blocks_once() {
    let (dir, all) = cases();
    for case in all["cases"].as_array().unwrap() {
        let file = case["file"].as_str().unwrap();
        let read = read(&dir, file, false);
        let got: Vec<Got> = read.entities.iter().flat_map(|e| placed(e, &read.blocks)).collect();
        let want = case["objects"].as_array().unwrap();
        assert_eq!(got.len(), want.len(), "{file}: {got:#?}");
        for w in want {
            assert!(got.iter().any(|g| same(g, w)), "{file}: nothing like {w} in {got:#?}");
        }
        // OUTER (raised, scaled) carries heights, its own and INNER's: opened, its lines with their heights. The flat
        // symbol stays the one insert.
        let inserts: Vec<&str> = read
            .entities
            .iter()
            .filter_map(|e| match e {
                Entity::Insert(i) => read.blocks.iter().find(|b| b.id == i.block).map(|b| b.name.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(inserts, ["SEMBOL"], "{file}");
        let blok: Vec<&Got> = got.iter().filter(|g| g.layer == "BLOK" && g.kind == "line").collect();
        assert_eq!(blok.len(), 2, "{file}");
        assert!(blok.iter().all(|g| g.points.iter().all(|p| p[2].is_some())), "{file}: {blok:?}");
        let said = |words: &str| read.report.notes.iter().any(|n| n.what == "Blok (INSERT)" && n.reason.starts_with(words));
        assert!(said("kotlu bloğun yükseltilmiş ya da Z'de ölçeklenmiş yerleştirmesi patlatılarak alındı"), "{file}: {:?}", read.report.notes);
        assert!(said("yerleştirmenin yüksekliği (Z) alınmadı"), "{file}: {:?}", read.report.notes);
    }
}

