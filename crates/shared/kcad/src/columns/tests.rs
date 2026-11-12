use super::*;

fn base(layer: &str) -> EntityBase {
    EntityBase {
        look: Default::default(),
        id: 7,
        layer_id: layer.into(),
        color: None,
        attrs: BTreeMap::new(),
        label: None,
        symbol: None,
        line_weight: None,
        label_pins: Vec::new(),
        attachments: Vec::new(),
    }
}

fn id(n: u8) -> EntityId {
    let mut b = [0u8; 16];
    b[15] = n;
    b[6] = 0x70;
    EntityId(b)
}

#[test]
fn a_point_is_laid_out_as_the_table_says() {
    let mut b = base("0");
    b.label = Some("P1".into());
    b.attrs.insert("Ad".into(), "Nokta".into());
    let p = Entity::Point(PointEntity {
        base: b,
        p: Vec2 { x: -0.0, y: 2.5 },
        z: Some(12.0),
        parts: None,
    });
    let cols = pack(vec![p], vec![id(1)]);
    assert_eq!(cols.kinds, [0]);
    assert_eq!(cols.ints, [1, 0, LABEL | OPT[0], 1]);
    assert_eq!(cols.floats[0].to_bits(), (-0.0f64).to_bits());
    assert_eq!(&cols.floats[1..], [2.5, 12.0]);
    let texts: Vec<String> = {
        let mut at = 0;
        cols.text_lengths
            .iter()
            .map(|&n| {
                let s = String::from_utf16(&cols.text[at..at + n as usize]).unwrap_or_default();
                at += n as usize;
                s
            })
            .collect()
    };
    assert_eq!(texts, ["0", "P1", "Ad", "Nokta"]);
}

#[test]
fn unpacking_gives_back_the_objects_and_refuses_what_does_not_hold_together() {
    let entities = vec![
        Entity::Line(LineEntity {
            ma: None,
            mb: None,
            base: base("a"),
            a: Vec2 { x: 1.0, y: 2.0 },
            b: Vec2 { x: 3.0, y: 4.0 },
            za: None,
            zb: None,
        }),
        Entity::Hatch(HatchEntity {
            boundary: None,
            base: base("b"),
            ring: vec![Vec2 { x: 0.0, y: 0.0 }; 3],
            holes: Some(vec![vec![Vec2 { x: 1.0, y: 1.0 }; 3]]),
            pattern: HatchPattern::user(HatchPatternType::Cross, 45.0, 2.0),
            assoc: None,
            annotative: None,
        }),
        // A pattern of families, tied to its objects (docs/adr/0186).
        Entity::Hatch(HatchEntity {
            boundary: None,
            base: base("b"),
            ring: vec![Vec2 { x: 0.0, y: 0.0 }; 3],
            holes: None,
            pattern: HatchPattern {
                name: Some("ANSI33".into()),
                scale: Some(0.5),
                lines: Some(vec![
                    PatternLine {
                        angle: 45.0,
                        origin: [0.0, 0.0],
                        offset: [0.0, 6.35],
                        dashes: Vec::new(),
                    },
                    PatternLine {
                        angle: 45.0,
                        origin: [4.49, -0.0],
                        offset: [0.0, 6.35],
                        dashes: vec![3.175, -1.5875, 0.0],
                    },
                ]),
                ..HatchPattern::user(HatchPatternType::Pattern, 30.0, 1.0)
            },
            assoc: Some(HatchAssoc {
                gap: None,
                outer: Some(id(1)),
                bounds: Vec::new(),
                islands: vec![id(2)],
                cutouts: Vec::new(),
                seed: Vec2 { x: 1.0, y: 0.5 },
            }),
            annotative: None,
        }),
        Entity::Hatch(HatchEntity {
            boundary: None,
            base: base("b"),
            ring: vec![Vec2 { x: 0.0, y: 0.0 }; 3],
            holes: None,
            pattern: HatchPattern {
                gradient: Some(HatchGradient {
                    shape: GradientShape::Cylinder,
                    inverted: true,
                    color2: "#FFFFFF".into(),
                }),
                ..HatchPattern::user(HatchPatternType::Gradient, 90.0, 1.0)
            },
            assoc: None,
            annotative: None,
        }),
    ];
    let cols = pack(entities.clone(), vec![id(1), id(2), id(3), id(4)]);
    let (back, uids) = unpack(&cols).expect("unpacks");
    assert_eq!(uids, [id(1), id(2), id(3), id(4)]);
    assert_eq!(first_difference(&entities, &uids, &back, &uids), None);
    assert_eq!(differs(&cols, &back, &uids), None);
    // Slots are the reader's: 1, 2 …
    assert_eq!(back[1].base().id, 2);

    let mut short = cols.clone();
    short.floats.pop();
    assert_eq!(
        unpack(&short).map(|_| ()).unwrap_err().code,
        Code::BadColumns
    );
    let mut extra = cols.clone();
    extra.ints.push(0);
    assert_eq!(
        unpack(&extra).map(|_| ()).unwrap_err().code,
        Code::BadColumns
    );
    let mut kind = cols.clone();
    kind.kinds[0] = 13;
    assert_eq!(
        unpack(&kind).map(|_| ()).unwrap_err().code,
        Code::BadColumns
    );
    let mut lone = cols;
    lone.text[0] = 0xd800;
    let e = unpack(&lone).map(|_| ()).unwrap_err();
    assert_eq!(e.code, Code::InvalidUtf8);
}

#[test]
fn elevations_are_laid_out_as_the_table_says() {
    let line = Entity::Line(LineEntity {
        ma: None,
        mb: None,
        base: base("0"),
        a: Vec2 { x: 1.0, y: 2.0 },
        b: Vec2 { x: 3.0, y: 4.0 },
        za: None,
        zb: Some(7.5),
    });
    let pts = vec![
        Vec2 { x: 0.0, y: 0.0 },
        Vec2 { x: 1.0, y: 1.0 },
        Vec2 { x: 2.0, y: 0.0 },
    ];
    let polygon = Entity::Polygon(PathEntity {
        space_separator: None,
        ms: None,
        base: base("0"),
        pts: pts.clone(),
        bulges: Some(vec![0.5]),
        holes: Some(vec![
            RingGeometry {
                pts: pts.clone(),
                bulges: None,
                zs: Some(vec![None, Some(3.0), Some(-0.0)]),
            },
            RingGeometry {
                pts,
                bulges: Some(vec![0.25]),
                zs: None,
            },
        ]),
        zs: Some(vec![Some(10.0), None, Some(12.0)]),
        parts: None,
        closed: false,
        widths: None,
    });
    let entities = vec![line, polygon];
    let uids = vec![id(1), id(2)];
    let cols = pack(entities.clone(), uids.clone());
    // The layer table (1); the line: layer, flags (zb), attributes; the polygon: layer, flags
    // (bulges, holes, elevations), attributes, n, m, e, h, then per hole its flags, k, j?, d?.
    assert_eq!(
        cols.ints,
        [
            1,
            0,
            OPT[1],
            0,
            0,
            OPT[0] | OPT[1] | OPT[2],
            0,
            3,
            1,
            3,
            2,
            HOLE_ELEVATIONS,
            3,
            3,
            HOLE_BULGES,
            3,
            1
        ]
    );
    // A vertex without an elevation is NaN, and only there.
    let nans: Vec<usize> = (0..cols.floats.len())
        .filter(|&i| cols.floats[i].is_nan())
        .collect();
    assert_eq!(nans, [13, 21]);
    assert_eq!(&cols.floats[..5], [1.0, 2.0, 3.0, 4.0, 7.5]);
    assert_eq!(&cols.floats[11..13], [0.5, 10.0]);
    assert_eq!(cols.floats[14], 12.0);
    assert_eq!(cols.floats[22], 3.0);
    assert_eq!(cols.floats[23].to_bits(), (-0.0f64).to_bits());

    let (back, back_uids) = unpack(&cols).expect("unpacks");
    assert_eq!(first_difference(&entities, &uids, &back, &back_uids), None);
    assert_eq!(differs(&cols, &back, &back_uids), None);
    let Entity::Polygon(p) = &back[1] else {
        panic!("a polygon")
    };
    assert_eq!(p.zs, Some(vec![Some(10.0), None, Some(12.0)]));
    let holes = p.holes.as_ref().expect("holes");
    assert_eq!(holes[0].zs.as_ref().map(Vec::len), Some(3));
    assert_eq!(holes[0].zs.as_ref().map(|z| z[0]), Some(None));
    assert_eq!(holes[1].zs, None);
    let Entity::Line(l) = &back[0] else {
        panic!("a line")
    };
    assert_eq!((l.za, l.zb), (None, Some(7.5)));

    // Every NaN is the same NaN: the page's engine may write another bit pattern for none.
    let mut other = cols.clone();
    other.floats[13] = f64::from_bits(0xfff8_0000_0000_0001);
    assert_eq!(differs(&other, &back, &back_uids), None);
    let (again, _) = unpack(&other).expect("unpacks");
    assert_eq!(
        first_difference(&back, &back_uids, &again, &back_uids),
        None
    );
    // A number of an elevation is a difference all the same.
    other.floats[14] = 12.5;
    assert!(differs(&other, &back, &back_uids).is_some());

    // Holes with flags no hole has, or lists running past the numbers, are refused.
    let mut flags = cols.clone();
    flags.ints[11] = 4;
    assert_eq!(
        unpack(&flags).map(|_| ()).unwrap_err().code,
        Code::BadColumns
    );
    let mut long = cols;
    long.ints[9] = 40;
    assert_eq!(
        unpack(&long).map(|_| ()).unwrap_err().code,
        Code::BadColumns
    );
}

#[test]
fn a_changed_bit_or_field_is_a_difference() {
    let line = |x: f64| {
        Entity::Line(LineEntity {
            ma: None,
            mb: None,
            base: base("a"),
            a: Vec2 { x, y: 0.0 },
            b: Vec2 { x: 1.0, y: 1.0 },
            za: None,
            zb: None,
        })
    };
    let cols = pack(vec![line(0.0)], vec![id(1)]);
    assert_eq!(differs(&cols, &[line(0.0)], &[id(1)]), None);
    assert!(differs(&cols, &[line(-0.0)], &[id(1)]).is_some());
    assert!(differs(&cols, &[line(0.0)], &[id(2)]).is_some());
    assert_eq!(
        first_difference(&[line(0.0)], &[id(1)], &[line(-0.0)], &[id(1)]),
        Some(0)
    );
    let mut labelled = line(0.0);
    if let Entity::Line(l) = &mut labelled {
        l.base.label = Some("x".into());
    }
    assert!(differs(&cols, &[labelled], &[id(1)]).is_some());
}
