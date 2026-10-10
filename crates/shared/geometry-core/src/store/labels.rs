//! Labels and grips from the store (`drawLabels`, `drawGrips`,
//! `apps/web/src/viewport/overlay.ts`): which texts, dimensions and labels a frame
//! draws and where, and the grips of selected objects. The overlay walked
//! every object each frame; the store asks its tree for the view and applies
//! the same tests (visible layer, box in view, text size on screen, the
//! label style's scale range and smallest feature). Strings and styles stay
//! with the overlay: it reads them from the object.

use super::Store;
use super::legible::{LABEL_SHOWN_STRIDE, TRUE_MAX_PX, TRUE_MIN_PX};
use super::placing::{PlaceOptions, Shown};
use crate::entity::{Shape, TextPlace, dimension_geom};
use crate::geom::dimension::layout_dimension;
use crate::geom::leader::note_place;
use crate::geometry::Bounds;
use crate::ops::grips::{entity_grips, mid_grip_segment};

/// What a label record says (`labels`, second number).
pub const LABEL_DIMENSION: f64 = 0.0;
pub const LABEL_TEXT: f64 = 1.0;
// 2–5 were the old labels' (centre, corner, beside, along); the label engine's are `placing::LABEL_PLACED` ….
/// A text among a block's pieces (docs/adr/0144): x, y where its baseline
/// starts, a its rotation, b its height as placed, c the piece's place, d
/// its width factor, e its mask's width (as a text's).
pub const LABEL_PIECE_TEXT: f64 = 6.0;
/// A dimension among a block's pieces: x, y its value's place, a its angle,
/// b the value, c the piece's place, d its text height as placed, e 1 for a
/// mask (docs/adr/0147; its unit and prefix from its style, `dimension_measure`).
pub const LABEL_PIECE_DIMENSION: f64 = 7.0;
/// A leader's note (docs/adr/0146 §5), as a text's: x, y where its baseline
/// starts, a its rotation, b 1 (it has no width factor), c its mask's width.
pub const LABEL_LEADER: f64 = 8.0;
/// A leader's note among a block's pieces, as `LABEL_PIECE_TEXT`.
pub const LABEL_PIECE_LEADER: f64 = 9.0;
/// One line of a multi-line text (docs/adr/0182 §3): x, y where its
/// baseline starts, a the text's rotation, b its height, c its width factor
/// (1 without one), d and e the line's letters `start..end` (Unicode scalar
/// values of the text). Its letters' formats are the text's `runs`. A
/// leaning text's (docs/adr/0183 §2) lower lines start their depth times
/// the slant's tangent back: the box leans as a whole; each line leans its
/// letters from its own baseline.
pub const LABEL_LINE: f64 = 10.0;
/// A multi-line text's mask (docs/adr/0182 §3), before its lines, its own
/// or a block's piece's: x, y the box's corner under its first letter's
/// left, a the rotation, b the box's width along the baseline and c its
/// height up from the corner, metres, a tenth of the height around the lines;
/// d a block's piece's place (0 for a text's own). A leaning text's corner
/// moves with the slant; the box leans from it.
pub const LABEL_PARAGRAPH_MASK: f64 = 11.0;
/// One line of a multi-line text among a block's pieces, as `LABEL_LINE`
/// but c the piece's place (its width factor the piece's own: an insert
/// scales evenly) and b its height as placed.
pub const LABEL_PIECE_LINE: f64 = 12.0;

/// One cell's words of a table (docs/adr/0184 §2): x, y where its baseline
/// starts, a the table's rotation, b its row, c its column, d 1 when bold (a
/// heading row's, or the face's); its words and face are the table's.
pub const LABEL_CELL: f64 = 13.0;

/// Numbers per label record.
pub const LABEL_STRIDE: usize = 9;

/// A dimension's unit as its label record says it (c): length, angle,
/// percent, coordinate (docs/adr/0147).
pub const DIMENSION_UNITS: [&str; 4] = ["length", "angle", "percent", "coordinate"];
/// A dimension's prefix as its label record says it (d).
pub const DIMENSION_PREFIXES: [&str; 7] = ["", "R ", "Ø ", "Y=", "X=", "t=", "%"];

fn code(of: &[&str], s: &str) -> f64 {
    of.iter().position(|x| *x == s).unwrap_or(0) as f64
}

/// 1 for a dimension with a mask or its value on its line, else 0.
fn masked(s: &Shape) -> f64 {
    match s {
        Shape::Dimension {
            mask: Some(true), ..
        } => 1.0,
        // A value on its line hides the line under it, as a mask does (docs/adr/0183 §3).
        Shape::Dimension { look, .. } if look.centre => 1.0,
        _ => 0.0,
    }
}

impl Store {
    /// What the overlay draws in `view` at `scale` px/m, in the document's
    /// order, `LABEL_STRIDE` numbers each: `id, what, x, y, a, b, c, d, e`.
    /// - dimension: x, y the value's place, a its angle (degrees), b the
    ///   value, c its unit (`DIMENSION_UNITS`: 0 length, 1 angle, 2 percent,
    ///   3 coordinate), d its prefix (`DIMENSION_PREFIXES`: 0 none, 1 "R ",
    ///   2 "Ø ", 3 "Y=", 4 "X=", 5 "t=", 6 "%"), e 1 for a mask (docs/adr/0147:
    ///   the overlay fills the value's measured box, as a text's);
    /// - text: x, y where its baseline starts (its `p` moved by its
    ///   alignment, docs/adr/0145), a its rotation, b its width factor (1
    ///   without one), c its mask's width, metres (0 without a mask: the
    ///   overlay fills `TextPlace::mask`'s box, from the origin c along and
    ///   a line and a tenth of the height around);
    /// - centre and beside: x, y the anchor; corner: x, y the box's top left;
    /// - along: x, y and a, b the two vertices the label sits between;
    /// - a leader's note (`LABEL_LEADER`), as a text's;
    /// - a block's text, dimension or leader pieces (`LABEL_PIECE_TEXT`,
    ///   `LABEL_PIECE_DIMENSION`, `LABEL_PIECE_LEADER`), before the insert's own label.
    ///
    /// `editing` is left out (the inline editor draws it).
    ///
    /// The objects' labels follow the drawing's text, placed by the label
    /// engine around it (docs/adr/0212 §3.8): `LABEL_PLACED` and its lines,
    /// letters and callout, their texts in `Shown::texts`.
    pub fn labels(
        &self,
        view: &Bounds,
        scale: f64,
        editing: Option<f64>,
        options: PlaceOptions,
    ) -> Shown {
        let shown = self.labels_shown(
            view,
            scale,
            editing,
            super::legible::LabelSize::True,
            options,
        );
        let mut records =
            Vec::with_capacity(shown.records.len() / LABEL_SHOWN_STRIDE * LABEL_STRIDE);
        for r in shown.records.chunks_exact(LABEL_SHOWN_STRIDE) {
            records.extend_from_slice(&r[..LABEL_STRIDE]);
        }
        Shown {
            records,
            texts: shown.texts,
        }
    }

    /// The drawing's own text at true size: texts, dimension values, leaders' notes, tables' cells.
    pub fn texts_only(&self, view: &Bounds, scale: f64, editing: Option<f64>) -> Vec<f64> {
        self.labels_with(view, scale, editing, TRUE_MIN_PX, TRUE_MAX_PX)
    }

    /// `labels`, a text, a dimension's value, a leader's note or a table's
    /// cells drawn when `min_px..=max_px` high on screen.
    pub(super) fn labels_with(
        &self,
        view: &Bounds,
        scale: f64,
        editing: Option<f64>,
        min_px: f64,
        max_px: f64,
    ) -> Vec<f64> {
        let mut out = Vec::new();
        for it in self.candidates(&super::padded(*view, 0.0)) {
            let flags = self.flags(it);
            if !flags.visible || !self.view_shown(it.id) {
                continue;
            }
            let b = &it.bounds;
            if b.max_x < view.min_x
                || b.min_x > view.max_x
                || b.max_y < view.min_y
                || b.min_y > view.max_y
            {
                continue;
            }
            if Some(it.id) == editing {
                continue;
            }
            // A block's texts and dimension values, where its pieces are placed (docs/adr/0144).
            if let Some(x) = &it.expanded {
                for (i, s) in x.shapes.iter().enumerate() {
                    match s {
                        // An attribute's text that shows nothing has no label (docs/adr/0144 §7).
                        Shape::Text {
                            text,
                            height,
                            rotation,
                            mask,
                            ..
                        } => {
                            let px = height * scale;
                            if px < min_px || px > max_px || text.is_empty() {
                                continue;
                            }
                            // A multi-line piece, line by line (docs/adr/0182 §3); one along a
                            // curve, letter by letter (docs/adr/0196 §2.6).
                            if let Some(t) = TextPlace::of(s).filter(TextPlace::by_records) {
                                let piece = Some(i as f64);
                                self.paragraph_labels(
                                    it.id,
                                    &t,
                                    *mask == Some(true),
                                    piece,
                                    &mut out,
                                );
                                continue;
                            }
                            let (o, factor, mask) = self.text_label(s);
                            out.extend([
                                it.id,
                                LABEL_PIECE_TEXT,
                                o.x,
                                o.y,
                                *rotation,
                                *height,
                                i as f64,
                                factor,
                                mask,
                            ]);
                        }
                        Shape::Leader {
                            height, rotation, ..
                        } => {
                            let px = height * scale;
                            let Some((o, mask)) = self.note_label(s) else {
                                continue;
                            };
                            if px < min_px || px > max_px {
                                continue;
                            }
                            out.extend([
                                it.id,
                                LABEL_PIECE_LEADER,
                                o.x,
                                o.y,
                                *rotation,
                                *height,
                                i as f64,
                                1.0,
                                mask,
                            ]);
                        }
                        Shape::Dimension { height, .. } => {
                            let px = height * scale;
                            let Some(l) = dimension_geom(s).and_then(|g| layout_dimension(&g))
                            else {
                                continue;
                            };
                            if px < min_px || px > max_px {
                                continue;
                            }
                            out.extend([
                                it.id,
                                LABEL_PIECE_DIMENSION,
                                l.text_at.x,
                                l.text_at.y,
                                l.rotation,
                                l.value,
                                i as f64,
                                *height,
                                masked(s),
                            ]);
                        }
                        _ => {}
                    }
                }
            }
            match &it.shape {
                Shape::Dimension { height, .. } => {
                    let px = height * scale;
                    let Some(l) = dimension_geom(&it.shape).and_then(|g| layout_dimension(&g))
                    else {
                        continue;
                    };
                    if px < min_px || px > max_px {
                        continue;
                    }
                    out.extend([
                        it.id,
                        LABEL_DIMENSION,
                        l.text_at.x,
                        l.text_at.y,
                        l.rotation,
                        l.value,
                        code(&DIMENSION_UNITS, l.unit),
                        code(&DIMENSION_PREFIXES, l.prefix),
                        masked(&it.shape),
                    ]);
                    continue;
                }
                Shape::Text {
                    height,
                    rotation,
                    mask,
                    ..
                } => {
                    let px = height * scale;
                    if px < min_px || px > max_px {
                        continue;
                    }
                    // A multi-line text, line by line (docs/adr/0182 §3); one along a curve,
                    // letter by letter (docs/adr/0196 §2.6).
                    if let Some(t) = TextPlace::of(&it.shape).filter(TextPlace::by_records) {
                        self.paragraph_labels(it.id, &t, *mask == Some(true), None, &mut out);
                        continue;
                    }
                    let (o, factor, mask) = self.text_label(&it.shape);
                    out.extend([
                        it.id, LABEL_TEXT, o.x, o.y, *rotation, factor, mask, 0.0, 0.0,
                    ]);
                    continue;
                }
                // Each cell's words where its layout puts them; under 5 px they are not drawn,
                // its lines are (docs/adr/0184 §2).
                Shape::Table {
                    height, rotation, ..
                } => {
                    let px = height * scale;
                    if (min_px..=max_px).contains(&px)
                        && let Some(t) = crate::geom::table::table_geom(&it.shape)
                    {
                        for c in t.layout(self.font).cells {
                            out.extend([
                                it.id,
                                LABEL_CELL,
                                c.at.x,
                                c.at.y,
                                *rotation,
                                c.row as f64,
                                c.col as f64,
                                if c.bold { 1.0 } else { 0.0 },
                                0.0,
                            ]);
                        }
                    }
                    continue;
                }
                // Its note, as a text's; under 5 px it is not drawn, its line and arrowhead are (docs/adr/0146 §5).
                Shape::Leader {
                    height, rotation, ..
                } => {
                    let px = height * scale;
                    if let Some((o, mask)) = self.note_label(&it.shape)
                        && (min_px..=max_px).contains(&px)
                    {
                        out.extend([
                            it.id,
                            LABEL_LEADER,
                            o.x,
                            o.y,
                            *rotation,
                            1.0,
                            mask,
                            0.0,
                            0.0,
                        ]);
                    }
                    continue;
                }
                _ => {}
            }
        }
        out
    }

    /// A leader's note (docs/adr/0146 §5): where its baseline starts and its
    /// mask's width (0 without a mask), in the drawing's typeface; none
    /// without a note.
    fn note_label(&self, s: &Shape) -> Option<(crate::vec2::Vec2, f64)> {
        let t = note_place(s)?;
        let masked = matches!(
            s,
            Shape::Leader {
                mask: Some(true),
                ..
            }
        );
        Some((
            t.origin(self.font),
            if masked { t.width(self.font) } else { 0.0 },
        ))
    }

    /// A multi-line text's records (docs/adr/0182 §3), in the drawing's typeface.
    fn paragraph_labels(
        &self,
        id: f64,
        t: &TextPlace<'_>,
        masked: bool,
        piece: Option<f64>,
        out: &mut Vec<f64>,
    ) {
        paragraph_records(t, self.font, masked, id, piece, out);
    }

    /// A text's label (docs/adr/0145): where its baseline starts, its width
    /// factor (1 without one) and its mask's width (0 without a mask), in the
    /// drawing's typeface.
    fn text_label(&self, s: &Shape) -> (crate::vec2::Vec2, f64, f64) {
        let Some(t) = TextPlace::of(s) else {
            return (crate::vec2::Vec2::new(0.0, 0.0), 1.0, 0.0);
        };
        let masked = matches!(
            s,
            Shape::Text {
                mask: Some(true),
                ..
            }
        );
        (
            t.origin(self.font),
            t.width_factor.unwrap_or(1.0),
            if masked { t.width(self.font) } else { 0.0 },
        )
    }

    /// Grips of these objects (`entityGrips`), known ids only, in the given
    /// order: `id, count, vertices`, then `x, y, segment` per grip, where
    /// `vertices` is a path's vertex count (0 otherwise) and `segment` the
    /// segment of a mid grip (−1 for other grips).
    pub fn grips(&self, ids: &[f64]) -> Vec<f64> {
        let mut out = Vec::new();
        for &id in ids {
            let Some(it) = self.get(id) else { continue };
            let grips = entity_grips(&it.shape);
            let vertices = match &it.shape {
                Shape::Polyline { pts, .. }
                | Shape::Polygon { pts, .. }
                | Shape::Leader { pts, .. } => pts.len(),
                _ => 0,
            };
            out.extend([id, grips.len() as f64, vertices as f64]);
            for (i, g) in grips.iter().enumerate() {
                let seg = mid_grip_segment(&it.shape, i).map_or(-1.0, |s| s as f64);
                out.extend([g.x, g.y, seg]);
            }
        }
        out
    }
}

/// A multi-line text's label records (docs/adr/0182 §3) in `font`: its mask's
/// box when it has one (`LABEL_PARAGRAPH_MASK`), then each line's where its
/// baseline starts (`LABEL_LINE`, or `LABEL_PIECE_LINE` for a block's piece
/// at `piece`). A text along a curve gives a mask and a line a letter
/// (docs/adr/0196 §2.6). The store's labels and the tools' previews (`textLines`).
pub fn paragraph_records(
    t: &TextPlace<'_>,
    font: crate::text::Font,
    masked: bool,
    id: f64,
    piece: Option<f64>,
    out: &mut Vec<f64>,
) {
    if let Some(a) = t.along(font) {
        let letters = a.letters();
        a.records(&letters, masked, id, piece, out);
        return;
    }
    let laid = t.layout(font);
    let o = t.origin(font);
    let r = (t.rotation * crate::jsmath::PI) / 180.0;
    let (u, v) = (
        crate::vec2::Vec2::new(crate::jsmath::cos(r), crate::jsmath::sin(r)),
        crate::vec2::Vec2::new(-crate::jsmath::sin(r), crate::jsmath::cos(r)),
    );
    if masked {
        let (h, m) = (t.height * 1.15, t.height * 0.1);
        let below = laid.lines.len().saturating_sub(1) as f64 * laid.pitch;
        let y0 = -h * 0.2 - m - below;
        let x0 = -m + y0 * t.lean;
        out.extend([
            id,
            LABEL_PARAGRAPH_MASK,
            o.x + u.x * x0 + v.x * y0,
            o.y + u.y * x0 + v.y * y0,
            t.rotation,
            laid.width + 2.0 * m,
            h * 1.2 + 2.0 * m + below,
            piece.unwrap_or(0.0),
            0.0,
        ]);
    }
    let (kind, c) = match piece {
        Some(place) => (LABEL_PIECE_LINE, place),
        None => (LABEL_LINE, t.width_factor.unwrap_or(1.0)),
    };
    for line in &laid.lines {
        let x = line.x - line.y * t.lean;
        out.extend([
            id,
            kind,
            o.x + u.x * x - v.x * line.y,
            o.y + u.y * x - v.y * line.y,
            t.rotation,
            t.height,
            c,
            line.start as f64,
            line.end as f64,
        ]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::placing::{LABEL_PLACED, LABEL_PLACED_LINE, ObjectLabels};

    fn objects(s: &mut Store, list: &[(f64, &str)]) {
        for (id, text) in list {
            s.set_object_labels(
                *id,
                ObjectLabels {
                    texts: vec![(0, (*text).to_owned())],
                    z: f64::NAN,
                },
            );
        }
    }

    /// The label records of a window (docs/adr/0212 §3.8): each label's frame, then its lines.
    fn frames(shown: &Shown) -> Vec<(f64, String)> {
        let mut out = Vec::new();
        for r in shown.records.chunks_exact(LABEL_STRIDE) {
            if r[1] == LABEL_PLACED_LINE {
                out.push((r[0], shown.texts[r[6] as usize].clone()));
            }
        }
        out
    }

    #[test]
    fn places_labels_by_the_layer_style_or_the_kinds_default() {
        let mut s = Store::new();
        s.put_json(
            r#"[{"id":1,"layerId":"parsel","label":"12","kind":"polygon","pts":[{"x":0,"y":0},{"x":10,"y":0},{"x":10,"y":10},{"x":0,"y":10}]},
                {"id":2,"layerId":"yol","label":"Cadde","kind":"polyline","pts":[{"x":0,"y":20},{"x":10,"y":20},{"x":20,"y":20}]},
                {"id":3,"layerId":"yol","label":"","kind":"line","a":{"x":0,"y":30},"b":{"x":9,"y":30}},
                {"id":4,"layerId":"yazi","kind":"text","p":{"x":60,"y":60},"text":"Not","height":2,"rotation":30},
                {"id":5,"layerId":"parsel","label":"7","kind":"polygon","pts":[{"x":100,"y":100},{"x":101,"y":100},{"x":101,"y":101}]}]"#,
        )
        .unwrap();
        s.set_label_layers_json(
            r#"[{"id":"parsel","rank":0,"label":{"placement":"corner","size":10,"minFeaturePx":5}},{"id":"yol","rank":1}]"#,
        )
        .unwrap();
        s.set_label_defaults_json(
            r#"{"polyline":{"placement":"along","size":10,"minScale":1.6},"line":{"placement":"along","size":10}}"#,
        )
        .unwrap();
        objects(&mut s, &[(1.0, "12"), (2.0, "Cadde"), (5.0, "7")]);
        let view = Bounds {
            min_x: -50.0,
            min_y: -50.0,
            max_x: 150.0,
            max_y: 150.0,
        };
        // At 3 px/m: the parcel by its corner, the street along its line, the text; the 1 m parcel (3 px) not.
        let shown = s.labels(&view, 3.0, None, PlaceOptions::default());
        assert_eq!(
            frames(&shown),
            [(1.0, "12".to_owned()), (2.0, "Cadde".to_owned())]
        );
        let text = [4.0, LABEL_TEXT, 60.0, 60.0, 30.0, 1.0, 0.0, 0.0, 0.0];
        assert_eq!(shown.records[..LABEL_STRIDE], text);
        // The corner: the line's left middle 8 px right of and 14 px under the box's top left.
        let corner = shown
            .records
            .chunks_exact(LABEL_STRIDE)
            .find(|r| r[0] == 1.0 && r[1] == LABEL_PLACED)
            .unwrap();
        let w = corner[5];
        assert!((corner[2] - (0.0 + (8.0 + w / 2.0) / 3.0)).abs() < 1e-9);
        assert!((corner[3] - (10.0 - 14.0 / 3.0)).abs() < 1e-9);
        // The street lies along its line: level, on it.
        let street = shown
            .records
            .chunks_exact(LABEL_STRIDE)
            .find(|r| r[0] == 2.0 && r[1] == LABEL_PLACED)
            .unwrap();
        assert_eq!((street[3], street[4]), (20.0, 0.0));
        // The text being edited is left out; at 1 px/m the street is below its scale range, the text below 5 px.
        assert!(frames(&s.labels(&view, 3.0, Some(4.0), PlaceOptions::default())).len() == 2);
        assert_eq!(
            frames(&s.labels(&view, 1.0, None, PlaceOptions::default())),
            [(1.0, "12".to_owned())]
        );
    }

    /// An object whose label a text writes shows none of its own (docs/adr/0175
    /// §4): the text is its label; the list is replaced whole and goes with `clear`.
    #[test]
    fn an_object_whose_label_a_text_writes_shows_none_of_its_own() {
        let mut s = Store::new();
        let objects_json = r#"[{"id":1,"layerId":"parsel","label":"12","kind":"polygon","pts":[{"x":0,"y":0},{"x":10,"y":0},{"x":10,"y":10},{"x":0,"y":10}]},
                {"id":2,"layerId":"parsel","label":"13","kind":"polygon","pts":[{"x":10,"y":0},{"x":20,"y":0},{"x":20,"y":10},{"x":10,"y":10}]},
                {"id":3,"layerId":"yazi","kind":"text","p":{"x":5,"y":5},"text":"12","height":2,"rotation":0}]"#;
        s.put_json(objects_json).unwrap();
        s.set_label_layers_json(
            r#"[{"id":"parsel","rank":0,"label":{"placement":"center","size":10}}]"#,
        )
        .unwrap();
        objects(&mut s, &[(1.0, "12"), (2.0, "13")]);
        let view = Bounds {
            min_x: -50.0,
            min_y: -50.0,
            max_x: 150.0,
            max_y: 150.0,
        };
        let at = |s: &Store| {
            frames(&s.labels(&view, 10.0, None, PlaceOptions::default()))
                .into_iter()
                .map(|(id, _)| id)
                .collect::<Vec<f64>>()
        };
        assert_eq!(at(&s), [1.0, 2.0]);
        s.set_text_labelled(&[1.0]);
        assert_eq!(at(&s), [2.0]);
        s.set_text_labelled(&[2.0]);
        assert_eq!(at(&s), [1.0]);
        s.clear();
        s.put_json(objects_json).unwrap();
        // The texts went with the objects; the layers' labelling stayed.
        assert!(at(&s).is_empty());
        objects(&mut s, &[(1.0, "12"), (2.0, "13")]);
        assert_eq!(at(&s), [1.0, 2.0]);
    }

    /// A multi-part polyline's label along it sits on its longest part
    /// (docs/adr/0175 §1): never between the end of one part and the start of the next.
    #[test]
    fn a_multi_part_objects_label_sits_on_its_main_part() {
        let mut s = Store::new();
        s.put_json(
            r#"[{"id":1,"layerId":"yol","label":"Dere","kind":"polyline","pts":[{"x":0,"y":0},{"x":5,"y":0}],
                 "parts":[{"pts":[{"x":0,"y":10},{"x":20,"y":10},{"x":40,"y":10},{"x":60,"y":10}]}]}]"#,
        )
        .unwrap();
        s.set_label_layers_json(
            r#"[{"id":"yol","rank":0,"label":{"placement":"along","size":10}}]"#,
        )
        .unwrap();
        objects(&mut s, &[(1.0, "Dere")]);
        let view = Bounds {
            min_x: -50.0,
            min_y: -50.0,
            max_x: 150.0,
            max_y: 150.0,
        };
        let shown = s.labels(&view, 3.0, None, PlaceOptions::default());
        let frame = shown
            .records
            .chunks_exact(LABEL_STRIDE)
            .find(|r| r[1] == LABEL_PLACED)
            .unwrap();
        // On the long part's middle, level.
        assert!(
            (frame[2] - 30.0).abs() < 1e-9 && frame[3] == 10.0 && frame[4] == 0.0,
            "{frame:?}"
        );
    }

    /// Etiketleri yazıya çevir through the store (docs/adr/0212 §4): the
    /// asked objects' labels as the engine places their window, each a text
    /// where its line is; a text's `item` is its object's place among the
    /// asked; a text object writes no label of its own.
    #[test]
    fn labels_become_texts_where_the_engine_places_them() {
        let mut s = Store::new();
        s.set_font(crate::text::Font::from_id("arimo"));
        s.put_json(
            r#"[{"id":1,"layerId":"parsel","label":"12","kind":"polygon","pts":[{"x":0,"y":0},{"x":30,"y":0},{"x":30,"y":20},{"x":0,"y":20}]},
                {"id":2,"layerId":"yazi","kind":"text","p":{"x":1,"y":1},"text":"Not","height":2,"rotation":0},
                {"id":3,"layerId":"nokta","label":"P7","kind":"point","p":{"x":100,"y":50}}]"#,
        )
        .unwrap();
        s.set_label_layers_json(
            r#"[{"id":"parsel","rank":1,"label":{"placement":"center","size":10,"template":"Parsel {label}"}},{"id":"nokta","rank":0,"point":6,"label":{"placement":"beside","size":10}}]"#,
        )
        .unwrap();
        objects(&mut s, &[(1.0, "Parsel 12"), (3.0, "P7")]);
        let out = s.label_texts(&[3.0, 2.0, 99.0, 1.0], 1000.0, false);
        let k = 96.0 / 0.0254 / 1000.0;
        assert_eq!(out.texts.len(), 2, "{out:?}");
        let (point, parcel) = (&out.texts[0], &out.texts[1]);
        assert_eq!((point.item, point.text.as_str()), (0, "P7"));
        // Around the point, its first place: top right of its 3 px symbol and 2 px.
        assert!(point.p.x > 100.0 && point.p.y > 50.0, "{point:?}");
        assert_eq!((parcel.item, parcel.text.as_str()), (3, "Parsel 12"));
        assert!(
            (parcel.p.x - 15.0).abs() < 1e-9 && (parcel.p.y - 10.0).abs() < 0.5,
            "{parcel:?}"
        );
        assert!((parcel.height - 10.0 / k).abs() < 1e-12);
        assert_eq!((out.out_of_scale, out.small, out.overlapping), (0, 0, 0));
    }

    /// A text's label starts where its alignment puts its baseline, with
    /// its width factor and its mask's width; it is picked where it is drawn
    /// (docs/adr/0145). By hand, at 90°: along the text is north, up from it
    /// west, so a top-right text's baseline starts its height east and its
    /// width south of its point.
    #[test]
    fn a_texts_label_starts_where_its_alignment_puts_it() {
        let mut s = Store::new();
        s.set_font(crate::text::Font::from_id("courier-prime"));
        s.put_json(
            r#"[{"id":7,"layerId":"yazi","kind":"text","p":{"x":10,"y":20},"text":"WW","height":2,"rotation":90,"align":"topRight","widthFactor":0.5,"mask":true}]"#,
        )
        .unwrap();
        let w =
            crate::text::width_em("WW", crate::text::Font::from_id("courier-prime")) * 2.0 * 0.5;
        let out = s.texts_only(
            &Bounds {
                min_x: 0.0,
                min_y: 0.0,
                max_x: 40.0,
                max_y: 40.0,
            },
            20.0,
            None,
        );
        assert_eq!(out.len(), LABEL_STRIDE);
        assert_eq!(out[..2], [7.0, LABEL_TEXT]);
        assert!(
            (out[2] - 12.0).abs() < 1e-12 && (out[3] - (20.0 - w)).abs() < 1e-12,
            "{out:?}"
        );
        assert_eq!(out[4..], [90.0, 0.5, w, 0.0, 0.0]);
        // Its body: 9.7 … 12.46 east, 20 − w … 20 north; where it stood unaligned is empty.
        assert_eq!(s.hit(crate::vec2::Vec2::new(11.0, 19.9), 0.01), Some(7.0));
        assert_eq!(s.hit(crate::vec2::Vec2::new(8.0, 20.4), 0.01), None);
        // A plain text's label is its point, width factor 1, no mask.
        s.put_json(r#"[{"id":8,"layerId":"yazi","kind":"text","p":{"x":30,"y":30},"text":"A","height":2,"rotation":0}]"#)
            .unwrap();
        let out = s.texts_only(
            &Bounds {
                min_x: 25.0,
                min_y: 25.0,
                max_x: 40.0,
                max_y: 40.0,
            },
            20.0,
            None,
        );
        assert_eq!(out, [8.0, LABEL_TEXT, 30.0, 30.0, 0.0, 1.0, 0.0, 0.0, 0.0]);
    }

    /// A leader's note (docs/adr/0146 §2, §5): half its height under the
    /// landing's level, from h/2 past the landing's end; to the left, its
    /// width back from there. By hand, h = 2: the landing 4 m, the note 1 m
    /// on; to the right from (4, 3) its baseline starts at (9, 2), to the
    /// left from (26, 3) at (21 − w, 2).
    #[test]
    fn a_leaders_note_is_labelled_past_its_landing() {
        let mut s = Store::new();
        let font = crate::text::Font::from_id("courier-prime");
        s.set_font(font);
        s.put_json(
            r#"[{"id":3,"layerId":"k","kind":"leader","pts":[{"x":0,"y":0},{"x":4,"y":3}],"text":"Not","height":2,"rotation":0,"mask":true},
                {"id":4,"layerId":"k","kind":"leader","pts":[{"x":30,"y":0},{"x":26,"y":3}],"text":"Not","height":2,"rotation":0,"label":"K-1"},
                {"id":5,"layerId":"k","kind":"leader","pts":[{"x":40,"y":0},{"x":44,"y":3}],"height":2,"rotation":0}]"#,
        )
        .unwrap();
        let w = crate::text::width_em("Not", font) * 2.0;
        let view = Bounds {
            min_x: -10.0,
            min_y: -10.0,
            max_x: 60.0,
            max_y: 20.0,
        };
        // Masked: its width; its etiket is not drawn, a text's is not either; without a note nothing.
        let right = [3.0, LABEL_LEADER, 9.0, 2.0, 0.0, 1.0, w, 0.0, 0.0];
        let left = [4.0, LABEL_LEADER, 21.0 - w, 2.0, 0.0, 1.0, 0.0, 0.0, 0.0];
        assert_eq!(s.texts_only(&view, 3.0, None), [right, left].concat());
        // Under 5 px its note is not drawn.
        assert!(s.texts_only(&view, 2.0, None).is_empty());
        // It is picked on its note, on its landing and on its line; not past the note.
        let at = |x: f64, y: f64| s.hit(crate::vec2::Vec2::new(x, y), 0.01);
        assert_eq!(at(9.5, 2.5), Some(3.0));
        assert_eq!(at(6.0, 3.0), Some(3.0));
        assert_eq!(at(2.0, 1.5), Some(3.0));
        assert_eq!(at(9.0 + w + 1.0, 2.5), None);
    }

    #[test]
    fn lists_grips_with_their_mid_segments() {
        let mut s = Store::new();
        s.put_json(r#"[{"id":9,"layerId":"a","kind":"polyline","pts":[{"x":0,"y":0},{"x":10,"y":0},{"x":10,"y":10}]}]"#)
            .unwrap();
        let g = s.grips(&[9.0, 99.0]);
        assert_eq!(&g[..3], &[9.0, 5.0, 3.0]);
        assert_eq!(
            &g[3..],
            &[
                0.0, 0.0, -1.0, 10.0, 0.0, -1.0, 10.0, 10.0, -1.0, 5.0, 0.0, 0.0, 10.0, 5.0, 1.0
            ]
        );
    }
}
