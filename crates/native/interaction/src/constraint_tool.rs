//! Kısıt (docs/adr/0335; AutoCAD's GEOMCONSTRAINT and DIMCONSTRAINT): a constraint added by clicking the parts it
//! names. The kind is chosen first (the ribbon's methods, or its letters typed); each click takes the part of an
//! object under the pointer the kind wants: a point (an end, a vertex, a centre, a point object), a straight edge or
//! a circle. Before the last click the drawing as the constraint would solve it follows the pointer, dashed, with the
//! solver's word; the last click writes it through `cad.constraints.edit` (one undo step “Kısıtlar”). A dimension
//! asks its value after its parts: Enter keeps what the drawing measures, a number or an expression drives it,
//! Başvuru (B) makes it only measure. The tool waits for the next constraint of the same kind.

use kentos_contracts::{
    CadElement, CommandResult, ConstraintEdit, ConstraintRule, ConstraintSense, ConstraintsEdit,
    DistanceAxis, ElementRef, Entity, EntityId, NewConstraint, SolveStatus, SubPlace,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::ops::subref::{EdgeKind, Place, edge_line, edge_place_near, place_near};
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, constraints_edit};

use crate::edge::{self, Outline};
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, Label, Pointer, Preview, Stroke, Tag, Tone, Tool};
use crate::{Vec2, js_trim};

/// The tool's id: its command is `tool.constraint`.
pub const ID: &str = "constraint";
pub const LABEL: &str = "Kısıt";

/// A constraint's kind, as the ribbon's methods name them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    #[default]
    Coincident,
    Horizontal,
    Vertical,
    Parallel,
    Perpendicular,
    Tangent,
    Concentric,
    Collinear,
    Equal,
    Symmetric,
    Fix,
    Distance,
    HorizontalDistance,
    VerticalDistance,
    Angle,
    Radius,
    Diameter,
}

/// The kinds with their names and the letters that choose them.
pub const KINDS: [(Kind, &str, &str); 17] = [
    (Kind::Coincident, "Çakışık", "ÇK"),
    (Kind::Horizontal, "Yatay", "Y"),
    (Kind::Vertical, "Düşey", "D"),
    (Kind::Parallel, "Paralel", "P"),
    (Kind::Perpendicular, "Dik", "DK"),
    (Kind::Tangent, "Teğet", "T"),
    (Kind::Concentric, "Eş merkezli", "EM"),
    (Kind::Collinear, "Doğrudaş", "DO"),
    (Kind::Equal, "Eşit", "E"),
    (Kind::Symmetric, "Simetrik", "S"),
    (Kind::Fix, "Sabit", "SB"),
    (Kind::Distance, "Uzaklık", "U"),
    (Kind::HorizontalDistance, "Yatay uzaklık", "YU"),
    (Kind::VerticalDistance, "Düşey uzaklık", "DU"),
    (Kind::Angle, "Açı", "A"),
    (Kind::Radius, "Yarıçap", "R"),
    (Kind::Diameter, "Çap", "C"),
];

fn name(kind: Kind) -> &'static str {
    KINDS
        .iter()
        .find(|(k, ..)| *k == kind)
        .map_or("", |(_, n, _)| n)
}

/// What a click may take.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Want {
    Point,
    Segment,
    Circle,
    /// A point, else the object's edge or circle (a point on a curve; a fix).
    PointOrCurve,
    /// A segment or a circle (tangent's first, equal's).
    SegmentOrCircle,
    /// A segment, or a point when a second point follows (horizontal, vertical).
    SegmentOrPoint,
    /// A point or a segment (a distance's second).
    PointOrSegment,
}

/// The parts a kind asks for, in order; with `first` the part the first click gave (a horizontal of two points).
fn wants(kind: Kind, first: Option<&CadElement>) -> Vec<Want> {
    use Want::*;
    match kind {
        Kind::Coincident => vec![Point, PointOrCurve],
        Kind::Horizontal | Kind::Vertical => match first {
            Some(CadElement::Point { .. }) => vec![SegmentOrPoint, Point],
            _ => vec![SegmentOrPoint],
        },
        Kind::Parallel | Kind::Perpendicular | Kind::Collinear | Kind::Angle => {
            vec![Segment, Segment]
        }
        Kind::Tangent => vec![SegmentOrCircle, Circle],
        Kind::Concentric => vec![Circle, Circle],
        Kind::Equal => match first {
            Some(CadElement::Circle) => vec![SegmentOrCircle, Circle],
            Some(_) => vec![SegmentOrCircle, Segment],
            None => vec![SegmentOrCircle, SegmentOrCircle],
        },
        Kind::Symmetric => vec![Point, Point, Segment],
        Kind::Fix => vec![PointOrCurve],
        Kind::Distance => vec![Point, PointOrSegment],
        Kind::HorizontalDistance | Kind::VerticalDistance => vec![Point, Point],
        Kind::Radius | Kind::Diameter => vec![Circle],
    }
}

fn dimensional(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Distance
            | Kind::HorizontalDistance
            | Kind::VerticalDistance
            | Kind::Angle
            | Kind::Radius
            | Kind::Diameter
    )
}

/// A part picked: its object, the part, and where it shows.
#[derive(Clone, Debug)]
struct Picked {
    slot: Slot,
    element: CadElement,
    at: Vec2,
    /// A segment's ends, for its mark.
    ends: Option<(Vec2, Vec2)>,
}

fn point_place(p: Place) -> Option<SubPlace> {
    Some(match p {
        Place::Start => SubPlace::Start,
        Place::End => SubPlace::End,
        Place::Center => SubPlace::Center,
        Place::Node { part } => SubPlace::Node { part: part as u32 },
        Place::Vertex { part, ring, index } => SubPlace::Vertex {
            part: part as u32,
            ring: ring as u32,
            index: index as u32,
        },
        _ => return None,
    })
}

/// The part of an object under the pointer that `want` takes.
fn part_at(p: &Pointer, cx: &Context<'_>, want: Want) -> Option<Picked> {
    let slot = edge::pick(p, cx, |_, _| true)?;
    let e = cx.doc.get(slot)?;
    let core = edge::core(e);
    let tol = cx.pick_tolerance();
    let at = p.raw;
    let round = matches!(e, Entity::Circle(_) | Entity::Arc(_));
    let point = || {
        place_near(&core, at, tol * 1.5).and_then(|(place, _)| {
            let sub = point_place(place)?;
            let q = kentos_geometry_core::ops::subref::point(&core, place)?;
            Some(Picked {
                slot,
                element: CadElement::Point { place: sub },
                at: q,
                ends: None,
            })
        })
    };
    let segment = || {
        let place = edge_place_near(&core, EdgeKind::Straight, at)?;
        let (a, b) = edge_line(&core, place)?;
        let (part, ring, index) = match place {
            Place::Along {
                part, ring, index, ..
            }
            | Place::Mid { part, ring, index } => (part, ring, index),
            _ => return None,
        };
        let element = CadElement::Segment {
            part: part as u32,
            ring: ring as u32,
            index: index as u32,
        };
        Some(Picked {
            slot,
            element,
            at,
            ends: Some((a, b)),
        })
    };
    let circle = || {
        round.then_some(Picked {
            slot,
            element: CadElement::Circle,
            at,
            ends: None,
        })
    };
    match want {
        Want::Point => point(),
        Want::Segment => segment(),
        Want::Circle => circle(),
        Want::PointOrCurve => point().or_else(segment).or_else(circle),
        Want::SegmentOrCircle => circle().or_else(segment),
        Want::SegmentOrPoint => point().or_else(segment),
        Want::PointOrSegment => point().or_else(segment),
    }
}

/// What the click asks for, as the prompt says it.
fn want_words(w: Want) -> &'static str {
    match w {
        Want::Point => "bir noktaya (uç, köşe, merkez)",
        Want::Segment => "bir düz kenara",
        Want::Circle => "bir daireye ya da yaya",
        Want::PointOrCurve => "bir noktaya ya da kenara",
        Want::SegmentOrCircle => "bir kenara ya da daireye",
        Want::SegmentOrPoint => "bir kenara ya da noktaya",
        Want::PointOrSegment => "bir noktaya ya da kenara",
    }
}

#[derive(Default)]
pub struct ConstraintTool {
    kind: Kind,
    picked: Vec<Picked>,
    /// A dimension's parts are all given: its value is asked.
    asking: bool,
    reference: bool,
    hover: Option<Picked>,
    cursor: Option<Vec2>,
    drawn: Preview,
}

impl ConstraintTool {
    pub fn new() -> Self {
        Self::default()
    }

    fn uid(doc: &Document, slot: Slot) -> Option<EntityId> {
        doc.uid(slot).map(|u| EntityId(u.into_bytes()))
    }

    /// The rule the picked parts make (with `last` as the last part when given).
    fn rule(&self, doc: &Document, last: Option<&Picked>) -> Option<ConstraintRule> {
        let parts: Vec<ElementRef> = self
            .picked
            .iter()
            .chain(last)
            .map(|p| {
                Some(ElementRef {
                    uid: Self::uid(doc, p.slot)?,
                    element: p.element,
                })
            })
            .collect::<Option<_>>()?;
        let a = *parts.first()?;
        let b = || parts.get(1).copied();
        let none = String::new;
        Some(match self.kind {
            Kind::Coincident => ConstraintRule::Coincident { a, b: b()? },
            Kind::Horizontal => ConstraintRule::Horizontal { a, b: b() },
            Kind::Vertical => ConstraintRule::Vertical { a, b: b() },
            Kind::Parallel => ConstraintRule::Parallel { a, b: b()? },
            Kind::Perpendicular => ConstraintRule::Perpendicular { a, b: b()? },
            Kind::Tangent => ConstraintRule::Tangent {
                a,
                b: b()?,
                inside: false,
                left: false,
            },
            Kind::Concentric => ConstraintRule::Concentric { a, b: b()? },
            Kind::Collinear => ConstraintRule::Collinear { a, b: b()? },
            Kind::Equal => ConstraintRule::Equal { a, b: b()? },
            Kind::Symmetric => ConstraintRule::Symmetric {
                a,
                b: b()?,
                axis: *parts.get(2)?,
            },
            Kind::Fix => ConstraintRule::Fix { a },
            Kind::Distance => ConstraintRule::Distance {
                a,
                b: b()?,
                axis: DistanceAxis::Aligned,
                left: false,
                parameter: none(),
            },
            Kind::HorizontalDistance => ConstraintRule::Distance {
                a,
                b: b()?,
                axis: DistanceAxis::Horizontal,
                left: false,
                parameter: none(),
            },
            Kind::VerticalDistance => ConstraintRule::Distance {
                a,
                b: b()?,
                axis: DistanceAxis::Vertical,
                left: false,
                parameter: none(),
            },
            Kind::Angle => ConstraintRule::Angle {
                a,
                b: b()?,
                reverse_a: false,
                reverse_b: false,
                parameter: none(),
            },
            Kind::Radius => ConstraintRule::Radius {
                a,
                parameter: none(),
            },
            Kind::Diameter => ConstraintRule::Diameter {
                a,
                parameter: none(),
            },
        })
    }

    fn input_of(rule: ConstraintRule, value: Option<String>, reference: bool) -> ConstraintsEdit {
        // AutoCAD's rule: the object picked first stays, the other is adjusted to it (when there are two).
        let objects: Vec<EntityId> =
            rule.elements()
                .iter()
                .map(|e| e.uid)
                .fold(Vec::new(), |mut out, u| {
                    if !out.contains(&u) {
                        out.push(u);
                    }
                    out
                });
        let hold = if objects.len() > 1
            && rule.parameter().is_none()
            && !matches!(rule, ConstraintRule::Fix { .. })
        {
            vec![kentos_domain::Uuid::from_bytes(objects[0].0).to_string()]
        } else {
            Vec::new()
        };
        ConstraintsEdit {
            edit: ConstraintEdit::Add {
                constraints: vec![NewConstraint {
                    rule,
                    sense: ConstraintSense::Drawing,
                    value,
                    reference,
                }],
            },
            hold,
            allow_flip: false,
            expected_revision: None,
        }
    }

    /// The parts still asked for.
    fn wanted(&self) -> Vec<Want> {
        wants(self.kind, self.picked.first().map(|p| &p.element))
    }

    fn complete_with(&self, extra: usize) -> bool {
        self.picked.len() + extra >= self.wanted().len()
    }

    fn reset(&mut self) {
        self.picked.clear();
        self.asking = false;
        self.hover = None;
        self.drawn = Preview::default();
    }

    /// Writes the constraint; the solver's word or its refusal said.
    fn write(&mut self, value: Option<String>, cx: &mut Context<'_>) {
        let Some(rule) = self.rule(cx.doc, None) else {
            self.reset();
            return;
        };
        let input = Self::input_of(rule, value, self.reference && dimensional(self.kind));
        let result = constraints_edit::execute(&mut ExecutionContext::new(cx.doc), input);
        if let Some(out) = points::written(result, cx) {
            let moved = out.changed.len();
            cx.say(
                Level::Success,
                format!(
                    "{}: {}; {}",
                    name(self.kind),
                    moved_words(moved),
                    status_words(&out.diagnosis.status, out.diagnosis.dof)
                ),
            );
        }
        self.reset();
    }

    /// The preview of the constraint the hovered part would complete: the plan's moved objects, dashed.
    fn show(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        let Some(h) = self.hover.clone() else {
            return;
        };
        let mut drawn = Preview::default();
        for p in self.picked.iter().chain(std::iter::once(&h)) {
            mark(&mut drawn, p);
        }
        if self.complete_with(1)
            && !dimensional(self.kind)
            && let Some(rule) = self.rule(cx.doc, Some(&h))
        {
            let input = Self::input_of(rule, None, false);
            let at = self.cursor.unwrap_or(h.at);
            match constraints_edit::plan_in(cx.doc, &input) {
                CommandResult::Completed { output, .. } => {
                    for e in &output.changed {
                        let o = Outline::of(&shape(e), Some([6.0, 4.0]), 2.0, Tone::Accent);
                        drawn.strokes.extend(o.strokes);
                        drawn.marks.extend(o.marks);
                    }
                    drawn.tag = Some(Tag {
                        at,
                        lines: vec![
                            name(self.kind).to_string(),
                            status_words(&output.diagnosis.status, output.diagnosis.dof),
                        ],
                    });
                }
                CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                    drawn.tag = Some(Tag {
                        at,
                        lines: vec![name(self.kind).to_string(), error.message],
                    });
                    drawn.tag_tone = Tone::Danger;
                }
                _ => {}
            }
        }
        self.drawn = drawn;
    }
}

fn moved_words(n: usize) -> String {
    if n == 0 {
        "nesneler yerinde kaldı".into()
    } else {
        format!("{n} nesne kısıta uydu")
    }
}

/// The solver's word for the drawing.
pub fn status_words(status: &SolveStatus, dof: u32) -> String {
    match status {
        SolveStatus::Fully => "tam kısıtlı".into(),
        SolveStatus::Under => format!("{dof} serbestlik kaldı"),
        SolveStatus::Over => "bazı kısıtlar tekrarlıyor".into(),
        SolveStatus::Inconsistent => "kısıtlar çelişiyor".into(),
        SolveStatus::Singular => "bir kısıt anlamını yitirdi".into(),
        SolveStatus::NonConvergent => "çözüm bulunamadı".into(),
    }
}

/// A picked part's mark: a point's square, a segment drawn over, a circle's outline.
fn mark(out: &mut Preview, p: &Picked) {
    match (&p.element, p.ends) {
        (CadElement::Segment { .. }, Some((a, b))) => out
            .strokes
            .push(Stroke::solid(vec![a, b], false).tone(Tone::Snap)),
        (CadElement::Point { .. }, _) => out.squares.push(p.at),
        _ => out.labels.push(Label {
            at: p.at,
            text: "○".into(),
            offset: [8.0, -8.0],
            tone: Tone::Snap,
        }),
    }
}

impl Tool for ConstraintTool {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let title = name(self.kind);
        let mut prompt = if self.asking {
            Prompt::new(title, "değeri yazın (Enter: çizimin ölçtüğü)").option_with(
                "Başvuru",
                "B",
                if self.reference { "evet" } else { "hayır" },
            )
        } else {
            let wanted = self.wanted();
            let k = self.picked.len().min(wanted.len().saturating_sub(1));
            Prompt::new(
                title,
                format!("{}. parça: {} tıklayın", k + 1, want_words(wanted[k])),
            )
        };
        if !self.asking {
            for (_, n, letters) in KINDS {
                prompt = prompt.option(n, letters);
            }
        }
        prompt
    }

    fn point_count(&self) -> usize {
        self.picked.len()
    }

    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }

    /// Parts are picked, not points: no snap.
    fn snaps(&self) -> bool {
        false
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.cursor = Some(p.raw);
        if self.asking {
            return;
        }
        let wanted = self.wanted();
        let Some(&want) = wanted.get(self.picked.len()) else {
            return;
        };
        self.hover = part_at(p, cx, want);
        cx.selection.set_hover(self.hover.as_ref().map(|h| h.slot));
        self.show(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.asking {
            return;
        }
        let wanted = self.wanted();
        let Some(&want) = wanted.get(self.picked.len()) else {
            return;
        };
        let Some(part) = part_at(p, cx, want) else {
            cx.say(
                Level::Warn,
                format!(
                    "{}: burada {} yok; {} tıklayın.",
                    name(self.kind),
                    want_words(want),
                    want_words(want)
                ),
            );
            return;
        };
        self.picked.push(part);
        if !self.complete_with(0) {
            return;
        }
        if dimensional(self.kind) {
            self.asking = true;
            self.drawn = Preview::default();
            return;
        }
        self.write(None, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let word = upper_tr(js_trim(text));
        if self.asking {
            if word == "B" {
                self.reference = !self.reference;
                return true;
            }
            if word.is_empty() {
                return false;
            }
            self.write(Some(js_trim(text).to_string()), cx);
            return true;
        }
        if let Some((k, ..)) = KINDS.iter().find(|(_, _, letters)| *letters == word) {
            self.kind = *k;
            self.reset();
            return true;
        }
        false
    }

    /// Enter: a dimension keeps what the drawing measures; otherwise the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.asking {
            self.write(None, cx);
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        self.undo_step(cx)
    }

    /// Ctrl+Z: the last part picked goes back.
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        if self.asking {
            self.asking = false;
        }
        if self.picked.pop().is_some() {
            self.drawn = Preview::default();
            return true;
        }
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        self.drawn.clone()
    }
}
