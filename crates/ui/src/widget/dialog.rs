//! İletişim kutusu, onay kutusu ve kısayol listesi.
//!
//! [`Dialog`] ve [`Confirm`] yalnızca kutunun kendisidir; ortalamak ve
//! arkasını karartmak için [`overlay::modal`](crate::widget::overlay::modal)
//! ile gösterilir.

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{Column, button, column, container, row, space};
use iced::{Background, Center, Element, Fill, Theme, border};

use crate::icon::icon;
use crate::label;
use crate::style;
use crate::theme::{Tokens, typography};
use crate::widget::{Severity, horizontal_divider};

/// Başlık, gövde ve sağa hizalı eylem düğmelerinden oluşan kutu.
pub struct Dialog<'a, Message> {
    title: Fragment<'a>,
    hint: Option<Fragment<'a>>,
    body: Vec<Element<'a, Message>>,
    actions: Vec<Element<'a, Message>>,
    width: f32,
    max_height: Option<f32>,
}

impl<'a, Message: 'a> Dialog<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>) -> Self {
        Self {
            title: title.into_fragment(),
            hint: None,
            body: Vec::new(),
            actions: Vec::new(),
            width: 500.0,
            max_height: None,
        }
    }

    /// Başlığın sağındaki kısa not (ör. kutuyu açan kısayol).
    pub fn hint(mut self, hint: impl IntoFragment<'a>) -> Self {
        self.hint = Some(hint.into_fragment());
        self
    }

    pub fn push(mut self, content: impl Into<Element<'a, Message>>) -> Self {
        self.body.push(content.into());
        self
    }

    pub fn action(mut self, action: impl Into<Element<'a, Message>>) -> Self {
        self.actions.push(action.into());
        self
    }

    /// Kutunun genişliği, 12 piksellik gövde metnine göre; yazı boyutuyla
    /// büyür.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Kutunun en çok yüksekliği (12 piksellik gövde metnine göre). Uzun
    /// gövdeyi `height(Fill)` bir kaydırma alanına koyun: kutu bu yükseklikte
    /// ya da pencere daha alçaksa pencerede durur, gövde kayar, eylem
    /// düğmeleri hep görünür kalır.
    pub fn max_height(mut self, height: f32) -> Self {
        self.max_height = Some(height);
        self
    }
}

impl<'a, Message: 'a> From<Dialog<'a, Message>> for Element<'a, Message> {
    fn from(dialog: Dialog<'a, Message>) -> Self {
        let mut header = row![label::title(dialog.title), space::horizontal()].align_y(Center);

        if let Some(hint) = dialog.hint {
            header = header.push(label::mono_caption(hint));
        }

        let mut content = column![header, horizontal_divider()].spacing(12);

        for part in dialog.body {
            content = content.push(part);
        }

        if !dialog.actions.is_empty() {
            let mut actions = row![space::horizontal()].spacing(6);

            for action in dialog.actions {
                actions = actions.push(action);
            }

            content = content.push(actions);
        }

        let boxed = container(content)
            .width(typography::scaled(dialog.width))
            .padding(18)
            .style(style::container::popover);

        match dialog.max_height {
            Some(height) => boxed.max_height(typography::scaled(height)).into(),
            None => boxed.into(),
        }
    }
}

/// Onay kutusu: yapılacak işin ne olduğu, sonucu ve iki düğme.
///
/// ```text
/// ┌──────────────────────────────────────────────┐
/// │ (⚠)  Bütün çizimler silinsin mi?              │
/// │      Çizimler katmanındaki 5 öğe silinecek.   │
/// │      Bildirimdeki "Geri al" geri getirir.     │
/// │                        [Vazgeç] [Tümünü sil]  │
/// └──────────────────────────────────────────────┘
/// ```
///
/// Başlık soru olarak yazılır; onay düğmesi işin adını taşır ("Tamam"
/// değil, "Tümünü sil"). Yıkıcı işte ([`destructive`](Confirm::destructive))
/// onay düğmesi kırmızıdır. Enter onaylar, Esc vazgeçer: uygulama bu
/// tuşları iletişim kutusu açıkken mesajlara bağlar.
pub struct Confirm<'a, Message> {
    title: Fragment<'a>,
    message: Option<Fragment<'a>>,
    detail: Option<Fragment<'a>>,
    severity: Severity,
    confirm: Fragment<'a>,
    cancel: Fragment<'a>,
    on_confirm: Message,
    on_cancel: Message,
    destructive: bool,
}

impl<'a, Message: Clone + 'a> Confirm<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>, on_confirm: Message, on_cancel: Message) -> Self {
        Self {
            title: title.into_fragment(),
            message: None,
            detail: None,
            severity: Severity::Warning,
            confirm: "Tamam".into_fragment(),
            cancel: "Vazgeç".into_fragment(),
            on_confirm,
            on_cancel,
            destructive: false,
        }
    }

    /// Ne olacağı.
    pub fn message(mut self, message: impl IntoFragment<'a>) -> Self {
        self.message = Some(message.into_fragment());
        self
    }

    /// Sonucu ya da geri alınıp alınamayacağı; sönük yazılır.
    pub fn detail(mut self, detail: impl IntoFragment<'a>) -> Self {
        self.detail = Some(detail.into_fragment());
        self
    }

    /// Onay düğmesinin adı: işin kendisi (ör. "Tümünü sil").
    pub fn confirm(mut self, label: impl IntoFragment<'a>) -> Self {
        self.confirm = label.into_fragment();
        self
    }

    pub fn cancel(mut self, label: impl IntoFragment<'a>) -> Self {
        self.cancel = label.into_fragment();
        self
    }

    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Yıkıcı iş: onay düğmesi ve ikon kırmızıdır.
    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self.severity = Severity::Error;
        self
    }
}

impl<'a, Message: Clone + 'a> From<Confirm<'a, Message>> for Element<'a, Message> {
    fn from(confirm: Confirm<'a, Message>) -> Self {
        let severity = confirm.severity;

        let symbol = container(
            icon(if confirm.destructive {
                crate::icon::Icon::Warning
            } else {
                severity.icon()
            })
            .size(18.0)
            .tone(severity.tone()),
        )
        .center_x(36)
        .center_y(36)
        .style(move |theme: &Theme| container::Style {
            background: Some(Background::Color(
                severity.color(&Tokens::of(theme)).scale_alpha(0.14),
            )),
            border: border::rounded(18.0),
            ..container::Style::default()
        });

        let mut text = column![label::title(confirm.title)].spacing(6).width(Fill);

        if let Some(message) = confirm.message {
            text = text.push(label::body(message));
        }

        if let Some(detail) = confirm.detail {
            text = text.push(label::muted(detail));
        }

        let actions = row![
            space::horizontal(),
            button(label::body(confirm.cancel))
                .on_press(confirm.on_cancel)
                .padding([5, 16])
                .style(style::button::secondary),
            button(label::body(confirm.confirm))
                .on_press(confirm.on_confirm)
                .padding([5, 16])
                .style(if confirm.destructive {
                    style::button::danger
                } else {
                    style::button::primary
                }),
        ]
        .spacing(6);

        container(column![row![symbol, text].spacing(14), actions].spacing(18))
            .width(typography::scaled(420.0))
            .padding(18)
            .style(style::container::popover)
            .into()
    }
}

/// Kısayol ve açıklamalarını iki sütunda listeler.
pub struct ShortcutList<'a> {
    items: Vec<(Fragment<'a>, Fragment<'a>)>,
    key_width: f32,
}

impl<'a> ShortcutList<'a> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            key_width: 160.0,
        }
    }

    pub fn item(mut self, keys: impl IntoFragment<'a>, description: impl IntoFragment<'a>) -> Self {
        self.items
            .push((keys.into_fragment(), description.into_fragment()));
        self
    }
}

impl<'a> Default for ShortcutList<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<ShortcutList<'a>> for Element<'a, Message> {
    fn from(list: ShortcutList<'a>) -> Self {
        Column::with_children(list.items.into_iter().map(|(keys, description)| {
            row![
                label::mono(keys).width(typography::scaled(list.key_width)),
                label::muted(description),
            ]
            .spacing(12)
            .into()
        }))
        .spacing(7)
        .into()
    }
}
