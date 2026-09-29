//! The DXF reader on the files in `fixtures/formats/v1/` (each built to
//! test one feature; the WASM test `apps/web/src/io/formats.wasm.test.ts` reads the
//! same files): every entity kind, object coordinate systems, nested and
//! array inserts, hatches with islands, dimensions, and files that are not
//! DXF at all. Expected values are worked out by hand from the file.

use kentos_contracts::{DxfReadOptions, Entity, HatchPatternType, ImportResult, LineType, Vec2};
use kentos_formats::dxf;
use kentos_formats::math::{cos, sin};

const PI: f64 = std::f64::consts::PI;

fn fixture(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/../../../fixtures/formats/v1/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
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
    r.report
        .skipped
        .iter()
        .find(|s| s.what == what)
        .map(|s| s.reason.clone())
}

#[test]
fn every_kind_with_layers_colours_and_the_turkish_code_page() {
    let r = read("entities.dxf");
    let facts: Vec<(&str, &str)> = r
        .report
        .source
        .iter()
        .map(|f| (f.label.as_str(), f.value.as_str()))
        .collect();
    assert!(
        facts.contains(&("Sürüm", "AutoCAD 2000 (AC1015)")),
        "{facts:?}"
    );
    assert!(facts.contains(&("Karakter kodlaması", "Windows-1254 (Türkçe)")));
    assert!(facts.contains(&("Birim ($INSUNITS)", "metre")));

    let layer = |n: &str| {
        r.layers
            .iter()
            .find(|l| l.name == n)
            .unwrap_or_else(|| panic!("layer {n}"))
    };
    assert_eq!(
        (layer("PARSEL").color.as_str(), layer("PARSEL").line_weight),
        ("#FF0000", Some(0.35))
    );
    let road = layer("Yol ekseni");
    assert_eq!(
        (road.color.as_str(), road.visible, road.line_type),
        ("#0000FF", false, LineType::Dashed)
    );
    assert!(layer("Kot").locked);
    // The Turkish name came through Windows-1254; a frozen layer is hidden.
    assert!(!layer("Yazı").visible);
    assert_eq!(layer("0").color, "ink");

    let e = &r.entities;
    let Entity::Line(l) = &e[0] else {
        panic!("{:?}", e[0])
    };
    // Coordinates bit for bit.
    assert_eq!(
        (l.a, l.b),
        (v(452000.125, 4412000.5), v(452010.25, 4412005.75))
    );
    assert_eq!(l.base.layer_id, "PARSEL");
    let Entity::Polygon(p) = &e[1] else {
        panic!("{:?}", e[1])
    };
    assert_eq!(p.pts.len(), 4);
    assert_eq!(p.bulges, Some(vec![0.0, 0.5, 0.0, 0.0]));
    let Entity::Polyline(p) = &e[2] else {
        panic!("{:?}", e[2])
    };
    assert_eq!(p.bulges, Some(vec![-1.0, 0.0]));
    let Entity::Polyline(p) = &e[3] else {
        panic!("{:?}", e[3])
    };
    assert_eq!(
        (p.pts.clone(), p.bulges.clone()),
        (
            vec![v(0.0, 0.0), v(4.0, 0.0), v(4.0, 4.0)],
            Some(vec![1.0, 0.0])
        )
    );
    let Entity::Circle(c) = &e[4] else {
        panic!("{:?}", e[4])
    };
    assert_eq!(
        (c.c, c.r, c.base.color.as_deref()),
        (v(100.0, 200.0), 2.5, Some("#FF0000"))
    );
    let Entity::Arc(a) = &e[5] else {
        panic!("{:?}", e[5])
    };
    assert_eq!((a.a0, a.a1), (30.0 * PI / 180.0, 120.0 * PI / 180.0));
    let Entity::Ellipse(el) = &e[6] else {
        panic!("{:?}", e[6])
    };
    assert_eq!(
        (el.c, el.major, el.ratio, el.t0, el.t1),
        (v(50.0, 60.0), v(10.0, 0.0), 0.5, 0.0, PI)
    );
    let Entity::Spline(s) = &e[7] else {
        panic!("{:?}", e[7])
    };
    assert_eq!((s.pts.len(), s.closed), (4, false));
    let Entity::Polyline(p) = &e[8] else {
        panic!("{:?}", e[8])
    };
    assert_eq!(
        (p.pts.first(), p.pts.last()),
        (Some(&v(0.0, 0.0)), Some(&v(2.0, 0.0)))
    );
    let Entity::Point(pt) = &e[9] else {
        panic!("{:?}", e[9])
    };
    assert_eq!(
        (pt.p, pt.z, pt.base.layer_id.as_str()),
        (v(452001.0, 4412001.0), Some(105.25), "Kot")
    );
    let Entity::Point(pt) = &e[10] else {
        panic!("{:?}", e[10])
    };
    assert_eq!(pt.z, None);
    let Entity::Text(t) = &e[11] else {
        panic!("{:?}", e[11])
    };
    assert_eq!(
        (
            t.text.as_str(),
            t.p,
            t.height,
            t.rotation,
            t.base.color.as_deref()
        ),
        (
            "Ada 101 Ø20 İş",
            v(452005.0, 4412005.0),
            2.5,
            30.0,
            Some("#00FF00")
        )
    );
    let Entity::Text(t) = &e[12] else {
        panic!("{:?}", e[12])
    };
    assert_eq!((t.text.as_str(), t.p), ("Birinci satır", v(10.0, 18.0)));
    let Entity::Text(t) = &e[13] else {
        panic!("{:?}", e[13])
    };
    assert_eq!(t.text, "Ikinci satır");
    assert!(near(t.p, v(10.0, 18.0 - 2.0 * 5.0 / 3.0)), "{:?}", t.p);
    let Entity::Polygon(p) = &e[14] else {
        panic!("{:?}", e[14])
    };
    assert_eq!(
        p.pts,
        vec![v(0.0, 0.0), v(1.0, 0.0), v(1.0, 1.0), v(0.0, 1.0)]
    );
    let Entity::Xline(x) = &e[15] else {
        panic!("{:?}", e[15])
    };
    assert_eq!((x.p, x.dir), (v(5.0, 5.0), v(0.0, 1.0)));
    let Entity::Ray(x) = &e[16] else {
        panic!("{:?}", e[16])
    };
    assert!(near(x.dir, v(0.6, 0.8)));
    assert_eq!(e.len(), 17);

    assert!(skipped(&r, "IMAGE").is_some_and(|s| s.contains("raster")));
    assert!(skipped(&r, "Kâğıt uzayı nesnesi").is_some());
    assert!(
        r.report
            .notes
            .iter()
            .any(|n| n.what == "Genişlikli çoklu çizgi")
    );
    assert!(
        r.report
            .notes
            .iter()
            .any(|n| n.what == "Denetim noktalı eğri (SPLINE)")
    );
    assert_eq!(
        (
            count(&r, "text"),
            count(&r, "polygon"),
            count(&r, "polyline")
        ),
        (3, 2, 3)
    );
    let b = r.bounds.expect("extent");
    assert!(b.max_x >= 452020.0 && b.min_x <= 0.0);
}

#[test]
fn the_mirrored_object_coordinate_system() {
    let r = read("ocs.dxf");
    let e = &r.entities;
    // x → −x; a counter-clockwise arc turns clockwise, so its ends swap.
    let Entity::Arc(a) = &e[0] else {
        panic!("{:?}", e[0])
    };
    assert_eq!((a.c, a.r), (v(-10.0, 5.0), 2.0));
    assert!(
        (a.a0 - PI / 2.0).abs() < 1e-15 && (a.a1 - PI).abs() < 1e-15,
        "{a:?}"
    );
    let Entity::Circle(c) = &e[1] else {
        panic!("{:?}", e[1])
    };
    assert_eq!((c.c, c.r), (v(-10.0, 5.0), 3.0));
    let Entity::Polyline(p) = &e[2] else {
        panic!("{:?}", e[2])
    };
    assert_eq!(
        (p.pts.clone(), p.bulges.clone()),
        (vec![v(-1.0, 0.0), v(-3.0, 0.0)], Some(vec![-1.0]))
    );
    let Entity::Text(t) = &e[3] else {
        panic!("{:?}", e[3])
    };
    assert_eq!((t.p, t.rotation), (v(-4.0, 1.0), 0.0));
    // The ellipse's minor axis is normal × major: it points down, so the arc runs from (0, −1) to (2, 0).
    let Entity::Ellipse(el) = &e[4] else {
        panic!("{:?}", e[4])
    };
    let at = |t: f64| {
        let m = v(-el.major.y * el.ratio, el.major.x * el.ratio);
        v(
            el.c.x + el.major.x * cos(t) + m.x * sin(t),
            el.c.y + el.major.y * cos(t) + m.y * sin(t),
        )
    };
    assert!(
        near(at(el.t0), v(0.0, -1.0)) && near(at(el.t1), v(2.0, 0.0)),
        "{el:?}"
    );
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
    assert_eq!(
        circles[0],
        (
            v(1000.0, 2010.0),
            1.0,
            Some("ink".to_string()),
            "KAPILAR".to_string()
        )
    );
    // The 2 × 2 array of NO, 10 apart in columns and 20 in rows.
    let array: Vec<Vec2> = circles[1..].iter().map(|c| c.0).collect();
    assert_eq!(
        array,
        vec![v(0.0, 0.0), v(10.0, 0.0), v(0.0, 20.0), v(10.0, 20.0)]
    );
    assert!(circles[1..].iter().all(|c| c.1 == 0.5 && c.3 == "DIZI"));
    let lines: Vec<(Vec2, Vec2, String)> = r
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Line(l) => Some((l.a, l.b, l.base.layer_id.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        lines[0],
        (v(1000.0, 2010.0), v(998.0, 2010.0), "DETAY".to_string())
    );
    assert_eq!(lines.len(), 5);
    let point = r.entities.iter().find_map(|e| match e {
        Entity::Point(p) => Some((p.p, p.z, p.base.layer_id.clone())),
        _ => None,
    });
    // (3, 4) turned 90° and moved; its elevation 1.5 sits on the insert's 100.
    assert_eq!(
        point,
        Some((v(996.0, 2003.0), Some(101.5), "KAPILAR".to_string()))
    );
    let texts: Vec<&str> = r
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(texts, vec!["P-7"]);
    let Some(Entity::Ellipse(el)) = r.entities.iter().find(|e| matches!(e, Entity::Ellipse(_)))
    else {
        panic!("no ellipse")
    };
    assert_eq!(
        (el.c, el.major, el.ratio, el.t0, el.t1),
        (v(50.0, 50.0), v(3.0, 0.0), 1.0 / 3.0, 0.0, 0.0)
    );
    assert!(skipped(&r, "Blok (INSERT)").is_some());
    let reasons: Vec<&str> = r.report.skipped.iter().map(|s| s.reason.as_str()).collect();
    assert!(
        reasons
            .iter()
            .any(|s| s.contains("“KENDI” bloğu kendini içeriyor")),
        "{reasons:?}"
    );
    assert!(
        reasons
            .iter()
            .any(|s| s.contains("“YOK” blok tanımı dosyada yok"))
    );
    assert!(skipped(&r, "Görünmez öznitelik (ATTRIB)").is_some());
    // An ASCII-only file reads the same in any code page; the report names the declared one.
    let facts: Vec<(&str, &str)> = r
        .report
        .source
        .iter()
        .map(|f| (f.label.as_str(), f.value.as_str()))
        .collect();
    assert!(
        facts.contains(&("Karakter kodlaması", "Windows-1254 (Türkçe)")),
        "{facts:?}"
    );
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
    assert_eq!(
        h.ring,
        vec![v(0.0, 0.0), v(10.0, 0.0), v(10.0, 10.0), v(0.0, 10.0)]
    );
    let holes = h.holes.as_ref().expect("island");
    assert_eq!(holes.len(), 1);
    // The clockwise arc edge is stored mirrored: it dips through (5, 5).
    assert!(
        holes[0].iter().any(|p| near(*p, v(5.0, 5.0))),
        "{:?}",
        holes[0]
    );
    assert!(holes[0].iter().all(|p| p.y <= 7.0 + 1e-9));
    assert_eq!(h.pattern.kind, HatchPatternType::Lines);
    assert!(
        (h.pattern.angle - 45.0).abs() < 1e-12 && (h.pattern.spacing - 3.175).abs() < 1e-9,
        "{:?}",
        h.pattern
    );
    assert_eq!(hatches[1].pattern.kind, HatchPatternType::Solid);
    assert_eq!(hatches[1].ring.len(), 72);
    assert_eq!(
        (
            hatches[2].pattern.kind,
            hatches[2].pattern.angle,
            hatches[2].pattern.spacing
        ),
        (HatchPatternType::Cross, 0.0, 3.175)
    );
}

fn text_width(text: &str) -> f64 {
    use kentos_geometry_core::text::{Font, width_em};
    width_em(text, Font::from_id("arimo"))
}

#[test]
fn a_dimension_is_its_block_exploded() {
    let r = read("dimension.dxf");
    assert_eq!(
        (
            count(&r, "line"),
            count(&r, "polygon"),
            count(&r, "text"),
            count(&r, "point")
        ),
        (1, 1, 1, 0)
    );
    assert!(r.entities.iter().all(|e| match e {
        Entity::Line(l) => l.base.layer_id == "OLCU",
        Entity::Polygon(p) => p.base.layer_id == "OLCU",
        Entity::Text(t) =>
            t.base.layer_id == "OLCU"
                && t.text == "10.00"
                // Centred on 5: moved left by half its width in Arial's measures (Arimo).
                && (t.p.x - (5.0 - text_width("10.00") * 0.5 / 2.0)).abs() < 1e-12,
        _ => false,
    }));
    assert!(r.report.notes.iter().any(|n| n.what == "Ölçü (DIMENSION)"));
    assert!(skipped(&r, "Ölçü (DIMENSION)").is_some_and(|s| s.contains("çizim bloğu dosyada yok")));
}

#[test]
fn files_that_are_not_ascii_dxf_say_what_to_do() {
    let err = |bytes: &[u8]| {
        dxf::read(bytes, &DxfReadOptions::default())
            .err()
            .unwrap_or_default()
    };
    assert!(err(b"AC1027\x00\x00\x00binary").contains("DWG dosyası"));
    assert!(err(b"AutoCAD Binary DXF\r\n\x1a\x00").contains("ikili (binary) DXF"));
    assert!(err(b"Nokta 1 452000 4412000\n").contains("grup kodu bir sayı olmalı"));
    assert!(err(b"  1\nmerhaba\n").contains("DXF dosyası değil"));
    assert!(
        err(b"  0\nSECTION\n  2\nENTITIES\n  0\nLINE\n 10\n")
            .contains("dosya bir grup kodundan sonra bitiyor")
    );
}

/// A DXF file from its sections, each a list of (code, value) groups.
fn dxf_of(sections: &[(&str, Vec<(i32, String)>)]) -> Vec<u8> {
    let mut t = String::new();
    for (name, groups) in sections {
        t.push_str(&format!("  0\nSECTION\n  2\n{name}\n"));
        for (code, value) in groups {
            t.push_str(&format!("{code:>3}\n{value}\n"));
        }
        t.push_str("  0\nENDSEC\n");
    }
    t.push_str("  0\nEOF\n");
    t.into_bytes()
}

fn groups(list: &[(i32, &str)]) -> Vec<(i32, String)> {
    list.iter().map(|(c, v)| (*c, v.to_string())).collect()
}

#[test]
fn layer_names_are_spelled_as_the_layer_table_spells_them() {
    // DXF layer names ignore case: two lines on "PARSEL" and "parsel" belong to the table's "Parsel".
    let tables = groups(&[
        (0, "TABLE"),
        (2, "LAYER"),
        (0, "LAYER"),
        (2, "Parsel"),
        (70, "0"),
        (62, "1"),
        (6, "CONTINUOUS"),
        (0, "ENDTAB"),
    ]);
    let mut entities = Vec::new();
    for layer in ["PARSEL", "parsel"] {
        entities.extend(groups(&[
            (0, "LINE"),
            (8, layer),
            (10, "0"),
            (20, "0"),
            (11, "1"),
            (21, "1"),
        ]));
    }
    let r = dxf::read(
        &dxf_of(&[("TABLES", tables), ("ENTITIES", entities)]),
        &DxfReadOptions::default(),
    )
    .expect("read");
    assert_eq!(
        r.layers
            .iter()
            .map(|l| (l.name.as_str(), l.color.as_str(), l.count))
            .collect::<Vec<_>>(),
        vec![("Parsel", "#FF0000", 2)]
    );
    assert!(
        r.entities
            .iter()
            .all(|e| matches!(e, Entity::Line(l) if l.base.layer_id == "Parsel"))
    );
}

#[test]
fn an_attribute_that_cannot_be_read_is_reported() {
    let blocks = groups(&[
        (0, "BLOCK"),
        (2, "B"),
        (70, "0"),
        (10, "0"),
        (20, "0"),
        (0, "POINT"),
        (8, "0"),
        (10, "1"),
        (20, "2"),
        (0, "ENDBLK"),
    ]);
    let entities = groups(&[
        (0, "INSERT"),
        (8, "K"),
        (66, "1"),
        (2, "B"),
        (10, "100"),
        (20, "200"),
        (0, "ATTRIB"),
        (8, "K"),
        (10, "abc"),
        (20, "0"),
        (40, "1"),
        (1, "P-1"),
        (0, "SEQEND"),
    ]);
    let r = dxf::read(
        &dxf_of(&[("BLOCKS", blocks), ("ENTITIES", entities)]),
        &DxfReadOptions::default(),
    )
    .expect("read");
    // The block's point arrives; the broken attribute is named with its line and why.
    assert!(
        matches!(r.entities.as_slice(), [Entity::Point(p)] if p.p == v(101.0, 202.0) && p.base.layer_id == "K")
    );
    let item = r
        .report
        .skipped
        .iter()
        .find(|s| s.what == "Blok özniteliği (ATTRIB)")
        .expect("reported");
    assert!(item.reason.contains("sayı okunamadı (grup 10"), "{item:?}");
    assert_eq!(item.lines.len(), 1);
}

#[test]
fn nested_blocks_that_draw_nothing_cannot_stall_the_reader() {
    // Eleven levels, each inserting the next ten times: 10¹⁰ inserts of a block holding only an
    // attribute definition. The walk stops at its limit and says so.
    let mut blocks = Vec::new();
    for k in 0..10 {
        blocks.extend(groups(&[
            (0, "BLOCK"),
            (2, &format!("L{k}")),
            (70, "0"),
            (10, "0"),
            (20, "0"),
        ]));
        for _ in 0..10 {
            blocks.extend(groups(&[
                (0, "INSERT"),
                (8, "0"),
                (2, &format!("L{}", k + 1)),
                (10, "1"),
                (20, "1"),
            ]));
        }
        blocks.extend(groups(&[(0, "ENDBLK")]));
    }
    blocks.extend(groups(&[
        (0, "BLOCK"),
        (2, "L10"),
        (70, "0"),
        (10, "0"),
        (20, "0"),
        (0, "ATTDEF"),
        (8, "0"),
        (0, "ENDBLK"),
    ]));
    let entities = groups(&[(0, "INSERT"), (8, "0"), (2, "L0"), (10, "0"), (20, "0")]);
    let bytes = dxf_of(&[("BLOCKS", blocks), ("ENTITIES", entities)]);
    let t0 = std::time::Instant::now();
    let r = dxf::read(&bytes, &DxfReadOptions { max_entities: 1000 }).expect("read");
    assert!(t0.elapsed().as_secs() < 5);
    assert!(r.entities.is_empty());
    assert!(
        r.report.skipped.iter().any(|s| s.what == "Blok (INSERT)"
            && s.reason.contains("8000 nesneden fazlasını dolaştırdı")),
        "{:?}",
        r.report.skipped
    );
}

#[test]
fn a_minsert_array_opens_by_columns_and_rows_and_is_capped() {
    let blocks = groups(&[
        (0, "BLOCK"),
        (2, "P"),
        (70, "0"),
        (10, "0"),
        (20, "0"),
        (0, "POINT"),
        (8, "0"),
        (10, "0"),
        (20, "0"),
        (0, "ENDBLK"),
    ]);
    let minsert = |cols: &str, rows: &str| {
        groups(&[
            (0, "INSERT"),
            (8, "D"),
            (2, "P"),
            (10, "0"),
            (20, "0"),
            (70, cols),
            (71, rows),
            (44, "2"),
            (45, "5"),
        ])
    };
    let points = |r: &ImportResult| -> Vec<Vec2> {
        r.entities
            .iter()
            .filter_map(|e| match e {
                Entity::Point(p) => Some(p.p),
                _ => None,
            })
            .collect()
    };
    let r = dxf::read(
        &dxf_of(&[("BLOCKS", blocks.clone()), ("ENTITIES", minsert("3", "2"))]),
        &DxfReadOptions::default(),
    )
    .expect("read");
    assert_eq!(
        points(&r),
        vec![
            v(0.0, 0.0),
            v(2.0, 0.0),
            v(4.0, 0.0),
            v(0.0, 5.0),
            v(2.0, 5.0),
            v(4.0, 5.0)
        ]
    );
    // More than 10 000 columns: 10 000 of each row are opened, and the rest is reported.
    let r = dxf::read(
        &dxf_of(&[("BLOCKS", blocks), ("ENTITIES", minsert("10001", "2"))]),
        &DxfReadOptions::default(),
    )
    .expect("read");
    let pts = points(&r);
    assert_eq!(pts.len(), 20_000);
    assert_eq!(
        (
            pts.iter().filter(|p| p.y == 5.0).count(),
            pts.iter().map(|p| p.x).fold(0.0, f64::max)
        ),
        (10_000, 19_998.0)
    );
    assert!(skipped(&r, "Blok dizisi (MINSERT)").is_some_and(|s| s.contains("10001 × 2")));
}

#[test]
fn a_large_file_reads_in_one_pass() {
    let mut text = String::from("  0\nSECTION\n  2\nENTITIES\n");
    for i in 0..60_000 {
        text.push_str(&format!(
            "  0\nLINE\n  8\nL{}\n 10\n{i}.5\n 20\n{}.25\n 11\n{}\n 21\n0\n",
            i % 7,
            i * 2,
            i + 1
        ));
    }
    text.push_str("  0\nENDSEC\n  0\nEOF\n");
    let t0 = std::time::Instant::now();
    let r = dxf::read(text.as_bytes(), &DxfReadOptions::default()).expect("read");
    assert_eq!(r.entities.len(), 60_000);
    assert_eq!(r.layers.len(), 7);
    let Entity::Line(l) = &r.entities[59_999] else {
        panic!()
    };
    assert_eq!(l.a, v(59_999.5, 119_998.25));
    // Far below a second even unoptimised; a quadratic pass would take minutes.
    assert!(t0.elapsed().as_secs() < 20);
    let limited = dxf::read(text.as_bytes(), &DxfReadOptions { max_entities: 1000 }).expect("read");
    assert_eq!(limited.entities.len(), 1000);
    assert!(skipped(&limited, "Nesne sınırı").is_some());
}

#[test]
fn an_object_takes_its_own_line_weight_and_a_block_member_its_inserts() {
    // docs/adr/0139: 370 in hundredths of a mm; −1 BYLAYER and −3 the drawing's default are the
    // layer's; −2 BYBLOCK is the insert's, and an insert that says BYLAYER hands on its layer's.
    let tables = groups(&[
        (0, "TABLE"),
        (2, "LAYER"),
        (0, "LAYER"),
        (2, "KALIN"),
        (70, "0"),
        (62, "7"),
        (370, "70"),
        (0, "ENDTAB"),
    ]);
    let blocks = groups(&[
        (0, "BLOCK"),
        (2, "KAPI"),
        (8, "0"),
        (10, "0"),
        (20, "0"),
        (0, "LINE"),
        (8, "0"),
        (370, "-2"),
        (10, "0"),
        (20, "0"),
        (11, "1"),
        (21, "0"),
        (0, "LINE"),
        (8, "0"),
        (370, "18"),
        (10, "0"),
        (20, "1"),
        (11, "1"),
        (21, "1"),
        (0, "ENDBLK"),
    ]);
    let line = |weight: Option<&str>| {
        let mut g = vec![(0, "LINE".to_string()), (8, "0".to_string())];
        if let Some(w) = weight {
            g.push((370, w.to_string()));
        }
        g.extend(groups(&[(10, "0"), (20, "0"), (11, "5"), (21, "5")]));
        g
    };
    let insert = |layer: &str, weight: &str| {
        groups(&[(0, "INSERT"), (2, "KAPI"), (8, layer), (370, weight), (10, "10"), (20, "10")])
    };
    let mut entities = Vec::new();
    for w in [Some("35"), Some("0"), Some("-1"), Some("-3"), None, Some("x"), Some("211")] {
        entities.extend(line(w));
    }
    entities.extend(insert("0", "50"));
    entities.extend(insert("KALIN", "-1"));
    let r = dxf::read(
        &dxf_of(&[("TABLES", tables), ("BLOCKS", blocks), ("ENTITIES", entities)]),
        &DxfReadOptions::default(),
    )
    .expect("read");
    let weights: Vec<Option<f64>> = r.entities.iter().map(|e| e.base().line_weight).collect();
    assert_eq!(
        weights,
        [
            Some(0.35),
            Some(0.0),
            None,
            None,
            None,
            None,
            Some(2.11),
            // The first insert (50): its BYBLOCK member takes 0.50, its own-weight member keeps 0.18.
            Some(0.5),
            Some(0.18),
            // The second says BYLAYER on KALIN (0.70): its BYBLOCK member takes the layer's.
            Some(0.7),
            Some(0.18),
        ]
    );
}

/// A line's two elevations.
fn ends(e: &Entity) -> (Option<f64>, Option<f64>) {
    match e {
        Entity::Line(l) => (l.za, l.zb),
        other => panic!("not a line: {other:?}"),
    }
}

/// A path's or an area's vertex elevations.
fn heights(e: &Entity) -> Option<Vec<Option<f64>>> {
    match e {
        Entity::Polyline(p) | Entity::Polygon(p) => p.zs.clone(),
        other => panic!("not a path: {other:?}"),
    }
}

/// Every vertex has this elevation, one of `n`.
fn all(z: f64, n: usize) -> Option<Vec<Option<f64>>> {
    Some(vec![Some(z); n])
}

#[test]
fn vertices_take_the_heights_a_file_gives_them() {
    let r = read("elevations.dxf");
    let e = &r.entities;
    assert_eq!(e.len(), 24, "{:?}", r.report);
    assert!(r.report.skipped.is_empty(), "{:?}", r.report.skipped);
    // Lines: both ends, a 2D one (Z 0 is no elevation), and one end at 0 with the other above it.
    assert_eq!(ends(&e[0]), (Some(105.5), Some(107.25)));
    assert_eq!(ends(&e[1]), (None, None));
    assert_eq!(ends(&e[2]), (Some(0.0), Some(12.5)));
    // 3D polylines: a 0 among the heights stays a height; one that is 0 all along is a 2D one.
    assert_eq!(heights(&e[3]), Some(vec![Some(10.0), Some(12.5), Some(0.0), Some(15.25)]));
    assert!(matches!(e[3], Entity::Polyline(_)) && matches!(e[4], Entity::Polygon(_)));
    assert_eq!(heights(&e[4]), Some(vec![Some(20.0), Some(21.0), Some(22.0), Some(23.0)]));
    assert_eq!(heights(&e[5]), None);
    // LWPOLYLINEs: the elevation is every vertex's; 0 is none; an arc stays; a mirrored plane's Z runs the other way.
    assert_eq!(heights(&e[6]), all(250.5, 3));
    assert_eq!(heights(&e[7]), None);
    let Entity::Polyline(arc) = &e[8] else {
        panic!("{:?}", e[8])
    };
    assert_eq!((arc.bulges.clone(), heights(&e[8])), (Some(vec![1.0, 0.0]), all(-12.75, 3)));
    let Entity::Polyline(mirrored) = &e[9] else {
        panic!("{:?}", e[9])
    };
    assert_eq!(mirrored.pts, vec![v(-1.0, 1.0), v(-3.0, 1.0), v(-3.0, 4.0)]);
    assert_eq!(heights(&e[9]), all(5.0, 3));
    // A 2D POLYLINE's elevation is its plane's.
    assert_eq!(heights(&e[10]), all(42.0, 3));
    // The block at Z 100: a LINE at 1 and 2, a LWPOLYLINE at 38 = 0 and a 2D LINE (an insert at a height
    // gives what is inside it that height) and a 3D POLYLINE at 0, 5 and 10; all on the insert's layer.
    assert_eq!(ends(&e[11]), (Some(101.0), Some(102.0)));
    let Entity::Line(l) = &e[11] else { panic!() };
    assert_eq!((l.a, l.b, l.base.layer_id.as_str()), (v(1000.0, 2000.0), v(1010.0, 2000.0), "PLAN"));
    assert_eq!(heights(&e[12]), all(100.0, 2));
    assert_eq!(heights(&e[13]), Some(vec![Some(100.0), Some(105.0), Some(110.0)]));
    assert_eq!(ends(&e[14]), (Some(100.0), Some(100.0)));
    // The same block at Z 0 with its Z scaled by 2: heights stretch, and what has none still has none.
    assert_eq!(ends(&e[15]), (Some(2.0), Some(4.0)));
    assert_eq!(heights(&e[16]), None);
    assert_eq!(heights(&e[17]), Some(vec![Some(0.0), Some(10.0), Some(20.0)]));
    assert_eq!(ends(&e[18]), (None, None));
    let Entity::Point(p) = &e[19] else {
        panic!("{:?}", e[19])
    };
    assert_eq!(p.z, Some(88.8));
    // KentOS's data: zeros that are heights, a vertex with none, and a mark gone stale (the vertex it
    // names has a height of its own now, another program's edit wins).
    assert_eq!(ends(&e[20]), (Some(0.0), None));
    assert_eq!(heights(&e[21]), all(0.0, 3));
    assert_eq!(heights(&e[22]), Some(vec![Some(5.0), None, Some(7.0)]));
    assert_eq!(heights(&e[23]), Some(vec![Some(5.0), Some(0.0), Some(7.0)]));
    // The report says how many objects came with elevations, once, as a fact of the file.
    let facts: Vec<(&str, &str)> = r
        .report
        .source
        .iter()
        .map(|f| (f.label.as_str(), f.value.as_str()))
        .collect();
    assert!(facts.contains(&("Kotlu nesne", "19")), "{facts:?}");
}

#[test]
fn a_closed_polyline_that_repeats_its_first_vertex_keeps_the_firsts_height() {
    let vertex = |x: &str, y: &str, z: &str| groups(&[(0, "VERTEX"), (8, "0"), (10, x), (20, y), (30, z), (70, "32")]);
    let mut entities = groups(&[(0, "POLYLINE"), (8, "0"), (66, "1"), (10, "0"), (20, "0"), (30, "0"), (70, "9")]);
    for v in [("0", "0", "10"), ("4", "0", "11"), ("4", "4", "12"), ("0", "4", "13"), ("0", "0", "99")] {
        entities.extend(vertex(v.0, v.1, v.2));
    }
    entities.extend(groups(&[(0, "SEQEND"), (8, "0")]));
    let r = dxf::read(&dxf_of(&[("ENTITIES", entities)]), &DxfReadOptions::default()).expect("read");
    assert_eq!(r.entities.len(), 1, "{:?}", r.report);
    assert_eq!(heights(&r.entities[0]), Some(vec![Some(10.0), Some(11.0), Some(12.0), Some(13.0)]));
    let said = r.report.notes.iter().find(|n| n.what == "Halka kapanışının Z'si");
    assert_eq!(said.map(|n| n.count), Some(1), "{:?}", r.report.notes);
}

#[test]
fn a_drawing_without_a_height_says_nothing_of_heights() {
    // Nothing above 0 in the file: no elevation on any object, and no fact of them.
    let r = read("hatch.dxf");
    assert!(!r.report.source.iter().any(|f| f.label == "Kotlu nesne"));
    assert!(r.entities.iter().all(|e| !kentos_formats::import::has_elevation(e)));
    // The block of blocks.dxf sits at Z 100: what is drawn in it stands there, the point and the lines alike.
    let r = read("blocks.dxf");
    assert!(r.report.source.iter().any(|f| f.label == "Kotlu nesne"));
    let Some(Entity::Line(l)) = r.entities.iter().find(|e| matches!(e, Entity::Line(_))) else {
        panic!("no line")
    };
    assert_eq!((l.za, l.zb), (Some(100.0), Some(100.0)));
}
