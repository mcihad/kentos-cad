//! Hareket: bileşenlerin kısa geçişleri ve azaltılmış hareket.
//!
//! Anahtarın kayması, dairesel menünün açılması ve mini araç çubuğunun
//! belirmesi gibi geçişler kısa sürer ve yalnızca kullanıcının bir eylemine
//! yanıt verir. Hareketi azaltmak isteyen kullanıcılar için (ya da
//! ekransız görüntüde, geçişler yarıda kalmasın diye) uygulama
//! [`set_reduced`] ile geçişleri kapatır; bileşenler son hâlleriyle
//! çizilir.
//!
//! ```ignore
//! kentos_rc::theme::motion::set_reduced(settings.reduce_motion);
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use iced::time::Instant;

static REDUCED: AtomicBool = AtomicBool::new(false);

/// Geçişleri kapatır ya da açar.
pub fn set_reduced(reduced: bool) {
    REDUCED.store(reduced, Ordering::Relaxed);
}

/// Geçişler kapalı mı.
pub fn reduced() -> bool {
    REDUCED.load(Ordering::Relaxed)
}

/// `start` anında başlayan geçişin `now` anındaki ilerlemesi: 0'dan 1'e,
/// yavaşlayarak biter. Hareket azaltılmışsa hep 1'dir.
pub fn progress(start: Instant, now: Instant, duration: Duration) -> f32 {
    if reduced() || duration.is_zero() {
        return 1.0;
    }

    ease(now.saturating_duration_since(start).as_secs_f32() / duration.as_secs_f32())
}

/// Yavaşlayarak biten eğri; `t` 0 ile 1 arasına sıkıştırılır.
pub fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);

    1.0 - (1.0 - t) * (1.0 - t)
}

/// Geçiş sürüyor mu: `start`tan bu yana `duration` geçmedi.
pub fn running(start: Instant, now: Instant, duration: Duration) -> bool {
    !reduced() && now.saturating_duration_since(start) < duration
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transitions_ease_out() {
        assert_eq!(ease(0.0), 0.0);
        assert!((ease(0.5) - 0.75).abs() < 1e-6);
        assert_eq!(ease(1.0), 1.0);
        assert_eq!(ease(2.0), 1.0);
        assert_eq!(ease(-1.0), 0.0);
    }
}
