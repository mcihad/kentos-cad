//! Seçim kutusu: aranabilir, açılır seçenek listesi.
//!
//! ```text
//! [Marmara                        ▾]
//! ┌──────────────────────────────────┐
//! │ ⌕ Ara                            │
//! │ ✓ ■ Marmara                   2  │
//! │     ■ Ege                     1  │
//! │     …                            │
//! │ ──────────────────────────────── │
//! │ ◎ Haritadan seç                  │
//! │ ✕ Boş bırak                      │
//! └──────────────────────────────────┘
//! ```
//!
//! Seçeneklerin yanında renk örneği, ikon ve sağda ayrıntı (ör. nesne
//! numarası) gösterilebilir. Uzun listelerde arama kutusu kendiliğinden
//! çıkar ve açılışta odaklanır; arama Türkçe harf ve büyük/küçük harf
//! duyarsızdır. Listenin altına ek komutlar eklenebilir (ör. haritadan
//! seçme).
//!
//! ```ignore
//! Select::new(
//!     REGIONS.map(Choice::new),
//!     selected,
//!     Message::RegionPicked,
//! )
//! .clear(Message::RegionCleared)
//! ```

use std::rc::Rc;

use iced::advanced::widget;
use iced::widget::{Column, button, column, container, row, rule, scrollable, space, text_input};
use iced::{Center, Color, Element, Fill};

use crate::attribute::text;
use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::typography;
use crate::widget::dropdown::{Dropdown, Reaction};
use crate::widget::swatch;

// Ölçüler 12 piksellik gövde metninde tasarlandı ve yazı boyutuyla büyür.

/// Satır yüksekliği.
const ROW_HEIGHT: f32 = 26.0;
/// Listenin en fazla yüksekliği; daha uzun listeler kayar.
const MAX_HEIGHT: f32 = 286.0;
/// Bu kadar seçenekten sonra arama kutusu gösterilir.
const SEARCH_FROM: usize = 8;
/// Listenin en dar ve en geniş hâli; alandan dar olmaz.
const MIN_WIDTH: f32 = 160.0;
const MAX_WIDTH: f32 = 380.0;

/// Listedeki bir seçenek.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    label: String,
    detail: Option<String>,
    color: Option<Color>,
    icon: Option<Icon>,
}

impl Choice {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            detail: None,
            color: None,
            icon: None,
        }
    }

    /// Sağda sönük yazılan ayrıntı (ör. "#12"); aramada da kullanılır.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Adın solundaki renk örneği.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Satırın yaklaşık genişliği.
    fn width(&self) -> f32 {
        typography::text_width(&self.label, typography::body())
            + self.detail.as_deref().map_or(0.0, |detail| {
                typography::mono_width(detail, typography::caption()) + 12.0
            })
            + if self.color.is_some() { 17.0 } else { 0.0 }
            + if self.icon.is_some() { 20.0 } else { 0.0 }
    }

    fn matches(&self, search: &str) -> bool {
        search.is_empty()
            || text::contains(&self.label, search)
            || self
                .detail
                .as_deref()
                .is_some_and(|detail| text::contains(detail, search))
    }
}

/// Panelin iç mesajları.
#[derive(Debug, Clone, PartialEq)]
enum Event {
    Search(String),
    Pick(usize),
    Action(usize),
    Clear,
}

/// Açılır seçim kutusu.
pub struct Select<'a, Message> {
    choices: Vec<Choice>,
    selected: Option<usize>,
    on_select: Rc<dyn Fn(usize) -> Message + 'a>,
    on_clear: Option<Message>,
    actions: Vec<(Icon, String, Message)>,
    placeholder: String,
    searchable: Option<bool>,
    borderless: bool,
}

impl<'a, Message: Clone + 'a> Select<'a, Message> {
    /// `on_select` seçilen seçeneğin sırasını alır.
    pub fn new(
        choices: impl IntoIterator<Item = Choice>,
        selected: Option<usize>,
        on_select: impl Fn(usize) -> Message + 'a,
    ) -> Self {
        Self {
            choices: choices.into_iter().collect(),
            selected,
            on_select: Rc::new(on_select),
            on_clear: None,
            actions: Vec::new(),
            placeholder: "Seçin".to_owned(),
            searchable: None,
            borderless: false,
        }
    }

    /// Seçimi kaldıran "Boş bırak" komutu.
    pub fn clear(mut self, on_clear: Message) -> Self {
        self.on_clear = Some(on_clear);
        self
    }

    /// Listenin altına eklenen komut (ör. haritadan seçme).
    pub fn action(mut self, glyph: Icon, label: impl Into<String>, message: Message) -> Self {
        self.actions.push((glyph, label.into(), message));
        self
    }

    /// Seçim yokken gösterilen metin.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Arama kutusunu zorla gösterir ya da gizler; verilmezse uzun
    /// listelerde gösterilir.
    pub fn searchable(mut self, searchable: bool) -> Self {
        self.searchable = Some(searchable);
        self
    }

    /// Kenarsız görünüm: özellik ızgarası hücreleri için.
    pub fn borderless(mut self) -> Self {
        self.borderless = true;
        self
    }
}

impl<'a, Message: Clone + 'a> From<Select<'a, Message>> for Element<'a, Message> {
    fn from(select: Select<'a, Message>) -> Self {
        let Select {
            choices,
            selected,
            on_select,
            on_clear,
            actions,
            placeholder,
            searchable,
            borderless,
        } = select;

        // Panelin genişliği açıkça verilir: esnek satırlar paneli pencere
        // boyunca yaymasın. İşaret sütunu, boşluklar ve kenar payı eklenir.
        let width = choices
            .iter()
            .map(Choice::width)
            .chain(
                actions
                    .iter()
                    .map(|(_, label, _)| typography::text_width(label, typography::body()) + 20.0),
            )
            .fold(0.0, f32::max);
        let width = (width + 14.0 + 6.0 + 12.0 + 8.0 + 10.0)
            .clamp(typography::scaled(MIN_WIDTH), typography::scaled(MAX_WIDTH));

        let choices = Rc::new(choices);
        let actions = Rc::new(actions);
        let searchable = searchable.unwrap_or(choices.len() >= SEARCH_FROM);
        let search_id = widget::Id::unique();

        let current = selected.and_then(|index| choices.get(index));
        let shown: Element<'a, Message> = match current {
            Some(choice) => {
                let mut shown = row![].spacing(6).align_y(Center);

                if let Some(color) = choice.color {
                    shown = shown.push(swatch(color));
                }

                shown
                    .push(label::body(choice.label.clone()).width(Fill))
                    .into()
            }
            None => label::muted(placeholder).width(Fill).into(),
        };

        let anchor_style: fn(&iced::Theme) -> container::Style = if borderless {
            |_theme| container::Style::default()
        } else {
            style::container::field_box
        };

        let anchor = container(
            row![shown, icon(Icon::ChevronDown).size(12.0).tone(Tone::Muted)]
                .spacing(6)
                .align_y(Center),
        )
        .padding([3, if borderless { 4 } else { 8 }])
        .width(Fill)
        .style(anchor_style);

        let panel = {
            let choices = choices.clone();
            let actions = actions.clone();
            let clearable = on_clear.is_some() && selected.is_some();
            let search_id = search_id.clone();

            move |search: &String| {
                panel(
                    &choices,
                    selected,
                    search,
                    searchable.then(|| search_id.clone()),
                    &actions,
                    clearable,
                    width,
                )
            }
        };

        let reduce = move |search: &mut String, event: Event| match event {
            Event::Search(text) => {
                *search = text;
                Reaction::stay()
            }
            Event::Pick(index) => Reaction::commit(on_select(index)),
            Event::Action(index) => actions
                .get(index)
                .map_or_else(Reaction::close, |(_, _, message)| {
                    Reaction::commit(message.clone())
                }),
            Event::Clear => on_clear
                .clone()
                .map_or_else(Reaction::close, Reaction::commit),
        };

        let dropdown = Dropdown::new(anchor, String::new, panel, reduce).match_width();

        if searchable {
            dropdown.focus(search_id).into()
        } else {
            dropdown.into()
        }
    }
}

/// Açık liste: arama, seçenekler ve komutlar.
fn panel<'a>(
    choices: &[Choice],
    selected: Option<usize>,
    search: &str,
    search_id: Option<widget::Id>,
    actions: &[(Icon, String, impl Clone)],
    clearable: bool,
    width: f32,
) -> Element<'a, Event> {
    let search = search.trim();

    let matches: Vec<(usize, &Choice)> = choices
        .iter()
        .enumerate()
        .filter(|(_, choice)| choice.matches(search))
        .collect();

    let mut content = Column::new().spacing(4);

    if let Some(id) = search_id {
        content = content.push(
            container(
                row![
                    icon(Icon::Search).size(13.0).tone(Tone::Muted),
                    text_input("Ara", search)
                        .id(id)
                        .on_input(Event::Search)
                        .font(typography::ui())
                        .size(typography::body())
                        .padding([3, 0])
                        .style(style::field::bare_input),
                ]
                .spacing(6)
                .align_y(Center),
            )
            .padding([0, 8])
            .style(style::container::field_box),
        );
    }

    let rows = Column::with_children(matches.iter().map(|&(index, choice)| {
        let is_selected = selected == Some(index);

        let mut line = row![
            container(if is_selected {
                Element::from(icon(Icon::Check).size(12.0))
            } else {
                space::horizontal().width(12).into()
            })
            .width(14)
        ]
        .spacing(6)
        .align_y(Center);

        if let Some(color) = choice.color {
            line = line.push(swatch(color));
        }

        if let Some(glyph) = choice.icon {
            line = line.push(icon(glyph).size(14.0));
        }

        line = line.push(label::body(choice.label.clone()).width(Fill));

        if let Some(detail) = &choice.detail {
            line = line.push(label::mono_caption(detail.clone()));
        }

        button(line)
            .on_press(Event::Pick(index))
            .width(Fill)
            .height(typography::scaled(ROW_HEIGHT))
            .padding([0, 6])
            .style(style::button::list_item(is_selected))
            .into()
    }))
    .spacing(1);

    content = if matches.is_empty() {
        content.push(
            container(label::muted("Eşleşen seçenek yok."))
                .padding([6, 8])
                .width(Fill),
        )
    } else {
        let height = (matches.len() as f32 * (typography::scaled(ROW_HEIGHT) + 1.0))
            .min(typography::scaled(MAX_HEIGHT));

        content.push(
            scrollable(rows)
                .direction(style::field::thin_scrollbar())
                .height(height)
                .width(Fill),
        )
    };

    if !actions.is_empty() || clearable {
        content = content.push(rule::horizontal(1).style(style::field::hairline));
    }

    for (index, (glyph, text, _)) in actions.iter().enumerate() {
        content = content.push(command(*glyph, text.clone(), Event::Action(index)));
    }

    if clearable {
        content = content.push(command(Icon::Close, "Boş bırak".to_owned(), Event::Clear));
    }

    container(column![content])
        .padding(4)
        .width(width)
        .style(style::container::popover)
        .into()
}

/// Listenin altındaki komut.
fn command<'a>(glyph: Icon, text: String, event: Event) -> Element<'a, Event> {
    button(
        row![
            container(icon(glyph).size(13.0)).width(14),
            label::body(text)
        ]
        .spacing(6)
        .align_y(Center),
    )
    .on_press(event)
    .width(Fill)
    .height(typography::scaled(ROW_HEIGHT))
    .padding([0, 6])
    .style(style::button::list_item(false))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_matches_labels_and_details_without_case() {
        let choice = Choice::new("İstanbul").detail("#1");

        assert!(choice.matches(""));
        assert!(choice.matches("istanbul"));
        assert!(choice.matches("#1"));
        assert!(!choice.matches("ankara"));
    }
}
