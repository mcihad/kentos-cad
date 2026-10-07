//! Hizala ve dağıt (docs/adr/0194 §1): the web's `AlignDistributeTool`
//! (`apps/web/src/tools/alignDistributeTool.ts`) on the modify tools' base
//! ([`crate::modify`]):
//!
//! - the objects first (or the selection); the method's letter (Sola S,
//!   Ortala O, Sağa A, Üste Ü, Ortaya R, Alta T, Yatay dağıt Y, Dikey dağıt
//!   D) switches it, while picking too (the ribbon's methods type it);
//! - an alignment waits for its reference: a click on an object takes its
//!   box, a click elsewhere or a typed point that point, Enter the
//!   selection's box; a spread writes on Enter or a click;
//! - the preview: every selected object's outline where it would go,
//!   dashed, and the reference's line.
//!
//! The boxes and the moves are the core's (`ops::arrange`); written through
//! `cad.entities.transform`'s `arrange`, one undo step named after the
//! method; the tool leaves, the selection stays. Nothing that would not
//! move is written.

use kentos_contracts::{ArrangeMode, Transform};
use kentos_domain::Slot;
use kentos_geometry_core::block::Blocks;
use kentos_geometry_core::geom::affine::translation;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::arrange::{self, Mode};
use kentos_geometry_core::text::Font;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::{drawing_font, shape};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::modify::{GHOST_DASH, MAX_GHOSTS, Modify, Stages, ghosts, transform_selection};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Pointer, Preview, Stroke};

/// The tool's id: its command is `tool.alignDistribute`.
pub const ID: &str = "alignDistribute";
pub const LABEL: &str = "Hizala ve dağıt";

/// The methods' chips: their words and letters, in the tool's order.
const CHIPS: [(Mode, &str, &str); 8] = [
    (Mode::Left, "Sola", "S"),
    (Mode::Center, "Ortala", "O"),
    (Mode::Right, "Sağa", "A"),
    (Mode::Top, "Üste", "Ü"),
    (Mode::Middle, "Ortaya", "R"),
    (Mode::Bottom, "Alta", "T"),
    (Mode::Horizontal, "Yatay dağıt", "Y"),
    (Mode::Vertical, "Dikey dağıt", "D"),
];

/// A move shorter than this is no move: the selection is already where it would go.
const STILL: f64 = 1e-9;

/// What the preview shows for the cursor where it is now.
#[derive(Clone, Debug, Default)]
struct Shown {
    ghosts: Vec<Stroke>,
    marks: Vec<Vec2>,
    guide: Option<Stroke>,
    reference: Option<Stroke>,
}

/// The tool's stages.
#[derive(Clone, Debug)]
pub struct AlignDistribute {
    mode: Mode,
    /// The selection off locked layers and their boxes, measured once it is confirmed.
    slots: Vec<Slot>,
    boxes: Vec<Bounds>,
    blocks: Blocks,
    font: Font,
    /// The object under the cursor and its box, while an alignment waits.
    under: Option<(Slot, Bounds)>,
    shown: Shown,
}

impl Default for AlignDistribute {
    fn default() -> Self {
        Self {
            mode: Mode::Left,
            slots: Vec::new(),
            boxes: Vec::new(),
            blocks: Blocks::default(),
            font: Font::DEFAULT,
            under: None,
            shown: Shown::default(),
        }
    }
}

/// The contract's word for a mode.
fn wire(mode: Mode) -> ArrangeMode {
    match mode {
        Mode::Left => ArrangeMode::Left,
        Mode::Center => ArrangeMode::Center,
        Mode::Right => ArrangeMode::Right,
        Mode::Top => ArrangeMode::Top,
        Mode::Middle => ArrangeMode::Middle,
        Mode::Bottom => ArrangeMode::Bottom,
        Mode::Horizontal => ArrangeMode::Horizontal,
        Mode::Vertical => ArrangeMode::Vertical,
    }
}

/// What is said once written.
fn done_message(mode: Mode, n: usize) -> String {
    match mode {
        Mode::Left => format!("{n} nesne sola hizalandı."),
        Mode::Center => format!("{n} nesne ortalandı."),
        Mode::Right => format!("{n} nesne sağa hizalandı."),
        Mode::Top => format!("{n} nesne üste hizalandı."),
        Mode::Middle => format!("{n} nesne ortaya hizalandı."),
        Mode::Bottom => format!("{n} nesne alta hizalandı."),
        Mode::Horizontal => format!("{n} nesne yatay olarak eşit aralıkla dağıtıldı."),
        Mode::Vertical => format!("{n} nesne dikey olarak eşit aralıkla dağıtıldı."),
    }
}

impl AlignDistribute {
    pub fn tool() -> Modify<Self> {
        Modify::with(Self::default())
    }

    /// Started with a method (the ribbon's Ortala …): its letter typed at once.
    fn take_letter(&mut self, text: &str) -> bool {
        let key = upper_tr(js_trim(text));
        match CHIPS.iter().find(|(_, _, k)| *k == key) {
            Some((mode, _, _)) => {
                self.mode = *mode;
                self.shown = Shown::default();
                true
            }
            None => false,
        }
    }

    fn chips(&self, prompt: Prompt) -> Prompt {
        CHIPS.iter().fold(prompt, |p, (mode, label, key)| {
            p.toggle(label, key, *mode == self.mode)
        })
    }

    /// The selection's objects off locked layers and their boxes (as the command measures them).
    fn measure(&mut self, cx: &Context<'_>) {
        self.blocks = kentos_native_application::blocks::core_blocks(cx.doc.blocks());
        self.font = drawing_font(cx.doc.settings().drawing_font);
        self.slots.clear();
        self.boxes.clear();
        for &slot in cx.selection.ids() {
            let Some(e) = cx.doc.get(slot) else {
                continue;
            };
            if cx.doc.layers().is_locked(&e.base().layer_id) {
                continue;
            }
            self.slots.push(slot);
            self.boxes
                .push(arrange::object_bounds(&shape(e), &self.blocks, self.font));
        }
    }

    fn box_of(&self, slot: Slot, cx: &Context<'_>) -> Option<Bounds> {
        if let Some(i) = self.slots.iter().position(|s| *s == slot) {
            return Some(self.boxes[i]);
        }
        let e = cx.doc.get(slot)?;
        Some(arrange::object_bounds(&shape(e), &self.blocks, self.font))
    }

    /// The easting or northing an alignment meets: the box's, else the point's.
    fn at(&self, reference: Reference) -> Option<f64> {
        match reference {
            Reference::Box(b) => Some(arrange::at_of(&b, self.mode)),
            Reference::Point(p) => Some(if self.mode.eastward() { p.x } else { p.y }),
            Reference::Selection => {
                arrange::union(&self.boxes).map(|u| arrange::at_of(&u, self.mode))
            }
        }
    }

    /// Writes the moves (through the command); the flow after.
    fn write(&mut self, reference: Reference, cx: &mut Context<'_>) -> Flow {
        let at = if self.mode.aligns() {
            match self.at(reference) {
                Some(at) => Some(at),
                None => return Flow::Stay,
            }
        } else {
            None
        };
        if self.slots.len() >= 3 || self.mode.aligns() {
            let moves = arrange::moves(&self.boxes, self.mode, at);
            if !self.slots.is_empty()
                && moves
                    .iter()
                    .all(|d| d.x.abs() <= STILL && d.y.abs() <= STILL)
            {
                cx.say(
                    Level::Info,
                    if self.mode.aligns() {
                        "Seçilenler zaten hizalı; bir şey değişmedi."
                    } else {
                        "Seçilenler zaten eşit aralıklı; bir şey değişmedi."
                    },
                );
                return Flow::Exit;
            }
        }
        let transform = Transform::Arrange {
            mode: wire(self.mode),
            at,
        };
        match transform_selection(transform, false, cx) {
            Some(n) => {
                cx.say(Level::Success, done_message(self.mode, n));
                Flow::Exit
            }
            None => Flow::Stay,
        }
    }

    /// The preview for a reference (an alignment) or none (a spread).
    fn show(&mut self, reference: Option<Reference>, cx: &Context<'_>) {
        self.shown = Shown::default();
        if self.slots.is_empty() {
            return;
        }
        let at = match (self.mode.aligns(), reference) {
            (true, Some(r)) => match self.at(r) {
                Some(at) => Some(at),
                None => return,
            },
            (true, None) => return,
            (false, _) => None,
        };
        let moves = arrange::moves(&self.boxes, self.mode, at);
        let store = cx.spatial.store();
        let mut paths = Vec::new();
        for (slot, d) in self.slots.iter().zip(&moves).take(MAX_GHOSTS + 1) {
            paths.extend(store.transform_outlines(
                &[f64::from(slot.0)],
                &[translation(d.x, d.y)],
                MAX_GHOSTS,
            ));
        }
        (self.shown.ghosts, self.shown.marks) = ghosts(&paths);
        // The reference's line across the boxes, where they arrive and the reference.
        if let (Some(at), Some(u)) = (at, arrange::union(&self.boxes)) {
            let mut span = u;
            if let Some(Reference::Box(b)) = reference {
                span = arrange::union(&[u, b]).unwrap_or(u);
            }
            let pad = (span.max_x - span.min_x).max(span.max_y - span.min_y) * 0.05;
            let line = if self.mode.eastward() {
                vec![
                    Vec2::new(at, span.min_y - pad),
                    Vec2::new(at, span.max_y + pad),
                ]
            } else {
                vec![
                    Vec2::new(span.min_x - pad, at),
                    Vec2::new(span.max_x + pad, at),
                ]
            };
            self.shown.guide = Some(Stroke::solid(line, false));
        }
        if let Some(Reference::Box(b)) = reference {
            let ring = vec![
                Vec2::new(b.min_x, b.min_y),
                Vec2::new(b.max_x, b.min_y),
                Vec2::new(b.max_x, b.max_y),
                Vec2::new(b.min_x, b.max_y),
            ];
            self.shown.reference = Some(Stroke::dashed(ring, true, GHOST_DASH));
        }
    }
}

/// What an alignment meets.
#[derive(Clone, Copy, Debug)]
enum Reference {
    Box(Bounds),
    Point(Vec2),
    Selection,
}

impl Stages for AlignDistribute {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn begin(&mut self, cx: &mut Context<'_>) -> Flow {
        self.under = None;
        self.measure(cx);
        self.show(Some(Reference::Selection), cx);
        Flow::Stay
    }

    fn picking_hint(&self, prompt: Prompt) -> Prompt {
        self.chips(prompt)
    }

    fn picking_input(&mut self, text: &str, _cx: &mut Context<'_>) -> bool {
        self.take_letter(text)
    }

    fn anchor(&self) -> Option<Vec2> {
        None
    }

    fn prompt(&self, _n: usize) -> Prompt {
        let step = if self.mode.aligns() {
            "başvuru nesnesine ya da noktaya tıklayın; Enter seçimin sınırına göre hizalar"
        } else {
            "Enter ile eşit aralıkla dağıtın"
        };
        self.chips(Prompt::new(LABEL, step))
    }

    /// A click on an object takes its box; the base's point (snapped) comes otherwise.
    fn pointer(&mut self, p: &Pointer, down: bool, cx: &mut Context<'_>) -> Option<Flow> {
        if !self.mode.aligns() {
            if down {
                return Some(self.write(Reference::Selection, cx));
            }
            self.show(None, cx);
            return None;
        }
        let under = cx
            .spatial
            .pick(p.raw, cx.pick_tolerance())
            .and_then(|slot| Some((slot, self.box_of(slot, cx)?)));
        self.under = under;
        if let Some((_, b)) = under {
            if down {
                return Some(self.write(Reference::Box(b), cx));
            }
            self.show(Some(Reference::Box(b)), cx);
        } else if !down {
            self.show(Some(Reference::Point(p.world)), cx);
        }
        None
    }

    fn point(&mut self, p: Vec2, cx: &mut Context<'_>) -> Flow {
        if !self.mode.aligns() {
            return self.write(Reference::Selection, cx);
        }
        self.write(Reference::Point(p), cx)
    }

    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> Option<Flow> {
        if !self.take_letter(text) {
            return None;
        }
        self.show(Some(Reference::Selection), cx);
        Some(Flow::Stay)
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        self.write(Reference::Selection, cx)
    }

    fn preview(&self, _hover: Vec2) -> Option<kentos_geometry_core::geom::affine::Affine> {
        None
    }

    fn stage_preview(&self, _hover: Option<Vec2>, _format: &Format) -> Option<Preview> {
        let mut strokes = self.shown.ghosts.clone();
        strokes.extend(self.shown.reference.clone());
        strokes.extend(self.shown.guide.clone());
        Some(Preview {
            strokes,
            marks: self.shown.marks.clone(),
            ..Preview::default()
        })
    }
}
