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

/// Simge karosu: başlık renginde, yuvarlak köşeli (ör. uygulama
/// menüsündeki komutun simgesi).
pub fn tile(theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(Tokens::of(theme).header)),
        border: Border {
            radius: RADIUS.into(),
            ..Border::default()
        },
        ..Style::default()
    }
}

/// Giriş alanı zemini (ör. komut satırı).
pub fn field(theme: &Theme) -> Style {
    fill(Tokens::of(theme).field)
}

/// Giriş alanıyla aynı zemin ve kenar: seçim kutusu, arama kutusu.
pub fn field_box(theme: &Theme) -> Style {
    let t = Tokens::of(theme);

    Style {
        background: Some(Background::Color(t.field)),
        border: Border {
            color: t.border,
            width: 1.0,
            radius: RADIUS.into(),
        },
        ..Style::default()
    }
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

/// Bağlam menüsü satırı: vurgulanınca vurgu zemini ve açık metin;
/// tehlikeli komut kırmızıyla, devre dışı komut sönük yazılır.
pub fn menu_item(highlighted: bool, enabled: bool, danger: bool) -> impl Fn(&Theme) -> Style {
    move |theme| {
        let t = Tokens::of(theme);

        let (background, text) = match (highlighted && enabled, enabled, danger) {
            (true, _, true) => (Some(t.danger), t.on_accent),
            (true, _, false) => (Some(t.accent), t.on_accent),
            (false, false, _) => (None, t.disabled()),
            (false, true, true) => (None, t.danger),
            (false, true, false) => (None, t.text),
        };

        Style {
            text_color: Some(text),
            background: background.map(Background::Color),
            border: border::rounded(RADIUS),
            ..Style::default()
        }
    }
}

/// Komut isteminde etkin komutun adı (ör. CIZGI): vurgu zemininde, vurgu
/// renginde.
pub fn token(theme: &Theme) -> Style {
    let t = Tokens::of(theme);

    Style {
        text_color: Some(t.accent_hover),
        background: Some(Background::Color(t.selection())),
        border: border::rounded(3.0),
        ..Style::default()
    }
}

/// Klavye tuşu (ör. Tab, Enter): ince kenarlı küçük kutu.
pub fn keycap(theme: &Theme) -> Style {
    let t = Tokens::of(theme);

    Style {
        text_color: Some(t.muted),
        border: Border {
            color: t.border,
            width: 1.0,
            radius: 3.0.into(),
        },
        ..Style::default()
    }
}

/// Otomatik tamamlama satırı: vurgulanan satır seçim zemininde.
pub fn suggestion(highlighted: bool) -> impl Fn(&Theme) -> Style {
    move |theme| {
        let t = Tokens::of(theme);

        Style {
            text_color: Some(t.text),
            background: highlighted.then(|| Background::Color(t.selection())),
            border: border::rounded(RADIUS),
            ..Style::default()
        }
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
