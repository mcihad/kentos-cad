//! Rulers along the top and left of the SVG editor's canvas, in drawing
//! units, and the guides pulled out of them (the web's `svgRulers.ts`,
//! Inkscape's): drag from the top ruler for a horizontal guide, from the
//! left one for a vertical guide. A guide is dragged to move it (snapping
//! like a point) and dragged back onto a ruler to delete it; a double click
//! opens its position and angle, with Sil. Guides are snap targets and live
//! in the drawing (undo keeps them); the SVG file does not.

use iced::widget::canvas::{Frame, LineDash, Path, Stroke, Text};
use iced::{Color, Pixels, Point, Radians, Size, Vector};
use kentos_svg_core::shape::Pt;
use kentos_ui::theme::typography;

use super::camera::ruler;
use super::doc::{GuideLine, shape_id};
use super::hit::{Axis, Hit};
use super::measure::fmt_num;
use super::snap::SnapOpts;
use super::state::SvgEditor;

#[derive(Clone, Debug)]
struct GuideOp {
    id: String,
    created: bool,
    over_ruler: bool,
}

/// A guide's position typed in its window.
#[derive(Clone, Debug, PartialEq)]
pub struct GuidePopup {
    pub id: String,
    /// Where it opened (screen, in the stage).
    pub at: [f64; 2],
    pub x: String,
    pub y: String,
    pub angle: String,
    /// The guide is horizontal: its Y comes first.
    pub horizontal: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Rulers {
    op: Option<GuideOp>,
    pub popup: Option<GuidePopup>,
}

impl Rulers {
    pub fn busy(&self) -> bool {
        self.op.is_some()
    }

    /// The guide being dragged back onto a ruler (drawn red, dashed).
    fn deleting(&self, id: &str) -> bool {
        self.op.as_ref().is_some_and(|o| o.id == id && o.over_ruler)
    }
}

/// The smallest 1-2-5 step not below `v` (`niceStep`).
pub fn nice_step(v: f64) -> f64 {
    let e = 10f64.powf(v.log10().floor());
    let m = v / e;
    (if m <= 1.0 {
        1.0
    } else if m <= 2.0 {
        2.0
    } else if m <= 5.0 {
        5.0
    } else {
        10.0
    }) * e
}

impl SvgEditor {
    /// Which ruler a screen point is on.
    fn ruler_at(&self, s: [f64; 2]) -> Option<Axis> {
        if !self.options.rulers {
            return None;
        }
        let r = ruler();
        if s[1] <= r && s[0] > r {
            Some(Axis::H)
        } else if s[0] <= r && s[1] > r {
            Some(Axis::V)
        } else {
            None
        }
    }

    /// A press on a ruler (a new guide) or on a guide (to move it).
    pub fn ruler_down(&mut self, p: Pt, hit: &Hit) -> bool {
        match hit {
            Hit::Guide(id) => {
                self.rulers.popup = None;
                self.begin();
                self.rulers.op = Some(GuideOp {
                    id: id.clone(),
                    created: false,
                    over_ruler: false,
                });
                true
            }
            Hit::Ruler(axis) => {
                self.rulers.popup = None;
                self.begin();
                let id = shape_id();
                let q = self.snap(
                    p,
                    &SnapOpts {
                        guide: Some(id.clone()),
                        ..SnapOpts::default()
                    },
                );
                self.doc.guides.push(GuideLine {
                    id: id.clone(),
                    x: q[0],
                    y: q[1],
                    angle: if *axis == Axis::H { 0.0 } else { 90.0 },
                });
                self.rulers.op = Some(GuideOp {
                    id,
                    created: true,
                    over_ruler: true,
                });
                self.touch();
                true
            }
            _ => false,
        }
    }

    pub fn ruler_move(&mut self, s: [f64; 2], p: Pt) -> bool {
        let Some(op) = self.rulers.op.clone() else {
            return false;
        };
        let q = self.snap(
            p,
            &SnapOpts {
                guide: Some(op.id.clone()),
                ..SnapOpts::default()
            },
        );
        let over = self.ruler_at(s).is_some();
        let Some(g) = self.doc.guides.iter_mut().find(|g| g.id == op.id) else {
            return true;
        };
        g.x = q[0];
        g.y = q[1];
        let text = if over {
            "Bırakınca kılavuz silinir.".to_owned()
        } else if g.angle == 0.0 {
            format!("Kılavuz: Y {}", fmt_num(g.y, 3))
        } else if g.angle == 90.0 {
            format!("Kılavuz: X {}", fmt_num(g.x, 3))
        } else {
            format!(
                "Kılavuz: {}, {} · {}°",
                fmt_num(g.x, 3),
                fmt_num(g.y, 3),
                fmt_num(g.angle, 3)
            )
        };
        if let Some(o) = &mut self.rulers.op {
            o.over_ruler = over;
        }
        self.say(text);
        self.touch();
        true
    }

    pub fn ruler_up(&mut self) -> bool {
        let Some(op) = self.rulers.op.take() else {
            return false;
        };
        if op.over_ruler {
            self.doc.guides.retain(|g| g.id != op.id);
        }
        let label = match (op.over_ruler, op.created) {
            (true, true) => "",
            (true, false) => "Kılavuzu sil",
            (false, true) => "Kılavuz ekle",
            (false, false) => "Kılavuzu taşı",
        };
        self.commit(label);
        self.snapper.reset();
        if op.over_ruler || !op.created {
            self.status("", false);
        } else {
            self.say("Kılavuz eklendi: sürükleyerek taşıyın, cetvele geri bırakınca silinir, çift tık konumunu açar.");
        }
        true
    }

    /// A double click on a guide: its position and angle, typed.
    pub fn ruler_dbl(&mut self, s: [f64; 2], hit: &Hit) -> bool {
        let Hit::Guide(id) = hit else {
            return false;
        };
        let Some(g) = self.doc.guides.iter().find(|g| &g.id == id) else {
            return false;
        };
        self.rulers.popup = Some(GuidePopup {
            id: id.clone(),
            at: s,
            x: fmt_num(g.x, 3),
            y: fmt_num(g.y, 3),
            angle: fmt_num(g.angle, 3),
            horizontal: g.angle == 0.0,
        });
        true
    }

    /// Tamam in the guide's window: the typed values, or a warning.
    pub fn guide_apply(&mut self) {
        let Some(pop) = self.rulers.popup.clone() else {
            return;
        };
        let read = |t: &str| {
            let v = kentos_native_style::classify::js_number(&t.replacen(',', ".", 1));
            (!kentos_processing::text::js_trim(t).is_empty() && v.is_finite()).then_some(v)
        };
        let (Some(x), Some(y), Some(a)) = (read(&pop.x), read(&pop.y), read(&pop.angle)) else {
            self.warn("Kılavuz için sayı yazın (ondalık ayırıcı nokta).");
            return;
        };
        self.begin();
        if let Some(g) = self.doc.guides.iter_mut().find(|g| g.id == pop.id) {
            g.x = x;
            g.y = y;
            g.angle = ((a % 180.0) + 180.0) % 180.0;
        }
        self.commit("Kılavuz konumu");
        self.snapper.reset();
        self.rulers.popup = None;
    }

    pub fn remove_guide(&mut self, id: &str) {
        self.begin();
        self.doc.guides.retain(|g| g.id != id);
        self.commit("Kılavuzu sil");
        self.snapper.reset();
        self.rulers.popup = None;
    }

    pub fn clear_guides(&mut self) {
        if self.doc.guides.is_empty() {
            return;
        }
        self.begin();
        self.doc.guides.clear();
        self.commit("Kılavuzları sil");
        self.snapper.reset();
    }
}

/// Colours of the rulers and guides.
pub struct RulerColors {
    pub panel_head: Color,
    pub line: Color,
    pub text: Color,
    pub accent: Color,
    pub info: Color,
    pub danger: Color,
}

/// Guides across the whole stage (under the handles).
pub fn draw_guides(frame: &mut Frame, ed: &SvgEditor, size: Size, c: &RulerColors) {
    let far = f64::from(size.width + size.height) * 2.0;
    for g in &ed.doc.guides {
        let [x, y] = ed.camera.to_screen([g.x, g.y]);
        let (dy, dx) = g.angle.to_radians().sin_cos();
        let a = Point::new((x - dx * far) as f32, (y - dy * far) as f32);
        let b = Point::new((x + dx * far) as f32, (y + dy * far) as f32);
        let deleting = ed.rulers.deleting(&g.id);
        frame.stroke(
            &Path::line(a, b),
            Stroke {
                line_dash: if deleting {
                    LineDash {
                        segments: &[5.0, 4.0],
                        offset: 0,
                    }
                } else {
                    LineDash::default()
                },
                ..Stroke::default()
                    .with_color(if deleting {
                        c.danger
                    } else {
                        Color { a: 0.85, ..c.info }
                    })
                    .with_width(1.0)
            },
        );
    }
}

/// The rulers (on top of everything): majors at least 56 px apart on a 1-2-5
/// scale with their values, minors a fifth (or half) of that, and the
/// canvas's extent along each.
pub fn draw_rulers(frame: &mut Frame, ed: &SvgEditor, size: Size, c: &RulerColors) {
    if !ed.options.rulers {
        return;
    }
    let r = ruler() as f32;
    let (w, h) = (size.width, size.height);
    let z = ed.camera.zoom;
    let edge = Stroke::default().with_color(c.line).with_width(1.0);
    for rect in [
        Path::rectangle(Point::ORIGIN, Size::new(w, r)),
        Path::rectangle(Point::ORIGIN, Size::new(r, h)),
    ] {
        frame.fill(&rect, c.panel_head);
        frame.stroke(&rect, edge);
    }
    let corner = Path::rectangle(Point::ORIGIN, Size::new(r, r));
    frame.fill(&corner, c.panel_head);
    frame.stroke(&corner, edge);
    let major = nice_step(56.0 / z);
    let minor = if major * z / 5.0 >= 5.0 {
        major / 5.0
    } else {
        major / 2.0
    };
    let o = ed.camera.to_screen([0.0, 0.0]);
    let label_size = typography::scaled(9.5);
    let ticks = Path::new(|b| {
        let mut along = |from: f64, to: f64, origin: f64, horizontal: bool| {
            let a = ((from - origin) / z / minor).ceil();
            let e = ((to - origin) / z / minor).floor();
            let mut k = a;
            while k <= e && k - a < 2000.0 {
                let value = k * minor;
                let is_major = (value / major - (value / major).round()).abs() < 1e-6;
                let s = (origin + value * z) as f32;
                let len = if is_major { r - 4.0 } else { 5.0 };
                if horizontal {
                    b.move_to(Point::new(s, r));
                    b.line_to(Point::new(s, r - len));
                } else {
                    b.move_to(Point::new(r, s));
                    b.line_to(Point::new(r - len, s));
                }
                k += 1.0;
            }
        };
        along(f64::from(r), f64::from(w), o[0], true);
        along(f64::from(r), f64::from(h), o[1], false);
    });
    frame.stroke(&ticks, Stroke::default().with_color(c.text).with_width(1.0));
    // The majors' values: across the top, and up the left side turned a quarter.
    let labels = |from: f64, to: f64, origin: f64, horizontal: bool, frame: &mut Frame| {
        let a = ((from - origin) / z / major).ceil();
        let e = ((to - origin) / z / major).floor();
        let mut k = a;
        while k <= e && k - a < 400.0 {
            let value = k * major;
            let s = (origin + value * z) as f32;
            let text = Text {
                content: fmt_num(value, 3),
                color: c.text,
                size: Pixels(label_size),
                font: typography::ui(),
                ..Text::default()
            };
            if horizontal {
                frame.fill_text(Text {
                    position: Point::new(s + 3.0, 1.0),
                    ..text
                });
            } else {
                frame.with_save(|f| {
                    f.translate(Vector::new(1.0, s - 3.0));
                    f.rotate(Radians(-std::f32::consts::FRAC_PI_2));
                    f.fill_text(Text {
                        position: Point::ORIGIN,
                        ..text
                    });
                });
            }
            k += 1.0;
        }
    };
    labels(f64::from(r), f64::from(w), o[0], true, frame);
    labels(f64::from(r), f64::from(h), o[1], false, frame);
    let [x1, y1] = ed.camera.to_screen([ed.doc.width, ed.doc.height]);
    let extent = Path::new(|b| {
        b.move_to(Point::new((o[0] as f32).max(r), r - 1.0));
        b.line_to(Point::new((x1 as f32).max(r), r - 1.0));
        b.move_to(Point::new(r - 1.0, (o[1] as f32).max(r)));
        b.line_to(Point::new(r - 1.0, (y1 as f32).max(r)));
    });
    frame.stroke(
        &extent,
        Stroke::default()
            .with_color(Color { a: 0.7, ..c.accent })
            .with_width(2.0),
    );
}
