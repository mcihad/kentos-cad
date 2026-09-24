//! Şerit düğmeleri ve uygulama (marka) düğmesi.

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{button, column, container, row, space, text, tooltip};
use iced::{Center, Element, Fill, Padding, Right, Top};

use super::{CONTENT, ICON, LARGE_ICON, LARGE_WIDTH, ROW, TAB_HEIGHT};
use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::{Tokens, typography};
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
/// dışıdır ve ikonuyla birlikte sönük görünür.
pub struct Button<'a, Message> {
    icon: Icon,
    label: Fragment<'a>,
    size: Size,
    on_press: Option<Message>,
    active: bool,
    tip: Option<Tip>,
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
        }
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

        let content: Element<'a, Message> = match ribbon_button.size {
            Size::Large => button(
                column![
                    icon(ribbon_button.icon).size(LARGE_ICON).tone(tone),
                    text(ribbon_button.label)
                        .size(typography::CAPTION)
                        .line_height(1.15)
                        .align_x(Center),
                ]
                .spacing(5)
                .align_x(Center)
                .width(Fill),
            )
            .width(LARGE_WIDTH)
            .height(CONTENT)
            .padding(Padding {
                top: 9.0,
                right: 2.0,
                bottom: 2.0,
                left: 2.0,
            }),
            Size::Small => button(
                row![
                    icon(ribbon_button.icon).size(ICON).tone(tone),
                    label::body(ribbon_button.label),
                ]
                .spacing(6)
                .height(Fill)
                .align_y(Center),
            )
            .height(ROW)
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
        .height(TAB_HEIGHT)
        .padding([0, 12])
        .style(style::button::brand(app_button.open))
        .into()
    }
}

/// KentOS marka işareti: köşesinde dolu kare olan çerçeve.
fn logo_mark<'a, Message: 'a>() -> Element<'a, Message> {
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
