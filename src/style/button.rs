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

    let background = match status {
        Status::Active => t.accent,
        Status::Hovered | Status::Pressed => t.accent_hover,
        Status::Disabled => t.accent.scale_alpha(0.5),
    };

    style(background, t.on_accent, border::rounded(RADIUS))
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
                color: if selected {
                    t.accent
                } else {
                    Color::TRANSPARENT
                },
                width: if selected { 1.0 } else { 0.0 },
                radius: 0.0.into(),
            },
        )
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
