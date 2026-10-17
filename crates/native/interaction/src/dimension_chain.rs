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
//! Each click writes one dimension through `cad.dimensions.create`'s `continue`
//! or `baseline` (docs/adr/0320 §3) as its own undo step (“Ekle”), a linear one
//! along the base's direction, in the base's style and look (AutoCAD's
//! DIMCONTINUEMODE 1, docs/adr/0183 §4), the last one the base of the next;
//! its first point follows what its base's does, its second the place it
//! snapped to. Enter, a quick right click or Esc ends. An undone dimension
//! takes the run back to where it was. The preview is the command's own
//! computation (`construct::continue_dimension`, `baseline_dimension`).

use kentos_contracts::{DimensionMeasure, DimensionsCreate, Entity, MeasuredPoint, NewLook};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::geom::dimension::{DimensionGeom, layout_dimension};
use kentos_geometry_core::store::snap::SnapHit;
use kentos_geometry_core::tools::construct::{baseline_dimension, continue_dimension};
use kentos_native_application::ExecutionContext;
use kentos_native_application::dimension_create::{LEVEL_TEXT_HEIGHTS, dimensions_create, straight};
use kentos_native_application::geometry::contract_look;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Continue,
    Baseline,
}

/// Where the run stands: the base (the dimension the next goes on from) and its persistent id.
#[derive(Clone, Debug)]
struct State {
    base: Option<(Uuid, DimensionGeom)>,
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
    fn dimension_of(uid: Uuid, doc: &Document) -> Option<DimensionGeom> {
        match doc.get(doc.slot_of(uid)?)? {
            Entity::Dimension(d) => straight(d),
            _ => None,
        }
    }

    /// Takes the dimension at `uid` as the run's base: chains go on from its end, baselines from its first point a
    /// level further out.
    fn rebase(&mut self, uid: Uuid, base: DimensionGeom) {
        self.state = State {
            base: Some((uid, base)),
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

    /// The next dimension with the cursor at `p` (the command's computation), or why there is none.
    fn next(&self, p: Vec2) -> Option<DimensionGeom> {
        let (_, base) = self.state.base.as_ref()?;
        match self.kind {
            Kind::Continue => continue_dimension(base, base.b, p),
            Kind::Baseline => baseline_dimension(base, p, 1.0, LEVEL_TEXT_HEIGHTS * base.height),
        }
    }

    /// Where the cursor's point is measured from: the ordinary anchor of ortho and polar.
    fn anchor(&self) -> Option<Vec2> {
        let (_, base) = self.state.base.as_ref()?;
        Some(match self.kind {
            Kind::Continue => base.b,
            Kind::Baseline => base.a,
        })
    }

    /// The command's input for the next dimension to `p`: the place it snapped to, if any, on the active layer
    /// (the base's style and look).
    fn request(&self, p: Vec2, snap: Option<SnapHit>, cx: &Context<'_>) -> Option<DimensionsCreate> {
        let (uid, _) = self.state.base.as_ref()?;
        let to = match crate::dimension::snapped_place(snap, p, cx) {
            Some(r) => MeasuredPoint::Place {
                uid: Uuid::from_bytes(r.uid.0).to_string(),
                place: r.place,
                path: r.path,
            },
            None => MeasuredPoint::At { at: wire(p) },
        };
        let base = uid.to_string();
        let measure = match self.kind {
            Kind::Continue => DimensionMeasure::Continue { base, to },
            Kind::Baseline => DimensionMeasure::Baseline { base, to },
        };
        Some(DimensionsCreate {
            measure,
            at: wire(p),
            offset: None,
            layer_id: cx.doc.layers().active().to_owned(),
            dimension_style: None,
            height: None,
            mask: None,
            look: NewLook {
                color: cx.draft.color_text(),
                line_weight: cx.draft.line_weight,
                attrs: cx.template_attrs(None),
                label: cx.template_label(),
                symbol: cx.template_symbol(),
            },
            expected_revision: None,
        })
    }

    /// A click or a computed point (with the snap it stood on): writes the dimension to it.
    fn point(&mut self, p: Vec2, snap: Option<SnapHit>, cx: &mut Context<'_>) {
        self.sync(cx);
        if self.choosing {
            return;
        }
        let Some(layout) = self.next(p).and_then(|g| layout_dimension(&g)) else {
            cx.say(
                Level::Warn,
                "Bu yerde ölçü oluşmuyor; nokta ölçülen noktayla çakışıyor.",
            );
            return;
        };
        let Some(input) = self.request(p, snap, cx) else {
            return;
        };
        let result = dimensions_create::execute(&mut ExecutionContext::new(cx.doc), input);
        let Some(out) = points::written(result, cx) else {
            return;
        };
        let Some(uid) = out.created.first().and_then(|u| Uuid::parse_str(u).ok()) else {
            return;
        };
        let look = match cx.doc.get(cx.doc.slot_of(uid).unwrap_or(kentos_domain::Slot(u32::MAX))) {
            Some(Entity::Dimension(d)) => d.look.clone(),
            _ => Default::default(),
        };
        let before = self.state.clone();
        if let Some(g) = Self::dimension_of(uid, cx.doc) {
            self.state = State {
                base: Some((uid, g)),
                last: Some(uid),
            };
        }
        if self.kind == Kind::Continue {
            cx.memory.last_dimension = Some(uid);
        }
        self.steps.push((uid, before));
        let value = cx
            .format()
            .dimension_in(layout.prefix, layout.unit, layout.value, &look);
        cx.say(Level::Success, format!("{} eklendi: {value}", self.label()));
    }

    /// The base dimension chosen by a click.
    fn choose(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let picked = edge::pick(p, cx, takes).and_then(|slot| {
            let uid = cx.doc.uid(slot)?;
            Some((uid, Self::dimension_of(uid, cx.doc)?))
        });
        let Some((uid, base)) = picked else {
            cx.say(
                Level::Warn,
                "Hizalı ya da doğrusal bir ölçüye tıklayın; açı, yarıçap ve çap ölçüleri zincirlenmez.",
            );
            return;
        };
        self.rebase(uid, base);
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
                f.length(LEVEL_TEXT_HEIGHTS * base.1.height)
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
            .and_then(|uid| Some((uid, Self::dimension_of(uid, cx.doc)?)));
        self.state.last = cx.memory.last_dimension;
        self.choosing = base.is_none();
        if let Some((uid, base)) = base {
            self.rebase(uid, base);
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
        self.point(p, None, cx);
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
        self.point(point, p.snap, cx);
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
        self.point(p, None, cx);
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
        if let Some((g, l)) = self
            .next(hover)
            .and_then(|g| layout_dimension(&g).map(|l| (g, l)))
        {
            preview.strokes = l
                .lines
                .iter()
                .map(|&[p, q]| Stroke::solid(vec![p, q], false))
                // The base's filled arrowheads and dots, outlined (docs/adr/0183 §3).
                .chain(
                    l.fills
                        .iter()
                        .flatten()
                        .map(|ring| Stroke::solid(ring.clone(), true)),
                )
                .collect();
            preview.tag = Some(Tag {
                at: hover,
                lines: vec![format.dimension_in(l.prefix, l.unit, l.value, &contract_look(g.look))],
            });
        }
        preview
    }
}
