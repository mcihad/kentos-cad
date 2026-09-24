//! The DXF reader on the files in `fixtures/formats/v1/` (each built to
//! test one feature; the WASM test `src/io/formats.wasm.test.ts` reads the
//! same files): every entity kind, object coordinate systems, nested and
//! array inserts, hatches with islands, dimensions, and files that are not
//! DXF at all. Expected values are worked out by hand from the file.

use kentos_contracts::{DxfReadOptions, Entity, HatchPatternType, ImportResult, LineType, Vec2};
use kentos_formats::dxf;
use kentos_formats::math::{cos, sin};

const PI: f64 = std::f64::consts::PI;

fn fixture(name: &str) -> Vec<u8> {
    let path = format!("{}/../../fixtures/formats/v1/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn read(name: &str) -> ImportResult {
    dxf::read(&fixture(name), &DxfReadOptions::default()).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn near(a: Vec2, b: Vec2) -> bool {
    (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9
}

fn v(x: f64, y: f64) -> Vec2 {
    Vec2 { x, y }
}

fn count(r: &ImportResult, kind: &str) -> u32 {
    r.report.counts.get(kind).copied().unwrap_or(0)
}

fn skipped(r: &ImportResult, what: &str) -> Option<String> {
    r.report.skipped.iter().find(|s| s.what == what).map(|s| s.reason.clone())
}

#[test]
fn every_kind_with_layers_colours_and_the_turkish_code_page() {
    let r = read("entities.dxf");
    let facts: Vec<(&str, &str)> = r.report.source.iter().map(|f| (f.label.as_str(), f.value.as_str())).collect();
    assert!(facts.contains(&("Sürüm", "AutoCAD 2000 (AC1015)")), "{facts:?}");
    assert!(facts.contains(&("Karakter kodlaması", "Windows-1254 (Türkçe)")));
    assert!(facts.contains(&("Birim ($INSUNITS)", "metre")));

    let layer = |n: &str| r.layers.iter().find(|l| l.name == n).unwrap_or_else(|| panic!("layer {n}"));
    assert_eq!((layer("PARSEL").color.as_str(), layer("PARSEL").line_weight), ("#FF0000", Some(0.35)));
    let road = layer("Yol ekseni");
    assert_eq!((road.color.as_str(), road.visible, road.line_type), ("#0000FF", false, LineType::Dashed));
    assert!(layer("Kot").locked);
    // The Turkish name came through Windows-1254; a frozen layer is hidden.
    assert!(!layer("Yazı").visible);
    assert_eq!(layer("0").color, "ink");

    let e = &r.entities;
    let Entity::Line(l) = &e[0] else { panic!("{:?}", e[0]) };
    // Coordinates bit for bit.
    assert_eq!((l.a, l.b), (v(452000.125, 4412000.5), v(452010.25, 4412005.75)));
    assert_eq!(l.base.layer_id, "PARSEL");
    let Entity::Polygon(p) = &e[1] else { panic!("{:?}", e[1]) };
    assert_eq!(p.pts.len(), 4);
    assert_eq!(p.bulges, Some(vec![0.0, 0.5, 0.0, 0.0]));
    let Entity::Polyline(p) = &e[2] else { panic!("{:?}", e[2]) };
    assert_eq!(p.bulges, Some(vec![-1.0, 0.0]));
    let Entity::Polyline(p) = &e[3] else { panic!("{:?}", e[3]) };
    assert_eq!((p.pts.clone(), p.bulges.clone()), (vec![v(0.0, 0.0), v(4.0, 0.0), v(4.0, 4.0)], Some(vec![1.0, 0.0])));
    let Entity::Circle(c) = &e[4] else { panic!("{:?}", e[4]) };
    assert_eq!((c.c, c.r, c.base.color.as_deref()), (v(100.0, 200.0), 2.5, Some("#FF0000")));
    let Entity::Arc(a) = &e[5] else { panic!("{:?}", e[5]) };
    assert_eq!((a.a0, a.a1), (30.0 * PI / 180.0, 120.0 * PI / 180.0));
    let Entity::Ellipse(el) = &e[6] else { panic!("{:?}", e[6]) };
    assert_eq!((el.c, el.major, el.ratio, el.t0, el.t1), (v(50.0, 60.0), v(10.0, 0.0), 0.5, 0.0, PI));
    let Entity::Spline(s) = &e[7] else { panic!("{:?}", e[7]) };
    assert_eq!((s.pts.len(), s.closed), (4, false));
    let Entity::Polyline(p) = &e[8] else { panic!("{:?}", e[8]) };
    assert_eq!((p.pts.first(), p.pts.last()), (Some(&v(0.0, 0.0)), Some(&v(2.0, 0.0))));
    let Entity::Point(pt) = &e[9] else { panic!("{:?}", e[9]) };
    assert_eq!((pt.p, pt.z, pt.base.layer_id.as_str()), (v(452001.0, 4412001.0), Some(105.25), "Kot"));
    let Entity::Point(pt) = &e[10] else { panic!("{:?}", e[10]) };
    assert_eq!(pt.z, None);
    let Entity::Text(t) = &e[11] else { panic!("{:?}", e[11]) };
    assert_eq!((t.text.as_str(), t.p, t.height, t.rotation, t.base.color.as_deref()), ("Ada 101 Ø20 İş", v(452005.0, 4412005.0), 2.5, 30.0, Some("#00FF00")));
    let Entity::Text(t) = &e[12] else { panic!("{:?}", e[12]) };
    assert_eq!((t.text.as_str(), t.p), ("Birinci satır", v(10.0, 18.0)));
    let Entity::Text(t) = &e[13] else { panic!("{:?}", e[13]) };
    assert_eq!(t.text, "Ikinci satır");
    assert!(near(t.p, v(10.0, 18.0 - 2.0 * 5.0 / 3.0)), "{:?}", t.p);
    let Entity::Polygon(p) = &e[14] else { panic!("{:?}", e[14]) };
    assert_eq!(p.pts, vec![v(0.0, 0.0), v(1.0, 0.0), v(1.0, 1.0), v(0.0, 1.0)]);
    let Entity::Xline(x) = &e[15] else { panic!("{:?}", e[15]) };
    assert_eq!((x.p, x.dir), (v(5.0, 5.0), v(0.0, 1.0)));
    let Entity::Ray(x) = &e[16] else { panic!("{:?}", e[16]) };
    assert!(near(x.dir, v(0.6, 0.8)));
    assert_eq!(e.len(), 17);

    assert!(skipped(&r, "IMAGE").is_some_and(|s| s.contains("raster")));
    assert!(skipped(&r, "Kâğıt uzayı nesnesi").is_some());
    assert!(r.report.notes.iter().any(|n| n.what == "Genişlikli çoklu çizgi"));
    assert!(r.report.notes.iter().any(|n| n.what == "Denetim noktalı eğri (SPLINE)"));
    assert_eq!((count(&r, "text"), count(&r, "polygon"), count(&r, "polyline")), (3, 2, 3));
    let b = r.bounds.expect("extent");
    assert!(b.max_x >= 452020.0 && b.min_x <= 0.0);
}

#[test]
fn the_mirrored_object_coordinate_system() {
    let r = read("ocs.dxf");
    let e = &r.entities;
    // x → −x; a counter-clockwise arc turns clockwise, so its ends swap.
    let Entity::Arc(a) = &e[0] else { panic!("{:?}", e[0]) };
    assert_eq!((a.c, a.r), (v(-10.0, 5.0), 2.0));
    assert!((a.a0 - PI / 2.0).abs() < 1e-15 && (a.a1 - PI).abs() < 1e-15, "{a:?}");
    let Entity::Circle(c) = &e[1] else { panic!("{:?}", e[1]) };
    assert_eq!((c.c, c.r), (v(-10.0, 5.0), 3.0));
    let Entity::Polyline(p) = &e[2] else { panic!("{:?}", e[2]) };
    assert_eq!((p.pts.clone(), p.bulges.clone()), (vec![v(-1.0, 0.0), v(-3.0, 0.0)], Some(vec![-1.0])));
    let Entity::Text(t) = &e[3] else { panic!("{:?}", e[3]) };
    assert_eq!((t.p, t.rotation), (v(-4.0, 1.0), 0.0));
    // The ellipse's minor axis is normal × major: it points down, so the arc runs from (0, −1) to (2, 0).
    let Entity::Ellipse(el) = &e[4] else { panic!("{:?}", e[4]) };
    let at = |t: f64| {
        let m = v(-el.major.y * el.ratio, el.major.x * el.ratio);
        v(el.c.x + el.major.x * cos(t) + m.x * sin(t), el.c.y + el.major.y * cos(t) + m.y * sin(t))
    };
    assert!(near(at(el.t0), v(0.0, -1.0)) && near(at(el.t1), v(2.0, 0.0)), "{el:?}");
    assert!((el.ratio - 0.5).abs() < 1e-15);
}

#[test]
fn nested_and_array_inserts_with_attributes() {
    let r = read("blocks.dxf");
    let circles: Vec<(Vec2, f64, Option<String>, String)> = r
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Circle(c) => Some((c.c, c.r, c.base.color.clone(), c.base.layer_id.clone())),
            _ => None,
        })
        .collect();
    // KAPI at (1000, 2000) turned 90°, holding NO at (10, 0) turned 90° and scaled 2: the circle lands at (1000, 2010), r 1.
    assert_eq!(circles[0], (v(1000.0, 2010.0), 1.0, Some("ink".to_string()), "KAPILAR".to_string()));
    // The 2 × 2 array of NO, 10 apart in columns and 20 in rows.
    let array: Vec<Vec2> = circles[1..].iter().map(|c| c.0).collect();
    assert_eq!(array, vec![v(0.0, 0.0), v(10.0, 0.0), v(0.0, 20.0), v(10.0, 20.0)]);
    assert!(circles[1..].iter().all(|c| c.1 == 0.5 && c.3 == "DIZI"));
    let lines: Vec<(Vec2, Vec2, String)> = r
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Line(l) => Some((l.a, l.b, l.base.layer_id.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(lines[0], (v(1000.0, 2010.0), v(998.0, 2010.0), "DETAY".to_string()));
    assert_eq!(lines.len(), 5);
    let point = r.entities.iter().find_map(|e| match e {
        Entity::Point(p) => Some((p.p, p.z, p.base.layer_id.clone())),
        _ => None,
    });
    // (3, 4) turned 90° and moved; its elevation 1.5 sits on the insert's 100.
    assert_eq!(point, Some((v(996.0, 2003.0), Some(101.5), "KAPILAR".to_string())));
    let texts: Vec<&str> = r
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(texts, vec!["P-7"]);
    let Some(Entity::Ellipse(el)) = r.entities.iter().find(|e| matches!(e, Entity::Ellipse(_))) else { panic!("no ellipse") };
    assert_eq!((el.c, el.major, el.ratio, el.t0, el.t1), (v(50.0, 50.0), v(3.0, 0.0), 1.0 / 3.0, 0.0, 0.0));
    assert!(skipped(&r, "Blok (INSERT)").is_some());
    let reasons: Vec<&str> = r.report.skipped.iter().map(|s| s.reason.as_str()).collect();
    assert!(reasons.iter().any(|s| s.contains("“KENDI” bloğu kendini içeriyor")), "{reasons:?}");
    assert!(reasons.iter().any(|s| s.contains("“YOK” blok tanımı dosyada yok")));
    assert!(skipped(&r, "Görünmez öznitelik (ATTRIB)").is_some());
    // An ASCII-only file reads the same in any code page; the report names the declared one.
    let facts: Vec<(&str, &str)> = r.report.source.iter().map(|f| (f.label.as_str(), f.value.as_str())).collect();
    assert!(facts.contains(&("Karakter kodlaması", "Windows-1254 (Türkçe)")), "{facts:?}");
}

#[test]
fn hatches_with_islands_solid_fill_and_a_cross_pattern() {
    let r = read("hatch.dxf");
    let hatches: Vec<_> = r
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Hatch(h) => Some(h),
            _ => None,
        })
        .collect();
    assert_eq!(hatches.len(), 3);
    let h = hatches[0];
    assert_eq!(h.ring, vec![v(0.0, 0.0), v(10.0, 0.0), v(10.0, 10.0), v(0.0, 10.0)]);
    let holes = h.holes.as_ref().expect("island");
    assert_eq!(holes.len(), 1);
    // The clockwise arc edge is stored mirrored: it dips through (5, 5).
    assert!(holes[0].iter().any(|p| near(*p, v(5.0, 5.0))), "{:?}", holes[0]);
    assert!(holes[0].iter().all(|p| p.y <= 7.0 + 1e-9));
    assert_eq!(h.pattern.kind, HatchPatternType::Lines);
    assert!((h.pattern.angle - 45.0).abs() < 1e-12 && (h.pattern.spacing - 3.175).abs() < 1e-9, "{:?}", h.pattern);
    assert_eq!(hatches[1].pattern.kind, HatchPatternType::Solid);
    assert_eq!(hatches[1].ring.len(), 72);
    assert_eq!((hatches[2].pattern.kind, hatches[2].pattern.angle, hatches[2].pattern.spacing), (HatchPatternType::Cross, 0.0, 3.175));
}

#[test]
fn a_dimension_is_its_block_exploded() {
    let r = read("dimension.dxf");
    assert_eq!((count(&r, "line"), count(&r, "polygon"), count(&r, "text"), count(&r, "point")), (1, 1, 1, 0));
    assert!(r.entities.iter().all(|e| match e {
        Entity::Line(l) => l.base.layer_id == "OLCU",
        Entity::Polygon(p) => p.base.layer_id == "OLCU",
        Entity::Text(t) => t.base.layer_id == "OLCU" && t.text == "10.00" && (t.p.x - (5.0 - 5.0 * 0.55 * 0.5 / 2.0)).abs() < 1e-12,
        _ => false,
    }));
    assert!(r.report.notes.iter().any(|n| n.what == "Ölçü (DIMENSION)"));
    assert!(skipped(&r, "Ölçü (DIMENSION)").is_some_and(|s| s.contains("çizim bloğu dosyada yok")));
}

#[test]
fn files_that_are_not_ascii_dxf_say_what_to_do() {
    let err = |bytes: &[u8]| dxf::read(bytes, &DxfReadOptions::default()).err().unwrap_or_default();
    assert!(err(b"AC1027\x00\x00\x00binary").contains("DWG dosyası"));
    assert!(err(b"AutoCAD Binary DXF\r\n\x1a\x00").contains("ikili (binary) DXF"));
    assert!(err(b"Nokta 1 452000 4412000\n").contains("grup kodu bir sayı olmalı"));
    assert!(err(b"  1\nmerhaba\n").contains("DXF dosyası değil"));
    assert!(err(b"  0\nSECTION\n  2\nENTITIES\n  0\nLINE\n 10\n").contains("dosya bir grup kodundan sonra bitiyor"));
}

#[test]
fn a_large_file_reads_in_one_pass() {
    let mut text = String::from("  0\nSECTION\n  2\nENTITIES\n");
    for i in 0..60_000 {
        text.push_str(&format!("  0\nLINE\n  8\nL{}\n 10\n{i}.5\n 20\n{}.25\n 11\n{}\n 21\n0\n", i % 7, i * 2, i + 1));
    }
    text.push_str("  0\nENDSEC\n  0\nEOF\n");
    let t0 = std::time::Instant::now();
    let r = dxf::read(text.as_bytes(), &DxfReadOptions::default()).expect("read");
    assert_eq!(r.entities.len(), 60_000);
    assert_eq!(r.layers.len(), 7);
    let Entity::Line(l) = &r.entities[59_999] else { panic!() };
    assert_eq!(l.a, v(59_999.5, 119_998.25));
    // Far below a second even unoptimised; a quadratic pass would take minutes.
    assert!(t0.elapsed().as_secs() < 20);
    let limited = dxf::read(text.as_bytes(), &DxfReadOptions { max_entities: 1000 }).expect("read");
    assert_eq!(limited.entities.len(), 1000);
    assert!(skipped(&limited, "Nesne sınırı").is_some());
}
