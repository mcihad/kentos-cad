//! Grafik doğrama listesi (docs/adr/8038 §6; M38): the drawing's door and window types side by side, each with its
//! joinery code, its elevation as Türkiye draws one (the opening lines start at the hinge side's corners and meet at
//! the middle of the handle side: İTÜ MIM 244, *Mimari Teknik Çizim İlkeleri* Şekil 1b; the British convention is the
//! other way round), its rough size (90/210), its sill height, how it opens, its frame and leaves, its U value and how
//! many openings follow it.
//!
//! The list is one block of the drawing (the digital project regulation's EK-2 4.18.2: doors and windows are blocks;
//! Tablo 4.4's name, `MM-XXXX-DOGRAMALISTESI`), KentOS's: the document writes it again in the step that changes a type
//! or an opening (`follow_joinery`), so every insert of it shows the drawing as it is. A hand edit of the block is not
//! kept. Its texts are 2.5 mm high at 1/50, the scale Mimarlar Odası draws joinery at.

use std::sync::Arc;

use kentos_contracts::{
    BlockDefinition, BlockId, Entity, OpeningForm, OpeningOperation, OpeningType,
};
use serde_json::{Value, json};

use crate::Document;
use crate::history::ArchOp;
use crate::history::Op;

/// The joinery list's block: one in a drawing, KentOS's.
pub const JOINERY_LIST_ID: BlockId = BlockId([
    0x01, 0x93, 0xa5, 0xe0, 0x30, 0x38, 0x70, 0x00, 0x80, 0x00, 0x64, 0x6f, 0x67, 0x72, 0x61, 0x6d,
]);
/// Its name by the digital project regulation's EK-2 Tablo 4.4: discipline, element (XXXX: none), what it shows.
pub const JOINERY_LIST_NAME: &str = "MM-XXXX-DOGRAMALISTESI";
/// The scale it is drawn for.
pub const JOINERY_LIST_SCALE: u32 = 50;

/// Its texts' height, metres: 2.5 mm at 1/50; a code's 3.5 mm.
const TEXT: f64 = 0.125;
const CODE: f64 = 0.175;
/// A cell's inner margin, a row's height, the code row's.
const PAD: f64 = 0.08;
const ROW: f64 = 0.32;
const HEAD_ROW: f64 = 0.45;
/// The row labels' column and the narrowest type column, metres.
const LABEL_W: f64 = 2.1;
const MIN_COL: f64 = 1.7;
/// Around an elevation, and the room its dimensions take.
const MARGIN: f64 = 0.25;
const DIM: f64 = 0.3;
/// Types in one band; bands one under another.
const PER_BAND: usize = 6;
const BAND_GAP: f64 = 0.8;
/// A text's width per letter, as a share of its height.
const LETTER: f64 = 0.56;

/// The list's row labels, top to bottom.
pub const ROWS: [&str; 8] = [
    "Kod",
    "Görünüş",
    "Kaba ölçü (cm)",
    "Denizlik (cm)",
    "Açılım",
    "Kasa ve kanat",
    "U (W/m²K)",
    "Adet",
];

/// A block's box about its base point, metres: its lines' and outlines' points (its texts lie inside them).
pub fn extent(def: &BlockDefinition) -> Option<[f64; 4]> {
    let mut b: Option<[f64; 4]> = None;
    let mut take = |x: f64, y: f64| {
        b = Some(match b {
            None => [x, y, x, y],
            Some(o) => [o[0].min(x), o[1].min(y), o[2].max(x), o[3].max(y)],
        });
    };
    for e in &def.entities {
        match e {
            Entity::Line(l) => {
                take(l.a.x, l.a.y);
                take(l.b.x, l.b.y);
            }
            Entity::Polyline(p) => p.pts.iter().for_each(|q| take(q.x, q.y)),
            _ => {}
        }
    }
    b
}

/// A type the list shows and how many openings follow it.
#[derive(Clone, Debug, PartialEq)]
pub struct JoineryRow {
    pub kind: OpeningType,
    pub count: usize,
}

/// The objects the list's block holds, as the file writes them.
#[derive(Default)]
struct Shapes {
    out: Vec<Value>,
}

impl Shapes {
    fn id(&self) -> u64 {
        self.out.len() as u64 + 1
    }

    fn line(&mut self, a: (f64, f64), b: (f64, f64)) {
        let id = self.id();
        self.out
            .push(json!({"kind": "line", "id": id, "layerId": "", "attrs": {},
            "a": {"x": a.0, "y": a.1}, "b": {"x": b.0, "y": b.1}}));
    }

    fn rect(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        let id = self.id();
        self.out.push(json!({"kind": "polyline", "id": id, "layerId": "", "attrs": {},
            "pts": [{"x": x0, "y": y0}, {"x": x1, "y": y0}, {"x": x1, "y": y1}, {"x": x0, "y": y1}], "closed": true}));
    }

    fn text(&mut self, p: (f64, f64), words: &str, height: f64, align: &str, rotation: f64) {
        if words.is_empty() {
            return;
        }
        let id = self.id();
        self.out.push(json!({"kind": "text", "id": id, "layerId": "", "attrs": {},
            "p": {"x": p.0, "y": p.1}, "text": words, "height": height, "rotation": rotation, "align": align}));
    }

    /// An arrow from `a` to `b`: its line and a head of two strokes.
    fn arrow(&mut self, a: (f64, f64), b: (f64, f64)) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = dx.hypot(dy);
        if len < 1e-9 {
            return;
        }
        let (ux, uy) = (dx / len, dy / len);
        let h = (0.08_f64).min(len / 3.0);
        self.line(a, b);
        for s in [1.0, -1.0] {
            let (px, py) = (-uy * s, ux * s);
            self.line(
                b,
                (b.0 - ux * h + px * h * 0.5, b.1 - uy * h + py * h * 0.5),
            );
        }
    }
}

/// `v` metres in centimetres, whole when whole: “90”, “90.5”.
fn cm(v: f64) -> String {
    let c = v * 100.0;
    if (c - c.round()).abs() < 0.05 {
        format!("{:.0}", c.round())
    } else {
        format!("{c:.1}")
    }
}

/// A U value as the list writes it: “1,6”.
fn u_text(v: f64) -> String {
    let t = format!("{v:.2}");
    t.trim_end_matches('0')
        .trim_end_matches('.')
        .replace('.', ",")
}

/// `words` in lines no wider than `width` metres at `height`.
fn wrap(words: &str, width: f64, height: f64) -> Vec<String> {
    let most = ((width / (height * LETTER)).floor() as usize).max(4);
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for w in words.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + w.chars().count() > most {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(w);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// A text's width, metres, as the list estimates it.
fn width_of(words: &str, height: f64) -> f64 {
    words.chars().count() as f64 * height * LETTER
}

/// An elevation's width and height, metres: a door's rough opening up to its head, a window's from its sill.
fn size_of(t: &OpeningType) -> (f64, f64) {
    let h = match t.form {
        OpeningForm::Window => {
            let (sill, head) = (t.sill.unwrap_or(0.9), t.head.unwrap_or(2.2));
            if head - sill > 0.05 { head - sill } else { 1.0 }
        }
        _ => t.head.filter(|h| *h > 0.05).unwrap_or(2.1),
    };
    (t.width, h)
}

/// How a type opens, as the list says it: “Kanatlı, 2 kanat”, “Sabit”.
fn opening_said(t: &OpeningType) -> String {
    let word = t.operation.label();
    match t.operation {
        OpeningOperation::Fixed => word.to_owned(),
        _ if t.leaves > 1 => format!("{word}, {} {}", t.leaves, t.operation.leaf_word()),
        _ => word.to_owned(),
    }
}

/// The side-hung marks of a leaf from `x0` to `x1`, `y0` to `y1`: two lines from the hinge side's corners to the
/// middle of the other side (Türkiye's convention).
fn swing_marks(s: &mut Shapes, x0: f64, y0: f64, x1: f64, y1: f64, hinge_left: bool) {
    let (hx, ox) = if hinge_left { (x0, x1) } else { (x1, x0) };
    let mid = (y0 + y1) / 2.0;
    s.line((hx, y1), (ox, mid));
    s.line((hx, y0), (ox, mid));
}

/// A type's elevation with its rough opening's bottom left at `(x, y)`, its dimensions below and on its left.
fn elevation(s: &mut Shapes, t: &OpeningType, x: f64, y: f64) {
    let (w, h) = size_of(t);
    let b = t.build.clone().unwrap_or_default();
    // The rough opening and the frame.
    s.rect(x, y, x + w, y + h);
    let door = t.form == OpeningForm::Door;
    let fr = t.frame.filter(|f| *f > 0.0 && 2.0 * f < w).unwrap_or(0.0);
    let (ix0, ix1) = (x + fr, x + w - fr);
    let (iy0, iy1) = (if door { y } else { y + fr }, y + h - fr);
    if fr > 0.0 {
        s.rect(ix0, iy0, ix1, iy1);
    }
    let light = ix1 - ix0;
    if door {
        let n = t.leaves.max(1) as usize;
        let share = b.split.filter(|v| *v > 0.0 && *v < 1.0).unwrap_or(0.5);
        // The leaves' edges along the light.
        let edges: Vec<f64> = match (n, t.operation) {
            (2, OpeningOperation::Swing) => vec![ix0, ix0 + light * share, ix1],
            _ => (0..=n).map(|i| ix0 + light * i as f64 / n as f64).collect(),
        };
        for k in 0..n {
            let (a, c) = (edges[k], edges[k + 1]);
            if k > 0 {
                s.line((a, iy0), (a, iy1));
            }
            match t.operation {
                // One leaf hinged on its right (İTÜ's figure); a pair at its jambs, meeting in the middle.
                OpeningOperation::Swing => {
                    let hinge_left = n > 1 && k == 0;
                    swing_marks(s, a, iy0, c, iy1, hinge_left);
                    let handle = if hinge_left { c - 0.04 } else { a + 0.04 };
                    let towards = if hinge_left { -0.12 } else { 0.12 };
                    s.line((handle, iy0 + 1.0), (handle + towards, iy0 + 1.0));
                }
                OpeningOperation::Slide | OpeningOperation::Pocket => {
                    let my = (iy0 + iy1) / 2.0;
                    let (from, to) = if k % 2 == 0 {
                        (c - 0.15, a + 0.15)
                    } else {
                        (a + 0.15, c - 0.15)
                    };
                    s.arrow((from, my), (to, my));
                }
                OpeningOperation::Folding => {
                    // A panel's fold, as a zigzag's stroke.
                    let up = k % 2 == 0;
                    s.line(
                        (a, if up { iy0 } else { iy1 }),
                        (c, if up { iy1 } else { iy0 }),
                    );
                }
                OpeningOperation::Revolving | OpeningOperation::Fixed => {}
            }
        }
    } else {
        let mullions = b.mullions.unwrap_or(0) as usize;
        let transoms = b.transoms.unwrap_or(0) as usize;
        let leaves = t.leaves.max(1) as usize;
        let cols = if mullions > 0 { mullions + 1 } else { leaves };
        let rows = transoms + 1;
        let col_w = light / cols as f64;
        let row_h = (iy1 - iy0) / rows as f64;
        for i in 1..cols {
            let cx = ix0 + col_w * i as f64;
            s.line((cx, iy0), (cx, iy1));
        }
        for j in 1..rows {
            let cy = iy0 + row_h * j as f64;
            s.line((ix0, cy), (ix1, cy));
        }
        // The opening sashes: the bottom row's first panes, as many as the leaves; each hinged on the side nearer its
        // jamb.
        if t.operation != OpeningOperation::Fixed {
            let sash = 0.03_f64.min(col_w / 6.0);
            for k in 0..leaves.min(cols) {
                let (a, c) = (
                    ix0 + col_w * k as f64 + sash,
                    ix0 + col_w * (k + 1) as f64 - sash,
                );
                let (b0, b1) = (iy0 + sash, iy0 + row_h - sash);
                s.rect(a, b0, c, b1);
                match t.operation {
                    OpeningOperation::Slide => {
                        let my = (b0 + b1) / 2.0;
                        let (from, to) = if k % 2 == 0 {
                            (c - 0.1, a + 0.1)
                        } else {
                            (a + 0.1, c - 0.1)
                        };
                        s.arrow((from, my), (to, my));
                    }
                    _ => {
                        let middle = ix0 + col_w * (k as f64 + 0.5);
                        swing_marks(s, a, b0, c, b1, middle < x + w / 2.0);
                    }
                }
            }
        }
        // The sill board, under the window.
        if b.sill_board.is_some_and(|v| v > 0.0) {
            s.rect(x - 0.05, y - 0.04, x + w + 0.05, y);
        }
    }
    // Its rough size: across under it, up on its left.
    let dy = y - 0.18;
    s.line((x, dy), (x + w, dy));
    for px in [x, x + w] {
        s.line((px - 0.04, dy - 0.04), (px + 0.04, dy + 0.04));
        s.line((px, dy - 0.06), (px, y - 0.02));
    }
    s.text(
        (x + w / 2.0, dy - 0.04),
        &cm(w),
        TEXT * 0.8,
        "topCenter",
        0.0,
    );
    let dx = x - 0.18;
    s.line((dx, y), (dx, y + h));
    for py in [y, y + h] {
        s.line((dx - 0.04, py - 0.04), (dx + 0.04, py + 0.04));
        s.line((dx - 0.06, py), (x - 0.02, py));
    }
    s.text(
        (dx - 0.04, y + h / 2.0),
        &cm(h),
        TEXT * 0.8,
        "bottomCenter",
        90.0,
    );
}

/// A type's cells under its code, in [`ROWS`]'s order after Görünüş.
fn cells(r: &JoineryRow) -> [String; 5] {
    let t = &r.kind;
    let (w, h) = size_of(t);
    [
        format!("{}/{}", cm(w), cm(h)),
        match t.form {
            OpeningForm::Window => cm(t.sill.unwrap_or(0.0)),
            _ => "—".to_owned(),
        },
        opening_said(t),
        t.material.clone().unwrap_or_else(|| "—".to_owned()),
        t.u_value.map_or_else(|| "—".to_owned(), u_text),
    ]
}

impl Document {
    /// The door and window types the list shows: those openings follow, by their joinery codes (types without one
    /// after, by name), each with how many openings follow it.
    pub fn joinery_rows(&self) -> Vec<JoineryRow> {
        let mut rows: Vec<JoineryRow> = self
            .opening_types()
            .iter()
            .filter(|t| matches!(t.form, OpeningForm::Door | OpeningForm::Window))
            .filter_map(|t| {
                let count = self
                    .entities()
                    .filter(|e| {
                        matches!(e, Entity::Opening(o)
                        if o.opening.follows_type() && o.opening.style.as_deref() == Some(t.id.as_str()))
                    })
                    .count();
                (count > 0).then(|| JoineryRow {
                    kind: t.clone(),
                    count,
                })
            })
            .collect();
        let key = |r: &JoineryRow| {
            (
                r.kind.code.is_none(),
                r.kind.code.clone().unwrap_or_else(|| r.kind.name.clone()),
            )
        };
        rows.sort_by(|a, b| {
            let (ka, kb) = (key(a), key(b));
            ka.0.cmp(&kb.0)
                .then_with(|| kentos_geometry_core::text::natural::natural_cmp(&ka.1, &kb.1))
        });
        rows
    }

    /// The joinery list's block as the drawing's types give it (docs/adr/8038 §6): its top left at its base point,
    /// bands of six types one under another.
    pub fn joinery_list_block(&self) -> BlockDefinition {
        let rows = self.joinery_rows();
        let mut s = Shapes::default();
        let mut top = 0.0_f64;
        if rows.is_empty() {
            s.rect(0.0, 0.0, LABEL_W + MIN_COL * 2.0, -HEAD_ROW);
            s.text(
                (PAD, -HEAD_ROW / 2.0),
                "Doğrama listesi: çizimde tipe bağlı kapı ya da pencere yok.",
                TEXT,
                "middleLeft",
                0.0,
            );
        }
        for band in rows.chunks(PER_BAND) {
            let texts: Vec<[String; 5]> = band.iter().map(cells).collect();
            let widths: Vec<f64> = band
                .iter()
                .zip(&texts)
                .map(|(r, c)| {
                    let (w, _) = size_of(&r.kind);
                    let words = c[..3]
                        .iter()
                        .chain(std::iter::once(&c[4]))
                        .map(|t| width_of(t, TEXT))
                        .fold(0.0, f64::max);
                    (w + 2.0 * MARGIN + DIM).max(words + 2.0 * PAD).max(MIN_COL)
                })
                .collect();
            let materials: Vec<Vec<String>> = texts
                .iter()
                .zip(&widths)
                .map(|(c, w)| wrap(&c[3], w - 2.0 * PAD, TEXT))
                .collect();
            let lines = materials.iter().map(Vec::len).max().unwrap_or(1).max(1);
            let tallest = band.iter().map(|r| size_of(&r.kind).1).fold(0.0, f64::max);
            let heights = [
                HEAD_ROW,
                tallest + 2.0 * MARGIN + DIM,
                ROW,
                ROW,
                ROW,
                ROW.max(lines as f64 * TEXT * 1.5 + 2.0 * PAD),
                ROW,
                ROW,
            ];
            let width = LABEL_W + widths.iter().sum::<f64>();
            let height: f64 = heights.iter().sum();
            // The grid: its frame, a line under each row, one after the labels and each type.
            s.rect(0.0, top, width, top - height);
            let mut y = top;
            for h in &heights[..heights.len() - 1] {
                y -= h;
                s.line((0.0, y), (width, y));
            }
            let mut x = LABEL_W;
            s.line((x, top), (x, top - height));
            for w in &widths[..widths.len() - 1] {
                x += w;
                s.line((x, top), (x, top - height));
            }
            // The labels.
            let mut y = top;
            for (label, h) in ROWS.iter().zip(heights) {
                s.text((PAD, y - h / 2.0), label, TEXT, "middleLeft", 0.0);
                y -= h;
            }
            // Each type's column.
            let mut x = LABEL_W;
            for ((r, c), (w, m)) in band.iter().zip(&texts).zip(widths.iter().zip(&materials)) {
                let mid = x + w / 2.0;
                let code = r.kind.code.clone().unwrap_or_else(|| r.kind.name.clone());
                s.text(
                    (mid, top - HEAD_ROW / 2.0),
                    &code,
                    CODE,
                    "middleCenter",
                    0.0,
                );
                // The elevation, on its cell's floor, its dimensions inside the cell.
                let (ew, _) = size_of(&r.kind);
                let cell_bottom = top - HEAD_ROW - heights[1];
                elevation(
                    &mut s,
                    &r.kind,
                    mid - ew / 2.0 + DIM / 2.0,
                    cell_bottom + MARGIN + DIM / 2.0,
                );
                let mut y = top - HEAD_ROW - heights[1];
                for (k, h) in heights.iter().enumerate().skip(2) {
                    let words = match k {
                        2 => c[0].clone(),
                        3 => c[1].clone(),
                        4 => c[2].clone(),
                        6 => c[4].clone(),
                        7 => r.count.to_string(),
                        _ => String::new(),
                    };
                    if k == 5 {
                        // The frame and leaves, line by line.
                        let n = m.len().max(1) as f64;
                        for (i, line) in m.iter().enumerate() {
                            let ly = y - h / 2.0 + (n - 1.0) * TEXT * 0.75 - i as f64 * TEXT * 1.5;
                            s.text((mid, ly), line, TEXT, "middleCenter", 0.0);
                        }
                    } else {
                        s.text((mid, y - h / 2.0), &words, TEXT, "middleCenter", 0.0);
                    }
                    y -= h;
                }
                x += w;
            }
            top -= height + BAND_GAP;
        }
        // The note under it: what the drawing shows and how the lines open.
        s.text(
            (0.0, top + BAND_GAP - 0.25),
            "Ölçüler kaba yapı boşluğudur (cm). Açılım çizgileri menteşe tarafından kol tarafına iner.",
            TEXT * 0.8,
            "middleLeft",
            0.0,
        );
        let entities: Vec<Entity> = s
            .out
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect();
        BlockDefinition {
            id: JOINERY_LIST_ID,
            name: JOINERY_LIST_NAME.to_owned(),
            base: kentos_contracts::Vec2 { x: 0.0, y: 0.0 },
            entities,
            attributes: Vec::new(),
            description: Some(
                "Grafik doğrama listesi: KentOS yazar; kapı ve pencere tipleri değişince yeniden yazılır (elle \
                 değişiklik korunmaz)."
                    .into(),
            ),
            units: Some(kentos_contracts::BlockUnits::M),
            card: None,
            dynamic: None,
            xref: None,
        }
    }

    /// The joinery list after `ops` (docs/adr/8038 §6), applied: when the drawing keeps the list and the step changed a
    /// type or an opening, the list's block written again in the step.
    pub(crate) fn follow_joinery(&mut self, ops: &[Op]) -> Vec<Op> {
        let touched = ops.iter().any(|op| match op {
            Op::Arch(ArchOp::OpeningTypes { .. }) => true,
            Op::Add(s) | Op::Remove(s) => s.entity.kind() == "opening",
            Op::Update { before, after } => {
                before.entity.kind() == "opening" || after.entity.kind() == "opening"
            }
            _ => false,
        });
        if !touched {
            return Vec::new();
        }
        let Some(before) = self
            .blocks
            .iter()
            .find(|b| b.id == JOINERY_LIST_ID)
            .cloned()
        else {
            return Vec::new();
        };
        let after = self.joinery_list_block();
        if *before == after {
            return Vec::new();
        }
        let op = Op::BlockUpdate {
            before,
            after: Arc::new(after),
        };
        self.apply_op(&op);
        vec![op]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_and_words_are_written_as_the_list_says_them() {
        assert_eq!(
            (cm(0.9), cm(2.1), cm(0.905)),
            ("90".into(), "210".into(), "90.5".into())
        );
        assert_eq!(
            (u_text(1.6), u_text(1.25), u_text(2.0)),
            ("1,6".into(), "1,25".into(), "2".into())
        );
        assert_eq!(
            wrap("PVC, 5 odacıklı; çift camlı Low-E 4+16+4", 1.6, TEXT),
            ["PVC, 5 odacıklı; çift", "camlı Low-E 4+16+4"]
        );
    }

    #[test]
    fn a_leaf_s_lines_start_at_its_hinge_and_meet_at_its_handle() {
        let mut s = Shapes::default();
        swing_marks(&mut s, 0.0, 0.0, 1.0, 2.0, false);
        // Hinged on the right: from the right corners to the left side's middle.
        let ends: Vec<(f64, f64, f64, f64)> = s
            .out
            .iter()
            .map(|v| {
                (
                    v["a"]["x"].as_f64().unwrap(),
                    v["a"]["y"].as_f64().unwrap(),
                    v["b"]["x"].as_f64().unwrap(),
                    v["b"]["y"].as_f64().unwrap(),
                )
            })
            .collect();
        assert_eq!(ends, [(1.0, 2.0, 0.0, 1.0), (1.0, 0.0, 0.0, 1.0)]);
    }
}
