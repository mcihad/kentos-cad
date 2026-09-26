//! Şerit düğmeleri ve uygulama (marka) düğmesi.

use std::rc::Rc;

use iced::widget::text::{Fragment, IntoFragment, Wrapping};
use iced::widget::{button, column, container, row, space, text, tooltip};
use iced::{Center, Element, Fill, Padding, Right, Top};

use super::{ICON, LARGE_ICON, LARGE_WEIGHT, content_height, row_height, tab_height};
use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::style::button::Ribbon as State;
use crate::theme::{Tokens, typography};
use crate::widget::context_menu::{Menu, MenuButton};
use crate::widget::{Tip, tip};

/// Şerit düğmesinin tasarlandığı boyut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Size {
    /// Üç satır yüksekliğinde; ikon üstte, iki satıra kadar etiket altta.
    Large,
    /// Tek satır yüksekliğinde; ikon ve etiket yan yana.
    Small,
}

/// Düğmenin çizildiği biçim. Panel daraldıkça düğmeler bir adım iner:
/// büyük → küçük etiketli → yalnız ikon (DESIGN.md §7.3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Form {
    Large,
    Small,
    Icon,
}

// Ölçüler (12 piksellik gövde metninde; metni taşıyanlar yazıyla büyür).
/// Küçük düğmenin sol ve sağ iç boşluğu.
const SMALL_LEFT: f32 = 4.0;
const SMALL_RIGHT: f32 = 7.0;
/// İkon ile etiket arası.
const GAP: f32 = 6.0;
/// Açılır ok ve onun parçasının iç boşluğu.
const CHEVRON: f32 = 9.0;
const ARROW_PAD: f32 = 3.0;
/// Yalnız ikonlu düğmenin genişliği.
const ICON_ONLY: f32 = 26.0;
/// Büyük düğmenin en dar ve en geniş hâli.
const LARGE_MIN: f32 = 50.0;
const LARGE_MAX: f32 = 100.0;
/// Büyük düğmede etiketin iki yanındaki pay.
const LARGE_PAD: f32 = 5.0;

/// Şerit düğmesi.
///
/// Çalışan araç ([`active`](Button::active)) dolu vurgu, açık anahtar
/// ([`on`](Button::on)) yumuşak vurgu zeminiyle gösterilir. `on_press`
/// verilmeyen düğme devre dışıdır ve sönük görünür. [`menu`](Button::menu)
/// ile menülü ya da bölünmüş düğme olur. Şerit düğmeyi panelin genişliğine
/// göre büyük, küçük ya da yalnız ikon olarak çizer.
pub struct Button<'a, Message> {
    icon: Icon,
    label: Fragment<'a>,
    size: Size,
    on_press: Option<Message>,
    state: State,
    tip: Option<Tip>,
    menu: Option<Rc<dyn Fn() -> Menu<Message> + 'a>>,
}

impl<'a, Message: Clone + 'a> Clone for Button<'a, Message> {
    fn clone(&self) -> Self {
        Self {
            icon: self.icon,
            label: self.label.clone(),
            size: self.size,
            on_press: self.on_press.clone(),
            state: self.state,
            tip: self.tip.clone(),
            menu: self.menu.clone(),
        }
    }
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
            state: State::Idle,
            tip: None,
            menu: None,
        }
    }

    /// Düğmeye menü ekler. `on_press` da verilmişse düğme bölünür: ikon
    /// (büyük düğmede üst kısım, küçükte etiketle birlikte sol kısım) eylemi
    /// yapar, ok menüyü açar. Verilmemişse düğmenin tamamı menüyü açar.
    pub fn menu(mut self, menu: impl Fn() -> Menu<Message> + 'a) -> Self {
        self.menu = Some(Rc::new(menu));
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

    /// Çalışan araç: dolu vurgu.
    pub fn active(mut self, active: bool) -> Self {
        if active {
            self.state = State::Running;
        } else if self.state == State::Running {
            self.state = State::Idle;
        }
        self
    }

    /// Açık anahtar (ör. kenet, panel): yumuşak vurgu zemini.
    pub fn on(mut self, on: bool) -> Self {
        if self.state != State::Running {
            self.state = if on { State::On } else { State::Idle };
        }
        self
    }

    pub fn tip(mut self, tip: Tip) -> Self {
        self.tip = Some(tip);
        self
    }

    /// Tasarlandığı boyut büyük mü.
    pub(crate) fn is_large(&self) -> bool {
        self.size == Size::Large
    }

    /// Katlanmış panelin menüsündeki satırı: menülü düğme alt menü, öbürü komut.
    pub(crate) fn menu_entry(&self, menu: Menu<Message>) -> Menu<Message> {
        let label = self.label.to_string().replace('\n', " ");
        match &self.menu {
            Some(submenu) => menu.submenu(label, submenu()).icon(self.icon),
            None => menu.item(label, self.on_press.clone()).icon(self.icon),
        }
    }

    /// Etiketin en uzun satırının genişliği (açıklama boyutunda).
    fn label_width(&self) -> f32 {
        self.label
            .lines()
            .map(|line| typography::text_width(line, typography::caption()))
            .fold(0.0, f32::max)
    }

    /// Biçimdeki genişliği (piksel): şerit panelleri buna göre sığdırır.
    pub(crate) fn width(&self, form: Form) -> f32 {
        let s = typography::scaled;
        let split = self.menu.is_some() && self.on_press.is_some();
        let menu_only = self.menu.is_some() && self.on_press.is_none();
        let arrow = CHEVRON + ARROW_PAD * 2.0;
        match form {
            Form::Large => large_width(self.label_width()),
            Form::Small => {
                let face = SMALL_LEFT + ICON + GAP + self.label_width();
                if split {
                    face + 5.0 + arrow
                } else if menu_only {
                    face + GAP + CHEVRON + SMALL_RIGHT
                } else {
                    face + SMALL_RIGHT
                }
            }
            Form::Icon => {
                if split {
                    s(ICON_ONLY - 2.0) + arrow
                } else if menu_only {
                    SMALL_LEFT + ICON + 2.0 + CHEVRON + SMALL_LEFT
                } else {
                    s(ICON_ONLY)
                }
            }
        }
    }

    /// Düğmeyi verilen biçimde çizer.
    pub(crate) fn render(&self, form: Form) -> Element<'a, Message> {
        let content = match (&self.menu, &self.on_press) {
            (Some(menu), Some(message)) => self.split(form, message.clone(), menu.clone()),
            (Some(menu), None) => self.dropdown(form, menu.clone()),
            (None, _) => self.plain(form),
        };

        match &self.tip {
            Some(button_tip) => tip(content, button_tip.clone(), tooltip::Position::Bottom),
            None => content,
        }
    }

    fn glyph(&self, form: Form) -> Element<'a, Message> {
        match form {
            Form::Large => icon(self.icon).size(LARGE_ICON).weight(LARGE_WEIGHT).into(),
            Form::Small | Form::Icon => icon(self.icon).size(ICON).into(),
        }
    }

    /// Etiket: kendi rengiyle (ikon düğmenin rengini alır, etiket almaz).
    fn caption(&self, large: bool) -> iced::widget::Text<'a> {
        self.caption_in(large, self.state)
    }

    /// Etiket, `state` durumundaki bir zeminin üstünde.
    fn caption_in(&self, large: bool, state: State) -> iced::widget::Text<'a> {
        let enabled = self.on_press.is_some() || self.menu.is_some();
        let label = text(self.label.clone())
            .font(typography::ui())
            .size(typography::caption())
            .style(move |theme: &iced::Theme| iced::widget::text::Style {
                color: Some(style::button::ribbon_label(theme, state, enabled)),
            });
        if large {
            label
                .line_height(1.18)
                .wrapping(Wrapping::Word)
                .align_x(Center)
        } else {
            label.wrapping(Wrapping::None)
        }
    }

    fn plain(&self, form: Form) -> Element<'a, Message> {
        let width = self.width(form);
        let face = match form {
            Form::Large => button(
                column![self.glyph(form), self.caption(true)]
                    .spacing(typography::scaled(4.0))
                    .align_x(Center)
                    .width(Fill),
            )
            .width(width)
            .height(content_height())
            .padding(Padding {
                top: typography::scaled(6.0),
                right: LARGE_PAD,
                bottom: 2.0,
                left: LARGE_PAD,
            }),
            Form::Small => button(
                row![self.glyph(form), self.caption(false)]
                    .spacing(GAP)
                    .height(Fill)
                    .align_y(Center),
            )
            .height(row_height())
            .padding(Padding {
                left: SMALL_LEFT,
                right: SMALL_RIGHT,
                ..Padding::ZERO
            }),
            Form::Icon => button(container(self.glyph(form)).center(Fill))
                .width(width)
                .height(row_height())
                .padding(0),
        };
        face.on_press_maybe(self.on_press.clone())
            .style(style::button::ribbon(self.state))
            .into()
    }

    /// Bölünmüş düğme: eylem ve menü ayrı ayrı aydınlanır.
    fn split(
        &self,
        form: Form,
        message: Message,
        menu: Rc<dyn Fn() -> Menu<Message> + 'a>,
    ) -> Element<'a, Message> {
        let chevron = || icon(Icon::ChevronDown).size(CHEVRON).tone(Tone::Muted);
        let open = move || menu();
        match form {
            Form::Large => {
                let width = self.width(form);
                let top = (content_height() * 0.54).round();
                column![
                    button(container(self.glyph(form)).center_x(Fill).padding(Padding {
                        top: typography::scaled(6.0),
                        ..Padding::ZERO
                    }))
                    .on_press(message)
                    .width(width)
                    .height(top)
                    .padding(0)
                    .style(style::button::ribbon(self.state)),
                    // The label stands under the lit part, on the ribbon's own
                    // ground: its colour stays the idle one while the tool runs,
                    // as on the web (`.rsplit[data-active]` fills `.rsplit__main` only).
                    MenuButton::new(
                        container(
                            column![self.caption_in(true, State::Idle), chevron()]
                                .spacing(1)
                                .align_x(Center)
                        )
                        .center_x(width)
                        .height(content_height() - top)
                        .padding(Padding {
                            top: 1.0,
                            ..Padding::ZERO
                        }),
                        open,
                    ),
                ]
                .width(width)
                .into()
            }
            Form::Small | Form::Icon => {
                let main: Element<'a, Message> = if form == Form::Small {
                    row![self.glyph(form), self.caption(false)]
                        .spacing(GAP)
                        .height(Fill)
                        .align_y(Center)
                        .into()
                } else {
                    container(self.glyph(form)).center(Fill).into()
                };
                let main = button(main)
                    .on_press(message)
                    .height(row_height())
                    .padding(Padding {
                        left: if form == Form::Small { SMALL_LEFT } else { 0.0 },
                        right: if form == Form::Small { 5.0 } else { 0.0 },
                        ..Padding::ZERO
                    })
                    .style(style::button::ribbon(self.state));
                let main: Element<'a, Message> = if form == Form::Icon {
                    main.width(typography::scaled(ICON_ONLY - 2.0)).into()
                } else {
                    main.into()
                };
                row![
                    main,
                    MenuButton::new(
                        container(chevron())
                            .center_y(row_height())
                            .padding([0.0, ARROW_PAD]),
                        open,
                    ),
                ]
                .into()
            }
        }
    }

    /// Yalnız menü açan düğme.
    fn dropdown(
        &self,
        form: Form,
        menu: Rc<dyn Fn() -> Menu<Message> + 'a>,
    ) -> Element<'a, Message> {
        let chevron = || icon(Icon::ChevronDown).size(CHEVRON).tone(Tone::Muted);
        let open = move || menu();
        let content: Element<'a, Message> = match form {
            Form::Large => container(
                column![self.glyph(form), self.caption(true), chevron()]
                    .spacing(typography::scaled(3.0))
                    .align_x(Center),
            )
            .center_x(self.width(form))
            .height(content_height())
            .padding(Padding {
                top: typography::scaled(6.0),
                ..Padding::ZERO
            })
            .into(),
            Form::Small => container(
                row![self.glyph(form), self.caption(false), chevron()]
                    .spacing(GAP)
                    .align_y(Center),
            )
            .center_y(row_height())
            .padding(Padding {
                left: SMALL_LEFT,
                right: SMALL_RIGHT,
                ..Padding::ZERO
            })
            .into(),
            Form::Icon => container(row![self.glyph(form), chevron()].spacing(2).align_y(Center))
                .center_y(row_height())
                .padding([0.0, SMALL_LEFT])
                .into(),
        };
        MenuButton::new(content, open).into()
    }
}

/// Büyük düğmenin genişliği: etiketin en uzun satırına göre, en dar ve en
/// geniş hâl arasında.
fn large_width(label: f32) -> f32 {
    let s = typography::scaled;
    (label + LARGE_PAD * 2.0)
        .ceil()
        .clamp(s(LARGE_MIN), s(LARGE_MAX))
}

impl<'a, Message: Clone + 'a> From<Button<'a, Message>> for Element<'a, Message> {
    fn from(ribbon_button: Button<'a, Message>) -> Self {
        let form = match ribbon_button.size {
            Size::Large => Form::Large,
            Size::Small => Form::Small,
        };
        ribbon_button.render(form)
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
