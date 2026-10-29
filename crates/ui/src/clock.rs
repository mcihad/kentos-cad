//! Arayüzün saati: bileşenlerin bir basışı öncekiyle karşılaştırdığı an
//! (ör. tutamağın çift tıkı).
//!
//! Uygulamada sistemin saatidir. Olayları arayüze kendisi veren ekransız
//! görüntü ([`Snapshot`](crate::snapshot::Snapshot)) olayları verirken saati
//! kendi anında durdurur: testin hareketleri, makine ne kadar meşgul olursa
//! olsun, testin dediği aralıklarla gelir.
//!
//! ```ignore
//! let double = now().duration_since(last_press) <= DOUBLE_CLICK;
//! ```

use std::cell::Cell;

use iced::time::Instant;

thread_local! {
    /// The moment this thread's clock stands at while a host hands it events.
    static FROZEN: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// Şimdiki an: bu iş parçacığının saati durdurulmuşsa durduğu an, yoksa
/// sistemin saati.
pub fn now() -> Instant {
    FROZEN.get().unwrap_or_else(Instant::now)
}

/// Bu iş parçacığının saatini, dönen değer bırakılana dek `at` anında
/// durdurur; sonra saat önceki hâline döner.
pub fn freeze(at: Instant) -> Frozen {
    Frozen(FROZEN.replace(Some(at)))
}

/// Durdurulmuş saat; bırakılınca saat önceki hâline döner.
#[must_use = "the clock runs again when this is dropped"]
pub struct Frozen(Option<Instant>);

impl Drop for Frozen {
    fn drop(&mut self) {
        FROZEN.set(self.0);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn a_frozen_clock_stands_still_and_runs_again_when_let_go() {
        let at = Instant::now() + Duration::from_secs(60);
        {
            let _frozen = freeze(at);
            assert_eq!(now(), at);
            {
                let _inner = freeze(at + Duration::from_secs(1));
                assert_eq!(now(), at + Duration::from_secs(1));
            }
            assert_eq!(now(), at, "the outer moment again");
        }
        assert!(now() < at, "the system's clock again");
    }
}
