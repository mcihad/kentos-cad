//! KentOS'un marka renkleri (DESIGN.md §2, web'in `--c-brand*` jetonları):
//! logo ve yazısı her zaman laciverttir; kullanıcının seçtiği vurgu rengini
//! izlemez, çünkü markanın kendi rengidir.

use iced::Color;

use super::Mode;
use super::tokens::hex;

/// Markanın temadaki renkleri.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Brand {
    /// Logonun karosu: geçişin koyu ucu (`--c-brand`).
    pub tile: Color,
    /// Geçişin açık ucu ve ölçme noktasının içi (`--c-brand-hi`).
    pub hi: Color,
    /// Karonun üstündeki K (`--c-brand-ink`).
    pub ink: Color,
    /// Marka yazısı: koyu temada açık lacivert, açıkta lacivert (`--c-brand-text`).
    pub text: Color,
}

/// Markanın renkleri; web'de olmayan gece ve yüksek karşıtlık koyu temanınkini alır.
pub const fn brand(mode: Mode) -> Brand {
    match mode {
        Mode::Light => Brand {
            tile: hex(0x1f4a96),
            hi: hex(0x3a6ccc),
            ink: hex(0xffffff),
            text: hex(0x1f4a96),
        },
        Mode::Dark | Mode::Night | Mode::HighContrast => Brand {
            tile: hex(0x1f4a96),
            hi: hex(0x4677d6),
            ink: hex(0xffffff),
            text: hex(0x8fb3f5),
        },
    }
}
