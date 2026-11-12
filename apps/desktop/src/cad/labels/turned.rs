//! Turned texts (leaning, narrowed or masked) drawn as pictures from their glyph outlines, the outlines
//! kept between frames; the object a spot writes for.

use super::*;

pub(super) fn draw(frame: &mut Frame, piece: &Piece<'_>, halo: Color) {
    if piece.size < SMALLEST || piece.text.is_empty() {
        return;
    }
    // Measured only where the anchor needs it (a label beside a point does not).
    let k = piece.size / REFERENCE;
    let m = || measure(piece.text, piece.font);
    let dx = match piece.anchor {
        Anchor::LeftBaseline => 0.0,
        Anchor::CenterBaseline | Anchor::CenterMiddle => -m().width * k / 2.0,
    };
    let dy = match piece.anchor {
        Anchor::LeftBaseline | Anchor::CenterBaseline => -m().baseline * k,
        Anchor::CenterMiddle => -piece.size / 2.0,
    };
    let text = |position: Point, color: Color| canvas::Text {
        content: piece.text.to_owned(),
        position,
        max_width: f32::INFINITY,
        color,
        size: Pixels(piece.size),
        line_height: text::LineHeight::Relative(1.0),
        font: piece.font,
        align_x: text::Alignment::Left,
        align_y: Vertical::Top,
        shaping: text::Shaping::Advanced,
    };
    // Upright, plain text through the glyph cache; a width factor, a mask or a lean needs the frame's transform.
    if piece.angle.abs() < 1e-4
        && piece.width_factor == 1.0
        && piece.mask <= 0.0
        && piece.lean == 0.0
    {
        let top_left = Point::new(piece.at.x + dx, piece.at.y + dy);
        // The halo as eight copies around the text, then the text; true sizes have none (docs/adr/0626 §6).
        if halo.a > 0.0 {
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::FRAC_PI_4;
                frame.fill_text(text(
                    top_left + Vector::new(a.cos() * HALO, a.sin() * HALO),
                    halo,
                ));
            }
        }
        frame.fill_text(text(top_left, piece.color));
        underline(frame, piece, Point::new(piece.at.x + dx, piece.at.y), halo);
        return;
    }
    // As a picture with exact coverage (label_picture.rs); a text too large for one as paths.
    if pictured(frame, piece, halo, dx, dy) {
        return;
    }
    frame.with_save(|frame| {
        frame.translate(Vector::new(piece.at.x, piece.at.y));
        frame.rotate(piece.angle);
        // The mask (docs/adr/0145): the text's box a tenth of its height wider all round,
        // 1.15 of it over the baseline and 0.23 under (`TextPlace::mask`), in the area's colour;
        // from where the text starts. A dimension's value (centred) is wider on its sides only,
        // leaving its dimension line and an arc length's symbol in view (docs/adr/0147).
        // A leaning text's mask leans with its letters from the baseline (docs/adr/0183 §2).
        let sheared = |path: Path, lean: f32| {
            if lean == 0.0 {
                path
            } else {
                path.transform(&canvas::path::lyon_path::math::Transform::new(
                    1.0, 0.0, -lean, 1.0, 0.0, 0.0,
                ))
            }
        };
        if piece.mask > 0.0 {
            let m = piece.size * 0.1;
            let v = if piece.anchor == Anchor::CenterBaseline {
                0.0
            } else {
                m
            };
            frame.fill(
                &sheared(
                    Path::rectangle(
                        Point::new(dx - m, -piece.size * 1.15 - v),
                        Size::new(piece.mask + 2.0 * m, piece.size * 1.38 + 2.0 * v),
                    ),
                    piece.lean,
                ),
                Color { a: 1.0, ..halo },
            );
        }
        if piece.width_factor != 1.0 {
            frame.scale_nonuniform(Vector::new(piece.width_factor, 1.0));
        }
        // The letters lean before the width factor narrows them: their own lean is the text's over the factor.
        let lean = piece.lean / piece.width_factor;
        let glyphs = outlines(piece, dx, dy, || {
            let mut glyphs: Vec<Path> = Vec::new();
            text(Point::new(dx, dy), piece.color)
                .draw_with(|path, _| glyphs.push(sheared(path, lean)));
            glyphs
        });
        let stroke = Stroke {
            width: HALO * 2.0,
            line_join: LineJoin::Round,
            ..Stroke::default()
        }
        .with_color(halo);
        if halo.a > 0.0 {
            for glyph in glyphs.iter() {
                frame.stroke(glyph, stroke);
            }
        }
        for glyph in glyphs.iter() {
            frame.fill(glyph, piece.color);
        }
        underline(frame, piece, Point::new(dx, 0.0), halo);
    });
}

/// A turned (leaning, narrowed or masked) text as a picture: its mask, its glyphs' halo, its glyphs and its
/// underline as the paths below draw them. False when it is too large for a picture.
pub(super) fn pictured(
    frame: &mut Frame,
    piece: &Piece<'_>,
    halo: Color,
    dx: f32,
    dy: f32,
) -> bool {
    use crate::label_picture::{Key, Local, Part};
    let bits = |c: Color| [c.r.to_bits(), c.g.to_bits(), c.b.to_bits(), c.a.to_bits()];
    let mut numbers = vec![
        piece.size.to_bits(),
        dx.to_bits(),
        dy.to_bits(),
        piece.lean.to_bits(),
        piece.width_factor.to_bits(),
        piece.mask.to_bits(),
        piece.underline.to_bits(),
    ];
    numbers.extend(bits(piece.color));
    numbers.extend(bits(halo));
    let key = Key {
        what: format!("{}\u{0}{:?}", piece.text, piece.font),
        numbers,
    };
    crate::label_picture::draw(frame, key, piece.at, piece.angle, |add| {
        let sheared = |path: Path, lean: f32| {
            if lean == 0.0 {
                path
            } else {
                path.transform(&canvas::path::lyon_path::math::Transform::new(
                    1.0, 0.0, -lean, 1.0, 0.0, 0.0,
                ))
            }
        };
        let plain = Local { width_factor: 1.0 };
        let narrow = Local {
            width_factor: piece.width_factor,
        };
        if piece.mask > 0.0 {
            let m = piece.size * 0.1;
            let v = if piece.anchor == Anchor::CenterBaseline {
                0.0
            } else {
                m
            };
            let mask = sheared(
                Path::rectangle(
                    Point::new(dx - m, -piece.size * 1.15 - v),
                    Size::new(piece.mask + 2.0 * m, piece.size * 1.38 + 2.0 * v),
                ),
                piece.lean,
            );
            add(Part::Fill(std::slice::from_ref(&mask), plain, Color { a: 1.0, ..halo }));
        }
        let lean = piece.lean / piece.width_factor;
        let glyphs = outlines(piece, dx, dy, || {
            let mut glyphs: Vec<Path> = Vec::new();
            canvas::Text {
                content: piece.text.to_owned(),
                position: Point::new(dx, dy),
                max_width: f32::INFINITY,
                color: piece.color,
                size: Pixels(piece.size),
                line_height: text::LineHeight::Relative(1.0),
                font: piece.font,
                align_x: text::Alignment::Left,
                align_y: Vertical::Top,
                shaping: text::Shaping::Advanced,
            }
            .draw_with(|path, _| glyphs.push(sheared(path, lean)));
            glyphs
        });
        if halo.a > 0.0 {
            add(Part::Halo(&glyphs, narrow, halo, HALO));
        }
        add(Part::Fill(&glyphs, narrow, piece.color));
        if piece.underline > 0.0 {
            let (top, thick) = (piece.size * 0.12, (piece.size * 0.06).max(1.0));
            let back = Path::rectangle(
                Point::new(dx - HALO, top - HALO),
                Size::new(piece.underline + 2.0 * HALO, thick + 2.0 * HALO),
            );
            let bar = Path::rectangle(Point::new(dx, top), Size::new(piece.underline, thick));
            if halo.a > 0.0 {
                add(Part::Fill(std::slice::from_ref(&back), narrow, halo));
            }
            add(Part::Fill(std::slice::from_ref(&bar), narrow, piece.color));
        }
    })
}

/// Turned texts' glyph outlines kept at most (a view at 1:1000 over a
/// parcel sheet turns about 1 300 edge lengths).
pub(super) const OUTLINES_KEPT: usize = 8_192;

/// What a turned text's outlines depend on: the text, its typeface, its
/// size, its offset from the anchor and its lean. The turn and the place
/// are the frame's transform, so a pan finds every text of the last frame here.
pub(super) type OutlineKey = (String, Font, u32, u32, u32, u32);

/// A turned text's glyph outlines in its own frame, from the kept ones or
/// made by `make`; the kept ones are let go all at once when too many.
pub(super) fn outlines(
    piece: &Piece<'_>,
    dx: f32,
    dy: f32,
    make: impl FnOnce() -> Vec<Path>,
) -> Rc<Vec<Path>> {
    let key = (
        piece.text.to_owned(),
        piece.font,
        piece.size.to_bits(),
        dx.to_bits(),
        dy.to_bits(),
        (piece.lean / piece.width_factor).to_bits(),
    );
    kept_outlines(key, make)
}

/// A text's glyph outlines `size` pixels high, its baseline's left end at
/// the origin (a map service's letter along its line, services/overlay.rs),
/// kept as a turned text's are.
pub(crate) fn text_outlines(text: &str, font: Font, size: f32) -> Rc<Vec<Path>> {
    let dy = -measure(text, font).baseline * size / REFERENCE;
    let key = (
        text.to_owned(),
        font,
        size.to_bits(),
        0f32.to_bits(),
        dy.to_bits(),
        0f32.to_bits(),
    );
    kept_outlines(key, || {
        let mut glyphs: Vec<Path> = Vec::new();
        canvas::Text {
            content: text.to_owned(),
            position: Point::new(0.0, dy),
            max_width: f32::INFINITY,
            color: Color::BLACK,
            size: Pixels(size),
            line_height: text::LineHeight::Relative(1.0),
            font,
            align_x: text::Alignment::Left,
            align_y: Vertical::Top,
            shaping: text::Shaping::Advanced,
        }
        .draw_with(|path, _| glyphs.push(path));
        glyphs
    })
}

/// How far below the top of a line one `size` high a text's baseline falls in `font`.
pub(crate) fn baseline_in(text: &str, font: Font, size: f32) -> f32 {
    measure(text, font).baseline * size / REFERENCE
}

pub(super) fn kept_outlines(key: OutlineKey, make: impl FnOnce() -> Vec<Path>) -> Rc<Vec<Path>> {
    OUTLINES.with(|kept| {
        if let Some(glyphs) = kept.borrow().get(&key) {
            return glyphs.clone();
        }
        let glyphs = Rc::new(make());
        let mut kept = kept.borrow_mut();
        if kept.len() >= OUTLINES_KEPT {
            kept.clear();
        }
        kept.insert(key, glyphs.clone());
        glyphs
    })
}

/// The object a spot writes for.
pub fn slot_of(spot: &LabelSpot) -> Slot {
    match spot {
        LabelSpot::Dimension { slot, .. }
        | LabelSpot::Text { slot, .. }
        | LabelSpot::Placed { slot, .. }
        | LabelSpot::PlacedLine { slot, .. }
        | LabelSpot::PlacedLetter { slot, .. }
        | LabelSpot::PlacedCallout { slot, .. }
        | LabelSpot::PieceText { slot, .. }
        | LabelSpot::PieceDimension { slot, .. }
        | LabelSpot::Line { slot, .. }
        | LabelSpot::ParagraphMask { slot, .. }
        | LabelSpot::Cell { slot, .. } => *slot,
    }
}

/// An object's slot in the spots (for tests).
#[cfg(test)]
pub(super) fn slots(spots: &[LabelSpot]) -> Vec<kentos_domain::Slot> {
    spots.iter().map(slot_of).collect()
}
