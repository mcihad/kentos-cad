//! The operations' machines against the independent reference
//! (fixtures/pointcloud/v1/ops.json, scripts/fixtures/pointcloud_ops_cases.py):
//! the same files read with the core's reader, every answer compared
//! exactly (counts, kept numbers, classes, raster values and records by
//! their FNV-1a 64; the figures as numbers, bit for bit).

use std::collections::BTreeMap;

use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::vec2::Vec2;
use kentos_pointcloud::ops::boundary::parts;
use kentos_pointcloud::ops::convert::{Converter, Input, plan};
use kentos_pointcloud::ops::ground::{Ground, Params as GroundParams};
use kentos_pointcloud::ops::height::{Height, Params as HeightParams};
use kentos_pointcloud::ops::raster::{Frame, Occupancy, Rasterize, Value};
use kentos_pointcloud::ops::region::Region;
use kentos_pointcloud::ops::stats::AreaQuery;
use kentos_pointcloud::ops::thin::{Cells, Radius, nth};
use kentos_pointcloud::ops::tile::tile_of;
use kentos_pointcloud::record::Layout;
use kentos_pointcloud::source::Cloud;
use serde_json::Value as Json;

use crate::host::{fixtures, fnv, open, records};

fn ops() -> Json {
    let text = std::fs::read_to_string(fixtures().join("ops.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

struct Read {
    cloud: Cloud,
    recs: Vec<u8>,
}

impl Read {
    fn of(rel: &str) -> Read {
        let bytes = std::fs::read(fixtures().join(rel)).unwrap();
        let cloud = open(&bytes).unwrap();
        let recs = records(&cloud, &bytes);
        Read { cloud, recs }
    }

    fn layout(&self) -> Layout {
        self.cloud.layout
    }

    fn plan(&self) -> [f64; 4] {
        let h = &self.cloud.head;
        [h.min[0], h.min[1], h.max[0], h.max[1]]
    }

    fn n(&self) -> usize {
        self.recs.len() / self.layout().len
    }

    fn scale(&self) -> [f64; 3] {
        self.cloud.head.scale
    }

    fn offset(&self) -> [f64; 3] {
        self.cloud.head.offset
    }

    fn input(&self) -> Input {
        Input {
            head: self.cloud.head.clone(),
            layout: self.cloud.layout,
            vlrs: self.cloud.vlrs.clone(),
            evlrs: self.cloud.evlrs.clone(),
        }
    }
}

fn koy() -> Read {
    Read::of("../../interaction/v1/pointclouds/koy.laz")
}

fn u64s(list: &[u64]) -> String {
    let mut b = Vec::with_capacity(list.len() * 8);
    for v in list {
        b.extend_from_slice(&v.to_le_bytes());
    }
    fnv(&b)
}

fn classes(r: &Read, recs: &[u8]) -> String {
    let layout = r.layout();
    let c: Vec<u8> = recs
        .chunks_exact(layout.len)
        .map(|x| layout.class(x))
        .collect();
    fnv(&c)
}

#[test]
fn seyrelt_keeps_the_references_points() {
    let o = ops();
    let c = &o["cases"];
    let k = koy();
    let (l, s, off) = (k.layout(), k.scale(), k.offset());
    let mut cells = Cells::new(1.0).unwrap();
    cells.feed(&l, &k.recs, s, off, 0);
    let kept = cells.kept();
    assert_eq!(kept.len() as u64, c["thinCell"]["count"].as_u64().unwrap());
    assert_eq!(u64s(&kept), c["thinCell"]["kept"]);
    let mut radius = Radius::new(0.6).unwrap();
    let mut mask = Vec::new();
    // In two pieces: the machine keeps what it kept across them.
    let half = (k.n() / 2) * l.len;
    let mut kept = Vec::new();
    for (start, piece) in [(0, &k.recs[..half]), (half / l.len, &k.recs[half..])] {
        radius.feed(&l, piece, s, off, &mut mask);
        kept.extend(
            mask.iter()
                .enumerate()
                .filter(|(_, m)| **m)
                .map(|(i, _)| (start + i) as u64),
        );
    }
    assert_eq!(
        kept.len() as u64,
        c["thinRadius"]["count"].as_u64().unwrap()
    );
    assert_eq!(u64s(&kept), c["thinRadius"]["kept"]);
    let kept: Vec<u64> = (0..k.n() as u64).filter(|&i| nth(i, 7)).collect();
    assert_eq!(kept.len() as u64, c["thinNth"]["count"].as_u64().unwrap());
    assert_eq!(u64s(&kept), c["thinNth"]["kept"]);
}

#[test]
fn zemin_suzgeci_and_yukseklik_are_the_references() {
    let o = ops();
    let c = &o["cases"];
    let k = koy();
    let (l, s, off) = (k.layout(), k.scale(), k.offset());
    let mut g = Ground::new(k.plan(), GroundParams::default()).unwrap();
    g.feed(&l, &k.recs, s, off);
    assert!(g.surface());
    let mut recs = k.recs.clone();
    let n = g.classify(&l, &mut recs, s, off);
    assert_eq!(n, c["ground"]["ground"].as_u64().unwrap());
    assert_eq!(classes(&k, &recs), c["ground"]["classes"]);
    let mut h = Height::new(
        k.plan(),
        HeightParams {
            all: true,
            ..HeightParams::default()
        },
    )
    .unwrap();
    h.feed(&l, &k.recs, s, off);
    assert!(h.surface());
    let mut recs = k.recs.clone();
    let n = h.classify(&l, &mut recs, s, off);
    let want: Vec<u64> = c["height"]["classed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect();
    assert_eq!(n.to_vec(), want);
    assert_eq!(classes(&k, &recs), c["height"]["classes"]);
}

/// The reference's shapes as areas: a rectangle, a disc, a rectangle with a hole.
fn shapes(o: &Json) -> (Area, Area, Area) {
    let sh = &o["shapes"];
    let origin = sh["origin"].as_array().unwrap();
    let (x0, y0) = (origin[0].as_f64().unwrap(), origin[1].as_f64().unwrap());
    let nums = |v: &Json| -> Vec<f64> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect()
    };
    let rect = |r: &[f64]| Ring {
        pts: vec![
            Vec2::new(x0 + r[0], y0 + r[1]),
            Vec2::new(x0 + r[2], y0 + r[1]),
            Vec2::new(x0 + r[2], y0 + r[3]),
            Vec2::new(x0 + r[0], y0 + r[3]),
        ],
        bulges: None,
    };
    let r = nums(&sh["rect"]);
    let d = nums(&sh["disc"]);
    let holed = sh["holed"].as_array().unwrap();
    (
        Area {
            outer: rect(&r),
            holes: vec![],
        },
        Area {
            outer: Ring {
                pts: vec![
                    Vec2::new(x0 + d[0] + d[2], y0 + d[1]),
                    Vec2::new(x0 + d[0] - d[2], y0 + d[1]),
                ],
                bulges: Some(vec![1.0, 1.0]),
            },
            holes: vec![],
        },
        Area {
            outer: rect(&nums(&holed[0])),
            holes: vec![rect(&nums(&holed[1]))],
        },
    )
}

#[test]
fn kirp_and_alan_sorgusu_are_the_references() {
    let o = ops();
    let c = &o["cases"];
    let k = koy();
    let (l, s, off) = (k.layout(), k.scale(), k.offset());
    let (a, b, h) = shapes(&o);
    let region = Region::new(&[a.clone(), b.clone(), h.clone()]).unwrap();
    let index = region.index();
    let kept: Vec<u64> = k
        .recs
        .chunks_exact(l.len)
        .enumerate()
        .filter(|(_, r)| {
            let p = Vec2::new(
                f64::from(l.x(r)) * s[0] + off[0],
                f64::from(l.y(r)) * s[1] + off[1],
            );
            region.contains(&index, p)
        })
        .map(|(i, _)| i as u64)
        .collect();
    assert_eq!(kept.len() as u64, c["clip"]["count"].as_u64().unwrap());
    assert_eq!(u64s(&kept), c["clip"]["kept"]);
    let mut q = AreaQuery::new(vec![
        Region::new(&[a]).unwrap(),
        Region::new(&[b]).unwrap(),
        Region::new(&[h]).unwrap(),
    ]);
    q.file(s, off);
    q.feed(&l, &k.recs, s, off);
    for (row, want) in q.rows().iter().zip(c["areaStats"].as_array().unwrap()) {
        assert_eq!(row.count, want["count"].as_u64().unwrap());
        assert_eq!(row.z_min, want["zMin"].as_f64());
        assert_eq!(row.z_max, want["zMax"].as_f64());
        assert_eq!(
            row.z_mean.map(f64::to_bits),
            want["zMean"].as_f64().map(f64::to_bits)
        );
        assert_eq!(
            row.z_std.map(f64::to_bits),
            want["zStd"].as_f64().map(f64::to_bits)
        );
        let classes: Vec<(u8, u64)> = want["classes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| (p[0].as_u64().unwrap() as u8, p[1].as_u64().unwrap()))
            .collect();
        assert_eq!(row.classes, classes);
    }
}

#[test]
fn karola_rasterlestir_and_sinir_are_the_references() {
    let o = ops();
    let c = &o["cases"];
    let k = koy();
    let (l, s, off) = (k.layout(), k.scale(), k.offset());
    let mut tiles: BTreeMap<(i64, i64), u64> = BTreeMap::new();
    for r in k.recs.chunks_exact(l.len) {
        let x = f64::from(l.x(r)) * s[0] + off[0];
        let y = f64::from(l.y(r)) * s[1] + off[1];
        *tiles.entry(tile_of(x, y, 50.0)).or_default() += 1;
    }
    let want: Vec<(i64, i64, u64)> = c["tile"]["tiles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            (
                t[0].as_i64().unwrap(),
                t[1].as_i64().unwrap(),
                t[2].as_u64().unwrap(),
            )
        })
        .collect();
    let got: Vec<(i64, i64, u64)> = tiles.into_iter().map(|((x, y), n)| (x, y, n)).collect();
    assert_eq!(got, want);
    for (key, value) in [
        ("min", Value::Min),
        ("max", Value::Max),
        ("mean", Value::Mean),
        ("count", Value::Count),
        ("idw", Value::Idw(2.5)),
    ] {
        let frame = Frame::over(k.plan(), 2.0).unwrap();
        let mut g = Rasterize::new(frame, value, None);
        g.feed(&l, &k.recs, s, off);
        let values = g.values();
        let want = &c["rasterize"][key];
        assert_eq!(frame.cols as u64, want["cols"].as_u64().unwrap(), "{key}");
        assert_eq!(frame.rows as u64, want["rows"].as_u64().unwrap(), "{key}");
        let bits: Vec<u8> = values
            .iter()
            .flat_map(|v| (*v as f32).to_le_bytes())
            .collect();
        assert_eq!(fnv(&bits), want["values"], "{key}");
    }
    let sinir = Read::of("ops/sinir.laz");
    let frame = Frame::over(sinir.plan(), 1.0).unwrap();
    let mut occ = Occupancy::new(frame);
    occ.feed(&sinir.layout(), &sinir.recs, sinir.scale(), sinir.offset());
    let found = parts(&frame, &occ.filled(2));
    let want = c["boundary"]["parts"].as_array().unwrap();
    assert_eq!(found.len(), want.len());
    let pts = |v: &Json| -> Vec<[f64; 2]> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|p| [p[0].as_f64().unwrap(), p[1].as_f64().unwrap()])
            .collect()
    };
    for (p, w) in found.iter().zip(want) {
        assert_eq!(p.outer, pts(&w["outer"]));
        let holes: Vec<Vec<[f64; 2]>> = w["holes"].as_array().unwrap().iter().map(pts).collect();
        assert_eq!(p.holes, holes);
    }
}

#[test]
fn birlestir_moves_the_records_as_the_reference() {
    let o = ops();
    let c = &o["cases"]["merge"];
    let (a, b) = (koy(), Read::of("ops/ikinci.laz"));
    let inputs = [a.input(), b.input()];
    let p = plan(&inputs, true).unwrap();
    let want_scale: Vec<f64> = c["scale"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    assert_eq!(p.spec.scale.to_vec(), want_scale);
    assert_eq!(p.spec.format, 7);
    let mut out = Vec::new();
    let mut rounded = 0;
    for (r, input) in [&a, &b].into_iter().zip(&inputs) {
        rounded += Converter::new(input, &p.spec)
            .convert(&r.recs, &mut out)
            .unwrap();
    }
    assert_eq!(rounded, c["rounded"].as_u64().unwrap());
    assert_eq!((out.len() / 36) as u64, c["count"].as_u64().unwrap());
    assert_eq!(fnv(&out), c["records"]);
}
