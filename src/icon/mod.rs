//! Vektör ikon seti.
//!
//! Unicode simgeleri farklı yazı tiplerinden geldiği için boyutları ve
//! taban çizgileri tutarsızdır. kentos-rc ikonları 16×16'lık bir ızgarada,
//! tek çizgi kalınlığıyla çizilir ve her boyutta aynı optik ağırlığı korur.
//!
//! ```ignore
//! use kentos_rc::icon::{Icon, icon};
//!
//! button(icon(Icon::ZoomIn)).on_press(Message::ZoomIn)
//! ```
//!
//! İkon varsayılan olarak içinde bulunduğu öğenin metin rengini alır
//! ([`Tone::Inherit`]); düğmenin etkin, üzerine gelinmiş ya da devre dışı
//! renkleri ikona da kendiliğinden yansır.

mod draw;

use std::cell::Cell;

use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Renderer as _};
use iced::advanced::widget::{Tree, Widget, tree};
use iced::widget::canvas;
use iced::{Color, Element, Length, Rectangle, Renderer, Size, Theme, Vector, mouse};

use crate::theme::Tokens;

/// İkon setindeki bütün ikonlar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    // Görünüm
    ZoomIn,
    ZoomOut,
    ZoomExtents,
    Home,
    Target,
    // Katman ve seçim
    Eye,
    EyeOff,
    ClearSelection,
    Eraser,
    // Arayüz
    Contrast,
    Help,
    Terminal,
    ChevronDown,
    ChevronRight,
    // Dosya
    Document,
    DocumentNew,
    Folder,
    Save,
    SaveAs,
    Export,
    Print,
    Power,
    // Gezinme ve ölçüm araçları
    Pan,
    Select,
    Measure,
    // Çizim araçları
    Line,
    Polyline,
    Polygon,
    Rectangle,
    Circle,
    Point,
}

impl Icon {
    /// Setteki bütün ikonlar, yukarıdaki gruplama sırasıyla.
    pub const ALL: [Icon; 31] = [
        Icon::ZoomIn,
        Icon::ZoomOut,
        Icon::ZoomExtents,
        Icon::Home,
        Icon::Target,
        Icon::Eye,
        Icon::EyeOff,
        Icon::ClearSelection,
        Icon::Eraser,
        Icon::Contrast,
        Icon::Help,
        Icon::Terminal,
        Icon::ChevronDown,
        Icon::ChevronRight,
        Icon::Document,
        Icon::DocumentNew,
        Icon::Folder,
        Icon::Save,
        Icon::SaveAs,
        Icon::Export,
        Icon::Print,
        Icon::Power,
        Icon::Pan,
        Icon::Select,
        Icon::Measure,
        Icon::Line,
        Icon::Polyline,
        Icon::Polygon,
        Icon::Rectangle,
        Icon::Circle,
        Icon::Point,
    ];
}

/// İkonun rengi.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Tone {
    /// İçinde bulunduğu öğenin metin rengi (düğme, kap).
    #[default]
    Inherit,
    Text,
    Muted,
    Accent,
    /// Vurgunun güçlü hâli: etkin aracın ikonu.
    Highlight,
    OnAccent,
    Disabled,
    Custom(Color),
}

impl Tone {
    fn resolve(self, theme: &Theme, inherited: Color) -> Color {
        let tokens = Tokens::of(theme);

        match self {
            Tone::Inherit => inherited,
            Tone::Text => tokens.text,
            Tone::Muted => tokens.muted,
            Tone::Accent => tokens.accent,
            Tone::Highlight => tokens.accent_hover,
            Tone::OnAccent => tokens.on_accent,
            Tone::Disabled => tokens.disabled(),
            Tone::Custom(color) => color,
        }
    }
}

/// Varsayılan (16 piksel, miras renkli) bir ikon oluşturur.
pub fn icon(icon: Icon) -> Glyph {
    Glyph {
        icon,
        size: 16.0,
        tone: Tone::Inherit,
    }
}

/// Ekrana çizilen ikon.
///
/// Çizim, ikon, boyut ya da renk değişene kadar önbellekte tutulur.
#[derive(Debug, Clone, Copy)]
pub struct Glyph {
    icon: Icon,
    size: f32,
    tone: Tone,
}

impl Glyph {
    /// Kenar uzunluğu (piksel).
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    pub fn color(self, color: Color) -> Self {
        self.tone(Tone::Custom(color))
    }
}

#[derive(Default)]
struct State {
    cache: canvas::Cache,
    drawn: Cell<Option<(Icon, Color)>>,
}

impl<Message> Widget<Message, Theme, Renderer> for Glyph {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(self.size), Length::Fixed(self.size))
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, self.size, self.size)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let color = self.tone.resolve(theme, style.text_color);
        let state = tree.state.downcast_ref::<State>();

        if state.drawn.get() != Some((self.icon, color)) {
            state.cache.clear();
            state.drawn.set(Some((self.icon, color)));
        }

        let geometry = state.cache.draw(renderer, bounds.size(), |frame| {
            draw::icon(frame, self.icon, color);
        });

        renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
            renderer.draw_geometry(geometry);
        });
    }
}

impl<'a, Message> From<Glyph> for Element<'a, Message> {
    fn from(glyph: Glyph) -> Self {
        Element::new(glyph)
    }
}
