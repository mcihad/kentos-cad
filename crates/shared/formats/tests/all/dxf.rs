//! The DXF reader on the files in `fixtures/formats/v1/` (each built to
//! test one feature; the WASM test `apps/web/src/io/formats.wasm.test.ts` reads the
//! same files): every entity kind, object coordinate systems, nested and
//! array inserts, hatches with islands, dimensions, and files that are not
//! DXF at all. Expected values are worked out by hand from the file.

use kentos_contracts::{
    BlockId, DxfReadOptions, Entity, HatchPatternType, ImportResult, LeaderArrow, LineType,
    TextAlign, Vec2,
};
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

/// Read with Blokları patlat: every insert opened into its objects, as before blocks (docs/adr/0144 §5).
fn opened(name: &str) -> ImportResult {
    let opts = DxfReadOptions {
        explode_blocks: true,
        ..DxfReadOptions::default()
    };
    dxf::read(&fixture(name), &opts).unwrap_or_else(|e| panic!("{name}: {e}"))
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
    // An MTEXT's lines hang from its attachment point, each aligned by it (docs/adr/0145 §7).
    assert_eq!(
        (t.text.as_str(), t.p, t.align),
        ("Birinci satır", v(10.0, 20.0), Some(TextAlign::TopLeft))
    );
    let Entity::Text(t) = &e[13] else {
        panic!("{:?}", e[13])
    };
    assert_eq!((t.text.as_str(), t.align), ("Ikinci satır", Some(TextAlign::TopLeft)));
    assert!(near(t.p, v(10.0, 20.0 - 2.0 * 5.0 / 3.0)), "{:?}", t.p);
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

/// blocks.dxf with its blocks kept (docs/adr/0144 §5), worked out by hand:
/// NO, KAPI, DAIRE and KENDI are definitions 1–4 in the file's order, their
/// objects on layer 0 (NO's BYBLOCK circle takes no colour: the insert's);
/// KAPI holds NO as an insert (at 10, 0; twice as large; a quarter turn) and
/// its point at Z 1.5. In the drawing KAPI's insert stands at (1000, 2000),
/// turned a quarter, blue; its Z of 100 is not kept. NO's 2 × 2 array and
/// DAIRE's unequal scales are opened as before, and said; KENDI's insert of
/// itself is left out of its definition; YOK has no definition.
#[test]
fn blocks_are_kept_as_definitions_and_inserts_place_them() {
    let r = read("blocks.dxf");
    let names: Vec<(&str, BlockId)> = r.blocks.iter().map(|b| (b.name.as_str(), b.id)).collect();
    let id = |n: u8| {
        let mut bytes = [0u8; 16];
        bytes[15] = n;
        BlockId(bytes)
    };
    assert_eq!(
        names,
        [
            ("NO", id(1)),
            ("KAPI", id(2)),
            ("DAIRE", id(3)),
            ("KENDI", id(4))
        ]
    );
    let no = &r.blocks[0];
    assert_eq!((no.base.x, no.base.y), (1.0, 1.0));
    let [Entity::Circle(c), Entity::Line(l)] = no.entities.as_slice() else {
        panic!("{:?}", no.entities)
    };
    assert_eq!(
        (
            c.c,
            c.r,
            c.base.color.as_deref(),
            c.base.layer_id.as_str(),
            c.base.id
        ),
        (v(1.0, 1.0), 0.5, None, "", 1)
    );
    // On 0: the block's layer (none of its own); on DETAY: named, for Patlat.
    assert_eq!(
        (l.a, l.b, l.base.layer_id.as_str(), l.base.id),
        (v(1.0, 1.0), v(2.0, 1.0), "DETAY", 2)
    );
    let [Entity::Insert(inner), Entity::Point(pt)] = r.blocks[1].entities.as_slice() else {
        panic!("{:?}", r.blocks[1].entities)
    };
    assert_eq!(
        (
            inner.block,
            inner.p,
            inner.scale,
            inner.rotation,
            inner.mirror
        ),
        (id(1), v(10.0, 0.0), 2.0, PI / 2.0, false)
    );
    assert_eq!((pt.p, pt.z), (v(3.0, 4.0), Some(1.5)));
    assert!(
        r.blocks[3].entities.is_empty(),
        "KENDI's insert of itself is left out"
    );

    /// An insert as the test reads it: its block, where, scale, turn, layer and colour.
    type Placed<'a> = (BlockId, Vec2, f64, f64, &'a str, Option<&'a str>);
    let inserts: Vec<Placed> = r
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Insert(i) => Some((
                i.block,
                i.p,
                i.scale,
                i.rotation,
                i.base.layer_id.as_str(),
                i.base.color.as_deref(),
            )),
            _ => None,
        })
        .collect();
    let blue = kentos_formats::dxf::aci::color(5);
    assert_eq!(
        inserts,
        [
            (
                id(2),
                v(1000.0, 2000.0),
                1.0,
                PI / 2.0,
                "KAPILAR",
                Some(blue.as_str())
            ),
            (id(4), v(0.0, 0.0), 1.0, 0.0, "0", None),
        ]
    );
    // NO's array is opened as before: its four circles; DAIRE's unequal scales an ellipse.
    let circles = r
        .entities
        .iter()
        .filter(|e| matches!(e, Entity::Circle(_)))
        .count();
    assert_eq!(circles, 4);
    assert!(r.entities.iter().any(|e| matches!(e, Entity::Ellipse(_))));
    let notes: Vec<&str> = r.report.notes.iter().map(|n| n.reason.as_str()).collect();
    for said in [
        "blok dizisi (MINSERT) patlatılarak alındı",
        "X ve Y ölçeği eşit olmayan yerleştirme patlatılarak alındı",
        "yerleştirmenin yüksekliği (Z) alınmadı",
    ] {
        assert!(notes.contains(&said), "{said}: {notes:?}");
    }
    // NO's line is on DETAY: drawn on its insert's layer with DETAY's look, and said.
    let other = r
        .report
        .notes
        .iter()
        .find(|n| n.what == "Blok (BLOCK)")
        .expect("the note on NO's layers");
    assert_eq!(other.lines, [53]);
    assert!(other.reason.contains("gizliliği ve kilidi uygulanmaz"));
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
            .any(|s| s.contains("“YOK” blok tanımı dosyada yok")),
        "{reasons:?}"
    );
    let facts: Vec<(&str, &str)> = r
        .report
        .source
        .iter()
        .map(|f| (f.label.as_str(), f.value.as_str()))
        .collect();
    assert!(
        facts.contains(&("Blok", "4 tanım (1 tanesi yerleştirilmemiş)")),
        "{facts:?}"
    );
    // Blokları patlat: no definition, every insert opened.
    let open = opened("blocks.dxf");
    assert!(open.blocks.is_empty());
    assert!(!open.entities.iter().any(|e| matches!(e, Entity::Insert(_))));
}

#[test]
fn an_anonymous_block_is_opened_and_said_why() {
    // The file's only block is an anonymous one (a dynamic block's copy): nothing is kept, its
    // insert is opened, and the report says why, not that Blokları patlat was chosen.
    let blocks = groups(&[
        (0, "BLOCK"),
        (2, "*U1"),
        (70, "1"),
        (10, "0"),
        (20, "0"),
        (0, "LINE"),
        (8, "0"),
        (10, "0"),
        (20, "0"),
        (11, "1"),
        (21, "0"),
        (0, "ENDBLK"),
    ]);
    let entities = groups(&[(0, "INSERT"), (8, "0"), (2, "*U1"), (10, "5"), (20, "5")]);
    let bytes = dxf_of(&[("BLOCKS", blocks), ("ENTITIES", entities)]);
    let r = dxf::read(&bytes, &DxfReadOptions::default()).expect("read");
    assert!(r.blocks.is_empty());
    let [Entity::Line(l)] = r.entities.as_slice() else {
        panic!("{:?}", r.entities)
    };
    assert_eq!((l.a, l.b), (v(5.0, 5.0), v(6.0, 5.0)));
    let notes: Vec<&str> = r.report.notes.iter().map(|n| n.reason.as_str()).collect();
    assert!(
        notes.contains(
            &"adsız blok (dinamik blok ya da grup) blok olarak tutulmaz; patlatılarak alındı"
        ),
        "{notes:?}"
    );
    assert!(
        !notes.iter().any(|n| n.starts_with("Blokları patlat")),
        "{notes:?}"
    );
}

#[test]
fn nested_and_array_inserts_with_attributes() {
    let r = opened("blocks.dxf");
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
                // Standing on its MTEXT's attachment point, the bottom's centre (docs/adr/0145 §7).
                && t.p == v(5.0, 5.5)
                && t.align == Some(TextAlign::BottomCenter),
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
        &DxfReadOptions {
            explode_blocks: true,
            ..DxfReadOptions::default()
        },
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
    let opts = DxfReadOptions {
        max_entities: 1000,
        explode_blocks: true,
        unit: None,
    };
    let r = dxf::read(&bytes, &opts).expect("read");
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
    let limit = DxfReadOptions {
        max_entities: 1000,
        ..DxfReadOptions::default()
    };
    let limited = dxf::read(text.as_bytes(), &limit).expect("read");
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
        groups(&[
            (0, "INSERT"),
            (2, "KAPI"),
            (8, layer),
            (370, weight),
            (10, "10"),
            (20, "10"),
        ])
    };
    let mut entities = Vec::new();
    for w in [
        Some("35"),
        Some("0"),
        Some("-1"),
        Some("-3"),
        None,
        Some("x"),
        Some("211"),
    ] {
        entities.extend(line(w));
    }
    entities.extend(insert("0", "50"));
    entities.extend(insert("KALIN", "-1"));
    let r = dxf::read(
        &dxf_of(&[
            ("TABLES", tables),
            ("BLOCKS", blocks),
            ("ENTITIES", entities),
        ]),
        &DxfReadOptions {
            explode_blocks: true,
            ..DxfReadOptions::default()
        },
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
    let r = opened("elevations.dxf");
    let e = &r.entities;
    assert_eq!(e.len(), 24, "{:?}", r.report);
    assert!(r.report.skipped.is_empty(), "{:?}", r.report.skipped);
    // Lines: both ends, a 2D one (Z 0 is no elevation), and one end at 0 with the other above it.
    assert_eq!(ends(&e[0]), (Some(105.5), Some(107.25)));
    assert_eq!(ends(&e[1]), (None, None));
    assert_eq!(ends(&e[2]), (Some(0.0), Some(12.5)));
    // 3D polylines: a 0 among the heights stays a height; one that is 0 all along is a 2D one.
    assert_eq!(
        heights(&e[3]),
        Some(vec![Some(10.0), Some(12.5), Some(0.0), Some(15.25)])
    );
    assert!(matches!(e[3], Entity::Polyline(_)) && matches!(e[4], Entity::Polygon(_)));
    assert_eq!(
        heights(&e[4]),
        Some(vec![Some(20.0), Some(21.0), Some(22.0), Some(23.0)])
    );
    assert_eq!(heights(&e[5]), None);
    // LWPOLYLINEs: the elevation is every vertex's; 0 is none; an arc stays; a mirrored plane's Z runs the other way.
    assert_eq!(heights(&e[6]), all(250.5, 3));
    assert_eq!(heights(&e[7]), None);
    let Entity::Polyline(arc) = &e[8] else {
        panic!("{:?}", e[8])
    };
    assert_eq!(
        (arc.bulges.clone(), heights(&e[8])),
        (Some(vec![1.0, 0.0]), all(-12.75, 3))
    );
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
    assert_eq!(
        (l.a, l.b, l.base.layer_id.as_str()),
        (v(1000.0, 2000.0), v(1010.0, 2000.0), "PLAN")
    );
    assert_eq!(heights(&e[12]), all(100.0, 2));
    assert_eq!(
        heights(&e[13]),
        Some(vec![Some(100.0), Some(105.0), Some(110.0)])
    );
    assert_eq!(ends(&e[14]), (Some(100.0), Some(100.0)));
    // The same block at Z 0 with its Z scaled by 2: heights stretch, and what has none still has none.
    assert_eq!(ends(&e[15]), (Some(2.0), Some(4.0)));
    assert_eq!(heights(&e[16]), None);
    assert_eq!(
        heights(&e[17]),
        Some(vec![Some(0.0), Some(10.0), Some(20.0)])
    );
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
    let vertex = |x: &str, y: &str, z: &str| {
        groups(&[
            (0, "VERTEX"),
            (8, "0"),
            (10, x),
            (20, y),
            (30, z),
            (70, "32"),
        ])
    };
    let mut entities = groups(&[
        (0, "POLYLINE"),
        (8, "0"),
        (66, "1"),
        (10, "0"),
        (20, "0"),
        (30, "0"),
        (70, "9"),
    ]);
    for v in [
        ("0", "0", "10"),
        ("4", "0", "11"),
        ("4", "4", "12"),
        ("0", "4", "13"),
        ("0", "0", "99"),
    ] {
        entities.extend(vertex(v.0, v.1, v.2));
    }
    entities.extend(groups(&[(0, "SEQEND"), (8, "0")]));
    let r = dxf::read(
        &dxf_of(&[("ENTITIES", entities)]),
        &DxfReadOptions::default(),
    )
    .expect("read");
    assert_eq!(r.entities.len(), 1, "{:?}", r.report);
    assert_eq!(
        heights(&r.entities[0]),
        Some(vec![Some(10.0), Some(11.0), Some(12.0), Some(13.0)])
    );
    let said = r
        .report
        .notes
        .iter()
        .find(|n| n.what == "Halka kapanışının Z'si");
    assert_eq!(said.map(|n| n.count), Some(1), "{:?}", r.report.notes);
}

#[test]
fn a_drawing_without_a_height_says_nothing_of_heights() {
    // Nothing above 0 in the file: no elevation on any object, and no fact of them.
    let r = read("hatch.dxf");
    assert!(!r.report.source.iter().any(|f| f.label == "Kotlu nesne"));
    assert!(
        r.entities
            .iter()
            .all(|e| !kentos_formats::import::has_elevation(e))
    );
    // The block of blocks.dxf sits at Z 100: what is drawn in it stands there, the point and the lines alike.
    let r = opened("blocks.dxf");
    assert!(r.report.source.iter().any(|f| f.label == "Kotlu nesne"));
    let Some(Entity::Line(l)) = r.entities.iter().find(|e| matches!(e, Entity::Line(_))) else {
        panic!("no line")
    };
    assert_eq!((l.za, l.zb), (Some(100.0), Some(100.0)));
}

/// A note of the report by what it is about.
fn noted(r: &ImportResult, what: &str) -> Option<String> {
    r.report
        .notes
        .iter()
        .find(|s| s.what == what)
        .map(|s| s.reason.clone())
}

/// ROGAR's attribute definitions (docs/adr/0144 §7, fixtures/formats/v1/attributes.dxf):
/// NO (a default, left), KOT (none), ORTA (centred on 0, 1.2) become its
/// attributes in the file's order; the invisible GIZLI is not taken, the
/// constant SABIT is a text of the block, a second NO and one without a tag
/// are left out, each said. The first insert's ATTRIBs are its attributes:
/// NO and KOT shown by the insert itself (no text of their own), the hidden
/// GIZLI kept as a value, EK (no definition) a value and a text as before.
#[test]
fn a_blocks_attribute_definitions_come_in_as_its_attributes() {
    let r = read("attributes.dxf");
    let [rogar] = r.blocks.as_slice() else {
        panic!("{:?}", r.blocks)
    };
    /// An attribute as the test reads it: tag, prompt, default, place, height, turn.
    type Attribute<'a> = (&'a str, Option<&'a str>, Option<&'a str>, Vec2, f64, f64);
    let attributes: Vec<Attribute> = rogar
        .attributes
        .iter()
        .map(|a| {
            (
                a.tag.as_str(),
                a.prompt.as_deref(),
                a.value.as_deref(),
                a.p,
                a.height,
                a.rotation,
            )
        })
        .collect();
    // ORTA stands centred on its baseline at its alignment point (11), as the drawing's twin
    // TEXT at (487130, 4420211.2) does (docs/adr/0145 §7).
    let twin = r
        .entities
        .iter()
        .find_map(|e| match e {
            Entity::Text(t) if t.text == "MERKEZ" => Some((t.p, t.align)),
            _ => None,
        })
        .expect("the twin text");
    assert_eq!(
        twin,
        (v(487130.0, 4420211.2), Some(TextAlign::BaselineCenter))
    );
    let orta = v(0.0, 1.2);
    assert_eq!(
        rogar.attributes.iter().map(|a| a.align).collect::<Vec<_>>(),
        [None, None, Some(TextAlign::BaselineCenter)]
    );
    assert_eq!(
        attributes[..2],
        [
            (
                "NO",
                Some("Rögar numarası"),
                Some("R-?"),
                v(0.9, 0.15),
                0.5,
                0.0
            ),
            ("KOT", Some("Kapak kotu"), None, v(0.9, -0.55), 0.4, 0.0),
        ]
    );
    let (tag, prompt, value, p, height, rotation) = attributes[2];
    assert_eq!(
        (tag, prompt, value, height, rotation),
        ("ORTA", Some("Orta"), Some("MERKEZ"), 0.3, 0.0)
    );
    assert_eq!(p, orta);
    // The circle, and the constant SABIT as a text of the block.
    let [Entity::Circle(c), Entity::Text(fixed)] = rogar.entities.as_slice() else {
        panic!("{:?}", rogar.entities)
    };
    assert_eq!(c.r, 0.75);
    assert_eq!((fixed.text.as_str(), fixed.p), ("SBT", v(-0.2, -1.2)));
    assert!(noted(&r, "Sabit öznitelik (ATTDEF)").is_some());
    assert!(skipped(&r, "Görünmez öznitelik tanımı (ATTDEF)").is_some());
    let left_out: Vec<&str> = r
        .report
        .skipped
        .iter()
        .filter(|s| s.what == "Blok öznitelik tanımı (ATTDEF)")
        .map(|s| s.reason.as_str())
        .collect();
    assert_eq!(
        left_out,
        [
            "“NO” etiketi blokta ikinci kez var; ilki alındı",
            "etiketi yok"
        ]
    );
    // The inserts: the first with its four values, the second with none; EK's text between them.
    let placed: Vec<String> = r
        .entities
        .iter()
        .map(|e| match e {
            Entity::Insert(i) => format!("insert {:?} {:?}", (i.p.x, i.p.y), i.base.attrs),
            Entity::Text(t) => format!("text {} {:?}", t.text, (t.p.x, t.p.y)),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(
        placed[..3],
        [
            r#"insert (487100.0, 4420200.0) {"EK": "ekstra", "GIZLI": "secret", "KOT": "101.35", "NO": "R-12"}"#,
            "text ekstra (487100.0, 4420202.0)",
            "insert (487110.0, 4420200.0) {}",
        ]
    );
    assert!(noted(&r, "Blok özniteliği (ATTRIB)").is_some_and(|n| n.contains("karşılığı olmayan")));
    // A hidden ATTRIB of a kept insert is its value: not a text left out.
    assert!(skipped(&r, "Görünmez öznitelik (ATTRIB)").is_none());
}

/// With Blokları patlat the inserts open: the ATTRIBs come in as texts, as
/// before (the hidden one said), the constant attribute as its text, the
/// other definitions as nothing.
#[test]
fn an_opened_blocks_attributes_come_in_as_texts() {
    let r = opened("attributes.dxf");
    assert!(r.blocks.is_empty());
    let texts: Vec<&str> = r
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(texts, ["SBT", "R-12", "101.35", "ekstra", "SBT", "MERKEZ"]);
    assert!(skipped(&r, "Görünmez öznitelik (ATTRIB)").is_some());
}

/// A text as the texts' test reads it: words, place, height, turn, alignment,
/// width factor and mask.
type Placed<'a> = (&'a str, Vec2, f64, f64, Option<TextAlign>, Option<f64>, bool);

fn placed(e: &Entity) -> Placed<'_> {
    let Entity::Text(t) = e else {
        panic!("not a text: {e:?}")
    };
    (
        t.text.as_str(),
        t.p,
        t.height,
        t.rotation,
        t.align,
        t.width_factor,
        t.mask,
    )
}

/// Texts of docs/adr/0145 §7 (fixtures/formats/v1/texts.dxf; the places
/// worked out by hand): a justified TEXT stands on its alignment point (11)
/// with the alignment its 72 and 73 name, whatever its 10 says; 72 = 4 is
/// the middle's centre; one without an 11 stands on its 10; 41 is its width
/// factor when it may be one; aligned (72 = 3) and fitted (72 = 5) ones lie
/// between their points, their height or width factor from their width in
/// Arimo, said; KentOS's data gives a mask. An MTEXT's attachment point is
/// its lines' alignment, the lines a gap (5/3 of the height) apart from it;
/// its background fill (90: 1 or 2, not a frame's 16) a mask. An attribute
/// definition and an ATTRIB its block does not define are justified as a
/// TEXT is; a non-uniform scale widens an opened block's text.
#[test]
fn texts_take_their_alignment_width_factor_and_mask() {
    use TextAlign::*;
    let r = read("texts.dxf");
    let e = &r.entities;
    let at = |i: usize| placed(&e[i]);
    let h = 2.0;
    assert_eq!(at(0), ("SOL", v(0.0, 0.0), h, 0.0, None, None, false));
    assert_eq!(at(1), ("ORTA", v(30.0, 0.0), h, 0.0, Some(BaselineCenter), None, false));
    assert_eq!(at(2), ("SAG", v(60.0, 0.0), h, 0.0, Some(BaselineRight), None, false));
    let rows = [
        ["SOL ALT", "ORTA ALT", "SAG ALT"],
        ["SOL ORTA", "ORTA ORTA", "SAG ORTA"],
        ["SOL UST", "ORTA UST", "SAG UST"],
    ];
    let aligns = [
        [BottomLeft, BottomCenter, BottomRight],
        [MiddleLeft, MiddleCenter, MiddleRight],
        [TopLeft, TopCenter, TopRight],
    ];
    for (row, (words, aligns)) in rows.iter().zip(aligns).enumerate() {
        for (col, (words, align)) in words.iter().zip(aligns).enumerate() {
            let p = v(30.0 * col as f64, 10.0 * (row + 1) as f64);
            assert_eq!(at(3 + 3 * row + col), (*words, p, h, 0.0, Some(align), None, false));
        }
    }
    assert_eq!(at(12), ("MIDDLE", v(100.0, 0.0), h, 0.0, Some(MiddleCenter), None, false));
    assert_eq!(at(13), ("GENIS", v(100.0, 10.0), h, 30.0, None, Some(0.8), false));
    assert_eq!(at(14), ("BOZUK", v(100.0, 20.0), h, 0.0, None, None, false));
    assert!(noted(&r, "Yazı genişliği (41)").is_some_and(|n| n.contains("1 alındı")));
    // HIZALI runs 10 m at a width factor of 1: its height makes it that long.
    let (words, p, height, rotation, align, widths, mask) = at(15);
    assert_eq!((words, p, rotation, align, widths, mask), ("HIZALI", v(130.0, 0.0), 0.0, None, None, false));
    assert!((height - 10.0 / text_width("HIZALI")).abs() < 1e-12, "{height}");
    assert!(noted(&r, "Hizalı yazı (72 = 3)").is_some());
    // SIGDIR runs 10 m up at a height of 2: its width factor makes it that long.
    let (words, p, height, rotation, align, widths, mask) = at(16);
    assert_eq!((words, p, height, align, mask), ("SIGDIR", v(130.0, 10.0), 2.0, None, false));
    assert!((rotation - 90.0).abs() < 1e-12, "{rotation}");
    let fitted = widths.expect("a width factor");
    assert!((fitted - 10.0 / (text_width("SIGDIR") * 2.0)).abs() < 1e-12, "{fitted}");
    assert!(noted(&r, "Sığdırılmış yazı (72 = 5)").is_some());
    assert_eq!(at(17), ("TEK NOKTA", v(160.0, 0.0), h, 0.0, Some(MiddleCenter), None, false));
    assert_eq!(at(18), ("ZEMINLI", v(160.0, 10.0), h, 0.0, None, None, true));
    // MTEXT: the top's left, on two lines.
    assert_eq!(at(19), ("UST SOL", v(0.0, 50.0), h, 0.0, Some(TopLeft), None, false));
    let (words, p, _, _, align, _, _) = at(20);
    assert_eq!((words, align), ("IKINCI", Some(TopLeft)));
    assert!(near(p, v(0.0, 50.0 - 2.0 * 5.0 / 3.0)), "{p:?}");
    assert_eq!(at(21), ("MERKEZ", v(30.0, 50.0), h, 0.0, Some(MiddleCenter), None, true));
    // The bottom's right, on three lines 3 × 5/3 = 5 apart: the last on the insertion point.
    for (i, (words, y)) in [("A", 60.0), ("B", 55.0), ("C", 50.0)].into_iter().enumerate() {
        assert_eq!(at(22 + i), (words, v(60.0, y), 3.0, 0.0, Some(BottomRight), None, false));
    }
    assert_eq!(at(25), ("ALT SOL", v(0.0, 70.0), h, 0.0, Some(BottomLeft), None, false));
    // ETIKET's insert holds its value; its definition's attribute is justified as a TEXT.
    let Entity::Insert(etiket) = &e[26] else {
        panic!("{:?}", e[26])
    };
    assert_eq!(etiket.base.attrs.get("NO").map(String::as_str), Some("7"));
    let definition = r.blocks.iter().find(|b| b.name == "ETIKET").expect("ETIKET");
    let [no] = definition.attributes.as_slice() else {
        panic!("{:?}", definition.attributes)
    };
    assert_eq!(
        (no.tag.as_str(), no.p, no.height, no.align, no.width_factor),
        ("NO", v(1.5, 0.0), 0.5, Some(MiddleCenter), Some(0.9))
    );
    // YAZILI at an X scale of 3 opens: 0.5 × 3 wide at the same height.
    assert_eq!(at(27), ("OLCEK", v(200.0, 20.0), 1.0, 0.0, None, Some(1.5), false));
    assert!(matches!(&e[28], Entity::Insert(_)));
    assert_eq!(at(29), ("SERBEST", v(225.0, 5.0), 1.0, 0.0, Some(TopRight), None, false));
    assert_eq!(e.len(), 30);
    // Nothing is placed by a guess of its width any more.
    assert!(r.report.notes.iter().all(|n| !n.reason.contains("tahmin")), "{:?}", r.report.notes);
}

/// A leader's vertices, note, height, turn, arrowhead and mask.
type Leader<'a> = (Vec<Vec2>, Option<&'a str>, f64, f64, Option<LeaderArrow>, bool);

fn leader(e: &Entity) -> Leader<'_> {
    let Entity::Leader(l) = e else {
        panic!("not a leader: {e:?}")
    };
    (l.pts.clone(), l.text.as_deref(), l.height, l.rotation, l.arrow, l.mask)
}

/// The leaders of fixtures/formats/v1/leaders.dxf (docs/adr/0146 §8): a
/// LEADER's MTEXT, after it or before it in the file, is its note (the
/// first line; the others stay texts); a hookline's end gives way to the
/// leader's own landing; the arrowhead comes from the arrow block's name
/// (the style's, or the entity's own DSTYLE data); without a note the
/// height is 40, else the style's arrow; a MULTILEADER is its first line
/// with its content as the note.
#[test]
fn leaders_come_with_their_notes_arrowheads_and_heights() {
    let r = read("leaders.dxf");
    let e = &r.entities;
    let dot = Some(LeaderArrow::Dot);
    let open = Some(LeaderArrow::Open);
    // A: its MTEXT after it; the hookline's end (8.5, 5) gives way to the leader's own landing.
    assert_eq!(
        leader(&e[0]),
        (vec![v(0.0, 0.0), v(6.0, 5.0)], Some("Mevcut bina"), 2.5, 0.0, None, false)
    );
    // B: its MTEXT before it, at 30° over a background; Harita's arrow block is _Dot.
    let (pts, note, height, turn, arrow, mask) = leader(&e[1]);
    assert_eq!(
        (pts, note, height, arrow, mask),
        (vec![v(40.0, -10.0), v(36.0, -6.0)], Some("Ø150 PVC"), 2.0, dot, true)
    );
    assert!((turn - 30.0).abs() < 1e-9, "{turn}");
    // Its second line stays a text, under the note as it was under the first line, 2 × 5/3
    // along the note's up; the note 2.5 heights from the last vertex, its landing to the left.
    let Entity::Text(dn) = &e[2] else {
        panic!("{:?}", e[2])
    };
    assert_eq!(
        (dn.text.as_str(), dn.height, dn.mask, dn.align, dn.rotation),
        ("DN 150", 2.0, true, Some(TextAlign::MiddleRight), turn)
    );
    let (s, c) = (sin(turn * PI / 180.0), cos(turn * PI / 180.0));
    let note = v(36.0 - 5.0 * c, -6.0 - 5.0 * s);
    let gap = 2.0 * 5.0 / 3.0;
    assert!(near(dn.p, v(note.x + s * gap, note.y - c * gap)), "{:?}", dn.p);
    // C: neither an annotation nor an arrowhead; no 40: Harita's arrow, 1.8 × 2.
    assert_eq!(
        leader(&e[3]),
        (vec![v(60.0, 0.0), v(64.0, 4.0)], None, 3.6, 0.0, Some(LeaderArrow::None), false)
    );
    // D: a spline path read straight; its MTEXT's formatting dropped.
    assert_eq!(
        leader(&e[4]),
        (vec![v(80.0, 0.0), v(84.0, 3.0), v(88.0, 3.0)], Some("Vana"), 1.5, 0.0, None, false)
    );
    // F: the MTEXT it names is not in the file.
    assert_eq!(
        leader(&e[5]),
        (vec![v(100.0, -20.0), v(104.0, -16.0)], None, 2.0, 0.0, None, false)
    );
    // G: its own DSTYLE data gives _Open and an arrow of 3 (Standard's scale, 1).
    assert_eq!(
        leader(&e[6]),
        (vec![v(120.0, -20.0), v(116.0, -16.0)], None, 3.0, 0.0, open, false)
    );
    // K: _ArchTick is no arrowhead KentOS has: filled.
    assert_eq!(
        leader(&e[7]),
        (vec![v(140.0, -20.0), v(144.0, -16.0)], None, 1.0, 0.0, None, false)
    );
    // H: its first leader line to its leader's last point; the content's first line its note.
    assert_eq!(
        leader(&e[8]),
        (vec![v(100.0, 0.0), v(104.0, 4.0), v(106.0, 6.0)], Some("Ada 101"), 2.0, 0.0, open, true)
    );
    let Entity::Text(parsel) = &e[9] else {
        panic!("{:?}", e[9])
    };
    // Under its note (106 + 2.5 × 2, 6), one line pitch down, aligned as the note.
    assert_eq!(
        (parsel.text.as_str(), parsel.height, parsel.align),
        ("Parsel 5", 2.0, Some(TextAlign::MiddleLeft))
    );
    assert!(near(parsel.p, v(111.0, 6.0 - 2.0 * 5.0 / 3.0)), "{:?}", parsel.p);
    // I: its block content is not taken; its height is the context's arrow, 1.5.
    assert_eq!(
        leader(&e[10]),
        (vec![v(120.0, 0.0), v(124.0, 4.0), v(126.0, 4.0)], None, 1.5, 0.0, None, false)
    );
    assert!(matches!(&e[11], Entity::Insert(_)));
    assert_eq!(e.len(), 12);
    // VANA's leader took the MTEXT that came before it in the block.
    let vana = r.blocks.iter().find(|b| b.name == "VANA").expect("VANA");
    assert_eq!(vana.entities.len(), 2);
    assert_eq!(
        leader(&vana.entities[1]),
        (vec![v(1.0, 0.0), v(3.0, 2.0)], Some("V"), 0.5, 0.0, None, false)
    );
    assert_eq!(
        (count(&r, "leader"), count(&r, "text"), count(&r, "insert")),
        (9, 2, 1)
    );
    let said = |what: &str, part: &str| {
        r.report
            .notes
            .iter()
            .any(|n| n.what == what && n.reason.contains(part))
    };
    assert!(said("Kılavuz (LEADER)", "ilk satırı kılavuzun notu oldu"));
    assert!(said("Kılavuz (LEADER)", "eğri yolu"));
    assert!(said("Kılavuz (LEADER)", "bağlı notu (340) dosyada yok"));
    assert!(said("Kılavuz (LEADER)", "“_ArchTick” KentOS'ta yok"));
    assert!(said("Çoklu kılavuz (MULTILEADER)", "öbür 1 ok çizgisi alınmadı"));
    assert!(said("Çoklu kılavuz (MULTILEADER)", "blok içeriği alınmadı"));
    assert!(said("Çoklu kılavuz (MULTILEADER)", "eğri ok çizgisi"));
    assert!(r.report.skipped.is_empty(), "{:?}", r.report.skipped);
    // Blokları patlat: VANA's leader comes into the drawing with its note, where the insert puts it.
    let o = opened("leaders.dxf");
    let opened: Vec<Leader<'_>> = o
        .entities
        .iter()
        .filter(|e| matches!(e, Entity::Leader(l) if l.text.as_deref() == Some("V")))
        .map(leader)
        .collect();
    assert_eq!(
        opened,
        vec![(vec![v(161.0, 0.0), v(163.0, 2.0)], Some("V"), 0.5, 0.0, None, false)]
    );
    assert_eq!((count(&o, "leader"), count(&o, "text")), (10, 2));
}

/// Another program's new dimensions (docs/adr/0147 §8;
/// `fixtures/formats/v1/dimension-kinds.dxf`, written as AutoCAD 2007 writes
/// them): the ordinates measured from the origin, the arc length and the
/// jogged radius come in as KentOS's own, as high as their style's text
/// (DIMTXT 2 × DIMSCALE 1.5), the arc length's value over the background by
/// its own DIMTFILL; an ordinate measured from another origin is its block,
/// and said; another program's aligned dimension is its block, as before.
#[test]
fn another_program_s_new_dimensions_come_in_as_kentos_s_own() {
    use kentos_contracts::DimensionStyle;
    let r = read("dimension-kinds.dxf");
    let dims: Vec<_> = r
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Dimension(d) => Some(d),
            _ => None,
        })
        .collect();
    let [east, north, arc, jogged] = dims.as_slice() else {
        panic!("four dimensions: {dims:?}")
    };
    // The east (AutoCAD's X type, bit 64): its point, its line's end; no offset, the style's height.
    assert_eq!(
        (east.style, east.angle, east.a, east.b, east.offset, east.height),
        (
            Some(DimensionStyle::Ordinate),
            Some(0.0),
            v(452310.0, 4412320.0),
            v(452310.0, 4412345.0),
            0.0,
            3.0
        )
    );
    assert_eq!((east.text.as_deref(), east.mask, east.base.layer_id.as_str()), (None, false, "OLCU"));
    // The north, with its own words.
    assert_eq!(
        (north.angle, north.b, north.text.as_deref()),
        (Some(90.0), v(452275.0, 4412320.0), Some("X=4412320.00"))
    );
    // The arc from the east to the north about (452350, 4412300), its dimension arc 3 m out, masked.
    assert_eq!(
        (arc.style, arc.a, arc.b, arc.c, arc.mask),
        (
            Some(DimensionStyle::ArcLength),
            v(452360.0, 4412300.0),
            v(452350.0, 4412310.0),
            Some(v(452350.0, 4412300.0)),
            true
        )
    );
    assert!((arc.offset - 3.0).abs() < 1e-9, "{arc:?}");
    // The jogged radius: the true centre, the point on the arc, the centre shown; its jog 8 m
    // from the centre shown; "R<>" is the measured value.
    assert_eq!(
        (jogged.style, jogged.a, jogged.b, jogged.c, jogged.text.as_deref()),
        (
            Some(DimensionStyle::Jogged),
            v(452400.0, 4412100.0),
            v(452475.244432, 4412306.732377),
            Some(v(452465.584951, 4412288.964585)),
            None
        )
    );
    assert!((jogged.offset - 8.0).abs() < 1e-5, "{jogged:?}");
    // The ordinate from another origin and the aligned one: their blocks' lines and values.
    assert_eq!((count(&r, "line"), count(&r, "text")), (4, 2));
    assert!(r.report.notes.iter().any(|n| n.what == "Ölçü (DIMENSION)"
        && n.reason.starts_with("koordinat ölçüsünün başlangıcı (0, 0) değil")
        && n.count == 1));
}
