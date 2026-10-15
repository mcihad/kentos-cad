//! Eğriyi düzenle (AutoCAD's PEDIT and SPLINEDIT, docs/adr/0303): click a polyline or a NURBS curve; its points
//! are marked (fit points as rings, control points as squares, the control polygon dashed) and the prompt offers
//! what that curve takes:
//!
//! - a polyline: Kapat (K) or Aç (A), Düzleştir (D) when it has an arc edge, Eğriye çevir (E: through its
//!   vertices), Denetim noktalarıyla (N: its vertices the control points), Derece (R) of the curve made;
//! - a NURBS curve: Kapat (K) or Aç (A); with fit points Nokta ekle (E), Noktayı taşı (T), Noktayı sil (S),
//!   Teğetler (G: a click gives the start's direction, another the end's, Enter leaves one free); Denetim taşı
//!   (D), Ağırlık (L), Denetim sil (İ), Düğüm ekle (Ü: on the curve where clicked), Çoklu çizgiye çevir (Ç: the
//!   tolerance typed).
//!
//! A point to move or add follows the mouse with the command's plan drawn; a click writes through
//! `cad.curves.edit`, one undo step “Eğriyi düzenle”, and the curve stays picked for the next change. Enter or Esc
//! drops the option asked for, then the curve; with neither, Enter leaves.

use kentos_contracts::{CommandResult, CurveEdit, CurvesEdit, Entity};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::geom::nurbs::OwnedNurbs;
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, curves_edit};

use crate::Vec2;
use crate::edge::{self, Outline};
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, plain_number, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{
    Context, Cursor, Flow, Marker, MarkerShape, Pointer, Preview, Stroke, Tone, Tool,
};

/// The tool's id: its command is `tool.curveEdit`.
pub const ID: &str = "curveEdit";
pub const LABEL: &str = "Eğriyi düzenle";

/// How many pick tolerances from a point a click still takes it.
const NEAR: f64 = 4.0;

/// What a click or a typed value is asked for; a point's number once it is taken.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Ask {
    AddFit,
    MoveFit(Option<usize>),
    RemoveFit,
    /// The start's direction (none yet), then the end's (the start's given or left free).
    Tangents(Option<Option<Vec2>>),
    MoveControl(Option<usize>),
    Weight(Option<usize>),
    RemoveControl,
    Knot,
    Tolerance,
    Degree,
}

/// The picked curve as the prompt shows it.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Polyline { closed: bool, arcs: bool },
    Nurbs { closed: bool, fit: bool },
}

/// A polyline or a NURBS curve on an unlocked layer.
fn editable(e: &Entity, doc: &Document) -> bool {
    matches!(e, Entity::Polyline(_) | Entity::Nurbs(_)) && edge::unlocked(e, doc)
}

fn kind_of(e: &Entity) -> Option<Kind> {
    match e {
        Entity::Polyline(p) => Some(Kind::Polyline {
            closed: p.closed,
            arcs: p
                .bulges
                .as_ref()
                .is_some_and(|b| b.iter().any(|x| *x != 0.0)),
        }),
        Entity::Nurbs(n) => Some(Kind::Nurbs {
            closed: n.closed,
            fit: n.fit.is_some(),
        }),
        _ => None,
    }
}

fn pt(p: &kentos_contracts::Vec2) -> Vec2 {
    Vec2::new(p.x, p.y)
}

/// A NURBS curve's fit points and drawn control points (a closed one's without its wrapped copies).
fn points_of(e: &Entity) -> (Vec<Vec2>, Vec<Vec2>) {
    match e {
        Entity::Nurbs(n) => {
            let count = if n.closed && n.periodic {
                n.ctrl.len().saturating_sub(n.degree as usize)
            } else {
                n.ctrl.len()
            };
            (
                n.fit.iter().flatten().map(pt).collect(),
                n.ctrl[..count].iter().map(pt).collect(),
            )
        }
        Entity::Polyline(p) => (Vec::new(), p.pts.iter().map(pt).collect()),
        _ => (Vec::new(), Vec::new()),
    }
}

/// The nearest of `pts` to `at`, when within `reach`.
fn nearest(pts: &[Vec2], at: Vec2, reach: f64) -> Option<usize> {
    pts.iter()
        .enumerate()
        .map(|(i, p)| (i, dist(*p, at)))
        .filter(|(_, d)| *d <= reach)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

/// Where a new fit point goes among `fit`: before the first, after the last or between two, whichever the way
/// through it grows least (a closed curve's last and first are neighbours too).
fn insertion(fit: &[Vec2], at: Vec2, closed: bool) -> usize {
    let n = fit.len();
    if n == 0 {
        return 0;
    }
    let mut best = (
        n,
        if closed {
            f64::INFINITY
        } else {
            dist(fit[n - 1], at)
        },
    );
    if !closed && dist(fit[0], at) < best.1 {
        best = (0, dist(fit[0], at));
    }
    let pairs = if closed { n } else { n - 1 };
    for i in 0..pairs {
        let (a, b) = (fit[i], fit[(i + 1) % n]);
        let grows = dist(a, at) + dist(at, b) - dist(a, b);
        if grows < best.1 {
            best = (i + 1, grows);
        }
    }
    best.0
}

/// The parameter on the NURBS curve nearest `at`.
fn knot_at(e: &Entity, at: Vec2) -> Option<f64> {
    let Entity::Nurbs(n) = e else {
        return None;
    };
    let c = OwnedNurbs {
        degree: n.degree as usize,
        ctrl: n.ctrl.iter().map(pt).collect(),
        knots: n.knots.clone(),
        weights: n.weights.clone(),
    };
    Some(c.view().closest(at))
}

/// What the message says once the edit is written.
fn said(edit: &CurveEdit) -> &'static str {
    match edit {
        CurveEdit::Close => "Eğri kapandı.",
        CurveEdit::Open => "Eğri açıldı.",
        CurveEdit::Decurve => "Yay kenarları düzleşti.",
        CurveEdit::ToSpline { through: true, .. } => "Çoklu çizgi köşelerinden geçen eğri oldu.",
        CurveEdit::ToSpline { .. } => "Çoklu çizgi köşeleri denetim noktası olan eğri oldu.",
        CurveEdit::AddFit { .. } => "Geçiş noktası eklendi.",
        CurveEdit::MoveFit { .. } => "Geçiş noktası taşındı.",
        CurveEdit::RemoveFit { .. } => "Geçiş noktası silindi.",
        CurveEdit::Tangents { .. } => "Uç teğetleri yazıldı.",
        CurveEdit::MoveControl { .. } => "Denetim noktası taşındı.",
        CurveEdit::Weight { .. } => "Ağırlık yazıldı.",
        CurveEdit::RemoveControl { .. } => "Denetim noktası silindi.",
        CurveEdit::InsertKnot { .. } => "Düğüm eklendi; eğri değişmedi.",
        CurveEdit::ToPolyline { .. } => "Eğri çoklu çizgi oldu.",
    }
}

/// The eğriyi düzenle tool.
#[derive(Clone, Debug, Default)]
pub struct CurveEditTool {
    picked: Option<Slot>,
    ask: Option<Ask>,
    mouse: Option<Vec2>,
    drawn: Preview,
    /// What the prompt shows: the picked curve's kind, the format, the degree and tolerance remembered.
    seen: Option<(Option<Kind>, Format, u32, f64)>,
}

impl CurveEditTool {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        let kind = self.picked.and_then(|s| cx.doc.get(s)).and_then(kind_of);
        if self.picked.is_some() && kind.is_none() {
            // Undone or removed elsewhere: back to picking.
            self.picked = None;
            self.ask = None;
        }
        self.seen = Some((
            kind,
            cx.format(),
            cx.memory.nurbs_degree,
            cx.memory.curve_tolerance,
        ));
    }

    fn input_for(&self, edit: CurveEdit, doc: &Document) -> Option<CurvesEdit> {
        let uid = doc.uid(self.picked?)?;
        Some(CurvesEdit {
            uid: uid.to_string(),
            edit,
            expected_revision: None,
        })
    }

    fn apply(&mut self, edit: CurveEdit, cx: &mut Context<'_>) {
        let words = said(&edit);
        let Some(input) = self.input_for(edit, cx.doc) else {
            return;
        };
        let result = curves_edit::execute(&mut ExecutionContext::new(cx.doc), input);
        if points::written(result, cx).is_some() {
            cx.say(Level::Success, words);
        }
        self.ask = None;
        self.see(cx);
        self.redraw(cx);
    }

    /// The edit a click at `at` makes in the step asked for: none when the step takes a point first (then the
    /// point is taken) or there is nothing near.
    fn edit_at(&mut self, at: Vec2, reach: f64, cx: &mut Context<'_>) -> Option<CurveEdit> {
        let e = cx.doc.get(self.picked?)?.clone();
        let (fit, ctrl) = points_of(&e);
        let closed = matches!(kind_of(&e), Some(Kind::Nurbs { closed: true, .. }));
        let take = |pts: &[Vec2], cx: &mut Context<'_>| {
            let i = nearest(pts, at, reach);
            if i.is_none() {
                cx.say(Level::Warn, "Bir noktanın yakınına tıklayın.");
            }
            i
        };
        match self.ask? {
            Ask::AddFit => Some(CurveEdit::AddFit {
                index: insertion(&fit, at, closed) as u32,
                at: wire(at),
            }),
            Ask::MoveFit(Some(i)) => Some(CurveEdit::MoveFit {
                index: i as u32,
                at: wire(at),
            }),
            Ask::MoveFit(None) => {
                self.ask = Some(Ask::MoveFit(Some(take(&fit, cx)?)));
                None
            }
            Ask::RemoveFit => Some(CurveEdit::RemoveFit {
                index: take(&fit, cx)? as u32,
            }),
            Ask::Tangents(None) => {
                let from = *fit.first()?;
                self.ask = Some(Ask::Tangents(Some(Some(Vec2::new(
                    at.x - from.x,
                    at.y - from.y,
                )))));
                None
            }
            Ask::Tangents(Some(start)) => {
                let to = *fit.last()?;
                Some(CurveEdit::Tangents {
                    start: start.map(wire),
                    end: Some(wire(Vec2::new(at.x - to.x, at.y - to.y))),
                })
            }
            Ask::MoveControl(Some(i)) => Some(CurveEdit::MoveControl {
                index: i as u32,
                at: wire(at),
            }),
            Ask::MoveControl(None) => {
                self.ask = Some(Ask::MoveControl(Some(take(&ctrl, cx)?)));
                None
            }
            Ask::Weight(None) => {
                self.ask = Some(Ask::Weight(Some(take(&ctrl, cx)?)));
                None
            }
            Ask::RemoveControl => Some(CurveEdit::RemoveControl {
                index: take(&ctrl, cx)? as u32,
            }),
            Ask::Knot => Some(CurveEdit::InsertKnot {
                at: knot_at(&e, at)?,
            }),
            Ask::Weight(Some(_)) | Ask::Tolerance | Ask::Degree => None,
        }
    }

    /// A point given in the step asked for: a click or a typed or computed point.
    fn give(&mut self, at: Vec2, reach: f64, cx: &mut Context<'_>) {
        if let Some(edit) = self.edit_at(at, reach, cx) {
            self.apply(edit, cx);
        } else {
            self.see(cx);
            self.redraw(cx);
        }
    }

    /// The preview: the picked curve's points marked and its control polygon dashed; with a point following the
    /// mouse, the command's plan of it drawn.
    fn redraw(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        let Some(e) = self.picked.and_then(|s| cx.doc.get(s)).cloned() else {
            return;
        };
        let mut shown = e.clone();
        let follows = matches!(
            self.ask,
            Some(Ask::AddFit | Ask::MoveFit(Some(_)) | Ask::MoveControl(Some(_)))
        );
        if follows
            && let Some(mouse) = self.mouse
            && let Some(edit) = self.clone().edit_without_saying(mouse, cx)
            && let Some(input) = self.input_for(edit, cx.doc)
            && let CommandResult::Completed { output, .. } = curves_edit::plan_in(cx.doc, &input)
            && let Some(after) = output.changed.into_iter().next()
        {
            let out = Outline::of(&shape(&after), None, 2.0, Tone::Accent);
            self.drawn.strokes.extend(out.strokes);
            shown = after;
        }
        let (fit, ctrl) = points_of(&shown);
        if matches!(shown, Entity::Nurbs(_)) && ctrl.len() > 1 {
            let closed = matches!(kind_of(&shown), Some(Kind::Nurbs { closed: true, .. }));
            self.drawn
                .strokes
                .push(Stroke::dashed(ctrl.clone(), closed, [4.0, 3.0]).tone(Tone::Snap));
        }
        let fit_asked = matches!(
            self.ask,
            Some(Ask::AddFit | Ask::MoveFit(_) | Ask::RemoveFit | Ask::Tangents(_))
        );
        for p in &fit {
            self.drawn.markers.push(Marker {
                at: *p,
                shape: MarkerShape::Ring(4.0),
                tone: if fit_asked { Tone::Accent } else { Tone::Snap },
            });
        }
        if matches!(shown, Entity::Nurbs(_)) || !fit_asked {
            for p in &ctrl {
                self.drawn.markers.push(Marker {
                    at: *p,
                    shape: MarkerShape::Square(3.0),
                    tone: if fit_asked { Tone::Snap } else { Tone::Accent },
                });
            }
        }
        if let (Some(Ask::Knot), Some(mouse)) = (self.ask, self.mouse)
            && let Some(u) = knot_at(&e, mouse)
            && let Entity::Nurbs(n) = &e
        {
            let c = OwnedNurbs {
                degree: n.degree as usize,
                ctrl: n.ctrl.iter().map(pt).collect(),
                knots: n.knots.clone(),
                weights: n.weights.clone(),
            };
            self.drawn.markers.push(Marker {
                at: c.view().point(u),
                shape: MarkerShape::Plus(5.0),
                tone: Tone::Accent,
            });
        }
        if let Some(Ask::Tangents(start)) = self.ask
            && let Some(mouse) = self.mouse
        {
            let from = if start.is_none() {
                fit.first()
            } else {
                fit.last()
            };
            if let Some(from) = from {
                self.drawn.guides.push([*from, mouse]);
            }
        }
    }

    /// [`Self::edit_at`] for the preview: nothing said, the tool unchanged.
    fn edit_without_saying(mut self, at: Vec2, cx: &Context<'_>) -> Option<CurveEdit> {
        let e = cx.doc.get(self.picked?)?;
        let (fit, _) = points_of(e);
        let closed = matches!(kind_of(e), Some(Kind::Nurbs { closed: true, .. }));
        match self.ask.take()? {
            Ask::AddFit => Some(CurveEdit::AddFit {
                index: insertion(&fit, at, closed) as u32,
                at: wire(at),
            }),
            Ask::MoveFit(Some(i)) => Some(CurveEdit::MoveFit {
                index: i as u32,
                at: wire(at),
            }),
            Ask::MoveControl(Some(i)) => Some(CurveEdit::MoveControl {
                index: i as u32,
                at: wire(at),
            }),
            _ => None,
        }
    }

    /// An option's letter: an edit at once, or the step it asks for.
    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> bool {
        let Some(kind) = self.picked.and_then(|s| cx.doc.get(s)).and_then(kind_of) else {
            return false;
        };
        let degree = cx.memory.nurbs_degree;
        let now = |edit: CurveEdit| Some(Err(edit));
        let ask = |a: Ask| Some(Ok(a));
        let chosen = match (key, kind) {
            ("K", Kind::Polyline { closed: false, .. } | Kind::Nurbs { closed: false, .. }) => {
                now(CurveEdit::Close)
            }
            ("A", Kind::Polyline { closed: true, .. } | Kind::Nurbs { closed: true, .. }) => {
                now(CurveEdit::Open)
            }
            ("D", Kind::Polyline { arcs: true, .. }) => now(CurveEdit::Decurve),
            ("E", Kind::Polyline { .. }) => now(CurveEdit::ToSpline {
                degree: Some(degree),
                through: true,
            }),
            ("N", Kind::Polyline { .. }) => now(CurveEdit::ToSpline {
                degree: Some(degree),
                through: false,
            }),
            ("R", Kind::Polyline { .. }) => ask(Ask::Degree),
            ("E", Kind::Nurbs { fit: true, .. }) => ask(Ask::AddFit),
            ("T", Kind::Nurbs { fit: true, .. }) => ask(Ask::MoveFit(None)),
            ("S", Kind::Nurbs { fit: true, .. }) => ask(Ask::RemoveFit),
            (
                "G",
                Kind::Nurbs {
                    fit: true,
                    closed: false,
                },
            ) => ask(Ask::Tangents(None)),
            ("D", Kind::Nurbs { .. }) => ask(Ask::MoveControl(None)),
            ("L", Kind::Nurbs { .. }) => ask(Ask::Weight(None)),
            ("İ", Kind::Nurbs { .. }) => ask(Ask::RemoveControl),
            ("Ü", Kind::Nurbs { closed: false, .. }) => ask(Ask::Knot),
            ("Ç", Kind::Nurbs { .. }) => ask(Ask::Tolerance),
            _ => None,
        };
        match chosen {
            Some(Err(edit)) => self.apply(edit, cx),
            Some(Ok(a)) => {
                self.ask = Some(a);
                self.see(cx);
                self.redraw(cx);
            }
            None => return false,
        }
        true
    }
}

impl Tool for CurveEditTool {
    fn cursor(&self) -> Cursor {
        match self.ask {
            Some(
                Ask::AddFit | Ask::MoveFit(Some(_)) | Ask::MoveControl(Some(_)) | Ask::Tangents(_),
            ) => Cursor::Cross,
            _ => Cursor::Pick,
        }
    }

    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let (kind, f, degree, tolerance) = self.seen.unwrap_or((None, Format::default(), 3, 0.01));
        let Some(kind) = kind.filter(|_| self.picked.is_some()) else {
            return Prompt::new(
                LABEL,
                "düzenlenecek çoklu çizgiye ya da NURBS eğrisine tıklayın",
            );
        };
        match self.ask {
            Some(Ask::AddFit) => return Prompt::new(LABEL, "yeni geçiş noktasını gösterin"),
            Some(Ask::MoveFit(None)) => {
                return Prompt::new(LABEL, "taşınacak geçiş noktasına tıklayın");
            }
            Some(Ask::MoveFit(Some(i))) => {
                return Prompt::new(LABEL, "geçiş noktasının yeni yerini gösterin")
                    .note(format!("nokta {}", i + 1));
            }
            Some(Ask::RemoveFit) => {
                return Prompt::new(LABEL, "silinecek geçiş noktasına tıklayın");
            }
            Some(Ask::Tangents(None)) => {
                return Prompt::new(LABEL, "başlangıç teğetinin doğrultusunu gösterin")
                    .note("Enter: serbest");
            }
            Some(Ask::Tangents(Some(_))) => {
                return Prompt::new(LABEL, "bitiş teğetinin doğrultusunu gösterin")
                    .note("Enter: serbest");
            }
            Some(Ask::MoveControl(None)) => {
                return Prompt::new(LABEL, "taşınacak denetim noktasına tıklayın");
            }
            Some(Ask::MoveControl(Some(i))) => {
                return Prompt::new(LABEL, "denetim noktasının yeni yerini gösterin")
                    .note(format!("nokta {}", i + 1));
            }
            Some(Ask::Weight(None)) => {
                return Prompt::new(LABEL, "ağırlığı değişecek denetim noktasına tıklayın");
            }
            Some(Ask::Weight(Some(i))) => {
                return Prompt::new(LABEL, "ağırlığı yazın (sıfırdan büyük; 1 olağan)")
                    .note(format!("nokta {}", i + 1));
            }
            Some(Ask::RemoveControl) => {
                return Prompt::new(LABEL, "silinecek denetim noktasına tıklayın");
            }
            Some(Ask::Knot) => {
                return Prompt::new(LABEL, "düğümün ekleneceği yere, eğrinin üstüne tıklayın");
            }
            Some(Ask::Tolerance) => {
                return Prompt::new(LABEL, "toleransı yazın: her kiriş eğriye bundan yakın olur")
                    .note(format!("son: {}", f.length(tolerance)));
            }
            Some(Ask::Degree) => {
                return Prompt::new(LABEL, "eğrinin derecesini yazın (1 ile 11 arası)");
            }
            None => {}
        }
        let mut prompt = Prompt::new(
            LABEL,
            "bir düzenleme seçin ya da Enter'la başka eğriye geçin",
        );
        match kind {
            Kind::Polyline { closed, arcs } => {
                prompt = prompt
                    .note(if closed {
                        "kapalı çoklu çizgi"
                    } else {
                        "açık çoklu çizgi"
                    })
                    .then();
                prompt = if closed {
                    prompt.option("Aç", "A")
                } else {
                    prompt.option("Kapat", "K")
                };
                if arcs {
                    prompt = prompt.option("Düzleştir", "D");
                }
                prompt
                    .option("Eğriye çevir", "E")
                    .option("Denetim noktalarıyla", "N")
                    .option_with("Derece", "R", degree.to_string())
            }
            Kind::Nurbs { closed, fit } => {
                prompt = prompt
                    .note(match (closed, fit) {
                        (true, true) => "kapalı eğri, geçiş noktalı",
                        (true, false) => "kapalı eğri, denetim noktalı",
                        (false, true) => "açık eğri, geçiş noktalı",
                        (false, false) => "açık eğri, denetim noktalı",
                    })
                    .then();
                prompt = if closed {
                    prompt.option("Aç", "A")
                } else {
                    prompt.option("Kapat", "K")
                };
                if fit {
                    prompt = prompt
                        .option("Nokta ekle", "E")
                        .option("Noktayı taşı", "T")
                        .option("Noktayı sil", "S");
                    if !closed {
                        prompt = prompt.option("Teğetler", "G");
                    }
                }
                prompt = prompt
                    .then()
                    .option("Denetim taşı", "D")
                    .option("Ağırlık", "L")
                    .option("Denetim sil", "İ");
                if !closed {
                    prompt = prompt.option("Düğüm ekle", "Ü");
                }
                prompt.option("Çoklu çizgiye çevir", "Ç")
            }
        }
    }

    fn point_count(&self) -> usize {
        0
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        // One polyline or NURBS curve selected: it is the curve to edit.
        if let [slot] = *cx.selection.ids()
            && cx.doc.get(slot).is_some_and(|e| editable(e, cx.doc))
        {
            self.picked = Some(slot);
        }
        self.see(cx);
        self.redraw(cx);
        Flow::Stay
    }

    fn snaps(&self) -> bool {
        matches!(
            self.ask,
            Some(
                Ask::AddFit | Ask::MoveFit(Some(_)) | Ask::MoveControl(Some(_)) | Ask::Tangents(_)
            )
        )
    }

    fn accepts_points(&self) -> bool {
        self.snaps()
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        if !self.snaps() {
            return false;
        }
        let reach = NEAR * cx.pick_tolerance();
        self.give(p, reach, cx);
        true
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        self.mouse = Some(p.world);
        if self.picked.is_none() {
            let hit = edge::pick(p, cx, editable);
            cx.selection.set_hover(hit);
        }
        self.redraw(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        if self.picked.is_some() && self.ask.is_some() {
            let at = if self.snaps() { p.world } else { p.raw };
            let reach = NEAR * cx.pick_tolerance();
            return self.give(at, reach, cx);
        }
        let Some(slot) = edge::pick_said(p, cx, editable) else {
            if self.picked.is_none() {
                cx.say(
                    Level::Warn,
                    "Kilitsiz katmanda bir çoklu çizgiye ya da NURBS eğrisine tıklayın.",
                );
            }
            return;
        };
        self.picked = Some(slot);
        self.ask = None;
        cx.selection.set_hover(None);
        self.see(cx);
        self.redraw(cx);
    }

    /// An option's letter, then a number: a weight, a tolerance or a degree.
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        self.see(cx);
        if self.picked.is_none() {
            return false;
        }
        match self.ask {
            Some(Ask::Weight(Some(i))) => {
                let Some(weight) = plain_number(text) else {
                    return false;
                };
                self.apply(
                    CurveEdit::Weight {
                        index: i as u32,
                        weight,
                    },
                    cx,
                );
                return true;
            }
            Some(Ask::Tolerance) => {
                let Some((_, tolerance)) = points::plain_value(text, cx) else {
                    return false;
                };
                if tolerance > 0.0 {
                    cx.memory.curve_tolerance = tolerance;
                }
                self.apply(CurveEdit::ToPolyline { tolerance }, cx);
                return true;
            }
            Some(Ask::Degree) => {
                let Some(n) = plain_number(text) else {
                    return false;
                };
                if n.fract() != 0.0 || !(1.0..=11.0).contains(&n) {
                    cx.say(Level::Warn, "Derece 1 ile 11 arasında bir tam sayıdır.");
                    return true;
                }
                cx.memory.nurbs_degree = n as u32;
                self.ask = None;
                self.see(cx);
                return true;
            }
            _ => {}
        }
        self.option(&upper_tr(js_trim(text)), cx)
    }

    /// Teğetler leaves the end asked for free; otherwise drops the option asked for, then the curve; with neither,
    /// the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        match self.ask {
            Some(Ask::Tangents(None)) => {
                self.ask = Some(Ask::Tangents(Some(None)));
                self.redraw(cx);
                return Flow::Stay;
            }
            Some(Ask::Tangents(Some(start))) => {
                self.apply(
                    CurveEdit::Tangents {
                        start: start.map(wire),
                        end: None,
                    },
                    cx,
                );
                return Flow::Stay;
            }
            Some(Ask::Tolerance) => {
                let tolerance = cx.memory.curve_tolerance;
                self.apply(CurveEdit::ToPolyline { tolerance }, cx);
                return Flow::Stay;
            }
            _ => {}
        }
        if self.ask.take().is_some() {
            self.redraw(cx);
            return Flow::Stay;
        }
        if self.picked.take().is_some() {
            self.drawn = Preview::default();
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        self.see(cx);
        if self.ask.take().is_some() {
            self.redraw(cx);
            return true;
        }
        if self.picked.take().is_some() {
            self.drawn = Preview::default();
            return true;
        }
        false
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        self.drawn.clone()
    }
}
