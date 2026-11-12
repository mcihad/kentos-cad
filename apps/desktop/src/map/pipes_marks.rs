//! Boru hattı denetimi's crossings on the drawing (plan/map-cad.md MAP-16, docs/adr/3160): Kesişimleri göster leaves
//! each place where a checked pipe crosses the other network's marked by a ring with its vertical clearance, and each
//! near pass by a dashed line between the two closest places with its clearance, until İşareti kaldır (`data.unmark`)
//! or another drawing. The colour is the command's own verdict: a clash, a crossing closer than the least or on the
//! wrong side (the findings at that very place) in the danger colour, a clearance not known in the warning colour,
//! the rest in the success colour; a near pass is listed only when it is too close, so always danger. Worked out again
//! with the same input whenever the drawing changes, so a pipe lowered off a clash turns green at once. Nothing is
//! written.

use std::cell::RefCell;
use std::sync::Arc;

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::{Pixels, Point, Vector};
use kentos_commands::{PipesCheck, PipesChecked};
use kentos_native_application::pipes_check;
use kentos_render_wgpu::Camera;
use kentos_ui::theme::{Tokens, typography};

use crate::app::App;
use crate::viewport::MarkColors;

/// The findings that fail a crossing at its place.
const FAILS: [&str; 3] = ["clash", "closeCrossing", "wrongSide"];

/// What Kesişimleri göster keeps: the drawing's session, the check's input, and the marks of the drawing as it was
/// last seen (its generation and revision).
#[derive(Debug)]
pub struct Shown {
    pub session: u64,
    pub input: PipesCheck,
    made: RefCell<Option<(u64, u64, Option<Arc<PipesMarks>>)>>,
}

impl Shown {
    pub fn new(session: u64, input: PipesCheck) -> Self {
        Self { session, input, made: RefCell::new(None) }
    }
}

/// A crossing's verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Fails,
    Unknown,
    Holds,
}

/// A crossing: its place, verdict and clearance as written.
#[derive(Clone, Debug, PartialEq)]
pub struct CrossingMark {
    pub at: [f64; 2],
    pub verdict: Verdict,
    pub text: String,
}

/// A near pass: the closest places on the two pipes and its clearance as written.
#[derive(Clone, Debug, PartialEq)]
pub struct NearMark {
    pub a: [f64; 2],
    pub b: [f64; 2],
    pub text: String,
}

/// The crossings and the near passes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PipesMarks {
    pub crossings: Vec<CrossingMark>,
    pub near: Vec<NearMark>,
}

/// A clearance as the marks write it: metres to the centimetre.
fn metres(v: f64) -> String {
    format!("{v:.2} m")
}

/// The marks of a check's answer; none without a crossing or a near pass.
pub fn of(r: &PipesChecked) -> Option<PipesMarks> {
    let fails = |at: [f64; 2]| r.findings.iter().any(|f| FAILS.contains(&f.kind.as_str()) && f.at == at);
    let crossings: Vec<CrossingMark> = r
        .crossings
        .iter()
        .map(|c| CrossingMark {
            at: c.at,
            verdict: if fails(c.at) {
                Verdict::Fails
            } else if c.clearance.is_none() {
                Verdict::Unknown
            } else {
                Verdict::Holds
            },
            text: c.clearance.map_or_else(|| "?".to_owned(), metres),
        })
        .collect();
    let near: Vec<NearMark> = r
        .near
        .iter()
        .map(|n| NearMark { a: n.at, b: n.other_at, text: metres(n.clearance.unwrap_or(n.distance)) })
        .collect();
    (!crossings.is_empty() || !near.is_empty()).then_some(PipesMarks { crossings, near })
}

impl App {
    /// The crossings for the drawing as it is: none when nothing is shown, for another drawing, when the check no
    /// longer reads or finds none. Kept until the drawing changes.
    pub(crate) fn pipes_marks(&self) -> Option<Arc<PipesMarks>> {
        let shown = self.pipes_shown_marks.as_ref()?;
        let doc = self.document.as_ref()?;
        if doc.session != shown.session {
            return None;
        }
        let (generation, revision) = (doc.model.generation(), doc.model.revision());
        if let Some((g, r, made)) = shown.made.borrow().as_ref()
            && *g == generation
            && *r == revision
        {
            return made.clone();
        }
        let made = pipes_check::preview(&doc.model, &shown.input).ok().and_then(|r| of(&r)).map(Arc::new);
        *shown.made.borrow_mut() = Some((generation, revision, made.clone()));
        made
    }
}

/// Words at `at` on a halo.
fn words(frame: &mut canvas::Frame, at: Point, content: &str, color: iced::Color, halo: iced::Color) {
    let text = Text {
        content: content.to_owned(),
        position: at,
        color: halo,
        size: Pixels(typography::scaled(11.0)),
        font: typography::ui_strong(),
        align_y: iced::alignment::Vertical::Center,
        ..Text::default()
    };
    for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
        frame.fill_text(Text { position: text.position + Vector::new(dx, dy), ..text.clone() });
    }
    frame.fill_text(Text { color, ..text });
}

fn screen(camera: &Camera, p: [f64; 2]) -> Point {
    let s = camera.world_to_screen(kentos_interaction::Vec2::new(p[0], p[1]));
    Point::new(s[0] as f32, s[1] as f32)
}

/// The marks on the screen: the near passes' dashed lines first, then each crossing's double ring and its clearance.
pub fn draw(frame: &mut canvas::Frame, m: &PipesMarks, camera: &Camera, colors: &MarkColors, tokens: &Tokens) {
    let stroke = |width: f32, color: iced::Color| Stroke::default().with_color(color).with_width(width);
    for n in &m.near {
        let (a, b) = (screen(camera, n.a), screen(camera, n.b));
        let line = Path::line(a, b);
        frame.stroke(&line, stroke(5.0, colors.halo));
        frame.stroke(&line, Stroke { line_dash: canvas::LineDash { segments: &[6.0, 4.0], offset: 0 }, ..stroke(2.0, tokens.danger) });
        for p in [a, b] {
            frame.fill(&Path::circle(p, 3.0), tokens.danger);
        }
        let mid = Point::new((a.x + b.x) / 2.0 + 8.0, (a.y + b.y) / 2.0);
        words(frame, mid, &n.text, tokens.danger, colors.halo);
    }
    for c in &m.crossings {
        let color = match c.verdict {
            Verdict::Fails => tokens.danger,
            Verdict::Unknown => tokens.warning,
            Verdict::Holds => tokens.success,
        };
        let p = screen(camera, c.at);
        for r in [9.0, 5.0] {
            let ring = Path::circle(p, r);
            frame.stroke(&ring, stroke(4.0, colors.halo));
            frame.stroke(&ring, stroke(2.0, color));
        }
        // Up and to the right: off the pipe the crossing lies on.
        words(frame, Point::new(p.x + 11.0, p.y - 13.0), &c.text, color, colors.halo);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_commands::{PipeCrossing, PipeFinding, PipeNear};

    fn checked(findings: Vec<PipeFinding>, crossings: Vec<PipeCrossing>, near: Vec<PipeNear>) -> PipesChecked {
        PipesChecked {
            network: "n".into(),
            cover_to: String::new(),
            pipes: Vec::new(),
            findings,
            counts: Vec::new(),
            parts: 1,
            length: 0.0,
            crossings,
            near,
        }
    }

    fn crossing(at: [f64; 2], clearance: Option<f64>) -> PipeCrossing {
        PipeCrossing { pipe: "p".into(), other: "o".into(), at, invert: None, other_invert: None, clearance, above: None }
    }

    #[test]
    fn a_crossing_takes_the_command_s_verdict_at_its_place() {
        let r = checked(
            vec![
                PipeFinding { kind: "clash".into(), uid: "p".into(), at: [1.0, 1.0], value: Some(-0.2), unit: None },
                // A finding of another kind at the same place is not the crossing's.
                PipeFinding { kind: "lowSlope".into(), uid: "p".into(), at: [3.0, 3.0], value: None, unit: None },
            ],
            vec![crossing([1.0, 1.0], Some(-0.2)), crossing([2.0, 2.0], None), crossing([3.0, 3.0], Some(0.5))],
            vec![PipeNear { pipe: "p".into(), other: "o".into(), at: [0.0, 0.0], other_at: [0.0, 0.4], distance: 0.4, clearance: Some(0.25) }],
        );
        let m = of(&r).expect("marks");
        let verdicts: Vec<Verdict> = m.crossings.iter().map(|c| c.verdict).collect();
        assert_eq!(verdicts, [Verdict::Fails, Verdict::Unknown, Verdict::Holds]);
        assert_eq!(m.crossings[0].text, "-0.20 m");
        assert_eq!(m.crossings[1].text, "?");
        assert_eq!(m.near[0].text, "0.25 m");
        assert!(of(&checked(Vec::new(), Vec::new(), Vec::new())).is_none());
    }
}
