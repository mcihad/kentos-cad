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

/// Giriş alanı, seçim kutusu, sayı girişi, parçalı seçim ve küçük düğme.
pub const CONTROL: f32 = 28.0;
/// Satır içi kontrol: tablo ve özellik hücresinde, araç çubuğunda.
pub const INLINE: f32 = 22.0;
/// İletişim kutusunun eylem düğmeleri (Tamam, Vazgeç).
pub const BUTTON: f32 = 32.0;

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

/// Geçerli yazı boyutunda iletişim düğmesinin yüksekliği.
pub fn button() -> f32 {
    typography::from_default(BUTTON)
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
