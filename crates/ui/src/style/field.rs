//! Giriş alanları, açılır listeler, bölücüler ve kaydırma çubukları.

use iced::widget::pick_list::{Status as PickListStatus, Style as PickListStyle};
use iced::widget::{overlay, rule, scrollable, text_editor, text_input};
use iced::{Background, Border, Color, Shadow, Theme, Vector};

use crate::theme::Tokens;

use super::button::RADIUS;

/// Kenarlı metin girişi; odaklanınca kenar vurgu rengini alır.
pub fn input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let t = Tokens::of(theme);

    let edge = match status {
        text_input::Status::Focused { .. } => t.accent,
        text_input::Status::Hovered => t.muted,
        text_input::Status::Active | text_input::Status::Disabled => t.border,
    };

    text_input::Style {
        background: Background::Color(t.field),
        border: Border {
            color: edge,
            width: 1.0,
            radius: RADIUS.into(),
        },
        icon: t.muted,
        placeholder: t.muted.scale_alpha(0.75),
        value: t.text,
        selection: t.accent.scale_alpha(0.35),
    }
}

/// Doğrulanan metin girişi: değer geçersizken kenar kırmızıdır.
pub fn validated(invalid: bool) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |theme, status| {
        let style = input(theme, status);

        if invalid {
            text_input::Style {
                border: Border {
                    color: Tokens::of(theme).danger,
                    ..style.border
                },
                ..style
            }
        } else {
            style
        }
    }
}

/// Zeminsiz ve kenarsız metin girişi; içinde bulunduğu yüzeyle bütünleşir
/// (ör. komut satırı).
pub fn bare_input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        ..input(theme, status)
    }
}

/// Özellik ızgarası hücresindeki metin girişi: hücreyle bütünleşir, odakta
/// vurgu kenarı alır; geçersiz değerde kenar kırmızıdır.
pub fn cell(invalid: bool) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |theme, status| {
        let t = Tokens::of(theme);

        let edge = match status {
            _ if invalid => t.danger,
            text_input::Status::Focused { .. } => t.accent,
            _ => Color::TRANSPARENT,
        };

        text_input::Style {
            background: Background::Color(Color::TRANSPARENT),
            border: Border {
                color: edge,
                width: 1.0,
                radius: RADIUS.into(),
            },
            ..input(theme, status)
        }
    }
}

/// Çok satırlı metin alanı; odaklanınca kenar vurgu rengini alır.
pub fn text_area(theme: &Theme, status: text_editor::Status) -> text_editor::Style {
    let t = Tokens::of(theme);

    let edge = match status {
        text_editor::Status::Focused { .. } => t.accent,
        text_editor::Status::Hovered => t.muted,
        text_editor::Status::Active | text_editor::Status::Disabled => t.border,
    };

    text_editor::Style {
        background: Background::Color(t.field),
        border: Border {
            color: edge,
            width: 1.0,
            radius: RADIUS.into(),
        },
        placeholder: t.muted.scale_alpha(0.75),
        value: t.text,
        selection: t.accent.scale_alpha(0.35),
    }
}

/// Özellik ızgarası hücresindeki açılır liste: kenarsız, üzerine gelince
/// belirginleşir.
pub fn cell_pick_list(theme: &Theme, status: PickListStatus) -> PickListStyle {
    let t = Tokens::of(theme);

    PickListStyle {
        background: Background::Color(Color::TRANSPARENT),
        border: Border {
            color: match status {
                PickListStatus::Hovered | PickListStatus::Opened { .. } => t.accent,
                PickListStatus::Active => Color::TRANSPARENT,
            },
            width: 1.0,
            radius: RADIUS.into(),
        },
        ..pick_list(theme, status)
    }
}

/// Açılır liste kutusu.
pub fn pick_list(theme: &Theme, status: PickListStatus) -> PickListStyle {
    let t = Tokens::of(theme);

    PickListStyle {
        text_color: t.text,
        placeholder_color: t.muted,
        handle_color: t.muted,
        background: Background::Color(t.field),
        border: Border {
            color: match status {
                PickListStatus::Hovered | PickListStatus::Opened { .. } => t.accent,
                PickListStatus::Active => t.border,
            },
            width: 1.0,
            radius: RADIUS.into(),
        },
    }
}

/// Açılır listenin seçenek menüsü.
pub fn menu(theme: &Theme) -> overlay::menu::Style {
    let t = Tokens::of(theme);

    overlay::menu::Style {
        background: Background::Color(t.popover),
        border: Border {
            color: t.border,
            width: 1.0,
            radius: RADIUS.into(),
        },
        text_color: t.text,
        selected_text_color: t.on_accent,
        selected_background: Background::Color(t.accent),
        shadow: Shadow {
            color: t.shadow(),
            offset: Vector::new(0.0, 3.0),
            blur_radius: 10.0,
        },
    }
}

/// 1 piksellik bölücü çizgi.
pub fn hairline(theme: &Theme) -> rule::Style {
    rule::Style {
        color: Tokens::of(theme).border,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}

/// Ağaçtaki girinti çizgisi: bölücüden silik.
pub fn guide(theme: &Theme) -> rule::Style {
    rule::Style {
        color: Tokens::of(theme).border.scale_alpha(0.7),
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}

/// İnce, dikey kaydırma çubuğu.
pub fn thin_scrollbar() -> scrollable::Direction {
    scrollable::Direction::Vertical(scrollable::Scrollbar::new().width(6).scroller_width(6))
}

/// İnce dikey kaydırma çubuğu, içeriğin yanında: içerik yalnız kayarken
/// çubuğa yer açar, çubuk hiçbir zaman yazının ya da denetimin üstüne
/// binmez. Pencere gövdeleri ve sağ kenarına dek dolu listeler için.
pub fn body_scrollbar() -> scrollable::Direction {
    scrollable::Direction::Vertical(
        scrollable::Scrollbar::new()
            .width(6)
            .scroller_width(6)
            .spacing(8),
    )
}
