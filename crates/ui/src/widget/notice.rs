//! Hazır geri bildirim kalıpları: uyarı şeridi ve boş ya da hata durumu.
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │ ⚠ Örnek veri salt okunur: yalnızca Çizimler düzenlenebilir. [Geç] × │ ← Banner
//! └──────────────────────────────────────────────────────────────────┘
//!
//!                              (◎)
//!                    Haritada görünür katman yok          ← EmptyState
//!         Katmanlar gizlendi; ağaçtan ya da buradan gösterin.
//!                         [Tümünü göster]
//! ```
//!
//! - [`Banner`] bir alanın üstünde kalıcı bir durumu anlatır: olay değil,
//!   süren bir hâl (salt okunur veri, bağlantı yok). Bildirimden farkı,
//!   kapatılana ya da durum değişene kadar yerinde kalmasıdır.
//! - [`EmptyState`] içeriği olmayan bir alanın ortasında ne olduğunu ve ne
//!   yapılabileceğini söyler; hata durumu ([`EmptyState::error`]) neyin
//!   yapılamadığını ve nasıl düzeltileceğini.

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{Row, button, column, container, row, rule};
use iced::{Background, Center, Element, Fill, Theme, border};

use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::{Tokens, typography};
use crate::widget::Severity;

/// Uyarı şeridi: alanın üstünde, önem renginde tam genişlikte şerit.
pub struct Banner<'a, Message> {
    severity: Severity,
    message: Fragment<'a>,
    actions: Vec<(Fragment<'a>, Message)>,
    on_dismiss: Option<Message>,
}

impl<'a, Message: Clone + 'a> Banner<'a, Message> {
    pub fn new(severity: Severity, message: impl IntoFragment<'a>) -> Self {
        Self {
            severity,
            message: message.into_fragment(),
            actions: Vec::new(),
            on_dismiss: None,
        }
    }

    pub fn info(message: impl IntoFragment<'a>) -> Self {
        Self::new(Severity::Info, message)
    }

    pub fn warning(message: impl IntoFragment<'a>) -> Self {
        Self::new(Severity::Warning, message)
    }

    pub fn error(message: impl IntoFragment<'a>) -> Self {
        Self::new(Severity::Error, message)
    }

    /// İletinin sağındaki eylem düğmesi (ör. "Yeniden bağlan").
    pub fn action(mut self, label: impl IntoFragment<'a>, message: Message) -> Self {
        self.actions.push((label.into_fragment(), message));
        self
    }

    /// Sağ uçtaki kapatma düğmesi.
    pub fn on_dismiss(mut self, message: Message) -> Self {
        self.on_dismiss = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<Banner<'a, Message>> for Element<'a, Message> {
    fn from(banner: Banner<'a, Message>) -> Self {
        let severity = banner.severity;

        let mut line = row![
            icon(severity.icon()).size(14.0).tone(severity.tone()),
            label::body(banner.message).width(Fill),
        ]
        .spacing(10)
        .align_y(Center);

        for (text, message) in banner.actions {
            line = line.push(
                button(label::body(text))
                    .on_press(message)
                    .padding([1, 8])
                    .style(style::button::keyword),
            );
        }

        if let Some(message) = banner.on_dismiss {
            line = line.push(
                button(icon(Icon::Close).size(12.0))
                    .on_press(message)
                    .padding(4)
                    .style(style::button::ghost),
            );
        }

        column![
            container(line)
                .padding([5, 12])
                .width(Fill)
                .style(move |theme: &Theme| {
                    let t = Tokens::of(theme);

                    container::Style {
                        text_color: Some(t.text),
                        background: Some(Background::Color(
                            severity
                                .color(&t)
                                .scale_alpha(if t.is_dark { 0.14 } else { 0.12 }),
                        )),
                        ..container::Style::default()
                    }
                }),
            rule::horizontal(1).style(move |theme: &Theme| {
                let t = Tokens::of(theme);

                rule::Style {
                    color: severity.color(&t).scale_alpha(0.45),
                    radius: 0.0.into(),
                    fill_mode: rule::FillMode::Full,
                    snap: true,
                }
            }),
        ]
        .into()
    }
}

/// Boş ya da hata durumu: ikon, başlık, açıklama ve eylemler, alanın
/// ortasında.
pub struct EmptyState<'a, Message> {
    icon: Icon,
    tone: Tone,
    badge: Option<Severity>,
    title: Fragment<'a>,
    description: Option<Fragment<'a>>,
    actions: Vec<(Fragment<'a>, Message, bool)>,
}

impl<'a, Message: Clone + 'a> EmptyState<'a, Message> {
    /// Boş alan: sönük ikon ve ne yapılabileceği.
    pub fn new(icon: Icon, title: impl IntoFragment<'a>) -> Self {
        Self {
            icon,
            tone: Tone::Muted,
            badge: None,
            title: title.into_fragment(),
            description: None,
            actions: Vec::new(),
        }
    }

    /// Hata durumu: neyin yapılamadığı; açıklama nedenini ve nasıl
    /// düzeltileceğini söyler.
    pub fn error(title: impl IntoFragment<'a>) -> Self {
        Self {
            tone: Tone::Danger,
            badge: Some(Severity::Error),
            ..Self::new(Icon::Error, title)
        }
    }

    pub fn description(mut self, description: impl IntoFragment<'a>) -> Self {
        self.description = Some(description.into_fragment());
        self
    }

    /// Birincil eylem (ör. "Tümünü göster", "Yeniden dene").
    pub fn primary(mut self, label: impl IntoFragment<'a>, message: Message) -> Self {
        self.actions.push((label.into_fragment(), message, true));
        self
    }

    /// İkincil eylem.
    pub fn secondary(mut self, label: impl IntoFragment<'a>, message: Message) -> Self {
        self.actions.push((label.into_fragment(), message, false));
        self
    }
}

impl<'a, Message: Clone + 'a> From<EmptyState<'a, Message>> for Element<'a, Message> {
    fn from(state: EmptyState<'a, Message>) -> Self {
        let badge = state.badge;

        let symbol = container(icon(state.icon).size(20.0).tone(state.tone))
            .center_x(44)
            .center_y(44)
            .style(move |theme: &Theme| {
                let t = Tokens::of(theme);

                container::Style {
                    background: Some(Background::Color(match badge {
                        Some(severity) => severity.color(&t).scale_alpha(0.14),
                        None => t.layer(0.06),
                    })),
                    border: border::rounded(22.0),
                    ..container::Style::default()
                }
            });

        let mut content = column![symbol, label::heading(state.title).align_x(Center)]
            .spacing(10)
            .align_x(Center)
            .max_width(typography::scaled(380.0));

        if let Some(description) = state.description {
            content = content.push(label::muted(description).align_x(Center).width(Fill));
        }

        if !state.actions.is_empty() {
            let actions = state.actions.into_iter().map(|(text, message, primary)| {
                button(label::body(text))
                    .on_press(message)
                    .padding([4, 14])
                    .style(if primary {
                        style::button::primary
                    } else {
                        style::button::secondary
                    })
                    .into()
            });

            content = content.push(
                container(Row::with_children(actions).spacing(8)).padding(iced::padding::top(4)),
            );
        }

        container(content)
            .padding(24)
            .center_x(Fill)
            .center_y(Fill)
            .into()
    }
}
