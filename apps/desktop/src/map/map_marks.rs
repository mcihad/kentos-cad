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
//! Y up from a small square, always at the bottom left (the owner's wish of
//! 12 Oct: it stood on the origin when 0,0 was on screen, in the middle of a
//! local drawing). Being fixed, it was read as the origin (usage review
//! UR-005): the real 0,0 has a mark of its own when it is in the view, a
//! ringed cross with “0,0” beside it, never the corner icon's arrows.
//!
//! They depend only on the view, so they are kept with the text's cached
//! picture.

use iced::alignment::Vertical;
use iced::widget::canvas::{Frame, Path, Stroke, Text};
use iced::widget::text;
use iced::{Color, Font, Pixels, Point, Size, Vector};
use kentos_interaction::Axes;
use kentos_render_wgpu::Camera;
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
/// project's coordinate axes (docs/adr/0165 §5); the scale bar `lift` pixels
/// higher, over the map services' credits (docs/adr/0208 §3).
/// A user coordinate system (docs/adr/0261 §4) turns the icon with its axes:
/// the CAD project's always, a CBS project's shows it beside its marks when
/// it is not the world's (`ucs`: its angle; none, the world's).
pub fn paint(
    frame: &mut Frame,
    camera: &Camera,
    colors: &Colors,
    axes: Axes,
    ucs: Option<f64>,
    lift: f32,
) {
    let size = frame.size();
    // A twisted view (docs/adr/0285 §2): grid north and the axes turn with the drawing.
    let twist = camera.twist;
    match axes {
        Axes::Gis => {
            north_arrow(frame, size, colors, twist);
            scale_bar(
                frame,
                camera.scale,
                Size::new(size.width, size.height - lift),
                colors,
            );
            if let Some(a) = ucs {
                ucs_icon(frame, ucs_at(size), colors, a - twist, ["Y", "X"], false);
            }
        }
        Axes::Cad => {
            origin_mark(frame, camera, colors);
            ucs_icon(
                frame,
                ucs_at(size),
                colors,
                ucs.unwrap_or(0.0) - twist,
                ["X", "Y"],
                ucs.is_none(),
            );
        }
    }
}

/// How far the origin's mark reaches from its middle, pixels.
const ORIGIN_ARM: f32 = 9.0;

/// Where the drawing's 0,0 stands on screen, when it is in the view and clear of the corner icon (the icon says
/// the axes' directions there; the mark would only be crowded under it).
pub fn origin_at(camera: &Camera, size: Size) -> Option<Point> {
    let [x, y] = camera.world_to_screen(kentos_render_wgpu::Vec2::new(0.0, 0.0));
    let at = Point::new(x as f32, y as f32);
    let corner = ucs_at(size);
    let inside = (0.0..=size.width).contains(&at.x) && (0.0..=size.height).contains(&at.y);
    let clear = (at.x - corner.x).abs() > UCS_ARM + 12.0 || (corner.y - at.y) > UCS_ARM + 12.0;
    (x.is_finite() && y.is_finite() && inside && clear).then_some(at)
}

/// The drawing's 0,0 (UR-005): a small ring with a cross through it and “0,0” above to the right, in the ink's
/// colour with a halo, so it reads on lines and fills alike.
fn origin_mark(frame: &mut Frame, camera: &Camera, colors: &Colors) {
    let Some(o) = origin_at(camera, frame.size()) else {
        return;
    };
    let halo = Stroke::default().with_color(colors.halo).with_width(3.5);
    let ink = Stroke::default().with_color(colors.fg).with_width(1.25);
    let cross = Path::new(|p| {
        p.move_to(Point::new(o.x - ORIGIN_ARM, o.y));
        p.line_to(Point::new(o.x + ORIGIN_ARM, o.y));
        p.move_to(Point::new(o.x, o.y - ORIGIN_ARM));
        p.line_to(Point::new(o.x, o.y + ORIGIN_ARM));
    });
    let ring = Path::circle(o, 4.5);
    for stroke in [halo, ink] {
        frame.stroke(&cross, stroke);
        frame.stroke(&ring, stroke);
    }
    haloed(
        frame,
        Mark {
            text: "0,0",
            at: Point::new(o.x + ORIGIN_ARM + 3.0, o.y - ORIGIN_ARM - 3.0),
            size: 11.0,
            font: typography::ui_strong(),
            align: text::Alignment::Left,
            color: colors.fg,
        },
        colors.halo,
    );
}

/// Where the coordinate axes' icon stands: the bottom left of the drawing
/// area, wherever the origin is.
pub fn ucs_at(size: Size) -> Point {
    Point::new(UCS_MARGIN, size.height - UCS_MARGIN)
}

/// X to the right and Y up from a small square, each arm with its arrow and
/// name, in the drawing's ink; turned by `angle` (the UCS's, counter-clockwise
/// on the ground, so the screen's way round), the square only for the world's
/// axes (AutoCAD's: a UCS's icon has none).
fn ucs_icon(
    frame: &mut Frame,
    o: Point,
    colors: &Colors,
    angle: f64,
    names: [&str; 2],
    world: bool,
) {
    let ink = Stroke::default().with_color(colors.fg).with_width(1.5);
    // Screen y grows downward: a turn counter-clockwise on the ground is clockwise here.
    let (c, s) = (angle.cos() as f32, angle.sin() as f32);
    let turn = |dx: f32, dy: f32| Point::new(o.x + dx * c + dy * s, o.y - dx * s + dy * c);
    let x_tip = turn(UCS_ARM, 0.0);
    let y_tip = turn(0.0, -UCS_ARM);
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
            turn(UCS_ARM + 2.0, 0.0),
            turn(UCS_ARM - 6.0, -3.5),
            turn(UCS_ARM - 6.0, 3.5),
        ),
        colors.fg,
    );
    frame.fill(
        &head(
            turn(0.0, -UCS_ARM - 2.0),
            turn(-3.5, -UCS_ARM + 6.0),
            turn(3.5, -UCS_ARM + 6.0),
        ),
        colors.fg,
    );
    if world {
        frame.stroke(
            &Path::rectangle(Point::new(o.x - 3.0, o.y - 3.0), Size::new(6.0, 6.0)),
            Stroke::default().with_color(colors.fg).with_width(1.0),
        );
    }
    let (x_at, x_align) = if world {
        (turn(UCS_ARM + 6.0, 6.0), text::Alignment::Left)
    } else {
        (turn(UCS_ARM + 9.0, 4.0), text::Alignment::Center)
    };
    for (name, at, align) in [
        (names[0], x_at, x_align),
        (names[1], turn(0.0, -UCS_ARM - 5.0), text::Alignment::Center),
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

fn north_arrow(frame: &mut Frame, size: Size, colors: &Colors, twist: f64) {
    if twist != 0.0 {
        // Turned about its middle as the view turns the north (clockwise on the screen).
        let middle = Vector::new(size.width - 30.0, 22.0 + 17.0);
        frame.with_save(|f| {
            f.translate(middle);
            f.rotate(twist as f32);
            f.translate(Vector::new(-middle.x, -middle.y));
            north_arrow(f, size, colors, 0.0);
        });
        return;
    }
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

    /// UR-005: 0,0 has its own mark where it is on screen, none when it is out of the view or under the corner
    /// icon; panned and zoomed, it follows the drawing, the icon stays.
    #[test]
    fn the_origins_mark_stands_on_zero_zero_and_only_there() {
        let size = Size::new(800.0, 600.0);
        let mut camera = Camera {
            center: kentos_render_wgpu::Vec2::new(0.0, 0.0),
            scale: 10.0,
            width: 800.0,
            height: 600.0,
            ..Camera::default()
        };
        assert_eq!(origin_at(&camera, size), Some(Point::new(400.0, 300.0)));
        // 30 m to the left and 20 m down: 0,0 is 300 px right and 200 px up of the middle.
        camera.center = kentos_render_wgpu::Vec2::new(-30.0, -20.0);
        assert_eq!(origin_at(&camera, size), Some(Point::new(700.0, 100.0)));
        // Negative coordinates on screen only: 0,0 is out of the view.
        camera.center = kentos_render_wgpu::Vec2::new(-100.0, -100.0);
        assert_eq!(origin_at(&camera, size), None);
        // A projected drawing's coordinates, far from 0,0: no mark.
        camera.center = kentos_render_wgpu::Vec2::new(486_512.0, 4_420_118.0);
        assert_eq!(origin_at(&camera, size), None);
        // Under the corner icon (30 px from the left and bottom edges): left to the icon.
        camera.center = kentos_render_wgpu::Vec2::new(37.0, 27.0);
        let corner = ucs_at(size);
        let [x, y] = camera.world_to_screen(kentos_render_wgpu::Vec2::new(0.0, 0.0));
        assert!((x as f32 - corner.x).abs() < 1.0 && (y as f32 - corner.y).abs() < 1.0);
        assert_eq!(origin_at(&camera, size), None);
    }

    /// UR-005's pictures (`.run/gui-evidence/kullanim-UR-005/`): a local CAD drawing, a line from 0,0 to 10,0 and
    /// a rectangle in the negative quarter, Tümünü göster, in both themes:
    ///
    /// ```text
    /// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib map_marks::tests::origin_screens -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn origin_screens() {
        use crate::app::{App, Message};
        use crate::tools_scenes::{E, N, Objects, open};
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kullanim-UR-005");
        std::fs::create_dir_all(&out).expect("a folder");
        for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
            let (mut app, _) = App::boot(None);
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let mut o = Objects::new();
            o.line("cizim", [-E, -N], [10.0 - E, -N]);
            o.path("parsel", &[[-8.0 - E, -6.0 - N], [-2.0 - E, -6.0 - N], [-2.0 - E, -2.0 - N], [-8.0 - E, -2.0 - N]], true);
            open(&mut app, o);
            crate::files_testing::make_cad(&mut app);
            let doc = app.document.as_mut().expect("a drawing");
            let mut settings = doc.settings().clone();
            settings.srid = 0;
            settings.second_srid = None;
            doc.model.set_settings(settings);
            let _ = app.update(Message::Run("view.zoomExtents"));
            let size = Size::new(1280.0, 800.0);
            let mut snapshot = kentos_ui::snapshot::Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("orijin-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }

    #[test]
    fn two_digits_are_javascripts_to_precision() {
        assert_eq!(two_digits(50.0), "50");
        assert_eq!(two_digits(5.0), "5.0");
        assert_eq!(two_digits(0.5), "0.50");
    }
}
