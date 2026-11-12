//! Görünümü döndür (docs/adr/0285 §2): the view turned so a direction of the
//! world lies horizontal on the screen, as AutoCAD's view twist. Two points
//! give the direction that becomes the screen's x axis (left to right); a
//! typed angle is the turn itself, in the project's unit and way (CAD
//! counter-clockwise, CBS clockwise); UCS'e göre (U) lays the UCS's x axis
//! horizontal, as AutoCAD's PLAN; Kuzey yukarı (K) turns it back. The view is
//! the host's camera ([`ViewChange::Twist`]): nothing reaches the drawing,
//! so the tool runs over a command without ending it (docs/adr/0283).

use kentos_geometry_core::jsmath::atan2;

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::Prompt;
use crate::tool::{Context, Flow, Pointer, Preview, Tool, ViewChange};

pub const ID: &str = "viewTwist";
pub const LABEL: &str = "Görünümü döndür";

/// Two points nearer than this give no direction, metres (1 µm).
const SAME: f64 = 1.0e-6;

/// Görünümü döndür.
#[derive(Clone, Debug, Default)]
pub struct ViewTwist {
    /// The direction's first point, once given.
    first: Option<Vec2>,
    hover: Option<Vec2>,
    done: bool,
}

impl ViewTwist {
    pub fn new() -> Self {
        Self::default()
    }

    /// Asks the host for the twist; the tool ends.
    fn twist(&mut self, angle: f64, cx: &mut Context<'_>) {
        cx.view_changes.push(ViewChange::Twist(angle));
        let said = if angle == 0.0 {
            "Görünüm: kuzey yukarı".to_owned()
        } else {
            format!("Görünüm döndürüldü: {}", cx.format().angle(angle))
        };
        cx.say(Level::Success, said);
        self.done = true;
    }

    /// The direction from the first point to `p`, when they are apart.
    fn through(&mut self, p: Vec2, cx: &mut Context<'_>) {
        let Some(a) = self.first else {
            self.first = Some(p);
            return;
        };
        if (p.x - a.x).hypot(p.y - a.y) < SAME {
            cx.say(
                Level::Warn,
                "İki nokta aynı yerde; doğrultu bulunamadı. İkinci noktayı ilkinden uzakta verin.",
            );
            return;
        }
        self.twist(atan2(p.y - a.y, p.x - a.x), cx);
    }

    /// A typed angle as the turn: the project's unit, counter-clockwise in CAD,
    /// clockwise in CBS (its directions run that way).
    fn typed_turn(text: &str, f: &Format) -> Option<f64> {
        let v = kentos_geometry_core::tools::point_text::parse_number(text)?;
        let rad = f.angle_from_typed(v);
        Some(match f.axes {
            crate::format::Axes::Cad => rad,
            crate::format::Axes::Gis => -rad,
        })
    }
}

impl Tool for ViewTwist {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        match self.first {
            None => Prompt::new(
                LABEL,
                "yatay olacak doğrultunun ilk noktasını verin ya da açı yazın",
            )
            .option("UCS'e göre", "U")
            .option("Kuzey yukarı", "K"),
            Some(_) => Prompt::new(LABEL, "doğrultunun ikinci noktasını verin"),
        }
        .note("Esc: bitir")
    }

    fn point_count(&self) -> usize {
        usize::from(self.first.is_some())
    }

    fn snaps(&self) -> bool {
        true
    }

    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.through(p, cx);
        true
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.first
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(match self.first {
            Some(a) => points::constrain(Some(a), p, cx).0,
            None => p.world,
        });
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let at = match self.first {
            Some(a) => points::constrain(Some(a), p, cx).0,
            None => p.world,
        };
        self.through(at, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let text = text.trim();
        match text.to_uppercase().as_str() {
            "U" if self.first.is_none() => {
                let angle = cx.doc.settings().ucs.map_or(0.0, |u| u.angle);
                self.twist(angle, cx);
                return true;
            }
            "K" if self.first.is_none() => {
                self.twist(0.0, cx);
                return true;
            }
            _ => {}
        }
        // A point: the direction's; a number at the first step: the turn.
        if self.first.is_none()
            && let Some(turn) = Self::typed_turn(text, &cx.format())
        {
            self.twist(turn, cx);
            return true;
        }
        let Some(p) = cx.typed_point(text, self.first, self.hover) else {
            return false;
        };
        self.through(p, cx);
        true
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        self.first.take().is_some()
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        self.first.take().is_some()
    }

    fn finished(&self) -> bool {
        self.done
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut preview = Preview::default();
        if let (Some(a), Some(h)) = (self.first, self.hover) {
            preview.guides.push([a, h]);
        }
        preview
    }
}
