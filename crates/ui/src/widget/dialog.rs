//! İletişim kutusu, onay kutusu ve kısayol listesi.
//!
//! [`Dialog`] ve [`Confirm`] yalnızca kutunun kendisidir; ortalamak ve
//! arkasını karartmak için [`overlay::modal`](crate::widget::overlay::modal)
//! ile gösterilir.

use iced::widget::space::Space;
use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{Column, button, column, container, row, scrollable, space};
use iced::{Background, Center, Element, Fill, Shrink, Theme, border};

use crate::icon::{Icon, icon};
use crate::label;
use crate::style;
use crate::theme::{Tokens, typography};
use crate::widget::fit::Fit;
use crate::widget::sizable;
use crate::widget::{Severity, Tip, horizontal_divider, overlay, tip};

/// Başlık, gövde ve sağa hizalı eylem düğmelerinden oluşan kutu.
pub struct Dialog<'a, Message> {
    title: Fragment<'a>,
    hint: Option<Fragment<'a>>,
    /// Gövdenin parçaları; `true`: kayan parça ([`Dialog::scroll`]).
    body: Vec<(Element<'a, Message>, bool)>,
    /// Düğme çubuğunun solunda, öbürlerinden ayrı duran eylemler.
    aside: Vec<Element<'a, Message>>,
    actions: Vec<Element<'a, Message>>,
    width: f32,
    max_height: Option<f32>,
    /// The key its size is kept by (docs/adr/0746); none: its title. `None` after `fixed`: not sized.
    key: Option<String>,
    sizable: bool,
    /// Başlıktaki kapatma düğmesinin mesajı; yoksa düğme yoktur.
    on_close: Option<Message>,
    close_hidden: bool,
}

impl<'a, Message: 'a> Dialog<'a, Message> {
    pub fn new(title: impl IntoFragment<'a>) -> Self {
        Self {
            title: title.into_fragment(),
            hint: None,
            body: Vec::new(),
            aside: Vec::new(),
            actions: Vec::new(),
            width: 500.0,
            max_height: None,
            key: None,
            sizable: true,
            on_close: None,
            close_hidden: false,
        }
    }

    /// Başlığın sağında kapatma düğmesi (×) gösterir; basılınca `message` gelir. Verilmezse düğme yoktur: kutu kendi
    /// düğmeleriyle kapanır (bir soru, yarıda bırakılmaması gereken bir iş).
    pub fn on_close(mut self, message: impl Into<Option<Message>>) -> Self {
        self.on_close = message.into();
        self
    }

    /// Kapatma düğmesini gizler, [`modal`](Dialog::modal) ile de.
    pub fn no_close(mut self) -> Self {
        self.on_close = None;
        self.close_hidden = true;
        self
    }

    /// Kutuyu pencerenin ortasına, arkasını karartarak yerleştirir ([`overlay::modal`]): karartılmış alana basmak ve
    /// başlıktaki × `on_dismiss`'i gönderir; × istenmiyorsa önce [`no_close`](Dialog::no_close).
    pub fn modal(mut self, on_dismiss: Message) -> Element<'a, Message>
    where
        Message: Clone,
    {
        if !self.close_hidden {
            self.on_close.get_or_insert_with(|| on_dismiss.clone());
        }
        overlay::modal(self, on_dismiss)
    }

    /// Başlığın sağındaki kısa not (ör. kutuyu açan kısayol).
    pub fn hint(mut self, hint: impl IntoFragment<'a>) -> Self {
        self.hint = Some(hint.into_fragment());
        self
    }

    pub fn push(mut self, content: impl Into<Element<'a, Message>>) -> Self {
        self.body.push((content.into(), false));
        self
    }

    /// Kayan gövde: kutu içeriği kadar uzar; pencereye ya da
    /// [`max_height`](Dialog::max_height)'e sığmayınca bu parça kayar,
    /// başlık ve eylem düğmeleri görünür kalır. Çubuk gövdenin yanında durur.
    pub fn scroll(mut self, content: impl Into<Element<'a, Message>>) -> Self {
        self.body.push((
            scrollable(content)
                .direction(style::field::body_scrollbar())
                .height(Shrink)
                .into(),
            true,
        ));
        self
    }

    /// Kutuyu en büyük boyunda dolduran kayan gövde: sekmeli ya da içeriği
    /// değişen kutular (ayarlar) sekme değişince boy değiştirmesin diye.
    /// Uygulama penceresi alçaksa kutu pencerede durur, gövde kayar.
    pub fn scroll_fill(mut self, content: impl Into<Element<'a, Message>>) -> Self {
        self.body.push((
            scrollable(content)
                .direction(style::field::body_scrollbar())
                .height(Fill)
                .into(),
            true,
        ));
        self
    }

    /// Kutunun kalan yerini alan gövde: kendi kayan bölmeleri olan düzenler
    /// (solda liste, sağda form) için; kaydırmayı parça kendisi yapar,
    /// içindekiler yüksekliği `Fill` ile doldurur. Pencere alçaksa kutu
    /// pencerede durur, bu parça kısalır, eylem düğmeleri görünür kalır.
    pub fn fill(mut self, content: impl Into<Element<'a, Message>>) -> Self {
        self.body.push((content.into(), true));
        self
    }

    pub fn action(mut self, action: impl Into<Element<'a, Message>>) -> Self {
        self.actions.push(action.into());
        self
    }

    /// Düğme çubuğunun soluna, öbür eylemlerden ayrı konan eylem (ör. bir
    /// işi bırakan cevap; web'in `aside`'ı).
    pub fn aside(mut self, action: impl Into<Element<'a, Message>>) -> Self {
        self.aside.push(action.into());
        self
    }

    /// Kutunun genişliği, 12 piksellik gövde metnine göre; yazı boyutuyla
    /// büyür.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Kutunun en çok yüksekliği (12 piksellik gövde metnine göre). Uzun
    /// gövdeyi [`scroll`](Dialog::scroll) ya da
    /// [`scroll_fill`](Dialog::scroll_fill) ile verin: kutu en çok bu
    /// yükseklikte, pencere daha alçaksa pencerede durur, gövde kayar, eylem
    /// düğmeleri hep görünür kalır.
    pub fn max_height(mut self, height: f32) -> Self {
        self.max_height = Some(height);
        self
    }

    /// The key the box's size is kept by, when its title changes (`Formla düzenle — bina`): one key, one size.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// A box that keeps its own size: it is not resized, not made full screen (a short question).
    pub fn fixed(mut self) -> Self {
        self.sizable = false;
        self
    }
}

impl<'a, Message: Clone + 'a> From<Dialog<'a, Message>> for Element<'a, Message> {
    fn from(dialog: Dialog<'a, Message>) -> Self {
        let key = dialog.key.unwrap_or_else(|| dialog.title.to_string());
        let kept = dialog.sizable && sizable::sizes::get(&key).is_some();
        let mut header = row![label::title(dialog.title), space::horizontal()]
            .spacing(6)
            .align_y(Center);

        if let Some(hint) = dialog.hint {
            header = header.push(
                container(label::caption(hint))
                    .padding([0, 5])
                    .style(style::container::keycap),
            );
        }
        let side = typography::from_default(sizable::TOGGLE);
        let closable = dialog.on_close.is_some();
        let mut tools = iced::widget::Row::new()
            .spacing(sizable::TOGGLE_GAP)
            .align_y(Center);
        // The full screen toggle's square, drawn by `Sizable` (docs/adr/0746).
        if dialog.sizable {
            tools = tools.push(Space::new().width(side).height(side));
        }
        if let Some(message) = dialog.on_close {
            tools = tools.push(tip(
                button(container(icon(Icon::Close).size(14.0)).center(Fill))
                    .on_press(message)
                    .padding(0)
                    .width(side)
                    .height(side)
                    .style(style::button::flat),
                Tip::new("Kapat"),
                iced::widget::tooltip::Position::Bottom,
            ));
        }
        header = header.push(tools);

        // Kayan parça, başlık ve düğmeler yerleştikten sonra kalan yeri alır.
        let mut content = Fit::new()
            .spacing(12.0)
            .push(header)
            .push(horizontal_divider());

        for (part, scrolls) in dialog.body {
            content = if scrolls {
                content.push_elastic(part)
            } else {
                content.push(part)
            };
        }

        if !dialog.actions.is_empty() || !dialog.aside.is_empty() {
            let mut actions = iced::widget::Row::new().spacing(6);

            for action in dialog.aside {
                actions = actions.push(action);
            }

            actions = actions.push(space::horizontal());

            for action in dialog.actions {
                actions = actions.push(action);
            }

            content = content.push(actions);
        }

        // The dialog is Tab's scope: the keyboard goes round its fields, not out to the window behind.
        let boxed = container(content)
            .id(crate::focus::scope_id())
            .padding(sizable::PAD)
            .style(style::container::dialog);

        // A kept size: the box fills what `Sizable` gives it; else its own width and height.
        let boxed = if kept {
            boxed.width(Fill).height(Fill)
        } else {
            let boxed = boxed.width(typography::scaled(dialog.width));
            match dialog.max_height {
                Some(height) => boxed.max_height(typography::scaled(height)),
                None => boxed,
            }
        };
        if dialog.sizable {
            sizable::Sizable::new(key, boxed).closable(closable).into()
        } else {
            boxed.into()
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

impl<'a, Message: Clone + 'a> Confirm<'a, Message> {
    /// Kutuyu pencerenin ortasına, arkasını karartarak yerleştirir ([`overlay::modal`]): karartılmış alana basmak
    /// `on_dismiss`'i gönderir. Bir sorunun × düğmesi yoktur: iki cevabı vardır.
    pub fn modal(self, on_dismiss: Message) -> Element<'a, Message> {
        overlay::modal(self, on_dismiss)
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
                .padding(crate::theme::metrics::action())
                .style(style::button::secondary),
            button(label::body(confirm.confirm))
                .on_press(confirm.on_confirm)
                .padding(crate::theme::metrics::action())
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
            .style(style::container::dialog)
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
