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
//! göre ayrılır, seçim değişince parçalar yerinden oynamaz. Seçim yerinden
//! taşmaz: sığmayınca parçalar ikonu varsa yalnız ikonlarıyla, yoksa yazıları
//! “…” ile kısalarak durur, tamamı ipucundadır ([`natural_width`] sığması
//! için gereken genişliği söyler). Parçalar ikon
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
use crate::widget::Responsive;
use crate::widget::elided::Elided;
use crate::widget::tip::{Tip, tip};

/// Parçaların yuvanın kenarından uzaklığı. Yuvayla birlikte seçim bir kontrol
/// yüksekliğindedir ([`metrics::control`]), sıkışığı da: yalnız yanları dardır.
const INSET: f32 = 2.0;
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
    /// Hücreyi dolduran görünüm ([`Segmented::cell`]).
    cell: bool,
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
            cell: false,
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

    /// Izgara hücresini dolduran görünüm (docs/adr/0804): yuvası yok, parçalar
    /// hücrenin bütün yüksekliğini ve genişliğini ince çizgilerle paylaşır.
    pub fn cell(mut self) -> Self {
        self.cell = true;
        self.compact = true;
        self.width = Length::Fill;
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
fn separator<'a, Message: 'a>(visible: bool, height: f32, cell: bool) -> Element<'a, Message> {
    // In a cell the parts share its whole height, a hairline from edge to edge between them.
    if cell {
        return container(space::horizontal())
            .width(1)
            .height(height)
            .style(|theme: &Theme| container::Style {
                background: Some(Tokens::of(theme).rule().into()),
                ..container::Style::default()
            })
            .into();
    }
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
/// diğerleri eşit paylaşır. `room`: parçalara kalan genişlik (aralıklar
/// düşülmüş). Yazılar birlikte sığmıyorsa [`shrunk_widths`].
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

/// Yazıları birlikte `room`'a sığmayan parçaların genişlikleri (docs/adr/0749):
/// dar parçalar kendi genişliğini tutar, geniş olanlar kalanı eşit paylaşır;
/// hiçbiri `least`'ten dar olmaz. Seçili parça (`keep`) diğerlerine en az
/// `least` kalıyorsa yazısının tamamını tutar.
pub fn shrunk_widths(needs: &[f32], room: f32, keep: Option<usize>, least: f32) -> Vec<f32> {
    if let Some(k) = keep.filter(|&k| k < needs.len() && needs.len() > 1) {
        let others: Vec<f32> = needs
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != k)
            .map(|(_, w)| *w)
            .collect();
        let left = room - needs[k];
        if left >= least * others.len() as f32 {
            let mut widths = shared(&others, left, least);
            widths.insert(k, needs[k]);
            return widths;
        }
    }
    shared(needs, room, least)
}

/// `room` shared so that the narrow keep their width and the wide get the same cap, none under `least`.
fn shared(needs: &[f32], room: f32, least: f32) -> Vec<f32> {
    let mut sorted = needs.to_vec();
    sorted.sort_by(f32::total_cmp);
    let mut left = room.max(0.0);
    let mut cap = f32::INFINITY;
    for (i, need) in sorted.iter().enumerate() {
        let even = left / (sorted.len() - i) as f32;
        if *need > even {
            cap = even;
            break;
        }
        left -= need;
    }
    needs.iter().map(|&need| need.min(cap.max(least))).collect()
}

/// Bir seçimin yazılarını kısaltmadan göstermesi için gereken genişlik (yuva ve
/// aralıklar dahil, ikonsuz): seçimi taşıyan bölme en az bu kadar olur ki
/// yazılar kısalmasın (docs/adr/0749).
pub fn natural_width(labels: &[&str], compact: bool) -> f32 {
    let padding = if compact { 8.0 } else { 12.0 };
    let inset = INSET;
    let parts: f32 = labels
        .iter()
        .map(|l| 2.0 * padding + typography::measured_width(l, typography::body(), true).ceil())
        .sum();
    parts + GAP * labels.len().saturating_sub(1) as f32 + 2.0 * inset
}

impl<'a, Message: Clone + 'a> From<Segmented<'a, Message>> for Element<'a, Message> {
    fn from(segmented: Segmented<'a, Message>) -> Self {
        let fill = segmented.width != Length::Shrink;
        let (outer, inset) = if segmented.cell {
            (metrics::row(), 0.0)
        } else {
            (metrics::control(), INSET)
        };
        let height = outer - 2.0 * inset;
        let width = segmented.width;
        // ← → (and ↑ ↓) choose the next part that takes a press, from the chosen one.
        let presses: Vec<Option<Message>> = segmented
            .segments
            .iter()
            .map(|s| s.on_press.clone())
            .collect();
        let at = segmented.segments.iter().position(|s| s.selected);
        let parts = Parts {
            segments: segmented.segments,
            hints: segmented.hints,
            icons: segmented.icons,
            icon_only: segmented.icon_only,
            compact: segmented.compact,
            accent: segmented.accent,
            cell: segmented.cell,
            height,
        };
        // The row is laid out when its room is known: filling it, the parts' shares and their labels'
        // widths decide (fill_widths); short of it, the labels give way (docs/adr/0749).
        let content: Element<'a, Message> =
            Responsive::new(move |size| parts.fitted(size.width, fill))
                .width(if fill { Length::Fill } else { Length::Shrink })
                .height(height)
                .into();
        let boxed = container(content)
            .padding(inset)
            .height(outer)
            .width(typography::length(width));
        let boxed = if segmented.cell {
            boxed
        } else {
            boxed.style(style::container::segmented)
        };
        super::key_focus(boxed)
            .on_step(move |by| step_to(&presses, at, by))
            .into()
    }
}

/// The message of the part `by` away from `at` that takes a press (`at` none: from before the first
/// or after the last); none past the ends.
pub(crate) fn step_to<Message: Clone>(
    presses: &[Option<Message>],
    at: Option<usize>,
    by: i32,
) -> Option<Message> {
    let n = presses.len() as i64;
    let mut i = at.map_or(if by > 0 { -1 } else { n }, |a| a as i64);
    loop {
        i += i64::from(by.signum());
        if i < 0 || i >= n {
            return None;
        }
        if let Some(message) = &presses[i as usize] {
            return Some(message.clone());
        }
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
    cell: bool,
    height: f32,
}

impl<'a, Message: Clone + 'a> Parts<Message> {
    /// The room between two parts.
    fn gap(&self) -> f32 {
        if self.cell { 1.0 } else { GAP }
    }

    fn padding(&self, shows_name: bool) -> f32 {
        if !shows_name {
            7.0
        } else if self.compact {
            8.0
        } else {
            12.0
        }
    }

    /// The parts in `room`: as wide as their labels (or sharing it when filling) when they fit;
    /// else their icons alone when every part has one, else their labels cut with “…”, the whole
    /// label in the tip (docs/adr/0749).
    fn fitted(&self, room: f32, fill: bool) -> Element<'a, Message> {
        let n = self.segments.len();
        let room = room - self.gap() * n.saturating_sub(1) as f32;
        let fits = |needs: &[f32]| !room.is_finite() || needs.iter().sum::<f32>() <= room + 0.5;
        let laid = |needs: &[f32], icon_only: bool| {
            if fill && room.is_finite() {
                self.row(Some(&fill_widths(needs, room)), icon_only)
            } else {
                self.row(None, icon_only)
            }
        };
        let needs = self.needs(self.icon_only);
        if fits(&needs) {
            return laid(&needs, self.icon_only);
        }
        if !self.icon_only && n > 0 && self.icons.len() >= n {
            let bare = self.needs(true);
            if fits(&bare) {
                return laid(&bare, true);
            }
        }
        let keep = self.segments.iter().position(|s| s.selected);
        let least = 2.0 * self.padding(true)
            + typography::measured_width("W…", typography::body(), true).ceil();
        let widths = shrunk_widths(&needs, room, keep, least);
        self.row(Some(&widths), self.icon_only)
    }

    /// Each part's least width: its label in the strong face (the part keeps that width when
    /// chosen), its icon and its padding; `icon_only`: its icon alone when it has one.
    fn needs(&self, icon_only: bool) -> Vec<f32> {
        let size = if self.compact { 12.0 } else { 14.0 };
        self.segments
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let glyph = self.icons.get(i).is_some();
                let shows_name = !(icon_only && glyph);
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

    /// The parts in a row: `widths` when filling or short of room (each part's), else as wide as
    /// their contents; a part narrower than its label shows it cut. `icon_only`: the icons alone.
    fn row(&self, widths: Option<&[f32]>, icon_only: bool) -> Element<'a, Message> {
        let height = self.height;
        let needs = widths.map(|_| self.needs(icon_only));
        let selected: Vec<bool> = self.segments.iter().map(|s| s.selected).collect();
        let mut segments = Row::new().align_y(Center);
        for (index, segment) in self.segments.iter().enumerate() {
            if index > 0 {
                let visible = !selected[index - 1] && !selected[index];
                segments = segments.push(separator(visible, height, self.cell));
            }
            let glyph = self.icons.get(index).copied();
            let name = segment.text.clone();
            let shows_name = !(icon_only && glyph.is_some());
            let cut = shows_name
                && matches!(
                    (widths.and_then(|w| w.get(index)), needs.as_ref().and_then(|n| n.get(index))),
                    (Some(w), Some(n)) if w + 0.5 < *n
                );
            let mut content = row![].spacing(6).align_y(Center);
            if let Some(glyph) = glyph {
                content = content.push(icon(glyph).size(if self.compact { 12.0 } else { 14.0 }));
            }
            if cut {
                let font = if segment.selected {
                    typography::ui_strong()
                } else {
                    typography::ui()
                };
                content = content.push(
                    Elided::new(name.clone())
                        .size(typography::body())
                        .font(font)
                        .width(Length::Fill),
                );
            } else if shows_name {
                content = content.push(face(name.clone(), segment.selected));
            }
            let on = segment.selected;
            let accent = self.accent;
            let in_cell = self.cell;
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
                    if in_cell {
                        style::button::segment_cell(on)(theme, status)
                    } else if accent {
                        style::button::segment_accent(on)(theme, status)
                    } else {
                        style::button::segment(on)(theme, status)
                    }
                });
            let segment_button: Element<'a, Message> = match widths.and_then(|w| w.get(index)) {
                Some(w) => segment_button.width(Length::Fixed(*w)).into(),
                None => segment_button.into(),
            };
            // Yalnız ikonlu parçanın ipucu, verilmemişse adıdır; yazısı kısalanınki
            // kısaldığında adının tamamıdır.
            let hint = self
                .hints
                .get(index)
                .cloned()
                .map(Tip::new)
                .or_else(|| (!shows_name).then(|| Tip::new(name.clone())))
                .or_else(|| cut.then(|| Tip::new(name.clone()).when_cut()));
            segments = segments.push(match hint {
                Some(hint) => tip(segment_button, hint, Position::Bottom),
                None => segment_button,
            });
        }
        segments.into()
    }
}

#[cfg(test)]
mod tests {
    use super::{fill_widths, shrunk_widths};

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

    #[test]
    fn short_of_room_the_wide_parts_share_what_is_left() {
        // The narrow keep their width, the widest get the same cap.
        assert_eq!(
            shrunk_widths(&[93.0, 86.0, 115.0, 80.0], 363.0, None, 20.0),
            vec![93.0, 86.0, 104.0, 80.0]
        );
        // The chosen one keeps its label whole, the others share the rest.
        assert_eq!(
            shrunk_widths(&[100.0, 100.0, 100.0], 250.0, Some(1), 20.0),
            vec![75.0, 100.0, 75.0]
        );
        // Too little for that: all share alike.
        assert_eq!(
            shrunk_widths(&[100.0, 100.0, 100.0], 120.0, Some(1), 20.0),
            vec![40.0, 40.0, 40.0]
        );
        // Never under the least.
        assert_eq!(
            shrunk_widths(&[100.0, 100.0], 20.0, None, 20.0),
            vec![20.0, 20.0]
        );
    }

    /// The texts a view shows: their words and where their right edges are.
    #[cfg(feature = "snapshot")]
    fn texts(
        snapshot: &mut crate::snapshot::Snapshot,
        view: iced::Element<'_, ()>,
    ) -> Vec<(String, f32)> {
        use iced::Rectangle;
        use iced::advanced::widget::{Id, Operation};
        use std::sync::{Arc, Mutex};
        struct Find(Arc<Mutex<Vec<(String, f32)>>>);
        impl Operation for Find {
            fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
                operate(self);
            }
            fn text(&mut self, _: Option<&Id>, bounds: Rectangle, text: &str) {
                self.0
                    .lock()
                    .expect("a lock")
                    .push((text.to_owned(), bounds.x + bounds.width));
            }
        }
        let found = Arc::new(Mutex::new(Vec::new()));
        snapshot.operate(view, Box::new(Find(found.clone())));
        found.lock().map_or(Vec::new(), |f| f.clone())
    }

    #[cfg(feature = "snapshot")]
    #[test]
    fn a_row_short_of_room_stays_in_it_its_labels_cut_and_whole_in_their_tips() {
        use super::{Segmented, natural_width};
        use crate::icon::Icon;
        use crate::snapshot::{Input, Snapshot};
        use iced::widget::{column, container};
        use iced::{Point, Size};
        use std::time::Duration;
        const TABS: [&str; 4] = ["Özellikler", "Önizleme", "Değişiklikler", "İş akışı"];
        fn view(_: &()) -> iced::Element<'_, ()> {
            column![
                container(Segmented::new(TABS, "Önizleme", |_| ())).width(220),
                container(Segmented::new(TABS, "Önizleme", |_| ()).width(iced::Fill)).width(220),
                container(Segmented::new(TABS, "Önizleme", |_| ()).icons([
                    Icon::Properties,
                    Icon::Eye,
                    Icon::Clock,
                    Icon::Play
                ]))
                .width(220),
            ]
            .spacing(10)
            .into()
        }
        assert!(
            natural_width(&TABS, false) > 220.0,
            "the four labels need more than the pane"
        );
        let mut snapshot = Snapshot::new(Size::new(400.0, 200.0)).expect("a renderer");
        let shown = texts(&mut snapshot, view(&()));
        // Nothing past the pane. Each label in the two rows of labels, whole or cut; the row of icons shows
        // none, its parts' names are in their tips.
        for (words, right) in &shown {
            assert!(*right <= 220.5, "“{words}” ends at {right}");
        }
        for tab in TABS {
            assert_eq!(
                shown.iter().filter(|(w, _)| w == tab).count(),
                2,
                "{tab}: {shown:?}"
            );
        }
        // A cut label's whole text in its tip when the pointer rests on it.
        let mut update = |_: &mut (), _: ()| {};
        let at = Point::new(200.0, 14.0);
        snapshot.input(&mut (), view, &mut update, Input::Move(at));
        snapshot.wait(Duration::from_millis(500));
        snapshot.input(
            &mut (),
            view,
            &mut update,
            Input::Move(Point::new(at.x + 1.0, at.y)),
        );
        let shown = texts(&mut snapshot, view(&()));
        assert_eq!(
            shown.iter().filter(|(w, _)| w == "İş akışı").count(),
            3,
            "the last part's tip: {shown:?}"
        );
    }
}
