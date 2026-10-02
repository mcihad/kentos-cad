//! Parçalı seçim: birbirini dışlayan birkaç seçenekten birini seçtiren
//! bitişik düğmeler (ör. seçim yöntemi: Yeni seçim / Ekle / Çıkar).
//!
//! ```text
//!  ╭──────────────────────────────────────╮
//!  │ ┌──────────┐                         │  ← gömük yuva (alan zemini)
//!  │ │ ◇ Yeni   │  ⊕ Ekle │ ⊖ Çıkar      │  ← seçili parça kalkık; seçili
//!  │ └──────────┘                         │    olmayan komşular ince çizgiyle
//!  ╰──────────────────────────────────────╯    ayrılır
//! ```
//!
//! Seçili parçanın yazısı yarı kalındır; parçanın genişliği kalın yazıya
//! göre ayrılır, seçim değişince parçalar yerinden oynamaz. Parçalar ikon
//! taşıyabilir ([`Segmented::icons`]), yalnız ikonla dizilebilir
//! ([`Segmented::icon_only`]; ad ipucuna geçer), satır içinde sıkışık
//! durabilir ([`Segmented::compact`]) ya da kip değiştiren büyük seçimlerde
//! seçili parça yumuşak vurgu alabilir ([`Segmented::accent`]).
//!
//! ```ignore
//! Segmented::new(View::ALL, view, Message::ViewChosen)
//!     .icons([Icon::Layout, Icon::Cube])
//!     .width(Fill)
//! ```

use std::fmt::Display;

use iced::widget::tooltip::Position;
use iced::widget::{Row, button, container, row, space, stack};
use iced::{Center, Element, Length, Theme};

use crate::icon::{Icon, icon};
use crate::label;
use crate::style;
use crate::theme::{Tokens, metrics, typography};
use crate::widget::tip::{Tip, tip};

/// Parçaların yuvanın kenarından uzaklığı. Yuvayla birlikte seçim bir
/// kontrol yüksekliğindedir ([`metrics::control`]); sıkışık seçim satır içi
/// kontrol yüksekliğindedir ([`metrics::inline`]), yuvası daha incedir.
const INSET: f32 = 2.0;
const COMPACT_INSET: f32 = 1.0;
/// Parçalar arasındaki aralık; seçili olmayan iki komşunun arasında ince bir
/// çizgi taşır.
const GAP: f32 = 2.0;

/// Parçalı seçimin bir parçası.
struct Segment<Message> {
    text: String,
    selected: bool,
    on_press: Option<Message>,
}

/// Parçalı seçim.
pub struct Segmented<'a, Message> {
    segments: Vec<Segment<Message>>,
    hints: Vec<String>,
    icons: Vec<Icon>,
    icon_only: bool,
    compact: bool,
    accent: bool,
    width: Length,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a, Message: Clone + 'a> Segmented<'a, Message> {
    /// Seçenekler `Display` ile yazılır; seçili seçenek vurgulanır.
    pub fn new<T>(
        options: impl IntoIterator<Item = T>,
        selected: T,
        on_select: impl Fn(T) -> Message,
    ) -> Self
    where
        T: Copy + PartialEq + Display,
    {
        Self::new_with(options, selected, on_select, |_| true)
    }

    /// `new` gibi; `enabled` seçeneği kabul etmezse parça sönük ve basılamaz
    /// olur (ör. içinde nesne olmayan kapsam).
    pub fn new_with<T>(
        options: impl IntoIterator<Item = T>,
        selected: T,
        on_select: impl Fn(T) -> Message,
        enabled: impl Fn(T) -> bool,
    ) -> Self
    where
        T: Copy + PartialEq + Display,
    {
        Self::from_segments(options.into_iter().map(|option| Segment {
            text: option.to_string(),
            selected: option == selected,
            on_press: enabled(option).then(|| on_select(option)),
        }))
    }

    /// Hiçbir parçası seçili olmayabilen parçalı seçim: `selected` `None`
    /// ise bütün parçalar seçilmemiş görünür (ör. çoklu seçimde değerleri
    /// farklı bir evet/hayır alanı).
    pub fn optional<T>(
        options: impl IntoIterator<Item = T>,
        selected: Option<T>,
        on_select: impl Fn(T) -> Message,
    ) -> Self
    where
        T: Copy + PartialEq + Display,
    {
        Self::from_segments(options.into_iter().map(|option| Segment {
            text: option.to_string(),
            selected: Some(option) == selected,
            on_press: Some(on_select(option)),
        }))
    }

    fn from_segments(segments: impl Iterator<Item = Segment<Message>>) -> Self {
        Self {
            segments: segments.collect(),
            hints: Vec::new(),
            icons: Vec::new(),
            icon_only: false,
            compact: false,
            accent: false,
            width: Length::Shrink,
            _lifetime: std::marker::PhantomData,
        }
    }

    /// `Fill` verilirse parçalar genişliği eşit paylaşır. Sabit genişlik yazı
    /// boyutuyla büyür.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Seçeneklerin ipuçları, sırayla: imleç parçanın üstünde durunca ne
    /// olduğunu söyler (ör. "Başladığı noktaya döner.").
    pub fn hints(mut self, hints: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.hints = hints.into_iter().map(Into::into).collect();
        self
    }

    /// Seçeneklerin ikonları, sırayla; adın önünde durur.
    pub fn icons(mut self, icons: impl IntoIterator<Item = Icon>) -> Self {
        self.icons = icons.into_iter().collect();
        self
    }

    /// Parçalarda yalnız ikon: adı ipucuna geçer (araç çubukları, dar
    /// paneller). İkon verilmeyen parça adıyla kalır.
    pub fn icon_only(mut self) -> Self {
        self.icon_only = true;
        self
    }

    /// Sıkışık parçalar: özellik ızgarasının ve tablonun satırına sığar.
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// Seçili parça yumuşak vurgu zemininde, vurgu yazısıyla: çalışma kipini
    /// ya da görünümü değiştiren, ekranda öne çıkan seçimler için (ör. 2B /
    /// 3B). Formlardaki sıradan seçimlerde kullanılmaz.
    pub fn accent(mut self) -> Self {
        self.accent = true;
        self
    }
}

/// Parçanın yazısı: seçiliyken yarı kalın. Genişliği kalın yazıya göre
/// ayrılır (görünmez kalın kopya altta durur), seçim değişince parça
/// genişlemez.
fn face<'a, Message: 'a>(text: String, selected: bool) -> Element<'a, Message> {
    let visible = if selected {
        label::strong(text.clone())
    } else {
        label::body(text.clone())
    };

    stack![
        label::strong(text).style(|_: &Theme| iced::widget::text::Style {
            color: Some(iced::Color::TRANSPARENT),
        }),
        container(visible).center_x(Length::Fill),
    ]
    .into()
}

/// Seçili olmayan iki komşunun arasındaki ince çizgi; seçili parçanın
/// yanında görünmez (kalkık parçanın kendi kenarı vardır).
fn separator<'a, Message: 'a>(visible: bool, height: f32) -> Element<'a, Message> {
    container(
        container(space::horizontal())
            .width(1)
            .height((height * 0.5).round())
            .style(move |theme: &Theme| container::Style {
                background: visible.then(|| Tokens::of(theme).border_strong().into()),
                ..container::Style::default()
            }),
    )
    .width(GAP)
    .height(height)
    .center_y(height)
    .into()
}

impl<'a, Message: Clone + 'a> From<Segmented<'a, Message>> for Element<'a, Message> {
    fn from(segmented: Segmented<'a, Message>) -> Self {
        let fill = segmented.width != Length::Shrink;
        let (outer, inset) = if segmented.compact {
            (metrics::inline(), COMPACT_INSET)
        } else {
            (metrics::control(), INSET)
        };
        let height = outer - 2.0 * inset;
        let padding = if segmented.compact { 8.0 } else { 12.0 };
        let accent = segmented.accent;
        let icon_only = segmented.icon_only;
        let mut hints = segmented.hints.into_iter();
        let mut icons = segmented.icons.into_iter();
        let selected: Vec<bool> = segmented.segments.iter().map(|s| s.selected).collect();

        let mut segments = Row::new().align_y(Center);

        for (index, segment) in segmented.segments.into_iter().enumerate() {
            if index > 0 {
                let visible = !selected[index - 1] && !selected[index];
                segments = segments.push(separator(visible, height));
            }

            let glyph = icons.next();
            let name = segment.text;
            let shows_name = !(icon_only && glyph.is_some());

            let mut content = row![].spacing(6).align_y(Center);

            if let Some(glyph) = glyph {
                content =
                    content.push(icon(glyph).size(if segmented.compact { 12.0 } else { 14.0 }));
            }

            if shows_name {
                content = content.push(face(name.clone(), segment.selected));
            }

            let on = segment.selected;
            let content = container(content).height(height).center_y(height);
            let content = if fill {
                content.center_x(Length::Fill)
            } else {
                content
            };

            let segment_button = button(content)
                .on_press_maybe(segment.on_press)
                .padding([0.0, if shows_name { padding } else { 7.0 }])
                .style(move |theme: &Theme, status| {
                    if accent {
                        style::button::segment_accent(on)(theme, status)
                    } else {
                        style::button::segment(on)(theme, status)
                    }
                });

            let segment_button: Element<'a, Message> = if fill {
                segment_button.width(Length::Fill).into()
            } else {
                segment_button.into()
            };

            // Yalnız ikonlu parçanın ipucu, verilmemişse adıdır.
            let hint = hints.next().or_else(|| (!shows_name).then(|| name.clone()));

            segments = segments.push(match hint {
                Some(hint) => tip(segment_button, Tip::new(hint), Position::Bottom),
                None => segment_button,
            });
        }

        container(segments)
            .padding(inset)
            .height(outer)
            .width(typography::length(segmented.width))
            .style(style::container::segmented)
            .into()
    }
}
