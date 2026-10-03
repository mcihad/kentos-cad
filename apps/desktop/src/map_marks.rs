//! The drawing's fixed marks, drawn after its text (labels.rs) as the web's
//! overlay draws them (`drawNorthArrow`, `drawScaleBar`,
//! apps/web/src/viewport/overlay.ts):
//!
//! - grid north at the top right: an arrow half filled, half outlined, in
//!   the drawing's ink, “K” (Kuzey) above it with a halo;
//! - the scale bar at the bottom right: 1, 2 or 5 times a power of ten,
//!   whichever is nearest 120 px on screen, in four alternating parts, “0”
//!   and its length above its ends.
//!
//! A CAD project has neither: its scene shows the coordinate axes' icon
//! instead (docs/adr/0165 §5; the web's `drawUcsIcon`), X to the right and
//! Y up from a small square, at the bottom left, or on the origin when 0,0
//! is on screen with room for it, as AutoCAD's UCS icon.
//!
//! They depend only on the view, so they are kept with the text's cached
//! picture.

use iced::alignment::Vertical;
use iced::widget::canvas::{Frame, Path, Stroke, Text};
use iced::widget::text;
use iced::{Color, Font, Pixels, Point, Size, Vector};
use kentos_interaction::Axes;
use kentos_render_wgpu::{Camera, Vec2};
use kentos_ui::theme::typography;

use crate::labels::Colors;

/// How far on screen the scale bar aims to reach, pixels (the web's `targetPx`).
const TARGET: f64 = 120.0;
/// The halo's width each side, as the text's (labels.rs).
const HALO: f32 = 1.5;

/// The coordinate axes' icon: how far from the drawing area's left and
/// bottom edges it stands, and its arms' length, pixels.
const UCS_MARGIN: f32 = 30.0;
const UCS_ARM: f32 = 38.0;

/// Draws a CBS project's grid north and scale bar over the drawing, or a CAD
/// project's coordinate axes (docs/adr/0165 §5).
pub fn paint(frame: &mut Frame, camera: &Camera, colors: &Colors, axes: Axes) {
    let size = frame.size();
    match axes {
        Axes::Gis => {
            north_arrow(frame, size, colors);
            scale_bar(frame, camera.scale, size, colors);
        }
        Axes::Cad => ucs_icon(frame, ucs_at(camera, size), colors),
    }
}

/// Where the coordinate axes' icon stands: on the origin when 0,0 is on
/// screen with room for the arms and their names, else at the bottom left.
pub fn ucs_at(camera: &Camera, size: Size) -> Point {
    let [x, y] = camera.world_to_screen(Vec2::new(0.0, 0.0));
    let (x, y) = (x as f32, y as f32);
    let room = UCS_ARM + 16.0;
    let on = x.is_finite()
        && y.is_finite()
        && x >= UCS_MARGIN
        && x + room <= size.width
        && y <= size.height - UCS_MARGIN
        && y - room >= 0.0;
    if on {
        Point::new(x, y)
    } else {
        Point::new(UCS_MARGIN, size.height - UCS_MARGIN)
    }
}

/// X to the right and Y up from a small square, each arm with its arrow and
/// name, in the drawing's ink.
fn ucs_icon(frame: &mut Frame, o: Point, colors: &Colors) {
    let ink = Stroke::default().with_color(colors.fg).with_width(1.5);
    let x_tip = Point::new(o.x + UCS_ARM, o.y);
    let y_tip = Point::new(o.x, o.y - UCS_ARM);
    frame.stroke(&Path::line(o, x_tip), ink);
    frame.stroke(&Path::line(o, y_tip), ink);
    let head = |a: Point, b: Point, c: Point| {
        Path::new(|p| {
            p.move_to(a);
            p.line_to(b);
            p.line_to(c);
            p.close();
        })
    };
    frame.fill(
        &head(
            Point::new(x_tip.x + 2.0, x_tip.y),
            Point::new(x_tip.x - 6.0, x_tip.y - 3.5),
            Point::new(x_tip.x - 6.0, x_tip.y + 3.5),
        ),
        colors.fg,
    );
    frame.fill(
        &head(
            Point::new(y_tip.x, y_tip.y - 2.0),
            Point::new(y_tip.x - 3.5, y_tip.y + 6.0),
            Point::new(y_tip.x + 3.5, y_tip.y + 6.0),
        ),
        colors.fg,
    );
    frame.stroke(
        &Path::rectangle(Point::new(o.x - 3.0, o.y - 3.0), Size::new(6.0, 6.0)),
        Stroke::default().with_color(colors.fg).with_width(1.0),
    );
    for (name, at, align) in [
        (
            "X",
            Point::new(x_tip.x + 6.0, x_tip.y + 6.0),
            text::Alignment::Left,
        ),
        (
            "Y",
            Point::new(y_tip.x, y_tip.y - 5.0),
            text::Alignment::Center,
        ),
    ] {
        haloed(
            frame,
            Mark {
                text: name,
                at,
                size: 11.0,
                font: typography::ui_strong(),
                align,
                color: colors.fg,
            },
            colors.halo,
        );
    }
}

/// The scale bar's length in metres and what its right end says, for a view
/// of `scale` pixels a metre (the web's `drawScaleBar`): 1, 2, 5 or 10 times
/// the power of ten below 120 px, the nearest to 120 px (the first among
/// equals); “2 km”, “50 m”, “5.0 cm”.
pub fn scale_length(scale: f64) -> (f64, String) {
    let raw = TARGET / scale;
    let p = 10f64.powf(raw.log10().floor());
    let mut best = p;
    for m in [2.0, 5.0, 10.0] {
        let v = m * p;
        if (v * scale - TARGET).abs() < (best * scale - TARGET).abs() {
            best = v;
        }
    }
    let unit = if best >= 1000.0 {
        format!("{} km", best / 1000.0)
    } else if best >= 1.0 {
        format!("{best} m")
    } else {
        format!("{} cm", two_digits(best * 100.0))
    };
    (best, unit)
}

/// A number with two significant digits, as JavaScript's `toPrecision(2)`
/// writes one below 100: “50”, “5.0”, “0.50”.
fn two_digits(v: f64) -> String {
    if v == 0.0 || !v.is_finite() {
        return format!("{v:.1}");
    }
    let exponent = v.abs().log10().floor() as i32;
    let decimals = (1 - exponent).max(0) as usize;
    format!("{v:.decimals$}")
}

fn north_arrow(frame: &mut Frame, size: Size, colors: &Colors) {
    let (x, y) = (size.width - 30.0, 22.0);
    let filled = Path::new(|b| {
        b.move_to(Point::new(x, y + 6.0));
        b.line_to(Point::new(x + 6.0, y + 28.0));
        b.line_to(Point::new(x, y + 23.0));
        b.close();
    });
    frame.fill(&filled, colors.fg);
    let outlined = Path::new(|b| {
        b.move_to(Point::new(x, y + 6.0));
        b.line_to(Point::new(x - 6.0, y + 28.0));
        b.line_to(Point::new(x, y + 23.0));
        b.close();
    });
    frame.stroke(
        &outlined,
        Stroke::default().with_color(colors.fg).with_width(1.0),
    );
    haloed(
        frame,
        Mark {
            text: "K",
            at: Point::new(x, y + 3.0),
            size: 11.0,
            font: typography::ui_strong(),
            align: text::Alignment::Center,
            color: colors.fg,
        },
        colors.halo,
    );
}

fn scale_bar(frame: &mut Frame, scale: f64, size: Size, colors: &Colors) {
    if !(scale.is_finite() && scale > 0.0) {
        return;
    }
    let (length, unit) = scale_length(scale);
    let px = (length * scale) as f32;
    let (x0, y0) = (size.width - 20.0 - px, size.height - 22.0);
    for i in 0..4 {
        let color = if i % 2 == 1 { colors.halo } else { colors.fg };
        frame.fill_rectangle(
            Point::new(x0 + px / 4.0 * i as f32, y0),
            Size::new(px / 4.0, 4.0),
            color,
        );
    }
    frame.stroke(
        &Path::rectangle(Point::new(x0 + 0.5, y0 + 0.5), Size::new(px, 4.0)),
        Stroke::default().with_color(colors.fg).with_width(1.0),
    );
    for (label, x, align) in [
        ("0", x0, text::Alignment::Left),
        (unit.as_str(), x0 + px, text::Alignment::Right),
    ] {
        haloed(
            frame,
            Mark {
                text: label,
                at: Point::new(x, y0 - 3.0),
                size: 10.5,
                font: typography::ui(),
                align,
                color: colors.label,
            },
            colors.halo,
        );
    }
}

/// A mark's text: its bottom on `at`, aligned there as `align` says.
struct Mark<'t> {
    text: &'t str,
    at: Point,
    size: f32,
    font: Font,
    align: text::Alignment,
    color: Color,
}

/// The text over eight copies of it in the halo's colour (the web's `haloText`).
fn haloed(frame: &mut Frame, mark: Mark<'_>, halo: Color) {
    let text = |position: Point, color: Color| Text {
        content: mark.text.to_owned(),
        position,
        max_width: f32::INFINITY,
        color,
        size: Pixels(mark.size),
        line_height: text::LineHeight::Relative(1.2),
        font: mark.font,
        align_x: mark.align,
        align_y: Vertical::Bottom,
        shaping: text::Shaping::Advanced,
    };
    for i in 0..8 {
        let a = i as f32 * std::f32::consts::FRAC_PI_4;
        frame.fill_text(text(
            mark.at + Vector::new(a.cos() * HALO, a.sin() * HALO),
            halo,
        ));
    }
    frame.fill_text(text(mark.at, mark.color));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scale_bar_takes_the_webs_length_and_words() {
        // 8 px a metre: 10 m is 80 px and 20 m 160 px, as near; the first stays.
        assert_eq!(scale_length(8.0), (10.0, "10 m".to_owned()));
        // 1 px a metre: 100 m is nearest 120 px.
        assert_eq!(scale_length(1.0), (100.0, "100 m".to_owned()));
        // 0.05 px a metre: 2 km is 100 px.
        assert_eq!(scale_length(0.05), (2000.0, "2 km".to_owned()));
        // 2000 px a metre: 5 cm is 100 px, written with two digits as the web's.
        let (length, unit) = scale_length(2000.0);
        assert!((length - 0.05).abs() < 1e-12, "{length}");
        assert_eq!(unit, "5.0 cm");
        // 400 px a metre: 20 cm is 80 px, 50 cm 200 px.
        assert_eq!(scale_length(400.0).1, "20 cm");
    }

    #[test]
    fn two_digits_are_javascripts_to_precision() {
        assert_eq!(two_digits(50.0), "50");
        assert_eq!(two_digits(5.0), "5.0");
        assert_eq!(two_digits(0.5), "0.50");
    }
}
