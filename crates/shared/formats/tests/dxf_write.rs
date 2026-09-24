//! The DXF writer through this crate's own reader: every kind written and
//! read back is the same object (TM coordinates bit for bit, labels,
//! attributes, symbols, colours, holes, exact arc angles and hatch
//! patterns), layers keep their colour, line type, weight, visibility and
//! lock, and the file has what AutoCAD needs of a 2000+ drawing (unique
//! handles below the seed, owners, the tables, blocks and objects).

use std::collections::{BTreeMap, HashSet};

use kentos_contracts::{
    ArcEntity, CircleEntity, ConstructionEntity, DimensionEntity, DimensionStyle, DxfReadOptions,
    DxfWriteInput, DxfWriteLayer, EllipseEntity, Entity, EntityBase, ExportReport, HatchEntity,
    HatchPattern, HatchPatternType, ImportResult, LineEntity, LineType, PathEntity, PointEntity,
    RingGeometry, SplineEntity, TextEntity, Vec2,
};
use kentos_formats::dxf;
use kentos_formats::math::{atan2, cos, sin};

const Y0: f64 = 452_345.123;
const X0: f64 = 4_412_345.678;

fn v(x: f64, y: f64) -> Vec2 {
    Vec2 { x, y }
}

/// A point at TM coordinates, `dy` metres east and `dx` north of the sheet corner.
fn tm(dy: f64, dx: f64) -> Vec2 {
    v(Y0 + dy, X0 + dx)
}

fn base(layer: &str) -> EntityBase {
    EntityBase {
        id: 0,
        layer_id: layer.into(),
        color: None,
        attrs: BTreeMap::new(),
        label: None,
        symbol: None,
    }
}

fn with(layer: &str, f: impl FnOnce(&mut EntityBase)) -> EntityBase {
    let mut b = base(layer);
    f(&mut b);
    b
}

fn layer(
    id: &str,
    name: &str,
    path: &[&str],
    color: &str,
    line_type: LineType,
    weight: f64,
) -> DxfWriteLayer {
    DxfWriteLayer {
        id: id.into(),
        name: name.into(),
        path: path.iter().map(|s| s.to_string()).collect(),
        color: color.into(),
        visible: true,
        locked: false,
        line_type,
        line_weight: weight,
    }
}

fn layers() -> Vec<DxfWriteLayer> {
    vec![
        layer(
            "parsel",
            "Parsel sınırı",
            &["Kadastro"],
            "fg-dim",
            LineType::Continuous,
            0.18,
        ),
        layer(
            "yapi",
            "Yapı",
            &["Kadastro"],
            "#7FB2E5",
            LineType::Continuous,
            0.25,
        ),
        layer(
            "yol",
            "Yol ekseni",
            &["Ulaşım"],
            "#e06c75",
            LineType::Dashdot,
            0.13,
        ),
        DxfWriteLayer {
            visible: false,
            locked: true,
            ..layer(
                "kot",
                "Kot noktaları",
                &["Topografya"],
                "ink",
                LineType::Dotted,
                0.35,
            )
        },
        layer("yazi", "Yazılar", &["Pafta"], "fg", LineType::Dashed, 0.5),
    ]
}

/// One or more objects of every kind (the dimension apart), at TM coordinates with awkward values.
fn objects() -> Vec<Entity> {
    let third = 1.0 / 3.0;
    let dir = (cos(0.3), sin(0.3));
    vec![
        Entity::Point(PointEntity {
            base: with("kot", |b| {
                b.label = Some("P1".into());
                b.attrs.insert("Ad".into(), "P1".into());
                b.attrs.insert("Kod".into(), "KS ^ çınar".into());
            }),
            p: tm(0.1 + 0.2, third),
            z: Some(0.0),
        }),
        Entity::Point(PointEntity {
            base: base("kot"),
            p: tm(10.0, 20.0),
            z: Some(105.2),
        }),
        Entity::Point(PointEntity {
            base: base("kot"),
            p: tm(-1e-7, 2.5),
            z: None,
        }),
        Entity::Line(LineEntity {
            base: with("parsel", |b| b.color = Some("#FF0000".into())),
            a: tm(0.0, 0.0),
            b: tm(12.345678, -0.001),
        }),
        Entity::Polyline(PathEntity {
            base: with("yol", |b| b.color = Some("fg".into())),
            pts: vec![tm(0.0, 0.0), tm(10.0, 0.0), tm(10.0, 10.0), tm(0.0, 0.0)],
            bulges: Some(vec![0.0, 0.4142135623730951, 0.0]),
            holes: None,
        }),
        Entity::Polyline(PathEntity {
            base: base("yol"),
            pts: vec![tm(0.0, 5.0), tm(3.0, 7.0)],
            bulges: None,
            holes: None,
        }),
        Entity::Polygon(PathEntity {
            base: with("parsel", |b| {
                b.label = Some("123".into());
                b.attrs.insert("Ada".into(), "45".into());
                b.attrs.insert("Parsel".into(), "123".into());
                b.attrs.insert("Tapu alanı".into(), "1234.56".into());
                b.symbol = Some("mpyy.konut".into());
            }),
            pts: vec![tm(0.0, 0.0), tm(40.0, 0.0), tm(40.0, 30.0), tm(0.0, 30.0)],
            bulges: Some(vec![0.0, -0.25, 0.0, 0.0]),
            holes: Some(vec![
                RingGeometry {
                    pts: vec![tm(5.0, 5.0), tm(10.0, 5.0), tm(10.0, 10.0), tm(5.0, 10.0)],
                    bulges: None,
                },
                RingGeometry {
                    pts: vec![tm(20.0, 20.0), tm(25.0, 20.0)],
                    bulges: Some(vec![1.0, 1.0]),
                },
            ]),
        }),
        Entity::Polygon(PathEntity {
            base: with("yapi", |b| b.color = Some("#7fb2e5".into())),
            pts: vec![tm(50.0, 0.0), tm(60.0, 0.0), tm(55.0, 8.0)],
            bulges: None,
            holes: None,
        }),
        Entity::Circle(CircleEntity {
            base: base("parsel"),
            c: tm(100.0, 100.0),
            r: 2.5 + third,
        }),
        Entity::Arc(ArcEntity {
            base: base("parsel"),
            c: tm(-20.0, 15.0),
            r: 7.25,
            a0: atan2(3.0, 4.0),
            a1: 2.5,
        }),
        Entity::Arc(ArcEntity {
            base: base("parsel"),
            c: tm(-20.0, -15.0),
            r: 1.0,
            a0: 30.0 * std::f64::consts::PI / 180.0,
            a1: 0.0,
        }),
        Entity::Ellipse(EllipseEntity {
            base: base("yapi"),
            c: tm(70.0, 70.0),
            major: v(30.5, -12.25),
            ratio: 0.4,
            t0: 0.5,
            t1: 4.0,
        }),
        Entity::Ellipse(EllipseEntity {
            base: base("yapi"),
            c: tm(90.0, 70.0),
            major: v(0.0, 3.0),
            ratio: third,
            t0: 0.0,
            t1: 0.0,
        }),
        Entity::Spline(SplineEntity {
            base: with("yol", |b| {
                b.attrs
                    .insert("Ad".into(), "dere".into())
                    .map_or((), |_| ())
            }),
            pts: vec![
                tm(0.0, 0.0),
                tm(10.0, 2.0),
                tm(12.0, 15.0),
                tm(40.0, 16.0),
                tm(41.0, 0.1 + 0.2),
            ],
            closed: false,
        }),
        Entity::Spline(SplineEntity {
            base: base("yol"),
            pts: vec![
                tm(0.0, 50.0),
                tm(10.0, 52.0),
                tm(12.0, 65.0),
                tm(-3.0, 60.0),
            ],
            closed: true,
        }),
        Entity::Xline(ConstructionEntity {
            base: base("yol"),
            p: tm(1.0, 2.0),
            dir: v(dir.0, dir.1),
        }),
        Entity::Ray(ConstructionEntity {
            base: base("yol"),
            p: tm(3.0, 4.0),
            dir: v(0.0, -1.0),
        }),
        Entity::Text(TextEntity {
            base: with("yazi", |b| b.color = Some("ink".into())),
            p: tm(5.0, 5.0),
            text: "Ada 12 / Parsel 3 çğıöşüİ ^ %%d 100%".into(),
            height: 2.5,
            rotation: 33.3,
        }),
        Entity::Text(TextEntity {
            base: base("yazi"),
            p: tm(6.0, 6.0),
            text: " boşluklu ".into(),
            height: third,
            rotation: 0.0,
        }),
        Entity::Hatch(HatchEntity {
            base: base("yapi"),
            ring: vec![tm(0.0, 0.0), tm(40.0, 0.0), tm(40.0, 30.0), tm(0.0, 30.0)],
            holes: Some(vec![
                vec![tm(20.0, 20.0), tm(25.0, 20.0), tm(25.0, 25.0)],
                vec![tm(5.0, 5.0), tm(10.0, 5.0), tm(10.0, 10.0), tm(5.0, 10.0)],
            ]),
            pattern: HatchPattern {
                kind: HatchPatternType::Cross,
                angle: 30.0,
                spacing: 2.0,
            },
        }),
        Entity::Hatch(HatchEntity {
            base: base("yapi"),
            ring: vec![tm(50.0, 0.0), tm(60.0, 0.0), tm(55.0, 8.0)],
            holes: None,
            pattern: HatchPattern {
                kind: HatchPatternType::Lines,
                angle: 37.5,
                spacing: 0.75,
            },
        }),
        Entity::Hatch(HatchEntity {
            base: base("yapi"),
            ring: vec![tm(70.0, 0.0), tm(80.0, 0.0), tm(75.0, 8.0)],
            holes: None,
            pattern: HatchPattern {
                kind: HatchPatternType::Solid,
                angle: 12.0,
                spacing: 3.0,
            },
        }),
        Entity::Hatch(HatchEntity {
            base: base("yapi"),
            ring: vec![tm(90.0, 0.0), tm(100.0, 0.0), tm(95.0, 8.0)],
            holes: None,
            pattern: HatchPattern {
                kind: HatchPatternType::Lines,
                angle: 0.0,
                spacing: 1.0,
            },
        }),
    ]
}

fn input(entities: Vec<Entity>) -> DxfWriteInput {
    DxfWriteInput {
        entities,
        layers: layers(),
        scale: 1000.0,
        length_decimals: 3,
        grads: true,
    }
}

fn write(input: &DxfWriteInput) -> (String, ExportReport) {
    let (bytes, report) = dxf::write(input);
    (String::from_utf8(bytes).expect("UTF-8"), report)
}

fn read(text: &str) -> ImportResult {
    dxf::read(text.as_bytes(), &DxfReadOptions::default()).unwrap_or_else(|e| panic!("{e}"))
}

/// What the reader should give back for `e`: the same object, on the DXF layer (by name).
fn expected(e: &Entity, layer_of: &dyn Fn(&str) -> String) -> Entity {
    let mut e = e.clone();
    let b = match &mut e {
        Entity::Point(x) => &mut x.base,
        Entity::Line(x) => &mut x.base,
        Entity::Polyline(x) | Entity::Polygon(x) => &mut x.base,
        Entity::Circle(x) => &mut x.base,
        Entity::Arc(x) => &mut x.base,
        Entity::Ellipse(x) => &mut x.base,
        Entity::Spline(x) => &mut x.base,
        Entity::Xline(x) | Entity::Ray(x) => &mut x.base,
        Entity::Text(x) => &mut x.base,
        Entity::Dimension(x) => &mut x.base,
        Entity::Hatch(x) => &mut x.base,
    };
    b.layer_id = layer_of(&b.layer_id);
    e
}

fn layer_name(id: &str) -> String {
    layers()
        .into_iter()
        .find(|l| l.id == id)
        .map(|l| l.name)
        .unwrap_or_default()
}

#[test]
fn every_kind_reads_back_as_the_same_object() {
    let objects = objects();
    let (text, report) = write(&input(objects.clone()));
    let r = read(&text);
    let want: Vec<Entity> = objects.iter().map(|e| expected(e, &layer_name)).collect();
    assert_eq!(r.entities.len(), want.len(), "{:#?}", r.report);
    for (got, want) in r.entities.iter().zip(&want) {
        assert_eq!(got, want);
    }
    // Nothing was approximated on the way back.
    assert!(r.report.skipped.is_empty(), "{:?}", r.report.skipped);
    assert!(r.report.notes.is_empty(), "{:?}", r.report.notes);
    let facts: Vec<(&str, &str)> = r
        .report
        .source
        .iter()
        .map(|f| (f.label.as_str(), f.value.as_str()))
        .collect();
    assert!(
        facts.contains(&("Sürüm", "AutoCAD 2007 (AC1021)")),
        "{facts:?}"
    );
    assert!(facts.contains(&("Karakter kodlaması", "UTF-8")));
    assert!(facts.contains(&("Birim ($INSUNITS)", "metre")));
    // The writer's own report: every object by kind, the holes said.
    let written: u32 = report.counts.values().sum();
    assert_eq!(written as usize, objects.len());
    assert_eq!(report.counts.get("hatch"), Some(&4));
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
    assert!(report.notes.iter().any(|n| n.what == "Adalı alan"));
}

#[test]
fn layers_keep_colour_line_type_weight_visibility_and_lock() {
    let (text, report) = write(&input(objects()));
    let r = read(&text);
    let got: Vec<(String, String, LineType, Option<f64>, bool, bool)> = r
        .layers
        .iter()
        .map(|l| {
            (
                l.name.clone(),
                l.color.clone(),
                l.line_type,
                l.line_weight,
                l.visible,
                l.locked,
            )
        })
        .collect();
    assert_eq!(
        got,
        vec![
            (
                "Parsel sınırı".into(),
                "fg-dim".into(),
                LineType::Continuous,
                Some(0.18),
                true,
                false
            ),
            (
                "Yapı".into(),
                "#7FB2E5".into(),
                LineType::Continuous,
                Some(0.25),
                true,
                false
            ),
            (
                "Yol ekseni".into(),
                "#e06c75".into(),
                LineType::Dashdot,
                Some(0.13),
                true,
                false
            ),
            (
                "Kot noktaları".into(),
                "ink".into(),
                LineType::Dotted,
                Some(0.35),
                false,
                true
            ),
            (
                "Yazılar".into(),
                "fg".into(),
                LineType::Dashed,
                Some(0.5),
                true,
                false
            ),
        ]
    );
    // Theme tokens are said: another program shows them as fixed colours.
    assert!(
        report
            .notes
            .iter()
            .any(|n| n.what == "Katman rengi" && n.reason.contains("fg-dim"))
    );
    // Dashes are sized for paper at the plot scale: 2.5 mm at 1:1000 is 2.5 m.
    assert!(text.contains("DASHED\r\n 70\r\n0\r\n  3\r\nKesikli __ __ __\r\n 72\r\n65\r\n 73\r\n2\r\n 40\r\n3.75\r\n 49\r\n2.5\r\n"));
}

/// The group pairs of a file (code, value).
fn pairs(text: &str) -> Vec<(i32, &str)> {
    let lines: Vec<&str> = text.split("\r\n").collect();
    lines
        .chunks(2)
        .filter(|c| c.len() == 2)
        .map(|c| (c[0].trim().parse::<i32>().expect("group code"), c[1]))
        .collect()
}

#[test]
fn the_file_has_what_autocad_needs() {
    let (text, _) = write(&input(objects()));
    assert!(text.ends_with("  0\r\nEOF\r\n"));
    let p = pairs(&text);
    let sections: Vec<&str> = p
        .windows(2)
        .filter(|w| w[0] == (0, "SECTION"))
        .map(|w| w[1].1)
        .collect();
    assert_eq!(
        sections,
        [
            "HEADER", "CLASSES", "TABLES", "BLOCKS", "ENTITIES", "OBJECTS"
        ]
    );
    let tables: Vec<&str> = p
        .windows(2)
        .filter(|w| w[0] == (0, "TABLE"))
        .map(|w| w[1].1)
        .collect();
    assert_eq!(
        tables,
        [
            "VPORT",
            "LTYPE",
            "LAYER",
            "STYLE",
            "VIEW",
            "UCS",
            "APPID",
            "DIMSTYLE",
            "BLOCK_RECORD"
        ]
    );
    // Every handle is unique and below the seed; every owner is a handle of the file.
    let seed = p
        .windows(2)
        .find(|w| w[0] == (9, "$HANDSEED"))
        .map(|w| u64::from_str_radix(w[1].1, 16).expect("seed"))
        .expect("$HANDSEED");
    // Group 5 is also the seed's own code in the header.
    let body = p
        .iter()
        .position(|&x| x == (2, "CLASSES"))
        .expect("CLASSES");
    let handles: Vec<u64> = p[body..]
        .iter()
        .filter(|(c, _)| *c == 5 || *c == 105)
        .map(|(_, v)| u64::from_str_radix(v, 16).expect("handle"))
        .collect();
    let unique: HashSet<u64> = handles.iter().copied().collect();
    assert_eq!(unique.len(), handles.len());
    assert!(handles.iter().all(|&h| h > 0 && h < seed));
    for (code, value) in &p {
        if matches!(code, 330 | 340 | 350 | 360 | 390 | 347 | 1005) && *value != "0" {
            let h = u64::from_str_radix(value, 16).expect("reference");
            assert!(
                unique.contains(&h),
                "group {code} names {value}, which no object has"
            );
        }
    }
    // Every application whose extended data the file holds is registered in the APPID table.
    let registered: HashSet<&str> = p
        .windows(8)
        .filter(|w| w[0] == (0, "APPID"))
        .filter_map(|w| w.iter().find(|x| x.0 == 2).map(|x| x.1))
        .collect();
    assert_eq!(registered, HashSet::from(["ACAD", "KENTOS"]));
    assert!(
        p.iter()
            .filter(|x| x.0 == 1001)
            .all(|x| registered.contains(x.1))
    );
    // The file opens on the drawing: the view's centre lies inside its extent.
    let at = |name: &str, code: i32| -> f64 {
        let i = p.iter().position(|&x| x == (9, name)).expect(name);
        p[i..]
            .iter()
            .find(|x| x.0 == code)
            .map(|x| x.1.parse().expect("number"))
            .expect("value")
    };
    assert!(at("$EXTMIN", 10) < Y0 && at("$EXTMAX", 20) > X0);
    assert_eq!(at("$INSUNITS", 70), 6.0);
    assert_eq!(at("$AUNITS", 70), 2.0);
    // Group codes are right-aligned in three places, as AutoCAD writes them.
    assert!(
        text.starts_with("  0\r\nSECTION\r\n  2\r\nHEADER\r\n  9\r\n$ACADVER\r\n  1\r\nAC1021\r\n")
    );
}

#[test]
fn a_dimension_is_written_as_its_lines_and_text() {
    let d = Entity::Dimension(DimensionEntity {
        base: with("yazi", |b| {
            b.attrs
                .insert("Not".into(), "ölçü".into())
                .map_or((), |_| ())
        }),
        a: tm(0.0, 0.0),
        b: tm(10.0, 0.0),
        offset: 2.0,
        height: 0.5,
        text: Some("10.00".into()),
        style: Some(DimensionStyle::Aligned),
        angle: None,
        c: None,
    });
    let (text, report) = write(&input(vec![d]));
    let r = read(&text);
    let lines = r
        .entities
        .iter()
        .filter(|e| matches!(e, Entity::Line(_)))
        .count();
    let texts: Vec<&TextEntity> = r
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Text(t) => Some(t),
            _ => None,
        })
        .collect();
    assert!(lines >= 3, "{:#?}", r.entities);
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].text, "10.00");
    // The pieces are on the dimension's layer, without its attributes (as Patlat does).
    assert!(r.entities.iter().all(|e| match e {
        Entity::Line(l) => l.base.layer_id == "Yazılar" && l.base.attrs.is_empty(),
        Entity::Text(t) => t.base.layer_id == "Yazılar" && t.base.attrs.is_empty(),
        _ => false,
    }));
    assert_eq!(report.counts.get("dimension"), Some(&1));
    assert!(
        report
            .notes
            .iter()
            .any(|n| n.what == "Ölçü" && n.reason.contains("patlatılarak"))
    );
}

#[test]
fn what_another_program_changed_wins_over_stale_kentos_data() {
    let arc = Entity::Arc(ArcEntity {
        base: with("parsel", |b| b.color = Some("fg".into())),
        c: tm(0.0, 0.0),
        r: 5.0,
        a0: atan2(3.0, 4.0),
        a1: 2.5,
    });
    let (text, _) = write(&input(vec![arc.clone()]));
    // As written, the radians come back exactly.
    let Entity::Arc(back) = &read(&text).entities[0] else {
        panic!()
    };
    assert_eq!((back.a0, back.a1), (atan2(3.0, 4.0), 2.5));
    assert_eq!(back.base.color.as_deref(), Some("fg"));
    // Another program turns the arc's start to 90° and its colour to red, keeping KentOS's data.
    let at = text.find("  0\r\nARC\r\n").expect("ARC");
    let (head, arc_text) = text.split_at(at);
    let p = pairs(arc_text);
    let start = p
        .iter()
        .find(|x| x.0 == 50)
        .map(|x| x.1.to_string())
        .expect("start angle");
    let edited = format!(
        "{head}{}",
        arc_text
            .replacen(&format!(" 50\r\n{start}\r\n"), " 50\r\n90.0\r\n", 1)
            .replacen(" 62\r\n7\r\n", " 62\r\n1\r\n", 1)
    );
    let Entity::Arc(back) = &read(&edited).entities[0] else {
        panic!()
    };
    assert_eq!(back.a0, std::f64::consts::FRAC_PI_2);
    assert_eq!(back.a1, 2.5);
    assert_eq!(back.base.color.as_deref(), Some("#FF0000"));
}

#[test]
fn names_and_attributes_that_dxf_cannot_hold_as_they_are() {
    let mut long = BTreeMap::new();
    long.insert("Açıklama".to_string(), "uzun ".repeat(100));
    long.insert("Satırlar".to_string(), "bir\nİki\tüç".to_string());
    let big: BTreeMap<String, String> = (0..40)
        .map(|i| (format!("A{i}"), "x".repeat(500)))
        .collect();
    let mut layers = layers();
    layers.push(layer(
        "dup",
        "parsel SINIRI",
        &["İmar"],
        "#123456",
        LineType::Continuous,
        0.33,
    ));
    let entities = vec![
        Entity::Point(PointEntity {
            base: with("parsel", |b| b.attrs = long.clone()),
            p: tm(1.0, 1.0),
            z: None,
        }),
        Entity::Point(PointEntity {
            base: with("parsel", |b| {
                b.attrs = big.clone();
                b.label = Some("büyük".into());
            }),
            p: tm(2.0, 2.0),
            z: None,
        }),
        Entity::Line(LineEntity {
            base: base("dup"),
            a: tm(0.0, 0.0),
            b: tm(1.0, 0.0),
        }),
        Entity::Line(LineEntity {
            base: base("yok"),
            a: tm(0.0, 0.0),
            b: tm(0.0, 1.0),
        }),
        Entity::Text(TextEntity {
            base: base("yazi"),
            p: tm(0.0, 0.0),
            text: "iki\nsatır".into(),
            height: 1.0,
            rotation: 0.0,
        }),
        Entity::Circle(CircleEntity {
            base: base("parsel"),
            c: tm(0.0, 0.0),
            r: 0.0,
        }),
    ];
    let (text, report) = write(&DxfWriteInput {
        entities: entities.clone(),
        layers,
        scale: 500.0,
        length_decimals: 2,
        grads: false,
    });
    let r = read(&text);
    // Long values came in pieces and control characters in caret notation: the attributes are back as they were.
    let Entity::Point(p) = &r.entities[0] else {
        panic!()
    };
    assert_eq!(p.base.attrs, long);
    // Over AutoCAD's 16 KB the attributes and label are left out, and that is said.
    let Entity::Point(p) = &r.entities[1] else {
        panic!()
    };
    assert!(p.base.attrs.is_empty() && p.base.label.is_none());
    assert!(report.notes.iter().any(|n| n.what == "Öznitelikler"));
    // Layers of the same name (DXF names ignore case) take their groups in front; a weight AutoCAD lacks moves.
    let Entity::Line(l) = &r.entities[2] else {
        panic!()
    };
    assert_eq!(l.base.layer_id, "İmar - parsel SINIRI");
    assert!(
        r.layers
            .iter()
            .any(|x| x.name == "Kadastro - Parsel sınırı")
    );
    assert!(
        report
            .notes
            .iter()
            .any(|n| n.what == "Çizgi kalınlığı" && n.reason.contains("0.35 mm"))
    );
    // An object on a layer the drawing lacks goes to 0.
    let Entity::Line(l) = &r.entities[3] else {
        panic!()
    };
    assert_eq!(l.base.layer_id, "0");
    // One line of text; a circle without a radius is left out and said.
    let Entity::Text(t) = &r.entities[4] else {
        panic!()
    };
    assert_eq!(t.text, "iki satır");
    assert_eq!(r.entities.len(), 5);
    assert!(report.skipped.iter().any(|s| s.what == "Daire"));
}

#[test]
fn nothing_to_write_is_still_a_file_autocad_opens() {
    let (text, report) = write(&DxfWriteInput {
        entities: Vec::new(),
        layers: Vec::new(),
        scale: 1000.0,
        length_decimals: 3,
        grads: false,
    });
    let r = read(&text);
    assert!(r.entities.is_empty() && r.layers.is_empty());
    assert!(report.counts.is_empty());
    assert!(
        text.contains("  2\r\n0\r\n 70\r\n0\r\n 62\r\n7\r\n"),
        "layer 0"
    );
}
