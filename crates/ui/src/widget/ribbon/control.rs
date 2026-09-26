//! Şerit düğmeleri ve uygulama (marka) düğmesi.

use iced::widget::text::{Fragment, IntoFragment, Wrapping};
use iced::widget::{button, column, container, row, space, text, tooltip};
use iced::{Center, Element, Fill, Padding, Right, Top};

use super::{ICON, LARGE_ICON, content_height, large_width, row_height, tab_height};
use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::{Tokens, typography};
use crate::widget::context_menu::{Menu, MenuButton};
use crate::widget::{Tip, tip};

/// Şerit düğmesinin boyutu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Size {
    /// Üç satır yüksekliğinde; ikon üstte, iki satıra kadar etiket altta.
    Large,
    /// Tek satır yüksekliğinde; ikon ve etiket yan yana.
    Small,
}

/// Şerit düğmesi.
///
/// Etkin (`active`) düğme vurgu zemini ve çerçevesiyle gösterilir; araç
/// seçimi gibi kalıcı durumları anlatır. `on_press` verilmeyen düğme devre
/// dışıdır ve ikonuyla birlikte sönük görünür. [`menu`](Button::menu) ile
/// menülü ya da bölünmüş düğme olur.
pub struct Button<'a, Message> {
    icon: Icon,
    label: Fragment<'a>,
    size: Size,
    on_press: Option<Message>,
    active: bool,
    tip: Option<Tip>,
    menu: Option<Box<dyn Fn() -> Menu<Message> + 'a>>,
}

impl<'a, Message: Clone + 'a> Button<'a, Message> {
    /// Büyük düğme. Etiket `\n` ile iki satıra bölünebilir.
    pub fn large(icon: Icon, label: impl IntoFragment<'a>) -> Self {
        Self::new(icon, label, Size::Large)
    }

    /// Küçük düğme.
    pub fn small(icon: Icon, label: impl IntoFragment<'a>) -> Self {
        Self::new(icon, label, Size::Small)
    }

    fn new(icon: Icon, label: impl IntoFragment<'a>, size: Size) -> Self {
        Self {
            icon,
            label: label.into_fragment(),
            size,
            on_press: None,
            active: false,
            tip: None,
            menu: None,
        }
    }

    /// Düğmeye menü ekler. `on_press` da verilmişse düğme bölünür: ikon
    /// (büyük düğmede üst kısım, küçükte etiketle birlikte sol kısım) eylemi
    /// yapar, ok menüyü açar. Verilmemişse düğmenin tamamı menüyü açar.
    pub fn menu(mut self, menu: impl Fn() -> Menu<Message> + 'a) -> Self {
        self.menu = Some(Box::new(menu));
        self
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    pub fn on_press_maybe(mut self, message: Option<Message>) -> Self {
        self.on_press = message;
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn tip(mut self, tip: Tip) -> Self {
        self.tip = Some(tip);
        self
    }
}

impl<'a, Message: Clone + 'a> From<Button<'a, Message>> for Element<'a, Message> {
    fn from(ribbon_button: Button<'a, Message>) -> Self {
        let tone = if ribbon_button.active {
            Tone::Highlight
        } else {
            Tone::Inherit
        };

        // Büyük düğme en az standart genişliktedir; etiketin en uzun satırı
        // sığmazsa genişler. Etiketler kırılmaz: satırlar etikette verilir.
        let widest_line = ribbon_button
            .label
            .lines()
            .map(|line| typography::text_width(line, typography::caption()))
            .fold(0.0, f32::max);
        let large = large_width().max((widest_line + 8.0).ceil());

        if let Some(menu) = ribbon_button.menu {
            let content = with_menu(
                ribbon_button.icon,
                ribbon_button.label,
                ribbon_button.size,
                ribbon_button.on_press,
                ribbon_button.active,
                large,
                menu,
            );

            return match ribbon_button.tip {
                Some(button_tip) => tip(content, button_tip, tooltip::Position::Bottom),
                None => content,
            };
        }

        let content: Element<'a, Message> = match ribbon_button.size {
            Size::Large => button(
                column![
                    icon(ribbon_button.icon).size(LARGE_ICON).tone(tone),
                    text(ribbon_button.label)
                        .font(typography::ui())
                        .size(typography::caption())
                        .line_height(1.15)
                        .wrapping(Wrapping::None)
                        .align_x(Center),
                ]
                .spacing(5)
                .align_x(Center)
                .width(Fill),
            )
            .width(large)
            .height(content_height())
            .padding(Padding {
                top: 9.0,
                right: 2.0,
                bottom: 2.0,
                left: 2.0,
            }),
            Size::Small => button(
                row![
                    icon(ribbon_button.icon).size(ICON).tone(tone),
                    label::body(ribbon_button.label).wrapping(Wrapping::None),
                ]
                .spacing(6)
                .height(Fill)
                .align_y(Center),
            )
            .height(row_height())
            .padding([0, 6]),
        }
        .on_press_maybe(ribbon_button.on_press)
        .style(style::button::tool(ribbon_button.active))
        .into();

        match ribbon_button.tip {
            Some(button_tip) => tip(content, button_tip, tooltip::Position::Bottom),
            None => content,
        }
    }
}

/// Menülü düğme: eylemi varsa bölünür (eylem ve menü ayrı ayrı vurgulanır),
/// yoksa tamamı menüyü açar.
fn with_menu<'a, Message: Clone + 'a>(
    glyph: Icon,
    title: Fragment<'a>,
    size: Size,
    on_press: Option<Message>,
    active: bool,
    width: f32,
    menu: Box<dyn Fn() -> Menu<Message> + 'a>,
) -> Element<'a, Message> {
    let tone = if active {
        Tone::Highlight
    } else {
        Tone::Inherit
    };
    let chevron = || icon(Icon::ChevronDown).size(9.0);
    let caption = |title: Fragment<'a>| {
        text(title)
            .font(typography::ui())
            .size(typography::caption())
            .line_height(1.15)
            .wrapping(Wrapping::None)
            .align_x(Center)
    };

    match (size, on_press) {
        (Size::Large, Some(message)) => {
            // Üstte eylem, altta etiket ve ok: ikisi ayrı düğmedir.
            let top = (content_height() * 0.52).round();

            column![
                button(
                    container(icon(glyph).size(LARGE_ICON).tone(tone))
                        .center_x(Fill)
                        .padding(Padding {
                            top: 7.0,
                            ..Padding::ZERO
                        })
                )
                .on_press(message)
                .width(width)
                .height(top)
                .padding(0)
                .style(style::button::tool(active)),
                MenuButton::new(
                    container(
                        column![caption(title), chevron()]
                            .spacing(1)
                            .align_x(Center)
                    )
                    .center_x(width)
                    .height(content_height() - top)
                    .padding(Padding {
                        top: 2.0,
                        ..Padding::ZERO
                    }),
                    menu,
                ),
            ]
            .width(width)
            .into()
        }
        (Size::Large, None) => MenuButton::new(
            container(
                column![
                    icon(glyph).size(LARGE_ICON).tone(tone),
                    caption(title),
                    chevron(),
                ]
                .spacing(3)
                .align_x(Center),
            )
            .center_x(width)
            .height(content_height())
            .padding(Padding {
                top: 7.0,
                ..Padding::ZERO
            }),
            menu,
        )
        .into(),
        (Size::Small, Some(message)) => row![
            button(
                row![
                    icon(glyph).size(ICON).tone(tone),
                    label::body(title).wrapping(Wrapping::None),
                ]
                .spacing(6)
                .height(Fill)
                .align_y(Center),
            )
            .on_press(message)
            .height(row_height())
            .padding([0, 6])
            .style(style::button::tool(active)),
            MenuButton::new(
                container(chevron()).center_y(row_height()).padding([0, 4]),
                menu,
            ),
        ]
        .into(),
        (Size::Small, None) => MenuButton::new(
            container(
                row![
                    icon(glyph).size(ICON).tone(tone),
                    label::body(title).wrapping(Wrapping::None),
                    chevron(),
                ]
                .spacing(6)
                .align_y(Center),
            )
            .center_y(row_height())
            .padding([0, 6]),
            menu,
        )
        .into(),
    }
}

/// Sekme şeridinin başındaki uygulama düğmesi (Office'teki "Dosya"
/// sekmesi gibi): marka işareti, uygulama adı ve açılır menü oku.
pub struct AppButton<'a, Message> {
    title: Fragment<'a>,
    open: bool,
    on_press: Option<Message>,
}

impl<'a, Message: Clone + 'a> AppButton<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>) -> Self {
        Self {
            title: title.into_fragment(),
            open: false,
            on_press: None,
        }
    }

    /// Menü açıkken düğme koyulaşır.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<AppButton<'a, Message>> for Element<'a, Message> {
    fn from(app_button: AppButton<'a, Message>) -> Self {
        button(
            row![
                logo_mark(),
                label::heading(app_button.title),
                icon(Icon::ChevronDown).size(10.0),
            ]
            .spacing(8)
            .height(Fill)
            .align_y(Center),
        )
        .on_press_maybe(app_button.on_press)
        .height(tab_height())
        .padding([0, 12])
        .style(style::button::brand(app_button.open))
        .into()
    }
}

/// KentOS marka işareti: köşesinde dolu kare olan çerçeve. Vurgu
/// zemininde durur (şeridin marka düğmesi, uygulama menüsünün başlığı).
pub fn logo_mark<'a, Message: 'a>() -> Element<'a, Message> {
    let on_accent = |theme: &iced::Theme| Tokens::of(theme).on_accent;

    container(
        container(space::horizontal())
            .width(4)
            .height(4)
            .style(move |theme| iced::widget::container::Style {
                background: Some(on_accent(theme).into()),
                ..Default::default()
            }),
    )
    .width(13)
    .height(13)
    .align_x(Right)
    .align_y(Top)
    .padding(2)
    .style(move |theme| style::container::outline(on_accent(theme), 1.5)(theme))
    .into()
}
