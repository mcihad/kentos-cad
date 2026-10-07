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
use iced::widget::{Row, button, container, responsive, row, space, stack};
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
#[derive(Clone)]
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

/// Genişliği dolduran seçimde parçaların genişlikleri (web'in `flex: 1`'i,
/// `white-space: nowrap` ile): parçalar yuvayı eşit paylaşır, ama hiçbiri
/// yazısından dar olmaz; yazısı payına sığmayan yazısı kadar alır, kalanı
/// öbürleri eşit paylaşır. `room`: parçalara kalan genişlik (aralıklar
/// düşülmüş).
pub fn fill_widths(needs: &[f32], room: f32) -> Vec<f32> {
    let n = needs.len();
    let mut fixed = vec![false; n];
    loop {
        let taken: f32 = (0..n).filter(|&i| fixed[i]).map(|i| needs[i]).sum();
        let free = n - fixed.iter().filter(|f| **f).count();
        if free == 0 {
            return needs.to_vec();
        }
        let share = (room - taken).max(0.0) / free as f32;
        let more: Vec<usize> = (0..n).filter(|&i| !fixed[i] && needs[i] > share).collect();
        if more.is_empty() {
            return (0..n)
                .map(|i| if fixed[i] { needs[i] } else { share })
                .collect();
        }
        for i in more {
            fixed[i] = true;
        }
    }
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
        let width = segmented.width;
        let parts = Parts {
            segments: segmented.segments,
            hints: segmented.hints,
            icons: segmented.icons,
            icon_only: segmented.icon_only,
            compact: segmented.compact,
            accent: segmented.accent,
            height,
        };
        // Filling its width, the row is laid out when the width is known: the parts' shares and
        // their labels' widths decide (fill_widths).
        let content: Element<'a, Message> = if fill {
            responsive(move |size| {
                let n = parts.segments.len();
                let room = size.width - GAP * n.saturating_sub(1) as f32;
                let needs = parts.needs();
                // Unbounded (a row that grows with its contents): each part as wide as its label.
                let widths = if room.is_finite() {
                    fill_widths(&needs, room)
                } else {
                    needs
                };
                parts.row(Some(&widths))
            })
            .height(height)
            .into()
        } else {
            parts.row(None)
        };
        container(content)
            .padding(inset)
            .height(outer)
            .width(typography::length(width))
            .style(style::container::segmented)
            .into()
    }
}

/// What a segmented choice is drawn from, kept so the row can be built again
/// once its width is known.
struct Parts<Message> {
    segments: Vec<Segment<Message>>,
    hints: Vec<String>,
    icons: Vec<Icon>,
    icon_only: bool,
    compact: bool,
    accent: bool,
    height: f32,
}

impl<'a, Message: Clone + 'a> Parts<Message> {
    fn padding(&self, shows_name: bool) -> f32 {
        if !shows_name {
            7.0
        } else if self.compact {
            8.0
        } else {
            12.0
        }
    }

    /// Each part's least width: its label in the strong face (the part keeps that width when
    /// chosen), its icon and its padding.
    fn needs(&self) -> Vec<f32> {
        let size = if self.compact { 12.0 } else { 14.0 };
        self.segments
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let glyph = self.icons.get(i).is_some();
                let shows_name = !(self.icon_only && glyph);
                let mut w = 2.0 * self.padding(shows_name);
                if glyph {
                    w += size;
                }
                if shows_name {
                    w += typography::measured_width(&s.text, typography::body(), true).ceil();
                    if glyph {
                        w += 6.0;
                    }
                }
                w
            })
            .collect()
    }

    /// The parts in a row: `widths` when filling (each part's), else as wide as their contents.
    fn row(&self, widths: Option<&[f32]>) -> Element<'a, Message> {
        let height = self.height;
        let selected: Vec<bool> = self.segments.iter().map(|s| s.selected).collect();
        let mut segments = Row::new().align_y(Center);
        for (index, segment) in self.segments.iter().enumerate() {
            if index > 0 {
                let visible = !selected[index - 1] && !selected[index];
                segments = segments.push(separator(visible, height));
            }
            let glyph = self.icons.get(index).copied();
            let name = segment.text.clone();
            let shows_name = !(self.icon_only && glyph.is_some());
            let mut content = row![].spacing(6).align_y(Center);
            if let Some(glyph) = glyph {
                content = content.push(icon(glyph).size(if self.compact { 12.0 } else { 14.0 }));
            }
            if shows_name {
                content = content.push(face(name.clone(), segment.selected));
            }
            let on = segment.selected;
            let accent = self.accent;
            let content = container(content).height(height).center_y(height);
            let content = if widths.is_some() {
                content.center_x(Length::Fill)
            } else {
                content
            };
            let segment_button = button(content)
                .on_press_maybe(segment.on_press.clone())
                .padding([0.0, self.padding(shows_name)])
                .style(move |theme: &Theme, status| {
                    if accent {
                        style::button::segment_accent(on)(theme, status)
                    } else {
                        style::button::segment(on)(theme, status)
                    }
                });
            let segment_button: Element<'a, Message> = match widths.and_then(|w| w.get(index)) {
                Some(w) => segment_button.width(Length::Fixed(*w)).into(),
                None => segment_button.into(),
            };
            // Yalnız ikonlu parçanın ipucu, verilmemişse adıdır.
            let hint = self
                .hints
                .get(index)
                .cloned()
                .or_else(|| (!shows_name).then(|| name.clone()));
            segments = segments.push(match hint {
                Some(hint) => tip(segment_button, Tip::new(hint), Position::Bottom),
                None => segment_button,
            });
        }
        segments.into()
    }
}

#[cfg(test)]
mod tests {
    use super::fill_widths;

    #[test]
    fn parts_share_the_width_but_none_is_narrower_than_its_label() {
        // Every label fits its share: equal parts.
        assert_eq!(fill_widths(&[60.0, 40.0, 50.0], 300.0), vec![100.0; 3]);
        // A long label takes its width; the others share the rest.
        assert_eq!(
            fill_widths(&[120.0, 50.0, 40.0], 300.0),
            vec![120.0, 90.0, 90.0]
        );
        // Too narrow for all: each its label (the row is wider than its room).
        assert_eq!(
            fill_widths(&[120.0, 110.0, 100.0], 300.0),
            vec![120.0, 110.0, 100.0]
        );
    }
}
