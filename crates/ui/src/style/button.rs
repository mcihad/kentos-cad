//! Düğme stilleri.

use iced::widget::button::{Status, Style};
use iced::{Background, Border, Color, Theme, border};

use crate::theme::Tokens;

/// Düğmelerin, giriş alanlarının ve menü satırlarının köşe yarıçapı:
/// biçimin `sm` kademesi (Görünüm → Köşeler, [`crate::theme::shape`]).
pub fn radius() -> f32 {
    crate::theme::shape::sm()
}

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
            radius: radius().into(),
        },
    )
}

/// Şerit düğmesinin durumu (DESIGN.md §7.3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Ribbon {
    #[default]
    Idle,
    /// Açık bir anahtar (ör. kenet): yumuşak vurgu zemini.
    On,
    /// Çalışan araç: dolu vurgu; vurgu yalnız onda dolu olur.
    Running,
}

/// Şerit düğmesi: çerçevesizdir. İkon düğmenin metin rengini alır:
/// dinlenirken ikincil, üzerine gelince ana renk; etiket kendi rengini
/// taşır ([`ribbon_label`]). Çalışan araç dolu vurgu, açık anahtar yumuşak
/// vurgu zeminidir; devre dışı düğme sönüktür.
pub fn ribbon(state: Ribbon) -> impl Fn(&Theme, Status) -> Style {
    move |theme, status| {
        let t = Tokens::of(theme);
        let hovered = is_hovered(status);

        let (background, icon) = match (state, status) {
            (_, Status::Disabled) => (Color::TRANSPARENT, t.disabled()),
            (Ribbon::Running, _) if hovered => (t.accent_hover, t.on_accent),
            (Ribbon::Running, _) => (t.accent, t.on_accent),
            (Ribbon::On, _) => (
                if hovered {
                    t.accent.scale_alpha(0.3)
                } else {
                    t.selection()
                },
                t.accent_hover,
            ),
            (Ribbon::Idle, Status::Pressed) => (t.layer(0.14), t.text),
            (Ribbon::Idle, Status::Hovered) => (t.layer(0.08), t.text),
            (Ribbon::Idle, _) => (Color::TRANSPARENT, t.muted),
        };

        style(background, icon, border::rounded(radius()))
    }
}

/// Şerit düğmesinin etiket rengi: vurgu zemininde vurgunun yazısı, devre
/// dışıyken sönük, yoksa ana metin.
pub fn ribbon_label(theme: &Theme, state: Ribbon, enabled: bool) -> Color {
    let t = Tokens::of(theme);

    match (state, enabled) {
        (_, false) => t.disabled(),
        (Ribbon::Running, true) => t.on_accent,
        _ => t.text,
    }
}

/// Birincil eylem: iletişim kutusunu onaylar, uygulamadan çıkar.
pub fn primary(theme: &Theme, status: Status) -> Style {
    let t = Tokens::of(theme);

    let (background, text) = match status {
        Status::Active => (t.accent, t.on_accent),
        Status::Hovered | Status::Pressed => (t.accent_hover, t.on_accent),
        Status::Disabled => (t.accent.scale_alpha(0.3), t.on_accent.scale_alpha(0.5)),
    };

    style(background, text, border::rounded(radius()))
}

/// Yıkıcı birincil eylem (ör. "Tümünü sil"): kırmızı zemin.
pub fn danger(theme: &Theme, status: Status) -> Style {
    let t = Tokens::of(theme);

    let (background, text) = match status {
        Status::Active => (t.danger, t.on_accent),
        Status::Hovered | Status::Pressed => (lighten(t.danger, 0.12), t.on_accent),
        Status::Disabled => (t.danger.scale_alpha(0.3), t.on_accent.scale_alpha(0.5)),
    };

    style(background, text, border::rounded(radius()))
}

/// Bir şeyi bırakan ya da silen, birincil olmayan eylem (web'in
/// `.btn--danger`'ı): ikincil düğme gibi kenarlı, yazısı tehlike renginde;
/// üzerine gelince kenarı da. Hiçbir zaman vurgu renginde değildir.
pub fn danger_outline(theme: &Theme, status: Status) -> Style {
    let t = Tokens::of(theme);

    let (background, text, edge) = match status {
        Status::Active => (t.surface_alt, t.danger, t.border),
        Status::Hovered | Status::Pressed => (t.surface_hover, t.danger, t.danger),
        Status::Disabled => (t.surface_alt, t.disabled(), t.border),
    };

    style(
        background,
        text,
        Border {
            color: edge,
            width: 1.0,
            radius: radius().into(),
        },
    )
}

/// Özellikler penceresinin solundaki bölüm listesi: seçili bölüm seçim
/// zemininde; üzerine gelince hafif bir katman.
pub fn navigation(selected: bool) -> impl Fn(&Theme, Status) -> Style {
    move |theme, status| {
        let t = Tokens::of(theme);

        let background = match (selected, status) {
            (true, _) => t.selection(),
            (false, Status::Hovered) => t.layer(0.05),
            (false, Status::Pressed) => t.layer(0.09),
            (false, _) => Color::TRANSPARENT,
        };

        style(background, t.text, border::rounded(radius()))
    }
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

/// İkincil eylem (web'in `.btn`'i): alan zemininde, belirgin kenarlı düğme;
/// üzerine gelince zemin açılır, kenar sönük yazının tonuna döner.
pub fn secondary(theme: &Theme, status: Status) -> Style {
    let t = Tokens::of(theme);

    let (background, text, edge) = match status {
        Status::Active => (t.field, t.text, t.border_strong()),
        Status::Hovered | Status::Pressed => (t.surface_hover, t.text, t.muted.scale_alpha(0.6)),
        Status::Disabled => (t.field, t.disabled(), t.border),
    };

    style(
        background,
        text,
        Border {
            color: edge,
            width: 1.0,
            radius: radius().into(),
        },
    )
}

/// Ağaç satırının göz, kilit gibi düğmeleri (web'in `.ibtn--row`'u):
/// dinlenirken üçüncül tonda ve soluk, öne çıkan durumda (gizli, kilitli)
/// ikincil tonda; üzerine gelince yazının renginde.
pub fn row_toggle(pressed: bool) -> impl Fn(&Theme, Status) -> Style {
    move |theme, status| {
        let t = Tokens::of(theme);

        Style {
            background: None,
            text_color: match status {
                Status::Hovered | Status::Pressed => t.text,
                Status::Disabled => t.disabled(),
                Status::Active if pressed => t.muted,
                Status::Active => t.faint.scale_alpha(0.55),
            },
            ..Style::default()
        }
    }
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
    let hovered = is_hovered(status);

    Style {
        background: hovered.then(|| t.layer(0.055).into()),
        text_color: if hovered { t.text } else { t.muted },
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
                radius: radius().into(),
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
                radius: crate::theme::shape::radius(3.0).into(),
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
                radius: radius().into(),
            },
        )
    }
}

/// Parçalı seçimin parçası (web'in `.seg__opt`'u): seçili parça gömük
/// yuvanın içinde kalkık durur (kalkık yüzey, belirgin kenar, hafif gölge);
/// öbürleri ikincil renkte yazılır, üzerine gelince hafif bir katman,
/// basılıyken biraz daha koyusunu alır.
pub fn segment(selected: bool) -> impl Fn(&Theme, Status) -> Style {
    move |theme, status| {
        let t = Tokens::of(theme);

        if selected {
            return Style {
                background: Some(Background::Color(t.raised())),
                text_color: t.text,
                border: Border {
                    color: t.border_strong(),
                    width: 1.0,
                    radius: radius().into(),
                },
                shadow: iced::Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, if t.is_dark { 0.3 } else { 0.1 }),
                    offset: iced::Vector::new(0.0, 1.0),
                    blur_radius: 2.0,
                },
                snap: true,
            };
        }

        unselected_segment(&t, status)
    }
}

/// Öne çıkan parçalı seçimin parçası (ör. 2B / 3B): seçili parça yumuşak
/// vurgu zemininde, vurgu kenarı ve vurgu yazısıyla; öbürleri
/// [`segment`] gibidir.
pub fn segment_accent(selected: bool) -> impl Fn(&Theme, Status) -> Style {
    move |theme, status| {
        let t = Tokens::of(theme);

        if selected {
            return Style {
                background: Some(Background::Color(t.selection())),
                text_color: t.accent_hover,
                border: Border {
                    color: t.accent_line(),
                    width: 1.0,
                    radius: radius().into(),
                },
                ..Style::default()
            };
        }

        unselected_segment(&t, status)
    }
}

fn unselected_segment(t: &Tokens, status: Status) -> Style {
    let (background, text) = match status {
        Status::Disabled => (Color::TRANSPARENT, t.disabled()),
        Status::Pressed => (t.layer(0.1), t.text),
        Status::Hovered => (t.layer(0.06), t.text),
        Status::Active => (Color::TRANSPARENT, t.muted),
    };

    style(background, text, border::rounded(radius()))
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

    style(
        background,
        text,
        border::rounded(crate::theme::shape::radius(3.0)),
    )
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
            border::rounded(crate::theme::shape::radius(3.0)),
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
            radius: crate::theme::shape::radius(3.0).into(),
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
                radius: crate::theme::shape::radius(3.0).into(),
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
            border::rounded(radius()),
        )
    }
}

/// Süzgeç hapı (ör. kitaplığın kategorileri): seçili değilken ince kenarlı
/// ve ikincil yazılı, seçiliyken yumuşak vurgu zemininde ve vurgu yazılı.
/// Hap köşeleri biçim ayarından etkilenmez (DESIGN.md §5.3).
pub fn chip(active: bool) -> impl Fn(&Theme, Status) -> Style {
    move |theme, status| {
        let t = Tokens::of(theme);
        let hovered = is_hovered(status);

        let (background, edge, text) = match (active, hovered) {
            (true, _) => (
                t.accent.scale_alpha(if hovered { 0.26 } else { 0.16 }),
                t.accent_line(),
                t.accent_hover,
            ),
            (false, true) => (t.surface_hover, t.border_strong(), t.text),
            (false, false) => (Color::TRANSPARENT, t.border, t.muted),
        };

        Style {
            background: Some(Background::Color(background)),
            text_color: text,
            border: Border {
                color: edge,
                width: 1.0,
                radius: 999.0.into(),
            },
            ..Style::default()
        }
    }
}

/// Kitaplık karosu (malzeme, 3B nesne): dinlenirken zeminsiz; üzerine
/// gelince hafif zemin ve belirgin kenar; seçiliyken yumuşak vurgu zemini ve
/// vurgu kenarı.
pub fn library_tile(selected: bool) -> impl Fn(&Theme, Status) -> Style {
    move |theme, status| {
        let t = Tokens::of(theme);
        let hovered = is_hovered(status);

        let (background, edge) = match (selected, hovered) {
            (true, _) => (t.selection(), t.accent_line()),
            (false, true) => (t.layer(0.05), t.border_strong()),
            (false, false) => (Color::TRANSPARENT, Color::TRANSPARENT),
        };

        Style {
            background: Some(Background::Color(background)),
            text_color: t.text,
            border: Border {
                color: edge,
                width: 1.0,
                radius: crate::theme::shape::md().into(),
            },
            ..Style::default()
        }
    }
}

/// Uygulama menüsünü açan marka düğmesi (web'in `.brand`'ı): zemini yok;
/// üzerine gelince ya da menü açıkken hafif bir katman. Yazısı kendi rengini
/// taşır; ok üçüncül tondadır.
pub fn brand(open: bool) -> impl Fn(&Theme, Status) -> Style {
    move |theme, status| {
        let t = Tokens::of(theme);

        style(
            if open || is_hovered(status) {
                t.layer(0.055)
            } else {
                Color::TRANSPARENT
            },
            t.faint,
            border::rounded(crate::theme::shape::md()),
        )
    }
}

/// Liste satırı: seçili ya da klavyeyle gelinen satır yumuşak vurgu
/// zemininde, üzerine gelinen hafif bir katmanla (web'in `--c-hover`'ı);
/// kenar yok.
pub fn list_item(highlighted: bool) -> impl Fn(&Theme, Status) -> Style {
    move |theme, status| {
        let t = Tokens::of(theme);

        let background = if highlighted {
            t.selection()
        } else if is_hovered(status) {
            t.layer(0.06)
        } else {
            Color::TRANSPARENT
        };

        style(background, t.text, border::rounded(radius()))
    }
}

/// Açılır listenin ya da menünün satırı: üzerine gelinen satır yumuşak
/// vurgu zemininde (web'in `.menu__item[data-active]`'i); geçerli değer
/// yanındaki işaretle belli olur, zeminle değil.
pub fn menu_row(theme: &Theme, status: Status) -> Style {
    let t = Tokens::of(theme);

    let background = if is_hovered(status) {
        t.selection()
    } else {
        Color::TRANSPARENT
    };

    style(background, t.text, border::rounded(radius()))
}
