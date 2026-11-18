//! Replay koruması: aynı imza isteği ikinci kez kullanılamaz (docs/adr/9002 §5).
//!
//! İkinci kullanım `KSI-1503 REQUEST_REPLAY_DETECTED` üretir.
//!
//! **Neden gerekli:** imzalı zarf ([`crate::signed_request`]) isteğin KentOS
//! tarafından yetkilendirildiğini kanıtlar, ama **kaç kez kullanılabileceğini**
//! söylemez. Zarfı bir kez yakalayan kötü niyetli bir sayfa, süresi dolana kadar
//! aynı imzayı tekrar tekrar attırabilirdi — ve her seferinde geçerli bir imza
//! üretilirdi.
//!
//! **`transactionId` VE `nonce` — ikisi birden**. `nonce` tek kullanımı,
//! `transactionId` ise aynı işlemin ikinci bir zarfla yeniden açılmasını engeller.

use std::collections::HashMap;

use kentos_imza_core::{ErrorCode, ImzaError};

/// Görülmüş istekleri tutan önbellek.
///
/// **Bellekte** tutuluyor ve bu bir sınırdır: servis yeniden başlarsa geçmiş
/// unutulur. Kalıcı bir `replay cache`/`nonce cache` henüz yok (docs/adr/9002 §7); bugünkü koruma, servis ayakta
/// olduğu sürece geçerli. Zarfın kendi süresi (`expiresAt`, varsayılan 15 dakika) pencereyi zaten daraltır.
#[derive(Debug, Default)]
pub struct ReplayGuard {
    /// nonce → ilk görüldüğü an (RFC 3339). Değer, temizlik için tutuluyor.
    nonces: HashMap<String, String>,
    transactions: HashMap<String, String>,
}

impl ReplayGuard {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// İsteği **tüketir**: ilk kez görülüyorsa kaydeder, ikinci kez görülüyorsa hata.
    ///
    /// Ad bilerek `consume`: `check()` gibi bir ad, çağıranın sonucu okuyup
    /// kaydetmeyi unutmasına açık kapı bırakırdı — ve unutulan tek çağrı, replay
    /// korumasının hiç olmaması demektir.
    ///
    /// # Errors
    /// Daha önce görülmüşse `KSI-1503`.
    pub fn consume(
        &mut self,
        transaction_id: &str,
        nonce: &str,
        seen_at: &str,
    ) -> Result<(), ImzaError> {
        if let Some(ilk) = self.nonces.get(nonce) {
            return Err(ImzaError::new(
                ErrorCode::RequestReplayDetected,
                format!("nonce {nonce} ilk kez {ilk} tarihinde kullanılmış"),
            ));
        }
        if let Some(ilk) = self.transactions.get(transaction_id) {
            return Err(ImzaError::new(
                ErrorCode::RequestReplayDetected,
                format!("transactionId {transaction_id} ilk kez {ilk} tarihinde kullanılmış"),
            ));
        }
        self.nonces.insert(nonce.to_owned(), seen_at.to_owned());
        self.transactions
            .insert(transaction_id.to_owned(), seen_at.to_owned());
        Ok(())
    }

    /// Belirtilen andan **önce** görülmüş kayıtları atar (saklama süresi).
    ///
    /// Temizlik olmadan önbellek sonsuza kadar büyür; ama **çok erken** temizlik
    /// replay penceresini açar. Eşik, isteklerin azami ömründen (`expiresAt`)
    /// büyük olmalı — daha kısa bir eşik, süresi dolmamış bir zarfın yeniden
    /// kullanılabilmesi demektir.
    pub fn prune_before(&mut self, cutoff_rfc3339: &str) {
        self.nonces.retain(|_, t| t.as_str() >= cutoff_rfc3339);
        self.transactions
            .retain(|_, t| t.as_str() >= cutoff_rfc3339);
    }

    /// İzlenen kayıt sayısı — tanılama için.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nonces.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nonces.is_empty()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test içinde başarısızlık zaten çökmeli")]
mod tests {
    use super::*;

    #[test]
    fn ilk_kullanim_gecer_ikincisi_1503() {
        let mut g = ReplayGuard::new();
        assert!(g.consume("tx-1", "n-1", "2026-08-28T10:00:00Z").is_ok());

        let hata = g
            .consume("tx-1", "n-1", "2026-08-28T10:00:05Z")
            .unwrap_err();
        assert_eq!(hata.code, "KSI-1503");
        // Tekrar denemek İŞE YARAMAZ: aynı zarf hiçbir zaman ikinci kez geçmez.
        assert!(!hata.retryable);
    }

    #[test]
    /// **Yeni nonce ama AYNI transaction** da reddedilir.
    fn ayni_islem_yeni_nonce_ile_de_acilamaz() {
        // Yalnız nonce izlenseydi, backend'i kandırıp aynı işlem için ikinci bir
        //    zarf ürettiren biri imzayı tekrarlatabilirdi.
        let mut g = ReplayGuard::new();
        g.consume("tx-1", "n-1", "2026-08-28T10:00:00Z").unwrap();
        let hata = g
            .consume("tx-1", "n-2", "2026-08-28T10:00:05Z")
            .unwrap_err();
        assert_eq!(hata.code, "KSI-1503");
        assert!(hata.technical_message.contains("transactionId"));
    }

    #[test]
    fn farkli_islem_farkli_nonce_gecer() {
        let mut g = ReplayGuard::new();
        g.consume("tx-1", "n-1", "2026-08-28T10:00:00Z").unwrap();
        assert!(g.consume("tx-2", "n-2", "2026-08-28T10:00:01Z").is_ok());
        assert_eq!(g.len(), 2);
    }

    #[test]
    /// Temizlik eskileri atar, **yenileri korur**.
    fn temizlik_yalniz_eskileri_atar() {
        let mut g = ReplayGuard::new();
        g.consume("tx-eski", "n-eski", "2026-08-28T09:00:00Z")
            .unwrap();
        g.consume("tx-yeni", "n-yeni", "2026-08-28T11:00:00Z")
            .unwrap();

        g.prune_before("2026-08-28T10:00:00Z");
        assert_eq!(g.len(), 1);

        // Atılan yeniden kullanılabilir (penceresi geçti); korunan hâlâ reddedilir.
        assert!(
            g.consume("tx-eski", "n-eski", "2026-08-28T11:05:00Z")
                .is_ok()
        );
        assert!(
            g.consume("tx-yeni", "n-yeni", "2026-08-28T11:05:00Z")
                .is_err()
        );
    }
}
