//! The NCZ reader on the fixtures of `fixtures/formats/v1/ncz/`, which
//! `scripts/fixtures/ncz_reference.py` writes block by block from their
//! offsets (the browser's test, `apps/web/src/io/ncz.wasm.test.ts`, reads the
//! same files through the NCZ module). Expected values are worked out by hand
//! from that script: what each block holds, where, and on which layer; the
//! origin is its `N0, E0` (northing 4 448 000, easting 421 000), and a file's
//! northing is the app's y, its easting the app's x.

use kentos_contracts::{CrsSource, Entity, ImportResult, NczReadOptions, TextAlign, TextEntity};
use kentos_formats::watch::{Quiet, STOPPED, Steps};

const N0: f64 = 4_448_000.0;
const E0: f64 = 421_000.0;

fn fixture(name: &str) -> Vec<u8> {
    let path = format!("{}/../../../fixtures/formats/v1/ncz/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn read(name: &str) -> ImportResult {
    kentos_ncz::read(&fixture(name), &NczReadOptions::default(), &mut Quiet).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn fact<'a>(r: &'a ImportResult, label: &str) -> Option<&'a str> {
    r.report.source.iter().find(|f| f.label == label).map(|f| f.value.as_str())
}

fn counts(r: &ImportResult) -> Vec<(&str, u32)> {
    r.report.counts.iter().map(|(k, n)| (k.as_str(), *n)).collect()
}

fn layers(r: &ImportResult) -> Vec<(&str, &str, u32)> {
    r.layers.iter().map(|l| (l.name.as_str(), l.color.as_str(), l.count)).collect()
}

fn attr<'a>(e: &'a Entity, key: &str) -> Option<&'a str> {
    e.base().attrs.get(key).map(String::as_str)
}

#[test]
fn every_record_type_on_its_layer_with_the_files_system() {
    let r = read("01-her-tur.ncz");
    assert_eq!(fact(&r, "Sürüm"), Some("Netcad 5.2.0.1035N"));
    // MPROJ says TM, ITRF, zone 39; TILED_XML names it: EPSG:5257, said, never reprojected.
    let crs = r.declared_crs.as_ref().expect("a declared system");
    assert_eq!((crs.srid, crs.source), (Some(5257), CrsSource::Ncz));
    // Five points (two named, the symbol, the block reference, the one in the container),
    // two lines, a circle, two arcs, two texts, four closed shapes (the closed polyline,
    // the rotated box, the map sheet, the triangle) and two open polylines.
    assert_eq!(
        counts(&r),
        [("arc", 2), ("circle", 1), ("line", 2), ("point", 5), ("polygon", 4), ("polyline", 2), ("text", 2)]
    );
    // The table's blank name is no layer (the objects of index 4 fall on the next name,
    // as the reference plugin reads the table); index 6 is past the table.
    assert_eq!(
        layers(&r),
        [
            ("PARSEL", "#FF0000", 5),
            ("YAZI", "ink", 5),
            ("İMAR_ŞERİT", "#008000", 4),
            ("PAFTA", "ink", 3),
            ("KATMAN_6", "#28323C", 1),
        ]
    );
    let Entity::Point(p) = &r.entities[0] else { panic!("{:?}", r.entities[0]) };
    assert_eq!((p.p.x, p.p.y, p.z, p.base.label.as_deref()), (E0, N0, Some(1088.5), Some("1284")));
    let Entity::Circle(c) = &r.entities[3] else { panic!("{:?}", r.entities[3]) };
    assert_eq!((c.c.x, c.c.y, c.r, c.base.color.as_deref()), (E0 + 50.0, N0 + 50.0, 12.5, Some("#0000FF")));
    // The extended arc's angles are degrees (past a turn in radians): 30° and 120°.
    let Entity::Arc(a) = &r.entities[5] else { panic!("{:?}", r.entities[5]) };
    assert_eq!((a.r, a.a0, a.a1), (5.0, 30f64.to_radians(), 120f64.to_radians()));
    let Entity::Text(t) = &r.entities[6] else { panic!("{:?}", r.entities[6]) };
    assert_eq!((t.text.as_str(), t.height), ("ADA 101 PARSEL ş", 2.5));
    assert!((t.rotation - 30.0).abs() < 1e-6, "{}", t.rotation);
    // The map sheet is its outline, named by its sheet: its box of round local coordinates is
    // no grid cell's, so it stays the box the file keeps, and the report says so.
    let sheet = r.entities.iter().find(|e| attr(e, "Pafta").is_some()).expect("the sheet");
    assert_eq!(attr(sheet, "Pafta"), Some("H40-D-07-B-1-C"));
    let skipped: Vec<&str> = r.report.skipped.iter().map(|s| s.what.as_str()).collect();
    assert_eq!(skipped, ["NCZ türü 8", "NCZ türü 14", "Pafta çerçevesinin gerçek biçimi"]);
    // One record gives a pen, 2 tenths of a millimetre: the line in the container, and only it
    // takes its own weight; the others are drawn in their layer's (docs/adr/0139).
    let weighed: Vec<(usize, f64)> = r
        .entities
        .iter()
        .enumerate()
        .filter_map(|(i, e)| e.base().line_weight.map(|w| (i, w)))
        .collect();
    assert_eq!(weighed, [(16, 0.2)]);
    assert!(matches!(r.entities[16], Entity::Line(_)));
    // What each layer holds, without walking its objects (docs/adr/0138).
    let parsel = &r.layers[0];
    assert_eq!(parsel.kinds.iter().map(|(k, n)| (k.as_str(), *n)).collect::<Vec<_>>(), [("line", 2), ("point", 3)]);
    let b = parsel.bounds.as_ref().expect("a box");
    assert_eq!((b.min_x, b.min_y, b.max_x, b.max_y), (E0, N0, E0 + 702.0, N0 + 702.0));
}

#[test]
fn a_smart_objects_frame_is_drawn_on_top_and_its_grid_marks_left_out() {
    let r = read("02-akilli-nesne.ncz");
    assert_eq!(counts(&r), [("point", 2), ("polygon", 2)]);
    // The smart layer comes first: the top of the layer tree is drawn last, over the plan.
    assert_eq!(layers(&r), [("AKILLI", "ink", 3), ("0", "ink", 1)]);
    let frames: Vec<Option<&str>> = r.entities.iter().filter(|e| e.kind() == "polygon").map(|e| attr(e, "Akıllı nesne")).collect();
    assert_eq!(frames, [Some("BASIC"), Some("GRID_A7")]);
    // The unrotated frame: 80 m east, 40 m north of its corner.
    let Entity::Polygon(g) = &r.entities[1] else { panic!("{:?}", r.entities[1]) };
    let corners: Vec<(f64, f64)> = g.pts.iter().map(|p| (p.x, p.y)).collect();
    assert_eq!(corners, [(E0, N0 + 500.0), (E0 + 80.0, N0 + 500.0), (E0 + 80.0, N0 + 540.0), (E0, N0 + 540.0)]);
    // The S0 on layer 0 is the frame's grid mark: left out and said; the other two stay.
    let marks: Vec<(&str, Option<&str>)> = r
        .entities
        .iter()
        .filter(|e| e.kind() == "point")
        .map(|e| (e.base().layer_id.as_str(), attr(e, "Sembol")))
        .collect();
    assert_eq!(marks, [("AKILLI", Some("S0")), ("0", Some("S5"))]);
    let note = r.report.notes.iter().find(|n| n.what == "Izgara işareti (S0)").expect("the note");
    assert_eq!(note.count, 1);
}

#[test]
fn a_layer_is_named_by_the_table_even_when_the_table_comes_later() {
    let r = read("03-gec-tablolar.ncz");
    assert_eq!(layers(&r), [("İKİ", "ink", 2), ("KATMAN_7", "ink", 1)]);
    let names: Vec<&str> = r.entities.iter().map(|e| e.base().layer_id.as_str()).collect();
    assert_eq!(names, ["İKİ", "KATMAN_7", "İKİ"]);
}

#[test]
fn broken_records_are_left_out_and_counted_by_kind() {
    let r = read("04-bozuk-kayitlar.ncz");
    assert!(r.entities.is_empty() && r.layers.is_empty());
    let mut skipped: Vec<(&str, u32)> = r.report.skipped.iter().map(|s| (s.what.as_str(), s.count)).collect();
    skipped.sort_unstable();
    // Three points (too short, not a number, a million kilometres out), a text with no
    // height, two polylines (one point; a curve that goes bad), a sheet with no size, a
    // triangle with no area and a box turned by no number.
    assert_eq!(
        skipped,
        [("Kapalı alan", 1), ("Nokta", 3), ("Pafta", 1), ("Yazı", 1), ("Çoklu çizgi", 2), ("Üçgen", 1)]
    );
}

#[test]
fn attribute_tables_are_read_and_said_when_no_object_takes_them() {
    let r = read("05-oznitelik-tablolari.ncz");
    assert_eq!(counts(&r), [("point", 1)]);
    let tables = r.report.skipped.iter().find(|s| s.what == "Öznitelik tablosu (@TAB)").expect("the tables");
    assert!(tables.reason.starts_with("2 tablo (@TAB1: 2 satır, @TAB23: 2 satır)"), "{}", tables.reason);
}

#[test]
fn a_cut_file_gives_the_whole_records_before_the_cut() {
    let r = read("06-kesik.ncz");
    assert_eq!(counts(&r), [("arc", 2), ("circle", 1), ("line", 1), ("point", 2), ("text", 2)]);
    assert_eq!(r.declared_crs.as_ref().and_then(|d| d.srid), Some(5257));
}

#[test]
fn netcad_8_smart_objects_become_their_symbols_on_the_layers_drawn_on_top() {
    let r = read("07-akilli-nesneler.ncz");
    assert_eq!(fact(&r, "Sürüm"), Some("Netcad 8.5.6.1095"));
    assert_eq!(
        layers(&r),
        [
            ("SM_YERLESIM", "#C80000", 16),
            ("SM_YAPILASMA", "#0000C8", 10),
            ("SM_YOL", "#007800", 9),
            ("SM_NOT", "#505050", 5),
            ("SM_FONKADI", "#780078", 1),
        ]
    );
    let (n, e) = (N0 + 1000.0, E0 + 1000.0);
    // The first settlement: its circle, 10 × its size, and its values; the rear yard
    // said empty (chkArkaIsNull) is not one of them.
    let Entity::Circle(c) = &r.entities[0] else { panic!("{:?}", r.entities[0]) };
    assert_eq!((c.c.x, c.c.y, c.r), (e, n, 5.0));
    let values: Vec<(&str, &str)> = c.base.attrs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    assert_eq!(
        values,
        [("Akıllı nesne", "Yerleşim"), ("Kat", "3"), ("Nizam", "AYRIK"), ("Yan bahçe", "3"), ("Ön bahçe", "5")]
    );
    // The first road's circle: its size is a float32 in the file (1.275 as written).
    let road = r.entities.iter().find(|x| attr(x, "Akıllı nesne") == Some("Yol")).expect("a road");
    let Entity::Circle(road) = road else { panic!("{road:?}") };
    assert_eq!((road.c.x, road.c.y, road.r), (e, n - 60.0, 10.0 * f64::from(1.275f32)));
    assert_eq!(road.base.attrs.get("Genişlik").map(String::as_str), Some("17"));
    // A function's name is a text, its value an attribute.
    let name = r.entities.iter().find(|x| x.base().layer_id == "SM_FONKADI").expect("the name");
    let Entity::Text(t) = name else { panic!("{name:?}") };
    assert_eq!(t.text, "TEKNOLOJİ GELİŞTİRME BÖLGESİ");
    // A symbol's text stands on its anchor with the anchor's alignment (docs/adr/0145 §7):
    // the function's name from the middle of its left, a settlement's values centred;
    // nothing is placed by a guess of its width.
    assert_eq!(t.align, Some(TextAlign::MiddleLeft));
    let settled: Vec<&TextEntity> = r
        .entities
        .iter()
        .filter_map(|x| match x {
            Entity::Text(t) if t.base.layer_id == "SM_YERLESIM" => Some(t),
            _ => None,
        })
        .collect();
    assert!(settled.iter().any(|t| t.align == Some(TextAlign::MiddleCenter)), "{settled:?}");
    assert!(settled.iter().all(|t| t.align.is_some()), "{settled:?}");
    assert!(r.report.notes.iter().all(|x| x.what != "Akıllı nesne yazısı"));
    // A class nothing knows stays where it is, as a point with its values.
    let odd = r.entities.iter().find(|x| attr(x, "Renk").is_some()).expect("the unknown one");
    let Entity::Point(p) = odd else { panic!("{odd:?}") };
    assert_eq!((p.p.x, p.p.y), (e, n - 120.0));
    let notes: Vec<(&str, u32)> = r.report.notes.iter().map(|x| (x.what.as_str(), x.count)).collect();
    for want in [
        ("Akıllı nesne (Yerleşim)", 3),
        ("Akıllı nesne (Yapılaşma)", 3),
        ("Akıllı nesne (Yol)", 2),
        ("Akıllı nesne (Plan Notu)", 1),
        ("Akıllı nesne (Fonksiyon Adı)", 1),
        ("Akıllı nesne (Akıllı Nesne)", 1),
    ] {
        assert!(notes.contains(&want), "{want:?} in {notes:?}");
    }
    // Where the view shows it: every object is near the others, nothing is a stray.
    assert_eq!(r.view, r.bounds);
}

/// A frame's corners in millimetres, (easting, northing).
fn millimetres(e: &Entity) -> Vec<(i64, i64)> {
    let Entity::Polygon(p) = e else { panic!("{e:?}") };
    p.pts.iter().map(|v| ((v.x * 1000.0).round() as i64, (v.y * 1000.0).round() as i64)).collect()
}

#[test]
fn a_sheets_frame_is_its_cell_turned_in_the_zone_the_file_names() {
    let r = read("08-pafta.ncz");
    assert_eq!(counts(&r), [("polygon", 5)]);
    assert_eq!(layers(&r), [("PINDEX_1000", "ink", 4), ("YEREL", "ink", 1)]);
    let sheet = |name: &str| r.entities.iter().find(|e| attr(e, "Pafta") == Some(name)).unwrap_or_else(|| panic!("{name}"));
    // The file keeps the box of each 22.5″ cell in TM39 (ITRF); the frame is the cell itself,
    // turned by the meridian convergence: south-west, south-east, north-east and north-west,
    // to the millimetre, PROJ's corners (+proj=tmerc +lon_0=39 +k=1 +x_0=500000 +ellps=GRS80).
    let frames = [
        ("GB", [(421_758_834, 4_450_753_176), (422_291_094, 4_450_747_687), (422_298_227, 4_451_441_691), (421_766_016, 4_451_447_181)]),
        ("GD", [(422_291_094, 4_450_747_687), (422_823_354, 4_450_742_235), (422_830_439, 4_451_436_239), (422_298_227, 4_451_441_691)]),
        ("KB", [(421_766_016, 4_451_447_181), (422_298_227, 4_451_441_691), (422_305_362, 4_452_135_696), (421_773_199, 4_452_141_186)]),
        ("KD", [(422_298_227, 4_451_441_691), (422_830_439, 4_451_436_239), (422_837_524, 4_452_130_244), (422_305_362, 4_452_135_696)]),
    ];
    for (name, corners) in frames {
        assert_eq!(millimetres(sheet(name)), corners, "{name}");
    }
    // Neighbours meet: the block's middle corner is one point of all four sheets.
    let Entity::Polygon(gb) = sheet("GB") else { unreachable!() };
    let middle = gb.pts[2];
    assert_eq!((middle.x, middle.y), (422_298.227, 4_451_441.691));
    for (name, at) in [("GD", 3), ("KB", 1), ("KD", 0)] {
        let Entity::Polygon(p) = sheet(name) else { unreachable!() };
        assert_eq!(p.pts[at], middle, "{name}");
    }
    // A sheet of round local coordinates is no grid cell's: it keeps the box.
    assert_eq!(
        millimetres(sheet("YEREL-1")),
        [(421_400_000, 4_448_400_000), (421_400_000, 4_449_100_000), (421_940_000, 4_449_100_000), (421_940_000, 4_448_400_000)]
    );
    let framed = r.report.notes.iter().find(|n| n.what == "Pafta çerçevesi").expect("the frames' note");
    assert_eq!(framed.count, 4);
    assert!(framed.reason.starts_with("dosyanın bildirdiği ITRF TM39 sisteminde gerçek biçimiyle"), "{}", framed.reason);
    let kept = r.report.skipped.iter().find(|s| s.what == "Pafta çerçevesinin gerçek biçimi").expect("the box kept");
    assert_eq!(kept.count, 1);
    assert!(kept.reason.ends_with("(yerel pafta)"), "{}", kept.reason);
}

#[test]
fn the_read_says_how_far_it_is_and_stops_when_told() {
    let bytes = fixture("01-her-tur.ncz");
    let mut heard = Vec::new();
    let read = kentos_ncz::read(&bytes, &NczReadOptions::default(), &mut Steps(|done, total| {
        heard.push((done, total));
        true
    }));
    assert!(read.is_ok());
    assert!(heard.windows(2).all(|w| w[0].0 <= w[1].0), "{heard:?}");
    assert_eq!(heard.last(), Some(&(kentos_ncz::PROGRESS_TOTAL, kentos_ncz::PROGRESS_TOTAL)));
    let stopped = kentos_ncz::read(&bytes, &NczReadOptions::default(), &mut Steps(|_, _| false));
    assert_eq!(stopped.err().as_deref(), Some(STOPPED));
}

#[test]
fn what_is_not_an_ncz_is_refused_with_the_reason() {
    let dxf = kentos_ncz::read(b"  0\r\nSECTION\r\n  2\r\nENTITIES\r\n  0\r\nEOF\r\n", &NczReadOptions::default(), &mut Quiet);
    assert!(dxf.as_ref().is_err_and(|e| e.starts_with("Bu dosyada Netcad NCZ çizimi bulunamadı")), "{dxf:?}");
    let kcad = kentos_ncz::read(b"KCAD\x02\x00", &NczReadOptions::default(), &mut Quiet);
    assert!(kcad.as_ref().is_err_and(|e| e.contains("Dosya → Aç")), "{kcad:?}");
}
