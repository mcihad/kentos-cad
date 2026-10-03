//! Zincir ölçü and Baz ölçü (docs/adr/0140): the next dimensions of a run of
//! measures, both starting from a straight dimension (an aligned or a linear
//! one):
//!
//! - it is the newest the session drew ([`crate::tool::Memory::last_dimension`]),
//!   or one clicked: when none is known, the tool asks for a click on one,
//!   and Ölçü seç (S) picks another any time;
//! - **Zincir ölçü**: each click measures from where the last one ended to
//!   the point, along the same direction, its line on the same line;
//! - **Baz ölçü**: each click measures from the base's first point to the
//!   point; the lines stack, one level further out each, a level being three
//!   times the base's text height.
//!
//! Each click writes one dimension through `cad.entities.create` as its own
//! undo step (“Ekle”), a linear one along the base's direction; Enter, a
//! quick right click or Esc ends. An undone dimension takes the run back to
//! where it was. The dimensions are the shared core's
//! (`construct::continue_dimension`, `baseline_dimension`).

use kentos_contracts::{DimensionEntity, DimensionStyle, Entity, EntityGeometry};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::geom::dimension::{DimensionGeom, layout_dimension};
use kentos_geometry_core::tools::construct::{baseline_dimension, continue_dimension};

use crate::Vec2;
use crate::edge;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, constrain, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{
    Context, Cursor, Flow, Marker, MarkerShape, Pointer, Preview, Stroke, Tag, Tone, Tool,
};
use crate::{Tracking, js_trim};

/// Zincir ölçü's id: its command is `tool.dimContinue`.
pub const CONTINUE_ID: &str = "dimContinue";
/// Baz ölçü's id: its command is `tool.dimBaseline`.
pub const BASELINE_ID: &str = "dimBaseline";

/// A baseline level is this many times the base's text height.
const LEVEL_TEXT_HEIGHTS: f64 = 3.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Continue,
    Baseline,
}

/// Where the run stands: the base, where the next chain link starts, the next level.
#[derive(Clone, Debug)]
struct State {
    base: Option<DimensionGeom>,
    from: Vec2,
    level: f64,
    /// What [`crate::tool::Memory::last_dimension`] was.
    last: Option<Uuid>,
}

/// Zincir ölçü or Baz ölçü.
#[derive(Clone, Debug)]
pub struct DimensionChain {
    kind: Kind,
    state: State,
    /// A click on a dimension is wanted.
    choosing: bool,
    /// The dimensions written, newest last, each with the state before it.
    steps: Vec<(Uuid, State)>,
    hover: Option<Vec2>,
    tracking: Option<Tracking>,
    format: Format,
}

/// A straight dimension as the core lays it out: aligned (no style) or
/// linear; none for an angle, a radius or a diameter, or one that does not lay out.
fn straight(d: &DimensionEntity) -> Option<DimensionGeom> {
    let style = match d.style {
        None => None,
        Some(DimensionStyle::Linear) => Some("linear".to_owned()),
        Some(_) => return None,
    };
    let g = DimensionGeom {
        a: Vec2::new(d.a.x, d.a.y),
        b: Vec2::new(d.b.x, d.b.y),
        offset: d.offset,
        height: d.height,
        style,
        angle: d.angle,
        c: None,
        za: None,
        zb: None,
    };
    layout_dimension(&g).map(|_| g)
}

/// Objects a click can choose as the base.
fn takes(e: &Entity, _: &Document) -> bool {
    matches!(e, Entity::Dimension(d) if straight(d).is_some())
}

impl DimensionChain {
    fn with(kind: Kind) -> Self {
        Self {
            kind,
            state: State {
                base: None,
                from: Vec2::new(0.0, 0.0),
                level: 1.0,
                last: None,
            },
            choosing: true,
            steps: Vec::new(),
            hover: None,
            tracking: None,
            format: Format::default(),
        }
    }

    pub fn continued() -> Self {
        Self::with(Kind::Continue)
    }

    pub fn baseline() -> Self {
        Self::with(Kind::Baseline)
    }

    fn label(&self) -> &'static str {
        match self.kind {
            Kind::Continue => "Zincir ölçü",
            Kind::Baseline => "Baz ölçü",
        }
    }

    /// The straight dimension the object at `uid` is, if it is still there.
    fn dimension_of(uid: Uuid, cx: &Context<'_>) -> Option<DimensionGeom> {
        match cx.doc.get(cx.doc.slot_of(uid)?)? {
            Entity::Dimension(d) => straight(d),
            _ => None,
        }
    }

    /// Takes `base` as the run's base: chains start from its end, baselines
    /// from its first point at the first level.
    fn rebase(&mut self, base: DimensionGeom) {
        self.state = State {
            from: base.b,
            level: 1.0,
            base: Some(base),
            last: self.state.last,
        };
    }

    /// Dimensions of the run that are no longer in the drawing (undone,
    /// deleted) are taken back: the run stands where it did before them.
    fn sync(&mut self, cx: &mut Context<'_>) {
        self.format = cx.format();
        while let Some((uid, _)) = self.steps.last() {
            if cx.doc.slot_of(*uid).is_some() {
                break;
            }
            let Some((_, before)) = self.steps.pop() else {
                break;
            };
            cx.memory.last_dimension = before.last;
            self.state = before;
        }
    }

    /// The next dimension with the cursor at `p`, or why there is none.
    fn next(&self, p: Vec2) -> Option<DimensionGeom> {
        let base = self.state.base.as_ref()?;
        match self.kind {
            Kind::Continue => continue_dimension(base, self.state.from, p),
            Kind::Baseline => {
                baseline_dimension(base, p, self.state.level, LEVEL_TEXT_HEIGHTS * base.height)
            }
        }
    }

    /// Where the cursor's point is measured from: the ordinary anchor of ortho and polar.
    fn anchor(&self) -> Option<Vec2> {
        let base = self.state.base.as_ref()?;
        Some(match self.kind {
            Kind::Continue => self.state.from,
            Kind::Baseline => base.a,
        })
    }

    /// A click or a computed point: writes the dimension to it.
    fn point(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.sync(cx);
        if self.choosing {
            return;
        }
        let Some(g) = self.next(p) else {
            cx.say(
                Level::Warn,
                "Bu yerde ölçü oluşmuyor; nokta ölçülen noktayla çakışıyor.",
            );
            return;
        };
        let Some(layout) = layout_dimension(&g) else {
            return;
        };
        let geometry = EntityGeometry::Dimension {
            a: wire(g.a),
            b: wire(g.b),
            offset: g.offset,
            height: g.height,
            text: None,
            style: Some(DimensionStyle::Linear),
            angle: g.angle,
            c: None,
            mask: false,
            za: None,
            zb: None,
        };
        let operation = match self.kind {
            Kind::Continue => kentos_contracts::CreateOperation::DimensionChain,
            Kind::Baseline => kentos_contracts::CreateOperation::DimensionBaseline,
        };
        let Some(out) = points::write_objects(vec![geometry], Some(operation), cx) else {
            return;
        };
        let uid = out.created.first().and_then(|u| Uuid::parse_str(u).ok());
        let before = self.state.clone();
        match self.kind {
            Kind::Continue => {
                self.state = State {
                    base: Some(g),
                    from: p,
                    level: 1.0,
                    last: uid,
                };
                cx.memory.last_dimension = uid;
            }
            Kind::Baseline => self.state.level += 1.0,
        }
        if let Some(uid) = uid {
            self.steps.push((uid, before));
        }
        let value = cx
            .format()
            .dimension(layout.prefix, layout.unit, layout.value);
        cx.say(Level::Success, format!("{} eklendi: {value}", self.label()));
    }

    /// The base dimension chosen by a click.
    fn choose(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let picked = edge::pick(p, cx, takes)
            .and_then(|slot| cx.doc.get(slot))
            .and_then(|e| match e {
                Entity::Dimension(d) => straight(d),
                _ => None,
            });
        let Some(base) = picked else {
            cx.say(
                Level::Warn,
                "Hizalı ya da doğrusal bir ölçüye tıklayın; açı, yarıçap ve çap ölçüleri zincirlenmez.",
            );
            return;
        };
        self.rebase(base);
        self.choosing = false;
        cx.selection.set_hover(None);
    }
}

impl Tool for DimensionChain {
    fn id(&self) -> &'static str {
        match self.kind {
            Kind::Continue => CONTINUE_ID,
            Kind::Baseline => BASELINE_ID,
        }
    }

    fn label(&self) -> &'static str {
        self.label()
    }

    fn prompt(&self) -> Prompt {
        if self.choosing {
            return Prompt::new(
                self.label(),
                "ölçüsü sürdürülecek hizalı ya da doğrusal ölçüye tıklayın",
            );
        }
        let step = match self.kind {
            Kind::Continue => "sonraki ölçü noktasını belirtin",
            Kind::Baseline => "sonraki noktayı belirtin",
        };
        let f = self.format;
        let prompt = Prompt::new(self.label(), step);
        let prompt = match (self.kind, &self.state.base) {
            (Kind::Baseline, Some(base)) => prompt.note(format!(
                "kat {}",
                f.length(LEVEL_TEXT_HEIGHTS * base.height)
            )),
            _ => prompt,
        };
        prompt.then().option("Ölçü seç", "S")
    }

    fn point_count(&self) -> usize {
        0
    }

    fn cursor(&self) -> Cursor {
        if self.choosing {
            Cursor::Pick
        } else {
            Cursor::Cross
        }
    }

    /// The base is remembered, or a click is asked for.
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.format = cx.format();
        let base = cx
            .memory
            .last_dimension
            .and_then(|uid| Self::dimension_of(uid, cx));
        self.state.last = cx.memory.last_dimension;
        self.choosing = base.is_none();
        if let Some(base) = base {
            self.rebase(base);
        }
        Flow::Stay
    }

    /// The base is picked by its edge: no snaps then (the web's `EdgePickTool`).
    fn snaps(&self) -> bool {
        !self.choosing
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.anchor()
    }

    fn accepts_points(&self) -> bool {
        !self.choosing
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        if self.choosing {
            return false;
        }
        self.point(p, cx);
        true
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.sync(cx);
        if self.choosing {
            edge::hover(p, cx, takes);
            self.hover = None;
            return;
        }
        let (point, tracking) = constrain(self.anchor(), p, cx);
        self.hover = Some(point);
        self.tracking = tracking;
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.sync(cx);
        if self.choosing {
            return self.choose(p, cx);
        }
        let (point, _) = constrain(self.anchor(), p, cx);
        self.point(point, cx);
    }

    /// S picks another base; anything else is a point, as everywhere.
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        self.sync(cx);
        if upper_tr(js_trim(text)) == "S" {
            self.choosing = true;
            self.hover = None;
            return true;
        }
        if self.choosing {
            return false;
        }
        let Some(p) = cx.typed_point(text, self.anchor(), self.hover) else {
            return false;
        };
        self.point(p, cx);
        true
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    /// Esc leaves the choosing of another base for the base the run has; else it ends.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.choosing && self.state.base.is_some() {
            self.choosing = false;
            cx.selection.set_hover(None);
            return true;
        }
        false
    }

    /// Ctrl+Z undoes the drawing; the run follows it (see [`Self::sync`]).
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, format: &Format) -> Preview {
        let (Some(hover), false) = (self.hover, self.choosing) else {
            return Preview::default();
        };
        let mut preview = Preview {
            tracking: self.tracking,
            markers: self
                .anchor()
                .map(|at| Marker {
                    at,
                    shape: MarkerShape::Ring(6.0),
                    tone: Tone::Snap,
                })
                .into_iter()
                .collect(),
            ..Preview::default()
        };
        if let Some(l) = self.next(hover).and_then(|g| layout_dimension(&g)) {
            preview.strokes = l
                .lines
                .iter()
                .map(|&[p, q]| Stroke::solid(vec![p, q], false))
                .collect();
            preview.tag = Some(Tag {
                at: hover,
                lines: vec![format.dimension(l.prefix, l.unit, l.value)],
            });
        }
        preview
    }
}
