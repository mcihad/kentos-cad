//! Leaders and their notes (docs/adr/0146 §8). A LEADER becomes a leader;
//! the MTEXT its 340 names, before or after it in the file, becomes its note
//! instead of a text: the note's first line with words gives the leader its
//! words, height, turn and mask, and its other lines stay a multi-line text
//! (said; docs/adr/0182 §5). Whichever of the two comes first waits for the
//! other here. A leader already in the drawing takes the first line when its
//! MTEXT comes, and that text, the last thing written, gives way to the
//! other lines (or is taken out). A note already in the drawing gives its
//! place to the leader, the other lines written after. Either way nothing
//! else moves, so the places other objects are known by stay true. The
//! note's other lines go under it, as they hang under the first in the
//! MTEXT. A MULTILEADER is its first leader line, its MTEXT content its note
//! in the same way.

use std::collections::HashMap;

use kentos_contracts::{
    Entity, EntityBase, LeaderArrow, LeaderEntity, Paragraph, TextAlign, TextEntity, TextRun, Vec2,
};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::geom::leader::layout;

use super::super::dimension::mtext_value;
use super::super::entity::{Kind, Parsed};
use super::super::leaders::{DimStyle, MLeader, arrow_of_block};
use super::super::strings::mtext_lines;
use super::super::xdata::caret_decode;
use super::{Ctx, Emitter, anchor_points, apply_meta, mapped_text, xy};
use crate::geom::v;
use crate::math::{atan2, deg, hypot, sin_cos_deg};

const LEADER: &str = "Kılavuz (LEADER)";
const MLEADER: &str = "Çoklu kılavuz (MULTILEADER)";
/// A leader's height when the file gives none: AutoCAD's ISO arrow.
const FALLBACK_HEIGHT: f64 = 2.5;
const NO_HEIGHT: &str =
    "yüksekliği dosyada yok (ne 40 ne boyut stilinin ok boyu); 2,5 alındı";
const MORE_LINES: &str =
    "notunun ilk satırı kılavuzun notu oldu; öbür satırları ayrı yazı olarak alındı";

/// What waits for its other half.
#[derive(Default)]
pub(super) struct Pending {
    /// Leaders in the drawing waiting for their MTEXT, by its handle.
    leaders: HashMap<u64, Waiting>,
    /// MTEXTs in the drawing a LEADER may still claim, by handle.
    notes: HashMap<u64, Note>,
}

struct Waiting {
    /// Where the leader is among the objects written.
    index: usize,
    /// Its last vertex is a hookline's end.
    hook: bool,
    exact: Exact,
    /// No height came with it: it has the fallback until its note comes.
    fallback: bool,
    line: u32,
}

/// An MTEXT written: where its text is.
#[derive(Clone, Copy)]
struct Note {
    index: usize,
}

/// What KentOS's data says exactly of a leader's note.
#[derive(Clone, Default)]
struct Exact {
    note: Option<String>,
    turn: Option<f64>,
}

/// Two turns (degrees) a billionth of a degree apart at most.
fn same_turn(a: f64, b: f64) -> bool {
    let d = (a - b).rem_euclid(360.0);
    d < 1e-9 || 360.0 - d < 1e-9
}

/// An MTEXT's text as a leader's note: its first line with words, a
/// one-line text of its words alone, and the lines after it, a multi-line
/// text of their own (none when they have no words).
fn split_note(t: &TextEntity) -> (TextEntity, Option<TextEntity>) {
    let letters: Vec<char> = t.text.chars().collect();
    let line_end = |from: usize| {
        letters[from..]
            .iter()
            .position(|&c| c == '\n')
            .map_or(letters.len(), |k| from + k)
    };
    let (mut a, mut b) = (0, line_end(0));
    while b < letters.len() && letters[a..b].iter().all(|c| c.is_whitespace()) {
        a = b + 1;
        b = line_end(a);
    }
    let first = TextEntity {
        text: letters[a..b].iter().collect(),
        paragraph: Paragraph::default(),
        ..t.clone()
    };
    let from = b + 1;
    let rest =
        (from < letters.len() && letters[from..].iter().any(|c| !c.is_whitespace())).then(|| {
            let shift = from as u32;
            let runs = t
                .paragraph
                .runs
                .iter()
                .filter(|r| r.end > shift)
                .map(|r| TextRun {
                    start: r.start.max(shift) - shift,
                    end: r.end - shift,
                    ..r.clone()
                })
                .collect();
            TextEntity {
                text: letters[from..].iter().collect(),
                paragraph: Paragraph {
                    runs,
                    ..t.paragraph.clone()
                },
                ..t.clone()
            }
        });
    (first, rest)
}

/// The leader takes its note from the first line of its MTEXT: the words
/// (KentOS's own, while the MTEXT still says them), the height, the turn
/// (KentOS's own, while the MTEXT's direction is still it) and the mask.
/// With a hookline its last vertex was the hook's end: the leader's own
/// landing stands in for it.
fn give_note(l: &mut LeaderEntity, first: &TextEntity, hook: bool, exact: &Exact) {
    let says = |n: &String| {
        mtext_lines(&caret_decode(&mtext_value(n)))
            .into_iter()
            .find(|line| !line.trim().is_empty())
            .is_some_and(|line| line == first.text)
    };
    l.text = Some(
        exact
            .note
            .clone()
            .filter(says)
            .unwrap_or_else(|| first.text.clone()),
    );
    l.height = first.height;
    l.rotation = exact
        .turn
        .filter(|t| same_turn(*t, first.rotation))
        .unwrap_or(first.rotation);
    l.mask = first.mask;
    if hook && l.pts.len() >= 3 {
        l.pts.pop();
    }
}

impl Emitter<'_> {
    /// What a dimension style says of a leader or a dimension: the one named, else Standard.
    pub(super) fn dim_style(&self, name: &str) -> DimStyle {
        let lib = self.lib;
        lib.dim_styles
            .get(&name.trim().to_uppercase())
            .or_else(|| lib.dim_styles.get("STANDARD"))
            .copied()
            .unwrap_or_default()
    }

    /// The arrowhead an arrow block stands for, by its record's name; one
    /// KentOS has no such arrowhead for is drawn filled, and said.
    fn arrow_of(&mut self, block: Option<u64>, what: &str, line: u32) -> Option<LeaderArrow> {
        let lib = self.lib;
        let name = block.and_then(|h| lib.block_records.get(&h))?;
        let (arrow, known) = arrow_of_block(name);
        if !known {
            self.note(
                what,
                &format!("ok başı “{name}” KentOS'ta yok; dolu ok olarak alındı"),
                line,
            );
        }
        arrow
    }

    /// Counts an object into the drawing's tallies, or out of them (a
    /// definition's objects are counted by their inserts).
    fn tally(&mut self, e: &Entity, add: bool) {
        if self.defining {
            return;
        }
        let layer = &e.base().layer_id;
        if add {
            *self.out.per_layer.entry(layer.clone()).or_insert(0) += 1;
            self.out.report.count(e.kind());
            for p in anchor_points(e) {
                self.out.extend_bounds(p);
            }
        } else {
            if let Some(n) = self.out.per_layer.get_mut(layer) {
                *n = n.saturating_sub(1);
                if *n == 0 {
                    self.out.per_layer.remove(layer);
                }
            }
            self.out.report.uncount(e.kind());
        }
    }

    /// The other lines of a leader's MTEXT (`rest`, a multi-line text) under
    /// its note: its first line's top one line pitch under the note's,
    /// aligned at the left, centre or right as the note is, turned as it.
    fn under_note(l: &LeaderEntity, mut rest: TextEntity) -> TextEntity {
        let pts: Vec<CoreVec2> = l.pts.iter().map(|p| CoreVec2::new(p.x, p.y)).collect();
        let arrow = l.arrow.map(LeaderArrow::name);
        let Some(laid) = layout(&pts, l.height, l.rotation, arrow, true) else {
            return rest;
        };
        let (Some(at), Some(align)) = (laid.note_point, laid.note_align) else {
            return rest;
        };
        let h = rest.height;
        let pitch = h * 5.0 / 3.0 * rest.paragraph.line_spacing.unwrap_or(1.0);
        // From the note's point down to its baseline, one pitch on, and up to that line's top.
        let k = h - pitch - align.up() * h;
        let (s, c) = sin_cos_deg(l.rotation);
        rest.p = Vec2 {
            x: at.x - s * k,
            y: at.y + c * k,
        };
        rest.align = Some(if align.along() == 0.0 {
            TextAlign::TopLeft
        } else if align.along() == 1.0 {
            TextAlign::TopRight
        } else {
            TextAlign::TopCenter
        });
        rest.rotation = l.rotation;
        rest
    }

    /// The text at `index` gives its place to `leader`.
    fn take_place(&mut self, index: usize, leader: Entity) {
        self.tally(&leader, true);
        let text = std::mem::replace(&mut self.out.entities[index], leader);
        self.tally(&text, false);
    }

    /// A leader's height and turn when no note gives them, as `ctx` places
    /// them: `own` (drawing units) and `turn` (degrees, its text's direction).
    fn plain_size(ctx: &Ctx, own: f64, turn: f64) -> Option<(f64, f64)> {
        let (_, rotation, height, _) = mapped_text(ctx.tf, v(0.0, 0.0), turn, own)?;
        Some((height, rotation))
    }

    /// A LEADER (docs/adr/0146 §8).
    pub(super) fn leader(&mut self, ctx: &Ctx, b: EntityBase, e: &Parsed) {
        let Kind::Leader {
            pts,
            horizontal,
            arrow,
            spline,
            hook,
            made_with,
            annotation,
            height,
            style,
            own_style,
        } = &e.kind
        else {
            return;
        };
        let pts: Vec<Vec2> = pts.iter().map(|p| ctx.tf.apply(xy(*p))).collect();
        if pts.len() < 2 {
            return self.skip(LEADER, "iki noktası yok", e.line);
        }
        if *spline {
            self.note(LEADER, "eğri yolu kırık çizgi olarak alındı", e.line);
        }
        let style = own_style.over(self.dim_style(style));
        let meta = e.meta.as_ref();
        let arrow = if *arrow {
            // KentOS's own arrowhead, while the file still draws one.
            match meta
                .and_then(|m| m.arrow.as_deref())
                .and_then(LeaderArrow::from_name)
            {
                Some(a) => Some(a),
                None => self.arrow_of(style.arrow_block, LEADER, e.line),
            }
        } else {
            Some(LeaderArrow::None)
        };
        // The text's height (40), else the style's arrow, else AutoCAD's own.
        let (own, fallback) = match Some(*height)
            .filter(|h| *h > 0.0)
            .or_else(|| style.arrow_length())
        {
            Some(h) => (h, false),
            None => (FALLBACK_HEIGHT, true),
        };
        // The direction its text runs (211), read as an MTEXT's (11) is.
        let turn = horizontal
            .map(|d| (d[0], d[1], hypot(d[0], d[1])))
            .filter(|(_, _, l)| *l > 0.0)
            .map_or(0.0, |(x, y, l)| deg(atan2(y / l, x / l)));
        let Some((height, rotation)) = Self::plain_size(ctx, own, turn) else {
            return self.skip(LEADER, "bloğun dönüşümü onu düzleştiriyor", e.line);
        };
        let exact = Exact {
            note: meta.and_then(|m| m.note.clone()),
            turn: meta.and_then(|m| m.note_turn),
        };
        let rotation = exact
            .turn
            .filter(|t| same_turn(*t, rotation))
            .unwrap_or(rotation);
        let mut leader = LeaderEntity {
            base: b,
            pts,
            text: None,
            height,
            rotation,
            arrow,
            mask: false,
        };
        // Made with an MTEXT (73 = 0, or not said): its note is the MTEXT 340 names.
        let Some(h) = annotation.filter(|_| made_with.unwrap_or(0) == 0) else {
            if fallback {
                self.note(LEADER, NO_HEIGHT, e.line);
            }
            return self.push(Entity::Leader(leader));
        };
        if let Some(note) = self.pending.notes.remove(&h)
            && let Some(Entity::Text(text)) = self.out.entities.get(note.index)
        {
            // Its MTEXT came first: the leader takes its place, the other lines come after.
            let (first, rest) = split_note(text);
            give_note(&mut leader, &first, *hook, &exact);
            let rest = rest.map(|r| Self::under_note(&leader, r));
            let mut leader = Entity::Leader(leader);
            if let Some(meta) = meta {
                apply_meta(meta, &mut leader);
            }
            self.take_place(note.index, leader);
            if let Some(rest) = rest {
                self.push(Entity::Text(rest));
                self.note(LEADER, MORE_LINES, e.line);
            }
            return;
        }
        let index = self.out.entities.len();
        self.push(Entity::Leader(leader));
        if self.out.entities.len() > index {
            self.pending.leaders.insert(
                h,
                Waiting {
                    index,
                    hook: *hook,
                    exact,
                    fallback,
                    line: e.line,
                },
            );
        }
    }

    /// An MTEXT (handle `h`) just written as a text at `start`: a leader
    /// waiting for it takes its first line; otherwise a LEADER may still claim it.
    pub(super) fn mtext_written(&mut self, h: u64, start: usize) {
        let end = self.out.entities.len();
        if let Some(w) = self.pending.leaders.remove(&h) {
            // An empty MTEXT gave nothing: the leader stays without a note.
            let Some(Entity::Text(text)) = self
                .out
                .entities
                .get(start)
                .filter(|_| end > start)
                .cloned()
            else {
                return;
            };
            let (first, rest) = split_note(&text);
            let Some(Entity::Leader(l)) = self.out.entities.get_mut(w.index) else {
                return;
            };
            give_note(l, &first, w.hook, &w.exact);
            let l = l.clone();
            match rest {
                Some(rest) => {
                    self.out.entities[start] = Entity::Text(Self::under_note(&l, rest));
                    self.note(LEADER, MORE_LINES, w.line);
                }
                None => {
                    let text = self.out.entities.remove(start);
                    self.tally(&text, false);
                }
            }
            return;
        }
        if end > start {
            self.pending.notes.insert(h, Note { index: start });
        }
    }

    /// At the end of the drawing or of a block: the leaders still waiting
    /// have no MTEXT in the file (said); the notes left are texts.
    pub fn notes_done(&mut self) {
        let mut left: Vec<(u32, bool)> = std::mem::take(&mut self.pending.leaders)
            .into_values()
            .map(|w| (w.line, w.fallback))
            .collect();
        self.pending.notes.clear();
        left.sort_unstable();
        for (line, fallback) in left {
            self.note(LEADER, "bağlı notu (340) dosyada yok; notsuz alındı", line);
            if fallback {
                self.note(LEADER, NO_HEIGHT, line);
            }
        }
    }

    /// A MULTILEADER: its first leader line, its MTEXT content its note as a
    /// LEADER's MTEXT is. Its other lines and a block content are not taken
    /// (said); without an MTEXT it has no note, its height its arrow's.
    pub(super) fn mleader(&mut self, ctx: &Ctx, m: &MLeader, b: EntityBase, e: &Parsed) {
        let Some(first) = m.lines.first() else {
            return self.skip(MLEADER, "ok çizgisi yok", e.line);
        };
        let mut pts: Vec<Vec2> = Vec::with_capacity(first.len());
        for p in first {
            let q = ctx.tf.apply(xy(*p));
            // The line's last vertex may be its leader's last point too.
            if pts.last() != Some(&q) {
                pts.push(q);
            }
        }
        if pts.len() < 2 {
            return self.skip(MLEADER, "iki noktası yok", e.line);
        }
        if m.lines.len() > 1 {
            self.note(
                MLEADER,
                &format!(
                    "ilk ok çizgisi alındı; öbür {} ok çizgisi alınmadı",
                    m.lines.len() - 1
                ),
                e.line,
            );
        }
        if m.spline {
            self.note(MLEADER, "eğri ok çizgisi kırık çizgi olarak alındı", e.line);
        }
        if m.block {
            self.note(MLEADER, "blok içeriği alınmadı; kılavuz notsuz alındı", e.line);
        }
        let arrow = self.arrow_of(m.arrow_block, MLEADER, e.line);
        let (own, fallback) = if m.arrow_size > 0.0 {
            (m.arrow_size, false)
        } else {
            (FALLBACK_HEIGHT, true)
        };
        let Some((height, rotation)) = Self::plain_size(ctx, own, 0.0) else {
            return self.skip(MLEADER, "bloğun dönüşümü onu düzleştiriyor", e.line);
        };
        let mut leader = LeaderEntity {
            base: b.clone(),
            pts,
            text: None,
            height,
            rotation,
            arrow,
            mask: false,
        };
        if let Some(text) = m.text.as_deref().filter(|_| !m.block) {
            // The content as an MTEXT at 12, its lines from the top down.
            let start = self.out.entities.len();
            let attach = match m.alignment {
                2 | 3 => m.alignment,
                _ => 1,
            };
            let fill = if m.mask { 3 } else { 0 };
            self.mtext(
                ctx,
                [0.0, 0.0, 1.0],
                m.text_at,
                m.text_height,
                attach,
                m.text_dir,
                None,
                text,
                0.0,
                m.spacing,
                fill,
                "",
                b,
                e,
            );
            let end = self.out.entities.len();
            if let Some(Entity::Text(text)) = self
                .out
                .entities
                .get(start)
                .filter(|_| end > start)
                .cloned()
            {
                let (first, rest) = split_note(&text);
                give_note(&mut leader, &first, false, &Exact::default());
                let rest = rest.map(|r| Self::under_note(&leader, r));
                self.take_place(start, Entity::Leader(leader));
                if let Some(rest) = rest {
                    self.push(Entity::Text(rest));
                    self.note(MLEADER, MORE_LINES, e.line);
                }
                return;
            }
        }
        if fallback {
            self.note(MLEADER, NO_HEIGHT, e.line);
        }
        self.push(Entity::Leader(leader));
    }
}
