//! Kontrol ölçüleri: giriş alanlarının, seçim kutularının, parçalı
//! seçimlerin ve düğmelerin yükseklikleri tek yerden (web'in `.field`,
//! `.dropdown`, `.btn` ve `.field--inline` ölçüleri, DESIGN.md §5.2).
//!
//! ```text
//!  ┌──────────────────────┐ ┌──────────────────▾┐ ┌ Yeni │ Ekle ┐   28  kontrol
//!  └──────────────────────┘ └───────────────────┘ └──────────────┘
//!  ┌────────────────┐ ┌──────┐                                      22  satır içi
//!  └────────────────┘ └──────┘
//!  ┌──────────┐ ┌──────────┐                                        32  iletişim düğmesi
//!  └──────────┘ └──────────┘
//! ```
//!
//! Aynı satırda yan yana duran bir metin girişi, bir seçim kutusu ve bir
//! sayı girişi aynı yüksekliktedir; satır içi kademe (tablo ve özellik
//! hücresi, araç çubuğu) da kendi içinde öyledir. Ölçüler varsayılan yazı
//! boyutunda (13 piksel) verilir ve yazı boyutuyla büyür
//! ([`typography::from_default`]).
//!
//! iced'in metin girişi ve açılır listesi yüksekliklerini metnin satır
//! yüksekliğiyle iç boşluktan kurar; [`padding`] istenen yüksekliği veren
//! iç boşluğu hesaplar:
//!
//! ```ignore
//! text_input("Ad", &name)
//!     .size(typography::body())
//!     .padding(metrics::padding(metrics::control(), 8.0))
//!     .style(style::field::input)
//! ```

use iced::Padding;

use super::typography;

/// Bütün girdiler ve düğmeler: metin, açılır liste, sayı, tarih, renk, arama,
/// parçalı seçim, eylem düğmesi (docs/adr/0804). Girdilerin tek yüksekliği budur.
pub const CONTROL: f32 = 28.0;
/// Satırın içinde duran küçük düğme: tablo satırındaki ve durum çubuğundaki işler.
/// Girdi bu boyda olmaz.
pub const INLINE: f32 = 22.0;
/// Izgara, tablo, ağaç ve liste satırı (DESIGN.md §5.2'nin `--row-h`'ı): bütün
/// satırlar kontrolle aynı boyda, hücrenin düzenleyicisi satırı tam doldurur.
pub const ROW: f32 = CONTROL;
/// Girdideki yazının kenara uzaklığı.
pub const FIELD_PAD: f32 = 8.0;
/// Eylem düğmesindeki yazının kenara uzaklığı.
pub const ACTION_PAD: f32 = 12.0;
/// Hücredeki yazının hücrenin sol kenarına uzaklığı.
pub const CELL_PAD: f32 = 10.0;
/// Hücrenin sonundaki birimin ve okun hücrenin sağ kenarına uzaklığı.
pub const CELL_END: f32 = 8.0;

/// iced metninin varsayılan satır yüksekliği, yazı boyutunun katı.
const LINE_HEIGHT: f32 = 1.3;

/// Geçerli yazı boyutunda kontrol yüksekliği.
pub fn control() -> f32 {
    typography::from_default(CONTROL)
}

/// Geçerli yazı boyutunda satır içi kontrol yüksekliği.
pub fn inline() -> f32 {
    typography::from_default(INLINE)
}

/// Geçerli yazı boyutunda satır yüksekliği.
pub fn row() -> f32 {
    typography::from_default(ROW)
}

/// Geçerli yazı boyutunda hücre içi boşluğu.
pub fn cell_pad() -> f32 {
    typography::from_default(CELL_PAD)
}

/// Geçerli yazı boyutunda hücrenin sonundaki boşluk.
pub fn cell_end() -> f32 {
    typography::from_default(CELL_END)
}

/// Metin girişinin iç boşluğu: kontrol yüksekliğine getirir.
pub fn field() -> Padding {
    padding(control(), FIELD_PAD)
}

/// Eylem düğmesinin iç boşluğu: kontrol yüksekliğine getirir.
pub fn action() -> Padding {
    padding(control(), ACTION_PAD)
}

/// Satırın içinde duran küçük düğmenin iç boşluğu.
pub fn action_small() -> Padding {
    padding(inline(), FIELD_PAD)
}

/// Yalnız ikonlu düğmenin iç boşluğu: `glyph` boyundaki ikonla kontrol yüksekliğinde bir kare.
pub fn icon(glyph: f32) -> Padding {
    Padding::new(((control() - glyph) / 2.0).max(0.0))
}

/// Satırın içinde duran, yalnız ikonlu küçük düğmenin iç boşluğu.
pub fn icon_small(glyph: f32) -> Padding {
    Padding::new(((inline() - glyph) / 2.0).max(0.0))
}

/// Gövde metniyle yazılan bir metin girişini ya da açılır listeyi
/// `height` yüksekliğine getiren iç boşluk; yanlarda `horizontal`.
pub fn padding(height: f32, horizontal: f32) -> Padding {
    padding_for(height, typography::body(), horizontal)
}

/// [`padding`] gibi; metin `size` boyutundaysa.
pub fn padding_for(height: f32, size: f32, horizontal: f32) -> Padding {
    let vertical = ((height - size * LINE_HEIGHT) / 2.0).max(0.0);

    Padding {
        top: vertical,
        bottom: vertical,
        left: horizontal,
        right: horizontal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_text_field_reaches_the_control_height() {
        let padding = padding_for(28.0, 13.0, 8.0);

        assert!((13.0 * LINE_HEIGHT + padding.top + padding.bottom - 28.0).abs() < 1e-4);
        assert_eq!(padding.left, 8.0);
    }

    #[test]
    fn text_taller_than_the_control_gets_no_padding() {
        assert_eq!(padding_for(14.0, 18.0, 4.0).top, 0.0);
    }
}
