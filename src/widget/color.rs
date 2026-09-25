//! Renk seçici ve renk rampası.
//!
//! ```text
//! [■ #E2A93B      ⌄]      ██████████████ rampa █████████████████
//! ┌──────────────────┐     ▲         ▲                 ▲        ▲
//! │  doygunluk ×     │    [■ #F46D43 ⌄] [K  42 %] [Sil] [Ters] [Hazır ⌄]
//! │  parlaklık       │
//! │ ═══ ton ════════ │
//! │ ═══ saydamlık ══ │
//! │ [▮▮] # E2A93B    │
//! │ ■ ■ ■ ■ ■ ■ ■ ■  │  hazır renkler, son kullanılanlar
//! └──────────────────┘
//! ```
//!
//! [`ColorPicker`] alanın altında açılan panelde rengi seçtirir: doygunluk ve
//! parlaklık düzlemi, ton ve (istenirse) saydamlık şeritleri, onaltılık
//! giriş, hazır renkler ve uygulamanın verdiği son kullanılan renkler.
//! Paneldeki önceki/yeni karşılaştırmasında önceki renge tıklamak geri
//! döndürür. Renk sürüklerken sürekli bildirilir.
//!
//! [`ramp`] renk rampası düzenleyicisidir: çubuğa tıklamak durak ekler,
//! durak sürüklenerek taşınır, çubuğun altına doğru uzağa sürüklenip
//! bırakılan durak silinir. Seçili durağın rengi ve yeri alttaki satırda
//! düzenlenir; rampa ters çevrilir ya da hazır rampalardan biri seçilir.
//!
//! ```ignore
//! ColorPicker::new(layer.color, Message::ColorChanged)
//!     .alpha()
//!     .recent(self.recent_colors.iter().copied())
//!
//! color::ramp(&self.ramp, self.stop, Message::RampChanged)
//! ```

use std::rc::Rc;

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::gradient::Linear;
use iced::widget::{Column, Row, button, column, container, row, space, text_input};
use iced::{
    Background, Border, Center, Color, Degrees, Element, Event, Fill, Gradient, Length, Point,
    Rectangle, Renderer, Shadow, Size, Theme, Vector, border, mouse,
};

use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::{Tokens, typography};
use crate::widget::context_menu::{Menu, MenuButton};
use crate::widget::dropdown::{Dropdown, Reaction};
use crate::widget::number::{NumberInput, units};

/// Panelin genişliği ve düzlemin yüksekliği.
const PANEL: f32 = 236.0;
const PLANE: f32 = 150.0;
/// Şeritlerin yüksekliği.
const STRIP: f32 = 12.0;
/// Hazır renk kutusu ve bir satırdaki sayısı.
const SWATCH: f32 = 16.0;
const PER_ROW: usize = 10;
/// Satranç tahtası karelerinin kenarı (saydamlığı göstermek için).
const CHECKER: f32 = 5.0;

/// Ton, doygunluk, parlaklık ve saydamlık. Ton 0–360, ötekiler 0–1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hsva {
    pub hue: f32,
    pub saturation: f32,
    pub value: f32,
    pub alpha: f32,
}

impl Hsva {
    /// Grilerde ton belirsizdir; 0 alınır.
    pub fn from_color(color: Color) -> Self {
        let max = color.r.max(color.g).max(color.b);
        let min = color.r.min(color.g).min(color.b);
        let delta = max - min;

        let hue = if delta <= f32::EPSILON {
            0.0
        } else if max == color.r {
            60.0 * ((color.g - color.b) / delta).rem_euclid(6.0)
        } else if max == color.g {
            60.0 * ((color.b - color.r) / delta + 2.0)
        } else {
            60.0 * ((color.r - color.g) / delta + 4.0)
        };

        Self {
            hue,
            saturation: if max <= f32::EPSILON {
                0.0
            } else {
                delta / max
            },
            value: max,
            alpha: color.a,
        }
    }

    pub fn to_color(self) -> Color {
        let hue = self.hue.rem_euclid(360.0) / 60.0;
        let chroma = self.value * self.saturation;
        let x = chroma * (1.0 - (hue.rem_euclid(2.0) - 1.0).abs());
        let (r, g, b) = match hue as u32 {
            0 => (chroma, x, 0.0),
            1 => (x, chroma, 0.0),
            2 => (0.0, chroma, x),
            3 => (0.0, x, chroma),
            4 => (x, 0.0, chroma),
            _ => (chroma, 0.0, x),
        };
        let m = self.value - chroma;

        Color::from_rgba(r + m, g + m, b + m, self.alpha)
    }

    /// Aynı tonun tam doygun ve parlak rengi.
    fn pure(self) -> Color {
        Hsva {
            saturation: 1.0,
            value: 1.0,
            alpha: 1.0,
            ..self
        }
        .to_color()
    }
}

/// Rengin onaltılık yazımı: `#E2A93B`, saydamsa `#E2A93B80`.
pub fn to_hex(color: Color) -> String {
    let [r, g, b, a] = color.into_rgba8();

    if a == u8::MAX {
        format!("#{r:02X}{g:02X}{b:02X}")
    } else {
        format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
    }
}

/// Onaltılık rengi çözümler: `#RGB`, `#RRGGBB` ya da `#RRGGBBAA`; baştaki #
/// isteğe bağlıdır.
pub fn parse_hex(text: &str) -> Option<Color> {
    let digits = text.trim().trim_start_matches('#');

    if !digits.chars().all(|digit| digit.is_ascii_hexdigit()) {
        return None;
    }

    let channel = |index: usize, width: usize| {
        u8::from_str_radix(&digits[index * width..(index + 1) * width], 16).ok()
    };

    match digits.len() {
        3 => {
            let [r, g, b] = [0, 1, 2].map(|index| channel(index, 1).map(|value| value * 17));
            Some(Color::from_rgb8(r?, g?, b?))
        }
        6 => Some(Color::from_rgb8(
            channel(0, 2)?,
            channel(1, 2)?,
            channel(2, 2)?,
        )),
        8 => Some(Color::from_rgba8(
            channel(0, 2)?,
            channel(1, 2)?,
            channel(2, 2)?,
            f32::from(channel(3, 2)?) / 255.0,
        )),
        _ => None,
    }
}

/// Hazır renkler: canlı, koyu ve gri tonlar.
pub const PALETTE: [u32; 30] = [
    0xe5534b, 0xee8446, 0xe2a93b, 0xc9c24a, 0x7cc05a, 0x2db5ac, 0x4c9be8, 0x6f7fe8, 0x9d86f0,
    0xe3689b, 0x9c2f28, 0xa8531f, 0x9a6b10, 0x7d7a1c, 0x3f7f30, 0x13766f, 0x1b5fa8, 0x3d4bb0,
    0x6547cf, 0xa33b6b, 0x000000, 0x1f2328, 0x3a3f46, 0x5b626b, 0x7d858f, 0xa1a8b0, 0xc3c8ce,
    0xe1e4e8, 0xf3f4f6, 0xffffff,
];

fn rgb(value: u32) -> Color {
    Color::from_rgb8((value >> 16) as u8, (value >> 8) as u8, value as u8)
}

/// Renk seçici: rengi ve onaltılık yazımını gösteren alan; tıklanınca
/// seçici paneli açılır.
pub struct ColorPicker<'a, Message> {
    color: Color,
    on_change: Rc<dyn Fn(Color) -> Message + 'a>,
    alpha: bool,
    palette: Vec<Color>,
    recent: Vec<Color>,
    width: Length,
}

impl<'a, Message: 'a> ColorPicker<'a, Message> {
    pub fn new(color: Color, on_change: impl Fn(Color) -> Message + 'a) -> Self {
        Self {
            color,
            on_change: Rc::new(on_change),
            alpha: false,
            palette: PALETTE.iter().map(|value| rgb(*value)).collect(),
            recent: Vec::new(),
            width: Length::Fixed(typography::scaled(132.0)),
        }
    }

    /// Saydamlık şeridi ve sekiz haneli onaltılık yazım.
    pub fn alpha(mut self) -> Self {
        self.alpha = true;
        self
    }

    /// Hazır renkler (varsayılan [`PALETTE`]).
    pub fn palette(mut self, colors: impl IntoIterator<Item = Color>) -> Self {
        self.palette = colors.into_iter().collect();
        self
    }

    /// Son kullanılan renkler; panelin altında ayrı satırda.
    pub fn recent(mut self, colors: impl IntoIterator<Item = Color>) -> Self {
        self.recent = colors.into_iter().collect();
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }
}

/// Paneldeki iç mesajlar.
#[derive(Debug, Clone)]
enum Pick {
    Plane(f32, f32),
    Hue(f32),
    Alpha(f32),
    Hex(String),
    HexSubmit,
    Swatch(Color),
}

/// Açık panelin durumu.
struct Picker {
    hsva: Hsva,
    hex: String,
    invalid: bool,
    /// Panel açıldığındaki renk; karşılaştırmada solda durur.
    original: Color,
}

impl Picker {
    fn new(color: Color) -> Self {
        Self {
            hsva: Hsva::from_color(color),
            hex: to_hex(color),
            invalid: false,
            original: color,
        }
    }

    fn set(&mut self, color: Color) {
        let hsva = Hsva::from_color(color);

        // Grilerde ve siyahta ton korunur: düzlemdeki işaret yerinde kalır.
        self.hsva = Hsva {
            hue: if hsva.saturation > 0.0 && hsva.value > 0.0 {
                hsva.hue
            } else {
                self.hsva.hue
            },
            ..hsva
        };
        self.hex = to_hex(color);
        self.invalid = false;
    }
}

/// Paneldeki değişmeyen ayarlar.
struct Props {
    alpha: bool,
    palette: Vec<Color>,
    recent: Vec<Color>,
}

impl<'a, Message: 'a> From<ColorPicker<'a, Message>> for Element<'a, Message> {
    fn from(picker: ColorPicker<'a, Message>) -> Self {
        let color = picker.color;
        let on_change = picker.on_change;
        let alpha = picker.alpha;
        let props = Rc::new(Props {
            alpha,
            palette: picker.palette,
            recent: picker.recent,
        });

        let reduce = move |state: &mut Picker, pick: Pick| -> Reaction<Message> {
            let current = |state: &mut Picker| {
                let mut color = state.hsva.to_color();

                if !alpha {
                    color.a = 1.0;
                }

                state.hex = to_hex(color);
                state.invalid = false;
                Reaction::publish(on_change(color))
            };

            match pick {
                Pick::Plane(saturation, value) => {
                    state.hsva.saturation = saturation;
                    state.hsva.value = value;
                    current(state)
                }
                Pick::Hue(hue) => {
                    state.hsva.hue = hue * 360.0;
                    current(state)
                }
                Pick::Alpha(opacity) => {
                    state.hsva.alpha = opacity;
                    current(state)
                }
                Pick::Hex(text) => {
                    // Yazarken yalnızca tam yazımlar (6 ya da 8 hane)
                    // uygulanır; kısa yazım Enter'la.
                    let digits = text.trim().trim_start_matches('#').len();
                    let parsed = parse_hex(&text).filter(|_| digits == 6 || digits == 8);

                    state.invalid = parse_hex(&text).is_none() && digits >= 6;
                    state.hex = text;

                    match parsed {
                        Some(color) => {
                            let hex = std::mem::take(&mut state.hex);
                            state.set(color);
                            state.hex = hex;
                            Reaction::publish(on_change(opaque(color, alpha)))
                        }
                        None => Reaction::stay(),
                    }
                }
                Pick::HexSubmit => match parse_hex(&state.hex) {
                    Some(color) => {
                        state.set(opaque(color, alpha));
                        Reaction::commit(on_change(opaque(color, alpha)))
                    }
                    None => {
                        state.invalid = true;
                        Reaction::stay()
                    }
                },
                Pick::Swatch(color) => {
                    state.set(opaque(color, alpha));
                    Reaction::publish(on_change(opaque(color, alpha)))
                }
            }
        };

        let anchor = container(
            row![
                container(space::horizontal())
                    .width(typography::scaled(SWATCH))
                    .height(typography::scaled(SWATCH))
                    .style(style::container::swatch(color)),
                label::mono(to_hex(color)).width(Fill),
                icon(Icon::ChevronDown).size(12.0).tone(Tone::Muted),
            ]
            .spacing(8)
            .align_y(Center),
        )
        .padding([3, 6])
        .width(picker.width)
        .style(style::container::field_box);

        Dropdown::new(
            anchor,
            move || Picker::new(color),
            move |state| panel(state, &props),
            reduce,
        )
        .into()
    }
}

fn opaque(color: Color, alpha: bool) -> Color {
    if alpha {
        color
    } else {
        Color { a: 1.0, ..color }
    }
}

/// Seçici paneli.
fn panel<'a>(state: &Picker, props: &Props) -> Element<'a, Pick> {
    let width = typography::scaled(PANEL);
    let current = state.hsva.to_color();

    let mut body = Column::new()
        .spacing(8)
        .width(width)
        .push(Plane::new(state.hsva, Pick::Plane).height(typography::scaled(PLANE)))
        .push(Strip::new(
            Channel::Hue(state.hsva),
            state.hsva.hue / 360.0,
            Pick::Hue,
        ));

    if props.alpha {
        body = body.push(Strip::new(
            Channel::Alpha(state.hsva),
            state.hsva.alpha,
            Pick::Alpha,
        ));
    }

    // Önceki ve yeni renk yan yana; önceki renge tıklamak geri döndürür.
    let compare = row![
        button(space::horizontal().width(Fill).height(Fill))
            .on_press(Pick::Swatch(state.original))
            .padding(0)
            .width(typography::scaled(22.0))
            .height(typography::scaled(24.0))
            .style(solid(state.original)),
        container(space::horizontal())
            .width(typography::scaled(22.0))
            .height(typography::scaled(24.0))
            .style(style::container::solid(current)),
    ];

    let hex = text_input("#RRGGBB", &state.hex)
        .on_input(Pick::Hex)
        .on_submit(Pick::HexSubmit)
        .font(typography::mono())
        .size(typography::body())
        .padding([3, 6])
        .style(style::field::validated(state.invalid));

    body = body.push(row![compare, hex].spacing(8).align_y(Center));
    body = body.push(swatches(&props.palette, current));

    if !props.recent.is_empty() {
        body = body
            .push(label::caption("Son kullanılanlar"))
            .push(swatches(&props.recent, current));
    }

    container(body)
        .padding(10)
        .style(style::container::popover)
        .into()
}

/// Renk kutuları, satır satır; seçili renk vurgu halkalıdır.
fn swatches<'a>(colors: &[Color], current: Color) -> Element<'a, Pick> {
    let same = |a: Color, b: Color| a.into_rgba8() == b.into_rgba8();

    colors
        .chunks(PER_ROW)
        .fold(Column::new().spacing(2), |column, chunk| {
            column.push(chunk.iter().fold(Row::new().spacing(2), |row, color| {
                row.push(
                    button(
                        container(space::horizontal())
                            .width(typography::scaled(SWATCH))
                            .height(typography::scaled(SWATCH))
                            .style(style::container::swatch(*color)),
                    )
                    .on_press(Pick::Swatch(*color))
                    .padding(2)
                    .style(style::button::swatch(same(*color, current))),
                )
            }))
        })
        .into()
}

/// Düz renkli, kenarlı düğme (karşılaştırmadaki önceki renk).
fn solid(color: Color) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let t = Tokens::of(theme);

        button::Style {
            background: Some(Background::Color(color)),
            border: Border {
                color: if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                    t.accent
                } else {
                    t.border
                },
                width: 1.0,
                radius: 2.0.into(),
            },
            ..button::Style::default()
        }
    }
}

/// Doygunluk (yatay) ve parlaklık (dikey) düzlemi.
struct Plane<'a, Message> {
    hsva: Hsva,
    on_change: Box<dyn Fn(f32, f32) -> Message + 'a>,
    height: f32,
}

impl<'a, Message: 'a> Plane<'a, Message> {
    fn new(hsva: Hsva, on_change: impl Fn(f32, f32) -> Message + 'a) -> Self {
        Self {
            hsva,
            on_change: Box::new(on_change),
            height: PLANE,
        }
    }

    fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
}

#[derive(Debug, Default)]
struct Drag {
    dragging: bool,
}

impl<'a, Message: 'a> Widget<Message, Theme, Renderer> for Plane<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Drag>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Drag::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fixed(self.height))
    }

    fn layout(&mut self, _tree: &mut Tree, _renderer: &Renderer, limits: &layout::Limits) -> Node {
        layout::atomic(limits, Length::Fill, self.height)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<Drag>();
        let bounds = layout.bounds();

        let publish = |shell: &mut Shell<'_, Message>, point: Point| {
            let saturation = ((point.x - bounds.x) / bounds.width).clamp(0.0, 1.0);
            let value = 1.0 - ((point.y - bounds.y) / bounds.height).clamp(0.0, 1.0);

            shell.publish((self.on_change)(saturation, value));
        };

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(point) = cursor.position_over(bounds) {
                    state.dragging = true;
                    publish(shell, point);
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) if state.dragging => {
                publish(shell, *position);
                shell.capture_event();
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.dragging => {
                state.dragging = false;
                shell.capture_event();
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if tree.state.downcast_ref::<Drag>().dragging || cursor.is_over(layout.bounds()) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::None
        }
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let t = Tokens::of(theme);
        let bounds = layout.bounds();
        let radius = border::radius(3.0);

        // Tonun saf rengi; üstüne soldan sağa beyazdan saydama, yukarıdan
        // aşağı saydamdan siyaha iki geçiş.
        let layer = |renderer: &mut Renderer, background: Background| {
            renderer.fill_quad(
                Quad {
                    bounds,
                    border: Border {
                        radius,
                        ..Border::default()
                    },
                    ..Quad::default()
                },
                background,
            );
        };

        layer(renderer, Background::Color(self.hsva.pure()));
        layer(
            renderer,
            Background::Gradient(Gradient::Linear(
                Linear::new(Degrees(90.0))
                    .add_stop(0.0, Color::WHITE)
                    .add_stop(1.0, Color::from_rgba(1.0, 1.0, 1.0, 0.0)),
            )),
        );
        layer(
            renderer,
            Background::Gradient(Gradient::Linear(
                Linear::new(Degrees(180.0))
                    .add_stop(0.0, Color::from_rgba(0.0, 0.0, 0.0, 0.0))
                    .add_stop(1.0, Color::BLACK),
            )),
        );

        renderer.fill_quad(
            Quad {
                bounds,
                border: Border {
                    color: t.border,
                    width: 1.0,
                    radius,
                },
                ..Quad::default()
            },
            Background::Color(Color::TRANSPARENT),
        );

        let center = Point::new(
            bounds.x + self.hsva.saturation * bounds.width,
            bounds.y + (1.0 - self.hsva.value) * bounds.height,
        );

        marker(
            renderer,
            center,
            6.0,
            Hsva {
                alpha: 1.0,
                ..self.hsva
            }
            .to_color(),
        );
    }
}

/// Seçim işareti: rengi içinde, beyaz halkalı, koyu dış çizgili daire.
fn marker(renderer: &mut Renderer, center: Point, radius: f32, fill: Color) {
    renderer.fill_quad(
        Quad {
            bounds: Rectangle::new(
                Point::new(center.x - radius, center.y - radius),
                Size::new(2.0 * radius, 2.0 * radius),
            ),
            border: Border {
                color: Color::WHITE,
                width: 2.0,
                radius: radius.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.6),
                offset: Vector::ZERO,
                blur_radius: 2.0,
            },
            ..Quad::default()
        },
        Background::Color(fill),
    );
}

/// Şeridin gösterdiği kanal.
#[derive(Debug, Clone, Copy)]
enum Channel {
    Hue(Hsva),
    Alpha(Hsva),
}

/// Ton ya da saydamlık şeridi.
struct Strip<'a, Message> {
    channel: Channel,
    value: f32,
    on_change: Box<dyn Fn(f32) -> Message + 'a>,
}

impl<'a, Message: 'a> Strip<'a, Message> {
    fn new(channel: Channel, value: f32, on_change: impl Fn(f32) -> Message + 'a) -> Self {
        Self {
            channel,
            value,
            on_change: Box::new(on_change),
        }
    }
}

impl<'a, Message: 'a> Widget<Message, Theme, Renderer> for Strip<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Drag>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Drag::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fixed(STRIP + 4.0))
    }

    fn layout(&mut self, _tree: &mut Tree, _renderer: &Renderer, limits: &layout::Limits) -> Node {
        layout::atomic(limits, Length::Fill, STRIP + 4.0)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<Drag>();
        let bounds = layout.bounds();

        let publish = |shell: &mut Shell<'_, Message>, point: Point| {
            shell.publish((self.on_change)(
                ((point.x - bounds.x) / bounds.width).clamp(0.0, 1.0),
            ));
        };

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(point) = cursor.position_over(bounds) {
                    state.dragging = true;
                    publish(shell, point);
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) if state.dragging => {
                publish(shell, *position);
                shell.capture_event();
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.dragging => {
                state.dragging = false;
                shell.capture_event();
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if tree.state.downcast_ref::<Drag>().dragging {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let t = Tokens::of(theme);
        let outer = layout.bounds();
        let bounds = Rectangle {
            y: outer.y + 2.0,
            height: STRIP,
            ..outer
        };
        let radius = border::radius(STRIP / 2.0);

        let gradient = match self.channel {
            Channel::Hue(_) => (0..=6).fold(Linear::new(Degrees(90.0)), |gradient, step| {
                gradient.add_stop(
                    step as f32 / 6.0,
                    Hsva {
                        hue: step as f32 * 60.0,
                        saturation: 1.0,
                        value: 1.0,
                        alpha: 1.0,
                    }
                    .to_color(),
                )
            }),
            Channel::Alpha(hsva) => {
                checkerboard(renderer, bounds);

                let color = Hsva { alpha: 1.0, ..hsva }.to_color();

                Linear::new(Degrees(90.0))
                    .add_stop(0.0, Color { a: 0.0, ..color })
                    .add_stop(1.0, color)
            }
        };

        renderer.fill_quad(
            Quad {
                bounds,
                border: Border {
                    color: t.border,
                    width: 1.0,
                    radius,
                },
                ..Quad::default()
            },
            Background::Gradient(Gradient::Linear(gradient)),
        );

        let fill = match self.channel {
            Channel::Hue(hsva) => hsva.pure(),
            Channel::Alpha(hsva) => hsva.to_color(),
        };
        let x = bounds.x + self.value.clamp(0.0, 1.0) * bounds.width;

        marker(
            renderer,
            Point::new(x, bounds.center_y()),
            STRIP / 2.0 + 1.0,
            fill,
        );
    }
}

/// Saydamlığın altındaki satranç tahtası.
fn checkerboard(renderer: &mut Renderer, bounds: Rectangle) {
    renderer.fill_quad(
        Quad {
            bounds,
            border: Border {
                radius: (bounds.height / 2.0).into(),
                ..Border::default()
            },
            ..Quad::default()
        },
        Background::Color(Color::from_rgb8(0xf2, 0xf2, 0xf2)),
    );

    let columns = (bounds.width / CHECKER).ceil() as usize;
    let rows = (bounds.height / CHECKER).ceil() as usize;

    for row in 0..rows {
        for column in (row % 2..columns).step_by(2) {
            let x = bounds.x + column as f32 * CHECKER;
            let y = bounds.y + row as f32 * CHECKER;

            // Uçlardaki yuvarlaklığa taşmasın diye kenarlar içeride kalır.
            if x < bounds.x + bounds.height / 2.0 - 1.0
                || x + CHECKER > bounds.x + bounds.width - bounds.height / 2.0 + 1.0
            {
                continue;
            }

            renderer.fill_quad(
                Quad {
                    bounds: Rectangle::new(
                        Point::new(x, y),
                        Size::new(CHECKER, CHECKER.min(bounds.y + bounds.height - y)),
                    ),
                    ..Quad::default()
                },
                Background::Color(Color::from_rgb8(0xc8, 0xc8, 0xc8)),
            );
        }
    }
}

impl<'a, Message: 'a> From<Plane<'a, Message>> for Element<'a, Message> {
    fn from(plane: Plane<'a, Message>) -> Self {
        Element::new(plane)
    }
}

impl<'a, Message: 'a> From<Strip<'a, Message>> for Element<'a, Message> {
    fn from(strip: Strip<'a, Message>) -> Self {
        Element::new(strip)
    }
}

/// Rampanın durağı: 0–1 arasındaki yeri ve rengi.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stop {
    pub position: f32,
    pub color: Color,
}

/// Renk rampası: sıralı duraklar; aralar doğrusal karışır.
#[derive(Debug, Clone, PartialEq)]
pub struct Ramp {
    stops: Vec<Stop>,
}

impl Ramp {
    /// Duraklar yerlerine göre sıralanır; en az iki durak olmalıdır, yoksa
    /// siyahtan beyaza rampa kurulur.
    pub fn new(stops: impl IntoIterator<Item = Stop>) -> Self {
        let mut stops: Vec<Stop> = stops
            .into_iter()
            .map(|stop| Stop {
                position: stop.position.clamp(0.0, 1.0),
                ..stop
            })
            .collect();
        stops.sort_by(|a, b| a.position.total_cmp(&b.position));

        if stops.len() < 2 {
            stops = vec![
                Stop {
                    position: 0.0,
                    color: Color::BLACK,
                },
                Stop {
                    position: 1.0,
                    color: Color::WHITE,
                },
            ];
        }

        Self { stops }
    }

    /// Renkleri eşit aralıklı duraklar olarak dizer.
    pub fn even(colors: impl IntoIterator<Item = Color>) -> Self {
        let colors: Vec<Color> = colors.into_iter().collect();
        let last = colors.len().saturating_sub(1).max(1) as f32;

        Self::new(colors.into_iter().enumerate().map(|(index, color)| Stop {
            position: index as f32 / last,
            color,
        }))
    }

    pub fn stops(&self) -> &[Stop] {
        &self.stops
    }

    /// Rampanın `t` (0–1) noktasındaki rengi.
    pub fn color_at(&self, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        let first = self.stops[0];

        if t <= first.position {
            return first.color;
        }

        for pair in self.stops.windows(2) {
            let (a, b) = (pair[0], pair[1]);

            if t <= b.position {
                let span = (b.position - a.position).max(f32::EPSILON);
                let amount = (t - a.position) / span;

                return Color::from_rgba(
                    a.color.r + (b.color.r - a.color.r) * amount,
                    a.color.g + (b.color.g - a.color.g) * amount,
                    a.color.b + (b.color.b - a.color.b) * amount,
                    a.color.a + (b.color.a - a.color.a) * amount,
                );
            }
        }

        self.stops[self.stops.len() - 1].color
    }

    /// Rampanın o noktadaki rengiyle durak ekler; durağın sırasını döndürür.
    pub fn insert(&mut self, position: f32) -> usize {
        let position = position.clamp(0.0, 1.0);
        let stop = Stop {
            position,
            color: self.color_at(position),
        };
        let index = self
            .stops
            .iter()
            .position(|other| other.position > position)
            .unwrap_or(self.stops.len());

        self.stops.insert(index, stop);
        index
    }

    /// Durağı siler; rampada en az iki durak kalır.
    pub fn remove(&mut self, index: usize) -> bool {
        if self.stops.len() <= 2 || index >= self.stops.len() {
            return false;
        }

        self.stops.remove(index);
        true
    }

    /// Durağı taşır; durakların sırası değişebilir. Durağın yeni sırasını
    /// döndürür.
    pub fn move_stop(&mut self, index: usize, position: f32) -> usize {
        let Some(stop) = self.stops.get(index).copied() else {
            return index;
        };

        self.stops.remove(index);

        let position = position.clamp(0.0, 1.0);
        let at = self
            .stops
            .iter()
            .position(|other| other.position > position)
            .unwrap_or(self.stops.len());

        self.stops.insert(at, Stop { position, ..stop });
        at
    }

    pub fn set_color(&mut self, index: usize, color: Color) {
        if let Some(stop) = self.stops.get_mut(index) {
            stop.color = color;
        }
    }

    /// Rampayı ters çevirir.
    pub fn reversed(&self) -> Self {
        Self::new(self.stops.iter().map(|stop| Stop {
            position: 1.0 - stop.position,
            color: stop.color,
        }))
    }

    /// Hazır rampalar: sürekli veriler, sıcaklık, arazi ve gri tonlar.
    pub fn presets() -> Vec<(&'static str, Ramp)> {
        let ramp = |colors: &[u32]| Ramp::even(colors.iter().map(|value| rgb(*value)));

        vec![
            (
                "Viridis",
                ramp(&[0x440154, 0x3b528b, 0x21918c, 0x5ec962, 0xfde725]),
            ),
            (
                "Magma",
                ramp(&[0x000004, 0x51127c, 0xb73779, 0xfc8961, 0xfcfdbf]),
            ),
            (
                "Spektral",
                ramp(&[0x9e0142, 0xf46d43, 0xfee08b, 0xe6f598, 0x66c2a5, 0x5e4fa2]),
            ),
            ("Soğuk sıcak", ramp(&[0x3b4cc0, 0xdddddd, 0xb40426])),
            (
                "Arazi",
                ramp(&[0x2e7d32, 0x9ccc65, 0xfff59d, 0xbcaaa4, 0x795548, 0xffffff]),
            ),
            ("Mavi", ramp(&[0xf7fbff, 0x6baed6, 0x08306b])),
            ("Gri", ramp(&[0x000000, 0xffffff])),
        ]
    }
}

/// Rampanın geçişi (ör. lejantta ya da listede önizleme).
pub fn gradient(ramp: &Ramp) -> Gradient {
    // Geçişte en fazla sekiz durak olur; fazlası eşit aralıklarla örneklenir.
    let stops: Vec<Stop> = if ramp.stops.len() <= 8 {
        ramp.stops.clone()
    } else {
        (0..8)
            .map(|index| {
                let position = index as f32 / 7.0;

                Stop {
                    position,
                    color: ramp.color_at(position),
                }
            })
            .collect()
    };

    Gradient::Linear(
        stops
            .iter()
            .fold(Linear::new(Degrees(90.0)), |gradient, stop| {
                gradient.add_stop(stop.position, stop.color)
            }),
    )
}

/// Rampa önizlemesi: kenarlı geçiş şeridi.
pub fn preview<'a, Message: 'a>(ramp: &Ramp, width: f32, height: f32) -> Element<'a, Message> {
    let gradient = gradient(ramp);

    container(space::horizontal())
        .width(width)
        .height(height)
        .style(move |theme: &Theme| container::Style {
            background: Some(Background::Gradient(gradient)),
            border: Border {
                color: Tokens::of(theme).border,
                width: 1.0,
                radius: 2.0.into(),
            },
            ..container::Style::default()
        })
        .into()
}

/// Renk rampası düzenleyicisi: duraklı çubuk ve seçili durağın satırı.
/// `on_change` yeni rampayı ve seçili durağın sırasını alır.
pub fn ramp<'a, Message: Clone + 'a>(
    ramp: &Ramp,
    selected: usize,
    on_change: impl Fn(Ramp, usize) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let selected = selected.min(ramp.stops.len().saturating_sub(1));
    let stop = ramp.stops[selected];

    let recolor = {
        let ramp = ramp.clone();
        let on_change = on_change.clone();

        move |color| {
            let mut ramp = ramp.clone();
            ramp.set_color(selected, color);
            on_change(ramp, selected)
        }
    };
    let reposition = {
        let ramp = ramp.clone();
        let on_change = on_change.clone();

        move |percent: f64| {
            let mut ramp = ramp.clone();
            let index = ramp.move_stop(selected, (percent / 100.0) as f32);
            on_change(ramp, index)
        }
    };

    let removed = {
        let mut ramp = ramp.clone();
        ramp.remove(selected)
            .then(|| on_change(ramp, selected.saturating_sub(1)))
    };
    let reversed = on_change(ramp.reversed(), ramp.stops.len() - 1 - selected);

    let presets: Vec<(&'static str, Message)> = Ramp::presets()
        .into_iter()
        .map(|(name, preset)| (name, on_change(preset, 0)))
        .collect();

    let action = |glyph: Icon, name: &'a str, message: Option<Message>| {
        button(
            row![icon(glyph).size(12.0), label::body(name)]
                .spacing(6)
                .align_y(Center),
        )
        .on_press_maybe(message)
        .padding([3, 8])
        .style(style::button::secondary)
    };

    column![
        Bar::new(ramp.clone(), selected, on_change),
        row![
            ColorPicker::new(stop.color, recolor).alpha(),
            NumberInput::new(f64::from(stop.position) * 100.0, reposition)
                .label("K")
                .units(units::PERCENT)
                .range(0.0..=100.0)
                .step(1.0)
                .width(typography::scaled(96.0)),
            action(Icon::Close, "Sil", removed),
            action(Icon::Retry, "Ters çevir", Some(reversed)),
            MenuButton::new(
                container(
                    row![
                        icon(Icon::Drop).size(12.0),
                        label::body("Hazır rampalar"),
                        icon(Icon::ChevronDown).size(10.0).tone(Tone::Muted),
                    ]
                    .spacing(6)
                    .align_y(Center),
                )
                .padding([3, 8]),
                move || {
                    presets.iter().fold(
                        Menu::new().header("Hazır rampalar"),
                        |menu, (name, message)| menu.item(*name, message.clone()),
                    )
                },
            ),
        ]
        .spacing(8)
        .align_y(Center),
    ]
    .spacing(10)
    .into()
}

/// Rampa çubuğu: geçiş ve altında duraklar.
struct Bar<'a, Message> {
    ramp: Ramp,
    selected: usize,
    on_change: Box<dyn Fn(Ramp, usize) -> Message + 'a>,
}

impl<'a, Message: 'a> Bar<'a, Message> {
    fn new(ramp: Ramp, selected: usize, on_change: impl Fn(Ramp, usize) -> Message + 'a) -> Self {
        Self {
            ramp,
            selected,
            on_change: Box::new(on_change),
        }
    }
}

/// Çubuğun ve durak işaretlerinin ölçüleri.
const BAR: f32 = 22.0;
const HANDLE: f32 = 12.0;
/// Durağın silinmek üzere bırakılabileceği uzaklık (çubuğun altına doğru).
const REMOVE: f32 = 28.0;

#[derive(Debug, Default)]
struct BarState {
    /// Sürüklenen durak ve sürüklemenin başladığı yer.
    dragging: Option<(usize, Point, f32)>,
    /// Sürüklenen durak silinecek kadar uzakta.
    removing: bool,
    hovered: Option<usize>,
}

impl<Message> Bar<'_, Message> {
    fn track(bounds: Rectangle) -> Rectangle {
        Rectangle {
            x: bounds.x + HANDLE / 2.0,
            width: bounds.width - HANDLE,
            height: BAR,
            ..bounds
        }
    }

    fn handle(&self, bounds: Rectangle, index: usize) -> Rectangle {
        let track = Self::track(bounds);
        let x = track.x + self.ramp.stops[index].position * track.width;

        Rectangle::new(
            Point::new(x - HANDLE / 2.0, track.y + BAR + 2.0),
            Size::new(HANDLE, HANDLE + 2.0),
        )
    }

    fn handle_at(&self, bounds: Rectangle, point: Point) -> Option<usize> {
        // Öndeki (seçili) durak önce yakalanır.
        std::iter::once(self.selected)
            .chain(0..self.ramp.stops.len())
            .find(|index| {
                *index < self.ramp.stops.len()
                    && self.handle(bounds, *index).expand(2.0).contains(point)
            })
    }
}

impl<'a, Message: 'a> Widget<Message, Theme, Renderer> for Bar<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<BarState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(BarState::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fixed(BAR + HANDLE + 4.0))
    }

    fn layout(&mut self, _tree: &mut Tree, _renderer: &Renderer, limits: &layout::Limits) -> Node {
        layout::atomic(limits, Length::Fill, BAR + HANDLE + 4.0)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<BarState>();
        let bounds = layout.bounds();
        let track = Self::track(bounds);
        let at = |x: f32| (x - track.x) / track.width;

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(point) = cursor.position_over(bounds) else {
                    return;
                };

                if let Some(index) = self.handle_at(bounds, point) {
                    if index != self.selected {
                        shell.publish((self.on_change)(self.ramp.clone(), index));
                    }

                    state.dragging = Some((index, point, self.ramp.stops[index].position));
                } else if track.contains(point) {
                    // Çubuğa tıklamak o noktanın rengiyle durak ekler.
                    let mut ramp = self.ramp.clone();
                    let index = ramp.insert(at(point.x));
                    let position = ramp.stops[index].position;

                    shell.publish((self.on_change)(ramp, index));
                    state.dragging = Some((index, point, position));
                } else {
                    return;
                }

                state.removing = false;
                shell.capture_event();
                shell.request_redraw();
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if let Some((index, origin, start)) = state.dragging {
                    let moved = start + (position.x - origin.x) / track.width;
                    let mut ramp = self.ramp.clone();
                    let index = ramp.move_stop(index, moved);

                    state.dragging = Some((index, origin, start));
                    state.removing =
                        self.ramp.stops.len() > 2 && position.y > track.y + BAR + REMOVE + HANDLE;

                    if ramp != self.ramp || index != self.selected {
                        shell.publish((self.on_change)(ramp, index));
                    }

                    shell.capture_event();
                    shell.request_redraw();
                    return;
                }

                let hovered = cursor
                    .position_over(bounds)
                    .and_then(|point| self.handle_at(bounds, point));

                if hovered != state.hovered {
                    state.hovered = hovered;
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if let Some((index, _, _)) = state.dragging.take() {
                    // Çubuğun altına uzağa bırakılan durak silinir.
                    if state.removing {
                        let mut ramp = self.ramp.clone();

                        if ramp.remove(index) {
                            shell.publish((self.on_change)(ramp, index.saturating_sub(1)));
                        }
                    }

                    state.removing = false;
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<BarState>();
        let bounds = layout.bounds();

        if state.dragging.is_some() {
            return mouse::Interaction::Grabbing;
        }

        match cursor.position_over(bounds) {
            Some(point) if self.handle_at(bounds, point).is_some() => mouse::Interaction::Grab,
            Some(point) if Self::track(bounds).contains(point) => mouse::Interaction::Crosshair,
            _ => mouse::Interaction::None,
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<BarState>();
        let t = Tokens::of(theme);
        let bounds = layout.bounds();
        let track = Self::track(bounds);

        checkerboard(
            renderer,
            Rectangle {
                height: BAR,
                ..track
            },
        );
        renderer.fill_quad(
            Quad {
                bounds: track,
                border: Border {
                    color: t.border,
                    width: 1.0,
                    radius: 3.0.into(),
                },
                ..Quad::default()
            },
            Background::Gradient(gradient(&self.ramp)),
        );

        // Seçili durak en üstte çizilir.
        let order = (0..self.ramp.stops.len())
            .filter(|index| *index != self.selected)
            .chain(std::iter::once(self.selected));

        for index in order {
            let handle = self.handle(bounds, index);
            let selected = index == self.selected;
            let fading = selected && state.removing;
            let hot = selected || state.hovered == Some(index);
            let alpha = if fading { 0.35 } else { 1.0 };

            // Durağın çubuğa değen ucu.
            renderer.fill_quad(
                Quad {
                    bounds: Rectangle::new(
                        Point::new(handle.center_x() - 0.5, track.y + BAR - 3.0),
                        Size::new(1.0, 5.0),
                    ),
                    ..Quad::default()
                },
                Background::Color(if hot { t.text } else { t.muted }.scale_alpha(alpha)),
            );

            renderer.fill_quad(
                Quad {
                    bounds: handle,
                    border: Border {
                        color: if selected { t.accent } else { t.muted }.scale_alpha(alpha),
                        width: if selected { 2.0 } else { 1.0 },
                        radius: 2.0.into(),
                    },
                    ..Quad::default()
                },
                Background::Color(Color {
                    a: self.ramp.stops[index].color.a * alpha,
                    ..self.ramp.stops[index].color
                }),
            );
        }
    }
}

impl<'a, Message: 'a> From<Bar<'a, Message>> for Element<'a, Message> {
    fn from(bar: Bar<'a, Message>) -> Self {
        Element::new(bar)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn same(a: Color, b: Color) -> bool {
        a.into_rgba8() == b.into_rgba8()
    }

    #[test]
    fn colors_round_trip_through_hsv_and_hex() {
        for value in PALETTE {
            let color = rgb(value);

            assert!(
                same(Hsva::from_color(color).to_color(), color),
                "{value:06x}"
            );
            assert_eq!(parse_hex(&to_hex(color)), Some(color));
        }

        let orange = Hsva::from_color(rgb(0xff8000));
        assert!((orange.hue - 30.0).abs() < 0.5);
        assert!((orange.saturation - 1.0).abs() < 1e-6);

        assert_eq!(to_hex(Color::from_rgba8(255, 0, 0, 0.5)), "#FF000080");
        assert!(same(parse_hex("#f80").expect("renk"), rgb(0xff8800)));
        assert!(same(parse_hex("e2a93b").expect("renk"), rgb(0xe2a93b)));
        assert_eq!(parse_hex("#12345"), None);
        assert_eq!(parse_hex("#GG0000"), None);
    }

    #[test]
    fn grays_keep_the_hue_the_user_chose() {
        let mut picker = Picker::new(rgb(0xff0000));
        picker.hsva.hue = 200.0;
        picker.set(rgb(0x808080));

        assert_eq!(picker.hsva.hue, 200.0);
        assert_eq!(picker.hsva.saturation, 0.0);
    }

    #[test]
    fn ramps_insert_move_and_remove_stops() {
        let mut ramp = Ramp::even([Color::BLACK, Color::WHITE]);

        assert!(same(ramp.color_at(0.5), Color::from_rgb(0.5, 0.5, 0.5)));

        let index = ramp.insert(0.25);
        assert_eq!(index, 1);
        assert!(same(
            ramp.stops()[1].color,
            Color::from_rgb(0.25, 0.25, 0.25)
        ));

        // Taşınan durak sırasını değiştirebilir.
        let moved = ramp.move_stop(1, 0.9);
        assert_eq!(moved, 1);
        let moved = ramp.move_stop(0, 0.95);
        assert_eq!(moved, 1);
        assert_eq!(
            ramp.stops()
                .iter()
                .map(|stop| stop.position)
                .collect::<Vec<_>>(),
            [0.9, 0.95, 1.0]
        );

        assert!(ramp.remove(0));
        assert!(!ramp.remove(0));
        assert_eq!(ramp.stops().len(), 2);

        let reversed = Ramp::even([Color::BLACK, rgb(0xff0000), Color::WHITE]).reversed();
        assert!(same(reversed.stops()[0].color, Color::WHITE));
        assert_eq!(reversed.stops()[1].position, 0.5);

        // Tek duraklı rampa kurulamaz.
        assert_eq!(Ramp::new([]).stops().len(), 2);
    }
}
