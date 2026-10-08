//! The drawing's text as the view shows it (docs/adr/0205 §5): Kaybolmasın
//! (a text smaller on screen than `LEGIBLE_PX` is drawn that high, about its
//! own anchor), Gerçek boy (as it is; under 5 px not at all) or Ekranda sabit
//! (every text as high as it is on paper at 96 dpi). A grown text keeps apart
//! from the others and from the texts drawn as they are: one that would
//! cover another is left out of this view (8 px cells, the layer labels'
//! rule). A table's cells and a text along a curve stay as they are, under
//! the true rule. The overlays draw a grown record through a camera zoomed by
//! its factor about its anchor (the web's `drawLabels`, the desktop's
//! labels.rs); hit tests and the sheet keep the true sizes.

use crate::entity::{Shape, TextPlace};
use crate::geom::leader;
use crate::geometry::Bounds;
use crate::jsmath::{PI, cos, js_max, js_min, sin};
use crate::store::Store;
use crate::store::labels::{
    LABEL_ALONG, LABEL_BESIDE, LABEL_CELL, LABEL_CENTER, LABEL_CORNER, LABEL_PARAGRAPH_MASK,
    LABEL_PIECE_DIMENSION, LABEL_PIECE_LEADER, LABEL_PIECE_LINE, LABEL_PIECE_TEXT, LABEL_STRIDE,
};
use crate::text::width_em;
use crate::vec2::Vec2;

/// How the drawing's text is sized on the screen (docs/adr/0205 §5).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LabelSize {
    /// Gerçek boy: as it is; under 5 px and over 240 px it is not drawn.
    True,
    /// Kaybolmasın: never smaller than `LEGIBLE_PX`.
    Legible,
    /// Ekranda sabit: as high as on paper at 96 dpi, the plot scale 1:`plot_scale`.
    Screen { plot_scale: f64 },
}

impl LabelSize {
    /// The setting's value (`graphics.annotationSize`): `legible`, `true`, `screen`.
    pub fn of(name: &str, plot_scale: f64) -> LabelSize {
        match name {
            "true" => LabelSize::True,
            "screen" => LabelSize::Screen { plot_scale },
            _ => LabelSize::Legible,
        }
    }
}

/// The least a text is drawn high while it may not vanish, logical pixels.
pub const LEGIBLE_PX: f64 = 8.0;
/// Numbers per record of `labels_shown`: a label record, its factor and the
/// point it grows about.
pub const LABEL_SHOWN_STRIDE: usize = LABEL_STRIDE + 3;
/// The true rule's sizes on screen, logical pixels.
pub(crate) const TRUE_MIN_PX: f64 = 5.0;
pub(crate) const TRUE_MAX_PX: f64 = 240.0;
/// The cells grown texts keep apart in, logical pixels (the overlays' `LabelRoom`).
const CELL: f64 = 8.0;
/// Screen pixels a paper millimetre is at 96 dpi.
const PX_PER_MM: f64 = 96.0 / 25.4;

/// The records of one text: their place in the list, their factor and
/// anchor, the box they take on screen at their true size (world), and
/// whether they keep apart at all (a layer label is the overlays').
struct Group {
    from: usize,
    to: usize,
    k: f64,
    anchor: Vec2,
    outline: Vec<Vec2>,
    apart: bool,
}

impl Store {
    /// `labels` as the view shows them under `size` (docs/adr/0205 §5): each
    /// record followed by its factor (1: as it is) and the point it grows
    /// about (x, y), `LABEL_SHOWN_STRIDE` numbers a record; the texts a grown
    /// one would cover left out.
    pub fn labels_shown(
        &self,
        view: &Bounds,
        scale: f64,
        editing: Option<f64>,
        size: LabelSize,
    ) -> Vec<f64> {
        let max_px = match size {
            LabelSize::Screen { .. } => f64::INFINITY,
            _ => TRUE_MAX_PX,
        };
        let min_px = if size == LabelSize::True {
            TRUE_MIN_PX
        } else {
            0.0
        };
        let records = self.labels_with(view, scale, editing, min_px, max_px);
        let n = records.len() / LABEL_STRIDE;
        let rec = |i: usize| &records[i * LABEL_STRIDE..(i + 1) * LABEL_STRIDE];
        let mut groups = Vec::new();
        let mut i = 0;
        while i < n {
            let key = self.group_key(rec(i));
            let mut j = i + 1;
            while j < n && self.group_key(rec(j)) == key {
                j += 1;
            }
            if let Some(g) = self.group(rec(i), i, j, scale, size) {
                groups.push(g);
            }
            i = j;
        }
        let kept = if size == LabelSize::True {
            vec![true; groups.len()]
        } else {
            keep_apart(&groups, view, scale)
        };
        let mut out = Vec::with_capacity(records.len() / LABEL_STRIDE * LABEL_SHOWN_STRIDE);
        for (g, keep) in groups.iter().zip(kept) {
            if !keep {
                continue;
            }
            for i in g.from..g.to {
                out.extend_from_slice(rec(i));
                out.extend([g.k, g.anchor.x, g.anchor.y]);
            }
        }
        out
    }

    /// Which text a record belongs to: its object, and a block's piece (−1
    /// the object's own, −2 a layer label).
    fn group_key(&self, r: &[f64]) -> (u64, i64) {
        let piece = match r[1] {
            k if k == LABEL_PIECE_TEXT
                || k == LABEL_PIECE_DIMENSION
                || k == LABEL_PIECE_LEADER
                || k == LABEL_PIECE_LINE =>
            {
                r[6] as i64
            }
            k if k == LABEL_PARAGRAPH_MASK => match self.get(r[0]) {
                Some(it) if it.expanded.is_some() => r[7] as i64,
                _ => -1,
            },
            k if k == LABEL_CENTER
                || k == LABEL_CORNER
                || k == LABEL_BESIDE
                || k == LABEL_ALONG =>
            {
                -2
            }
            _ => -1,
        };
        (r[0].to_bits(), piece)
    }

    /// One text's group from its first record `r` (records `from..to`):
    /// none for a table or a curve's text the true rule leaves out.
    fn group(
        &self,
        r: &[f64],
        from: usize,
        to: usize,
        scale: f64,
        size: LabelSize,
    ) -> Option<Group> {
        let label = |apart| Group {
            from,
            to,
            k: 1.0,
            anchor: Vec2::new(r[2], r[3]),
            outline: Vec::new(),
            apart,
        };
        let kind = r[1];
        if kind == LABEL_CENTER
            || kind == LABEL_CORNER
            || kind == LABEL_BESIDE
            || kind == LABEL_ALONG
        {
            return Some(label(false));
        }
        let item = self.get(r[0])?;
        let piece = (kind == LABEL_PIECE_TEXT
            || kind == LABEL_PIECE_DIMENSION
            || kind == LABEL_PIECE_LEADER
            || kind == LABEL_PIECE_LINE
            || (kind == LABEL_PARAGRAPH_MASK && item.expanded.is_some()))
        .then(|| {
            if kind == LABEL_PARAGRAPH_MASK {
                r[7]
            } else {
                r[6]
            }
        });
        let shape = match (piece, &item.expanded) {
            (Some(i), Some(x)) => x.shapes.get(i as usize)?,
            _ => &item.shape,
        };
        let height = match shape {
            Shape::Text { height, .. }
            | Shape::Leader { height, .. }
            | Shape::Dimension { height, .. }
            | Shape::Table { height, .. } => *height,
            _ => return Some(label(true)),
        };
        let px = height * scale;
        // A table's cells and a curve's letters keep the true rule (docs/adr/0205 §5).
        let fixed = kind == LABEL_CELL || matches!(shape, Shape::Text { path: Some(_), .. });
        if fixed {
            if !(TRUE_MIN_PX..=TRUE_MAX_PX).contains(&px) {
                return None;
            }
            return Some(Group {
                outline: corners(&item.bounds),
                ..label(true)
            });
        }
        let k = match size {
            LabelSize::True => 1.0,
            LabelSize::Legible if px > 0.0 => js_max(LEGIBLE_PX / px, 1.0),
            LabelSize::Screen { plot_scale } if plot_scale > 0.0 && scale > 0.0 => {
                1000.0 / plot_scale * PX_PER_MM / scale
            }
            _ => 1.0,
        };
        let (anchor, outline) = match shape {
            Shape::Text { p, .. } => (
                *p,
                TextPlace::of(shape)
                    .map(|t| t.outline(self.font))
                    .unwrap_or_default(),
            ),
            Shape::Leader { .. } => {
                let end = leader::layout_of(shape)
                    .and_then(|l| l.landing.map(|[_, end]| end))
                    .unwrap_or(Vec2::new(r[2], r[3]));
                let note = leader::note_place(shape)
                    .map(|t| t.outline(self.font))
                    .unwrap_or_default();
                (end, note)
            }
            Shape::Dimension { .. } => {
                let at = Vec2::new(r[2], r[3]);
                (at, value_box(r, height, self.font))
            }
            _ => (Vec2::new(r[2], r[3]), corners(&item.bounds)),
        };
        Some(Group {
            from,
            to,
            k,
            anchor,
            outline,
            apart: true,
        })
    }
}

fn corners(b: &Bounds) -> Vec<Vec2> {
    vec![
        Vec2::new(b.min_x, b.min_y),
        Vec2::new(b.max_x, b.min_y),
        Vec2::new(b.max_x, b.max_y),
        Vec2::new(b.min_x, b.max_y),
    ]
}

/// A dimension value's box about its baseline's middle (x, y; turned by
/// its angle): its digits with two decimals and a prefix, wide as the
/// drawing's typeface writes them.
fn value_box(r: &[f64], height: f64, font: crate::text::Font) -> Vec<Vec2> {
    let words = format!("{:.2}", r[5].abs());
    let w = (width_em(&words, font) + 0.5) * height;
    let a = r[4] * PI / 180.0;
    let (u, v) = (Vec2::new(cos(a), sin(a)), Vec2::new(-sin(a), cos(a)));
    let at = |x: f64, y: f64| Vec2::new(r[2] + u.x * x + v.x * y, r[3] + u.y * x + v.y * y);
    vec![
        at(-w / 2.0, -0.25 * height),
        at(w / 2.0, -0.25 * height),
        at(w / 2.0, 1.15 * height),
        at(-w / 2.0, 1.15 * height),
    ]
}

/// Which groups stay: those drawn as they are, always (they take their
/// cells); then the grown ones in order, each that finds its cells free.
fn keep_apart(groups: &[Group], view: &Bounds, scale: f64) -> Vec<bool> {
    let cols =
        (js_max(((view.max_x - view.min_x) * scale / CELL).ceil(), 1.0) as usize).min(1 << 14);
    let rows =
        (js_max(((view.max_y - view.min_y) * scale / CELL).ceil(), 1.0) as usize).min(1 << 14);
    let mut taken = vec![false; cols * rows];
    // A group's cells: its outline grown about its anchor, on the view's cells.
    let cells = |g: &Group| -> Option<(usize, usize, usize, usize)> {
        let mut b = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for p in &g.outline {
            let q = Vec2::new(
                g.anchor.x + (p.x - g.anchor.x) * g.k,
                g.anchor.y + (p.y - g.anchor.y) * g.k,
            );
            let (x, y) = ((q.x - view.min_x) * scale, (view.max_y - q.y) * scale);
            b = [
                js_min(b[0], x),
                js_min(b[1], y),
                js_max(b[2], x),
                js_max(b[3], y),
            ];
        }
        if !b.iter().all(|v| v.is_finite()) {
            return None;
        }
        let c0 = js_max((b[0] / CELL).floor(), 0.0) as usize;
        let r0 = js_max((b[1] / CELL).floor(), 0.0) as usize;
        let c1 = (js_max((b[2] / CELL).floor(), 0.0) as usize).min(cols - 1);
        let r1 = (js_max((b[3] / CELL).floor(), 0.0) as usize).min(rows - 1);
        // Wholly off the view: nothing to keep apart from.
        (b[2] >= 0.0 && b[3] >= 0.0 && c0 <= c1 && r0 <= r1).then_some((c0, r0, c1, r1))
    };
    let mut keep = vec![true; groups.len()];
    for g in groups.iter().filter(|g| g.apart && g.k == 1.0) {
        if let Some((c0, r0, c1, r1)) = cells(g) {
            for r in r0..=r1 {
                taken[r * cols + c0..=r * cols + c1].fill(true);
            }
        }
    }
    for (i, g) in groups.iter().enumerate() {
        if !g.apart || g.k == 1.0 {
            continue;
        }
        let Some((c0, r0, c1, r1)) = cells(g) else {
            continue;
        };
        if (r0..=r1).any(|r| taken[r * cols + c0..=r * cols + c1].iter().any(|t| *t)) {
            keep[i] = false;
            continue;
        }
        for r in r0..=r1 {
            taken[r * cols + c0..=r * cols + c1].fill(true);
        }
    }
    keep
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::labels::{LABEL_CELL, LABEL_TEXT};

    fn store(json: &str) -> Store {
        let mut s = Store::new();
        s.put_json(json).expect("the objects go in");
        s
    }

    fn view() -> Bounds {
        Bounds {
            min_x: -100.0,
            min_y: -100.0,
            max_x: 100.0,
            max_y: 100.0,
        }
    }

    /// The records as (id, kind, k, anchor x, anchor y).
    fn shown(s: &Store, scale: f64, size: LabelSize) -> Vec<(f64, f64, f64, f64, f64)> {
        s.labels_shown(&view(), scale, None, size)
            .chunks_exact(LABEL_SHOWN_STRIDE)
            .map(|r| (r[0], r[1], r[9], r[10], r[11]))
            .collect()
    }

    const ONE: &str = r#"[{"id":1,"layerId":"a","kind":"text","p":{"x":10,"y":20},"text":"Ada 101","height":2.5,"rotation":0}]"#;

    #[test]
    fn a_text_too_small_is_grown_to_the_legible_size_about_its_point() {
        let s = store(ONE);
        // 2.5 m at 1 px/m is 2.5 px: Gerçek boy leaves it out, Kaybolmasın grows it 8 / 2.5 times about (10, 20).
        assert!(shown(&s, 1.0, LabelSize::True).is_empty());
        assert_eq!(
            shown(&s, 1.0, LabelSize::Legible),
            [(1.0, LABEL_TEXT, 3.2, 10.0, 20.0)]
        );
        // At 4 px/m it is 10 px: as it is.
        assert_eq!(
            shown(&s, 4.0, LabelSize::Legible),
            [(1.0, LABEL_TEXT, 1.0, 10.0, 20.0)]
        );
        assert_eq!(
            shown(&s, 4.0, LabelSize::True),
            [(1.0, LABEL_TEXT, 1.0, 10.0, 20.0)]
        );
    }

    #[test]
    fn on_screen_every_text_is_as_high_as_on_paper() {
        let s = store(ONE);
        // 2.5 m at 1:1000 is 2.5 mm on paper, 9.45 px at 96 dpi; at 2 px/m it is 5 px, grown 1.89 times.
        let got = shown(&s, 2.0, LabelSize::Screen { plot_scale: 1000.0 });
        assert_eq!(got.len(), 1);
        assert!(
            (got[0].2 - 1000.0 / 1000.0 * PX_PER_MM / 2.0).abs() < 1e-12,
            "{got:?}"
        );
        // Over 240 px it stays in view, shrunk.
        let got = shown(&s, 200.0, LabelSize::Screen { plot_scale: 1000.0 });
        assert_eq!(got.len(), 1);
        assert!(got[0].2 < 1.0);
    }

    #[test]
    fn a_grown_text_that_would_cover_another_is_left_out() {
        // Two small texts a metre apart: grown, the second covers the first and goes.
        let s = store(
            r#"[{"id":1,"layerId":"a","kind":"text","p":{"x":0,"y":0},"text":"Ada 101","height":0.5,"rotation":0},
                {"id":2,"layerId":"a","kind":"text","p":{"x":1,"y":0},"text":"Ada 102","height":0.5,"rotation":0}]"#,
        );
        let got: Vec<f64> = shown(&s, 2.0, LabelSize::Legible)
            .iter()
            .map(|r| r.0)
            .collect();
        assert_eq!(got, [1.0]);
        // A text drawn as it is keeps its place: the small one beside it goes.
        let s = store(
            r#"[{"id":1,"layerId":"a","kind":"text","p":{"x":2,"y":0},"text":"Küçük","height":0.5,"rotation":0},
                {"id":2,"layerId":"a","kind":"text","p":{"x":0,"y":0},"text":"Büyük yazı","height":10,"rotation":0}]"#,
        );
        let got: Vec<f64> = shown(&s, 2.0, LabelSize::Legible)
            .iter()
            .map(|r| r.0)
            .collect();
        assert_eq!(got, [2.0]);
        // Far apart both stay.
        let s = store(
            r#"[{"id":1,"layerId":"a","kind":"text","p":{"x":-80,"y":0},"text":"Ada 101","height":0.5,"rotation":0},
                {"id":2,"layerId":"a","kind":"text","p":{"x":60,"y":0},"text":"Ada 102","height":0.5,"rotation":0}]"#,
        );
        assert_eq!(shown(&s, 2.0, LabelSize::Legible).len(), 2);
    }

    #[test]
    fn a_tables_cells_keep_the_true_rule() {
        let table = r#"[{"id":1,"layerId":"a","kind":"table","p":{"x":0,"y":10},"rotation":0,"height":1,"rows":[2],"columns":[6],"cells":[["Ad"]]}]"#;
        let s = store(table);
        // 1 m at 2 px/m is 2 px: no cell, whatever the size.
        assert!(shown(&s, 2.0, LabelSize::Legible).is_empty());
        // At 10 px/m it is 10 px: as it is.
        let got = shown(&s, 10.0, LabelSize::Legible);
        assert_eq!(
            got.iter().map(|r| (r.0, r.1, r.2)).collect::<Vec<_>>(),
            [(1.0, LABEL_CELL, 1.0)]
        );
    }
}
