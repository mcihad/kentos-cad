//! The 3D views' look around the scene (DESIGN.md §7.20, docs/adr/8073 §2): the sky the scene stands on, the north
//! arrow at the top right that turns with the camera and turns it north when pressed, the legend that folds at the
//! bottom left, a card at the top left while a view has one (the IFC viewer's section, docs/adr/8118), and the views'
//! menu. The building's and the road's 3B görünüm and the IFC viewer wear the same. A measured value's digits by its
//! unit, for a host that writes its numbers ([`digits`], [`plain`]).

use iced::widget::{container, mouse_area, stack, tooltip};
use iced::{Background, Color, Element, Fill, Gradient, Radians, Theme, gradient, mouse};
use kentos_model_view::{OrbitCamera, VIEWS};
use kentos_ui::theme::typography;
use kentos_ui::widget::legend::{self, Symbol};
use kentos_ui::widget::{Compass, Legend, Menu, Tip, tip};

/// The legend's heading, which folds it.
pub const LEGEND: &str = "Lejant";
/// The views menu's last item and the north arrow's deed.
pub const NORTH: &str = "Kuzeye bak";
/// The legend's width, at 12 px text: its longest name (Bulgulu mahal) and its symbol.
const LEGEND_WIDTH: f32 = 150.0;

/// A part's colour as a face toward the light shows it: the renderer writes the part's colour times the face's light
/// to the screen as it is (the program draws with the web's colours, no gamma step), so a face lit fully is the colour
/// itself.
pub fn screen(c: [f32; 4]) -> Color {
    Color::from_rgb(
        c[0].clamp(0.0, 1.0),
        c[1].clamp(0.0, 1.0),
        c[2].clamp(0.0, 1.0),
    )
}

/// `a` towards `b` by `t`.
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    Color::from_rgb(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
    )
}

/// The sky over a drawing area's `ground`: a little towards `blue` at the zenith, less at the foot, more above on a
/// light ground than on a dark one; no blue (the high-contrast ground) keeps the ground plain.
pub fn sky(ground: Color, blue: Option<Color>) -> Background {
    let Some(blue) = blue else {
        return Background::Color(ground);
    };
    let light = ground.r + ground.g + ground.b > 1.5;
    let (zenith, foot) = if light { (0.16, 0.03) } else { (0.03, 0.12) };
    // From the top (the stop at 0) down: the gradient's angle is a half turn from upward.
    Background::Gradient(Gradient::Linear(
        gradient::Linear::new(Radians(std::f32::consts::PI))
            .add_stop(0.0, mix(ground, blue, zenith))
            .add_stop(1.0, mix(ground, blue, foot)),
    ))
}

/// The legend over the scene: each part a face in its colour as a face toward the light shows it, its edge a shade
/// darker (a see-through part: its tint inside its colour's edge); its heading folds it (`fold`). It takes the pointer
/// over itself: a drag begun on it does not turn the model.
pub fn legend<'a, M: Clone + 'a>(
    swatches: &[(&str, [f32; 4])],
    open: bool,
    fold: M,
) -> Element<'a, M> {
    let mut l = Legend::new()
        .title(LEGEND)
        .collapsible(!open, fold)
        .width(typography::scaled(LEGEND_WIDTH));
    for &(name, c) in swatches {
        let shown = screen(c);
        let symbol = if c[3] < 1.0 {
            Symbol::Area {
                fill: Color {
                    a: c[3].max(0.25),
                    ..shown
                },
                stroke: shown,
            }
        } else {
            Symbol::Area {
                fill: shown,
                stroke: mix(shown, Color::BLACK, 0.35),
            }
        };
        l = l.item(symbol, name);
    }
    mouse_area(legend::frame(l))
        .interaction(mouse::Interaction::Idle)
        .into()
}

/// The north arrow: north where the camera shows it, its tip the side the model is seen from; pressed, `north`.
pub fn compass<'a, M: Clone + 'a>(orbit: &OrbitCamera, north: M) -> Element<'a, M> {
    tip(
        Compass::new(orbit.north_on_screen())
            .size(typography::scaled(38.0))
            .on_press(north),
        Tip::new("Kuzey").body(format!(
            "Model {} görülüyor, {:.0}° yukarıdan. Tıklayın: kamera kuzeye bakar.",
            orbit.seen_from(),
            orbit.elevation
        )),
        tooltip::Position::Left,
    )
}

/// The views, the one the camera is at chosen (`view` its place), and Kuzeye bak (`north`).
pub fn views_menu<M: Clone>(orbit: &OrbitCamera, view: impl Fn(usize) -> M, north: M) -> Menu<M> {
    let at = orbit.preset();
    let mut m = Menu::new();
    for (i, (name, ..)) in VIEWS.iter().enumerate() {
        m = m.radio(*name, at == Some(i), view(i));
    }
    m.separator().item(NORTH, north)
}

/// The views menu's button words: the view the camera is at, else Görünüş.
pub fn view_name(orbit: &OrbitCamera) -> &'static str {
    orbit.preset().map_or("Görünüş", |i| VIEWS[i].0)
}

/// The scene on its sky, the north arrow at its top right, the legend at its bottom left, a view's card (none: no
/// card) at its top left and another at its bottom right (the measurements', docs/adr/8121).
pub fn framed<'a, M: Clone + 'a>(
    scene: Element<'a, M>,
    compass: Element<'a, M>,
    legend: Element<'a, M>,
    card: Option<Element<'a, M>>,
    corner: Option<Element<'a, M>>,
    sky: Background,
) -> Element<'a, M> {
    let mut layers = stack![
        scene,
        container(compass)
            .align_right(Fill)
            .align_top(Fill)
            .padding(10),
        container(legend)
            .align_left(Fill)
            .align_bottom(Fill)
            .padding(10),
    ];
    // A card takes the pointer over itself: a drag on its slider or its list does not turn the model.
    if let Some(card) = card {
        layers = layers.push(
            container(mouse_area(card).interaction(mouse::Interaction::Idle))
                .align_left(Fill)
                .align_top(Fill)
                .padding(10),
        );
    }
    if let Some(corner) = corner {
        layers = layers.push(
            container(mouse_area(corner).interaction(mouse::Interaction::Idle))
                .align_right(Fill)
                .align_bottom(Fill)
                .padding(10),
        );
    }
    container(layers)
        .width(Fill)
        .height(Fill)
        .style(move |_: &Theme| container::Style {
            background: Some(sky),
            ..container::Style::default()
        })
        .into()
}

/// The units of length a file's values come in; a volume's are these cubed, an area's squared.
const LENGTHS: [&str; 6] = ["m", "mm", "cm", "km", "ft", "in"];

/// The decimals a measured value of `unit` is written with: a length or a volume the host's `length`, an area its
/// `area`; none for a number without a unit of length (a count, a ratio, an angle), written with [`plain`].
pub fn digits(unit: Option<&str>, length: usize, area: usize) -> Option<usize> {
    let u = unit?.trim();
    if u.ends_with('²') {
        Some(area)
    } else if u.ends_with('³') || LENGTHS.contains(&u) {
        Some(length)
    } else {
        None
    }
}

/// A number with the digits it needs, six at most and no trailing zeros: `21.6` for a file's `21.599999999999998`.
pub fn plain(v: f64) -> String {
    let t = format!("{v:.6}");
    let t = t.trim_end_matches('0').trim_end_matches('.');
    if t == "-0" {
        "0".to_owned()
    } else {
        t.to_owned()
    }
}
