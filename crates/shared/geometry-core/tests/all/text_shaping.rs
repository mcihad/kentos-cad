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

#[test]
fn bundled_contours_match_independent_fonttools_with_offsets_and_synthetic_slant() {
    use shaping::{Glyph, OutlineOp};
    let reference: Value =
        serde_json::from_str(include_str!("../../../../../fixtures/text/v1/shaping.json")).unwrap();
    for c in reference["outlines"].as_array().unwrap() {
        let glyph = Glyph {
            font: FontId(c["font"].as_u64().unwrap() as u16),
            id: c["id"].as_u64().unwrap() as u16,
            bytes: 0..0,
            scalars: 0..0,
            bidi_level: 0,
            origin: [2.25, -0.5],
            pen: 0.,
            size: 1.,
            advance: 0.,
            slant: 0.375,
        };
        let ops = shaping::outline(&glyph).unwrap();
        let expected = c["ops"].as_array().unwrap();
        assert_eq!(ops.len(), expected.len(), "{} {}", c["font"], c["source"]);
        let upm = c["unitsPerEm"].as_f64().unwrap();
        for (actual, expected) in ops.iter().zip(expected) {
            let (tag, points): (_, Vec<_>) = match actual {
                OutlineOp::Move(p) => ("move", vec![*p]),
                OutlineOp::Line(p) => ("line", vec![*p]),
                OutlineOp::Quad(a, p) => ("quad", vec![*a, *p]),
                OutlineOp::Cubic(a, b, p) => ("cubic", vec![*a, *b, *p]),
                OutlineOp::Close => ("close", vec![]),
            };
            assert_eq!(tag, expected[0].as_str().unwrap(), "{}", c["source"]);
            for (p, want) in points
                .iter()
                .zip(expected.as_array().unwrap().iter().skip(1))
            {
                let (x, y) = (
                    want[0].as_f64().unwrap() / upm,
                    want[1].as_f64().unwrap() / upm,
                );
                assert!(
                    (p[0] - (2.25 + x + y * 0.375)).abs() < 1e-6
                        && (p[1] - (-0.5 + y)).abs() < 1e-6,
                    "{} {:?} {:?}",
                    c["source"],
                    p,
                    want
                );
            }
        }
    }
    let space = shaping::line(" ", Style::default()).unwrap();
    assert!(shaping::outline(&space.glyphs[0]).unwrap().is_empty());
    let mut bad = space.glyphs[0].clone();
    bad.id = u16::MAX;
    assert_eq!(
        shaping::outline(&bad),
        Err(Error::BadGlyph {
            font: bad.font,
            id: u16::MAX
        })
    );
    bad.font = FontId(u16::MAX);
    assert_eq!(shaping::outline(&bad), Err(Error::BadFont(bad.font)));
    let mut huge = shaping::line("A", Style::default())
        .unwrap()
        .glyphs
        .remove(0);
    huge.size = f64::MAX;
    huge.slant = f64::MAX;
    assert_eq!(shaping::outline(&huge), Err(Error::InvalidGeometry));
}

#[test]
fn physical_unicode_paragraphs_match_external_harfbuzz_greedy_word_flow() {
    use kentos_geometry_core::{
        api::json::{FromJson, Json},
        text::paragraph::Flow,
    };
    use shaping::paragraph::{self, Settings};
    let reference: Value =
        serde_json::from_str(include_str!("../../../../../fixtures/text/v1/shaping.json")).unwrap();
    for c in reference["flow"].as_array().unwrap() {
        let text = c["text"].as_str().unwrap();
        let source = text.as_bytes().to_vec();
        let flow = Flow::from_json(&Json::parse(&c["flow"].to_string()).unwrap()).unwrap();
        let style = Style {
            family: c["family"].as_str().unwrap(),
            language: Some("und"),
            direction: direction(c["direction"].as_str().unwrap()),
            ..Style::default()
        };
        let actual = paragraph::layout(
            text,
            style,
            &[],
            Settings {
                height: c["height"].as_f64().unwrap(),
                width_factor: c["widthFactor"].as_f64().unwrap(),
                box_width: c["boxWidth"].as_f64(),
                line_spacing: c["lineSpacing"].as_f64().unwrap(),
                along: c["along"].as_f64().unwrap(),
                flow: Some(&flow),
            },
        )
        .unwrap();
        assert!(
            (actual.width - c["width"].as_f64().unwrap()).abs() < 1e-9,
            "{}",
            text
        );
        assert!((actual.pitch - c["pitch"].as_f64().unwrap()).abs() < 1e-9);
        assert_eq!(actual.depth, c["depth"].as_u64().unwrap() as usize);
        let expected = c["pieces"].as_array().unwrap();
        assert_eq!(actual.pieces.len(), expected.len(), "{}", text);
        let offsets: Vec<_> = text
            .char_indices()
            .map(|(at, _)| at)
            .chain(std::iter::once(text.len()))
            .collect();
        for (piece, want) in actual.pieces.iter().zip(expected) {
            let (from, to) = (
                want["start"].as_u64().unwrap() as usize,
                want["end"].as_u64().unwrap() as usize,
            );
            assert_eq!(piece.scalars, from as u32..to as u32, "{}", text);
            assert_eq!(piece.bytes, offsets[from]..offsets[to], "{}", text);
            assert!((piece.at[0] - want["x"].as_f64().unwrap()).abs() < 1e-9);
            assert!((piece.at[1] - want["y"].as_f64().unwrap()).abs() < 1e-9);
            assert!((piece.line.advance - want["advance"].as_f64().unwrap()).abs() < 1e-9);
            let expected = want["glyphs"].as_array().unwrap();
            assert_eq!(piece.line.glyphs.len(), expected.len());
            let mut cursor = 0.;
            let upm = c["unitsPerEm"].as_f64().unwrap();
            for (glyph, want) in piece.line.glyphs.iter().zip(expected) {
                assert_eq!(u64::from(glyph.id), want["g"].as_u64().unwrap());
                assert_eq!(
                    glyph.bytes.start,
                    offsets[from] + want["cl"].as_u64().unwrap() as usize
                );
                assert!(
                    glyph.bytes.start >= piece.bytes.start && glyph.bytes.end <= piece.bytes.end
                );
                assert!(
                    glyph.scalars.start >= piece.scalars.start
                        && glyph.scalars.end <= piece.scalars.end
                );
                assert!(
                    (glyph.origin[0] - cursor - want["dx"].as_f64().unwrap() / upm).abs() < 1e-9
                );
                assert!((glyph.origin[1] - want["dy"].as_f64().unwrap() / upm).abs() < 1e-9);
                cursor += want["ax"].as_f64().unwrap() / upm;
            }
        }
        assert_eq!(text.as_bytes(), source);
        assert!(actual.ink.is_some());
    }
}

#[test]
fn unicode_flow_rejects_nonfinite_boxes_and_keeps_empty_paragraph_source_offsets() {
    use shaping::paragraph::{self, Settings};
    let s = Settings {
        height: 1.,
        width_factor: 1.,
        box_width: Some(10.),
        line_spacing: 1.,
        along: 0.,
        flow: None,
    };
    for s in [
        Settings {
            height: f64::NAN,
            ..s
        },
        Settings {
            width_factor: 0.,
            ..s
        },
        Settings {
            line_spacing: f64::INFINITY,
            ..s
        },
        Settings {
            box_width: Some(0.),
            ..s
        },
        Settings { along: 2., ..s },
        Settings {
            height: 1e-300,
            line_spacing: 1e-300,
            ..s
        },
    ] {
        assert_eq!(
            paragraph::layout("A", Style::default(), &[], s),
            Err(Error::InvalidGeometry)
        );
    }
    let p = paragraph::layout("\r\n\r\n", Style::default(), &[], s).unwrap();
    assert_eq!(p.depth, 3);
    assert_eq!(
        p.pieces
            .iter()
            .map(|p| p.scalars.clone())
            .collect::<Vec<_>>(),
        vec![0..0, 2..2, 4..4]
    );
    assert!(p.ink.is_none());
    let text = "Ög\u{306}ç\nAVATAR office";
    let format = shaping::Format {
        bytes: 8..text.len(),
        style: Style {
            family: "arimo",
            weight: 600,
            ..Style::default()
        },
    };
    let p = paragraph::layout(
        text,
        Style {
            family: "arimo",
            ..Style::default()
        },
        &[format],
        s,
    )
    .unwrap();
    assert!(
        p.pieces[1]
            .line
            .glyphs
            .iter()
            .all(|g| shaping::font_key(g.font).unwrap().1 == 600)
    );
    assert_eq!(p.pieces[1].bytes.start, 8);
    assert_eq!(p.pieces[1].scalars.start, 5);
}

#[test]
fn unicode_wrapping_keeps_marks_attached_to_spaces_and_handles_dense_word_rows() {
    use shaping::paragraph::{self, Settings};
    use unicode_segmentation::UnicodeSegmentation;
    let s = Settings {
        height: 0.2,
        width_factor: 0.85,
        box_width: Some(0.3),
        line_spacing: 1.,
        along: 0.,
        flow: None,
    };
    let text = "A \u{306}B C \u{306}D";
    let p = paragraph::layout(
        text,
        Style {
            family: "arimo",
            ..Style::default()
        },
        &[],
        s,
    )
    .unwrap();
    let bounds: Vec<_> = text
        .grapheme_indices(true)
        .map(|(at, _)| at)
        .chain(std::iter::once(text.len()))
        .collect();
    for piece in p.pieces {
        assert!(bounds.contains(&piece.bytes.start) && bounds.contains(&piece.bytes.end));
    }
    // An exhaustive linear word-end oracle verifies the accelerated search.
    // Actual glyph metrics are independently tested against C HarfBuzz above.
    let text = "AVATAR office Ög\u{306}ç 2500 mm العربية עברית 123";
    let word_ranges: Vec<_> = text
        .split_whitespace()
        .map(|word| {
            let start = word.as_ptr() as usize - text.as_ptr() as usize;
            start..start + word.len()
        })
        .collect();
    for family in [
        "arimo",
        "barlow",
        "quicksand",
        "architects-daughter",
        "plex-mono",
        "overpass",
        "courier-prime",
    ] {
        for width in [0.2, 0.5, 1., 2., 10.] {
            let style = Style {
                family,
                weight: 600,
                italic: true,
                ..Style::default()
            };
            let paragraph = shaping::Paragraph::new(text, style.direction).unwrap();
            let mut ranges = Vec::new();
            let mut word = 0;
            while word < word_ranges.len() {
                let start = word_ranges[word].start;
                let mut end = word;
                for (next, range) in word_ranges.iter().enumerate().skip(word + 1) {
                    let line = paragraph.line(start..range.end, style, &[]).unwrap();
                    if line.advance * s.height * s.width_factor > width {
                        break;
                    }
                    end = next;
                }
                ranges.push(start..word_ranges[end].end);
                word = end + 1;
            }
            let actual = paragraph::layout(
                text,
                style,
                &[],
                Settings {
                    box_width: Some(width),
                    ..s
                },
            )
            .unwrap();
            assert_eq!(
                actual
                    .pieces
                    .iter()
                    .map(|p| p.bytes.clone())
                    .collect::<Vec<_>>(),
                ranges,
                "{family} {width}"
            );
        }
    }
    let text = "Ög\u{306}ç ".repeat(6000);
    let actual = paragraph::layout(
        &text,
        Style {
            family: "arimo",
            ..Style::default()
        },
        &[],
        s,
    )
    .unwrap();
    assert_eq!(actual.depth, 6000);
    assert_eq!(actual.pieces.len(), 6000);
    assert_eq!(actual.pieces.last().unwrap().bytes.end, text.len() - 1);
}

#[test]
fn atomic_fractions_follow_external_fribidi_object_order_without_rewriting_source() {
    use shaping::Atom;
    let reference: Value =
        serde_json::from_str(include_str!("../../../../../fixtures/text/v1/shaping.json")).unwrap();
    for c in reference["atoms"].as_array().unwrap() {
        let text = c["text"].as_str().unwrap();
        let original = text.as_bytes().to_vec();
        let offsets: Vec<_> = text
            .char_indices()
            .map(|(at, _)| at)
            .chain(std::iter::once(text.len()))
            .collect();
        let atoms: Vec<_> = c["ranges"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(tag, range)| Atom {
                tag,
                bytes: offsets[range[0].as_u64().unwrap() as usize]
                    ..offsets[range[1].as_u64().unwrap() as usize],
                advance: 2.5,
                ink: Some([-0.1, -0.25, 2., 0.75]),
            })
            .collect();
        let style = Style {
            family: "arimo",
            direction: direction(c["direction"].as_str().unwrap()),
            ..Style::default()
        };
        let paragraph = shaping::Paragraph::with_atoms(text, style.direction, &atoms).unwrap();
        let actual = paragraph.line(0..text.len(), style, &[]).unwrap();
        let mut items = Vec::new();
        let mut previous = None;
        for glyph in &actual.glyphs {
            if previous.as_ref() == Some(&glyph.scalars) {
                continue;
            }
            previous = Some(glyph.scalars.clone());
            let mut scalars: Vec<_> = glyph.scalars.clone().collect();
            if glyph.bidi_level % 2 == 1 {
                scalars.reverse();
            }
            items.push((
                glyph.origin[0],
                scalars
                    .into_iter()
                    .map(|s| serde_json::json!({"scalar":s}))
                    .collect::<Vec<_>>(),
            ));
        }
        for object in &actual.objects {
            assert_eq!(object.bytes, atoms[object.tag].bytes);
            assert_eq!(object.advance, 2.5);
            items.push((
                object.origin[0],
                vec![serde_json::json!({"atom":object.tag})],
            ));
        }
        items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let ordered: Vec<_> = items.into_iter().flat_map(|(_, v)| v).collect();
        assert_eq!(
            ordered,
            *c["visual"].as_array().unwrap(),
            "{} {}",
            text,
            c["direction"]
        );
        for expected in c["isolated"].as_array().unwrap() {
            let scalar = expected["scalar"].as_u64().unwrap() as u32;
            let ids: Vec<_> = actual
                .glyphs
                .iter()
                .filter(|g| g.scalars.start == scalar)
                .map(|g| u64::from(g.id))
                .collect();
            let want: Vec<_> = expected["glyphs"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| n.as_u64().unwrap())
                .collect();
            assert_eq!(ids, want, "joining must stop at the object: {text}");
        }
        assert_eq!(actual.objects.len(), atoms.len());
        assert!(
            (actual.advance
                - actual.glyphs.iter().map(|g| g.advance).sum::<f64>()
                - atoms.iter().map(|a| a.advance).sum::<f64>())
            .abs()
                < 1e-9
        );
        assert_eq!(text.as_bytes(), original);
    }
    // A graphic object's source is deliberately not sent to the font shaper.
    let text = "A 👩🏽‍💻 B";
    let atoms = [Atom {
        tag: 7,
        bytes: 2..text.len() - 2,
        advance: 3.,
        ink: None,
    }];
    let p = shaping::Paragraph::with_atoms(text, BaseDirection::Auto, &atoms).unwrap();
    let line = p.line(0..text.len(), Style::default(), &[]).unwrap();
    assert_eq!(line.objects[0].tag, 7);
    assert!(
        line.glyphs
            .iter()
            .all(|g| g.bytes.end <= 2 || g.bytes.start >= text.len() - 2)
    );
    assert_eq!(
        p.line(0..6, Style::default(), &[]),
        Err(Error::InvalidRange)
    );
    assert_eq!(
        p.line(
            0..text.len(),
            Style::default(),
            &[shaping::Format {
                bytes: 2..6,
                style: Style::default()
            }]
        ),
        Err(Error::InvalidFormats)
    );
    assert!(matches!(
        shaping::Paragraph::new("A\u{2029}B", BaseDirection::Auto),
        Err(Error::LineBreak)
    ));
}

#[test]
fn inline_fraction_sources_wrap_as_one_object_and_keep_global_offsets_across_paragraphs() {
    use shaping::{
        Atom,
        paragraph::{self, Settings},
    };
    let text = "Ö\nA 1 / 2 B";
    let from = text.find("1 / 2").unwrap();
    let atoms = [Atom {
        tag: 47,
        bytes: from..from + 5,
        advance: 2.,
        ink: Some([0., -0.25, 2., 0.9]),
    }];
    let style = Style {
        family: "arimo",
        ..Style::default()
    };
    let s = Settings {
        height: 1.,
        width_factor: 1.,
        box_width: Some(1.5),
        line_spacing: 1.,
        along: 0.,
        flow: None,
    };
    let p = paragraph::layout_with_atoms(text, style, &[], &atoms, s).unwrap();
    assert_eq!(p.depth, 4);
    assert_eq!(
        p.pieces
            .iter()
            .map(|p| &text[p.bytes.clone()])
            .collect::<Vec<_>>(),
        vec!["Ö", "A", "1 / 2", "B"]
    );
    let fraction = &p.pieces[2];
    assert!(fraction.line.glyphs.is_empty());
    assert_eq!(fraction.line.objects.len(), 1);
    assert_eq!(fraction.line.objects[0].tag, 47);
    assert_eq!(fraction.line.objects[0].bytes, from..from + 5);
    assert_eq!(fraction.line.objects[0].scalars, 4..9);
    assert_eq!(fraction.line.objects[0].origin, [0., 0.]);
    assert_eq!(fraction.at, [0., -2. * p.pitch]);
    assert_eq!(fraction.line.advance, 2.);
    assert_eq!(p.width, 1.5); // An unbreakable over-wide object overhangs its box.
    assert!(p.ink.unwrap()[2] >= 2.);
    let overlap = [atoms[0].clone(), atoms[0].clone()];
    assert_eq!(
        paragraph::layout_with_atoms(text, style, &[], &overlap, s),
        Err(Error::InvalidAtoms)
    );
    let bad = [Atom {
        advance: f64::INFINITY,
        ..atoms[0].clone()
    }];
    assert_eq!(
        paragraph::layout_with_atoms(text, style, &[], &bad, s),
        Err(Error::InvalidAtoms)
    );
    let big = [
        Atom {
            tag: 0,
            bytes: 0..1,
            advance: f64::MAX,
            ink: None,
        },
        Atom {
            tag: 1,
            bytes: 1..2,
            advance: f64::MAX,
            ink: None,
        },
    ];
    let p = shaping::Paragraph::with_atoms("AA", BaseDirection::Auto, &big).unwrap();
    assert_eq!(p.line(0..2, style, &[]), Err(Error::InvalidGeometry));
}

#[test]
fn unicode_v1_font_bytes_and_runtime_ids_are_frozen_independently_of_ui_asset_updates() {
    use sha2::{Digest, Sha256};
    // This manifest must not be regenerated to bless a font update. Keep v1
    // bytes and introduce a new profile; saved v1 text cannot silently reflow.
    let manifest: Value =
        serde_json::from_str(include_str!("../../data/text-fonts/manifest.json")).unwrap();
    assert_eq!(manifest["profile"], "unicode-v1");
    let faces = manifest["profileFaces"].as_array().unwrap();
    assert_eq!(faces.len(), shaping::font_count());
    for (index, face) in faces.iter().enumerate() {
        assert_eq!(face["index"].as_u64().unwrap(), index as u64);
        let id = FontId(index as u16);
        let (family, weight, italic) = shaping::font_key(id).unwrap();
        assert_eq!(face["family"], family);
        assert_eq!(face["weight"].as_u64().unwrap(), u64::from(weight));
        assert_eq!(face["italic"].as_bool().unwrap(), italic);
        let hash = format!("{:x}", Sha256::digest(shaping::font_bytes(id).unwrap()));
        assert_eq!(face["sha256"].as_str().unwrap(), hash, "{}", face["file"]);
    }
}

#[test]
fn superscript_and_subscript_use_exact_glyph_metrics_for_advance_ink_and_contours() {
    use shaping::{OutlineOp, Shift};
    let reference: Value =
        serde_json::from_str(include_str!("../../../../../fixtures/text/v1/shaping.json")).unwrap();
    let c = reference["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "Arimo-400.ttf")
        .unwrap();
    let text = c["text"].as_str().unwrap();
    let style = Style {
        family: "arimo",
        direction: BaseDirection::LeftToRight,
        language: Some("und"),
        ..Style::default()
    };
    let baseline = shaping::line(text, style).unwrap();
    for shift in [Shift::Super, Shift::Sub] {
        let actual = shaping::line(text, Style { shift, ..style }).unwrap();
        let expected = c["glyphs"].as_array().unwrap();
        let upm = c["unitsPerEm"].as_f64().unwrap();
        let lift = if shift == Shift::Super { 0.4 } else { -0.15 };
        assert_eq!(actual.glyphs.len(), expected.len());
        let mut pen = 0.;
        for (glyph, want) in actual.glyphs.iter().zip(expected) {
            assert_eq!(u64::from(glyph.id), want["g"].as_u64().unwrap());
            assert_eq!(glyph.bytes.start, want["cl"].as_u64().unwrap() as usize);
            assert_eq!(glyph.size, 0.6);
            assert!((glyph.pen - pen).abs() < 1e-10);
            assert!(
                (glyph.origin[0] - pen - want["dx"].as_f64().unwrap() / upm * 0.6).abs() < 1e-10
            );
            assert!(
                (glyph.origin[1] - lift - want["dy"].as_f64().unwrap() / upm * 0.6).abs() < 1e-10
            );
            assert!((glyph.advance - want["ax"].as_f64().unwrap() / upm * 0.6).abs() < 1e-10);
            pen += want["ax"].as_f64().unwrap() / upm * 0.6;
        }
        assert!((actual.advance - baseline.advance * 0.6).abs() < 1e-10);
        let (a, b) = (actual.ink.unwrap(), baseline.ink.unwrap());
        for (a, b) in
            a.into_iter()
                .zip([b[0] * 0.6, b[1] * 0.6 + lift, b[2] * 0.6, b[3] * 0.6 + lift])
        {
            assert!((a - b).abs() < 1e-10);
        }
        let a = shaping::outline(&actual.glyphs[0]).unwrap();
        let b = shaping::outline(&baseline.glyphs[0]).unwrap();
        let scale = |p: [f64; 2]| [p[0] * 0.6, p[1] * 0.6 + lift];
        let b: Vec<_> = b
            .into_iter()
            .map(|op| match op {
                OutlineOp::Move(p) => OutlineOp::Move(scale(p)),
                OutlineOp::Line(p) => OutlineOp::Line(scale(p)),
                OutlineOp::Quad(a, p) => OutlineOp::Quad(scale(a), scale(p)),
                OutlineOp::Cubic(a, b, p) => OutlineOp::Cubic(scale(a), scale(b), scale(p)),
                OutlineOp::Close => OutlineOp::Close,
            })
            .collect();
        assert_eq!(a, b);
    }
}

#[test]
fn rich_stacked_fractions_match_independent_c_harfbuzz_metrics_including_rtl_numbers() {
    use kentos_geometry_core::text::paragraph::{Run, Script};
    use shaping::{paragraph::Settings, rich};
    let reference: Value =
        serde_json::from_str(include_str!("../../../../../fixtures/text/v1/shaping.json")).unwrap();
    for c in reference["fractions"].as_array().unwrap() {
        let text = c["text"].as_str().unwrap();
        let source = text.as_bytes().to_vec();
        let runs = [Run {
            start: 0,
            end: text.chars().count() as u32,
            script: Some(Script::Stack),
            ..Run::default()
        }];
        let style = Style {
            family: c["family"].as_str().unwrap(),
            weight: c["weight"].as_u64().unwrap() as u16,
            direction: direction(c["direction"].as_str().unwrap()),
            language: Some("und"),
            ..Style::default()
        };
        let actual = rich::layout(
            text,
            &runs,
            style,
            Settings {
                height: 1.,
                width_factor: 1.,
                box_width: None,
                line_spacing: 1.,
                along: 0.,
                flow: None,
            },
        )
        .unwrap();
        assert_eq!(actual.flow.pieces.len(), 1);
        assert!(
            (actual.flow.width - c["width"].as_f64().unwrap()).abs() < 1e-9,
            "{text}"
        );
        let piece = &actual.flow.pieces[0];
        assert!(piece.line.objects.is_empty());
        let expected = c["glyphs"].as_array().unwrap();
        assert_eq!(piece.line.glyphs.len(), expected.len(), "{text}");
        for (glyph, want) in piece.line.glyphs.iter().zip(expected) {
            assert_eq!(u64::from(glyph.id), want["id"].as_u64().unwrap(), "{text}");
            assert_eq!(
                glyph.bytes.start,
                want["byte"].as_u64().unwrap() as usize,
                "{text}"
            );
            assert_eq!(
                glyph.scalars.start,
                text[..glyph.bytes.start].chars().count() as u32
            );
            assert_eq!(
                glyph.scalars.end,
                text[..glyph.bytes.end].chars().count() as u32
            );
            for (actual, want) in [
                (glyph.origin[0], want["x"].as_f64().unwrap()),
                (glyph.origin[1], want["y"].as_f64().unwrap()),
                (glyph.pen, want["pen"].as_f64().unwrap()),
                (glyph.advance, want["advance"].as_f64().unwrap()),
                (glyph.size, want["size"].as_f64().unwrap()),
            ] {
                assert!((actual - want).abs() < 1e-9, "{text}: {actual} {want}");
            }
        }
        let bars = c["bars"].as_array().unwrap();
        assert_eq!(actual.rules.len(), bars.len());
        for (rule, want) in actual.rules.iter().zip(bars) {
            assert_eq!(rule.run, 0);
            assert_eq!(rule.piece, 0);
            assert_eq!(rule.scalars, 0..text.chars().count() as u32);
            for (a, b) in rule.rect.iter().zip(want.as_array().unwrap()) {
                assert!((*a - b.as_f64().unwrap()).abs() < 1e-9);
            }
        }
        assert!(actual.flow.ink.is_some());
        assert_eq!(text.as_bytes(), source);
    }
}

#[test]
fn rich_underline_and_baseline_shift_are_in_the_measured_ink_and_keep_font_style() {
    use kentos_geometry_core::text::paragraph::{Run, Script};
    use shaping::{paragraph::Settings, rich};
    let s = Settings {
        height: 2.,
        width_factor: 0.8,
        box_width: None,
        line_spacing: 1.,
        along: 0.,
        flow: None,
    };
    let run = Run {
        start: 0,
        end: 1,
        underline: true,
        bold: true,
        italic: true,
        script: Some(Script::Sub),
        color: Some("#C21858".into()),
    };
    let actual = rich::layout(
        "A",
        &[run],
        Style {
            family: "arimo",
            ..Style::default()
        },
        s,
    )
    .unwrap();
    assert_eq!(actual.rules.len(), 1);
    let g = &actual.flow.pieces[0].line.glyphs[0];
    assert_eq!(shaping::font_key(g.font).unwrap().1, 600);
    assert_eq!(g.size, 0.6);
    assert_eq!(g.slant, 0.25);
    let rule = &actual.rules[0];
    assert_eq!(rule.run, 0);
    assert_eq!(rule.scalars, 0..1);
    assert!((rule.rect[1] - (-0.15 - 0.18 * 0.6)).abs() < 1e-10);
    assert!((rule.rect[3] - (-0.15 - 0.12 * 0.6)).abs() < 1e-10);
    assert!((actual.flow.ink.unwrap()[1] - rule.rect[1] * s.height).abs() < 1e-10);
    assert!((rule.rect[2] - rule.rect[0] - g.advance).abs() < 1e-10);
    let bad = [Run {
        start: 1,
        end: 2,
        bold: true,
        ..Run::default()
    }];
    assert_eq!(
        rich::layout("g\u{306}", &bad, Style::default(), s),
        Err(Error::InvalidFormats)
    );
    let bad = [Run {
        start: 0,
        end: 1,
        script: Some(Script::Stack),
        ..Run::default()
    }];
    assert_eq!(
        rich::layout("A", &bad, Style::default(), s),
        Err(Error::InvalidStack { bytes: 0..1 })
    );
}

#[test]
fn paint_changes_do_not_reflow_ligatures_and_unimplemented_component_paint_is_explicit() {
    use kentos_geometry_core::text::paragraph::Run;
    use shaping::{paragraph::Settings, rich};
    let s = Settings {
        height: 1.,
        width_factor: 1.,
        box_width: None,
        line_spacing: 1.,
        along: 0.,
        flow: None,
    };
    let style = Style {
        family: "barlow",
        ..Style::default()
    };
    let baseline = rich::layout("office", &[], style, s).unwrap();
    let ffi = baseline.flow.pieces[0]
        .line
        .glyphs
        .iter()
        .find(|g| g.scalars == (1..4))
        .expect("external hb-shape confirms Barlow ffi");
    assert_eq!(ffi.id, 361);
    let colored = [Run {
        start: 1,
        end: 4,
        color: Some("#C21858".into()),
        ..Run::default()
    }];
    let actual = rich::layout("office", &colored, style, s).unwrap();
    assert_eq!(actual.flow, baseline.flow);
    let under = [Run {
        start: 1,
        end: 4,
        underline: true,
        ..Run::default()
    }];
    let actual = rich::layout("office", &under, style, s).unwrap();
    assert_eq!(actual.flow.width, baseline.flow.width);
    assert_eq!(actual.rules.len(), 1);
    assert!((actual.rules[0].rect[2] - actual.rules[0].rect[0] - ffi.advance).abs() < 1e-10);
    let partial = [Run {
        start: 2,
        end: 3,
        color: Some("#C21858".into()),
        ..Run::default()
    }];
    assert_eq!(
        rich::layout("office", &partial, style, s),
        Err(Error::MixedClusterPaint { bytes: 1..4 })
    );
    let redundant = [Run {
        start: 2,
        end: 3,
        bold: true,
        ..Run::default()
    }];
    assert!(
        rich::layout(
            "office",
            &redundant,
            Style {
                weight: 600,
                ..style
            },
            s
        )
        .is_ok()
    );
}

#[test]
fn rich_dense_underlines_and_a_long_combining_cluster_keep_bounded_metadata_work() {
    use kentos_geometry_core::text::paragraph::Run;
    use shaping::{paragraph::Settings, rich};
    let s = Settings {
        height: 0.2,
        width_factor: 1.,
        box_width: Some(0.1),
        line_spacing: 1.,
        along: 0.,
        flow: None,
    };
    let text = "A ".repeat(6000);
    let runs = [Run {
        start: 0,
        end: text.chars().count() as u32,
        underline: true,
        ..Run::default()
    }];
    let p = rich::layout(
        &text,
        &runs,
        Style {
            family: "arimo",
            ..Style::default()
        },
        s,
    )
    .unwrap();
    assert_eq!(p.flow.depth, 6000);
    assert_eq!(p.rules.len(), 6000);
    for (index, rule) in p.rules.iter().enumerate() {
        assert_eq!(rule.piece, index);
        assert_eq!(rule.run, 0);
    }
    let text = format!("g{}", "\u{306}".repeat(600));
    let runs = [Run {
        start: 0,
        end: text.chars().count() as u32,
        color: Some("#017FFF".into()),
        ..Run::default()
    }];
    let p = rich::layout(
        &text,
        &runs,
        Style {
            family: "arimo",
            ..Style::default()
        },
        Settings {
            box_width: None,
            ..s
        },
    )
    .unwrap();
    assert_eq!(p.flow.pieces.len(), 1);
    assert!(p.flow.pieces[0].line.glyphs.len() >= 590);
    assert!(
        p.flow.pieces[0]
            .line
            .glyphs
            .iter()
            .all(|g| g.id != 0 && g.scalars == (0..601))
    );
    assert!(p.rules.is_empty());
}

#[test]
fn mixed_latin_greek_cyrillic_and_paired_brackets_match_explicit_external_script_runs() {
    let reference: Value =
        serde_json::from_str(include_str!("../../../../../fixtures/text/v1/shaping.json")).unwrap();
    for c in reference["mixedScripts"].as_array().unwrap() {
        let text = c["text"].as_str().unwrap();
        let line = shaping::line(
            text,
            Style {
                family: "noto-sans",
                direction: BaseDirection::LeftToRight,
                language: Some("und"),
                ..Style::default()
            },
        )
        .unwrap();
        let want = c["glyphs"].as_array().unwrap();
        assert_eq!(line.glyphs.len(), want.len(), "{text}");
        for (glyph, want) in line.glyphs.iter().zip(want) {
            assert_eq!(glyph.font, line.requested, "{text}");
            assert_eq!(u64::from(glyph.id), want["id"].as_u64().unwrap(), "{text}");
            assert_eq!(
                glyph.bytes.start,
                want["byte"].as_u64().unwrap() as usize,
                "{text}"
            );
            for (a, b) in [
                (glyph.origin[0], want["x"].as_f64().unwrap()),
                (glyph.origin[1], want["y"].as_f64().unwrap()),
                (glyph.advance, want["advance"].as_f64().unwrap()),
            ] {
                assert!((a - b).abs() < 1e-9, "{text}: {a} {b}");
            }
        }
        assert!((line.advance - c["advance"].as_f64().unwrap()).abs() < 1e-9);
    }
}

#[test]
fn positioned_glyphs_match_external_flow_at_rotated_top_center_without_losing_large_coordinates() {
    use kentos_geometry_core::{
        entity::TextPlace, Vec2,
        api::json::{FromJson, Json},
        geom::affine::apply,
        text::{Font, TextAlign, paragraph::Flow},
    };
    let reference: Value =
        serde_json::from_str(include_str!("../../../../../fixtures/text/v1/shaping.json")).unwrap();
    for c in reference["flow"].as_array().unwrap() {
        let flow = Flow::from_json(&Json::parse(&c["flow"].to_string()).unwrap()).unwrap();
        let mut t = TextPlace::line(
            Vec2::new(5_000_000., 4_400_000.),
            c["text"].as_str().unwrap(),
            c["height"].as_f64().unwrap(),
            90.,
            Some(TextAlign::TopCenter),
            c["widthFactor"].as_f64(),
        );
        t.box_width = c["boxWidth"].as_f64();
        t.line_spacing = c["lineSpacing"].as_f64();
        t.flow = Some(&flow);
        let font = Font::from_id(c["family"].as_str().unwrap());
        // Noto Arabic is a fallback face, not a selectable DrawingFont. Its
        // explicit-family paragraph is covered by the lower-level C oracle.
        if font.id() != c["family"].as_str().unwrap() { continue; }
        // The external flow uses its own alignment within a fixed box. For
        // each actual center-aligned row measure the glyph positions with C
        // HarfBuzz again; the persisted oracle already provides exact advances.
        let plan = shaping::placed::layout(&t, font, false).unwrap();
        let h = t.height;
        let sx = h * t.width_factor.unwrap();
        let width = c["width"].as_f64().unwrap();
        let depth = c["depth"].as_u64().unwrap() as usize;
        let pitch = c["pitch"].as_f64().unwrap();
        let up = h;
        assert_eq!(plan.depth, depth);
        let mut index = 0;
        for row in c["pieces"].as_array().unwrap() {
            let row_width = row["advance"].as_f64().unwrap() * sx;
            let original = row["x"].as_f64().unwrap();
            let old_align = c["along"].as_f64().unwrap();
            // Actual available row width comes from the external profile's
            // known left offset and original alignment; don't use our plan.
            let left = original - old_align * (width - row_width);
            // Tabbed/column pieces have shared row adjustment, so compare only
            // cases with full-row pieces below (other flow tests cover tabs).
            let simple = c["flow"].get("columns").is_none()
                && !c["text"].as_str().unwrap().contains('\t')
                && c["flow"].get("indent").is_none()
                && c["flow"].get("leftIndent").is_none();
            let x = left + 0.5 * (width - row_width);
            let y = row["y"].as_f64().unwrap();
            let mut cursor = 0.;
            let em = c["unitsPerEm"].as_f64().unwrap();
            for g in row["glyphs"].as_array().unwrap() {
                let actual = &plan.glyphs[index];
                assert_eq!(u64::from(actual.glyph.id), g["g"].as_u64().unwrap());
                if simple {
                    let local_x = x + (cursor + g["dx"].as_f64().unwrap() / em) * sx - width / 2.;
                    let local_y = y + g["dy"].as_f64().unwrap() / em * h - up;
                    let p = apply(
                        &actual.transform,
                        Vec2::new(actual.glyph.origin[0], actual.glyph.origin[1]),
                    );
                    assert!(
                        (p.x + local_y).abs() < 1e-9 && (p.y - local_x).abs() < 1e-9,
                        "{c}"
                    );
                }
                cursor += g["ax"].as_f64().unwrap() / em;
                index += 1;
            }
        }
        assert_eq!(index, plan.glyphs.len());
        assert!((plan.pitch - pitch).abs() < 1e-9);
        // p stays a separate f64 origin; no cancellation in model glyphs.
        t.p = Vec2::new(-9e12, 2e12);
        assert_eq!(plan, shaping::placed::layout(&t, font, false).unwrap());
        let ink = plan.ink.unwrap();
        for g in &plan.glyphs {
            for op in g.outline().unwrap() {
                let pts = match op {
                    shaping::OutlineOp::Move(p) | shaping::OutlineOp::Line(p) => vec![p],
                    shaping::OutlineOp::Quad(a, b) => vec![a, b],
                    shaping::OutlineOp::Cubic(a, b, c) => vec![a, b, c],
                    shaping::OutlineOp::Close => vec![],
                };
                // Bézier control points can exceed the TTF ink extrema;
                // endpoints are checked separately in the contour oracle.
                assert!(pts.iter().flatten().all(|v| v.is_finite()));
            }
        }
        assert!(ink.iter().all(|x| x.is_finite()));
    }
}

#[test]
fn curved_unicode_keeps_combining_clusters_and_fractions_on_one_analytic_tangent() {
    use kentos_geometry_core::{
        entity::TextPlace, Vec2,
        text::{
            Font,
            along::Curve,
            paragraph::{Run, Script},
        },
    };
    let curve = Curve {
        pts: vec![Vec2::new(10., 10.)],
        bulges: Some(vec![(std::f64::consts::PI / 8.).tan()]),
    };
    let runs = [Run {
        start: 3,
        end: 6,
        script: Some(Script::Stack),
        underline: true,
        ..Run::default()
    }];
    let mut t = TextPlace::line(
        Vec2::new(5e6, 4.4e6),
        "a\u{0301}\u{0323}1/2",
        1.,
        0.,
        None,
        None,
    );
    t.runs = &runs;
    t.path = Some(&curve);
    let plan = shaping::placed::layout(&t, Font::from_id("arimo"), false).unwrap();
    let base = plan
        .glyphs
        .iter()
        .filter(|g| g.glyph.scalars.start == 0)
        .collect::<Vec<_>>();
    assert!(base.len() >= 2, "must exercise separate base/mark glyphs");
    for g in &base {
        assert_eq!(g.transform, base[0].transform);
    }
    let fraction = plan
        .glyphs
        .iter()
        .filter(|g| g.glyph.scalars.start >= 3)
        .collect::<Vec<_>>();
    assert_eq!(fraction.len(), 2);
    assert_eq!(fraction[0].transform, fraction[1].transform);
    // The arc is x=10sin(theta), y=10-10cos(theta), radius ten.
    // Its glyph group's center lies at arc length (left+right)/2.
    for group in [&base, &fraction] {
        let left = group
            .iter()
            .map(|g| g.glyph.pen)
            .fold(f64::INFINITY, f64::min);
        let right = group
            .iter()
            .map(|g| g.glyph.pen + g.glyph.advance)
            .fold(f64::NEG_INFINITY, f64::max);
        let middle = (left + right) / 2.;
        let theta = middle / 10.;
        let m = group[0].transform;
        assert!((m[0] - theta.cos()).abs() < 1e-12);
        assert!((m[1] - theta.sin()).abs() < 1e-12);
        assert!((m[4] - (10. * theta.sin() - middle * theta.cos())).abs() < 1e-12);
        assert!((m[5] - (10. - 10. * theta.cos() - middle * theta.sin())).abs() < 1e-12);
    }
    assert_eq!(
        plan.rules.len(),
        2,
        "fraction bar and underline are both preserved"
    );
}

#[test]
fn straight_curve_and_straight_text_share_contours_and_profile_geometry_rejects_overflow() {
    use kentos_geometry_core::{
        entity::TextPlace, Vec2,
        text::{Font, along::Curve},
    };
    let curve = Curve {
        pts: vec![Vec2::new(0., 0.), Vec2::new(100., 0.)],
        bulges: None,
    };
    let mut t = TextPlace::line(
        Vec2::new(0., 0.),
        "Ög\u{0306}ç AV مرحبا",
        0.3,
        37.,
        None,
        Some(0.8),
    );
    let flat = shaping::placed::layout(&t, Font::from_id("arimo"), true).unwrap();
    t.path = Some(&curve);
    let bent = shaping::placed::layout(&t, Font::from_id("arimo"), true).unwrap();
    assert_eq!(flat.glyphs.len(), bent.glyphs.len());
    for (a, b) in flat.glyphs.iter().zip(&bent.glyphs) {
        assert_eq!(a.glyph, b.glyph);
        for (x, y) in a.transform.iter().zip(b.transform) {
            assert!((x - y).abs() < 1e-12);
        }
    }
    t.height = f64::MAX;
    assert_eq!(
        shaping::placed::layout(&t, Font::DEFAULT, false),
        Err(Error::InvalidGeometry)
    );
}

#[test]
fn curved_profile_geometry_uses_atomic_groups_for_origin_direction_width_and_masks() {
    use kentos_geometry_core::{entity::TextPlace,Vec2,text::{Font,along::Curve,paragraph::{Flow,Engine}}};
    let flow=Flow {engine:Some(Engine::UnicodeV1),..Default::default()};
    let curve=Curve {pts:vec![Vec2::new(100.,0.)],bulges:None};
    let mut t=TextPlace::line(Vec2::new(5_000_000.,4_400_000.),"a\u{301}\u{323} AV",0.3,0.,None,Some(0.8));
    t.flow=Some(&flow);t.path=Some(&curve);t.box_width=Some(0.01);
    let font=Font::from_id("arimo");
    let plan=t.unicode_plan(font).unwrap();
    assert_eq!(plan.clusters.len(),4,"base/marks, space, A, V");
    assert_eq!(t.origin(font),t.p);
    assert_eq!(t.curved_baseline(font),Some((t.p,0.)));
    assert!((t.width(font)-plan.width).abs()<1e-12,"curve text ignores paragraph wrapping");
    let [a,b,c,d]=plan.frame.unwrap();
    let margin=t.height*0.1;
    let ring=t.mask(font);
    assert!((ring[0].x-(t.p.x+a-margin)).abs()<1e-9);
    assert!((ring[0].y-(t.p.y+b-margin)).abs()<1e-9);
    assert!((ring[2].x-(t.p.x+c+margin)).abs()<1e-9);
    assert!((ring[2].y-(t.p.y+d+margin)).abs()<1e-9);
    t.rotation=180.;
    let reversed=t.readable_curve(font).unwrap();
    assert!((reversed.p.x-(t.p.x-100.)).abs()<1e-9);
    assert_eq!(reversed.rotation,0.);
}

#[test]
fn unicode_readable_keeps_the_measured_frame_at_its_original_location() {
    use kentos_geometry_core::{
        Vec2,
        entity::TextPlace,
        text::{
            Font, TextAlign,
            paragraph::{Engine, Flow},
        },
    };
    let flow = Flow {
        engine: Some(Engine::UnicodeV1),
        ..Default::default()
    };
    let font = Font::from_id("arimo");
    for text in [
        "Ög\u{301}\u{306}\u{323}ç — AV",
        "مرحبا עברית\n⌀32 ≤ 5‰",
        "   ",
    ] {
        for rotation in [91., 137., 180., 257., 270.] {
            for align in [
                None,
                Some(TextAlign::TopCenter),
                Some(TextAlign::BottomRight),
            ] {
                let mut t = TextPlace::line(
                    Vec2::new(5_000_000., 4_400_000.),
                    text,
                    0.3,
                    rotation,
                    align,
                    Some(0.8),
                );
                t.flow = Some(&flow);
                t.lean = 0.45;
                let before = t.outline(font);
                let old = t.unicode_plan(font).unwrap();
                if text == "Ög\u{301}\u{306}\u{323}ç — AV" && align.is_none() {
                    // Independent C HarfBuzz + FontTools glyf bounds, Arimo
                    // for Ö/ç—AV and Noto Sans for the complete g+marks
                    // cluster (Arimo lacks U+0306). Both fonts' own UPMs.
                    let expected = [-0.03105, -0.1248, 1.271990625, 0.345];
                    for (actual, expected) in old.frame.unwrap().iter().zip(expected) {
                        assert!((actual - expected).abs() < 1e-12);
                    }
                    assert!((old.width - 1.116740625).abs() < 1e-12);
                }
                let (p, rotation) = t.readable(font).unwrap();
                t.p = p;
                t.rotation = rotation;
                let after = t.outline(font);
                // A half turn swaps opposite corners, including the true glyph
                // overhang and the sheared nominal frame; it must not drift.
                for (i, a) in before.iter().enumerate() {
                    let b = after[(i + 2) % 4];
                    assert!(
                        (a.x - b.x).abs() < 2e-9 && (a.y - b.y).abs() < 2e-9,
                        "{text:?} angle {rotation} align {align:?}: {a:?} != {b:?}"
                    );
                }
                let new = t.unicode_plan(font).unwrap();
                assert_eq!(
                    old.glyphs.iter().map(|g| &g.glyph).collect::<Vec<_>>(),
                    new.glyphs.iter().map(|g| &g.glyph).collect::<Vec<_>>()
                );
                assert!(t.readable(font).is_none(), "the command is idempotent");
            }
        }
    }
}

#[test]
fn unicode_realign_keeps_every_glyph_and_rule_in_world_coordinates() {
    use kentos_geometry_core::{
        Vec2,
        entity::TextPlace,
        text::{
            Font, TextAlign,
            paragraph::{Engine, Flow},
        },
    };
    let flow = Flow {
        engine: Some(Engine::UnicodeV1),
        left: 0.4,
        ..Default::default()
    };
    let font = Font::from_id("arimo");
    let aligns = [
        None,
        Some(TextAlign::TopCenter),
        Some(TextAlign::BottomRight),
        Some(TextAlign::MiddleLeft),
    ];
    for multiline in [false, true] {
        for from in aligns {
            for to in aligns {
                // Horizontal paragraph alignment also justifies each shorter
                // line: that intentional change is not a translation invariant.
                if multiline && from.map_or(0., TextAlign::along) != to.map_or(0., TextAlign::along)
                {
                    continue;
                }
                let text = if multiline {
                    "Ög\u{306}ç AV مرحبا עברית\n⌀32 ≤ 5‰"
                } else {
                    "Ög\u{306}ç AV مرحبا עברית"
                };
                let runs = [kentos_geometry_core::text::paragraph::Run {
                    start: 0,
                    end: 4,
                    underline: true,
                    ..Default::default()
                }];
                let mut t = TextPlace::line(
                    Vec2::new(5_000_000., 4_400_000.),
                    text,
                    0.3,
                    137.,
                    from,
                    Some(0.8),
                );
                t.flow = Some(&flow);
                t.lean = 0.45;
                t.runs = &runs;
                if multiline {
                    t.box_width = Some(2.5);
                }
                let old = t.unicode_plan(font).unwrap();
                let anchor = t.p;
                t.p = t.realigned(to, font);
                t.align = to;
                let new = t.unicode_plan(font).unwrap();
                assert_eq!(old.glyphs.len(), new.glyphs.len());
                for (a, b) in old.glyphs.iter().zip(&new.glyphs) {
                    assert_eq!(a.glyph, b.glyph);
                    assert_eq!(&a.transform[..4], &b.transform[..4]);
                    assert!((anchor.x + a.transform[4] - t.p.x - b.transform[4]).abs() < 2e-9);
                    assert!((anchor.y + a.transform[5] - t.p.y - b.transform[5]).abs() < 2e-9);
                }
                assert_eq!(old.rules.len(), new.rules.len());
                for (a, b) in old.rules.iter().zip(&new.rules) {
                    for (a, b) in a.corners.iter().zip(b.corners) {
                        assert!((anchor.x + a[0] - t.p.x - b[0]).abs() < 2e-9);
                        assert!((anchor.y + a[1] - t.p.y - b[1]).abs() < 2e-9);
                    }
                }
            }
        }
    }
}

#[test]
fn unicode_mirror_keeps_the_measured_center_at_the_reflected_location() {
    use kentos_geometry_core::api::json::{FromJson, Json};
    use kentos_geometry_core::{
        Vec2,
        entity::{Shape, TextPlace},
        geom::affine::{apply, mirror},
        ops::transform::{TextPolicy, transform_shape_with},
        text::Font,
    };
    let center = |s: &Shape| {
        let t = TextPlace::of(s).unwrap();
        let ring = t.outline(Font::DEFAULT);
        Vec2::new((ring[0].x + ring[2].x) / 2., (ring[0].y + ring[2].y) / 2.)
    };
    for words in ["Ög\u{301}\u{306}\u{323}ç — AV", "مرحبا עברית\n⌀32 ≤ 5‰"] {
        for rotation in [0., 37., 137., 257.] {
            let e: kentos_geometry_core::entity::Entity = kentos_geometry_core::entity::Entity::from_json(&Json::parse(&serde_json::json!({
                "kind":"text","p":{"x":5000000.,"y":4400000.},"text":words,"height":0.3,"rotation":rotation,
                "font":"arimo","oblique":24.,"boxWidth":2.,"layout":{"engine":"unicode-v1"}
            }).to_string()).unwrap()).unwrap();
            for b in [Vec2::new(1., 0.), Vec2::new(0., 1.), Vec2::new(1., 0.4)] {
                let m = mirror(
                    Vec2::new(5000001., 4400001.),
                    Vec2::new(5000001. + b.x, 4400001. + b.y),
                );
                let expected = apply(&m, center(&e.shape));
                let changed = transform_shape_with(&e.shape, &m, TextPolicy::Readable);
                let actual = center(&changed);
                assert!(
                    (actual.x - expected.x).abs() < 2e-9 && (actual.y - expected.y).abs() < 2e-9,
                    "{words:?} {rotation}: {actual:?} != {expected:?}"
                );
                let t = TextPlace::of(&changed).unwrap();
                assert!(!(t.rotation > 90. && t.rotation <= 270.));
                assert_eq!(t.text, words);
                assert!(t.is_unicode());
            }
        }
    }
}

#[test]
fn unicode_nested_mirrors_keep_inherited_fonts_and_rebuild_after_a_project_font_change() {
    use kentos_geometry_core::{
        Vec2,
        api::json::{FromJson, Json},
        block::{Blocks, Definition, Underlay, placement},
        entity::{Entity, Shape, TextPlace},
        geom::affine::apply,
        store::Store,
        text::Font,
    };
    let entity = |v: Value| Entity::from_json(&Json::parse(&v.to_string()).unwrap()).unwrap();
    let original = entity(
        serde_json::json!({"kind":"text","p":{"x":0.,"y":0.},"text":"Ög\u{306}ç AV مرحبا","height":0.3,
        "rotation":37.,"boxWidth":2.,"layout":{"engine":"unicode-v1"}}),
    );
    let inner = entity(
        serde_json::json!({"kind":"insert","block":"inner","p":{"x":2.,"y":1.},"scale":2.,"rotation":0.3,"mirror":true}),
    );
    let outer = entity(
        serde_json::json!({"kind":"insert","block":"outer","p":{"x":5000000.,"y":4400000.},"scale":1.5,"rotation":0.7,"mirror":true}),
    );
    let defs = vec![
        Definition {
            id: "inner".into(),
            base: Vec2::new(0., 0.),
            entities: vec![original.clone()],
            attributes: vec![],
            dynamic: None,
            underlay: Some(Underlay {
                fade: 0.3,
                monochrome: false,
            }),
        },
        Definition {
            id: "outer".into(),
            base: Vec2::new(0., 0.),
            entities: vec![inner],
            attributes: vec![],
            dynamic: None,
            underlay: None,
        },
    ];
    let center = |shape: &Shape, font| {
        let t = TextPlace::of(shape).unwrap();
        let ring = t.outline(font);
        Vec2::new((ring[0].x + ring[2].x) / 2., (ring[0].y + ring[2].y) / 2.)
    };
    let mut store = Store::new();
    store.set_blocks(Blocks::new_in(defs.clone(), Font::DEFAULT));
    store.put(1., "a", false, outer.shape.clone());
    for font in [
        Font::from_id("arimo"),
        Font::from_id("overpass"),
        Font::DEFAULT,
    ] {
        store.set_font(font);
        let expected = apply(
            &placement(Vec2::new(5000000., 4400000.), 1.5, 0.7, true),
            apply(
                &placement(Vec2::new(2., 1.), 2., 0.3, true),
                center(&original.shape, font),
            ),
        );
        let parts = store.get(1.).unwrap().expanded.as_ref().unwrap();
        let piece = &parts.shapes[0];
        let actual = center(piece, font);
        assert!(
            (actual.x - expected.x).abs() < 3e-9 && (actual.y - expected.y).abs() < 3e-9,
            "{font:?}: {actual:?} != {expected:?}"
        );
        assert_eq!(
            parts.strokes[0].transparency,
            Some(0.3),
            "rebuilding retains source metadata"
        );
        let placed = TextPlace::of(piece).unwrap();
        assert_eq!(
            placed.font, None,
            "a runtime measurement must not freeze an inherited font into the object"
        );
        let rebuilt = Blocks::new_in(defs.clone(), font).expand(&outer.shape);
        assert_eq!(
            &rebuilt[0].shape, piece,
            "font-change cache matches a fresh library"
        );
    }
}
