//! Unifont and Big Font files built here from the published shape language
//! (ADR 1120); no third-party font is read. Each builder mirrors the layout
//! scripts/fixtures/shx_font_reference.py writes and checks independently.
use super::*;

const A: &[u8] = &[0x44, 0x20, 2, 0x4C, 0x10, 0]; // up 4, right 2, pen up, down 4, advance 1
const B: &[u8] = &[0x24, 2, 0x2C, 0x20, 0]; // up 2, pen up, back down, advance 2

pub(super) fn unifont(encoding: u8, embed: u8, records: &[(u16, &[u8])]) -> Vec<u8> {
    let mut b = UNIFONT.to_vec();
    b.extend((records.len() as u32 + 1).to_le_bytes());
    let def = [&b"KENTOS TEST\0"[..], &[4, 1, 0, encoding, embed, 0]].concat();
    b.extend((def.len() as u16).to_le_bytes());
    b.extend(&def);
    for (n, code) in records {
        let record = [&[0u8][..], code].concat();
        b.extend(n.to_le_bytes());
        b.extend((record.len() as u16).to_le_bytes());
        b.extend(record);
    }
    b
}

/// One empty index slot is written first, as compiled Big Fonts keep them.
pub(super) fn bigfont(descriptor: &[u8], records: &[(u16, &[u8])]) -> Vec<u8> {
    let ranges: &[(u16, u16)] = &[(0x81, 0x9f), (0xe0, 0xfc)];
    let mut all = vec![(0u16, [&b"KENTOS BIG\0"[..], descriptor].concat())];
    all.extend(records.iter().map(|(n, c)| (*n, [&[0u8][..], c].concat())));
    let mut b = BIGFONT.to_vec();
    b.extend((4 + 4 * ranges.len() as u16).to_le_bytes());
    b.extend((all.len() as u16 + 1).to_le_bytes());
    b.extend((ranges.len() as u16).to_le_bytes());
    for (s, e) in ranges {
        b.extend(s.to_le_bytes());
        b.extend(e.to_le_bytes());
    }
    let mut offset = b.len() + 8 * (all.len() + 1);
    b.extend([0u8; 8]);
    for (n, r) in &all {
        b.extend(n.to_le_bytes());
        b.extend((r.len() as u16).to_le_bytes());
        b.extend((offset as u32).to_le_bytes());
        offset += r.len();
    }
    for (_, r) in all {
        b.extend(r);
    }
    b
}

fn p(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, y)
}

#[test]
fn unifont_text_runs_in_capital_heights_and_carries_the_advance() {
    let data = unifont(0, 0, &[(0x41, A), (0x42, B)]);
    let f = ShapeFile::read(&data).unwrap();
    assert_eq!(f.kind(), Kind::Unifont);
    assert_eq!(f.font().unwrap().above, 4);
    let run: Vec<_> = "AB".chars().map(|c| (&f, f.glyph(c).unwrap())).collect();
    assert_eq!(
        draw_text(&run).unwrap(),
        vec![
            vec![p(0., 0.), p(0., 1.), p(0.5, 1.)],
            vec![p(0.75, 0.), p(0.75, 0.5)],
        ]
    );
    assert!(f.glyph('C').unwrap_err().contains("yok"));
    assert!(f.glyph('😀').unwrap_err().contains("aralığının dışında"));
}

#[test]
fn unifont_subshape_operand_is_two_bytes_high_first() {
    // U+0100 calls U+0041 (bytes 00 41); U+0102 calls U+0100 (bytes 01 00):
    // a one-byte or low-first reading would call 0 or 1, which do not exist.
    let data = unifont(
        0,
        0,
        &[(0x41, A), (0x100, &[7, 0x00, 0x41, 0]), (0x102, &[7, 0x01, 0x00, 0])],
    );
    let f = ShapeFile::read(&data).unwrap();
    let direct = draw_text(&[(&f, f.glyph('A').unwrap())]).unwrap();
    assert_eq!(draw_text(&[(&f, f.glyph('Ă').unwrap())]).unwrap(), direct);
    // Unscaled, as a linetype shape: vector units.
    assert_eq!(f.draw_number(0x102).unwrap()[0][1], p(0., 4.));
}

#[test]
fn a_unifont_that_is_not_unicode_or_has_a_bad_descriptor_is_refused() {
    let shapes = unifont(2, 0, &[(0x41, A)]);
    let f = ShapeFile::read(&shapes).unwrap();
    assert!(f.glyph('A').unwrap_err().contains("Unicode değil"));
    assert!(f.draw_number(0x41).is_ok(), "a shape-file unifont still serves shapes");
    let mut bad = unifont(0, 0, &[(0x41, A)]);
    let modes = UNIFONT.len() + 4 + 2 + b"KENTOS TEST\0".len() + 2;
    bad[modes] = 1;
    assert!(ShapeFile::read(&bad).is_err());
    bad[modes] = 0;
    bad[modes - 2] = 0; // above 0
    assert!(ShapeFile::read(&bad).is_err());
}

#[test]
fn every_unifont_and_bigfont_truncation_and_trailer_is_rejected() {
    let u = unifont(0, 0, &[(0x41, A), (0x42, B)]);
    let g = bigfont(&[8, 0, 0, 0], &[(0x8140, A)]);
    for data in [&u, &g] {
        ShapeFile::read(data).unwrap();
        for end in 0..data.len() {
            assert!(ShapeFile::read(&data[..end]).is_err(), "{end}");
        }
    }
    let mut extra = u.clone();
    extra.push(0);
    assert!(ShapeFile::read(&extra).is_err());
    assert!(ShapeFile::read(&unifont(0, 0, &[(0x41, A), (0x41, B)])).is_err());
    assert!(ShapeFile::read(&unifont(0, 0, &[(0, A)])).is_err());
}

#[test]
fn bigfont_codes_need_an_escape_lead_byte_and_mix_with_the_primary_font() {
    let g = bigfont(&[8, 2, 0, 0], &[(0x8140, A), (0x4141, A)]);
    let big = ShapeFile::read(&g).unwrap();
    assert_eq!(big.kind(), Kind::BigFont);
    assert_eq!(big.big_glyph(0x8140), Ok(0x8140));
    assert!(big.big_glyph(0x4141).unwrap_err().contains("kaçış"));
    assert!(big.big_glyph(0x8141).unwrap_err().contains("yok"));
    assert!(big.glyph('A').is_err());
    let u = unifont(0, 0, &[(0x41, A)]);
    let primary = ShapeFile::read(&u).unwrap();
    // The Big Font's own capital height (8) scales its glyph: half the size in
    // vector units, the same in capital heights.
    let out = draw_text(&[(&primary, 0x41), (&big, 0x8140)]).unwrap();
    assert_eq!(out[1], vec![p(0.75, 0.), p(0.75, 0.5), p(1., 0.5)]);
}

#[test]
fn extended_bigfont_primitives_are_refused_unless_skipped_as_vertical() {
    let ext = [16, 0, 2, 16, 0];
    let g = bigfont(&ext, &[(0x8140, &[7, 0, 0x81, 0x41, 0, 0, 16, 16, 0])]);
    let big = ShapeFile::read(&g).unwrap();
    assert!(big.font().unwrap().extended);
    assert!(
        draw_text(&[(&big, 0x8140)])
            .unwrap_err()
            .contains("Genişletilmiş")
    );
    let g = bigfont(&ext, &[(0x8140, &[14, 7, 0, 0x81, 0x41, 0, 0, 16, 16, 0x10, 0])]);
    let big = ShapeFile::read(&g).unwrap();
    assert_eq!(
        draw_text(&[(&big, 0x8140)]).unwrap(),
        vec![vec![p(0., 0.), p(1. / 16., 0.)]]
    );
}

#[test]
fn bigfont_index_offsets_stay_inside_the_records() {
    let mut g = bigfont(&[8, 0, 0, 0], &[(0x8140, A)]);
    // The first real index entry (after the empty slot) points into the header.
    let entry = BIGFONT.len() + 6 + 8 + 8;
    g[entry + 4..entry + 8].copy_from_slice(&4u32.to_le_bytes());
    assert!(ShapeFile::read(&g).is_err());
}

#[test]
fn a_classic_text_font_reads_ascii_only() {
    let mut b = SHAPES_10.to_vec();
    for n in [0u16, 0x41, 2] {
        b.extend(n.to_le_bytes());
    }
    let records: [(u16, Vec<u8>); 2] = [
        (0, [&b"FONT\0"[..], &[4, 1, 0, 0]].concat()),
        (0x41, [&[0u8][..], A].concat()),
    ];
    for (n, r) in &records {
        b.extend(n.to_le_bytes());
        b.extend((r.len() as u16).to_le_bytes());
    }
    for (_, r) in &records {
        b.extend(r);
    }
    b.extend(b"EOF");
    let f = ShapeFile::read(&b).unwrap();
    assert_eq!(f.kind(), Kind::Shapes);
    assert_eq!(f.glyph('A'), Ok(0x41));
    assert!(f.glyph('Ş').unwrap_err().contains("kod sayfası"));
}
