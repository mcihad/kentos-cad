//! A view's text kept object by object (docs/adr/0786 §1). Under true sizes (and the paper's) no text keeps apart
//! from another, so what a view shows of one object (its records, how each grows, the boxes its texts hold against
//! the labels) is its shape's, its block's pieces' and what they are read with (the drawing's typeface, the sizes on
//! screen a text is drawn at, a table's box of cells), nothing of the other objects. While those stay, the object's
//! part is taken again as it was; an object put again (a change, an undo) has a new stamp and is read again. The
//! memo is its holder's (the desktop's view of the drawing, `Spatial`): the store stays shareable and unchanged.

use std::collections::HashMap;
use std::sync::Arc;

use super::{Expanded, Item};
use crate::entity::Shape;
use crate::geometry::Bounds;
use crate::text::Font;
use crate::vec2::Vec2;

/// One object's part of a view's text (`Store::labels_shown_parts`): its records, each followed by its factor and
/// the point it grows about (`LABEL_SHOWN_STRIDE` numbers), and the boxes its texts hold against the labels.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Part {
    pub records: Arc<[f64]>,
    pub outlines: Arc<[Vec<Vec2>]>,
}

/// What every object's part is read with: the typeface, the sizes on screen a text is drawn at and its rule (with
/// the scale when the rule or the sizes leave some out: none, any scale reads the same).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Reading {
    font: Font,
    rule: u8,
    min_px: u64,
    max_px: u64,
    scale: u64,
}

/// One object's part as last read, and what it was read from.
struct Kept {
    stamp: u64,
    expanded: Option<Arc<Expanded>>,
    veiled: Option<Arc<[bool]>>,
    /// A table's box of cells (its others have none).
    cells: Option<[u64; 4]>,
    /// The last ask that met it.
    round: u64,
    parts: Vec<Part>,
    /// A table's, by its blocks of rows.
    table: Option<TableKept>,
}

/// A table's parts by its blocks of rows (docs/adr/0786 §2): what the table is besides its cells' words (`frame`),
/// each block's words (`prints`) and the parts of the blocks that have cells near the view, by block.
#[derive(Clone, Debug, Default)]
pub(super) struct TableKept {
    pub(super) frame: u64,
    pub(super) prints: Vec<u64>,
    pub(super) parts: Vec<(usize, Part)>,
}

/// The parts kept by object, between asks of one view.
#[derive(Default)]
pub struct LabelMemo {
    reading: Option<Reading>,
    cells: Option<[u64; 4]>,
    round: u64,
    kept: HashMap<u64, Kept>,
    taken: usize,
    made: usize,
    /// The blocks of the tables read again that were taken as kept, in the last ask.
    blocks: usize,
}

impl LabelMemo {
    /// An ask begins: what it reads with (`rule`: the sizes' rule, `scaled`: whether the scale decides something
    /// under it); another reading than the last forgets every object.
    pub(super) fn begin(
        &mut self,
        font: Font,
        rule: u8,
        (min_px, max_px): (f64, f64),
        (scale, scaled): (f64, bool),
        cells: Option<&Bounds>,
    ) {
        // A text is left out when `min_px..=max_px` does not hold its height times the scale: with 0..∞ only the
        // scale's sign and its being a number decide (no height is under 0 or over ∞ at any other scale).
        let free = !scaled && min_px == 0.0 && max_px == f64::INFINITY;
        let scale = if free {
            u64::from(scale > 0.0) | u64::from(scale.is_finite()) << 1
        } else {
            scale.to_bits()
        };
        let reading = Reading {
            font,
            rule,
            min_px: min_px.to_bits(),
            max_px: max_px.to_bits(),
            scale,
        };
        if self.reading != Some(reading) {
            self.kept.clear();
            self.reading = Some(reading);
        }
        self.cells = cells.map(|b| [b.min_x, b.min_y, b.max_x, b.max_y].map(f64::to_bits));
        self.round += 1;
        self.taken = 0;
        self.made = 0;
        self.blocks = 0;
    }

    /// `it`'s parts: as kept while it, its pieces and (a table) its box of cells stay; else made by `make` and kept,
    /// a made part the same to the bit as the kept one in its place being the kept one (a table's rows a change left
    /// as they were, an object put again as it was).
    pub(super) fn parts(
        &mut self,
        it: &Item,
        make: impl FnOnce(Option<&TableKept>) -> (Vec<Part>, Option<TableKept>),
    ) -> Vec<Part> {
        let cells = matches!(it.shape, Shape::Table { .. })
            .then_some(self.cells)
            .flatten();
        let key = it.id.to_bits();
        if let Some(k) = self.kept.get_mut(&key)
            && k.stamp == it.stamp
            && k.cells == cells
            && same(&k.expanded, &it.expanded)
            && same(&k.veiled, &it.veiled)
        {
            k.round = self.round;
            self.taken += 1;
            return k.parts.clone();
        }
        // A table read again in the same box of cells: its blocks whose words stay are taken as they were.
        let old = self
            .kept
            .get(&key)
            .filter(|k| k.cells == cells)
            .and_then(|k| k.table.as_ref());
        let olds: Vec<*const f64> = old
            .map(|o| o.parts.iter().map(|(_, p)| p.records.as_ptr()).collect())
            .unwrap_or_default();
        let (parts, table) = make(old);
        self.blocks += parts
            .iter()
            .filter(|p| olds.contains(&p.records.as_ptr()))
            .count();
        self.made += 1;
        self.kept.insert(
            key,
            Kept {
                stamp: it.stamp,
                expanded: it.expanded.clone(),
                veiled: it.veiled.clone(),
                cells,
                round: self.round,
                parts: parts.clone(),
                table,
            },
        );
        parts
    }

    /// The ask ended: the objects it did not meet are forgotten (the memo holds one view's objects).
    pub(super) fn end(&mut self) {
        let round = self.round;
        self.kept.retain(|_, k| k.round == round);
    }

    /// How many objects the last ask took as kept and how many it read.
    pub fn taken(&self) -> (usize, usize) {
        (self.taken, self.made)
    }

    /// How many blocks of the tables the last ask read again it took as kept (docs/adr/0786 §2).
    pub fn blocks_taken(&self) -> usize {
        self.blocks
    }

    /// Forgets every object (the next ask reads them all).
    pub fn clear(&mut self) {
        self.kept.clear();
        self.reading = None;
    }
}

/// The same `Arc`, or both none. A kept one is held by the memo, so its address is never another's.
fn same<T: ?Sized>(a: &Option<Arc<T>>, b: &Option<Arc<T>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => Arc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}
