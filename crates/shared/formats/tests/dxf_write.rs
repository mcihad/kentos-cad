//! The DXF writer through this crate's own reader: every kind written and
//! read back is the same object (TM coordinates bit for bit, labels,
//! attributes, symbols, colours, holes, exact arc angles and hatch
//! patterns), layers keep their colour, line type, weight, visibility and
//! lock, and the file has what AutoCAD needs of a 2000+ drawing (unique
//! handles below the seed, owners, the tables, blocks and objects).

use std::collections::{BTreeMap, HashSet};

use kentos_contracts::{
    ArcEntity, BlockDefinition, BlockId, CircleEntity, ConstructionEntity, DimensionEntity,
    DimensionStyle, DrawingUnit, DxfReadOptions, DxfWriteInput, DxfWriteLayer, EllipseEntity,
    Entity, EntityBase, ExportReport, HatchEntity, HatchPattern, HatchPatternType, ImportResult,
    InsertEntity, LineEntity, LineType, PathEntity, PointEntity, RingGeometry, SplineEntity,
    TextEntity, Vec2,
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
        line_weight: None,
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
            parts: None,
        }),
        Entity::Point(PointEntity {
            base: base("kot"),
            p: tm(10.0, 20.0),
            z: Some(105.2),
            parts: None,
        }),
        Entity::Point(PointEntity {
            base: base("kot"),
            p: tm(-1e-7, 2.5),
            z: None,
            parts: None,
        }),
        Entity::Line(LineEntity {
            base: with("parsel", |b| b.color = Some("#FF0000".into())),
            a: tm(0.0, 0.0),
            b: tm(12.345678, -0.001),
            za: None,
            zb: None,
        }),
        Entity::Polyline(PathEntity {
            base: with("yol", |b| b.color = Some("fg".into())),
            pts: vec![tm(0.0, 0.0), tm(10.0, 0.0), tm(10.0, 10.0), tm(0.0, 0.0)],
            bulges: Some(vec![0.0, 0.4142135623730951, 0.0]),
            holes: None,
            zs: None,
            parts: None,
        }),
        Entity::Polyline(PathEntity {
            base: base("yol"),
            pts: vec![tm(0.0, 5.0), tm(3.0, 7.0)],
            bulges: None,
            holes: None,
            zs: None,
            parts: None,
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
                    zs: None,
                },
                RingGeometry {
                    pts: vec![tm(20.0, 20.0), tm(25.0, 20.0)],
                    bulges: Some(vec![1.0, 1.0]),
                    zs: None,
                },
            ]),
            zs: None,
            parts: None,
        }),
        Entity::Polygon(PathEntity {
            base: with("yapi", |b| b.color = Some("#7fb2e5".into())),
            pts: vec![tm(50.0, 0.0), tm(60.0, 0.0), tm(55.0, 8.0)],
            bulges: None,
            holes: None,
            zs: None,
            parts: None,
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
            align: None,
            width_factor: None,
            mask: false,
            label_of: None,
            label_scale: None,
            paragraph: Default::default(),
            face: Default::default(),
        }),
        Entity::Text(TextEntity {
            base: base("yazi"),
            p: tm(6.0, 6.0),
            text: " boşluklu ".into(),
            height: third,
            rotation: 0.0,
            align: None,
            width_factor: None,
            mask: false,
            label_of: None,
            label_scale: None,
            paragraph: Default::default(),
            face: Default::default(),
        }),
        Entity::Hatch(HatchEntity {
            base: base("yapi"),
            ring: vec![tm(0.0, 0.0), tm(40.0, 0.0), tm(40.0, 30.0), tm(0.0, 30.0)],
            holes: Some(vec![
                vec![tm(20.0, 20.0), tm(25.0, 20.0), tm(25.0, 25.0)],
                vec![tm(5.0, 5.0), tm(10.0, 5.0), tm(10.0, 10.0), tm(5.0, 10.0)],
            ]),
            pattern: HatchPattern::user(HatchPatternType::Cross, 30.0, 2.0),
            assoc: None,
        }),
        Entity::Hatch(HatchEntity {
            base: base("yapi"),
            ring: vec![tm(50.0, 0.0), tm(60.0, 0.0), tm(55.0, 8.0)],
            holes: None,
            pattern: HatchPattern::user(HatchPatternType::Lines, 37.5, 0.75),
            assoc: None,
        }),
        Entity::Hatch(HatchEntity {
            base: base("yapi"),
            ring: vec![tm(70.0, 0.0), tm(80.0, 0.0), tm(75.0, 8.0)],
            holes: None,
            pattern: HatchPattern::user(HatchPatternType::Solid, 12.0, 3.0),
            assoc: None,
        }),
        Entity::Hatch(HatchEntity {
            base: base("yapi"),
            ring: vec![tm(90.0, 0.0), tm(100.0, 0.0), tm(95.0, 8.0)],
            holes: None,
            pattern: HatchPattern::user(HatchPatternType::Lines, 0.0, 1.0),
            assoc: None,
        }),
    ]
    .into_iter()
    .chain(dimensions())
    .collect()
}

/// A dimension of every style at TM coordinates (one without a style, two linear, with and without an angle).
fn dimensions() -> Vec<Entity> {
    let dim = |id: u32, style, a, b, offset, angle, c, text: Option<&str>| {
        Entity::Dimension(DimensionEntity {
            base: with("yazi", |b| {
                b.id = id;
                if id == 101 {
                    b.label = Some("Ö1".into());
                    b.attrs.insert("Not".into(), "yol ölçüsü".into());
                    b.color = Some("#7fb2e5".into());
                }
            }),
            a,
            b,
            offset,
            height: 0.75,
            text: text.map(str::to_string),
            style,
            angle,
            c,
            mask: false,
            za: None,
            zb: None,
            look: Default::default(),
        })
    };
    vec![
        dim(
            100,
            None,
            tm(0.0, 0.0),
            tm(10.0, 0.0),
            2.0,
            None,
            None,
            None,
        ),
        dim(
            101,
            Some(DimensionStyle::Aligned),
            tm(0.3, 20.1),
            tm(12.7, 27.9),
            -1.5,
            None,
            None,
            Some("yol {kenar} \\ 45%%d ^"),
        ),
        dim(
            102,
            Some(DimensionStyle::Linear),
            tm(20.0, 0.0),
            tm(31.25, 7.5),
            3.0,
            Some(90.0),
            None,
            None,
        ),
        dim(
            103,
            Some(DimensionStyle::Linear),
            tm(20.0, 10.0),
            tm(31.25, 17.5),
            2.5,
            None,
            None,
            None,
        ),
        dim(
            104,
            Some(DimensionStyle::Angular),
            tm(45.0, 0.0),
            tm(40.0, 6.0),
            4.5,
            None,
            Some(tm(40.0, 0.0)),
            None,
        ),
        dim(
            105,
            Some(DimensionStyle::Radius),
            tm(60.0, 0.0),
            tm(62.5, 1.75),
            1.0,
            None,
            None,
            None,
        ),
        dim(
            106,
            Some(DimensionStyle::Diameter),
            tm(70.0, 0.0),
            tm(71.2, 3.3),
            0.0,
            None,
            None,
            Some("Ø 7,00"),
        ),
    ]
}

/// The value every dimension without a text of its own shows (the app formats it).
fn shown(id: u32) -> String {
    format!("{}.500", id - 90)
}

fn input(entities: Vec<Entity>) -> DxfWriteInput {
    let dimension_values = entities
        .iter()
        .filter_map(|e| match e {
            Entity::Dimension(d) if d.text.is_none() => Some((d.base.id, shown(d.base.id))),
            _ => None,
        })
        .collect();
    DxfWriteInput {
        entities,
        layers: layers(),
        scale: 1000.0,
        length_decimals: 3,
        grads: true,
        dimension_values,
        blocks: Vec::new(),
        unit: None,
        text_styles: Vec::new(),
        dimension_styles: Vec::new(),
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
    let b = e.base_mut();
    b.layer_id = layer_of(&b.layer_id);
    // The reader numbers nothing: the app gives imported objects their ids.
    b.id = 0;
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
    assert_eq!(report.counts.get("dimension"), Some(&7));
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

/// The groups of each entity of a kind, in the order the file has them.
fn entities_of<'a>(p: &[(i32, &'a str)], kind: &str) -> Vec<Vec<(i32, &'a str)>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < p.len() {
        if p[i] == (0, kind) {
            let end = p[i + 1..]
                .iter()
                .position(|x| x.0 == 0)
                .map_or(p.len(), |k| i + 1 + k);
            out.push(p[i..end].to_vec());
            i = end;
        } else {
            i += 1;
        }
    }
    out
}

fn group<'a>(e: &[(i32, &'a str)], code: i32) -> Option<&'a str> {
    e.iter().find(|x| x.0 == code).map(|x| x.1)
}

/// Each dimension is a DIMENSION of its DXF type with a block of its own
/// (a block record, BLOCK, the lines and ticks, the arc as an ARC, the
/// value as MTEXT on layer 0 in the dimension's colour) and AutoCAD's style
/// overrides; KentOS reads the same dimensions back (`every_kind_…`).
#[test]
fn a_dimension_is_a_dxf_dimension_with_a_block_of_its_own() {
    let (text, report) = write(&input(dimensions()));
    let p = pairs(&text);
    let dims = entities_of(&p, "DIMENSION");
    assert_eq!(dims.len(), 7);
    let types: Vec<i64> = dims
        .iter()
        .map(|d| group(d, 70).expect("70").parse::<i64>().expect("type") & 7)
        .collect();
    assert_eq!(types, [1, 1, 0, 0, 5, 4, 3]);
    let records: HashSet<&str> = entities_of(&p, "BLOCK_RECORD")
        .iter()
        .filter_map(|r| group(r, 2))
        .collect();
    let blocks = entities_of(&p, "BLOCK");
    let mtexts = entities_of(&p, "MTEXT");
    for (k, d) in dims.iter().enumerate() {
        let name = group(d, 2).expect("block name");
        assert_eq!(name, format!("*D{}", k + 1));
        assert!(records.contains(name), "{name}");
        let block = blocks
            .iter()
            .find(|b| group(b, 2) == Some(name))
            .expect("block");
        assert_eq!(group(block, 70), Some("1"));
        // AutoCAD's DSTYLE data: the text height is the dimension's.
        let at = d
            .iter()
            .position(|x| *x == (1000, "DSTYLE"))
            .expect("DSTYLE");
        assert_eq!(d[at + 2..at + 4], [(1070, "140"), (1040, "0.75")]);
    }
    // Standard shows a redrawn value as KentOS does: DIMZIN and DIMAZIN keep
    // trailing zeros, DIMDSEP is a point, DIMRND is left out (ezdxf rounds 0 to whole numbers).
    let style = entities_of(&p, "DIMSTYLE");
    assert_eq!(style.len(), 1);
    assert_eq!(group(&style[0], 2), Some("Standard"));
    assert_eq!(
        [45, 78, 79, 278].map(|c| group(&style[0], c)),
        [None, Some("0"), Some("0"), Some("46")]
    );
    // The values: the app's for dimensions without a text, the own text otherwise (in MTEXT's notation).
    let values: Vec<&str> = mtexts.iter().map(|m| group(m, 1).expect("text")).collect();
    assert_eq!(
        values,
        [
            "10.500",
            "yol \\{kenar\\} \\\\ 45%%%%%%d ^ ",
            "12.500",
            "13.500",
            "14.500",
            "15.500",
            "Ø 7,00"
        ]
    );
    assert!(
        mtexts
            .iter()
            .all(|m| group(m, 8) == Some("0") && group(m, 62) == Some("0"))
    );
    // The own text is the dimension's text; the others have none (the measured value).
    assert_eq!(group(&dims[1], 1), Some(values[1]));
    assert_eq!(group(&dims[0], 1), Some(""));
    // The angular dimension's arc is one ARC, not chords.
    assert_eq!(entities_of(&p, "ARC").len(), 1);
    assert_eq!(report.counts.get("dimension"), Some(&7));
    assert!(
        report.notes.iter().all(|n| n.what != "Ölçü"),
        "{:?}",
        report.notes
    );
    let r = read(&text);
    assert!(r.entities.iter().all(|e| matches!(e, Entity::Dimension(_))));
    assert!(r.report.notes.is_empty(), "{:?}", r.report.notes);
}

/// A dimension another program moved (its definition point is not where
/// KentOS put it) is drawn by its block, as any other program's dimension.
#[test]
fn a_dimension_changed_elsewhere_is_drawn_by_its_block() {
    let d = dimensions().remove(0);
    let (text, _) = write(&input(vec![d]));
    let at = text.find("  0\r\nDIMENSION\r\n").expect("DIMENSION");
    let ten = at + text[at..].find(" 10\r\n").expect("group 10") + " 10\r\n".len();
    let end = ten + text[ten..].find("\r\n").expect("value");
    let moved = format!("{}{}{}", &text[..ten], "452345.5", &text[end..]);
    let r = read(&moved);
    assert!(!r.entities.iter().any(|e| matches!(e, Entity::Dimension(_))));
    assert!(r.entities.iter().any(|e| matches!(e, Entity::Line(_))));
    let texts: Vec<&TextEntity> = r
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Text(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].text, "10.500");
    assert!(
        r.report
            .notes
            .iter()
            .any(|n| n.what == "Ölçü (DIMENSION)" && n.reason.contains("başka bir programda"))
    );
    // A definition point that cannot be read leaves the dimension to its block too; nothing is refused.
    let at13 = at + text[at..].find(" 13\r\n").expect("group 13") + " 13\r\n".len();
    let end13 = at13 + text[at13..].find("\r\n").expect("value");
    let broken = format!("{}{}{}", &text[..at13], "x", &text[end13..]);
    let r = read(&broken);
    assert!(!r.entities.iter().any(|e| matches!(e, Entity::Dimension(_))));
    assert!(r.entities.iter().any(|e| matches!(e, Entity::Line(_))));
    assert!(r.report.skipped.is_empty(), "{:?}", r.report.skipped);
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
            parts: None,
        }),
        Entity::Point(PointEntity {
            base: with("parsel", |b| {
                b.attrs = big.clone();
                b.label = Some("büyük".into());
            }),
            p: tm(2.0, 2.0),
            z: None,
            parts: None,
        }),
        Entity::Line(LineEntity {
            base: base("dup"),
            a: tm(0.0, 0.0),
            b: tm(1.0, 0.0),
            za: None,
            zb: None,
        }),
        Entity::Line(LineEntity {
            base: base("yok"),
            a: tm(0.0, 0.0),
            b: tm(0.0, 1.0),
            za: None,
            zb: None,
        }),
        Entity::Text(TextEntity {
            base: base("yazi"),
            p: tm(0.0, 0.0),
            text: "iki\nsatır".into(),
            height: 1.0,
            rotation: 0.0,
            align: None,
            width_factor: None,
            mask: false,
            label_of: None,
            label_scale: None,
            paragraph: Default::default(),
            face: Default::default(),
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
        dimension_values: BTreeMap::new(),
        blocks: Vec::new(),
        unit: None,
        text_styles: Vec::new(),
        dimension_styles: Vec::new(),
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
    // A text of two lines is an MTEXT and comes back so (docs/adr/0182 §5); a circle without a radius is
    // left out and said.
    let Entity::Text(t) = &r.entities[4] else {
        panic!()
    };
    assert_eq!(t.text, "iki\nsatır");
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
        dimension_values: BTreeMap::new(),
        blocks: Vec::new(),
        unit: None,
        text_styles: Vec::new(),
        dimension_styles: Vec::new(),
    });
    let r = read(&text);
    assert!(r.entities.is_empty() && r.layers.is_empty());
    assert!(report.counts.is_empty());
    assert!(
        text.contains("  2\r\n0\r\n 70\r\n0\r\n 62\r\n7\r\n"),
        "layer 0"
    );
}

#[test]
fn an_objects_line_weight_goes_out_as_370_and_comes_back_exactly() {
    // docs/adr/0139: the nearest of AutoCAD's weights in 370; one it rounds goes in KentOS's
    // data and comes back exactly, while the file's 370 still is the weight it rounds to.
    let line = |w: Option<f64>, dy: f64| {
        Entity::Line(LineEntity {
            base: with("parsel", |b| b.line_weight = w),
            a: tm(dy, 0.0),
            b: tm(dy, 10.0),
            za: None,
            zb: None,
        })
    };
    let objects = vec![
        line(Some(0.35), 0.0),
        line(Some(0.33), 1.0),
        line(Some(0.0), 2.0),
        line(None, 3.0),
    ];
    let (text, report) = write(&input(objects.clone()));
    let p = pairs(&text);
    let written: Vec<Option<&str>> = entities_of(&p, "LINE")
        .iter()
        .map(|e| group(e, 370))
        .collect();
    assert_eq!(written, [Some("35"), Some("35"), Some("0"), None]);
    assert!(
        report.notes.iter().any(|n| n.what == "Nesne kalınlığı"),
        "{:?}",
        report.notes
    );
    let back = read(&text);
    let weights: Vec<Option<f64>> = back.entities.iter().map(|e| e.base().line_weight).collect();
    assert_eq!(weights, [Some(0.35), Some(0.33), Some(0.0), None]);
    // Another program sets the second one to 0.50 mm: its 370 wins over KentOS's stale 0.33.
    let at = text
        .match_indices("  0\r\nLINE\r\n")
        .nth(1)
        .expect("the second LINE")
        .0;
    let (head, rest) = text.split_at(at);
    let edited = format!(
        "{head}{}",
        rest.replacen("370\r\n35\r\n", "370\r\n50\r\n", 1)
    );
    let weights: Vec<Option<f64>> = read(&edited)
        .entities
        .iter()
        .map(|e| e.base().line_weight)
        .collect();
    assert_eq!(weights[1], Some(0.5));
}

/// FNV-1a over a file's bytes (64 bits): a fingerprint to hold a written file to.
fn fnv64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// A drawing without elevations is the file it was before elevations were
/// (docs/adr/0142): the same bytes, so nothing an old reader was given changes.
/// The length and fingerprint are those the writer gave for `objects()` before
/// the elevations existed.
#[test]
fn a_drawing_without_elevations_is_written_byte_for_byte_as_before() {
    let (bytes, report) = dxf::write(&input(objects()));
    assert_eq!((bytes.len(), fnv64(&bytes)), (38014, 0x5452_5a32_43b1_f0e6));
    // And it says nothing of elevations.
    assert!(
        report
            .notes
            .iter()
            .all(|n| n.what != "Kot (Z)" && n.what != "Kotsuz köşe")
    );
}

/// Objects with elevations: lines, paths with a vertex without one, areas with holes, and paths with arcs.
fn elevated() -> Vec<Entity> {
    let third = 1.0 / 3.0;
    let square = || vec![tm(0.0, 0.0), tm(40.0, 0.0), tm(40.0, 30.0), tm(0.0, 30.0)];
    vec![
        // 0-2: a line with both ends, one with an end without, and one at 0 on purpose.
        Entity::Line(LineEntity {
            base: base("parsel"),
            a: tm(0.0, 0.0),
            b: tm(12.5, 3.25),
            za: Some(105.5 + third),
            zb: Some(-107.25),
        }),
        Entity::Line(LineEntity {
            base: base("parsel"),
            a: tm(1.0, 1.0),
            b: tm(2.0, 2.0),
            za: None,
            zb: Some(12.0),
        }),
        Entity::Line(LineEntity {
            base: base("parsel"),
            a: tm(3.0, 3.0),
            b: tm(4.0, 4.0),
            za: Some(0.0),
            zb: Some(0.0),
        }),
        // 3-4: a path whose second vertex has none, with the label and attributes DXF cannot hold; one at 0 on purpose.
        Entity::Polyline(PathEntity {
            base: with("yol", |b| {
                b.label = Some("Y1".into());
                b.attrs.insert("Ad".into(), "Cadde".into());
            }),
            pts: vec![tm(0.0, 0.0), tm(10.0, 0.0), tm(10.0, 10.0), tm(0.0, 10.0)],
            bulges: None,
            holes: None,
            zs: Some(vec![Some(10.0), None, Some(12.5 + third), Some(-3.0)]),
            parts: None,
        }),
        Entity::Polyline(PathEntity {
            base: base("yol"),
            pts: vec![tm(0.0, 5.0), tm(3.0, 7.0), tm(9.0, 9.0)],
            bulges: None,
            holes: None,
            zs: Some(vec![Some(0.0), Some(0.0), Some(0.0)]),
            parts: None,
        }),
        // 5: an area whose outline has a vertex without a height, with a hole that has them and one that has none.
        Entity::Polygon(PathEntity {
            base: with("parsel", |b| b.label = Some("123".into())),
            pts: square(),
            bulges: None,
            holes: Some(vec![
                RingGeometry {
                    pts: vec![tm(5.0, 5.0), tm(10.0, 5.0), tm(10.0, 10.0), tm(5.0, 10.0)],
                    bulges: None,
                    zs: Some(vec![Some(100.5), Some(100.5), Some(101.5), Some(101.5)]),
                },
                RingGeometry {
                    pts: vec![tm(20.0, 20.0), tm(25.0, 20.0), tm(25.0, 25.0)],
                    bulges: None,
                    zs: None,
                },
            ]),
            zs: Some(vec![Some(100.0), Some(101.0), None, Some(103.0)]),
            parts: None,
        }),
        // 6-7: arcs at one height (a LWPOLYLINE holds one), at 0 on purpose.
        Entity::Polyline(PathEntity {
            base: base("yol"),
            pts: vec![tm(0.0, 20.0), tm(10.0, 20.0), tm(10.0, 30.0)],
            bulges: Some(vec![0.5, 0.0]),
            holes: None,
            zs: Some(vec![Some(250.5); 3]),
            parts: None,
        }),
        Entity::Polygon(PathEntity {
            base: base("yapi"),
            pts: vec![tm(50.0, 0.0), tm(60.0, 0.0), tm(55.0, 8.0)],
            bulges: Some(vec![0.0, -0.25, 0.0]),
            holes: None,
            zs: Some(vec![Some(0.0); 3]),
            parts: None,
        }),
        // 8: arcs at several heights: DXF has no 3D polyline with arcs.
        Entity::Polyline(PathEntity {
            base: base("yol"),
            pts: vec![tm(0.0, 40.0), tm(10.0, 40.0), tm(10.0, 50.0)],
            bulges: Some(vec![1.0, 0.0]),
            holes: None,
            zs: Some(vec![Some(1.0), Some(2.0), Some(3.0)]),
            parts: None,
        }),
    ]
}

#[test]
fn elevations_read_back_as_the_same_objects() {
    let objects = elevated();
    let (text, report) = write(&input(objects.clone()));
    let r = read(&text);
    assert_eq!(r.entities.len(), objects.len(), "{:#?}", r.report);
    for (i, (got, sent)) in r.entities.iter().zip(&objects).enumerate() {
        let mut want = expected(sent, &layer_name);
        // Arcs at several heights were written without: the arcs stayed, the heights did not.
        if i == 8 {
            let Entity::Polyline(p) = &mut want else {
                panic!()
            };
            p.zs = None;
        }
        assert_eq!(*got, want, "object {i}");
    }
    assert!(r.report.skipped.is_empty(), "{:?}", r.report.skipped);
    // How many objects have elevations is a fact of the file, as it comes back.
    assert!(
        r.report
            .source
            .iter()
            .any(|f| (f.label.as_str(), f.value.as_str()) == ("Kotlu nesne", "8")),
        "{:?}",
        r.report.source
    );
    // The writer says what it did with them.
    let noted: Vec<(&str, u32)> = report
        .notes
        .iter()
        .map(|n| (n.what.as_str(), n.count))
        .collect();
    let n = |what: &str, part: &str| {
        report
            .notes
            .iter()
            .find(|i| i.what == what && i.reason.contains(part))
            .map(|i| i.count)
    };
    assert_eq!(n("Kot (Z)", "LINE'ın Z'si"), Some(3), "{noted:?}");
    assert_eq!(
        n("Kot (Z)", "3B çoklu çizgi (POLYLINE) olarak"),
        Some(3),
        "{noted:?}"
    );
    assert_eq!(n("Kot (Z)", "köşe kotları aynı"), Some(2), "{noted:?}");
    assert_eq!(n("Kot (Z)", "köşe kotları farklı"), Some(1), "{noted:?}");
    assert_eq!(n("Kotsuz köşe", "0 yazıldı"), Some(3), "{noted:?}");
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
}

#[test]
fn a_path_with_elevations_is_a_3d_polyline_and_a_line_holds_its_ends_heights() {
    let all = elevated();
    let (text, _) = write(&input(all[..6].to_vec()));
    let p = pairs(&text);
    // The lines: each end's Z, 0 where an end has none.
    let lines = entities_of(&p, "LINE");
    let z = |l: &Vec<(i32, &str)>| {
        (
            group(l, 30).map(|s| s.parse::<f64>().expect("z")),
            group(l, 31).map(|s| s.parse::<f64>().expect("z")),
        )
    };
    assert_eq!(z(&lines[0]), (Some(105.5 + 1.0 / 3.0), Some(-107.25)));
    assert_eq!(z(&lines[1]), (Some(0.0), Some(12.0)));
    // The paths and the area's outline and elevated hole: 3D polylines (the flat hole stays a LWPOLYLINE), each closed by its own flag.
    let polylines = entities_of(&p, "POLYLINE");
    assert_eq!(polylines.len(), 4);
    let flags: Vec<&str> = polylines.iter().filter_map(|e| group(e, 70)).collect();
    assert_eq!(flags, ["8", "8", "9", "9"]);
    assert!(polylines.iter().all(|e| group(e, 100) == Some("AcDbEntity")
        && e.contains(&(100, "AcDb3dPolyline"))
        && group(e, 66) == Some("1")));
    assert_eq!(entities_of(&p, "LWPOLYLINE").len(), 1);
    // A vertex has its own Z, 0 where it has none, and every VERTEX and SEQEND has a handle, an owner and a layer of its own.
    let vertices = entities_of(&p, "VERTEX");
    assert_eq!(vertices.len(), 4 + 3 + 4 + 4);
    let first: Vec<f64> = vertices[..4]
        .iter()
        .map(|v| group(v, 30).expect("z").parse().expect("z"))
        .collect();
    assert_eq!(first, [10.0, 0.0, 12.5 + 1.0 / 3.0, -3.0]);
    assert!(
        vertices
            .iter()
            .all(|v| v.contains(&(100, "AcDb3dPolylineVertex"))
                && group(v, 70) == Some("32")
                && group(v, 330).is_some()
                && group(v, 8).is_some())
    );
    let ends = entities_of(&p, "SEQEND");
    assert_eq!(ends.len(), 4);
    // KentOS's data rides between a polyline's header and its vertices: the vertex without an elevation (the
    // second), the label and the attribute.
    let header = &polylines[0];
    assert!(
        header.contains(&(1001, "KENTOS"))
            && header.contains(&(1000, "label"))
            && header.contains(&(1000, "attr"))
    );
    let at = header
        .iter()
        .position(|x| *x == (1000, "noz"))
        .expect("noz");
    assert_eq!(header[at + 1], (1000, "2"));
    // Handles stay unique, owners real, in a drawing of nothing else.
    let handles: Vec<&str> = p.iter().filter(|x| x.0 == 5).map(|x| x.1).collect();
    let unique: HashSet<&str> = handles.iter().copied().collect();
    assert_eq!(unique.len(), handles.len());
    assert!(
        p.iter()
            .filter(|x| x.0 == 330 && x.1 != "0")
            .all(|x| unique.contains(x.1))
    );
}

#[test]
fn a_path_with_arcs_keeps_them_and_holds_one_elevation_at_most() {
    let all = elevated();
    let (text, _) = write(&input(vec![all[6].clone(), all[7].clone(), all[8].clone()]));
    let p = pairs(&text);
    assert!(entities_of(&p, "POLYLINE").is_empty());
    let lw = entities_of(&p, "LWPOLYLINE");
    assert_eq!(lw.len(), 3);
    // At one height it is the LWPOLYLINE's own elevation (38), 0 too (KentOS's data says it is on purpose); at several, none.
    assert_eq!(group(&lw[0], 38), Some("250.5"));
    assert_eq!(group(&lw[1], 38), Some("0.0"));
    assert!(lw[1].contains(&(1000, "z")));
    assert_eq!(group(&lw[2], 38), None);
    // The arcs are there in all three.
    assert!(lw.iter().all(|e| e.iter().any(|x| x.0 == 42)));
}

#[test]
fn what_cannot_be_written_is_left_out_and_said() {
    let bad = |za: f64| {
        Entity::Line(LineEntity {
            base: base("parsel"),
            a: tm(0.0, 0.0),
            b: tm(1.0, 1.0),
            za: Some(za),
            zb: None,
        })
    };
    let path = Entity::Polyline(PathEntity {
        base: base("yol"),
        pts: vec![tm(0.0, 0.0), tm(1.0, 0.0)],
        bulges: None,
        holes: None,
        zs: Some(vec![Some(1.0), Some(f64::INFINITY)]),
        parts: None,
    });
    let (text, report) = write(&input(vec![bad(f64::NAN), path, bad(5.0)]));
    let r = read(&text);
    // The two objects with a height that is no number are not written; the line with a good one is.
    assert_eq!(r.entities.len(), 1);
    assert_eq!(
        report
            .skipped
            .iter()
            .map(|i| (i.what.as_str(), i.count))
            .collect::<Vec<_>>(),
        [("Çizgi", 1), ("Çoklu çizgi", 1)]
    );
    // A list of elevations that does not match the vertices is flat: nothing to write it from.
    let short = Entity::Polyline(PathEntity {
        base: base("yol"),
        pts: vec![tm(0.0, 0.0), tm(1.0, 0.0), tm(2.0, 0.0)],
        bulges: None,
        holes: None,
        zs: Some(vec![Some(1.0)]),
        parts: None,
    });
    let (text, _) = write(&input(vec![short]));
    assert!(entities_of(&pairs(&text), "POLYLINE").is_empty());
}

/// A multi-part area (docs/adr/0143): DXF has no such object, so each part
/// is a closed polyline carrying the object's data, its holes linked to it.
/// Read back, the parts are areas of their own, each with its holes.
#[test]
fn a_multi_part_area_is_one_closed_polyline_a_part() {
    let ring = |x: f64| {
        vec![
            tm(x, 0.0),
            tm(x + 10.0, 0.0),
            tm(x + 10.0, 10.0),
            tm(x, 10.0),
        ]
    };
    let hole = RingGeometry {
        pts: vec![tm(22.0, 2.0), tm(24.0, 2.0), tm(24.0, 4.0)],
        bulges: None,
        zs: None,
    };
    let area = Entity::Polygon(PathEntity {
        base: with("parsel", |b| {
            b.attrs.insert("Ada".into(), "101".into());
            b.label = Some("101/5".into());
        }),
        pts: ring(0.0),
        bulges: None,
        holes: None,
        zs: None,
        parts: Some(vec![kentos_contracts::AreaPart {
            pts: ring(20.0),
            bulges: None,
            holes: Some(vec![hole.clone()]),
            zs: None,
        }]),
    });
    let (text, report) = write(&input(vec![area]));
    assert_eq!(entities_of(&pairs(&text), "LWPOLYLINE").len(), 3);
    assert!(
        report.notes.iter().any(|n| n.what == "Çok parçalı alan"),
        "{:?}",
        report.notes
    );
    assert_eq!(report.counts.get("polygon"), Some(&1));
    let back = read(&text);
    let areas: Vec<&PathEntity> = back
        .entities
        .iter()
        .map(|e| match e {
            Entity::Polygon(p) => p,
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(areas.len(), 2);
    assert_eq!(
        (areas[0].pts.clone(), areas[0].holes.clone()),
        (ring(0.0), None)
    );
    assert_eq!(
        (areas[1].pts.clone(), areas[1].holes.clone()),
        (ring(20.0), Some(vec![hole]))
    );
    for a in areas {
        assert_eq!(
            (
                a.base.attrs.get("Ada").map(String::as_str),
                a.base.label.as_deref()
            ),
            (Some("101"), Some("101/5"))
        );
    }
}

// ── Blocks (docs/adr/0144 §5) ───────────────────────────────────────────

fn block_id(n: u8) -> BlockId {
    let mut b = [0u8; 16];
    b[0] = 0x01;
    b[15] = n;
    BlockId(b)
}

fn insert(layer: &str, block: u8, p: Vec2, scale: f64, rotation: f64, mirror: bool) -> Entity {
    Entity::Insert(InsertEntity {
        base: base(layer),
        block: block_id(block),
        p,
        scale,
        rotation,
        mirror,
    })
}

fn definition(n: u8, name: &str, base_point: Vec2, entities: Vec<Entity>) -> BlockDefinition {
    BlockDefinition {
        id: block_id(n),
        name: name.into(),
        base: base_point,
        entities: entities
            .into_iter()
            .zip(1..)
            .map(|(mut e, k)| {
                e.base_mut().id = k;
                e
            })
            .collect(),
        attributes: Vec::new(),
        description: None,
    }
}

/// A street light: “Direk” holds a pole, a hatched base with a hole and a
/// “Lamba”, which holds a coloured bulb and a line on the Yapı layer; an
/// unused block; the drawing's two inserts of Direk (one turned by a turn
/// degrees cannot hold exactly, mirrored, scaled, coloured, with an
/// attribute) and one of a block the export does not have.
fn lights() -> DxfWriteInput {
    let bulb = Entity::Circle(CircleEntity {
        base: with("", |b| b.color = Some("#F5D90A".into())),
        c: v(0.0, 0.0),
        r: 0.4,
    });
    let arm = Entity::Line(LineEntity {
        base: base("yapi"),
        a: v(-0.5, 0.0),
        b: v(0.5, 0.0),
        za: None,
        zb: None,
    });
    let pole = Entity::Line(LineEntity {
        base: base(""),
        a: v(1.0, 2.0),
        b: v(1.0, 8.0),
        za: None,
        zb: None,
    });
    let foot = Entity::Polygon(PathEntity {
        base: base(""),
        pts: vec![v(0.0, 1.0), v(2.0, 1.0), v(2.0, 3.0), v(0.0, 3.0)],
        bulges: None,
        holes: Some(vec![RingGeometry {
            pts: vec![v(0.5, 1.5), v(1.5, 1.5), v(1.5, 2.5)],
            bulges: None,
            zs: None,
        }]),
        zs: None,
        parts: None,
    });
    let mut first = insert("yapi", 1, tm(10.0, 20.0), 2.5, 0.1, true);
    first.base_mut().color = Some("#E5484D".into());
    first.base_mut().attrs.insert("No".into(), "7".into());
    DxfWriteInput {
        blocks: vec![
            definition(
                1,
                "Direk",
                v(1.0, 2.0),
                vec![
                    pole,
                    foot,
                    insert("", 2, v(1.0, 8.0), 1.0, std::f64::consts::FRAC_PI_2, false),
                ],
            ),
            definition(2, "Lamba", v(0.0, 0.0), vec![bulb, arm]),
            definition(3, "Kullanılmayan", v(0.0, 0.0), vec![pole_like()]),
        ],
        ..input(vec![
            first,
            insert("parsel", 1, tm(30.0, 20.0), 1.0, 0.0, false),
            insert("parsel", 9, tm(40.0, 20.0), 1.0, 0.0, false),
        ])
    }
}

fn pole_like() -> Entity {
    Entity::Point(PointEntity {
        base: base(""),
        p: v(0.0, 0.0),
        z: None,
        parts: None,
    })
}

/// A BLOCK of the file: its name, its record's handle, the groups of its objects.
type Block<'a> = (&'a str, &'a str, Vec<Vec<(i32, &'a str)>>);

/// Each BLOCK of the file, in its order.
fn blocks_of<'a>(p: &[(i32, &'a str)]) -> Vec<Block<'a>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < p.len() {
        if p[i] == (0, "BLOCK") {
            let head_end = p[i + 1..]
                .iter()
                .position(|x| x.0 == 0)
                .map_or(p.len(), |k| i + 1 + k);
            let head = &p[i..head_end];
            let (name, owner) = (group(head, 2).unwrap_or(""), group(head, 330).unwrap_or(""));
            let end = p[i..]
                .iter()
                .position(|x| *x == (0, "ENDBLK"))
                .map_or(p.len(), |k| i + k);
            let mut objects = Vec::new();
            let mut k = head_end;
            while k < end {
                let next = p[k + 1..end]
                    .iter()
                    .position(|x| x.0 == 0)
                    .map_or(end, |j| k + 1 + j);
                objects.push(p[k..next].to_vec());
                k = next;
            }
            out.push((name, owner, objects));
            i = end;
        } else {
            i += 1;
        }
    }
    out
}

/// The blocks are BLOCKs with their records, the ones they hold first, the
/// unused one left out; their objects are the block's, on 0 or their own
/// layer, BYBLOCK where they have no colour or weight of their own; the
/// inserts are INSERTs (mirrored as −Y, turned in degrees); what cannot be
/// written is said. The file still has what AutoCAD needs.
#[test]
fn blocks_are_written_as_blocks_their_inserts_as_inserts() {
    let (text, report) = write(&lights());
    let p = pairs(&text);
    let records: Vec<(&str, &str)> = entities_of(&p, "BLOCK_RECORD")
        .iter()
        .map(|r| (group(r, 2).unwrap_or(""), group(r, 5).unwrap_or("")))
        .collect();
    let names: Vec<&str> = records.iter().map(|r| r.0).collect();
    assert_eq!(names, ["*Model_Space", "*Paper_Space", "Lamba", "Direk"]);
    let blocks = blocks_of(&p);
    let written: Vec<&str> = blocks.iter().map(|b| b.0).collect();
    assert_eq!(written, ["*Model_Space", "*Paper_Space", "Lamba", "Direk"]);
    for (name, owner, objects) in &blocks[2..] {
        let record = records.iter().find(|r| r.0 == *name).map(|r| r.1);
        assert_eq!(Some(*owner), record, "{name}'s BLOCK is its record's");
        for o in objects {
            assert_eq!(group(o, 330), record, "{name}: {o:?}");
        }
    }
    let lamba = &blocks[2].2;
    // The bulb: on 0 with its own colour (true colour 0xF5D90A), BYBLOCK weight; the arm: on its own layer, BYBLOCK colour.
    assert_eq!(
        (
            group(&lamba[0], 8),
            group(&lamba[0], 420),
            group(&lamba[0], 370)
        ),
        (Some("0"), Some("16111882"), Some("-2"))
    );
    assert_eq!(
        (
            group(&lamba[1], 8),
            group(&lamba[1], 62),
            group(&lamba[1], 370)
        ),
        (Some("Yapı"), Some("0"), Some("-2"))
    );
    let direk = &blocks[3].2;
    let nested = direk
        .iter()
        .find(|o| o[0] == (0, "INSERT"))
        .expect("the lamp's insert");
    assert_eq!(
        (group(nested, 2), group(nested, 50)),
        (Some("Lamba"), Some("90.0"))
    );
    let inserts = entities_of(&p, "INSERT");
    let placed: Vec<Vec<(i32, &str)>> = inserts
        .iter()
        .filter(|i| group(i, 330) == Some("17"))
        .cloned()
        .collect();
    assert_eq!(placed.len(), 2, "the unknown block's insert is left out");
    let first = &placed[0];
    assert_eq!(
        [2, 8, 41, 42, 43].map(|c| group(first, c)),
        [
            Some("Direk"),
            Some("Yapı"),
            Some("2.5"),
            Some("-2.5"),
            Some("2.5")
        ]
    );
    let degrees: f64 = group(first, 50).expect("50").parse().expect("degrees");
    assert_eq!(degrees, 0.1 * 180.0 / std::f64::consts::PI);
    // What was not written is said; the unused block is not a note.
    let skipped: Vec<&str> = report.skipped.iter().map(|s| s.reason.as_str()).collect();
    assert_eq!(
        skipped,
        ["bloğunun tanımı dışa aktarılanlarda yok; yazılmadı"]
    );
    assert!(!text.contains("Kullanılmayan"));
    assert_eq!(report.counts.get("insert"), Some(&2));
    assert_eq!(
        report.counts.get("circle"),
        None,
        "a block's objects are not the drawing's"
    );
    // The header's extent holds the inserts' blocks, placed: the mirrored, scaled pole reaches 15 m.
    let at = |name: &str, code: i32| -> f64 {
        let i = p.iter().position(|&x| x == (9, name)).expect(name);
        p[i..]
            .iter()
            .find(|x| x.0 == code)
            .map(|x| x.1.parse().expect("number"))
            .expect("value")
    };
    assert!(
        at("$EXTMIN", 20) < X0 + 20.0 - 14.0,
        "{}",
        at("$EXTMIN", 20)
    );
    let unique: HashSet<&str> = p.iter().filter(|x| x.0 == 5).map(|x| x.1).collect();
    assert_eq!(
        unique.len(),
        p.iter().filter(|x| x.0 == 5).count(),
        "handles are unique"
    );
}

/// KentOS's own file reads back as the same blocks and inserts: the
/// definitions with their objects (a ring back in its polygon, a layer by
/// its DXF name, none of its own as none), the inserts exactly (the turn
/// from KentOS's data, the mirror from −Y, colour and attribute).
#[test]
fn blocks_read_back_as_the_same_blocks() {
    let input = lights();
    let (text, _) = write(&input);
    let r = read(&text);
    let names: Vec<&str> = r.blocks.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(names, ["Lamba", "Direk"]);
    let (lamba, direk) = (&r.blocks[0], &r.blocks[1]);
    let expect = |e: &Entity, layer: &str| {
        let mut e = e.clone();
        e.base_mut().layer_id = layer.into();
        e
    };
    let given = |n: u8| {
        input
            .blocks
            .iter()
            .find(|b| b.id == block_id(n))
            .expect("given")
    };
    assert_eq!(lamba.base, given(2).base);
    assert_eq!(
        lamba.entities,
        [
            expect(&given(2).entities[0], ""),
            expect(&given(2).entities[1], "Yapı")
        ]
    );
    assert_eq!(direk.base, given(1).base);
    assert_eq!(direk.entities[..2], given(1).entities[..2]);
    let Entity::Insert(nested) = &direk.entities[2] else {
        panic!("{:?}", direk.entities[2])
    };
    assert_eq!(
        (
            nested.block,
            nested.p,
            nested.scale,
            nested.rotation,
            nested.mirror
        ),
        (
            lamba.id,
            v(1.0, 8.0),
            1.0,
            std::f64::consts::FRAC_PI_2,
            false
        )
    );
    let inserts: Vec<&InsertEntity> = r
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Insert(i) => Some(i),
            _ => None,
        })
        .collect();
    let Entity::Insert(first) = &input.entities[0] else {
        panic!("an insert")
    };
    assert_eq!(
        (
            inserts[0].block,
            inserts[0].p,
            inserts[0].scale,
            inserts[0].rotation,
            inserts[0].mirror
        ),
        (direk.id, first.p, 2.5, 0.1, true)
    );
    assert_eq!(inserts[0].base.color.as_deref(), Some("#E5484D"));
    assert_eq!(
        inserts[0].base.attrs.get("No").map(String::as_str),
        Some("7")
    );
    assert_eq!(inserts[0].base.layer_id, "Yapı");
    assert_eq!(
        (inserts[1].scale, inserts[1].rotation, inserts[1].mirror),
        (1.0, 0.0, false)
    );
}

/// Names DXF cannot take as they are, and a block that holds itself.
#[test]
fn block_names_dxf_refuses_change_and_a_block_holding_itself_is_written_once() {
    let dot = || pole_like();
    let input = DxfWriteInput {
        blocks: vec![
            definition(1, "Kapi", v(0.0, 0.0), vec![dot()]),
            definition(2, "KAPI", v(0.0, 0.0), vec![dot()]),
            definition(3, "*Adsız", v(0.0, 0.0), vec![dot()]),
            definition(4, "Ağaç/Çınar", v(0.0, 0.0), vec![dot()]),
            definition(
                5,
                "Döngü",
                v(0.0, 0.0),
                vec![dot(), insert("", 5, v(1.0, 0.0), 1.0, 0.0, false)],
            ),
        ],
        ..input(
            (1..=5)
                .map(|n| insert("parsel", n, tm(f64::from(n), 0.0), 1.0, 0.0, false))
                .collect(),
        )
    };
    let (text, report) = write(&input);
    let r = read(&text);
    let names: Vec<&str> = r.blocks.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(names, ["Kapi", "KAPI (2)", "_Adsız", "Ağaç_Çınar", "Döngü"]);
    let notes: Vec<&str> = report
        .notes
        .iter()
        .filter(|n| n.what == "Blok adı")
        .map(|n| n.reason.as_str())
        .collect();
    assert_eq!(
        notes,
        [
            "“KAPI” bloğu “KAPI (2)” adıyla yazıldı (aynı adlı başka bir blok var; DXF blok adları büyük/küçük harf ayırmaz)",
            "“*Adsız” bloğu “_Adsız” adıyla yazıldı (DXF'in kabul etmediği karakterler “_” oldu)",
            "“Ağaç/Çınar” bloğu “Ağaç_Çınar” adıyla yazıldı (DXF'in kabul etmediği karakterler “_” oldu)",
        ]
    );
    assert!(
        report
            .skipped
            .iter()
            .any(|s| s.reason == "“Döngü” bloğu kendini içeriyor; o yerleştirme yazılmadı"),
        "{:?}",
        report.skipped
    );
    assert_eq!(
        r.blocks[4].entities.len(),
        1,
        "written once, without its insert of itself"
    );
    assert_eq!(
        r.entities
            .iter()
            .filter(|e| matches!(e, Entity::Insert(_)))
            .count(),
        5
    );
}

/// The blocks' fixture (`fixtures/formats/v1/dxf-write/blocks.input.json`,
/// written by hand) goes out as its committed bytes: `KENTOS_WRITE_DXF=1`
/// rewrites them (read the difference first). The web's module writes the
/// same bytes (`dxf.wasm.test.ts`), and `scripts/fixtures/dxf_write_reference.py`
/// checks them against the input without KentOS's code.
#[test]
fn the_blocks_fixture_is_written_to_its_committed_bytes() {
    let report = written_as_committed("blocks");
    let notes: Vec<(&str, &str)> = report
        .notes
        .iter()
        .map(|n| (n.what.as_str(), n.reason.as_str()))
        .collect();
    assert_eq!(
        notes,
        [
            (
                "Katman rengi",
                "“Parsel sınırı”: “fg-dim” (ikincil mürekkep) tema rengi DXF'te 8 (gri) oldu"
            ),
            (
                "Blok adı",
                "“Ağaç/Çınar” bloğu “Ağaç_Çınar” adıyla yazıldı (DXF'in kabul etmediği karakterler “_” oldu)"
            ),
            (
                "Adalı alan",
                "adaları ayrı kapalı çoklu çizgiler olarak yazıldı (KentOS'a geri okununca yine adalı alan olur)"
            ),
            (
                "Öznitelik etiketi",
                "“Direk” bloğunun “Kol boyu” etiketi “Kol_boyu” olarak yazıldı (DXF'te etiket boşluk içermez, büyük küçük harf ayırmaz)"
            ),
            (
                "Blok açıklaması",
                "DXF'te açıklama tek satırdır; satır sonları boşluk oldu"
            ),
        ]
    );
    let skipped: Vec<&str> = report.skipped.iter().map(|s| s.reason.as_str()).collect();
    assert_eq!(
        skipped,
        ["bloğunun tanımı dışa aktarılanlarda yok; yazılmadı"]
    );
}

/// A writer fixture's input (`fixtures/formats/v1/dxf-write/<name>.input.json`).
fn fixture_input(name: &str) -> DxfWriteInput {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/formats/v1/dxf-write");
    let json = std::fs::read_to_string(dir.join(format!("{name}.input.json"))).expect("the input");
    dxf::input_from_json(&json).expect("the writer reads it")
}

/// The blocks' fixture's input (`fixtures/formats/v1/dxf-write/blocks.input.json`).
fn blocks_input() -> DxfWriteInput {
    fixture_input("blocks")
}

/// A writer fixture written: its bytes are the committed `<name>.dxf`
/// (`KENTOS_WRITE_DXF=1` rewrites it; read the difference first); its report.
fn written_as_committed(name: &str) -> ExportReport {
    let (bytes, report) = dxf::write(&fixture_input(name));
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/formats/v1/dxf-write")
        .join(format!("{name}.dxf"));
    if std::env::var_os("KENTOS_WRITE_DXF").is_some() {
        std::fs::write(&path, &bytes).expect("written");
    }
    assert!(
        String::from_utf8(bytes).expect("UTF-8")
            == std::fs::read_to_string(&path).expect("the committed file"),
        "the writer's bytes differ from {name}.dxf (KENTOS_WRITE_DXF=1 rewrites it; read the difference first)"
    );
    report
}

/// Whether two objects are the same, each number within a few float64 steps
/// of the other: a value ×1000 and ÷1000 again may land a step away.
fn same_within(a: &Entity, b: &Entity) -> bool {
    fn near(a: &serde_json::Value, b: &serde_json::Value) -> bool {
        use serde_json::Value;
        match (a, b) {
            (Value::Number(x), Value::Number(y)) => {
                let (x, y) = (
                    x.as_f64().unwrap_or(f64::NAN),
                    y.as_f64().unwrap_or(f64::NAN),
                );
                (x - y).abs() <= 4.0 * f64::EPSILON * x.abs().max(y.abs())
            }
            (Value::Array(x), Value::Array(y)) => {
                x.len() == y.len() && x.iter().zip(y).all(|(x, y)| near(x, y))
            }
            (Value::Object(x), Value::Object(y)) => {
                x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| near(v, w)))
            }
            _ => a == b,
        }
    }
    near(
        &serde_json::to_value(a).expect("JSON"),
        &serde_json::to_value(b).expect("JSON"),
    )
}

/// A local project's drawing in millimetres (docs/adr/0165 §2): $INSUNITS 4,
/// every coordinate and length a thousand times its metres, the paper sizes
/// in millimetres too; read back into the same project it is the same
/// drawing in metres, and nothing is said of the unit.
#[test]
fn a_local_project_is_written_in_its_unit_and_read_back_in_metres() {
    let objects = objects();
    let mut mm = input(objects.clone());
    mm.unit = Some(DrawingUnit::Mm);
    let (text, report) = write(&mm);
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
    let (metres, _) = write(&input(objects.clone()));
    let (p, q) = (pairs(&text), pairs(&metres));
    let header = |p: &[(i32, &str)], name: &str, code: i32| -> f64 {
        let i = p.iter().position(|&x| x == (9, name)).expect(name);
        p[i..]
            .iter()
            .find(|x| x.0 == code)
            .map(|x| x.1.parse().expect("number"))
            .expect("value")
    };
    assert_eq!(
        (header(&p, "$INSUNITS", 70), header(&q, "$INSUNITS", 70)),
        (4.0, 6.0)
    );
    assert_eq!(
        header(&p, "$EXTMIN", 10),
        header(&q, "$EXTMIN", 10) * 1000.0
    );
    // The first line's start, and a circle's radius, a thousand times the metres.
    // The first of a kind that has the group.
    let value = |p: &[(i32, &str)], kind: &str, code: i32| -> f64 {
        entities_of(p, kind)
            .iter()
            .find_map(|e| e.iter().find(|x| x.0 == code))
            .map(|x| x.1.parse().expect("number"))
            .expect("value")
    };
    assert_eq!(value(&p, "LINE", 10), value(&q, "LINE", 10) * 1000.0);
    assert_eq!(value(&p, "CIRCLE", 40), value(&q, "CIRCLE", 40) * 1000.0);
    // A line type's dashes: millimetres on paper at 1:1000, in the drawing's millimetres.
    assert_eq!(value(&p, "LTYPE", 49), value(&q, "LTYPE", 49) * 1000.0);
    let r = dxf::read(
        text.as_bytes(),
        &DxfReadOptions {
            unit: Some(DrawingUnit::Mm),
            ..DxfReadOptions::default()
        },
    )
    .expect("reads");
    assert!(
        r.report.notes.iter().all(|n| n.what != "Birim"),
        "{:?}",
        r.report.notes
    );
    let want: Vec<Entity> = objects.iter().map(|e| expected(e, &layer_name)).collect();
    assert_eq!(r.entities.len(), want.len());
    for (got, want) in r.entities.iter().zip(&want) {
        assert!(same_within(got, want), "{got:?}\n{want:?}");
    }
}

/// The millimetre fixture (`fixtures/formats/v1/dxf-write/units.input.json`,
/// docs/adr/0165 §2) goes out as its committed bytes, which
/// `scripts/fixtures/dxf_write_reference.py` checks without KentOS's code:
/// $INSUNITS 4 and every coordinate and length in millimetres.
#[test]
fn the_units_fixture_is_written_to_its_committed_bytes() {
    let report = written_as_committed("units");
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
}

/// The texts' fixture (`fixtures/formats/v1/dxf-write/texts.input.json`,
/// docs/adr/0145 §7) goes out as its committed bytes, which
/// `scripts/fixtures/dxf_write_reference.py` checks without KentOS's code;
/// the masks are said.
#[test]
fn the_texts_fixture_is_written_to_its_committed_bytes() {
    let report = written_as_committed("texts");
    let notes: Vec<(&str, &str, u32)> = report
        .notes
        .iter()
        .map(|n| (n.what.as_str(), n.reason.as_str(), n.count))
        .collect();
    assert_eq!(
        notes,
        [(
            "Yazı zemini",
            "DXF yazısının zemini yoktur: zemin KentOS verisi olarak yazıldı; başka programlar göstermez, KentOS geri okur",
            2
        )]
    );
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
}

/// Texts go out justified (72, 73 and 11; an attribute's 72, 74 and 11),
/// widened (41) and masked (KentOS's data) and come back as they were:
/// every alignment at its point exactly, the block's attribute definition
/// and its insert's value (docs/adr/0145 §7).
#[test]
fn texts_read_back_with_their_alignment_width_factor_and_mask() {
    let input = fixture_input("texts");
    let (text, _) = write(&input);
    let r = read(&text);
    let texts = |entities: &[Entity]| -> Vec<Entity> {
        entities
            .iter()
            .filter(|e| matches!(e, Entity::Text(_)))
            .map(|e| {
                let mut e = e.clone();
                let b = e.base_mut();
                b.id = 0;
                b.layer_id.clear();
                e
            })
            .collect()
    };
    assert_eq!(texts(&r.entities), texts(&input.entities));
    assert!(r.report.notes.is_empty(), "{:?}", r.report.notes);
    let [etiket] = r.blocks.as_slice() else {
        panic!("{:?}", r.blocks)
    };
    assert_eq!(etiket.attributes, input.blocks[0].attributes);
    let Some(Entity::Insert(i)) = r.entities.iter().find(|e| matches!(e, Entity::Insert(_))) else {
        panic!("no insert")
    };
    assert_eq!(i.base.attrs.get("No").map(String::as_str), Some("12"));
}

/// The attributes of the inserts of the block named `name`, in the file's order.
fn insert_values<'a>(r: &'a ImportResult, name: &str) -> Vec<Vec<(&'a str, &'a str)>> {
    let id = r
        .blocks
        .iter()
        .find(|b| b.name == name)
        .map(|b| b.id)
        .expect("the block");
    r.entities
        .iter()
        .filter_map(|e| match e {
            Entity::Insert(i) if i.block == id => Some(
                i.base
                    .attrs
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str()))
                    .collect(),
            ),
            _ => None,
        })
        .collect()
}

/// A block's attribute definitions and its inserts' values go out as ATTDEF
/// and ATTRIB and come back (docs/adr/0144 §7; the blocks fixture's Direk):
/// the tags as DXF wrote them, the values as the inserts showed them (a
/// default shown comes back as the insert's value), the other attributes
/// from KentOS's data, and no ATTRIB comes back as a text of its own.
#[test]
fn attributes_go_out_as_attdef_and_attrib_and_come_back() {
    let input = blocks_input();
    let (text, _) = write(&input);
    let r = read(&text);
    let direk = r.blocks.iter().find(|b| b.name == "Direk").expect("Direk");
    let given = input
        .blocks
        .iter()
        .find(|b| b.name == "Direk")
        .expect("Direk given");
    let back: Vec<(&str, Option<&str>, Option<&str>)> = direk
        .attributes
        .iter()
        .map(|a| (a.tag.as_str(), a.prompt.as_deref(), a.value.as_deref()))
        .collect();
    assert_eq!(
        back,
        [
            ("No", Some("Direk numarası"), Some("?")),
            ("Tür", Some("Lamba türü"), None),
            ("Kol_boyu", None, Some("2.5 m")),
        ]
    );
    for (a, g) in direk.attributes.iter().zip(&given.attributes) {
        assert_eq!(
            (a.p, a.height, a.rotation),
            (g.p, g.height, g.rotation),
            "{}",
            a.tag
        );
    }
    assert_eq!(
        insert_values(&r, "Direk"),
        [
            vec![
                ("Kol_boyu", "2.5 m"),
                ("Malzeme", "Çelik"),
                ("No", "7"),
                ("Tür", "LED")
            ],
            vec![("Kol_boyu", "2.5 m"), ("No", "?"), ("Tür", "")],
        ]
    );
    assert!(
        !r.entities.iter().any(|e| matches!(e, Entity::Text(_))),
        "no ATTRIB comes back as a text"
    );
}

/// An insert's attribute values go out once, as its ATTRIBs: an ATTRIB
/// another program edited comes back edited, and an ATTRIB wins over
/// KentOS's data under its tag (§7).
#[test]
fn an_attrib_edited_elsewhere_wins() {
    let (text, _) = write(&blocks_input());
    // KentOS's data of the first insert holds only its other attribute.
    let p = pairs(&text);
    let first = entities_of(&p, "INSERT")
        .into_iter()
        .find(|e| group(e, 2) == Some("Direk"))
        .expect("Direk's first insert");
    let data: Vec<&str> = first.iter().filter(|g| g.0 == 1000).map(|g| g.1).collect();
    assert!(
        data.contains(&"Malzeme") && !data.contains(&"No") && !data.contains(&"Tür"),
        "{data:?}"
    );
    // Another program sets its No to 8.
    let seven = "AcDbAttribute\r\n  2\r\nNo\r\n";
    let at = text.find("  0\r\nATTRIB\r\n").expect("an ATTRIB");
    let (head, rest) = text.split_at(at);
    let (value, tag) = (
        rest.find("  1\r\n7\r\n").expect("a 7"),
        rest.find(seven).expect("No's ATTRIB"),
    );
    assert!(value < tag, "the first ATTRIB is No's, 7");
    let edited = format!("{head}{}", rest.replacen("  1\r\n7\r\n", "  1\r\n8\r\n", 1));
    let back = read(&edited);
    let values = insert_values(&back, "Direk");
    assert_eq!(
        values[0].iter().find(|(k, _)| *k == "No"),
        Some(&("No", "8"))
    );
    // KentOS's data says No is “Çelik” (a hand edit): the ATTRIB's 7 wins.
    let clash = text.replacen("1000\r\nMalzeme\r\n", "1000\r\nNo\r\n", 1);
    assert_ne!(clash, text);
    let back = read(&clash);
    let values = insert_values(&back, "Direk");
    assert_eq!(
        values[0],
        [("Kol_boyu", "2.5 m"), ("No", "7"), ("Tür", "LED")]
    );
}

/// DXF's attribute texts are one line: a line break in a value, a default
/// or a prompt becomes a space, and is said.
#[test]
fn an_attribute_text_on_more_lines_is_written_on_one_and_said() {
    let mut input = blocks_input();
    let said = |input: &DxfWriteInput| {
        write(input)
            .1
            .notes
            .iter()
            .filter(|n| n.what == "Öznitelik")
            .count()
    };
    assert_eq!(said(&input), 0);
    if let Entity::Insert(i) = &mut input.entities[0] {
        i.base.attrs.insert("No".into(), "7\nA".into());
    }
    let (text, _) = write(&input);
    assert!(text.contains("  1\r\n7 A\r\n"), "the value on one line");
    assert_eq!(said(&input), 1);
    let mut input = blocks_input();
    let direk = input
        .blocks
        .iter_mut()
        .find(|b| b.name == "Direk")
        .expect("Direk");
    direk.attributes[0].prompt = Some("Direk\nnumarası".into());
    assert_eq!(said(&input), 1);
    let r = read(&write(&input).0);
    let back = r.blocks.iter().find(|b| b.name == "Direk").expect("Direk");
    assert_eq!(back.attributes[0].prompt.as_deref(), Some("Direk numarası"));
}

/// The leaders' fixture (`fixtures/formats/v1/dxf-write/leaders.input.json`,
/// docs/adr/0146 §8) goes out as its committed bytes, which
/// `scripts/fixtures/dxf_write_reference.py` checks without KentOS's code;
/// the open and dot arrowheads are said.
#[test]
fn the_leaders_fixture_is_written_to_its_committed_bytes() {
    let report = written_as_committed("leaders");
    let notes: Vec<(&str, &str, u32)> = report
        .notes
        .iter()
        .map(|n| (n.what.as_str(), n.reason.as_str(), n.count))
        .collect();
    assert_eq!(
        notes,
        [(
            "Kılavuz oku",
            "açık ve nokta ok KentOS verisi olarak yazıldı; başka programlar dolu ok gösterir, KentOS geri okur",
            2
        )]
    );
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
}

/// Leaders go out as LEADERs with their notes' MTEXTs and come back as they
/// were: their vertices (the landing's end written as a hookline goes
/// again), notes (one with an escaped backslash and braces, one with a tab
/// MTEXT cannot hold), heights, turns exactly, arrowheads, masks, colour and
/// weight; the block's leader with its note. No note comes as a text of its
/// own (docs/adr/0146 §8).
#[test]
fn leaders_read_back_as_they_were() {
    let input = fixture_input("leaders");
    let (text, _) = write(&input);
    let r = read(&text);
    let leaders = |entities: &[Entity]| -> Vec<Entity> {
        entities
            .iter()
            .filter(|e| matches!(e, Entity::Leader(_)))
            .map(|e| {
                let mut e = e.clone();
                let b = e.base_mut();
                b.id = 0;
                b.layer_id.clear();
                e
            })
            .collect()
    };
    assert_eq!(leaders(&r.entities).len(), 7);
    assert_eq!(leaders(&r.entities), leaders(&input.entities));
    assert!(r.report.notes.is_empty(), "{:?}", r.report.notes);
    assert!(r.entities.iter().all(|e| !matches!(e, Entity::Text(_))));
    let [vana] = r.blocks.as_slice() else {
        panic!("{:?}", r.blocks)
    };
    assert_eq!(leaders(&vana.entities), leaders(&input.blocks[0].entities));
}

/// The new dimensions' fixture (`fixtures/formats/v1/dxf-write/dimensions.input.json`,
/// docs/adr/0147 §8) goes out as its committed bytes, which
/// `scripts/fixtures/dxf_write_reference.py` checks without KentOS's code:
/// an ordinate of the east and one of the north (DIMENSION type 6), an arc
/// length (ARC_DIMENSION), a jogged radius (LARGE_RADIAL_DIMENSION), Semt
/// and Eğim as aligned ones, a masked aligned one; Semt and Eğim are said.
#[test]
fn the_dimensions_fixture_is_written_to_its_committed_bytes() {
    let report = written_as_committed("dimensions");
    let notes: Vec<(&str, &str, u32)> = report
        .notes
        .iter()
        .map(|n| (n.what.as_str(), n.reason.as_str(), n.count))
        .collect();
    assert_eq!(
        notes,
        [(
            "Ölçü",
            "semt ve eğim ölçüleri DXF'te hizalı ölçü olarak, kendi çizgileri ve değeriyle yazıldı; başka programlar çizgilerini gösterir, KentOS ölçü olarak geri okur",
            2
        )]
    );
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
}

/// The new dimensions come back as they were (docs/adr/0147 §8): their
/// kinds, points, centres, offsets, heights, an ordinate's axis, a slope's
/// elevations and the mask; nothing is said.
#[test]
fn the_new_dimensions_read_back_as_they_were() {
    let input = fixture_input("dimensions");
    let (text, _) = write(&input);
    let r = read(&text);
    let dims = |entities: &[Entity]| -> Vec<Entity> {
        entities
            .iter()
            .filter(|e| matches!(e, Entity::Dimension(_)))
            .map(|e| {
                let mut e = e.clone();
                let b = e.base_mut();
                b.id = 0;
                b.layer_id.clear();
                e
            })
            .collect()
    };
    assert_eq!(dims(&r.entities).len(), 7);
    assert_eq!(dims(&r.entities), dims(&input.entities));
    assert!(r.report.notes.is_empty(), "{:?}", r.report.notes);
}

/// Multi-part lines and multi-point objects (docs/adr/0174 §5): DXF has
/// none, so each part goes out as a polyline and each point as a POINT, in
/// the committed bytes `scripts/fixtures/dxf_write_reference.py` checks
/// without KentOS's code; the report says so once for each; read back,
/// every part is a polyline and every point a point of its own with the
/// object's layer, colour, attributes and label.
#[test]
fn multi_part_lines_and_points_go_out_part_by_part() {
    let report = written_as_committed("parts");
    let notes: Vec<&str> = report.notes.iter().map(|n| n.what.as_str()).collect();
    assert!(notes.contains(&"Çok parçalı çoklu çizgi"), "{notes:?}");
    assert!(notes.contains(&"Çok noktalı nesne"), "{notes:?}");
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
    let input = fixture_input("parts");
    let (text, _) = write(&input);
    let r = read(&text);
    let paths: Vec<&kentos_contracts::PathEntity> = r
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Polyline(p) => Some(p),
            _ => None,
        })
        .collect();
    let Entity::Polyline(road) = &input.entities[0] else {
        panic!("a polyline")
    };
    // The road's four parts, then the one-part polyline.
    assert_eq!(paths.len(), 5);
    for (k, p) in paths[..4].iter().enumerate() {
        assert_eq!(p.base.attrs, road.base.attrs, "part {k}");
        assert_eq!(p.base.label, road.base.label, "part {k}");
        assert_eq!(p.base.color, road.base.color, "part {k}");
        assert!(p.parts.is_none(), "part {k}");
    }
    let parts = road.parts.as_deref().expect("parts");
    assert_eq!(paths[1].pts, parts[0].pts);
    assert_eq!(paths[2].bulges.as_deref().map(|b| b[0]), Some(0.5));
    assert_eq!(paths[3].zs, parts[2].zs);
    let points: Vec<&kentos_contracts::PointEntity> = r
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Point(p) => Some(p),
            _ => None,
        })
        .collect();
    assert_eq!(points.len(), 4);
    assert_eq!(
        points[..3].iter().map(|p| p.z).collect::<Vec<_>>(),
        [Some(850.5), None, Some(0.0)]
    );
    assert!(points.iter().all(|p| p.parts.is_none()));
    assert_eq!(points[1].base.label.as_deref(), Some("N-1"));
}

/// The multi-line texts' fixture (`fixtures/formats/v1/dxf-write/paragraphs.input.json`,
/// docs/adr/0182 §5) goes out as its committed bytes, which
/// `scripts/fixtures/dxf_write_reference.py` checks without KentOS's code;
/// the baseline alignment moved to the top and the raised letters a stack
/// cannot hold are said.
#[test]
fn the_paragraphs_fixture_is_written_to_its_committed_bytes() {
    let report = written_as_committed("paragraphs");
    let notes: Vec<(&str, u32)> = report
        .notes
        .iter()
        .map(|n| (n.what.as_str(), n.count))
        .collect();
    assert_eq!(
        notes,
        [("Çok satırlı yazı", 1), ("Çok satırlı yazı", 1)],
        "{:?}",
        report.notes
    );
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
}

/// Multi-line texts go out as MTEXT and come back as they were (docs/adr/0182
/// §5): their lines, box, spacing, width factor, mask, turn and letter
/// formats exactly; a baseline alignment as the top's, where the text stood;
/// raised letters a stack cannot hold on the line; a block's own.
#[test]
fn paragraphs_read_back_as_they_were() {
    let input = fixture_input("paragraphs");
    let (text, _) = write(&input);
    let r = read(&text);
    let texts = |entities: &[Entity]| -> Vec<TextEntity> {
        entities
            .iter()
            .filter_map(|e| match e {
                Entity::Text(t) => {
                    let mut t = t.clone();
                    t.base.id = 0;
                    t.base.layer_id.clear();
                    Some(t)
                }
                _ => None,
            })
            .collect()
    };
    let (got, want) = (texts(&r.entities), texts(&input.entities));
    assert_eq!(got.len(), 5);
    for i in [0, 1, 2] {
        assert_eq!(got[i], want[i], "text {}", i + 1);
    }
    // The baseline's left went out as the top's left: the same text where it stood.
    assert_eq!(
        (got[3].text.as_str(), got[3].align),
        (
            want[3].text.as_str(),
            Some(kentos_contracts::TextAlign::TopLeft)
        )
    );
    assert!(
        (got[3].p.y - (want[3].p.y + want[3].height)).abs() < 1e-9,
        "{:?}",
        got[3].p
    );
    // “1/2” cannot be stacked: it reads back on the line, the other run kept.
    assert_eq!(got[4].text, want[4].text);
    assert_eq!(got[4].paragraph.runs, want[4].paragraph.runs[1..]);
    let [note] = r.blocks.as_slice() else {
        panic!("{:?}", r.blocks)
    };
    let [Entity::Text(inside)] = note.entities.as_slice() else {
        panic!("{:?}", note.entities)
    };
    let Entity::Text(given) = &input.blocks[0].entities[0] else {
        panic!()
    };
    assert_eq!(
        (&inside.text, &inside.paragraph, inside.align),
        (&given.text, &given.paragraph, given.align)
    );
    assert!(r.report.notes.is_empty(), "{:?}", r.report.notes);
}

/// The styles' fixture (`fixtures/formats/v1/dxf-write/styles.input.json`,
/// docs/adr/0183 §7) goes out as its committed bytes, which
/// `scripts/fixtures/dxf_write_reference.py` checks without KentOS's code;
/// the open arrow and the dot without DXF's arrowhead blocks are said once.
#[test]
fn the_styles_fixture_is_written_to_its_committed_bytes() {
    let report = written_as_committed("styles");
    let notes: Vec<(&str, u32)> = report
        .notes
        .iter()
        .map(|n| (n.what.as_str(), n.count))
        .collect();
    assert_eq!(notes, [("Ölçü stili", 2)], "{:?}", report.notes);
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
}

/// The objects' styles go out as STYLE and DIMSTYLE records and come back
/// as they were (docs/adr/0183 §7): the styles the objects follow, each as
/// the project had it (its KENTOS data), under the reader's ids in the order
/// met (the block's first) and not one the objects do not follow; every
/// text's face and every dimension's look, their own changes too; a
/// styleless face without a style, Standard as Standart.
#[test]
fn styles_read_back_as_they_were() {
    let input = fixture_input("styles");
    let (text, _) = write(&input);
    let r = read(&text);
    let names = |v: Vec<&str>| v.into_iter().map(str::to_owned).collect::<Vec<_>>();
    assert_eq!(
        r.text_styles
            .iter()
            .map(|s| s.name.clone())
            .collect::<Vec<_>>(),
        names(vec!["Ada no", "Yol adı", "Not"])
    );
    assert_eq!(
        r.dimension_styles
            .iter()
            .map(|s| s.name.clone())
            .collect::<Vec<_>>(),
        names(vec!["Mimari", "Kadastro", "Noktalı", "Açık", "Oksuz"])
    );
    // The reader's id → the project's.
    let mut ids = BTreeMap::new();
    for (i, s) in r.text_styles.iter().enumerate() {
        assert_eq!(s.id, format!("dxf-text-{}", i + 1));
        let given = input
            .text_styles
            .iter()
            .find(|g| g.name == s.name)
            .expect("given");
        assert_eq!(
            kentos_contracts::TextStyleDef {
                id: given.id.clone(),
                ..s.clone()
            },
            *given
        );
        ids.insert(s.id.clone(), given.id.clone());
    }
    for (i, s) in r.dimension_styles.iter().enumerate() {
        assert_eq!(s.id, format!("dxf-dim-{}", i + 1));
        let given = input
            .dimension_styles
            .iter()
            .find(|g| g.name == s.name)
            .expect("given");
        assert_eq!(
            kentos_contracts::DimensionStyleDef {
                id: given.id.clone(),
                ..s.clone()
            },
            *given
        );
        ids.insert(s.id.clone(), given.id.clone());
    }
    let ours = |entities: &[Entity]| -> Vec<Entity> {
        entities
            .iter()
            .map(|e| {
                let mut e = e.clone();
                let base = e.base_mut();
                base.id = 0;
                base.layer_id.clear();
                match &mut e {
                    Entity::Text(t) => {
                        t.face.text_style = t
                            .face
                            .text_style
                            .take()
                            .map(|id| ids.get(&id).cloned().unwrap_or(id));
                    }
                    Entity::Dimension(d) => {
                        d.look.dim_style = d
                            .look
                            .dim_style
                            .take()
                            .map(|id| ids.get(&id).cloned().unwrap_or(id));
                    }
                    _ => {}
                }
                e
            })
            .collect()
    };
    let given: Vec<Entity> = ours(&input.entities)
        .into_iter()
        .filter(|e| !matches!(e, Entity::Insert(_)))
        .collect();
    let got: Vec<Entity> = ours(&r.entities)
        .into_iter()
        .filter(|e| !matches!(e, Entity::Insert(_)))
        .collect();
    assert_eq!(got.len(), given.len());
    for (i, (g, w)) in got.iter().zip(&given).enumerate() {
        assert_eq!(g, w, "object {}", i + 1);
    }
    let [block] = r.blocks.as_slice() else {
        panic!("{:?}", r.blocks)
    };
    assert_eq!(ours(&block.entities), ours(&input.blocks[0].entities));
    assert!(r.report.notes.is_empty(), "{:?}", r.report.notes);
}

/// The tables' fixture (`fixtures/formats/v1/dxf-write/tables.input.json`,
/// docs/adr/0184 §7) goes out as its committed bytes, which
/// `scripts/fixtures/dxf_write_reference.py` checks without KentOS's code:
/// each table an anonymous block of lines and texts and its INSERT, said once
/// for the three.
#[test]
fn the_tables_fixture_is_written_to_its_committed_bytes() {
    let report = written_as_committed("tables");
    let notes: Vec<(&str, u32)> = report
        .notes
        .iter()
        .map(|n| (n.what.as_str(), n.count))
        .collect();
    assert_eq!(notes, [("Tablo", 3)], "{:?}", report.notes);
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
}

/// Tables go out as anonymous blocks with their KENTOS data and come back as
/// they were (docs/adr/0184 §7): cells, sizes, merged range, alignments,
/// heading row, lines, frame, face, colour, label and attributes; a file's
/// source kept, a schedule's dropped (the objects read have ids of their own).
#[test]
fn tables_read_back_as_they_were() {
    let input = fixture_input("tables");
    let (text, _) = write(&input);
    let r = read(&text);
    let tables = |entities: &[Entity]| -> Vec<kentos_contracts::TableEntity> {
        entities
            .iter()
            .filter_map(|e| match e {
                Entity::Table(t) => {
                    let mut t = t.clone();
                    t.base.id = 0;
                    t.base.layer_id.clear();
                    t.base.line_weight = None;
                    Some(t)
                }
                _ => None,
            })
            .collect()
    };
    let (got, mut want) = (tables(&r.entities), tables(&input.entities));
    assert_eq!(got.len(), 3, "{:?}", r.report);
    want[0].source = None;
    assert_eq!(got, want);
    assert!(r.blocks.is_empty(), "{:?}", r.blocks);
}

/// The hatches' fixture (`fixtures/formats/v1/dxf-write/hatches.input.json`,
/// docs/adr/0186 §9) goes out as its committed bytes, which
/// `scripts/fixtures/dxf_write_reference.py` checks without KentOS's code:
/// patterns turned and scaled, gradients by name with their two colours, the
/// tie left out and said once.
#[test]
fn the_hatches_fixture_is_written_to_its_committed_bytes() {
    let report = written_as_committed("hatches");
    let notes: Vec<(&str, &str, u32)> = report
        .notes
        .iter()
        .map(|n| (n.what.as_str(), n.reason.as_str(), n.count))
        .collect();
    assert_eq!(
        notes,
        [(
            "Tarama",
            "ilişkili taramalar ilişkisiz yazıldı; DXF'te sınır nesnelerini izlemezler",
            1
        )],
        "{:?}",
        report.notes
    );
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
}

/// Two JSON values the same, each number to a billionth (of 1 at least).
fn close_json(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    use serde_json::Value;
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (
                x.as_f64().unwrap_or(f64::NAN),
                y.as_f64().unwrap_or(f64::NAN),
            );
            (x - y).abs() <= 1e-9 * x.abs().max(y.abs()).max(1.0)
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(x, y)| close_json(x, y))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| close_json(v, w)))
        }
        _ => a == b,
    }
}

/// Patterns and gradients come back as they went out (docs/adr/0186 §9):
/// exactly from KentOS's data; without it (another program dropped it)
/// from the groups to a billionth, an inverted linear gradient the linear
/// one turned half round. A tie does not come back: the file's hatch
/// follows nothing.
#[test]
fn hatches_read_back_as_they_were() {
    let input = fixture_input("hatches");
    let (text, _) = write(&input);
    let hatches = |entities: &[Entity]| -> Vec<HatchEntity> {
        entities
            .iter()
            .filter_map(|e| match e {
                Entity::Hatch(h) => {
                    let mut h = h.clone();
                    h.base.id = 0;
                    h.base.layer_id.clear();
                    h.base.line_weight = None;
                    h.assoc = None;
                    Some(h)
                }
                _ => None,
            })
            .collect()
    };
    let want = hatches(&input.entities);
    let r = read(&text);
    assert_eq!(hatches(&r.entities), want);
    assert!(r.report.notes.is_empty(), "{:?}", r.report.notes);
    // Without KentOS's exact pattern the groups say it.
    let bare = read(&text.replace("\n1000\r\nhatch\r\n", "\n1000\r\nhatch-\r\n"));
    assert_ne!(bare.entities, r.entities);
    let mut want = want;
    let linear = &mut want[2].pattern;
    linear.angle = 210.0;
    linear.gradient.as_mut().expect("gradient").inverted = false;
    let got = hatches(&bare.entities);
    assert_eq!(got.len(), want.len());
    for (g, w) in got.iter().zip(&want) {
        let (g, w) = (
            serde_json::to_value(g).expect("JSON"),
            serde_json::to_value(w).expect("JSON"),
        );
        assert!(close_json(&g, &w), "{g}\n{w}");
    }
}
