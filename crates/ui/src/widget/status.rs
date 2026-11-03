//! Durum: bir kaynağın, bir işin ya da bir denetimin hâli, tek renk dilinde.
//!
//! ```text
//! (● Güncel)  (● Güncel değil)  (● Kurulamıyor)  (● Onaylı)  (● Kabul edilmedi)
//!   yeşil       amber        kırmızı          mavi         gri
//! ```
//!
//! Hap küçük ve yumuşaktır: tonun %14'ü zemin, %35'i 1 px kenar, tonun kendisi nokta ve yazı (11 px, 500). Süzgeç
//! hapı ([`status_filter`]) aynısının basılanıdır, sayısıyla: seçiliyken kenarı tam ton, zemini koyulaşır. Durum
//! ağacı ([`StatusTree`](super::status_tree::StatusTree)), sunucu işleri, kaynak durumu ve denetimler aynı beş tonu
//! kullanır: Güncel olan sessizdir, Bilgi ve Nötr dikkat istemez, Uyarı ve Hata ister.

use iced::widget::{button, container, row, text};
use iced::{Background, Border, Center, Color, Element, Theme};

use crate::icon::Tone;
use crate::theme::{Tokens, typography};

/// Bir şeyin hâli.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Status {
    /// Güncel, tamam: sessiz.
    Ok,
    /// Bir hâl ama sorun değil (kabul edilmemiş, boş).
    #[default]
    Neutral,
    /// Bilgi (onaylı, kilitli, paylaşılan).
    Info,
    /// Dikkat: güncel değil, eksik, elle değişti.
    Warning,
    /// Sorun: kurulamıyor, kaynağı yok, onaydan sonra değişti.
    Danger,
}

impl Status {
    /// Tonun rengi.
    pub fn color(self, t: &Tokens) -> Color {
        match self {
            Status::Ok => t.success,
            Status::Neutral => t.muted,
            Status::Info => t.accent,
            Status::Warning => t.warning,
            Status::Danger => t.danger,
        }
    }

    /// İkonun tonu.
    pub fn tone(self) -> Tone {
        match self {
            Status::Ok => Tone::Success,
            Status::Neutral => Tone::Muted,
            Status::Info => Tone::Accent,
            Status::Warning => Tone::Warning,
            Status::Danger => Tone::Danger,
        }
    }

    /// Dikkat istiyor mu: uyarı ya da sorun.
    pub fn wants_attention(self) -> bool {
        matches!(self, Status::Warning | Status::Danger)
    }
}

fn mix(c: Color, a: f32) -> Color {
    Color { a, ..c }
}

fn face<'a, Message: 'a>(status: Status, words: String) -> iced::widget::Row<'a, Message> {
    let dot = container(text("").size(1)).width(6).height(6).style(move |t: &Theme| container::Style {
        background: Some(Background::Color(status.color(&Tokens::of(t)))),
        border: Border { radius: 3.0.into(), ..Border::default() },
        ..container::Style::default()
    });
    row![
        dot,
        text(words)
            .size(typography::caption())
            .font(typography::ui())
            .wrapping(text::Wrapping::None)
            .style(move |t: &Theme| text::Style { color: Some(status.color(&Tokens::of(t))) }),
    ]
    .spacing(5)
    .align_y(Center)
}

fn pill_style(status: Status, strong: bool) -> impl Fn(&Theme) -> container::Style {
    move |t: &Theme| {
        let c = status.color(&Tokens::of(t));
        container::Style {
            background: Some(Background::Color(mix(c, if strong { 0.24 } else { 0.14 }))),
            border: Border { color: mix(c, if strong { 0.9 } else { 0.35 }), width: 1.0, radius: 9.0.into() },
            ..container::Style::default()
        }
    }
}

/// Durum hapı: tonun noktası ve kısa sözü.
pub fn status_pill<'a, Message: 'a>(status: Status, words: impl Into<String>) -> Element<'a, Message> {
    container(face(status, words.into())).padding([1, 7]).style(pill_style(status, false)).into()
}

/// Süzgeç hapı: sözü ve sayısıyla basılan durum hapı; `active` seçili olandır.
pub fn status_filter<'a, Message: Clone + 'a>(
    status: Status,
    words: impl Into<String>,
    count: usize,
    active: bool,
    on_press: Message,
) -> Element<'a, Message> {
    let body = row![
        face(status, words.into()),
        text(count.to_string())
            .size(typography::caption())
            .font(typography::mono())
            .style(move |t: &Theme| text::Style { color: Some(status.color(&Tokens::of(t))) }),
    ]
    .spacing(6)
    .align_y(Center);
    button(container(body).padding([1, 8]).style(pill_style(status, active)))
        .padding(0)
        .style(|_: &Theme, _| button::Style::default())
        .on_press(on_press)
        .into()
}
