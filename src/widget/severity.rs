//! Geri bildirimin önemi: bildirimler, uyarı şeritleri ve boş ya da hata
//! durumları aynı dört düzeyi ve aynı renkleri kullanır.

use iced::Color;

use crate::icon::{Icon, Tone};
use crate::theme::Tokens;

/// Bildirimin önemi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Severity {
    /// Bilgi: bir iş yapıldı, bir şey değişti.
    #[default]
    Info,
    /// Uzun süren ya da önemli bir iş başarıyla bitti.
    Success,
    /// Dikkat: iş yapıldı ama beklenenden farklı.
    Warning,
    /// Hata: iş yapılamadı.
    Error,
}

impl Severity {
    pub fn icon(self) -> Icon {
        match self {
            Severity::Info => Icon::Info,
            Severity::Success => Icon::Success,
            Severity::Warning => Icon::Warning,
            Severity::Error => Icon::Error,
        }
    }

    /// İkonun tonu.
    pub fn tone(self) -> Tone {
        match self {
            Severity::Info => Tone::Accent,
            Severity::Success => Tone::Success,
            Severity::Warning => Tone::Warning,
            Severity::Error => Tone::Danger,
        }
    }

    pub fn color(self, tokens: &Tokens) -> Color {
        match self {
            Severity::Info => tokens.accent,
            Severity::Success => tokens.success,
            Severity::Warning => tokens.warning,
            Severity::Error => tokens.danger,
        }
    }

    /// Ekran okuyucular ve günlükler için adı.
    pub fn label(self) -> &'static str {
        match self {
            Severity::Info => "Bilgi",
            Severity::Success => "Tamamlandı",
            Severity::Warning => "Uyarı",
            Severity::Error => "Hata",
        }
    }
}
