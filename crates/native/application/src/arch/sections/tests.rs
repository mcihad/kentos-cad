//! Tests of `sections.rs`, in a file of their own (the file is over the size limit and may not grow).

use super::*;

#[test]
fn titles_and_levels_are_written_as_turkiye_writes_them() {
    assert_eq!(title("a"), "A-A KESİTİ");
    assert_eq!(title("i"), "İ-İ KESİTİ");
    assert_eq!(title(""), "KESİT");
    assert_eq!(label("B"), "B-B");
    assert_eq!(level_words(0.0), "±0.00");
    assert_eq!(level_words(3.0), "+3.00");
    assert_eq!(level_words(-2.75), "-2.75");
    assert_eq!(block_name("A"), "MM-XXXX-KESITA");
}

#[test]
fn a_marks_block_is_the_same_in_every_file() {
    let u = Uuid::parse_str("0192f5a1-7242-7000-8000-000000000001").unwrap();
    assert_eq!(block_id(u), block_id(u));
    assert_ne!(
        block_id(u),
        block_id(Uuid::parse_str("0192f5a1-7242-7000-8000-000000000002").unwrap())
    );
}

#[test]
fn the_ground_stops_at_what_is_cut() {
    let cut = sec::Cut {
        solid: 0,
        ring: vec![
            P::new(1.0, -0.5),
            P::new(3.0, -0.5),
            P::new(3.0, 0.2),
            P::new(1.0, 0.2),
        ],
    };
    assert_eq!(
        uncut(&[cut], (-2.0, 6.0), 0.0),
        vec![(-2.0, 1.0), (3.0, 6.0)]
    );
}

#[test]
fn seen_pieces_on_one_line_are_joined() {
    let joined = merged(vec![
        (P::new(0.0, 1.0), P::new(2.0, 1.0)),
        (P::new(3.0, 1.0), P::new(1.5, 1.0)),
        (P::new(5.0, 1.0), P::new(6.0, 1.0)),
    ]);
    assert_eq!(joined.len(), 2);
}
