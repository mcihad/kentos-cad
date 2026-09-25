//! Varlık tarayıcısı: semboller, bloklar ve malzemeler gibi kitaplık
//! öğelerinin aranabilir, kategorili ızgarası.
//!
//! ```text
//! [⌕ Ara…               ]                              ▦  ☰
//! [Tümü] [Donatı] [Bitki] [Altyapı]
//! ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐
//! │  ✿   │ │  ⊕   │ │  ◎   │ │  ▭   │
//! │ Ağaç │ │ Direk│ │ Rögar│ │ Bank │
//! └──────┘ └──────┘ └──────┘ └──────┘
//! ```
//!
//! Öğelerin önizlemesi uygulamanındır (ikon, tuval, resim). Tarayıcı
//! yazılanı Türkçe harf ayırmadan ada ve kategoriye göre süzer; kategoriler
//! öğelerden çıkarılır. Tıklamak seçer, çift tıklamak kullanır (ör. blok
//! ekle). Izgara ve liste görünümü arasında geçilir.
//!
//! ```ignore
//! AssetBrowser::new(assets, self.selected, Message::AssetSelected)
//!     .on_activate(Message::AssetInserted)
//!     .search(&self.query, Message::AssetSearch)
//!     .category(self.category.as_deref(), Message::AssetCategory)
//!     .view(self.view, Message::AssetView)
//! ```

use iced::advanced::layout::{self, Layout};
use iced::advanced::mouse::Click;
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::widget::text::Wrapping;
use iced::widget::{Column, Row, button, column, container, row, scrollable, space, text_input};
use iced::{Center, Element, Event, Fill, Length, Rectangle, Renderer, Size, Theme, Vector, mouse};

use crate::attribute::text;
use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::typography;

/// Izgaradaki kutunun kenarı ve önizlemenin boyu (12 piksellik gövde
/// metnine göre).
const TILE: f32 = 84.0;
const PREVIEW: f32 = 48.0;

/// Kitaplık öğesi: adı, kategorisi, önizlemesi ve isteğe bağlı ayrıntı.
pub struct Asset<'a, Message> {
    name: String,
    category: String,
    detail: Option<String>,
    preview: Element<'a, Message>,
}

impl<'a, Message> Asset<'a, Message> {
    pub fn new(
        name: impl Into<String>,
        category: impl Into<String>,
        preview: impl Into<Element<'a, Message>>,
    ) -> Self {
        Self {
            name: name.into(),
            category: category.into(),
            detail: None,
            preview: preview.into(),
        }
    }

    /// Liste görünümünde adın sağında yazar (ör. "2 katman", "12 KB").
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// Görünüm: önizlemeli ızgara ya da satırlar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Grid,
    List,
}

type OnIndex<'a, Message> = Box<dyn Fn(usize) -> Message + 'a>;
type OnText<'a, Message> = Box<dyn Fn(String) -> Message + 'a>;
type OnCategory<'a, Message> = Box<dyn Fn(Option<String>) -> Message + 'a>;
type OnView<'a, Message> = Box<dyn Fn(View) -> Message + 'a>;

/// Varlık tarayıcısı.
pub struct AssetBrowser<'a, Message> {
    assets: Vec<Asset<'a, Message>>,
    selected: Option<usize>,
    on_select: OnIndex<'a, Message>,
    on_activate: Option<OnIndex<'a, Message>>,
    search: Option<(&'a str, OnText<'a, Message>)>,
    category: Option<(Option<&'a str>, OnCategory<'a, Message>)>,
    view: Option<(View, OnView<'a, Message>)>,
    height: Length,
}

impl<'a, Message: Clone + 'a> AssetBrowser<'a, Message> {
    /// `selected` öğelerin sırasıdır (süzmeden önceki).
    pub fn new(
        assets: impl IntoIterator<Item = Asset<'a, Message>>,
        selected: Option<usize>,
        on_select: impl Fn(usize) -> Message + 'a,
    ) -> Self {
        Self {
            assets: assets.into_iter().collect(),
            selected,
            on_select: Box::new(on_select),
            on_activate: None,
            search: None,
            category: None,
            view: None,
            height: Length::Shrink,
        }
    }

    /// Çift tıklanınca (ör. bloğu ekle).
    pub fn on_activate(mut self, on_activate: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_activate = Some(Box::new(on_activate));
        self
    }

    /// Arama kutusu ve yazılan.
    pub fn search(mut self, query: &'a str, on_search: impl Fn(String) -> Message + 'a) -> Self {
        self.search = Some((query, Box::new(on_search)));
        self
    }

    /// Kategori süzgeci; `None` bütün kategoriler.
    pub fn category(
        mut self,
        category: Option<&'a str>,
        on_category: impl Fn(Option<String>) -> Message + 'a,
    ) -> Self {
        self.category = Some((category, Box::new(on_category)));
        self
    }

    /// Izgara ya da liste; düğmelerle değişir.
    pub fn view(mut self, view: View, on_view: impl Fn(View) -> Message + 'a) -> Self {
        self.view = Some((view, Box::new(on_view)));
        self
    }

    /// Öğelerin alanının yüksekliği; verilirse kayar.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
}

/// Öğe arama ve kategoriye uyuyor mu.
fn matches(name: &str, category: &str, query: &str, selected: Option<&str>) -> bool {
    let query = query.trim();

    selected.is_none_or(|selected| selected == category)
        && (query.is_empty() || text::contains(name, query) || text::contains(category, query))
}

impl<'a, Message: Clone + 'a> From<AssetBrowser<'a, Message>> for Element<'a, Message> {
    fn from(browser: AssetBrowser<'a, Message>) -> Self {
        let query = browser.search.as_ref().map_or("", |(query, _)| *query);
        let selected_category = browser
            .category
            .as_ref()
            .and_then(|(category, _)| *category);
        let view = browser.view.as_ref().map_or(View::Grid, |(view, _)| *view);

        // Kategoriler öğelerdeki ilk görünüş sırasıyla.
        let mut categories: Vec<String> = Vec::new();

        for asset in &browser.assets {
            if !categories.contains(&asset.category) {
                categories.push(asset.category.clone());
            }
        }

        let mut header = Column::new().spacing(6);
        let mut top = Row::new().spacing(6).align_y(Center);

        if let Some((query, on_search)) = browser.search {
            top = top.push(
                text_input("Kitaplıkta ara", query)
                    .on_input(on_search)
                    .font(typography::ui())
                    .size(typography::body())
                    .padding([3, 8])
                    .width(Fill)
                    .style(style::field::input),
            );
        } else {
            top = top.push(space::horizontal());
        }

        if let Some((current, on_view)) = &browser.view {
            for (option, glyph, name) in [
                (View::Grid, Icon::Grid, "Izgara"),
                (View::List, Icon::Table, "Liste"),
            ] {
                top = top.push(crate::widget::tip(
                    button(icon(glyph).size(14.0))
                        .on_press(on_view(option))
                        .padding(4)
                        .style(style::button::toggle(*current == option)),
                    crate::widget::Tip::new(name),
                    iced::widget::tooltip::Position::Top,
                ));
            }
        }

        header = header.push(top);

        if let Some((selected, on_category)) = &browser.category {
            let chip = |name: String, active: bool, message: Message| {
                button(label::caption(name).style(if active {
                    style::text::default
                } else {
                    style::text::muted
                }))
                .on_press(message)
                .padding([2, 8])
                .style(style::button::toggle(active))
            };

            let mut chips = Row::new().spacing(4).push(chip(
                "Tümü".to_owned(),
                selected.is_none(),
                on_category(None),
            ));

            for category in &categories {
                chips = chips.push(chip(
                    category.clone(),
                    *selected == Some(category.as_str()),
                    on_category(Some(category.clone())),
                ));
            }

            header = header.push(chips.wrap());
        }

        let on_select = browser.on_select;
        let on_activate = browser.on_activate;
        let chosen = browser.selected;
        let tile = typography::scaled(TILE).round();
        let preview_size = typography::scaled(PREVIEW).round();

        let items: Vec<Element<'a, Message>> = browser
            .assets
            .into_iter()
            .enumerate()
            .filter(|(_, asset)| matches(&asset.name, &asset.category, query, selected_category))
            .map(|(index, asset)| {
                let selected = chosen == Some(index);
                let content: Element<'a, Message> = match view {
                    View::Grid => column![
                        container(asset.preview).center(preview_size),
                        label::caption(asset.name)
                            .style(style::text::default)
                            .wrapping(Wrapping::None)
                            .center(),
                    ]
                    .spacing(4)
                    .align_x(Center)
                    .width(tile - 8.0)
                    .into(),
                    View::List => {
                        let mut line = row![
                            container(asset.preview).center(typography::scaled(24.0).round()),
                            label::body(asset.name).wrapping(Wrapping::None),
                            space::horizontal(),
                            label::caption(asset.category),
                        ]
                        .spacing(8)
                        .align_y(Center);

                        if let Some(detail) = asset.detail {
                            line = line.push(label::mono_caption(detail));
                        }

                        line.into()
                    }
                };

                let item = button(content)
                    .on_press(on_select(index))
                    .padding(match view {
                        View::Grid => [6, 4],
                        View::List => [3, 8],
                    })
                    .style(style::button::tool(selected));
                let item: Element<'a, Message> = match view {
                    View::Grid => item.width(tile).into(),
                    View::List => item.width(Fill).into(),
                };

                match &on_activate {
                    Some(on_activate) => DoubleClick {
                        content: item,
                        message: on_activate(index),
                    }
                    .into(),
                    None => item,
                }
            })
            .collect();

        let body: Element<'a, Message> = if items.is_empty() {
            container(
                column![
                    icon(Icon::Search).size(18.0).tone(Tone::Muted),
                    label::muted("Aramaya uyan öğe yok."),
                ]
                .spacing(6)
                .align_x(Center),
            )
            .center_x(Fill)
            .padding(16)
            .into()
        } else {
            match view {
                View::Grid => Row::with_children(items).spacing(6).wrap().into(),
                View::List => Column::with_children(items).spacing(1).into(),
            }
        };

        let body: Element<'a, Message> = match browser.height {
            Length::Shrink => body,
            height => scrollable(container(body).width(Fill).padding(iced::Padding {
                right: 10.0,
                ..iced::Padding::ZERO
            }))
            .direction(style::field::thin_scrollbar())
            .width(Fill)
            .height(height)
            .into(),
        };

        column![header, body].spacing(8).into()
    }
}

/// Çift tıklanınca mesaj gönderen sarmalayıcı; tek tıklama içeriğe
/// ulaşır (ör. düğme öğeyi seçer).
struct DoubleClick<'a, Message> {
    content: Element<'a, Message>,
    message: Message,
}

#[derive(Default)]
struct ClickState {
    last: Option<Click>,
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for DoubleClick<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<ClickState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(ClickState::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
            && let Some(position) = cursor.position_over(layout.bounds())
        {
            let state = tree.state.downcast_mut::<ClickState>();
            let click = Click::new(position, mouse::Button::Left, state.last);

            if click.kind() == iced::advanced::mouse::click::Kind::Double {
                shell.publish(self.message.clone());
            }

            state.last = Some(click);
        }

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: Clone + 'a> From<DoubleClick<'a, Message>> for Element<'a, Message> {
    fn from(wrapper: DoubleClick<'a, Message>) -> Self {
        Element::new(wrapper)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assets_match_by_name_or_category_ignoring_turkish_case() {
        assert!(matches("Çınar", "Bitki", "çın", None));
        assert!(matches("Çınar", "Bitki", "BİTKİ", None));
        assert!(!matches("Çınar", "Bitki", "rögar", None));
        assert!(matches("Rögar", "Altyapı", "", Some("Altyapı")));
        assert!(!matches("Rögar", "Altyapı", "", Some("Bitki")));
    }
}

/// Gerçek olaylarla: tıklamak seçer, çift tıklamak kullanır, arama süzer.
#[cfg(all(test, feature = "snapshot"))]
mod interaction {
    use iced::widget::space;
    use iced::{Element, Point, Size};

    use super::{Asset, AssetBrowser};
    use crate::snapshot::{Input, Snapshot};

    #[derive(Debug, Clone, PartialEq)]
    enum Event {
        Selected(usize),
        Activated(usize),
    }

    type Events = Vec<Event>;

    fn view(_events: &Events) -> Element<'_, Event> {
        let asset =
            |name: &str| Asset::new(name, "Donatı", space::horizontal().width(20).height(20));

        AssetBrowser::new(
            [asset("Bank"), asset("Direk"), asset("Rögar")],
            None,
            Event::Selected,
        )
        .on_activate(Event::Activated)
        .into()
    }

    #[test]
    fn clicks_select_and_double_clicks_activate() {
        let mut snapshot = Snapshot::new(Size::new(400.0, 200.0)).expect("çizici kurulamadı");
        let mut events: Events = Vec::new();
        let mut update = |events: &mut Events, event| events.push(event);
        let mut input =
            |events: &mut Events, input| snapshot.input(events, view, &mut update, input);

        // İkinci kutu: ızgaranın solundan bir kutu ve boşluk sonra.
        let tile = crate::theme::typography::scaled(super::TILE).round();
        let second = Point::new(tile + 6.0 + tile / 2.0, 30.0);

        input(&mut events, Input::Click(second));
        assert_eq!(events, [Event::Selected(1)]);

        input(&mut events, Input::Click(second));
        assert!(events.contains(&Event::Activated(1)));
    }
}
