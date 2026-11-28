//! The DXF reader on what another program writes for a text's baseline and on what a DWG's bridge brings: wipeouts,
//! images, circles seen edge-on, OLE frames, empty texts and hatch boundaries in pieces (docs/adr/8046 §3, 9227–9231;
//! moved out of dxf.rs, which is over the size limit, CLAUDE.md §8). Expected values are worked out by hand.

use kentos_contracts::{DxfReadOptions, Entity};
use kentos_formats::dxf;

use super::dxf::{dxf_of, groups, skipped};

/// An aligned text's baseline stands where another program draws it (docs/adr/8046 §3): a middle-centred Arial text
/// of 40 = 2.5 at 11 = (0, 0) has its baseline half the file's height (its capitals) under 11 there, at −1.25; KentOS
/// puts its point half its em over its baseline, so the point is raised to −1.25 + ½ × 2.5 / (1466/2048). A top
/// aligned one, the whole height; a baseline one does not move; in an SHX style none moves.
#[test]
fn an_aligned_text_s_baseline_is_where_another_program_draws_it() {
    let arial = 1466.0 / 2048.0;
    let tables = groups(&[
        (0, "TABLE"),
        (2, "STYLE"),
        (0, "STYLE"),
        (2, "ARIAL"),
        (70, "0"),
        (40, "0"),
        (41, "1"),
        (50, "0"),
        (71, "0"),
        (42, "2.5"),
        (3, "arial.ttf"),
        (4, ""),
        (0, "STYLE"),
        (2, "SHX"),
        (70, "0"),
        (40, "0"),
        (41, "1"),
        (50, "0"),
        (71, "0"),
        (42, "2.5"),
        (3, "romans.shx"),
        (4, ""),
        (0, "ENDTAB"),
    ]);
    let text = |style: &'static str, h: &'static str, v: &'static str| {
        vec![
            (0, "TEXT"),
            (8, "0"),
            (10, "0"),
            (20, "0"),
            (40, "2.5"),
            (1, "Ada"),
            (7, style),
            (72, h),
            (11, "0"),
            (21, "0"),
            (73, v),
        ]
    };
    let mut entities = Vec::new();
    entities.extend(text("ARIAL", "1", "2")); // middle centre
    entities.extend(text("ARIAL", "1", "3")); // top centre
    entities.extend(text("ARIAL", "1", "0")); // baseline centre
    entities.extend(text("SHX", "1", "2"));
    let r = dxf::read(
        &dxf_of(&[("TABLES", tables), ("ENTITIES", groups(&entities))]),
        &DxfReadOptions::default(),
    )
    .expect("read");
    let ys: Vec<(f64, f64)> = r
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Text(t) => Some((t.p.y, t.height)),
            _ => None,
        })
        .collect();
    let em = 2.5 / arial;
    let near = |a: f64, b: f64| (a - b).abs() <= 1e-12;
    // Each one's KentOS baseline: its point less its alignment's share of its em.
    assert!(
        near(ys[0].0 - 0.5 * ys[0].1, -0.5 * 2.5) && near(ys[0].1, em),
        "{ys:?}"
    );
    assert!(near(ys[1].0 - ys[1].1, -2.5), "{ys:?}");
    assert_eq!(ys[2].0, 0.0);
    assert_eq!(ys[3], (0.0, 2.5), "SHX: as the file says");
}

/// A WIPEOUT is a mask (docs/adr/0310, 9227): an area of the drawing's background, its frame as the drawing's
/// WIPEOUTVARIABLES say. Its clip in the DXF reference's image space, a point (x, y) at
/// `insert + (x + ½)·U + (½ − y)·V`: the SİVAS AVM DWG's grid boxes, U and V turned 41.2° and millions of units long,
/// the clip near a corner of the unit square, are rectangles of 17.15 × 5 m in their block (millimetres there). A
/// rectangular clip gives two corners.
#[test]
fn a_wipeout_is_a_mask_where_its_clip_says() {
    let g = |code: i32, value: &str| (code, value.to_string());
    let mut entities = vec![
        g(0, "WIPEOUT"),
        g(5, "934323"),
        g(8, "A_ANNO_GRID_BOX_TXT"),
        g(100, "AcDbEntity"),
        g(100, "AcDbWipeout"),
        g(90, "0"),
        g(10, "7443.174011286348"),
        g(20, "-13502.374265313614"),
        g(30, "0.0"),
        g(11, "3550849.9940145127"),
        g(21, "3109326.9238783014"),
        g(31, "0.0"),
        g(12, "-3109326.9238783014"),
        g(22, "3550849.9940145127"),
        g(32, "0.0"),
        g(13, "1.0"),
        g(23, "1.0"),
        g(70, "7"),
        g(71, "2"),
        g(91, "5"),
    ];
    for (x, y) in [
        ("-0.5", "0.4976061783798207"),
        ("-0.4993017872360454", "0.49680881970827984"),
        ("-0.49656804399121646", "0.49920264132838754"),
        ("-0.4972662567552838", "0.5"),
        ("-0.5", "0.4976061783798207"),
    ] {
        entities.extend([g(14, x), g(24, y)]);
    }
    entities.extend([
        g(0, "WIPEOUT"),
        g(5, "A1"),
        g(8, "0"),
        g(100, "AcDbEntity"),
        g(100, "AcDbWipeout"),
        g(10, "100"),
        g(20, "200"),
        g(30, "0"),
        g(11, "4"),
        g(21, "0"),
        g(31, "0"),
        g(12, "0"),
        g(22, "2"),
        g(32, "0"),
        g(13, "1"),
        g(23, "1"),
        g(71, "1"),
        g(91, "2"),
        g(14, "-0.5"),
        g(24, "-0.5"),
        g(14, "0.5"),
        g(24, "0.5"),
    ]);
    let objects = vec![
        g(0, "WIPEOUTVARIABLES"),
        g(5, "11D5D6"),
        g(100, "AcDbWipeoutVariables"),
        g(70, "0"),
    ];
    let options = DxfReadOptions {
        unit: Some(kentos_contracts::DrawingUnit::M),
        ..Default::default()
    };
    let r = dxf::read(
        &dxf_of(&[("ENTITIES", entities.clone()), ("OBJECTS", objects)]),
        &options,
    )
    .expect("read");
    let masks: Vec<_> = r
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Polygon(p) => Some(p),
            _ => None,
        })
        .collect();
    assert_eq!(masks.len(), 2, "{:?}", r.report.skipped);
    for m in &masks {
        assert_eq!(
            m.base.look.wipeout,
            Some(kentos_contracts::look::Wipeout { frame: false }),
            "frames off"
        );
    }
    // The grid box: a rectangle from (0, 0) to (17 150.286, −5 002.273), its first point (0, −5 002.273).
    let grid = &masks[0].pts;
    let want = [
        (0.0, -5002.273),
        (0.0, 0.0),
        (17150.286, 0.0),
        (17150.286, -5002.273),
    ];
    assert_eq!(grid.len(), 4, "the closing point dropped");
    for (p, (x, y)) in grid.iter().zip(want) {
        assert!(
            (p.x - x).abs() < 1e-3 && (p.y - y).abs() < 1e-3,
            "{p:?} ≠ ({x}, {y})"
        );
    }
    // The rectangle: two corners of the unit square, U 4 and V 2 from (100, 200).
    let rect: Vec<(f64, f64)> = masks[1].pts.iter().map(|p| (p.x, p.y)).collect();
    assert_eq!(
        rect,
        vec![
            (100.0, 202.0),
            (104.0, 202.0),
            (104.0, 200.0),
            (100.0, 200.0)
        ]
    );
    assert!(skipped(&r, "WIPEOUT").is_none());
    // No WIPEOUTVARIABLES: the format's default, the frame drawn.
    let r = dxf::read(&dxf_of(&[("ENTITIES", entities)]), &options).expect("read");
    let Some(Entity::Polygon(p)) = r.entities.first() else {
        panic!("{:?}", r.entities)
    };
    assert_eq!(
        p.base.look.wipeout,
        Some(kentos_contracts::look::Wipeout { frame: true })
    );
}

/// An IMAGE's file is not in the drawing (docs/adr/9227): the SİVAS AVM DWG's logo, 2 409 × 3 435 pixels of 1.7118
/// units, comes as its frame, a closed line where it was, and the report names its file (its IMAGEDEF's 1).
#[test]
fn an_image_is_its_frame_and_its_file_is_named() {
    let g = |code: i32, value: &str| (code, value.to_string());
    let entities = vec![
        g(0, "IMAGE"),
        g(5, "92E9D8"),
        g(8, "A_SHET"),
        g(100, "AcDbEntity"),
        g(100, "AcDbRasterImage"),
        g(10, "1000"),
        g(20, "2000"),
        g(30, "0"),
        g(11, "1.7117828025000665"),
        g(21, "0"),
        g(31, "0"),
        g(12, "0"),
        g(22, "1.7117828025000665"),
        g(32, "0"),
        g(13, "2409"),
        g(23, "3435"),
        g(340, "146B6"),
        g(70, "7"),
        g(280, "0"),
        g(71, "1"),
        g(91, "2"),
        g(14, "-0.5"),
        g(24, "-0.5"),
        g(14, "2408.5"),
        g(24, "3434.5"),
    ];
    let objects = vec![
        g(0, "IMAGEDEF"),
        g(5, "146B6"),
        g(100, "AcDbRasterImageDef"),
        g(1, "DDS LOGO.jpg"),
    ];
    let options = DxfReadOptions {
        unit: Some(kentos_contracts::DrawingUnit::M),
        ..Default::default()
    };
    let r = dxf::read(
        &dxf_of(&[("ENTITIES", entities.clone()), ("OBJECTS", objects)]),
        &options,
    )
    .expect("read");
    let [Entity::Polyline(frame)] = r.entities.as_slice() else {
        panic!("{:?}", r.entities)
    };
    assert!(frame.closed);
    let (w, h) = (2409.0 * 1.7117828025000665, 3435.0 * 1.7117828025000665);
    let want = [
        (1000.0, 2000.0 + h),
        (1000.0 + w, 2000.0 + h),
        (1000.0 + w, 2000.0),
        (1000.0, 2000.0),
    ];
    for (p, (x, y)) in frame.pts.iter().zip(want) {
        assert!(
            (p.x - x).abs() < 1e-6 && (p.y - y).abs() < 1e-6,
            "{p:?} ≠ ({x}, {y})"
        );
    }
    let note = r
        .report
        .notes
        .iter()
        .find(|n| n.what == "Görüntü (IMAGE)")
        .expect("said");
    assert!(note.reason.contains("“DDS LOGO.jpg”"), "{}", note.reason);
    assert!(skipped(&r, "IMAGE").is_none());

    // Its definition naming no file: the frame still shows where it was, and the report says the file is not named.
    let objects = vec![
        g(0, "IMAGEDEF"),
        g(5, "146B6"),
        g(100, "AcDbRasterImageDef"),
        g(1, " "),
    ];
    let r = dxf::read(
        &dxf_of(&[("ENTITIES", entities), ("OBJECTS", objects)]),
        &options,
    )
    .expect("read");
    let [Entity::Polyline(unnamed)] = r.entities.as_slice() else {
        panic!("{:?}", r.entities)
    };
    assert_eq!(unnamed.pts, frame.pts);
    let note = r
        .report
        .notes
        .iter()
        .find(|n| n.what == "Görüntü (IMAGE)")
        .expect("said");
    assert!(note.reason.contains("adlandırılmamış"), "{}", note.reason);
    assert!(skipped(&r, "IMAGE").is_none());
}

/// A circle or arc seen edge on (docs/adr/9231): its normal along x puts it in a plane that holds the view's direction,
/// and a plan shows it as a segment. It was left out ("görüş doğrultusuna dik"): TEKNOKENT MİMARİ's 28 circles. Its
/// object coordinates' x runs along the world's y and their y up (the arbitrary axis rule), so the centre (0, 5) at
/// elevation 3 is (3, 0) in plan and the radius 2 runs along y; the quarter arc from 0° to 90° is its upper half. One
/// of no radius is left out for that.
#[test]
fn a_circle_seen_edge_on_is_the_segment_it_is_in_plan() {
    let g = |code: i32, value: &str| (code, value.to_string());
    let edge_on = |kind: &str, r: &str, angles: Option<(&str, &str)>| {
        let mut e = vec![
            g(0, kind),
            g(8, "0"),
            g(10, "0"),
            g(20, "5"),
            g(30, "3"),
            g(40, r),
        ];
        if let Some((a0, a1)) = angles {
            e.extend([g(50, a0), g(51, a1)]);
        }
        e.extend([g(210, "1"), g(220, "0"), g(230, "0")]);
        e
    };
    let mut entities = edge_on("CIRCLE", "2", None);
    entities.extend(edge_on("ARC", "2", Some(("0", "90"))));
    entities.extend(edge_on("CIRCLE", "0", None));
    let options = DxfReadOptions {
        unit: Some(kentos_contracts::DrawingUnit::M),
        ..Default::default()
    };
    let r = dxf::read(&dxf_of(&[("ENTITIES", entities)]), &options).expect("read");
    let lines: Vec<[(f64, f64); 2]> = r
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Line(l) => Some([(l.a.x, l.a.y), (l.b.x, l.b.y)]),
            _ => None,
        })
        .collect();
    let close =
        |p: (f64, f64), q: (f64, f64)| (p.0 - q.0).abs() < 1e-12 && (p.1 - q.1).abs() < 1e-12;
    assert_eq!(lines.len(), 2, "{:?} {:?}", r.entities, r.report.skipped);
    assert!(
        close(lines[0][0], (3.0, -2.0)) && close(lines[0][1], (3.0, 2.0)),
        "{lines:?}"
    );
    assert!(
        close(lines[1][0], (3.0, 0.0)) && close(lines[1][1], (3.0, 2.0)),
        "{lines:?}"
    );
    let said = r
        .report
        .notes
        .iter()
        .find(|n| n.what == "CIRCLE")
        .expect("said");
    assert!(
        said.reason.contains("doğru parçası olarak alındı"),
        "{}",
        said.reason
    );
    let point = r
        .report
        .skipped
        .iter()
        .find(|s| s.what == "CIRCLE")
        .expect("the point is left out");
    assert_eq!(
        (point.count, point.reason.as_str()),
        (1, "yarıçapı sıfır ya da negatif")
    );
}

/// An embedded document is its frame, said (docs/adr/9231): OLE2FRAME's corners, upper left (10) and lower right
/// (11), give a closed outline where it lies; its content was another program's data and stays out. An empty text
/// draws nothing and loses nothing: a note, not "not taken".
#[test]
fn an_ole_frame_is_its_outline_and_an_empty_text_a_note() {
    let entities = groups(&[
        (0, "OLE2FRAME"),
        (8, "0"),
        (70, "2"),
        (10, "10"),
        (20, "8"),
        (30, "0"),
        (11, "16"),
        (21, "4"),
        (31, "0"),
        (71, "2"),
        (0, "TEXT"),
        (8, "0"),
        (10, "0"),
        (20, "0"),
        (40, "2.5"),
        (1, ""),
    ]);
    let options = DxfReadOptions {
        unit: Some(kentos_contracts::DrawingUnit::M),
        ..Default::default()
    };
    let r = dxf::read(&dxf_of(&[("ENTITIES", entities)]), &options).expect("read");
    let Some(Entity::Polyline(p)) = r.entities.first() else {
        panic!("{:?} {:?}", r.entities, r.report.skipped)
    };
    assert!(p.closed);
    let outline: Vec<(f64, f64)> = p.pts.iter().map(|q| (q.x, q.y)).collect();
    assert_eq!(
        outline,
        vec![(10.0, 8.0), (16.0, 8.0), (16.0, 4.0), (10.0, 4.0)]
    );
    assert_eq!(r.entities.len(), 1);
    assert!(r.report.skipped.is_empty(), "{:?}", r.report.skipped);
    let said = |what: &str| {
        r.report
            .notes
            .iter()
            .find(|n| n.what == what)
            .map(|n| n.reason.clone())
    };
    assert!(
        said("OLE nesnesi (OLE2FRAME)")
            .is_some_and(|w| w.contains("yerini gösteren çerçevesi alındı"))
    );
    assert_eq!(
        said("Yazı (TEXT)").as_deref(),
        Some("boş; çizilecek yazısı yok")
    );
}

/// A hatch whose boundary is pieces that close only together (docs/adr/9230): paths flagged not closed (92 bit 32),
/// as the YILTAŞ DWGs' 130 column hatches are, four paths of one line each round a square, one of them the other way
/// round. Before, each path was a ring of two points and the hatch was left out ("sınırı okunamadı").
#[test]
fn a_boundary_of_pieces_is_joined_into_its_ring() {
    let g = |code: i32, value: &str| (code, value.to_string());
    let mut e = vec![
        g(0, "HATCH"),
        g(5, "C61473"),
        g(8, "KOLON TARAMA"),
        g(100, "AcDbEntity"),
        g(100, "AcDbHatch"),
        g(10, "0"),
        g(20, "0"),
        g(30, "0"),
        g(210, "0"),
        g(220, "0"),
        g(230, "1"),
        g(2, "ANSI31"),
        g(70, "0"),
        g(71, "0"),
        g(91, "4"),
    ];
    let sides = [
        ((0.0, 4.0), (5.0, 4.0)),
        ((5.0, 4.0), (5.0, 0.0)),
        ((0.0, 0.0), (5.0, 0.0)),
        ((0.0, 0.0), (0.0, 4.0)),
    ];
    for (a, b) in sides {
        e.extend([g(92, "33"), g(93, "1"), g(72, "1")]);
        for (code, value) in [(10, a.0), (20, a.1), (11, b.0), (21, b.1)] {
            e.push(g(code, &value.to_string()));
        }
        e.push(g(97, "0"));
    }
    e.extend([
        g(75, "1"),
        g(76, "1"),
        g(52, "0"),
        g(41, "1"),
        g(77, "0"),
        g(78, "0"),
        g(98, "0"),
    ]);
    let options = DxfReadOptions {
        unit: Some(kentos_contracts::DrawingUnit::M),
        ..Default::default()
    };
    let r = dxf::read(&dxf_of(&[("ENTITIES", e)]), &options).expect("read");
    let hatches: Vec<_> = r
        .entities
        .iter()
        .filter_map(|x| match x {
            Entity::Hatch(h) => Some(h),
            _ => None,
        })
        .collect();
    assert_eq!(hatches.len(), 1, "{:?}", r.report.skipped);
    let mut ring: Vec<(f64, f64)> = hatches[0].ring.iter().map(|p| (p.x, p.y)).collect();
    ring.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(ring, vec![(0.0, 0.0), (0.0, 4.0), (5.0, 0.0), (5.0, 4.0)]);
    assert!(hatches[0].holes.is_none());
    assert!(
        r.report.skipped.iter().all(|s| !s.what.contains("HATCH")),
        "{:?}",
        r.report.skipped
    );
}
