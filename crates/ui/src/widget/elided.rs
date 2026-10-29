//! Tek satırlık metin: yerine sığmayınca sonu “…” ile kısalır (web'in
//! `text-overflow: ellipsis`'i). Genişlik tahminle değil, metin gerçek yazı
//! tipiyle dizilerek ölçülür; kesilen yer karakter sınırıdır.
//!
//! ```text
//! ┌────────────────────────────┐
//! │ ⚠ “yollar.shp” içinde alın…│
//! └────────────────────────────┘
//! ```
//!
//! Ölçüm yalnız metin, genişlik ya da yazı değişince yapılır: durum
//! çubuğu gibi her karede yeniden kurulan yerlerde de ucuzdur.
//!
//! ```ignore
//! Elided::new(&message).size(typography::body()).style(style::text::muted)
//! ```

use std::borrow::Cow;

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::text::{self as text_core, Paragraph as _, Renderer as _};
use iced::advanced::widget::{self, Tree, Widget, tree};
use iced::alignment::Vertical;
use iced::widget::text::{self, LineHeight, Wrapping};
use iced::{Element, Font, Length, Pixels, Point, Rectangle, Renderer, Size, Theme, mouse};

type Paragraph = <Renderer as text_core::Renderer>::Paragraph;

/// Sığmayan sonu “…” ile kısalan tek satırlık metin.
pub struct Elided<'a> {
    content: Cow<'a, str>,
    size: Option<f32>,
    font: Option<Font>,
    line_height: LineHeight,
    width: Length,
    reserve: f32,
    style: Box<dyn Fn(&Theme) -> text::Style + 'a>,
}

impl<'a> Elided<'a> {
    pub fn new(content: impl Into<Cow<'a, str>>) -> Self {
        Self {
            content: content.into(),
            size: None,
            font: None,
            line_height: LineHeight::default(),
            width: Length::Shrink,
            reserve: 0.0,
            style: Box::new(|_| text::Style::default()),
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    pub fn font(mut self, font: Font) -> Self {
        self.font = Some(font);
        self
    }

    pub fn line_height(mut self, line_height: impl Into<LineHeight>) -> Self {
        self.line_height = line_height.into();
        self
    }

    /// Kapladığı genişlik; varsayılan metin kadar (en çok verilen yer).
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Satırda arkasından gelenlere bırakılan genişlik (piksel): metin verilen
    /// yerin bu kadar eksiğine sığar. Web'de `flex: none` komşularının yanında
    /// kısalan metin (komut satırının istemi ve çipleri).
    pub fn reserve(mut self, px: f32) -> Self {
        self.reserve = px.max(0.0);
        self
    }

    pub fn style(mut self, style: impl Fn(&Theme) -> text::Style + 'a) -> Self {
        self.style = Box::new(style);
        self
    }
}

/// What an [`Elided`] tells an operation besides its text (`Operation::custom`): whether the text was cut to fit. A
/// tip that shows the whole text only then asks for it ([`Tip::when_cut`](crate::widget::Tip::when_cut), docs/adr/0748).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cut(pub bool);

/// Son ölçüm: neyin, hangi genişlikte ve boyda ölçüldüğü, dizilen metin ve kesilip kesilmediği.
#[derive(Default)]
struct State {
    key: Option<(String, u32, u32, Font)>,
    paragraph: Paragraph,
    cut: bool,
}

impl Elided<'_> {
    #[cfg(test)]
    fn shaped(&self, content: &str, size: f32, font: Font) -> Paragraph {
        shaped(content, size, font, self.line_height)
    }

    /// Metin `room` genişliğine sığacak biçimde: olduğu gibi ya da kesilip “…” ile; kesildi mi.
    fn fit(&self, room: f32, size: f32, font: Font) -> (Paragraph, bool) {
        let (paragraph, cut) = fit(&self.content, room, size, font, self.line_height);
        (paragraph, cut.is_some())
    }
}

fn shaped(content: &str, size: f32, font: Font, line_height: LineHeight) -> Paragraph {
    Paragraph::with_text(text_core::Text {
        content,
        bounds: Size::INFINITE,
        size: Pixels(size),
        line_height,
        font,
        align_x: text_core::Alignment::Default,
        align_y: Vertical::Top,
        shaping: text_core::Shaping::Advanced,
        wrapping: Wrapping::None,
    })
}

/// `content` as it fits in `room`: shaped as it is, or cut at a character and ended in “…”, with the shortened text
/// (an [`EditCell`](crate::widget::EditCell) at rest shows it, docs/adr/0748).
pub(crate) fn fit(
    content: &str,
    room: f32,
    size: f32,
    font: Font,
    line_height: LineHeight,
) -> (Paragraph, Option<String>) {
    let full = shaped(content, size, font, line_height);
    if full.min_width() <= room {
        return (full, None);
    }
    let ellipsis = shaped("…", size, font, line_height).min_width();
    let target = (room - ellipsis).max(0.0);
    let middle = full.min_height() / 2.0;
    // The character at the cut, then back a character at a time while
    // the cut text and its ellipsis do not fit.
    let mut cut = full
        .hit_test(Point::new(target, middle))
        .map_or(0, text_core::Hit::cursor)
        .min(content.len());
    loop {
        while cut > 0 && !content.is_char_boundary(cut) {
            cut -= 1;
        }
        let kept = format!("{}…", content[..cut].trim_end());
        let shaped = shaped(&kept, size, font, line_height);
        if shaped.min_width() <= room || cut == 0 {
            return (shaped, Some(kept));
        }
        cut -= 1;
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Elided<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<State>();
        let size = self.size.unwrap_or_else(|| renderer.default_size().0);
        let font = self.font.unwrap_or_else(|| renderer.default_font());
        let limits = limits.width(self.width);
        let room = (limits.max().width - self.reserve).max(0.0);
        let key = (
            self.content.to_string(),
            room.to_bits(),
            size.to_bits(),
            font,
        );
        if state.key.as_ref() != Some(&key) {
            (state.paragraph, state.cut) = self.fit(room, size, font);
            state.key = Some(key);
        }
        let bounds = state.paragraph.min_bounds();
        let height = self.line_height.to_absolute(Pixels(size)).0;
        layout::Node::new(limits.resolve(
            self.width,
            Length::Shrink,
            Size::new(bounds.width, height),
        ))
    }

    /// The whole text, not the shortened one: what it says, for finding it
    /// (tests, a screen reader); and whether it was cut ([`Cut`]).
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        operation.text(None, layout.bounds(), &self.content);
        let mut cut = Cut(tree.state.downcast_ref::<State>().cut);
        operation.custom(None, layout.bounds(), &mut cut);
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let color = (self.style)(theme).color.unwrap_or(style.text_color);
        renderer.fill_paragraph(
            &state.paragraph,
            bounds.position(),
            color,
            bounds.intersection(viewport).unwrap_or(bounds),
        );
    }
}

impl<'a, Message: 'a> From<Elided<'a>> for Element<'a, Message> {
    fn from(elided: Elided<'a>) -> Self {
        Element::new(elided)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_text_is_cut_with_an_ellipsis_to_fit_and_a_short_one_is_kept() {
        crate::theme::typography::load();
        let font = crate::theme::typography::ui();
        let long =
            Elided::new("“yollar.shp” içinde alınmayanlar: Kısa halka: 1, üçten az köşesi var");
        let whole = long.shaped(&long.content, 13.0, font).min_width();
        let room = (whole / 2.0).round();
        let (cut, was_cut) = long.fit(room, 13.0, font);
        assert!(was_cut);
        assert!(cut.min_width() <= room, "{} > {room}", cut.min_width());
        // Not cut much shorter than the room allows: within two letters of it.
        assert!(cut.min_width() > room - 2.0 * 13.0, "{}", cut.min_width());

        let short = Elided::new("Kaydedildi.");
        let (kept, was_cut) = short.fit(400.0, 13.0, font);
        assert!(!was_cut);
        assert_eq!(
            kept.min_width(),
            short.shaped("Kaydedildi.", 13.0, font).min_width()
        );
        // No room at all: the ellipsis alone.
        let (none, _) = long.fit(0.0, 13.0, font);
        assert_eq!(none.min_width(), long.shaped("…", 13.0, font).min_width());
    }
}
