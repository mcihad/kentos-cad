//! Çatı kesitte ve görünüşte (docs/adr/8044 §5) on the section building (fixtures/arch/v1/section-building.kcad): a hip
//! roof over its 1. kat, its eaves 50 cm out of the walls' outer faces (`roofs::eaves_from_walls`) at 33 %, its eaves at
//! the storey's top (+6.00). B-B runs north–south through the stair and the ridge: the two long faces cut down their fall
//! lines, hatched as wood (TS 5319 SN23) on MM-CATI-KIREMIT, "%33" over each; "+6.00 Saçak" and the ridge's top
//! "+7.69 Mahya" among the levels; the roof no longer listed as missing. A-A runs east–west along the south face: no
//! slope is written where the band lies level. The north facade sees the roof's fascia, its hips and its ridge.

use kentos_commands::{
    BuildingSectionLeft, BuildingSectionsEdit, BuildingSectionsOperation, CallOp, CommandResult,
    DocumentSnapshotV2, Entity, ViewMarkForm,
};
use kentos_domain::{Document, Uuid};
use kentos_native_application::{ExecutionContext, registry, roofs, sections, sections_edit};
use serde_json::json;

fn building() -> Document {
    let snap: DocumentSnapshotV2 = kentos_kcad::decode(include_bytes!(
        "../../../../../fixtures/arch/v1/section-building.kcad"
    ))
    .unwrap();
    Document::from_snapshot_v2(snap).unwrap()
}

/// The building with a hip roof over its 1. kat: its walls' outer outline 50 cm out, 33 % on every edge.
fn roofed(doc: &Document) -> Document {
    let mut doc = doc.clone();
    let eaves = roofs::eaves_from_walls(
        &doc,
        kentos_commands::Vec2 { x: 6.0, y: 4.5 },
        Some("k1"),
        0.5,
    )
    .expect("the walls enclose the point");
    let slopes = vec![33.0; eaves.len()];
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.roofs.create",
        CallOp::Execute,
        json!({"layerId": "0", "roofs": [{"pts": eaves, "slopes": slopes, "overhang": 0.5,
                                          "arch": {"level": "k1"}}]}),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    doc
}

fn mark(doc: &Document, form: ViewMarkForm, letter: &str) -> Uuid {
    doc.entities_with_uids()
        .find_map(|(u, e)| match e {
            Entity::ViewMark(v) if v.mark.form == form && v.mark.mark == letter => Some(u),
            _ => None,
        })
        .unwrap()
}

fn derive(doc: &mut Document, m: Uuid) -> kentos_commands::BuildingSectionsEdited {
    match sections_edit::execute(
        &mut ExecutionContext::new(doc),
        BuildingSectionsEdit {
            operation: BuildingSectionsOperation::Derive {
                mark: m.to_string(),
                at: Some(kentos_commands::Vec2 { x: 20.0, y: 0.0 }),
            },
            expected_revision: None,
        },
    ) {
        CommandResult::Completed { output, .. } => output,
        other => panic!("{other:?}"),
    }
}

fn texts(doc: &Document, m: Uuid) -> Vec<String> {
    doc.block(sections::block_id(m))
        .unwrap()
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn b_b_cuts_the_roof_down_its_fall_lines_with_its_slope_and_levels() {
    let mut doc = roofed(&building());
    let b = mark(&doc, ViewMarkForm::Section, "B");
    let out = derive(&mut doc, b);
    assert!(
        !out.left_out
            .iter()
            .any(|l| l.why == BuildingSectionLeft::Roof),
        "the roof is drawn now: {:?}",
        out.left_out
    );
    let block = doc.block(sections::block_id(b)).unwrap();
    // Its band hatched as wood on its covering's layer, outlined heavy.
    let hatch = block
        .entities
        .iter()
        .find_map(|e| match e {
            Entity::Hatch(h) if h.base.layer_id == "mm-cati-kiremit-t" => Some(h),
            _ => None,
        })
        .expect("the roof's hatch");
    assert!(
        format!("{:?}", hatch.pattern).contains("SN23"),
        "{:?}",
        hatch.pattern
    );
    assert!(
        block
            .entities
            .iter()
            .any(|e| matches!(e, Entity::Polyline(p) if p.base.layer_id == "mm-cati-kiremit")),
        "its outline"
    );
    // The two long faces cut down their fall lines: their slope over each; the eaves' storey top and the ridge's top
    // among the levels (13.25 × 10.25 m at 33 %: 1.69 m over +6.00).
    let t = texts(&doc, b);
    assert_eq!(t.iter().filter(|x| *x == "%33").count(), 2, "{t:?}");
    for w in ["+6.00 Saçak", "+7.69 Mahya", "±0.00", "+3.00"] {
        assert!(t.iter().any(|x| x == w), "{w} in {t:?}");
    }
    // The storey top is the saçak seviyesi once: no bare "+6.00" beside it.
    assert!(!t.iter().any(|x| x == "+6.00"), "{t:?}");
}

#[test]
fn a_a_along_the_south_face_writes_no_slope_where_the_band_lies_level() {
    let mut doc = roofed(&building());
    let a = mark(&doc, ViewMarkForm::Section, "A");
    derive(&mut doc, a);
    let t = texts(&doc, a);
    // A-A at y 2.4 cuts the south face level (no slope written) and the east and west hip faces down their fall
    // lines, each over a metre: their slopes are written.
    assert_eq!(t.iter().filter(|x| *x == "%33").count(), 2, "{t:?}");
    assert!(t.iter().any(|x| x == "+7.69 Mahya"), "{t:?}");
}

#[test]
fn without_a_roof_the_section_still_says_it_is_missing() {
    let mut doc = building();
    let b = mark(&doc, ViewMarkForm::Section, "B");
    let out = derive(&mut doc, b);
    assert!(
        out.left_out
            .iter()
            .any(|l| l.why == BuildingSectionLeft::Roof)
    );
    let t = texts(&doc, b);
    assert!(t.iter().any(|x| x == "+6.00"), "{t:?}");
    assert!(!t.iter().any(|x| x.contains("Mahya")), "{t:?}");
}

/// The facade building (fixtures/arch/v1/facade-building.kcad): the section building with four elevation marks.
fn facades() -> Document {
    let snap: DocumentSnapshotV2 = kentos_kcad::decode(include_bytes!(
        "../../../../../fixtures/arch/v1/facade-building.kcad"
    ))
    .unwrap();
    Document::from_snapshot_v2(snap).unwrap()
}

#[test]
fn the_north_facade_sees_the_roofs_fascia_its_hips_and_its_ridge() {
    let mut doc = roofed(&facades());
    let north = mark(&doc, ViewMarkForm::Elevation, "1");
    derive(&mut doc, north);
    let block = doc.block(sections::block_id(north)).unwrap();
    let lines: Vec<&kentos_commands::LineEntity> = block
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Line(l) if l.base.layer_id.starts_with("mm-cati-kiremit-") => Some(l),
            _ => None,
        })
        .collect();
    let level = |l: &&kentos_commands::LineEntity, z: f64| {
        (l.a.y - z).abs() < 1e-6 && (l.b.y - z).abs() < 1e-6
    };
    let long = |l: &&kentos_commands::LineEntity| (l.a.x - l.b.x).abs();
    // The ridge: 3 m level at its top, 1.69 m over the eaves (13.25 × 10.25 m at 33 %).
    let ridge = 6.0 + 0.33 * 10.25 / 2.0;
    assert!(
        lines
            .iter()
            .any(|l| level(l, ridge) && (long(l) - 3.0).abs() < 1e-6),
        "the ridge in {lines:?}"
    );
    // The fascia across the roof's width, its top at the eaves (+6.00) and its bottom 25 cm under it.
    for z in [6.0, 5.75] {
        assert!(
            lines
                .iter()
                .any(|l| level(l, z) && (long(l) - 13.25).abs() < 1e-6),
            "the fascia at {z} in {lines:?}"
        );
    }
    // Its two hips, slanting from the eaves' corners to the ridge's ends.
    let slanted = lines
        .iter()
        .filter(|l| (l.a.y - l.b.y).abs() > 1.0 && long(l) > 1.0)
        .count();
    assert_eq!(slanted, 2, "{lines:?}");
    // Its covering named among the facade's finishes (Mimarlar Odası 14.3.4 c: "çatı örtüleri kaplamaları belirtilir").
    let t = texts(&doc, north);
    for w in ["+6.00 Saçak", "+7.69 Mahya", "Kiremit"] {
        assert!(t.iter().any(|x| x == w), "{w} in {t:?}");
    }
}
