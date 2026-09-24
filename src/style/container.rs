//! Kap (container) stilleri: yüzeyler, açılır paneller ve küçük süsler.

use iced::widget::container::Style;
use iced::{Background, Border, Color, Shadow, Theme, Vector, border};

use crate::theme::Tokens;

use super::button::RADIUS;

fn fill(color: Color) -> Style {
    Style {
        background: Some(Background::Color(color)),
        ..Style::default()
    }
}

/// En dış çerçeve: sekme şeridi, durum çubuğu ve pencere zemini.
pub fn window(theme: &Theme) -> Style {
    fill(Tokens::of(theme).window)
}

/// Şerit, paneller ve menülerin gövdesi.
pub fn surface(theme: &Theme) -> Style {
    fill(Tokens::of(theme).surface)
}

/// İkincil yüzey: özellik değerleri, menünün ayrıntı bölmesi.
pub fn surface_alt(theme: &Theme) -> Style {
    fill(Tokens::of(theme).surface_alt)
}

/// Panel başlıkları ve kategori satırları.
pub fn header(theme: &Theme) -> Style {
    fill(Tokens::of(theme).header)
}

/// Giriş alanı zemini (ör. komut satırı).
pub fn field(theme: &Theme) -> Style {
    fill(Tokens::of(theme).field)
}

/// Kenar renginde dolgu. İçine 1 piksel aralıkla dizilen hücreler arasında
/// ızgara çizgisi gibi görünür.
pub fn grid_lines(theme: &Theme) -> Style {
    fill(Tokens::of(theme).border)
}

/// Kenarlı yüzey: gruplanmış içerik ve örnek alanları.
pub fn bordered(theme: &Theme) -> Style {
    let t = Tokens::of(theme);

    Style {
        background: Some(Background::Color(t.surface)),
        border: Border {
            color: t.border,
            width: 1.0,
            radius: RADIUS.into(),
        },
        ..Style::default()
    }
}

/// Açılır menüler, ipuçları ve iletişim kutuları: kenar ve gölge.
pub fn popover(theme: &Theme) -> Style {
    let t = Tokens::of(theme);

    Style {
        text_color: Some(t.text),
        background: Some(Background::Color(t.popover)),
        border: Border {
            color: t.border,
            width: 1.0,
            radius: RADIUS.into(),
        },
        shadow: Shadow {
            color: t.shadow(),
            offset: Vector::new(0.0, 3.0),
            blur_radius: 10.0,
        },
        ..Style::default()
    }
}

/// Model alanı üzerinde yüzen, yarı saydam araç çubukları.
pub fn floating(theme: &Theme) -> Style {
    let t = Tokens::of(theme);

    Style {
        background: Some(Background::Color(t.surface.scale_alpha(0.92))),
        border: Border {
            color: t.border,
            width: 1.0,
            radius: RADIUS.into(),
        },
        ..Style::default()
    }
}

/// Parçalı seçim çerçevesi: parçalar arasındaki 1 piksellik boşluklar kenar
/// renginde görünür.
pub fn segmented(theme: &Theme) -> Style {
    let t = Tokens::of(theme);

    Style {
        background: Some(Background::Color(t.border)),
        border: Border {
            color: t.border,
            width: 1.0,
            radius: RADIUS.into(),
        },
        ..Style::default()
    }
}

/// Kısa kod veya biçim etiketi (ör. "PDF").
pub fn badge(theme: &Theme) -> Style {
    let t = Tokens::of(theme);

    Style {
        text_color: Some(t.text),
        background: Some(Background::Color(t.surface)),
        border: Border {
            color: t.border,
            width: 1.0,
            radius: RADIUS.into(),
        },
        ..Style::default()
    }
}

/// Kalıcı iletişim kutusunun arkasındaki karartma.
pub fn scrim(theme: &Theme) -> Style {
    fill(Tokens::of(theme).scrim())
}

/// Seçili sekmenin üstündeki vurgu çizgisi.
pub fn accent(theme: &Theme) -> Style {
    fill(Tokens::of(theme).accent)
}

/// Verilen renkte, ince koyu kenarlı küçük renk örneği.
pub fn swatch(color: Color) -> impl Fn(&Theme) -> Style {
    move |_theme| Style {
        background: Some(Background::Color(color)),
        border: Border {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.35),
            width: 1.0,
            radius: 1.0.into(),
        },
        ..Style::default()
    }
}

/// Verilen renkte dolgu.
pub fn solid(color: Color) -> impl Fn(&Theme) -> Style {
    move |_theme| fill(color)
}

/// Verilen renkte, dolgusuz çerçeve.
pub fn outline(color: Color, width: f32) -> impl Fn(&Theme) -> Style {
    move |_theme| Style {
        border: Border {
            color,
            width,
            radius: border::radius(1.0),
        },
        ..Style::default()
    }
}
