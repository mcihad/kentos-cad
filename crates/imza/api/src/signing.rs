//! Servisin imza koşusu: paylaşılan motoru ([`kentos_imza_signature::sign_batch`]) servisin deposuna ve olay
//! yayınına bağlar (docs/adr/9002 §4). Döngünün kendisi motordadır; masaüstü aynı motoru süreç içinde yürütür.

use std::sync::Arc;

use kentos_imza_core::ImzaError;
use kentos_imza_domain::{
    BatchStatus, CertificateInfo, FileStatus, PackageStatus, SignatureResult, SigningBatch,
};
use kentos_imza_signature::{Choice, RunSink, Token, sign_batch};

use crate::state::AppState;

/// Bir imzalama koşusunun girdileri.
///
/// **`pin` bu yapıda yaşar ama hiçbir yere kopyalanmaz**: koşu bitince düşer, `Debug` çıktısına girmez (`Debug` elle
/// yazıldı). Türetilmiş bir `Debug` PIN'i olduğu gibi basardı ve `tracing::debug!(?run)` gibi tek bir satır onu loga
/// düşürürdü.
pub struct SignRun {
    pub batch_id: String,
    pub slot_id: u64,
    pub certificate_id: String,
    pub pin: String,
}

impl std::fmt::Debug for SignRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignRun")
            .field("batch_id", &self.batch_id)
            .field("slot_id", &self.slot_id)
            .field("certificate_id", &self.certificate_id)
            .field("pin", &"<gizlendi>")
            .finish()
    }
}

/// Motorun adımlarını servisin deposuna ve olay yayınına yazan çıkış.
///
/// Kart çağrıları uzun sürer; depo kilidi yalnız tek bir adımın süresince tutulur, imzalama boyunca DEĞİL: kilidi
/// tutarak imzalamak, WebSocket olaylarını yayınlayan yolu da bloke ederdi.
struct ServiceSink<'a> {
    state: &'a AppState,
    batch: &'a SigningBatch,
}

impl RunSink for ServiceSink<'_> {
    fn batch_status(&mut self, to: BatchStatus) -> Result<(), ImzaError> {
        self.state.transition(&self.batch.id, to)
    }

    fn package_status(&mut self, package_id: &str, to: PackageStatus) -> Result<(), ImzaError> {
        self.state
            .set_package_status(&self.batch.id, package_id, to)
    }

    fn file_status(&mut self, file_id: &str, to: FileStatus) -> Result<(), ImzaError> {
        self.state.set_file_status(&self.batch.id, file_id, to)
    }

    fn signed(&mut self, file_id: &str, result: SignatureResult) -> Result<(), ImzaError> {
        self.state.put_result(&self.batch.id, file_id, result)
    }

    fn signer(&mut self, certificate: &CertificateInfo) {
        let _ = self.state.set_signer(&self.batch.id, certificate);
    }

    fn last_error(&mut self, error: ImzaError) {
        let _ = self.state.set_last_error(&self.batch.id, error);
    }

    fn event(&mut self, event_type: &str, payload: serde_json::Value) {
        self.state.emit(event_type, Some(self.batch), payload);
    }
}

/// **Batch'i baştan sona imzalar.** Bloke eden bir iştir — `spawn_blocking` içinde koşar.
///
/// # Errors
/// Batch bulunamazsa ya da akış başlamadan durursa (sertifika yok, süresi geçmiş, PIN, kart) `ImzaError`.
pub fn run(state: &AppState, token: &Arc<dyn Token>, run: SignRun) -> Result<(), ImzaError> {
    let batch = state.batch(&run.batch_id)?.batch;
    let mut sink = ServiceSink {
        state,
        batch: &batch,
    };
    sign_batch(
        &batch,
        token.as_ref(),
        &Choice {
            slot_id: run.slot_id,
            certificate_id: run.certificate_id.clone(),
        },
        &run.pin,
        &mut sink,
    )
}
