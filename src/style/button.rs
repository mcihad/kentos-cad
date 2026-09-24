//! Düğme stilleri.

use iced::widget::button::{Status, Style};
use iced::{Background, Border, Color, Theme, border};

use crate::theme::Tokens;

/// Bütün düğmelerin köşe yarıçapı.
pub const RADIUS: f32 = 2.0;

fn is_hovered(status: Status) -> bool {
    matches!(status, Status::Hovered | Status::Pressed)
}

fn style(background: Color, text_color: Color, border: Border) -> Style {
    Style {
        background: Some(Background::Color(background)),
        text_color,
        border,
        ..Style::default()
    }
}

/// Çerçevesiz düğme: şerit, gezinme çubuğu ve küçük eylemler.
///
/// Üzerine gelince yüzey rengi ve kenar, basılıyken vurgu kenarı alır;
/// devre dışı durumda metni ve ikonu sönükleşir.
pub fn flat(theme: &Theme, status: Status) -> Style {
    let t = Tokens::of(theme);

    let (background, edge, text) = match status {
        Status::Active => (Color::TRANSPARENT, Color::TRANSPARENT, t.text),
        Status::Hovered => (t.surface_hover, t.border, t.text),
        Status::Pressed => (t.selection(), t.accent, t.text),
        Status::Disabled => (Color::TRANSPARENT, Color::TRANSPARENT, t.disabled()),
    };

    style(
        background,
        text,
        Border {
            color: edge,
            width: 1.0,
            radius: RADIUS.into(),
        },
    )
}

/// Birincil eylem: iletişim kutusunu onaylar, uygulamadan çıkar.
pub fn primary(theme: &Theme, status: Status) -> Style {
    let t = Tokens::of(theme);

    let (background, text) = match status {
        Status::Active => (t.accent, t.on_accent),
        Status::Hovered | Status::Pressed => (t.accent_hover, t.on_accent),
        Status::Disabled => (t.accent.scale_alpha(0.3), t.on_accent.scale_alpha(0.5)),
    };

    style(background, text, border::rounded(RADIUS))
}

/// Yıkıcı birincil eylem (ör. "Tümünü sil"): kırmızı zemin.
pub fn danger(theme: &Theme, status: Status) -> Style {
    let t = Tokens::of(theme);

    let (background, text) = match status {
        Status::Active => (t.danger, t.on_accent),
        Status::Hovered | Status::Pressed => (lighten(t.danger, 0.12), t.on_accent),
        Status::Disabled => (t.danger.scale_alpha(0.3), t.on_accent.scale_alpha(0.5)),
    };

    style(background, text, border::rounded(RADIUS))
}

/// Rengi beyaza doğru `amount` kadar açar.
fn lighten(color: Color, amount: f32) -> Color {
    Color {
        r: color.r + (1.0 - color.r) * amount,
        g: color.g + (1.0 - color.g) * amount,
        b: color.b + (1.0 - color.b) * amount,
        a: color.a,
    }
}

/// İkincil eylem: kenarlı, yüzey renginde düğme.
pub fn secondary(theme: &Theme, status: Status) -> Style {
    let t = Tokens::of(theme);

    let (background, text) = match status {
        Status::Active => (t.surface_alt, t.text),
        Status::Hovered | Status::Pressed => (t.surface_hover, t.text),
        Status::Disabled => (t.surface_alt, t.disabled()),
    };

    style(
        background,
        text,
        Border {
            color: t.border,
            width: 1.0,
            radius: RADIUS.into(),
        },
    )
}

/// Tablo satırındaki küçük ikon düğmeleri: zemin yok, üzerine gelince
/// vurgu rengi.
pub fn subtle(theme: &Theme, status: Status) -> Style {
    let t = Tokens::of(theme);

    Style {
        background: None,
        text_color: match status {
            Status::Hovered | Status::Pressed => t.accent,
            Status::Disabled => t.disabled(),
            Status::Active => t.muted,
        },
        ..Style::default()
    }
}

/// Seçili olmayan şerit sekmesi.
pub fn tab(theme: &Theme, status: Status) -> Style {
    let t = Tokens::of(theme);

    Style {
        background: None,
        text_color: if is_hovered(status) { t.text } else { t.muted },
        ..Style::default()
    }
}

/// Araç düğmesi: etkin araç vurgu zemini ve kenarıyla gösterilir.
pub fn tool(active: bool) -> impl Fn(&Theme, Status) -> Style {
    move |theme, status| {
        if !active {
            return flat(theme, status);
        }

        let t = Tokens::of(theme);

        style(
            t.selection(),
            t.text,
            Border {
                color: t.accent,
                width: 1.0,
                radius: RADIUS.into(),
            },
        )
    }
}

/// Tablo satırı: seçili satır vurgu zemini ve kenarıyla gösterilir.
pub fn row(selected: bool) -> impl Fn(&Theme, Status) -> Style {
    table_row(selected, selected)
}

/// Çoklu seçimli tablo satırı: seçili satırlar vurgu zeminiyle, birincil
/// (`current`) satır ayrıca vurgu kenarıyla gösterilir.
pub fn table_row(selected: bool, current: bool) -> impl Fn(&Theme, Status) -> Style {
    move |theme, status| {
        let t = Tokens::of(theme);

        let background = match (selected, is_hovered(status)) {
            (true, _) => t.selection(),
            (false, true) => t.surface_hover,
            (false, false) => Color::TRANSPARENT,
        };

        style(
            background,
            t.text,
            Border {
                color: if current {
                    t.accent
                } else {
                    Color::TRANSPARENT
                },
                width: if current { 1.0 } else { 0.0 },
                radius: 0.0.into(),
            },
        )
    }
}

/// Özellik ızgarasındaki kategori başlığı: başlık zemininde, üzerine
/// gelince açılır.
pub fn category(theme: &Theme, status: Status) -> Style {
    let t = Tokens::of(theme);

    style(
        if is_hovered(status) {
            t.surface_hover
        } else {
            t.header
        },
        t.text,
        Border::default(),
    )
}

/// Açılıp kapanan panelin başlığı: başlık zemininde; üzerine gelince hafif
/// bir katman.
pub fn panel_header(theme: &Theme, status: Status) -> Style {
    let t = Tokens::of(theme);

    Style {
        background: match status {
            Status::Hovered => Some(Background::Color(t.layer(0.04))),
            Status::Pressed => Some(Background::Color(t.layer(0.08))),
            Status::Active | Status::Disabled => None,
        },
        text_color: t.text,
        ..Style::default()
    }
}

/// Takvim ve saat ızgarası hücresinin metin tonu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellTone {
    Normal,
    /// Hafta sonu günleri.
    Weekend,
    /// Gösterilen ayın ya da on yılın dışında kalan hücreler.
    Outside,
}

/// Takvim ve saat ızgarası hücresi: seçili hücre vurgu zeminiyle, bugün (ya
/// da şimdiki saat) vurgu kenarıyla gösterilir.
pub fn calendar(selected: bool, today: bool, tone: CellTone) -> impl Fn(&Theme, Status) -> Style {
    move |theme, status| {
        let t = Tokens::of(theme);

        let background = match (selected, is_hovered(status)) {
            (true, false) => t.accent,
            (true, true) => t.accent_hover,
            (false, true) => t.surface_hover,
            (false, false) => Color::TRANSPARENT,
        };

        let text = match (selected, tone) {
            (true, _) => t.on_accent,
            (false, CellTone::Normal) => t.text,
            (false, CellTone::Weekend) => t.muted,
            (false, CellTone::Outside) => t.disabled(),
        };

        style(
            background,
            text,
            Border {
                color: if today && !selected {
                    t.accent
                } else {
                    Color::TRANSPARENT
                },
                width: if today && !selected { 1.0 } else { 0.0 },
                radius: 3.0.into(),
            },
        )
    }
}

/// Ağaçtaki onay kutusu: işaretli ya da karışık kutu vurgu renginde dolu,
/// işaretsiz kutu kenarlı; üzerine gelince kenar vurgulanır.
pub fn check(filled: bool) -> impl Fn(&Theme, Status) -> Style {
    move |theme, status| {
        let t = Tokens::of(theme);
        let hovered = is_hovered(status);

        let (background, edge) = match (filled, hovered) {
            (true, false) => (t.accent, t.accent),
            (true, true) => (t.accent_hover, t.accent_hover),
            (false, false) => (t.field, t.border),
            (false, true) => (t.field, t.accent),
        };

        style(
            background,
            t.on_accent,
            Border {
                color: edge,
                width: 1.0,
                radius: RADIUS.into(),
            },
        )
    }
}

/// Parçalı seçim düğmesi (ör. seçim yöntemi); seçili parça vurgu zeminiyle.
pub fn segment(selected: bool) -> impl Fn(&Theme, Status) -> Style {
    move |theme, status| {
        let t = Tokens::of(theme);

        let (background, text) = match (selected, is_hovered(status)) {
            (true, _) => (t.accent, t.on_accent),
            (false, true) => (t.surface_hover, t.text),
            (false, false) => (t.surface_alt, t.text),
        };

        style(background, text, Border::default())
    }
}

/// Tablo başlığı: sıralanabilir sütun adları; sıralı sütun belirgin.
pub fn header(sorted: bool) -> impl Fn(&Theme, Status) -> Style {
    move |theme, status| {
        let t = Tokens::of(theme);

        Style {
            background: None,
            text_color: if sorted || is_hovered(status) {
                t.text
            } else {
                t.muted
            },
            ..Style::default()
        }
    }
}

/// Zemini ve kenarı olmayan küçük ikon düğmesi (ör. komut kutusunun
/// denetimleri): sönük ikon, üzerine gelince hafif bir katman ve tam renk.
pub fn ghost(theme: &Theme, status: Status) -> Style {
    let t = Tokens::of(theme);

    let (background, text) = match status {
        Status::Active => (Color::TRANSPARENT, t.muted),
        Status::Hovered => (t.layer(0.07), t.text),
        Status::Pressed => (t.layer(0.12), t.text),
        Status::Disabled => (Color::TRANSPARENT, t.disabled()),
    };

    style(background, text, border::rounded(3.0))
}

/// Durum çubuğu anahtarı: açıkken ikonu vurgu renginde, metni tam renkte;
/// kapalıyken ikisi de sönük. Zemin yalnızca üzerine gelince belirir.
pub fn status_toggle(active: bool) -> impl Fn(&Theme, Status) -> Style {
    move |theme, status| {
        let t = Tokens::of(theme);

        let background = match status {
            Status::Hovered => t.layer(0.06),
            Status::Pressed => t.layer(0.1),
            Status::Active | Status::Disabled => Color::TRANSPARENT,
        };

        style(
            background,
            if active { t.text } else { t.muted },
            border::rounded(3.0),
        )
    }
}

/// Komut istemindeki seçenek (ör. "Geri al"): ince kenarlı; üzerine gelince
/// vurgu kenarı alır.
pub fn keyword(theme: &Theme, status: Status) -> Style {
    let t = Tokens::of(theme);

    let (background, edge) = match status {
        Status::Active | Status::Disabled => (Color::TRANSPARENT, t.border),
        Status::Hovered => (t.selection(), t.accent),
        Status::Pressed => (t.accent.scale_alpha(0.35), t.accent),
    };

    style(
        background,
        t.text,
        Border {
            color: edge,
            width: 1.0,
            radius: 3.0.into(),
        },
    )
}

/// Renk seçimindeki örneğin çerçevesi: seçili renk vurgu halkasıyla,
/// üzerine gelinen renk ince kenarla gösterilir. İçine rengi gösteren
/// küçük bir kutu konur.
pub fn swatch(selected: bool) -> impl Fn(&Theme, Status) -> Style {
    move |theme, status| {
        let t = Tokens::of(theme);

        let (edge, width) = match (selected, is_hovered(status)) {
            (true, _) => (t.accent, 2.0),
            (false, true) => (t.muted, 1.0),
            (false, false) => (Color::TRANSPARENT, 1.0),
        };

        Style {
            background: None,
            text_color: t.text,
            border: Border {
                color: edge,
                width,
                radius: 3.0.into(),
            },
            ..Style::default()
        }
    }
}

/// Durum çubuğundaki açık/kapalı anahtarlar.
pub fn toggle(active: bool) -> impl Fn(&Theme, Status) -> Style {
    move |theme, status| {
        let t = Tokens::of(theme);
        let hovered = is_hovered(status);

        let background = match (active, hovered) {
            (true, _) => t.accent.scale_alpha(if hovered { 0.3 } else { 0.18 }),
            (false, true) => t.surface_hover,
            (false, false) => Color::TRANSPARENT,
        };

        style(
            background,
            if active { t.accent_hover } else { t.muted },
            border::rounded(RADIUS),
        )
    }
}

/// Uygulama menüsünü açan marka düğmesi; menü açıkken koyulaşır.
pub fn brand(open: bool) -> impl Fn(&Theme, Status) -> Style {
    move |theme, status| {
        let t = Tokens::of(theme);

        style(
            if open || is_hovered(status) {
                t.accent_hover
            } else {
                t.accent
            },
            t.on_accent,
            border::rounded(border::top(RADIUS)),
        )
    }
}

/// Liste ve menü satırları; `highlighted` satır (ör. alt menüsü açık
/// komut) vurgu zemini ve kenarıyla gösterilir.
pub fn list_item(highlighted: bool) -> impl Fn(&Theme, Status) -> Style {
    move |theme, status| {
        let t = Tokens::of(theme);

        let background = if highlighted {
            t.selection()
        } else if is_hovered(status) {
            t.surface_hover
        } else {
            Color::TRANSPARENT
        };

        style(
            background,
            t.text,
            Border {
                color: if highlighted {
                    t.accent
                } else {
                    Color::TRANSPARENT
                },
                width: if highlighted { 1.0 } else { 0.0 },
                radius: RADIUS.into(),
            },
        )
    }
}
