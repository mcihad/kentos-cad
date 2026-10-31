//! The drawing's text as the view shows it (docs/adr/0205 §5, 0626 §6): Kaybolmasın
//! (a text smaller on screen than `LEGIBLE_PX` is drawn that high, about its
//! own anchor), Gerçek boy (as it is, small and large alike: the overlays draw
//! one under a few pixels as a faint band; the paper's rule, [`LabelSize::Print`])
//! or Ekranda sabit (every text as high as it is on paper at 96 dpi). A grown text keeps apart
//! from the others and from the texts drawn as they are: one that would
//! cover another is left out of this view (8 px cells, the layer labels'
//! rule). A table's cells, a text along a curve and a tag's words stay as
//! they are, under the true rule. The overlays draw a grown record through a camera zoomed by
//! its factor about its anchor (the web's `drawLabels`, the desktop's
//! labels.rs); hit tests and the sheet keep the true sizes.

use crate::entity::{Shape, TextPlace};
use crate::geom::leader;
use crate::geometry::Bounds;
use crate::jsmath::{PI, cos, js_max, js_min, sin};
use crate::store::Store;
use crate::store::label_memo::{Part, TableKept};
use crate::store::labels::{
    LABEL_CELL, LABEL_PARAGRAPH_MASK, LABEL_PIECE_DIMENSION, LABEL_PIECE_LEADER, LABEL_PIECE_LINE,
    LABEL_PIECE_TEXT, LABEL_STRIDE,
};
use crate::store::placing::{PlaceOptions, Shown};
use crate::text::width_em;
use crate::vec2::Vec2;

/// How the drawing's text is sized on the screen (docs/adr/0205 §5).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LabelSize {
    /// As it is, under 5 px and over 240 px not at all: what a view can read (`Store::labels`).
    True,
    /// Paper output, and Gerçek boy on the screen (docs/adr/0626 §6): exact source size without
    /// size culling or enlargement.
    Print,
    /// Kaybolmasın: never smaller than `LEGIBLE_PX`.
    Legible,
    /// Ekranda sabit: as high as on paper at 96 dpi, the plot scale 1:`plot_scale`.
    Screen { plot_scale: f64 },
}

impl LabelSize {
    /// The setting's value (`graphics.annotationSize`): `legible`, `true`, `screen`; `type`, the
    /// project's type's own, is Kaybolmasın here ([`LabelSize::for_project`] knows the type). Gerçek boy
    /// is the paper's rule (docs/adr/0626 §6).
    pub fn of(name: &str, plot_scale: f64) -> LabelSize {
        match name {
            "true" => LabelSize::Print,
            "screen" => LabelSize::Screen { plot_scale },
            _ => LabelSize::Legible,
        }
    }

    /// [`LabelSize::of`] for a project of its type (docs/adr/0626 §6): `type` is Gerçek boy in a
    /// CAD project, Kaybolmasın in a CBS one.
    pub fn for_project(name: &str, plot_scale: f64, cad: bool) -> LabelSize {
        match name {
            "type" if cad => LabelSize::Print,
            _ => LabelSize::of(name, plot_scale),
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
        options: PlaceOptions,
    ) -> Shown {
        self.labels_shown_in(view, scale, editing, size, options, None)
    }

    /// [`Store::labels_shown`] with the layers a view of its own shows (docs/adr/0523 §2: a split area's
    /// view with a theme): `shown` says of each layer id whether its labels are placed, in place of the
    /// layer table's visibility; none is the table's.
    pub fn labels_shown_in(
        &self,
        view: &Bounds,
        scale: f64,
        editing: Option<f64>,
        size: LabelSize,
        options: PlaceOptions,
        shown: Option<&dyn Fn(&str) -> bool>,
    ) -> Shown {
        self.labels_shown_after(view, scale, editing, size, options, shown, None)
            .0
    }

    /// [`Store::labels_shown_in`] for a view that moved a little since `prior` (docs/adr/0533 §3), with where
    /// these labels went.
    #[allow(clippy::too_many_arguments)]
    pub fn labels_shown_after(
        &self,
        view: &Bounds,
        scale: f64,
        editing: Option<f64>,
        size: LabelSize,
        options: PlaceOptions,
        shown: Option<&dyn Fn(&str) -> bool>,
        prior: Option<&super::placing::Prior>,
    ) -> (Shown, super::placing::Prior) {
        self.labels_shown_parts(
            view, scale, editing, size, options, shown, prior, None, None,
        )
        .flat()
    }

    /// [`Store::labels_shown_after`] in parts (docs/adr/0786 §1): under true sizes and the paper's, where no text
    /// keeps apart from another, object by object (each its records, as grown, and the boxes its texts hold against
    /// the labels), taken from `memo` while the object and what it is read with stay; under a rule whose texts keep
    /// apart, one part of them all. A table's cells start inside `cells` (but its first, §2), when given. The label
    /// engine places the objects' labels after them, as before. Put together ([`ShownParts::flat`]) the parts are
    /// the records the whole read gives, to the bit.
    #[allow(clippy::too_many_arguments)]
    pub fn labels_shown_parts(
        &self,
        view: &Bounds,
        scale: f64,
        editing: Option<f64>,
        size: LabelSize,
        options: PlaceOptions,
        shown: Option<&dyn Fn(&str) -> bool>,
        prior: Option<&super::placing::Prior>,
        cells: Option<&Bounds>,
        memo: Option<&mut super::label_memo::LabelMemo>,
    ) -> ShownParts {
        let mask: Option<Vec<bool>> =
            shown.map(|f| self.layer_names().into_iter().map(f).collect());
        let max_px = match size {
            LabelSize::Screen { .. } | LabelSize::Print => f64::INFINITY,
            _ => TRUE_MAX_PX,
        };
        let min_px = if size == LabelSize::True {
            TRUE_MIN_PX
        } else {
            0.0
        };
        let px = (min_px, max_px);
        let parts: Vec<(f64, Part)> = if matches!(size, LabelSize::True | LabelSize::Print) {
            // Object by object: a text's group is its object's (`group_key`), and none keeps apart.
            let items = self.label_items(view, editing, mask.as_deref());
            let make = |it: &super::Item, old: Option<&TableKept>| {
                if let (Some(near), Some(t)) = (cells, crate::geom::table::table_geom(&it.shape)) {
                    return self.table_parts(it, &t, near, (scale, px, size), old);
                }
                let mut records = Vec::new();
                self.item_labels(it, scale, px, cells, &mut records);
                let groups = self.groups(&records, scale, size);
                (
                    blocks(part(&records, &groups, &vec![true; groups.len()])),
                    None,
                )
            };
            match memo {
                Some(m) => {
                    // The rule and, under true sizes, the scale (a table's or a curve's text out of 5…240 px).
                    let rule = u8::from(size == LabelSize::True);
                    m.begin(self.font, rule, px, (scale, size == LabelSize::True), cells);
                    let parts = items
                        .into_iter()
                        .flat_map(|it| {
                            m.parts(it, |old| make(it, old))
                                .into_iter()
                                .map(|p| (it.id, p))
                        })
                        .collect();
                    m.end();
                    parts
                }
                None => items
                    .into_iter()
                    .flat_map(|it| make(it, None).0.into_iter().map(|p| (it.id, p)))
                    .collect(),
            }
        } else {
            let records = self.labels_near(view, scale, editing, px, mask.as_deref(), cells);
            let groups = self.groups(&records, scale, size);
            let kept = keep_apart(&groups, view, scale);
            vec![(f64::NAN, part(&records, &groups, &kept))]
        };
        // The texts as shown keep the objects' labels off them (docs/adr/0212 §3.5).
        let fixed: Vec<Vec<Vec2>> = parts
            .iter()
            .flat_map(|(_, p)| p.outlines.iter().cloned())
            .collect();
        let (placed, prior) =
            self.place_labels_after_in(view, scale, &fixed, options, prior, mask.as_deref());
        ShownParts {
            parts,
            placed,
            prior,
        }
    }

    /// A table's parts by its blocks of rows (docs/adr/0786 §2), its cells near `near`, with what they were made of:
    /// a block whose words stay in a table that stays but for its words (`old`) is its part as it was, the other
    /// blocks are laid out alone, their records as the whole read gives them (each cell's place, its factor and the
    /// table's first cell as the point it grows about, the table's box held on its first part); else the whole read.
    fn table_parts(
        &self,
        it: &super::Item,
        t: &crate::geom::table::TableGeom<'_>,
        near: &Bounds,
        (scale, px, size): (f64, (f64, f64), LabelSize),
        old: Option<&TableKept>,
    ) -> (Vec<Part>, Option<TableKept>) {
        use std::hash::{Hash, Hasher};
        let hasher = || std::collections::hash_map::DefaultHasher::new();
        // What places a cell besides its words: the table's place, sizes, merges, aligns, heading, face and
        // styles, the typeface, and its first cell (whose start every cell grows about).
        let mut frame = hasher();
        (
            t.p.x.to_bits(),
            t.p.y.to_bits(),
            t.rotation.to_bits(),
            t.height.to_bits(),
        )
            .hash(&mut frame);
        t.rows
            .iter()
            .chain(t.columns)
            .for_each(|x| x.to_bits().hash(&mut frame));
        (t.rows.len(), t.header, format!("{:?}", self.font)).hash(&mut frame);
        format!("{:?}{:?}{:?}{:?}", t.merges, t.aligns, t.face, t.styles).hash(&mut frame);
        let first = t
            .cells
            .iter()
            .enumerate()
            .find_map(|(i, r)| r.iter().position(|w| !w.is_empty()).map(|j| (i, j, &r[j])));
        first.hash(&mut frame);
        let frame = frame.finish();
        let n = t.cells.len().div_ceil(BLOCK_ROWS);
        let prints: Vec<u64> = (0..n)
            .map(|b| {
                let mut h = hasher();
                for row in t.cells.iter().skip(b * BLOCK_ROWS).take(BLOCK_ROWS) {
                    row.hash(&mut h);
                }
                h.finish()
            })
            .collect();
        if let Some(old) = old.filter(|o| o.frame == frame && o.prints.len() == n) {
            let Some((_, head)) = old.parts.first() else {
                return (
                    Vec::new(),
                    Some(TableKept {
                        frame,
                        prints,
                        parts: Vec::new(),
                    }),
                );
            };
            let grow = [
                head.records[LABEL_STRIDE],
                head.records[LABEL_STRIDE + 1],
                head.records[LABEL_STRIDE + 2],
            ];
            let mut made: Vec<(usize, std::sync::Arc<[f64]>)> = Vec::new();
            for b in 0..n {
                if prints[b] == old.prints[b] {
                    if let Some((_, p)) = old.parts.iter().find(|(x, _)| *x == b) {
                        made.push((b, p.records.clone()));
                    }
                    continue;
                }
                let mut records = Vec::new();
                for c in t.cells_near_in(self.font, near, b * BLOCK_ROWS..(b + 1) * BLOCK_ROWS) {
                    records.extend([
                        it.id,
                        LABEL_CELL,
                        c.at.x,
                        c.at.y,
                        t.rotation,
                        c.row as f64,
                        c.col as f64,
                        if c.bold { 1.0 } else { 0.0 },
                        0.0,
                    ]);
                    records.extend(grow);
                }
                if !records.is_empty() {
                    made.push((b, records.into()));
                }
            }
            let parts: Vec<(usize, Part)> = made
                .into_iter()
                .enumerate()
                .map(|(i, (b, records))| {
                    let outlines = if i == 0 {
                        head.outlines.clone()
                    } else {
                        Vec::new().into()
                    };
                    (b, Part { records, outlines })
                })
                .collect();
            let out = parts.iter().map(|(_, p)| p.clone()).collect();
            return (
                out,
                Some(TableKept {
                    frame,
                    prints,
                    parts,
                }),
            );
        }
        let mut records = Vec::new();
        self.item_labels(it, scale, px, Some(near), &mut records);
        let groups = self.groups(&records, scale, size);
        let parts = blocks(part(&records, &groups, &vec![true; groups.len()]));
        let by_block = parts
            .iter()
            .map(|p| {
                (
                    p.records.get(5).map_or(0, |r| *r as usize / BLOCK_ROWS),
                    p.clone(),
                )
            })
            .collect();
        (
            parts,
            Some(TableKept {
                frame,
                prints,
                parts: by_block,
            }),
        )
    }

    /// The groups of `records` (one text's records each, `group_key`), those the rule shows.
    fn groups(&self, records: &[f64], scale: f64, size: LabelSize) -> Vec<Group> {
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
        groups
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
        // A table's cells and a curve's letters keep the true rule (docs/adr/0205 §5); a tag's words too, its frame
        // sized round them (docs/adr/0344 §1).
        let fixed = kind == LABEL_CELL
            || matches!(shape, Shape::Text { path: Some(_), .. })
            || matches!(item.shape, Shape::Tag { .. } | Shape::ViewMark { .. });
        if fixed {
            if size!=LabelSize::Print && !(TRUE_MIN_PX..=TRUE_MAX_PX).contains(&px) {
                return None;
            }
            return Some(Group {
                outline: corners(&item.bounds),
                ..label(true)
            });
        }
        let k = match size {
            LabelSize::True | LabelSize::Print => 1.0,
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

/// A view's text in parts (`Store::labels_shown_parts`, docs/adr/0786 §1): the objects' parts in the document's
/// order (each its object's id; one part of every object, id NaN, under a rule whose texts keep apart), then the
/// label engine's labels and where they went.
#[derive(Clone, Debug, Default)]
pub struct ShownParts {
    pub parts: Vec<(f64, Part)>,
    /// The label engine's records (`LABEL_STRIDE` numbers each) and texts.
    pub placed: Shown,
    pub prior: super::placing::Prior,
}

impl ShownParts {
    /// The parts put together: the records `Store::labels_shown_after` gives (`LABEL_SHOWN_STRIDE` numbers each,
    /// the label engine's after the objects' as grown 1 about their place), and where the labels went.
    pub fn flat(self) -> (Shown, super::placing::Prior) {
        let n: usize = self
            .parts
            .iter()
            .map(|(_, p)| p.records.len())
            .sum::<usize>()
            + self.placed.records.len() / LABEL_STRIDE * LABEL_SHOWN_STRIDE;
        let mut out = Vec::with_capacity(n);
        for (_, p) in &self.parts {
            out.extend_from_slice(&p.records);
        }
        for r in self.placed.records.chunks_exact(LABEL_STRIDE) {
            out.extend_from_slice(r);
            out.extend([1.0, r[2], r[3]]);
        }
        (
            Shown {
                records: out,
                texts: self.placed.texts,
            },
            self.prior,
        )
    }
}

/// The part of `records` whose `groups` are `kept`: each record followed by its group's factor and anchor, and the
/// boxes the groups hold against the labels, grown with them.
fn part(records: &[f64], groups: &[Group], kept: &[bool]) -> Part {
    let rec = |i: usize| &records[i * LABEL_STRIDE..(i + 1) * LABEL_STRIDE];
    let n: usize = groups
        .iter()
        .zip(kept)
        .filter(|(_, k)| **k)
        .map(|(g, _)| (g.to - g.from) * LABEL_SHOWN_STRIDE)
        .sum();
    let mut out = Vec::with_capacity(n);
    let mut outlines = Vec::new();
    for (g, keep) in groups.iter().zip(kept) {
        if !keep {
            continue;
        }
        for i in g.from..g.to {
            out.extend_from_slice(rec(i));
            out.extend([g.k, g.anchor.x, g.anchor.y]);
        }
        if !g.outline.is_empty() {
            outlines.push(
                g.outline
                    .iter()
                    .map(|p| {
                        Vec2::new(
                            g.anchor.x + (p.x - g.anchor.x) * g.k,
                            g.anchor.y + (p.y - g.anchor.y) * g.k,
                        )
                    })
                    .collect(),
            );
        }
    }
    Part {
        records: out.into(),
        outlines: outlines.into(),
    }
}

/// The rows of a table's cells a part holds (docs/adr/0786 §2): a long table's cells come in parts of so many rows,
/// a change of a few of them making those parts alone again.
pub const BLOCK_ROWS: usize = 64;

/// An object's part as parts of [`BLOCK_ROWS`] rows of a table's cells (the boxes it holds in the first); the part
/// itself for any other object's.
fn blocks(p: Part) -> Vec<Part> {
    let block = |r: &[f64]| (r[1] == LABEL_CELL).then(|| r[5] as usize / BLOCK_ROWS);
    let records: Vec<&[f64]> = p.records.chunks_exact(LABEL_SHOWN_STRIDE).collect();
    let first = records.first().map(|r| block(r));
    if records.iter().all(|r| block(r) == first.flatten()) {
        return vec![p];
    }
    let mut out: Vec<Part> = Vec::new();
    let mut from = 0;
    for i in 1..=records.len() {
        if i < records.len() && block(records[i]) == block(records[from]) {
            continue;
        }
        let numbers: Vec<f64> = records[from..i]
            .iter()
            .flat_map(|r| r.iter().copied())
            .collect();
        out.push(Part {
            records: numbers.into(),
            outlines: if out.is_empty() {
                p.outlines.clone()
            } else {
                Vec::new().into()
            },
        });
        from = i;
    }
    out
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
        s.labels_shown(&view(), scale, None, size, PlaceOptions::default())
            .records
            .chunks_exact(LABEL_SHOWN_STRIDE)
            .map(|r| (r[0], r[1], r[9], r[10], r[11]))
            .collect()
    }

    const ONE: &str = r#"[{"id":1,"layerId":"a","kind":"text","p":{"x":10,"y":20},"text":"Ada 101","height":2.5,"rotation":0}]"#;

    /// The view's text as it was read before it came in parts (docs/adr/0786 §1): every object's records at once,
    /// grouped, kept apart under a rule whose texts keep apart, then the label engine's.
    fn whole(s: &Store, view: &Bounds, scale: f64, size: LabelSize) -> Vec<u64> {
        let max_px = match size {
            LabelSize::Screen { .. } | LabelSize::Print => f64::INFINITY,
            _ => TRUE_MAX_PX,
        };
        let min_px = if size == LabelSize::True {
            TRUE_MIN_PX
        } else {
            0.0
        };
        let records = s.labels_with(view, scale, None, min_px, max_px, None);
        let groups = s.groups(&records, scale, size);
        let kept = if matches!(size, LabelSize::True | LabelSize::Print) {
            vec![true; groups.len()]
        } else {
            keep_apart(&groups, view, scale)
        };
        let p = part(&records, &groups, &kept);
        let fixed: Vec<Vec<Vec2>> = p.outlines.to_vec();
        let (placed, _) =
            s.place_labels_after_in(view, scale, &fixed, PlaceOptions::default(), None, None);
        let mut out: Vec<f64> = p.records.to_vec();
        for r in placed.records.chunks_exact(LABEL_STRIDE) {
            out.extend_from_slice(r);
            out.extend([1.0, r[2], r[3]]);
        }
        out.iter().map(|x| x.to_bits()).collect()
    }

    /// One random-ish object of the view: a text (one line, several, turned), a table or a labelled parcel.
    fn object(i: i32) -> String {
        let (x, y) = (
            f64::from(i % 6) * 30.0 - 90.0,
            f64::from(i / 6) * 30.0 - 70.0,
        );
        let id = i + 1;
        match i % 5 {
            0 => format!(
                r#"{{"id":{id},"layerId":"a","kind":"text","p":{{"x":{x},"y":{y}}},"text":"Oda {i}","height":{},"rotation":{}}}"#,
                0.2 + f64::from(i) * 0.1,
                i * 13
            ),
            1 => format!(
                r#"{{"id":{id},"layerId":"a","kind":"text","p":{{"x":{x},"y":{y}}},"text":"Salon\nMutfak {i}","height":1.5,"rotation":0}}"#
            ),
            2 => format!(
                r#"{{"id":{id},"layerId":"c","kind":"table","p":{{"x":{x},"y":{y}}},"rotation":{},"height":0.5,"rows":[1,1,1],"columns":[4,3],"cells":[["No","Alan"],["{i}","12,5"],["",""]],"header":true}}"#,
                i * 7
            ),
            3 => format!(
                r#"{{"id":{id},"layerId":"p","label":"{i}","kind":"polygon","pts":[{{"x":{x},"y":{y}}},{{"x":{},"y":{y}}},{{"x":{},"y":{}}},{{"x":{x},"y":{}}}]}}"#,
                x + 20.0,
                x + 20.0,
                y + 20.0,
                y + 20.0
            ),
            _ => format!(
                r#"{{"id":{id},"layerId":"a","kind":"text","p":{{"x":{x},"y":{y}}},"text":"Kat {i}","height":3,"rotation":90}}"#
            ),
        }
    }

    /// Object by object, taken from the memo or read afresh, the parts put together are the whole read's records,
    /// to the bit, under every rule, at scales that leave some texts out, in two views.
    #[test]
    fn the_parts_put_together_are_the_whole_read() {
        let objects: Vec<String> = (0..30).map(object).collect();
        let mut s = store(&format!("[{}]", objects.join(",")));
        s.set_label_layers_json(
            r#"[{"id":"p","rank":0,"label":{"placement":"center","size":10}}]"#,
        )
        .unwrap();
        for i in (3..30).step_by(5) {
            s.set_object_labels(
                f64::from(i + 1),
                crate::store::placing::ObjectLabels {
                    texts: vec![(0, format!("{i}"))],
                    z: f64::NAN,
                },
            );
        }
        let mut memo = crate::store::label_memo::LabelMemo::default();
        let small = Bounds {
            min_x: -40.0,
            min_y: -30.0,
            max_x: 20.0,
            max_y: 15.0,
        };
        let sizes = [
            LabelSize::Print,
            LabelSize::True,
            LabelSize::Legible,
            LabelSize::Screen { plot_scale: 100.0 },
        ];
        let mut placed = 0;
        for size in sizes {
            for scale in [0.5, 2.0, 4.0, 20.0, 200.0] {
                for view in [view(), small] {
                    let want = whole(&s, &view, scale, size);
                    placed += want.len();
                    for memo in [None, Some(&mut memo)] {
                        let (got, _) = s
                            .labels_shown_parts(
                                &view,
                                scale,
                                None,
                                size,
                                PlaceOptions::default(),
                                None,
                                None,
                                None,
                                memo,
                            )
                            .flat();
                        let got: Vec<u64> = got.records.iter().map(|x| x.to_bits()).collect();
                        assert_eq!(got, want, "{size:?} at {scale}");
                    }
                }
            }
        }
        assert!(placed > 1000, "{placed}");
    }

    #[test]
    fn a_text_too_small_is_grown_to_the_legible_size_about_its_point() {
        let s = store(ONE);
        // 2.5 m at 1 px/m is 2.5 px: what a view reads leaves it out, Gerçek boy keeps it as it is,
        // Kaybolmasın grows it 8 / 2.5 times about (10, 20).
        assert!(shown(&s, 1.0, LabelSize::True).is_empty());
        assert_eq!(
            shown(&s, 1.0, LabelSize::of("true", 1000.0)),
            [(1.0, LABEL_TEXT, 1.0, 10.0, 20.0)]
        );
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

    /// The setting's values (docs/adr/0626 §6): Türe göre is Gerçek boy in a CAD project, Kaybolmasın in a CBS one.
    #[test]
    fn the_settings_values_are_their_rules() {
        assert_eq!(LabelSize::for_project("type", 1000.0, true), LabelSize::Print);
        assert_eq!(LabelSize::for_project("type", 1000.0, false), LabelSize::Legible);
        assert_eq!(LabelSize::for_project("true", 1000.0, false), LabelSize::Print);
        assert_eq!(LabelSize::for_project("legible", 1000.0, true), LabelSize::Legible);
        assert_eq!(
            LabelSize::for_project("screen", 500.0, true),
            LabelSize::Screen { plot_scale: 500.0 }
        );
    }

    #[test]
    fn print_keeps_small_and_large_cad_text_at_original_physical_size() {
        let s = store(ONE);
        for scale in [0.01, 1., 200., 2000.] {
            assert_eq!(
                shown(&s, scale, LabelSize::Print),
                [(1., LABEL_TEXT, 1., 10., 20.)]
            );
        }
        assert!(shown(&s, 1., LabelSize::True).is_empty());
        assert!(shown(&s, 200., LabelSize::True).is_empty());
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
