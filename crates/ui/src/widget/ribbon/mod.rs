//! Şerit (ribbon): sekme şeridi ve seçili sekmenin araç grupları.
//!
//! ```text
//! ┌──────────────┬───────┬──────┬──────────┬─────────────────── meta ┐
//! │ ■ KentOS ⌄   │ Giriş │ Ekle │ Açıklama │                          │
//! ├──────────────┘       └──────┴──────────┴──────────────────────────┤
//! │ ┌────┐ ┌────┐ ▢ Küçük   │ ┌────┐ ▢ Küçük   │ ...                   │
//! │ │ ⊕  │ │ ⊕  │ ▢ Küçük   │ │ ⊕  │ ▢ Küçük   │                       │
//! │ └────┘ └────┘ ▢ Küçük   │ └────┘           │                       │
//! │        Grup adı         │   Grup adı       │                       │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! Grup içeriği 3 satırlık bir ızgaraya oturur: küçük düğmeler ve alanlar
//! bir satır ([`row_height`]), büyük düğmeler üç satır ([`content_height`])
//! yüksekliğindedir.
//!
//! Düğmelerle kurulan gruplar pencereye sığar (DESIGN.md §7.3.1): paneller
//! sağdan sola, her biri bir adım inerek küçülür: büyük → küçük etiketli →
//! yalnız ikon → panelin adını taşıyan tek düğme ([`fit`]). Hiçbir düğme
//! kesilmez; en küçük hâli de sığmayan şerit (çok dar pencere) kaydırılır.
//! Seçili sekmenin alt çizgisi yoktur ve zemini panelle aynıdır; sekme
//! alttaki panele akar.
//!
//! Sekme şeridinde uygulama düğmesinin yanında hızlı erişim düğmeleri
//! durur ([`Ribbon::quick`]); sağ uçtaki düğme şeridi daraltır, daraltılmış
//! şeritte yalnızca sekmeler görünür ([`Ribbon::collapsible`]). Düğmeler
//! menülü ya da bölünmüş olabilir ([`Button::menu`]); [`Gallery`]
//! seçeneklerin önizlemelerini dizer.

mod control;
mod layout;

pub use control::{AppButton, Button, logo_mark};
pub use layout::{Choice, Field, Gallery, Group, Level, Preview, Row, Stack, Tile};

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{
    Column, button, column, container, responsive, row, scrollable, space, tooltip,
};
use iced::{Element, Fill, Length, Padding, Right};

use crate::icon::{Icon, icon};
use crate::label;
use crate::style;
use crate::theme::typography;
use crate::widget::context_menu::{ContextMenu, Menu, MenuButton};
use crate::widget::key_tip::{KeyTip, Place, key_tip};
use crate::widget::{Tip, horizontal_divider, tip, vertical_divider};

// Ölçüler 12 piksellik gövde metninde tasarlandı; metni taşıyanlar yazı
// boyutuyla büyür (aşağıdaki fonksiyonlar). İkonlar sabit boyuttadır.

/// Izgara satırının yüksekliği: küçük düğme, alan.
pub const ROW: f32 = 24.0;
/// Izgara satırları arasındaki boşluk.
pub const ROW_GAP: f32 = 1.0;
/// Küçük düğme ikonlarının boyutu; alanların önündeki sütun genişliği.
pub const ICON: f32 = 16.0;
/// Büyük düğme ikonlarının boyutu.
pub const LARGE_ICON: f32 = 28.0;
/// Büyük ikonların çizgi kalınlığı: büyüyünce ağırlaşmaz.
pub const LARGE_WEIGHT: f32 = 1.55;

/// Grup adının satır yüksekliği.
pub const CAPTION: f32 = 17.0;
const PANEL_PADDING_TOP: f32 = 6.0;
const PANEL_PADDING_BOTTOM: f32 = 2.0;

/// Sekme başlıklarının yüksekliği.
pub const TAB_HEIGHT: f32 = 30.0;
/// Seçili sekmenin üstündeki vurgu çizgisi.
const TAB_ACCENT: f32 = 2.0;
/// Sekme şeridinin pencere kenarından uzaklığı.
const STRIP_TOP: f32 = 4.0;

/// Geçerli yazı boyutunda ızgara satırının yüksekliği.
pub fn row_height() -> f32 {
    typography::scaled(ROW)
}

/// Grup içeriğinin yüksekliği: üç satır.
pub fn content_height() -> f32 {
    row_height() * 3.0 + ROW_GAP * 2.0
}

/// Grup adının satır yüksekliği.
pub fn caption_height() -> f32 {
    typography::scaled(CAPTION)
}

/// Araç panelinin yüksekliği (alt kenar hariç).
pub fn panel_height() -> f32 {
    PANEL_PADDING_TOP + content_height() + caption_height() + PANEL_PADDING_BOTTOM
}

/// Sekme başlıklarının yüksekliği.
pub fn tab_height() -> f32 {
    typography::scaled(TAB_HEIGHT)
}

/// Sekme şeridinin toplam yüksekliği; uygulama menüsü bunun hemen altına
/// açılır.
pub fn strip_height() -> f32 {
    STRIP_TOP + tab_height()
}

const EDGE: f32 = 10.0;

/// Şerit.
pub struct Ribbon<'a, Message> {
    application: Option<AppButton<'a, Message>>,
    tabs: Vec<Tab<'a, Message>>,
    trailing: Option<Element<'a, Message>>,
    panel: Panel<'a, Message>,
    /// Hızlı erişim düğmeleri ve menüsü.
    quick: Vec<Element<'a, Message>>,
    quick_menu: Option<Box<dyn Fn() -> Menu<Message> + 'a>>,
    quick_tip: Option<Tip>,
    /// Daraltılmış mı ve daraltma düğmesinin mesajı.
    collapse: Option<(bool, Message)>,
    /// Şeridin düğmelerin almadığı bir yerinde sağ tık (sekme, panel adı, boş yer).
    context: Option<Box<dyn Fn() -> Menu<Message> + 'a>>,
    /// Harf ipuçları: sekmelerin ve hızlı erişim düğmelerinin, sırasıyla.
    key_tips: (Vec<Option<KeyTip>>, Vec<Option<KeyTip>>),
    /// Daraltılmış şeridin seçili sekmesi açık (çizimin üstünde gösterilir).
    peek: bool,
}

struct Tab<'a, Message> {
    label: Fragment<'a>,
    selected: bool,
    on_press: Message,
    /// Bağlamsal sekmenin sayısı (ör. seçili nesneler); varsa sekme bağlamsaldır.
    count: Option<String>,
}

enum Panel<'a, Message> {
    Groups(Vec<Group<'a, Message>>),
    Placeholder(Fragment<'a>),
}

impl<'a, Message: Clone + 'a> Ribbon<'a, Message> {
    pub fn new() -> Self {
        Self {
            application: None,
            tabs: Vec::new(),
            trailing: None,
            panel: Panel::Groups(Vec::new()),
            quick: Vec::new(),
            quick_menu: None,
            quick_tip: None,
            collapse: None,
            context: None,
            key_tips: (Vec::new(), Vec::new()),
            peek: false,
        }
    }

    /// Hızlı erişim düğmesi: uygulama düğmesinin yanında, sekmelerden önce
    /// duran küçük ikon (ör. kaydet, geri al). `on_press` yoksa devre
    /// dışıdır.
    pub fn quick(
        mut self,
        glyph: Icon,
        description: impl Into<String>,
        on_press: Option<Message>,
    ) -> Self {
        self.quick.push(tip(
            button(icon(glyph).size(14.0))
                .on_press_maybe(on_press)
                .padding([4, 5])
                .style(style::button::flat),
            Tip::new(description.into()),
            tooltip::Position::Bottom,
        ));
        self
    }

    /// Sağ tıklanınca menü açan hızlı erişim düğmesi (ör. çubuktan
    /// kaldırmak için); menü düğme kapalıyken de açılır.
    pub fn quick_with_menu(
        mut self,
        glyph: Icon,
        description: impl Into<String>,
        on_press: Option<Message>,
        menu: impl Fn() -> Menu<Message> + 'a,
    ) -> Self {
        let face = tip(
            button(icon(glyph).size(14.0))
                .on_press_maybe(on_press)
                .padding([4, 5])
                .style(style::button::flat),
            Tip::new(description.into()),
            tooltip::Position::Bottom,
        );
        self.quick
            .push(ContextMenu::new(face, move |_| menu()).into());
        self
    }

    /// Hızlı erişim düğmelerinin sonundaki ⌄ menüsü (ör. düğmeleri gösterip
    /// gizlemek için).
    pub fn quick_menu(mut self, menu: impl Fn() -> Menu<Message> + 'a) -> Self {
        self.quick_menu = Some(Box::new(menu));
        self
    }

    /// ⌄ menüsünün ipucu (ör. “Hızlı erişimi özelleştir”).
    pub fn quick_menu_tip(mut self, quick_tip: Tip) -> Self {
        self.quick_tip = Some(quick_tip);
        self
    }

    /// Harf ipuçları (web'in KeyTips'i): sekmelerin ve hızlı erişim
    /// düğmelerinin, eklendikleri sırayla; yoksa `None`.
    pub fn key_tips(mut self, tabs: Vec<Option<KeyTip>>, quick: Vec<Option<KeyTip>>) -> Self {
        self.key_tips = (tabs, quick);
        self
    }

    /// Şeridin düğmelerin almadığı bir yerinde (sekme, panel adı, boş yer)
    /// sağ tıklanınca açılan menü. Kendi menüsü olan düğme önce gelir.
    pub fn context_menu(mut self, menu: impl Fn() -> Menu<Message> + 'a) -> Self {
        self.context = Some(Box::new(menu));
        self
    }

    /// Daraltılmış şeridin seçili sekmesi açık: panel de çizilir (uygulama
    /// onu çizimin üstündeki katmanda gösterir, çizim yerinden oynamaz).
    pub fn peek(mut self, peek: bool) -> Self {
        self.peek = peek;
        self
    }

    /// Sekme şeridinin sağ ucunda şeridi daraltan düğme; daraltılmış
    /// şeritte yalnızca sekmeler görünür.
    pub fn collapsible(mut self, collapsed: bool, on_toggle: Message) -> Self {
        self.collapse = Some((collapsed, on_toggle));
        self
    }

    /// Sekme şeridinin başındaki uygulama (marka) düğmesi.
    pub fn application(mut self, application: AppButton<'a, Message>) -> Self {
        self.application = Some(application);
        self
    }

    /// Tek bir sekme ekler.
    pub fn tab(mut self, label: impl IntoFragment<'a>, selected: bool, on_press: Message) -> Self {
        self.tabs.push(Tab {
            label: label.into_fragment(),
            selected,
            on_press,
            count: None,
        });
        self
    }

    /// Bağlamsal sekme ekler (web'in `ribbon__tab--context`'i; ör. nesne
    /// seçiliyken Seçim): adı vurgu renginde, üstünde ince vurgu çizgisi ve
    /// yanında sayı rozeti (`count`).
    pub fn contextual_tab(
        mut self,
        label: impl IntoFragment<'a>,
        count: impl Into<String>,
        selected: bool,
        on_press: Message,
    ) -> Self {
        self.tabs.push(Tab {
            label: label.into_fragment(),
            selected,
            on_press,
            count: Some(count.into()),
        });
        self
    }

    /// Bir sekme listesi ekler; etiketler `Display` ile yazılır.
    pub fn tabs<T>(
        self,
        tabs: impl IntoIterator<Item = T>,
        selected: T,
        on_select: impl Fn(T) -> Message,
    ) -> Self
    where
        T: Copy + PartialEq + std::fmt::Display,
    {
        tabs.into_iter().fold(self, |ribbon, tab| {
            ribbon.tab(tab.to_string(), tab == selected, on_select(tab))
        })
    }

    /// Sekme şeridinin sağ ucundaki içerik (ör. açık belgenin adı).
    pub fn trailing(mut self, trailing: impl Into<Element<'a, Message>>) -> Self {
        self.trailing = Some(trailing.into());
        self
    }

    /// Seçili sekmenin paneline bir grup ekler.
    pub fn group(mut self, group: Group<'a, Message>) -> Self {
        match &mut self.panel {
            Panel::Groups(groups) => groups.push(group),
            Panel::Placeholder(_) => self.panel = Panel::Groups(vec![group]),
        }
        self
    }

    /// Seçili sekmenin panelini grupsuz, açıklama metniyle gösterir.
    pub fn placeholder(mut self, message: impl IntoFragment<'a>) -> Self {
        self.panel = Panel::Placeholder(message.into_fragment());
        self
    }

    /// The tab strip, from the ribbon whose panel and menu were taken out.
    fn strip(self) -> Element<'a, Message> {
        let Ribbon {
            application,
            tabs,
            trailing,
            quick,
            quick_menu,
            quick_tip,
            collapse,
            key_tips,
            peek,
            ..
        } = self;
        let (tab_tips, quick_tips) = key_tips;
        let quick: Vec<Element<'a, Message>> = quick
            .into_iter()
            .enumerate()
            .map(|(i, button)| {
                key_tip(
                    button,
                    quick_tips.get(i).cloned().flatten(),
                    Place::Icon(12.0),
                )
            })
            .collect();
        let mut strip = iced::widget::Row::new().height(tab_height() + 1.0);
        // Open over the drawing, the folded ribbon's selected tab flows into its panel.
        let collapsed = collapse.as_ref().is_some_and(|(collapsed, _)| *collapsed) && !peek;

        strip = match application {
            Some(application) => strip.push(underlined(
                row![application, space::horizontal().width(8)],
                Length::Shrink,
            )),
            None => strip.push(underlined(space::horizontal().width(EDGE), Length::Shrink)),
        };

        // Hızlı erişim düğmeleri, ardından sekmelerden ayıran çizgi.
        if !quick.is_empty() || quick_menu.is_some() {
            let mut buttons = iced::widget::Row::with_children(quick)
                .spacing(1)
                .align_y(iced::Center);

            if let Some(menu) = quick_menu {
                let more: Element<'a, Message> = MenuButton::new(
                    container(icon(Icon::ChevronDown).size(9.0)).padding([6, 4]),
                    menu,
                )
                .into();
                buttons = buttons.push(match quick_tip {
                    Some(quick_tip) => tip(more, quick_tip, tooltip::Position::Bottom),
                    None => more,
                });
            }

            strip = strip.push(underlined(
                container(
                    row![
                        buttons,
                        container(vertical_divider()).height(typography::scaled(16.0)),
                    ]
                    .spacing(6)
                    .align_y(iced::Center),
                )
                .height(tab_height())
                .padding(Padding {
                    right: 8.0,
                    ..Padding::ZERO
                })
                .align_y(iced::Center),
                Length::Shrink,
            ));
        }

        for (i, tab) in tabs.into_iter().enumerate() {
            let tab_tip = tab_tips.get(i).cloned().flatten();
            let contextual = tab.count.is_some();
            let face = tab_face(tab.label, tab.selected, tab.count);
            // Daraltılmış şeritte seçili sekme panele akmaz; kalın yazıyla
            // ayrılır.
            let tab_button = if tab.selected && !collapsed {
                selected_tab(face)
            } else {
                underlined(
                    button(container(face).height(Fill).align_y(iced::Center))
                        .on_press(tab.on_press)
                        .height(tab_height())
                        .padding([0, 14])
                        .style(style::button::tab),
                    Length::Shrink,
                )
            };
            // The tab's key tip under its middle (the web's KeyTips).
            let tab_button = key_tip(tab_button, tab_tip, Place::Under);
            // A contextual tab stands a little apart, a thin accent line over it (web).
            strip = strip.push(if contextual {
                let line = !tab.selected || collapsed;
                underlined(
                    row![
                        space::horizontal().width(4),
                        iced::widget::stack![
                            tab_button,
                            container(space::horizontal())
                                .width(Fill)
                                .height(if line { TAB_ACCENT } else { 0.0 })
                                .style(style::container::accent),
                        ]
                    ],
                    Length::Shrink,
                )
            } else {
                tab_button
            });
        }

        let trailing = container(trailing.unwrap_or_else(|| space::horizontal().into()))
            .width(Fill)
            .height(tab_height())
            .padding([0.0, EDGE])
            .align_x(Right)
            .align_y(iced::Center);

        strip = strip.push(underlined(trailing, Fill));

        if let Some((collapsed, on_toggle)) = collapse {
            let (glyph, description) = if collapsed {
                (Icon::ChevronDown, "Şeridi göster")
            } else {
                (Icon::ChevronUp, "Şeridi daralt")
            };

            strip = strip.push(underlined(
                container(tip(
                    button(icon(glyph).size(11.0))
                        .on_press(on_toggle)
                        .padding([4, 6])
                        .style(style::button::flat),
                    Tip::new(description).detail("Ctrl+F1"),
                    tooltip::Position::Left,
                ))
                .height(tab_height())
                .padding(Padding {
                    right: 6.0,
                    ..Padding::ZERO
                })
                .align_y(iced::Center),
                Length::Shrink,
            ));
        }

        container(column![space::vertical().height(STRIP_TOP), strip])
            .width(Fill)
            .style(style::container::window)
            .into()
    }

    fn panel(panel: Panel<'a, Message>) -> Element<'a, Message> {
        let content: Element<'a, Message> = match panel {
            // Pencereye göre küçülen gruplar: seviyeler yerleşimde, o anki genişlikle seçilir.
            Panel::Groups(groups) if groups.iter().all(Group::adaptive) => {
                responsive(move |size| fitted(&groups, size.width))
                    .width(Fill)
                    .height(Fill)
                    .into()
            }
            Panel::Groups(groups) => {
                let mut groups_row = iced::widget::Row::new().height(Fill);

                for group in groups {
                    groups_row = groups_row.push(group).push(vertical_divider());
                }

                // Büyük yazı boyutunda ya da dar pencerede gruplar sığmazsa
                // şerit yatay kaydırılır; ince çubuk yalnızca o zaman görünür.
                scrollable(groups_row)
                    .direction(scrollable::Direction::Horizontal(
                        scrollable::Scrollbar::new().width(3).scroller_width(3),
                    ))
                    .width(Fill)
                    .height(Fill)
                    .into()
            }
            Panel::Placeholder(message) => container(label::muted(message))
                .padding([0, 16])
                .height(Fill)
                .align_y(iced::Center)
                .into(),
        };

        container(content)
            .width(Fill)
            .height(panel_height())
            .style(style::container::surface)
            .into()
    }
}

impl<'a, Message: Clone + 'a> Default for Ribbon<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: Clone + 'a> From<Ribbon<'a, Message>> for Element<'a, Message> {
    fn from(mut ribbon: Ribbon<'a, Message>) -> Self {
        let panel = std::mem::replace(&mut ribbon.panel, Panel::Groups(Vec::new()));
        let context = ribbon.context.take();
        let collapsed = ribbon
            .collapse
            .as_ref()
            .is_some_and(|(collapsed, _)| *collapsed);
        // Open over the drawing, the folded ribbon's tab flows into its panel.
        let shows_panel = !collapsed || ribbon.peek;
        let strip = ribbon.strip();

        let whole: Element<'a, Message> = if !shows_panel {
            strip
        } else {
            Column::new()
                .push(strip)
                .push(Ribbon::panel(panel))
                .push(horizontal_divider())
                .into()
        };
        match context {
            Some(menu) => ContextMenu::new(whole, move |_| menu()).into(),
            None => whole,
        }
    }
}

/// Grupları genişliğe sığdırıp çizer; en küçük hâli de sığmıyorsa kaydırır.
fn fitted<'a, Message: Clone + 'a>(
    groups: &[Group<'a, Message>],
    available: f32,
) -> Element<'a, Message> {
    let widths: Vec<[f32; 4]> = groups.iter().map(Group::widths).collect();
    let keep: Vec<bool> = groups.iter().map(Group::keeps).collect();
    let (levels, overflow) = fit(&widths, &keep, available);
    let row = groups.iter().zip(levels).fold(
        iced::widget::Row::new().height(Fill),
        |row, (group, level)| row.push(group.view(level)).push(vertical_divider()),
    );
    if overflow {
        scrollable(row)
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new().width(3).scroller_width(3),
            ))
            .width(Fill)
            .height(Fill)
            .into()
    } else {
        row.into()
    }
}

/// Her panelin seviyesini genişliğe göre seçer (web'in `Ribbon.fit`'i):
/// her panel bir adım inmeden hiçbiri iki adım inmez, önce en sağdaki.
/// `keep` paneller öbürleri yalnız ikona inene kadar büyük düğmelerini
/// korur; yalnız tek düğmeye katlanmak onlardan önce gelir. Genişlik
/// kazandırmayan adım atlanır. `widths[i][l]`: panelin `l` seviyesindeki
/// genişliği. Sonuç seviyeler ve en küçük hâlin de sığmadığı.
pub fn fit(widths: &[[f32; 4]], keep: &[bool], available: f32) -> (Vec<u8>, bool) {
    let mut levels = vec![0u8; widths.len()];
    let mut total: f32 = widths.iter().map(|w| w[0]).sum();
    let next = |levels: &[u8], i: usize| -> Option<u8> {
        let here = widths[i][usize::from(levels[i])];
        (levels[i] + 1..=3).find(|&l| widths[i][usize::from(l)] < here - 0.5)
    };
    let kept = |i: usize| keep.get(i).copied().unwrap_or(false);
    let rank = |i: usize, l: u8| f32::from(l) + if kept(i) && l < 3 { 1.5 } else { 0.0 };
    // Tutulmayanlar önce, sağdaki önce.
    let mut order: Vec<usize> = (0..widths.len()).collect();
    order.sort_by_key(|&i| (kept(i), std::cmp::Reverse(i)));
    while total > available {
        let mut pick: Option<(usize, u8)> = None;
        for &i in &order {
            if let Some(l) = next(&levels, i)
                && pick.is_none_or(|(p, to)| rank(i, l) < rank(p, to))
            {
                pick = Some((i, l));
            }
        }
        let Some((i, l)) = pick else { break };
        total -= widths[i][usize::from(levels[i])] - widths[i][usize::from(l)];
        levels[i] = l;
    }
    (levels, total > available + 0.5)
}

/// Sekme şeridindeki bir öğe; altında şerit boyunca uzanan çizgiyle.
fn underlined<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    width: impl Into<Length>,
) -> Element<'a, Message> {
    column![content.into(), horizontal_divider()]
        .width(width)
        .into()
}

/// Seçili sekme: üstte vurgu çizgisi, yanlarda kenar çizgisi; altta çizgi
/// yoktur ve gövdesi panelle aynı renktedir.
/// Sekmenin yazısı: seçiliyse kalın; bağlamsal sekmede vurgu renginde ve
/// yanında sayı rozeti.
fn tab_face<'a, Message: 'a>(
    title: Fragment<'a>,
    selected: bool,
    count: Option<String>,
) -> Element<'a, Message> {
    let Some(count) = count else {
        return if selected {
            label::strong(title).into()
        } else {
            label::body(title).into()
        };
    };
    let accent_text = |theme: &iced::Theme| iced::widget::text::Style {
        color: Some(crate::theme::Tokens::of(theme).accent_hover),
    };
    let name = if selected {
        label::strong(title)
    } else {
        label::body(title)
    };
    row![
        name.style(accent_text),
        container(
            iced::widget::text(count)
                .font(typography::mono())
                .size(typography::caption() - 1.0)
                .style(accent_text),
        )
        .padding([0.0, 5.0])
        .height(typography::scaled(16.0))
        .center_y(typography::scaled(16.0))
        .style(style::container::count),
    ]
    .spacing(6)
    .align_y(iced::Center)
    .into()
}

fn selected_tab<'a, Message: 'a>(face: Element<'a, Message>) -> Element<'a, Message> {
    row![
        vertical_divider(),
        column![
            container(space::horizontal())
                .width(Fill)
                .height(TAB_ACCENT)
                .style(style::container::accent),
            // Alt boşluk vurgu çizgisini dengeler; yazı diğer sekmelerle aynı
            // taban çizgisinde kalır.
            container(face)
                .height(Fill)
                .padding(Padding {
                    top: 0.0,
                    right: 14.0,
                    bottom: TAB_ACCENT + 1.0,
                    left: 14.0,
                })
                .align_y(iced::Center)
                .style(style::container::surface),
        ]
        .width(Length::Shrink),
        vertical_divider(),
    ]
    .height(tab_height() + 1.0)
    .into()
}

/// Şeritte yeri gösterilen öğe (ör. komut aramasının "Şeritte göster"i):
/// üstünde kısa süre duran vurgu çerçevesi (web'in `[data-flash]`'i, 2 px
/// vurgu rengi). Çerçeve öğenin yerini değiştirmez.
pub(crate) fn flashed<'a, Message: 'a>(
    content: Element<'a, Message>,
    on: bool,
) -> Element<'a, Message> {
    if !on {
        return content;
    }
    iced::widget::stack![
        content,
        container(space::horizontal())
            .width(iced::Fill)
            .height(iced::Fill)
            .style(|theme: &iced::Theme| container::Style {
                border: iced::Border {
                    color: crate::theme::Tokens::of(theme).accent,
                    width: 2.0,
                    radius: crate::style::button::RADIUS.into(),
                },
                ..container::Style::default()
            }),
    ]
    .into()
}
