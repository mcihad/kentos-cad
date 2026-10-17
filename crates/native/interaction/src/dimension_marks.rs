//! Merkez işareti, Eksen çizgisi and Kot (docs/adr/0322): the marks that follow their objects, each written by
//! `cad.dimensions.create`'s `centerMark`, `centerLine` or `elevation` as its own undo step (“Ekle”), on the active
//! layer. The preview is the command's own computation (`measured_in`).
//!
//! - **Merkez işareti** (AutoCAD's CENTERMARK): a click on a circle, an arc or a path's arc segment puts the cross
//!   at its centre; the tool stays for the next.
//! - **Eksen çizgisi** (CENTERLINE): a click on one straight edge, then on another, puts the axis between them.
//! - **Kot**: a click on a point (snapped to a vertex, it follows it and its elevation), then where the value goes;
//!   a point with no elevation asks for one first.
//!
//! Enter, a quick right click or Esc ends; Esc first drops a half-made mark.

use kentos_contracts::{
    DimensionMeasure, DimensionsCreate, EdgeNear, Entity, MeasuredPoint, NewLook,
};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::geom::dimension::layout_dimension;
use kentos_geometry_core::store::snap::SnapHit;
use kentos_native_application::dimension_create::{dimensions_create, measured_in};
use kentos_native_application::geometry::contract_look;
use kentos_native_application::{ExecutionContext, codes};

use crate::Vec2;
use crate::edge;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, constrain, wire};
use crate::prompt::Prompt;
use crate::tool::{Context, Cursor, Flow, Pointer, Preview, Stroke, Tag, Tool};
use crate::{Tracking, js_trim};

/// Merkez işareti's id: its command is `tool.centerMark`.
pub const CENTER_MARK_ID: &str = "centerMark";
/// Eksen çizgisi's id: `tool.dimCenterLine`.
pub const CENTER_LINE_ID: &str = "dimCenterLine";
/// Kot's id: `tool.dimElevation`.
pub const ELEVATION_ID: &str = "dimElevation";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    CenterMark,
    CenterLine,
    Elevation,
}

/// A curve a centre mark goes on: a circle, an arc, a path with an arc segment.
fn curved(e: &Entity, doc: &Document) -> bool {
    let arcs = |b: &Option<Vec<f64>>| b.as_ref().is_some_and(|b| b.iter().any(|x| *x != 0.0));
    edge::unlocked(e, doc)
        && match e {
            Entity::Circle(_) | Entity::Arc(_) => true,
            Entity::Polyline(p) | Entity::Polygon(p) => arcs(&p.bulges),
            _ => false,
        }
}

/// An object with a straight edge an axis goes along.
fn straight(e: &Entity, doc: &Document) -> bool {
    edge::unlocked(e, doc)
        && matches!(
            e,
            Entity::Line(_) | Entity::Polyline(_) | Entity::Polygon(_)
        )
}

/// Merkez işareti, Eksen çizgisi or Kot.
#[derive(Clone, Debug)]
pub struct DimensionMarks {
    kind: Kind,
    /// Eksen çizgisi's first edge.
    first: Option<EdgeNear>,
    /// Kot's point.
    point: Option<MeasuredPoint>,
    /// Kot's elevation typed for a point that has none.
    z: Option<f64>,
    /// Kot is waiting for an elevation to be typed.
    asking_z: bool,
    hover: Option<Vec2>,
    /// What the command would write at the cursor: its lines and its value.
    shown: Option<(Vec<[Vec2; 2]>, Option<String>)>,
    tracking: Option<Tracking>,
    format: Format,
}

impl DimensionMarks {
    fn with(kind: Kind) -> Self {
        Self {
            kind,
            first: None,
            point: None,
            z: None,
            asking_z: false,
            hover: None,
            shown: None,
            tracking: None,
            format: Format::default(),
        }
    }

    pub fn center_mark() -> Self {
        Self::with(Kind::CenterMark)
    }

    pub fn center_line() -> Self {
        Self::with(Kind::CenterLine)
    }

    pub fn elevation() -> Self {
        Self::with(Kind::Elevation)
    }

    fn name(&self) -> &'static str {
        match self.kind {
            Kind::CenterMark => "Merkez işareti",
            Kind::CenterLine => "Eksen çizgisi",
            Kind::Elevation => "Kot",
        }
    }

    /// Whether a click picks an object's edge (not a point).
    fn picks_edges(&self) -> bool {
        self.kind != Kind::Elevation
    }

    /// The command's input for `measure` at `at`, on the active layer.
    fn input(measure: DimensionMeasure, at: Vec2, cx: &Context<'_>) -> DimensionsCreate {
        DimensionsCreate {
            measure,
            at: wire(at),
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
        }
    }

    /// What a click on the edge under `p` (or with the cursor at `p`) would write.
    fn request(&self, p: &Pointer, at: Vec2, cx: &Context<'_>) -> Option<DimensionsCreate> {
        let near = |slot| EdgeNear {
            uid: edge::uid(cx.doc, slot),
            near: wire(p.raw),
        };
        let measure = match self.kind {
            Kind::CenterMark => DimensionMeasure::CenterMark {
                of: near(edge::pick(p, cx, curved)?),
            },
            Kind::CenterLine => DimensionMeasure::CenterLine {
                first: self.first.clone()?,
                second: near(edge::pick(p, cx, straight)?),
            },
            Kind::Elevation => DimensionMeasure::Elevation {
                a: self.point.clone()?,
                z: self.z,
            },
        };
        Some(Self::input(measure, at, cx))
    }

    /// The command's computation of `input` as the preview shows it.
    fn preview_of(
        input: &DimensionsCreate,
        cx: &Context<'_>,
    ) -> Option<(Vec<[Vec2; 2]>, Option<String>)> {
        let m = measured_in(cx.doc, input).ok()?;
        let l = layout_dimension(&m.geom)?;
        let value = (l.unit != "none").then(|| {
            cx.format().dimension_in(
                l.prefix,
                l.unit,
                l.value,
                &contract_look(m.geom.look.clone()),
            )
        });
        Some((l.lines, value))
    }

    /// Writes `input`; the mark's words, or the refusal said.
    fn write(&mut self, input: DimensionsCreate, cx: &mut Context<'_>) -> bool {
        let result = dimensions_create::execute(&mut ExecutionContext::new(cx.doc), input);
        if points::written(result, cx).is_none() {
            return false;
        }
        cx.say(Level::Success, format!("{} eklendi", self.name()));
        true
    }

    /// Kot's point: where it was clicked, bound to the place it snapped to.
    fn take_point(&mut self, p: Vec2, snap: Option<SnapHit>, cx: &mut Context<'_>) {
        let point = match crate::dimension::snapped_place(snap, p, cx) {
            Some(r) => MeasuredPoint::Place {
                uid: Uuid::from_bytes(r.uid.0).to_string(),
                place: r.place,
                path: r.path,
            },
            None => MeasuredPoint::At { at: wire(p) },
        };
        let probe = Self::input(
            DimensionMeasure::Elevation {
                a: point.clone(),
                z: None,
            },
            p,
            cx,
        );
        self.point = Some(point);
        self.z = None;
        // A point with no elevation of its own asks for one.
        if let Err(e) = measured_in(cx.doc, &probe) {
            if e.code == codes::MISSING_ELEVATION {
                self.asking_z = true;
            } else {
                cx.say(Level::Warn, e.message);
                self.point = None;
            }
        }
    }

    fn reset(&mut self) {
        self.first = None;
        self.point = None;
        self.z = None;
        self.asking_z = false;
        self.shown = None;
    }
}

impl Tool for DimensionMarks {
    fn id(&self) -> &'static str {
        match self.kind {
            Kind::CenterMark => CENTER_MARK_ID,
            Kind::CenterLine => CENTER_LINE_ID,
            Kind::Elevation => ELEVATION_ID,
        }
    }

    fn label(&self) -> &'static str {
        self.name()
    }

    fn prompt(&self) -> Prompt {
        let step = match self.kind {
            Kind::CenterMark => "daireye, yaya ya da yay parçasına tıklayın",
            Kind::CenterLine if self.first.is_none() => "birinci düz kenara tıklayın",
            Kind::CenterLine => "ikinci düz kenara tıklayın",
            Kind::Elevation if self.point.is_none() => "kotu yazılacak noktayı belirtin",
            Kind::Elevation if self.asking_z => "noktanın kotunu yazın",
            Kind::Elevation => "değerin yerini belirtin",
        };
        Prompt::new(self.name(), step)
    }

    fn point_count(&self) -> usize {
        usize::from(self.first.is_some() || self.point.is_some())
    }

    fn cursor(&self) -> Cursor {
        if self.picks_edges() {
            Cursor::Pick
        } else {
            Cursor::Cross
        }
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.format = cx.format();
        Flow::Stay
    }

    /// Edges are picked: no snaps then; Kot's point and place snap.
    fn snaps(&self) -> bool {
        !self.picks_edges()
    }

    fn snap_from(&self) -> Option<Vec2> {
        match &self.point {
            Some(MeasuredPoint::At { at }) => Some(Vec2::new(at.x, at.y)),
            _ => None,
        }
    }

    fn accepts_points(&self) -> bool {
        self.kind == Kind::Elevation && !self.asking_z
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        if !self.accepts_points() {
            return false;
        }
        if self.point.is_none() {
            self.take_point(p, None, cx);
            return true;
        }
        let input = Self::input(
            DimensionMeasure::Elevation {
                a: self
                    .point
                    .clone()
                    .unwrap_or(MeasuredPoint::At { at: wire(p) }),
                z: self.z,
            },
            p,
            cx,
        );
        if self.write(input, cx) {
            self.reset();
        }
        true
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.format = cx.format();
        match self.kind {
            Kind::CenterMark | Kind::CenterLine => {
                let filter = if self.kind == Kind::CenterMark {
                    curved
                } else {
                    straight
                };
                edge::hover(p, cx, filter);
                self.hover = Some(p.raw);
                let ready = self.kind == Kind::CenterMark || self.first.is_some();
                self.shown = ready
                    .then(|| self.request(p, p.raw, cx))
                    .flatten()
                    .and_then(|input| Self::preview_of(&input, cx));
            }
            Kind::Elevation => {
                let (point, tracking) = constrain(self.snap_from(), p, cx);
                self.hover = Some(point);
                self.tracking = tracking;
                self.shown = (self.point.is_some() && !self.asking_z)
                    .then(|| self.request(p, point, cx))
                    .flatten()
                    .and_then(|input| Self::preview_of(&input, cx));
            }
        }
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        match self.kind {
            Kind::CenterMark => {
                if edge::pick_said(p, cx, curved).is_none() {
                    cx.say(
                        Level::Warn,
                        "Daireye, yaya ya da çoklu çizginin yay parçasına tıklayın.",
                    );
                    return;
                }
                if let Some(input) = self.request(p, p.raw, cx) {
                    self.write(input, cx);
                }
            }
            Kind::CenterLine => {
                let Some(slot) = edge::pick_said(p, cx, straight) else {
                    cx.say(
                        Level::Warn,
                        "Çizgiye ya da çoklu çizginin, alanın düz kenarına tıklayın.",
                    );
                    return;
                };
                if self.first.is_none() {
                    self.first = Some(EdgeNear {
                        uid: edge::uid(cx.doc, slot),
                        near: wire(p.raw),
                    });
                    return;
                }
                if let Some(input) = self.request(p, p.raw, cx)
                    && self.write(input, cx)
                {
                    self.reset();
                }
            }
            Kind::Elevation => {
                if self.asking_z {
                    return;
                }
                let (point, _) = constrain(self.snap_from(), p, cx);
                if self.point.is_none() {
                    return self.take_point(point, p.snap, cx);
                }
                if let Some(input) = self.request(p, point, cx)
                    && self.write(input, cx)
                {
                    self.reset();
                }
            }
        }
    }

    /// Kot's elevation when asked for; else a typed point.
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.asking_z {
            let Some(z) = cx.typed_length(js_trim(text)) else {
                cx.say(Level::Warn, "Kotu bir sayıyla yazın.");
                return true;
            };
            self.z = Some(z);
            self.asking_z = false;
            return true;
        }
        if !self.accepts_points() {
            return false;
        }
        let Some(p) = cx.typed_point(text, self.snap_from(), self.hover) else {
            return false;
        };
        self.accept_point(p, cx)
    }

    fn takes_words(&self) -> bool {
        false
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    /// Esc drops a half-made mark first; else it ends.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.first.is_some() || self.point.is_some() {
            self.reset();
            cx.selection.set_hover(None);
            return true;
        }
        false
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        let Some((lines, value)) = &self.shown else {
            return Preview {
                tracking: self.tracking,
                ..Preview::default()
            };
        };
        Preview {
            tracking: self.tracking,
            strokes: lines
                .iter()
                .map(|&[p, q]| Stroke::solid(vec![p, q], false))
                .collect(),
            tag: match (value, self.hover) {
                (Some(v), Some(at)) => Some(Tag {
                    at,
                    lines: vec![v.clone()],
                }),
                _ => None,
            },
            ..Preview::default()
        }
    }
}
