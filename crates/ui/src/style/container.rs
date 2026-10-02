//! Kap (container) stilleri: yüzeyler, açılır paneller ve küçük süsler.

use iced::widget::container::Style;
use iced::{Background, Border, Color, Theme, border};

use crate::theme::Tokens;
use crate::theme::shape::{self, Level};

use super::button::radius;

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
            radius: radius().into(),
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
            radius: radius().into(),
        },
        ..Style::default()
    }
}

/// Kenar renginde dolgu. İçine 1 piksel aralıkla dizilen hücreler arasında
/// ızgara çizgisi gibi görünür.
pub fn grid_lines(theme: &Theme) -> Style {
    fill(Tokens::of(theme).border)
}

/// Kenarlı yüzey: gruplanmış içerik ve örnek alanları (kart kademesi).
pub fn bordered(theme: &Theme) -> Style {
    let t = Tokens::of(theme);

    Style {
        background: Some(Background::Color(t.surface)),
        border: Border {
            color: t.border,
            width: 1.0,
            radius: shape::md().into(),
        },
        ..Style::default()
    }
}

/// Açılır menüler, açılır paneller ve ipuçları: kenar, menü kademesinin
/// köşesi ve gölgesi (web'in `.menu`'sü: `--r-md`, `--shadow-pop`).
pub fn popover(theme: &Theme) -> Style {
    let t = Tokens::of(theme);

    Style {
        text_color: Some(t.text),
        background: Some(Background::Color(t.popover)),
        border: Border {
            color: t.border_strong(),
            width: 1.0,
            radius: shape::md().into(),
        },
        shadow: shape::shadow(Level::Pop, &t),
        ..Style::default()
    }
}

/// Gölgesini kendisi çizmeyen açılır kutu: kutuyu kırpan bir katmanda çizilen
/// (ör. pencereye sığmayınca kayan bağlam menüsü) ve gölgesi o katmanın
/// dışında ayrıca çizilen kutular için. Kırpılan katmanda gölge yalnız yuvarlak
/// köşelerin dışında kalır ve köşeleri karartırdı.
pub fn popover_flat(theme: &Theme) -> Style {
    Style {
        shadow: iced::Shadow::default(),
        ..popover(theme)
    }
}

/// İletişim kutuları ve pencereler: panel zemini, belirgin kenar, pencere
/// kademesinin köşesi ve gölgesi (web'in `.dialog`'u).
pub fn dialog(theme: &Theme) -> Style {
    let t = Tokens::of(theme);

    Style {
        text_color: Some(t.text),
        background: Some(Background::Color(t.surface)),
        border: Border {
            color: t.border_strong(),
            width: 1.0,
            radius: shape::lg().into(),
        },
        shadow: shape::shadow(Level::Window, &t),
        ..Style::default()
    }
}

/// Bağlam menüsü satırı: vurgulanınca yumuşak vurgu zemini, yazı kendi
/// renginde (DESIGN.md §3.3; web'in `.menu__item[data-active]`'i);
/// tehlikeli komut kırmızıyla, devre dışı komut sönük yazılır.
pub fn menu_item(highlighted: bool, enabled: bool, danger: bool) -> impl Fn(&Theme) -> Style {
    move |theme| {
        let t = Tokens::of(theme);

        let (background, text) = match (highlighted && enabled, enabled, danger) {
            (true, _, true) => (Some(t.danger.scale_alpha(0.16)), t.danger),
            (true, _, false) => (Some(t.selection()), t.text),
            (false, false, _) => (None, t.disabled()),
            (false, true, true) => (None, t.danger),
            (false, true, false) => (None, t.text),
        };

        Style {
            text_color: Some(text),
            background: background.map(Background::Color),
            border: border::rounded(radius()),
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
        border: border::rounded(shape::xs()),
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
            radius: shape::xs().into(),
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
            border: border::rounded(radius()),
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
            radius: shape::md().into(),
        },
        shadow: shape::shadow(Level::Float, &t),
        ..Style::default()
    }
}

/// Parçalı seçimin gömük yuvası (web'in `.seg`'i): koyu temada alan
/// zemini, aydınlıkta başlık grisi; ince kenar. Parçalar içinde 2 piksel
/// arayla durur, seçili olan kalkık yüzeydedir.
pub fn segmented(theme: &Theme) -> Style {
    let t = Tokens::of(theme);

    Style {
        background: Some(Background::Color(t.well())),
        border: Border {
            color: t.border,
            width: 1.0,
            radius: shape::md().into(),
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
            radius: shape::xs().into(),
        },
        ..Style::default()
    }
}

/// Sayı rozeti (ör. bağlamsal sekmenin seçili nesne sayısı): yumuşak vurgu
/// zemini, yuvarlak (web'in `ribbon__count`'u).
pub fn count(theme: &Theme) -> Style {
    let t = Tokens::of(theme);

    Style {
        text_color: Some(t.accent_hover),
        background: Some(Background::Color(t.accent.scale_alpha(0.18))),
        border: Border {
            radius: crate::theme::shape::radius(8.0).into(),
            ..Border::default()
        },
        ..Style::default()
    }
}

/// Uyarı sayısı rozeti (ör. alt panelin Uyarılar sekmesi): amber zemin,
/// üzerinde okunur koyu ya da açık yazı, yuvarlak (web'in `.badge`'i).
pub fn warning_count(theme: &Theme) -> Style {
    let t = Tokens::of(theme);
    let dark = crate::theme::accent::mix(Color::BLACK, t.warning, 0.1);
    let ink = if crate::theme::accent::contrast(t.warning, dark)
        >= crate::theme::accent::contrast(t.warning, Color::WHITE)
    {
        dark
    } else {
        Color::WHITE
    };

    Style {
        text_color: Some(ink),
        background: Some(Background::Color(t.warning)),
        border: Border {
            radius: crate::theme::shape::radius(8.0).into(),
            ..Border::default()
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
            radius: shape::radius(1.0).into(),
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
            radius: border::radius(shape::radius(1.0)),
        },
        ..Style::default()
    }
}
