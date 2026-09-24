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
