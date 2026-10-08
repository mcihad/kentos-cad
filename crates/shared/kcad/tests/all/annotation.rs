//! Annotation heights, a dimension's lines and a leader's AutoCAD arrowheads
//! in the project file (docs/adr/0205 §8, docs/specs/kcad-v2.md §6.1, §6.4.6,
//! §6.6): document schema 30 is written only for a drawing that has one of
//! them, in the drawing or a block definition; every other drawing keeps its
//! schema and its bytes; they come back as written; the writer refuses what
//! the reader would.

use kentos_kcad::Code;
use kentos_kcad::contracts::{
    AnnotationHeights, DocumentSnapshotV2, Entity, LeaderArrow, LineType,
};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn content(name: &str) -> DocumentSnapshotV2 {
    serde_json::from_slice(&read(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The document schema a file's payload says: the unsigned integer after the
/// `version` key (from 24 on, a byte 0x18 and the value).
fn schema(file: &[u8]) -> u8 {
    let payload = &file[36..file.len() - 32];
    let key = b"\x67version";
    let at = payload
        .windows(key.len())
        .position(|w| w == key)
        .expect("a version key")
        + key.len();
    match payload[at] {
        0x18 => payload[at + 1],
        small => small,
    }
}

fn written(doc: &DocumentSnapshotV2) -> u8 {
    schema(&kentos_kcad::encode(doc).expect("writes"))
}

/// The drawing without anything of schema 30's: no heights, no lines, the
/// leaders' arrowheads and sizes of schema 8.
fn plain(doc: &DocumentSnapshotV2) -> DocumentSnapshotV2 {
    let mut out = doc.clone();
    out.settings.annotation = None;
    for s in &mut out.settings.dimension_styles {
        *s = kentos_kcad::contracts::DimensionStyleDef {
            dim_line_color: None,
            dim_line_weight: None,
            dim_line_type: None,
            ext_color: None,
            ext_weight: None,
            ext_line_type: None,
            text_color: None,
            ..s.clone()
        };
    }
    let unline = |e: &mut Entity| match e {
        Entity::Dimension(d) => {
            d.look.dim_line_color = None;
            d.look.dim_line_weight = None;
            d.look.dim_line_type = None;
            d.look.ext_color = None;
            d.look.ext_weight = None;
            d.look.ext_line_type = None;
            d.look.text_color = None;
        }
        Entity::Leader(l) => {
            l.arrow_size = None;
            if l.arrow.is_some_and(|a| a.is_added()) {
                l.arrow = Some(LeaderArrow::Open);
            }
        }
        _ => {}
    };
    out.entities.iter_mut().for_each(unline);
    for b in &mut out.blocks {
        b.entities.iter_mut().for_each(unline);
    }
    out
}

#[test]
fn a_drawing_with_heights_lines_or_new_arrowheads_is_schema_30_and_one_without_stays_as_it_was() {
    let doc = content("annotation.json");
    let bytes = kentos_kcad::encode(&doc).expect("writes");
    assert_eq!(schema(&bytes), 30);
    assert!(bytes == read("annotation.kcad"), "the reference's bytes");
    let without = plain(&doc);
    assert!(written(&without) < 30, "{}", written(&without));
    // Each one alone makes it 30: the heights, a style's lines, a dimension's
    // lines, a leader's size, an added arrowhead in a block definition.
    let mut one = without.clone();
    one.settings.annotation = Some(
        AnnotationHeights::default()
            .with(kentos_kcad::contracts::AnnotationKind::Station, Some(1.5)),
    );
    assert_eq!(written(&one), 30);
    let mut one = without.clone();
    one.settings.dimension_styles[0].ext_line_type = Some(LineType::Dotted);
    assert_eq!(written(&one), 30);
    let mut one = without.clone();
    if let Entity::Dimension(d) = &mut one.entities[3] {
        d.look.text_color = Some("#1F4E79".into());
    }
    assert_eq!(written(&one), 30);
    let mut one = without.clone();
    if let Entity::Leader(l) = &mut one.entities[1] {
        l.arrow_size = Some(1.5);
    }
    assert_eq!(written(&one), 30);
    let mut one = without.clone();
    if let Entity::Leader(l) = &mut one.blocks[0].entities[1] {
        l.arrow = Some(LeaderArrow::DotSmall);
    }
    assert_eq!(written(&one), 30);
}

#[test]
fn the_heights_lines_and_arrowheads_come_back_as_written() {
    let doc = content("annotation.json");
    let back = kentos_kcad::decode(&kentos_kcad::encode(&doc).expect("writes")).expect("reads");
    assert_eq!(back, doc);
    let h = back.settings.annotation.expect("heights");
    assert_eq!(h.measure, Some(1.5));
    let Entity::Dimension(d) = &back.entities[3] else {
        panic!("a dimension")
    };
    assert_eq!(d.look.dim_line_type, Some(LineType::Dashed));
    assert_eq!(d.look.ext_weight, Some(0.0));
    let Entity::Leader(l) = &back.entities[2] else {
        panic!("a leader")
    };
    assert_eq!(l.arrow, Some(LeaderArrow::DatumFilled));
    assert_eq!(l.arrow_size, Some(0.75));
}

#[test]
fn the_writer_refuses_what_the_reader_refuses() {
    let doc = content("annotation.json");
    let refused = |doc: DocumentSnapshotV2| kentos_kcad::encode(&doc).expect_err("refused");
    let mut bad = doc.clone();
    bad.settings.annotation = Some(AnnotationHeights {
        leader: Some(0.0),
        ..Default::default()
    });
    let e = refused(bad);
    assert_eq!(e.code, Code::BadValue);
    assert!(
        e.message.contains("settings/annotation/leader"),
        "the place: {e}"
    );
    let mut bad = doc.clone();
    bad.settings.annotation = Some(AnnotationHeights {
        dimension: Some(101.0),
        ..Default::default()
    });
    assert_eq!(refused(bad).code, Code::BadValue);
    let mut bad = doc.clone();
    if let Entity::Dimension(d) = &mut bad.entities[3] {
        d.look.dim_line_color = Some("red".into());
    }
    let e = refused(bad);
    assert_eq!(e.code, Code::BadValue);
    assert!(e.message.contains("dimLineColor"), "{e}");
    let mut bad = doc.clone();
    if let Entity::Dimension(d) = &mut bad.entities[3] {
        d.look.ext_weight = Some(-0.1);
    }
    assert_eq!(refused(bad).code, Code::BadValue);
    let mut bad = doc.clone();
    bad.settings.dimension_styles[0].text_color = Some("#F00".into());
    assert_eq!(refused(bad).code, Code::BadValue);
    let mut bad = doc.clone();
    if let Entity::Leader(l) = &mut bad.entities[0] {
        l.arrow_size = Some(10.5);
    }
    let e = refused(bad);
    assert_eq!(e.code, Code::BadValue);
    assert!(e.message.contains("ok boyu"), "{e}");
}
