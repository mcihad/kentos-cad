//! Metin renkleri.

use iced::Theme;
use iced::widget::text::Style;

use crate::theme::Tokens;

/// Birincil metin.
pub fn default(theme: &Theme) -> Style {
    Style {
        color: Some(Tokens::of(theme).text),
    }
}

/// İkincil metin: açıklamalar, meta bilgisi.
pub fn muted(theme: &Theme) -> Style {
    Style {
        color: Some(Tokens::of(theme).muted),
    }
}

/// Küçük notlar, alanların yardımı, birimler, sayılar: okunur kalan en sessiz
/// yazı tonu ([`Tokens::soft`]; docs/adr/0804).
pub fn faint(theme: &Theme) -> Style {
    Style {
        color: Some(Tokens::of(theme).soft()),
    }
}

/// Vurgu renginde metin (ör. açık olan çizim).
pub fn accent(theme: &Theme) -> Style {
    Style {
        color: Some(Tokens::of(theme).accent),
    }
}

/// Vurgu zemini üzerindeki metin.
pub fn on_accent(theme: &Theme) -> Style {
    Style {
        color: Some(Tokens::of(theme).on_accent),
    }
}

/// Devre dışı öğelerin metni.
pub fn disabled(theme: &Theme) -> Style {
    Style {
        color: Some(Tokens::of(theme).disabled()),
    }
}

/// Bilgi metni (web'in `--c-info`'su): ör. “Yeni sürüm var”.
pub fn info(theme: &Theme) -> Style {
    Style {
        color: Some(Tokens::of(theme).info),
    }
}

/// Uyarı metni (web'in `--c-warn`'u): ör. katmanın alanının kuralına uymayan değer.
pub fn warning(theme: &Theme) -> Style {
    Style {
        color: Some(Tokens::of(theme).warning),
    }
}

/// Hata ve uyarı metni.
pub fn danger(theme: &Theme) -> Style {
    Style {
        color: Some(Tokens::of(theme).danger),
    }
}
