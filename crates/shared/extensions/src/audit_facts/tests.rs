use super::*;

fn layer(invalid: Vec<InvalidGeometry>, overlaps: Vec<Overlap>, gaps: Vec<Gap>) -> LayerFacts {
    LayerFacts {
        id: "parsel".into(),
        name: "Parsel".into(),
        objects: ObjectCounts {
            areas: 3,
            ..ObjectCounts::default()
        },
        area: 1_234.567_891,
        length: -0.0,
        totals: Totals::default(),
        invalid,
        overlaps,
        gaps,
    }
}

fn invalid(uid: &str, problem: ValidityProblem, at: [f64; 2]) -> InvalidGeometry {
    InvalidGeometry {
        uid: uid.into(),
        problem,
        at,
    }
}

fn overlap(a: &str, b: &str, area: f64, at: [f64; 2]) -> Overlap {
    Overlap {
        a: a.into(),
        b: b.into(),
        area,
        at,
    }
}

fn gap(area: f64, at: [f64; 2], around: &[&str]) -> Gap {
    Gap {
        area,
        at,
        around: around.iter().map(|s| (*s).to_owned()).collect(),
    }
}

/// The written-down rules: areas to 0.0001 m², places to 1 mm, −0 and what is not a number as 0; each list in its
/// order whatever order it was measured in, an overlap's pair the smaller id first.
#[test]
fn the_facts_are_rounded_and_ordered_as_written_down() {
    let made = ProjectAuditFacts::new(
        0.001,
        vec![layer(
            vec![
                invalid("u2", ValidityProblem::RingCrossing, [5.0004, 1.0]),
                invalid("u1", ValidityProblem::ZeroArea, [f64::NAN, -0.0]),
                invalid("u1", ValidityProblem::Repeated, [3.0, 4.0]),
            ],
            vec![
                overlap("u3", "u2", 49.999_96, [17.5, 5.0]),
                overlap("u1", "u3", 0.000_04, [0.1234, 0.0]),
            ],
            vec![
                gap(10.0, [1.0, 1.0], &["u1"]),
                gap(100.000_05, [15.0, 15.0], &["u4", "u2"]),
                gap(5.0, [0.0, 1.0], &["u2"]),
            ],
        )],
        false,
    );
    assert_eq!(made.format, FORMAT);
    assert_eq!(made.schema_version, VERSION);
    let l = &made.layers[0];
    assert_eq!(l.area, 1234.5679);
    assert!(l.length == 0.0 && l.length.is_sign_positive(), "−0 is 0");
    let problems: Vec<(&str, &str)> = l
        .invalid
        .iter()
        .map(|x| (x.uid.as_str(), x.problem.key()))
        .collect();
    assert_eq!(
        problems,
        [
            ("u1", "repeated"),
            ("u1", "zeroArea"),
            ("u2", "ringCrossing")
        ]
    );
    assert_eq!(l.invalid[1].at, [0.0, 0.0], "what is not a number is 0");
    assert_eq!(l.invalid[2].at, [5.0, 1.0], "a millimetre's place");
    let pairs: Vec<(&str, &str, f64)> = l
        .overlaps
        .iter()
        .map(|o| (o.a.as_str(), o.b.as_str(), o.area))
        .collect();
    assert_eq!(pairs, [("u1", "u3", 0.0), ("u2", "u3", 50.0)]);
    assert_eq!(l.overlaps[0].at, [0.123, 0.0]);
    let gaps: Vec<(f64, [f64; 2])> = l.gaps.iter().map(|g| (g.area, g.at)).collect();
    assert_eq!(
        gaps,
        [
            (100.0001, [15.0, 15.0]),
            (5.0, [0.0, 1.0]),
            (10.0, [1.0, 1.0])
        ],
        "north to south, then west to east"
    );
    assert_eq!(l.gaps[0].around, ["u4", "u2"], "the neighbours as measured");
    assert_eq!(
        l.totals,
        Totals {
            invalid: 3,
            overlaps: 2,
            gaps: 3
        }
    );
    // Doing it again changes nothing: the same bytes.
    let again = ProjectAuditFacts::new(made.tolerance, made.layers.clone(), made.cut);
    assert_eq!(again.to_json(), made.to_json());
}

/// A list holds at most `MOST_ITEMS`, the first in its order; `totals` counts all of them, also when the facts are
/// normalised again.
#[test]
fn a_list_is_cut_and_its_total_counts_all() {
    assert_eq!(MOST_ITEMS, 10_000, "the schema's words say 10 000");
    let many: Vec<InvalidGeometry> = (0..MOST_ITEMS + 5)
        .rev()
        .map(|i| invalid(&format!("u{i:06}"), ValidityProblem::Repeated, [0.0, 0.0]))
        .collect();
    let l = layer(many, Vec::new(), Vec::new()).normalized();
    assert_eq!(l.invalid.len(), MOST_ITEMS);
    assert_eq!(l.totals.invalid, (MOST_ITEMS + 5) as u64);
    assert_eq!(l.invalid[0].uid, "u000000", "the first by id are kept");
    assert_eq!(l.clone().normalized(), l);
}

/// The JSON a rule reads: camelCase keys, a problem by its key, places as [x, y]; it reads back as written.
#[test]
fn the_json_is_the_documented_shape_and_reads_back() {
    let made = ProjectAuditFacts::new(
        0.001,
        vec![layer(
            vec![invalid("u1", ValidityProblem::HolesOverlap, [1.0, 2.0])],
            Vec::new(),
            Vec::new(),
        )],
        true,
    );
    let text = made.to_json();
    let head = concat!(
        r#"{"format":"kentos.auditFacts","schemaVersion":1,"tolerance":0.001,"#,
        r#""layers":[{"id":"parsel""#
    );
    assert!(text.starts_with(head), "{text}");
    assert!(
        text.contains(r#""invalid":[{"uid":"u1","problem":"holesOverlap","at":[1.0,2.0]}]"#),
        "{text}"
    );
    assert!(text.ends_with(r#""cut":true}"#), "{text}");
    let read: ProjectAuditFacts = serde_json::from_str(&text).unwrap();
    assert_eq!(read, made);
    for p in ValidityProblem::ALL {
        assert_eq!(ValidityProblem::of_key(p.key()), Some(p));
        assert_eq!(serde_json::to_value(p).unwrap(), p.key());
    }
    assert_eq!(ValidityProblem::of_key("unclosed"), None);
}

/// The committed `factsSchema` is the one the facts' types make: a fact added is written there by
/// `KENTOS_WRITE_SDK=1 cargo test -p kentos-extensions --lib -- sdk_pack` (this test's name holds `sdk_pack` so that
/// the SDK's one command writes it with the other parts). The facts KentOS writes meet it; another problem does not.
#[cfg(feature = "schema")]
#[test]
fn the_sdk_pack_facts_schema_is_the_facts_types() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let made = serde_json::to_string_pretty(&json_schema()).unwrap() + "\n";
    if std::env::var_os("KENTOS_WRITE_SDK").is_some() {
        std::fs::write(root.join("sdk/kentos-audit-facts.schema.json"), &made).expect("the schema");
        return;
    }
    assert!(
        crate::sdk_pack::FACTS_SCHEMA == made,
        "the facts' types changed: KENTOS_WRITE_SDK=1 cargo test -p kentos-extensions --lib -- sdk_pack"
    );
    let schema = crate::sdk_pack::sdk_part("factsSchema").expect("the part");
    assert_eq!(schema["properties"]["format"]["const"], FORMAT);
    let facts = ProjectAuditFacts::new(
        0.001,
        vec![layer(
            vec![invalid("u1", ValidityProblem::RingCrossing, [1.0, 2.0])],
            vec![overlap("u1", "u2", 2.5, [1.0, 1.0])],
            vec![gap(4.0, [3.0, 3.0], &["u1", "u2"])],
        )],
        false,
    );
    let value = serde_json::to_value(&facts).unwrap();
    let errors = crate::schema_check::errors(&schema, &value);
    assert!(errors.is_empty(), "{errors:#?}");
    let mut odd = value.clone();
    odd["layers"][0]["invalid"][0]["problem"] = serde_json::json!("unclosed");
    assert!(!crate::schema_check::errors(&schema, &odd).is_empty());
    let mut short = value;
    short["layers"][0]["gaps"][0]["at"] = serde_json::json!([3.0]);
    assert!(!crate::schema_check::errors(&schema, &short).is_empty());
}
