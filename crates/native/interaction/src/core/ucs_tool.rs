//! The UCS tools (docs/adr/0261 §2, §4): Nesneye hizala, İki nokta,
//! Orijin, Döndür and Adlı UCS. Each gives `cad.ucs.set` what the user
//! clicked or typed; the command finds the frame and keeps it in the
//! project's settings (no undo step, no object moves). The preview draws the
//! frame to come: its x and y axes from its origin.
//!
//! - Nesne: click an object; its edge, end or turn gives the frame (§3).
//! - İki nokta: the origin, then a point on the x axis.
//! - Orijin: the new origin; the axes keep their angle.
//! - Döndür: a typed angle (the project's unit and way) or a click whose
//!   direction from the origin becomes the x axis.
//! - Adlı UCS: a name restores it; Kaydet (K) keeps the current UCS under
//!   the name typed next, Sil (S) removes one; the names are the options'
//!   choices.

use kentos_commands::{UcsSet, UcsTo, Vec2 as Wire};
use kentos_native_application::{ExecutionContext, ucs_set};

use kentos_geometry_core::jsmath::{atan2, cos, sin};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::Prompt;
use crate::tool::{Context, Flow, Label, OptionChoice, Pointer, Preview, Tone, Tool};

pub const OBJECT_ID: &str = "ucsObject";
pub const TWO_POINTS_ID: &str = "ucsTwoPoints";
pub const ORIGIN_ID: &str = "ucsOrigin";
pub const ROTATE_ID: &str = "ucsRotate";
pub const NAMED_ID: &str = "ucsNamed";

/// How long the preview's axes are on screen, logical pixels.
const ARM_PX: f64 = 60.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Object,
    TwoPoints,
    Origin,
    Rotate,
    Named,
}

/// What Adlı UCS waits for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Naming {
    /// A name to restore.
    #[default]
    Restore,
    /// A name to keep the current UCS under.
    Save,
    /// A name to remove.
    Delete,
}

/// The UCS tools.
#[derive(Clone, Debug)]
pub struct UcsTool {
    mode: Mode,
    naming: Naming,
    /// İki nokta's origin, once given.
    origin: Option<Vec2>,
    /// The pointer, and how long the axes are drawn in the world.
    hover: Option<Vec2>,
    arm: f64,
    /// The current UCS's origin and angle (Döndür turns about it).
    now: (Vec2, f64),
    /// The named UCSs' names, for the options' choices.
    names: Vec<String>,
    done: bool,
}

fn wire(p: Vec2) -> Wire {
    Wire { x: p.x, y: p.y }
}

impl UcsTool {
    fn of(mode: Mode) -> Self {
        Self {
            mode,
            naming: Naming::Restore,
            origin: None,
            hover: None,
            arm: 1.0,
            now: (Vec2::new(0.0, 0.0), 0.0),
            names: Vec::new(),
            done: false,
        }
    }

    pub fn object() -> Self {
        Self::of(Mode::Object)
    }
    pub fn two_points() -> Self {
        Self::of(Mode::TwoPoints)
    }
    pub fn origin() -> Self {
        Self::of(Mode::Origin)
    }
    pub fn rotate() -> Self {
        Self::of(Mode::Rotate)
    }
    pub fn named() -> Self {
        Self::of(Mode::Named)
    }

    fn see(&mut self, cx: &Context<'_>) {
        let s = cx.doc.settings();
        let u = s.ucs.unwrap_or(kentos_commands::Ucs::WORLD);
        self.now = (Vec2::new(u.origin.x, u.origin.y), u.angle);
        self.names = s.named_ucs.iter().map(|n| n.name.clone()).collect();
    }

    /// Runs the command; the tool ends when it is done.
    fn set(&mut self, to: UcsTo, cx: &mut Context<'_>) -> bool {
        let result = ucs_set::execute(
            &mut ExecutionContext::new(cx.doc),
            UcsSet {
                to,
                expected_revision: None,
            },
        );
        let Some(out) = points::written(result, cx) else {
            return false;
        };
        let f = cx.format();
        let said = match &out.ucs.name {
            Some(name) => format!("UCS: {name}"),
            None if out.ucs.origin.x == 0.0 && out.ucs.origin.y == 0.0 && out.ucs.angle == 0.0 => {
                "UCS: Dünya".to_owned()
            }
            None => format!(
                "UCS: başlangıç {}, açı {}",
                f.point(Vec2::new(out.ucs.origin.x, out.ucs.origin.y)),
                f.angle(out.ucs.angle)
            ),
        };
        cx.say(Level::Success, said);
        self.see(cx);
        true
    }

    /// The axes to come from `o` at `angle`, `arm` long.
    fn axes(&self, o: Vec2, angle: f64, preview: &mut Preview) {
        let (c, s) = (cos(angle), sin(angle));
        let x = Vec2::new(o.x + c * self.arm, o.y + s * self.arm);
        let y = Vec2::new(o.x - s * self.arm, o.y + c * self.arm);
        preview.guides.push([o, x]);
        preview.guides.push([o, y]);
        for (at, text) in [(x, "X"), (y, "Y")] {
            preview.labels.push(Label {
                at,
                text: text.to_owned(),
                offset: [4.0, -4.0],
                tone: Tone::Snap,
            });
        }
    }

    /// A typed angle as a turn: the project's unit, counter-clockwise in CAD,
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

impl Tool for UcsTool {
    fn id(&self) -> &'static str {
        match self.mode {
            Mode::Object => OBJECT_ID,
            Mode::TwoPoints => TWO_POINTS_ID,
            Mode::Origin => ORIGIN_ID,
            Mode::Rotate => ROTATE_ID,
            Mode::Named => NAMED_ID,
        }
    }

    fn label(&self) -> &'static str {
        match self.mode {
            Mode::Object => "UCS: Nesneye hizala",
            Mode::TwoPoints => "UCS: İki nokta",
            Mode::Origin => "UCS: Orijin",
            Mode::Rotate => "UCS: Döndür",
            Mode::Named => "Adlı UCS",
        }
    }

    fn prompt(&self) -> Prompt {
        let p = match (self.mode, self.origin, self.naming) {
            (Mode::Object, ..) => {
                Prompt::new(self.label(), "eksenlerin hizalanacağı nesneye tıklayın")
            }
            (Mode::TwoPoints, None, _) => {
                Prompt::new(self.label(), "yeni başlangıç noktasını verin")
            }
            (Mode::TwoPoints, Some(_), _) => {
                Prompt::new(self.label(), "x ekseni üzerinde bir nokta verin")
            }
            (Mode::Origin, ..) => Prompt::new(self.label(), "yeni başlangıç noktasını verin"),
            (Mode::Rotate, ..) => Prompt::new(
                self.label(),
                "dönüş açısını yazın ya da x ekseninin yönünü tıklayın",
            ),
            (Mode::Named, _, Naming::Restore) => {
                Prompt::new(self.label(), "geçilecek UCS'in adını yazın")
                    .option("Kaydet", "K")
                    .option("Sil", "S")
            }
            (Mode::Named, _, Naming::Save) => {
                Prompt::new(self.label(), "geçerli UCS'in kaydedileceği adı yazın")
            }
            (Mode::Named, _, Naming::Delete) => {
                Prompt::new(self.label(), "silinecek UCS'in adını yazın")
            }
        };
        p.note("Esc: bitir")
    }

    fn point_count(&self) -> usize {
        usize::from(self.origin.is_some())
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        if self.mode == Mode::Named && self.names.is_empty() {
            self.naming = Naming::Save;
        }
        Flow::Stay
    }

    fn snaps(&self) -> bool {
        self.mode != Mode::Object && self.mode != Mode::Named
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.origin
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.arm = cx.view.world_length(ARM_PX);
        self.hover = Some(match self.origin {
            Some(o) => points::constrain(Some(o), p, cx).0,
            None => p.world,
        });
        if self.mode == Mode::Object {
            let hit = cx.spatial.pick(p.raw, cx.pick_tolerance());
            cx.selection.set_hover(hit);
        }
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        match self.mode {
            Mode::Object => {
                let Some(slot) = cx.spatial.pick(p.raw, cx.pick_tolerance()) else {
                    cx.say(Level::Warn, "Eksenlerin hizalanacağı bir nesneye tıklayın.");
                    return;
                };
                let Some(uid) = cx.doc.uid(slot) else { return };
                cx.selection.set_hover(None);
                self.done = self.set(
                    UcsTo::Object {
                        uid: uid.to_string(),
                        at: wire(p.raw),
                    },
                    cx,
                );
            }
            Mode::TwoPoints => match self.origin {
                None => self.origin = Some(p.world),
                Some(o) => {
                    let x = points::constrain(Some(o), p, cx).0;
                    self.done = self.set(
                        UcsTo::TwoPoints {
                            origin: wire(o),
                            x_point: wire(x),
                        },
                        cx,
                    );
                }
            },
            Mode::Origin => {
                self.done = self.set(
                    UcsTo::Origin {
                        point: wire(p.world),
                    },
                    cx,
                )
            }
            // A click: its direction from the origin becomes the x axis.
            Mode::Rotate => {
                let o = self.now.0;
                let to = UcsTo::TwoPoints {
                    origin: wire(o),
                    x_point: wire(p.world),
                };
                self.done = self.set(to, cx);
            }
            Mode::Named => {}
        }
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let text = text.trim();
        match self.mode {
            Mode::Rotate => {
                let Some(turn) = Self::typed_turn(text, &cx.format()) else {
                    return false;
                };
                self.done = self.set(UcsTo::Rotate { angle: turn }, cx);
                true
            }
            Mode::Named if !text.is_empty() => {
                let name = text.to_owned();
                let to = match self.naming {
                    Naming::Restore => UcsTo::Named { name },
                    Naming::Save => UcsTo::Save { name },
                    Naming::Delete => UcsTo::Delete { name },
                };
                self.done = self.set(to, cx);
                true
            }
            Mode::TwoPoints | Mode::Origin => {
                let Some(p) = cx.typed_point(text, self.origin, self.hover) else {
                    return false;
                };
                match (self.mode, self.origin) {
                    (Mode::TwoPoints, None) => self.origin = Some(p),
                    (Mode::TwoPoints, Some(o)) => {
                        self.done = self.set(
                            UcsTo::TwoPoints {
                                origin: wire(o),
                                x_point: wire(p),
                            },
                            cx,
                        )
                    }
                    _ => self.done = self.set(UcsTo::Origin { point: wire(p) }, cx),
                }
                true
            }
            _ => false,
        }
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        if self.mode != Mode::Named || !matches!(key, "S") {
            return Vec::new();
        }
        self.names
            .iter()
            .map(|n| OptionChoice {
                label: n.clone(),
                typed: n.clone(),
                icon: None,
                preview: None,
                checked: false,
                command: None,
            })
            .collect()
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        if self.mode != Mode::Named {
            return false;
        }
        match key {
            "K" => {
                self.naming = Naming::Save;
                true
            }
            "S" if typed.is_empty() => {
                self.naming = Naming::Delete;
                true
            }
            "S" => {
                self.done = self.set(
                    UcsTo::Delete {
                        name: typed.to_owned(),
                    },
                    cx,
                );
                true
            }
            _ => false,
        }
    }

    fn takes_words(&self) -> bool {
        self.mode == Mode::Named
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        if self.origin.take().is_some() {
            return true;
        }
        if self.mode == Mode::Named && self.naming != Naming::Restore {
            self.naming = Naming::Restore;
            return true;
        }
        false
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        self.origin.take().is_some()
    }

    fn finished(&self) -> bool {
        self.done
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut preview = Preview::default();
        let Some(h) = self.hover else { return preview };
        match (self.mode, self.origin) {
            (Mode::TwoPoints, Some(o)) => {
                let a = atan2(h.y - o.y, h.x - o.x);
                self.axes(o, a, &mut preview);
            }
            (Mode::TwoPoints, None) | (Mode::Origin, _) => self.axes(h, self.now.1, &mut preview),
            (Mode::Rotate, _) => {
                let o = self.now.0;
                let a = atan2(h.y - o.y, h.x - o.x);
                self.axes(o, a, &mut preview);
            }
            _ => {}
        }
        preview
    }
}
