//! The measure tool (M) of the SVG editor (the web's `svgMeasure.ts`):
//! between two snapped points (drag, or click and click) it shows the
//! distance and the angle, in drawing units and in millimetres at the
//! symbol's size (Belge özellikleri's width in mm). With nothing being
//! measured, the path under the pointer shows the length of each segment.
//!
//! The angle is the drawing's own: y runs down, so it grows clockwise from
//! the right, from −180° to 180°, as ΔY, the rulers and a shape's Döndürme
//! read (the web gave it counter-clockwise from 0° to 360° beside a ΔY that
//! runs down, so one readout contradicted the other; both platforms now
//! say it the same way, and a measured edge's angle typed into Döndürme
//! lines a shape up with it).

use iced::widget::canvas::{Frame, Path, Stroke};
use iced::{Color, Point};
use kentos_svg_core::bezier::{bez, segment_count, segment_cubic, segment_length};
use kentos_svg_core::shape::Pt;

use super::paint::{View, outline};
use super::snap::{SnapOpts, tag};
use super::state::SvgEditor;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Stage {
    #[default]
    Idle,
    Placing,
    Done,
}

#[derive(Clone, Debug, Default)]
pub struct Measure {
    pub stage: Stage,
    pressed: bool,
    pub a: Option<Pt>,
    pub b: Option<Pt>,
    pub hover: Option<String>,
}

impl Measure {
    pub fn reset(&mut self) {
        *self = Measure::default();
    }
}

/// A number for labels: at most `digits` decimals, no trailing zeros (`fmtNum`).
pub fn fmt_num(v: f64, digits: u32) -> String {
    let s = kentos_native_style::classify::rounded(v, digits);
    if s == "-0" { "0".to_owned() } else { s }
}

/// The readout of a measure from a to b: the main line and ΔX, ΔY.
pub fn readout(a: Pt, b: Pt, width: f64, size_mm: Option<f64>) -> (String, String) {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let d = dx.hypot(dy);
    // As the drawing counts it: y runs down, so the angle grows clockwise, as a
    // shape's Döndürme does; −180° … 180°, so a line slightly up reads −20°.
    let ang = dy.atan2(dx).to_degrees();
    let mm = mm_of(d, width, size_mm);
    let main = format!(
        "{} birim{} · açı {}°",
        fmt_num(d, 3),
        if mm.is_empty() {
            String::new()
        } else {
            format!(" · {mm}")
        },
        fmt_num(ang, 2)
    );
    (main, format!("ΔX {}, ΔY {}", fmt_num(dx, 3), fmt_num(dy, 3)))
}

/// A length at the symbol's size, or nothing when the drawing has no size in mm.
pub fn mm_of(units: f64, width: f64, size_mm: Option<f64>) -> String {
    match size_mm {
        Some(mm) if mm > 0.0 => format!("{} mm", fmt_num(units * mm / width, 2)),
        _ => String::new(),
    }
}

impl SvgEditor {
    pub fn measure_down(&mut self, p: Pt) {
        if self.measure.stage == Stage::Placing
            && let Some(a) = self.measure.a
        {
            let b = self.snap(
                p,
                &SnapOpts {
                    from: Some(a),
                    no_grid: true,
                    ..SnapOpts::default()
                },
            );
            self.measure.b = Some(b);
            self.measure.stage = Stage::Done;
        } else {
            let a = self.snap(
                p,
                &SnapOpts {
                    no_grid: true,
                    ..SnapOpts::default()
                },
            );
            self.measure.a = Some(a);
            self.measure.b = Some(a);
            self.measure.stage = Stage::Placing;
        }
        self.measure.pressed = true;
        self.measure_report();
    }

    pub fn measure_move(&mut self, p: Pt) {
        if self.measure.stage == Stage::Placing
            && let Some(a) = self.measure.a
        {
            let b = self.snap(
                p,
                &SnapOpts {
                    from: Some(a),
                    no_grid: true,
                    ..SnapOpts::default()
                },
            );
            self.measure.b = Some(b);
            self.measure_report();
        } else if self.measure.stage != Stage::Done {
            let id = self.shape_at(p);
            if id != self.measure.hover {
                self.measure.hover = id;
                self.measure_hover_status();
            }
        }
    }

    pub fn measure_up(&mut self) {
        // A drag measures on release; a click waits for the second click.
        if self.measure.pressed
            && self.measure.stage == Stage::Placing
            && let (Some(a), Some(b)) = (self.measure.a, self.measure.b)
            && (b[0] - a[0]).hypot(b[1] - a[1]) * self.camera.zoom > 4.0
        {
            self.measure.stage = Stage::Done;
        }
        self.measure.pressed = false;
    }

    pub fn measure_cancel(&mut self) -> bool {
        if self.measure.stage == Stage::Idle {
            return false;
        }
        self.measure.reset();
        self.status("", false);
        true
    }

    fn measure_report(&mut self) {
        let (Some(a), Some(b)) = (self.measure.a, self.measure.b) else {
            return;
        };
        let (main, more) = readout(a, b, self.doc.width, self.doc.size_mm);
        let hint = if self.doc.size_mm.is_some_and(|v| v > 0.0) {
            ""
        } else {
            " · mm için Dosya → Belge özellikleri’nden sembol boyunu verin"
        };
        self.say(format!("Ölçü: {main} · {more}{hint}"));
    }

    /// The hovered path's length in the status line.
    fn measure_hover_status(&mut self) {
        let Some(s) = self.measure.hover.as_deref().and_then(|id| self.doc.shape(id)) else {
            return;
        };
        let Some(subs) = outline(s) else {
            return;
        };
        let mut count = 0usize;
        let mut total = 0.0;
        for sp in &subs {
            for i in 0..segment_count(sp) {
                total += segment_length(sp, i);
                count += 1;
            }
        }
        let mm = mm_of(total, self.doc.width, self.doc.size_mm);
        self.say(format!(
            "Yol uzunluğu {} birim{}, {count} parça. İki noktayı ölçmek için tıklayıp sürükleyin.",
            fmt_num(total, 3),
            if mm.is_empty() {
                String::new()
            } else {
                format!(" ({mm})")
            }
        ));
    }
}

/// The measure's line and labels, or the hovered path's segment lengths.
pub fn draw_measure(frame: &mut Frame, ed: &SvgEditor, view: &View, accent: Color, text: Color, halo: Color) {
    let m = &ed.measure;
    if let (Some(a), Some(b)) = (m.a, m.b)
        && m.stage != Stage::Idle
    {
        let (p1, p2) = (view.point(a), view.point(b));
        frame.stroke(
            &Path::line(p1, p2),
            Stroke::default().with_color(accent).with_width(1.4),
        );
        for p in [p1, p2] {
            frame.fill(&Path::circle(p, 3.0), accent);
        }
        let (main, more) = readout(a, b, ed.doc.width, ed.doc.size_mm);
        let mx = (p1.x + p2.x) / 2.0 + 10.0;
        let my = (p1.y + p2.y) / 2.0 - 10.0;
        tag(frame, &main, Point::new(mx, my), accent, halo, true);
        tag(frame, &more, Point::new(mx, my + 15.0), accent, halo, true);
        return;
    }
    let Some(s) = m.hover.as_deref().and_then(|id| ed.doc.shape(id)) else {
        return;
    };
    let Some(subs) = outline(s) else {
        return;
    };
    let mut count = 0;
    for sp in &subs {
        for i in 0..segment_count(sp) {
            count += 1;
            if count > 81 {
                return;
            }
            let len = segment_length(sp, i);
            if len * view.zoom < 24.0 {
                continue;
            }
            let at = view.point(bez(&segment_cubic(sp, i), 0.5));
            tag(
                frame,
                &fmt_num(len, 2),
                Point::new(at.x + 4.0, at.y - 4.0),
                text,
                halo,
                false,
            );
        }
    }
}
