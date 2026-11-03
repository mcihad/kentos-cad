//! External C HarfBuzz and FriBidi are the oracle; no KentOS-generated expected glyphs.
use kentos_geometry_core::text::shaping::{self, BaseDirection, Error, FontId, Style};
use serde_json::Value;

fn direction(s: &str) -> BaseDirection {
    match s {
        "ltr" => BaseDirection::LeftToRight,
        "rtl" => BaseDirection::RightToLeft,
        _ => BaseDirection::Auto,
    }
}

#[test]
fn external_harfbuzz_positions_and_source_clusters_match_all_twenty_nine_bundled_faces() {
    let reference: Value =
        serde_json::from_str(include_str!("../../../../../fixtures/text/v1/shaping.json")).unwrap();
    for c in reference["cases"].as_array().unwrap() {
        let text = c["text"].as_str().unwrap();
        let line = shaping::line(
            text,
            Style {
                family: c["family"].as_str().unwrap(),
                weight: c["weight"].as_u64().unwrap() as u16,
                italic: c["italic"].as_bool().unwrap(),
                direction: direction(c["direction"].as_str().unwrap()),
                language: Some("und"),
                ..Style::default()
            },
        )
        .unwrap();
        let expected = c["glyphs"].as_array().unwrap();
        assert_eq!(line.glyphs.len(), expected.len(), "{}", c["name"]);
        let upm = c["unitsPerEm"].as_f64().unwrap();
        let mut x = 0.;
        for (actual, want) in line.glyphs.iter().zip(expected) {
            assert_eq!(
                actual.font, line.requested,
                "unexpected fallback for {}",
                c["name"]
            );
            assert_eq!(
                u64::from(actual.id),
                want["g"].as_u64().unwrap(),
                "{} glyph",
                c["name"]
            );
            assert_eq!(
                actual.bytes.start,
                want["cl"].as_u64().unwrap() as usize,
                "{} cluster",
                c["name"]
            );
            assert_eq!(
                actual.scalars.start,
                text[..actual.bytes.start].chars().count() as u32
            );
            assert_eq!(
                actual.scalars.end,
                text[..actual.bytes.end].chars().count() as u32
            );
            assert!(
                (actual.origin[0] - (x + want["dx"].as_f64().unwrap()) / upm).abs() < 1e-10,
                "{} x",
                c["name"]
            );
            assert!(
                (actual.origin[1] - want["dy"].as_f64().unwrap() / upm).abs() < 1e-10,
                "{} y",
                c["name"]
            );
            assert!((actual.advance - want["ax"].as_f64().unwrap() / upm).abs() < 1e-10);
            x += want["ax"].as_f64().unwrap();
        }
        assert!((line.advance - x / upm).abs() < 1e-10);
        assert!(line.ink.is_some());
    }
}

#[test]
fn external_fribidi_visual_order_keeps_the_original_logical_scalar_ranges() {
    let reference: Value =
        serde_json::from_str(include_str!("../../../../../fixtures/text/v1/shaping.json")).unwrap();
    for c in reference["bidi"].as_array().unwrap() {
        let text = c["text"].as_str().unwrap();
        let line = shaping::line(
            text,
            Style {
                family: "arimo",
                direction: direction(c["direction"].as_str().unwrap()),
                ..Style::default()
            },
        )
        .unwrap();
        let mut actual = Vec::new();
        let mut previous = None;
        for g in &line.glyphs {
            if previous.as_ref() == Some(&g.scalars) {
                continue;
            }
            previous = Some(g.scalars.clone());
            let mut indices: Vec<_> = g.scalars.clone().collect();
            if g.bidi_level % 2 == 1 {
                indices.reverse();
            }
            actual.extend(indices);
        }
        let expected: Vec<u32> = c["visualToLogical"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_u64().unwrap() as u32)
            .collect();
        assert_eq!(actual, expected, "{} {}", text, c["direction"]);
        assert!(line.glyphs.iter().all(|g| g.id != 0));
    }
}

#[test]
fn unicode_shaping_preserves_combined_source_without_normalizing_or_question_marks() {
    let input = "Ög\u{306}ç ⌀32 ≤ 5‰ العربية עברית";
    let original = input.as_bytes().to_vec();
    let line = shaping::line(
        input,
        Style {
            family: "arimo",
            ..Style::default()
        },
    )
    .unwrap();
    assert!(
        line.glyphs
            .iter()
            .any(|g| &input[g.bytes.clone()] == "g\u{306}")
    );
    assert!(
        line.glyphs
            .iter()
            .any(|g| shaping::font_key(g.font).unwrap().0 == "noto-sans-math")
    );
    assert!(
        line.glyphs
            .iter()
            .any(|g| shaping::font_key(g.font).unwrap().0 == "noto-sans-arabic")
    );
    assert_eq!(input.as_bytes(), original);
    assert!(line.advance.is_finite() && line.advance > 0.);
    for g in &line.glyphs {
        assert!(g.origin.iter().all(|v| v.is_finite()));
    }
}

#[test]
fn unavailable_sources_and_controlled_limits_fail_explicitly() {
    assert_eq!(
        shaping::line(
            "test",
            Style {
                family: "Arial Narrow",
                ..Style::default()
            }
        ),
        Err(Error::UnknownFamily)
    );
    assert!(matches!(
        shaping::line("👩🏽‍💻", Style::default()),
        Err(Error::Unsupported { .. })
    ));
    assert_eq!(
        shaping::line("a\nb", Style::default()),
        Err(Error::LineBreak)
    );
    assert_eq!(
        shaping::line(&"a".repeat(shaping::MAX_BYTES + 1), Style::default()),
        Err(Error::TooLong)
    );
    assert!(shaping::font_bytes(FontId(u16::MAX)).is_none());
    let spaces = shaping::line("  ", Style::default()).unwrap();
    assert!(spaces.ink.is_none() && spaces.advance > 0.);
    let empty = shaping::line("", Style::default()).unwrap();
    assert!(empty.ink.is_none() && empty.glyphs.is_empty() && empty.advance == 0.);
}

#[test]
fn wrapped_lines_use_external_fribidi_paragraph_levels_not_substring_direction() {
    let reference: Value =
        serde_json::from_str(include_str!("../../../../../fixtures/text/v1/shaping.json")).unwrap();
    for c in reference["wrapped"].as_array().unwrap() {
        let text = c["text"].as_str().unwrap();
        let offsets: Vec<_> = text
            .char_indices()
            .map(|(at, _)| at)
            .chain(std::iter::once(text.len()))
            .collect();
        let from = c["start"].as_u64().unwrap() as usize;
        let to = c["end"].as_u64().unwrap() as usize;
        let style = Style {
            family: "arimo",
            direction: direction(c["direction"].as_str().unwrap()),
            ..Style::default()
        };
        let paragraph = shaping::Paragraph::new(text, style.direction).unwrap();
        let line = paragraph
            .line(offsets[from]..offsets[to], style, &[])
            .unwrap();
        let mut actual = Vec::new();
        let mut previous = None;
        for glyph in &line.glyphs {
            if previous.as_ref() == Some(&glyph.scalars) {
                continue;
            }
            previous = Some(glyph.scalars.clone());
            let mut scalars: Vec<_> = glyph.scalars.clone().collect();
            if glyph.bidi_level % 2 == 1 {
                scalars.reverse();
            }
            actual.extend(scalars);
        }
        let expected: Vec<_> = c["visualToLogical"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_u64().unwrap() as u32)
            .collect();
        assert_eq!(
            actual,
            expected,
            "{} {:?} {}",
            text,
            from..to,
            c["direction"]
        );
        assert!(
            line.glyphs
                .iter()
                .all(|g| g.scalars.start >= from as u32 && g.scalars.end <= to as u32)
        );
    }
}

#[test]
fn format_boundaries_keep_arabic_joining_context_and_match_external_harfbuzz() {
    let reference: Value =
        serde_json::from_str(include_str!("../../../../../fixtures/text/v1/shaping.json")).unwrap();
    for c in reference["styled"].as_array().unwrap() {
        let text = c["text"].as_str().unwrap();
        let offsets: Vec<_> = text
            .char_indices()
            .map(|(at, _)| at)
            .chain(std::iter::once(text.len()))
            .collect();
        let style = Style {
            family: c["family"].as_str().unwrap(),
            direction: direction(c["direction"].as_str().unwrap()),
            language: Some("und"),
            ..Style::default()
        };
        let spans = c["spans"].as_array().unwrap();
        let formats: Vec<_> = spans
            .iter()
            .map(|s| shaping::Format {
                bytes: offsets[s["start"].as_u64().unwrap() as usize]
                    ..offsets[s["end"].as_u64().unwrap() as usize],
                style: Style {
                    weight: s["weight"].as_u64().unwrap() as u16,
                    ..style
                },
            })
            .collect();
        let actual = shaping::Paragraph::new(text, style.direction)
            .unwrap()
            .line(0..text.len(), style, &formats)
            .unwrap();
        let ordered: Vec<_> = if style.direction == BaseDirection::RightToLeft {
            spans.iter().rev().collect()
        } else {
            spans.iter().collect()
        };
        let mut cursor = 0.;
        let mut index = 0;
        for s in ordered {
            let upm = s["unitsPerEm"].as_f64().unwrap();
            let byte_start = offsets[s["start"].as_u64().unwrap() as usize];
            for expected in s["glyphs"].as_array().unwrap() {
                let glyph = &actual.glyphs[index];
                index += 1;
                assert_eq!(
                    u64::from(glyph.id),
                    expected["g"].as_u64().unwrap(),
                    "{}",
                    text
                );
                assert_eq!(
                    glyph.bytes.start,
                    byte_start + expected["cl"].as_u64().unwrap() as usize
                );
                assert_eq!(
                    shaping::font_key(glyph.font).unwrap().1,
                    s["weight"].as_u64().unwrap() as u16
                );
                assert!(
                    (glyph.origin[0] - cursor - expected["dx"].as_f64().unwrap() / upm).abs()
                        < 1e-10
                );
                assert!((glyph.origin[1] - expected["dy"].as_f64().unwrap() / upm).abs() < 1e-10);
                cursor += expected["ax"].as_f64().unwrap() / upm;
            }
        }
        assert_eq!(index, actual.glyphs.len());
        assert!((actual.advance - cursor).abs() < 1e-10);
    }
}

#[test]
fn formatted_lines_refuse_overlaps_and_splitting_combined_characters() {
    let text = "Ög\u{306}ç";
    let paragraph = shaping::Paragraph::new(text, BaseDirection::Auto).unwrap();
    let style = Style {
        family: "arimo",
        ..Style::default()
    };
    assert_eq!(paragraph.line(0..3, style, &[]), Err(Error::InvalidRange));
    assert_eq!(
        paragraph.line(
            0..text.len(),
            style,
            &[shaping::Format { bytes: 2..3, style }]
        ),
        Err(Error::InvalidFormats)
    );
    assert_eq!(
        paragraph.line(
            0..text.len(),
            style,
            &[
                shaping::Format { bytes: 0..5, style },
                shaping::Format {
                    bytes: 2..text.len(),
                    style
                }
            ]
        ),
        Err(Error::InvalidFormats)
    );
    // Redundant formatting does not break kerning or ligatures.
    let text = "AVATAR office";
    let full = shaping::line(text, style).unwrap();
    let formatted = shaping::Paragraph::new(text, BaseDirection::Auto)
        .unwrap()
        .line(
            0..text.len(),
            style,
            &[
                shaping::Format { bytes: 0..2, style },
                shaping::Format {
                    bytes: 2..text.len(),
                    style,
                },
            ],
        )
        .unwrap();
    assert_eq!(full, formatted);
}
