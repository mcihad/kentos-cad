//! Servis durumu: batch deposu, olay yayını, kart, güven anahtarı (docs/adr/9002 §4).
//!
//! Her şey **bellekte**: kalıcı depolama henüz yok (docs/adr/9002 §7). Servis yeniden başlarsa açık batch'ler kaybolur;
//! sunucudaki kayıt kalıcıdır, yani imza kaybolmaz — yalnız yarım kalan oturum kaybolur. Çökme sonrası kurtarma bu
//! yüzden henüz yok.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use kentos_imza_audit::AuditChain;
use kentos_imza_core::{ErrorCode, ImzaError};
use kentos_imza_domain::{
    BatchRecord, BatchStatus, CertificateInfo, FileStatus, PackageStatus, SignatureResult,
    SigningBatch,
};
use kentos_imza_security::ReplayGuard;
use kentos_imza_signature::{SignatureProviderRegistry, Token};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

pub use kentos_imza_signature::verify::now_rfc3339;

/// PIN'in nereden alınacağı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PinEntryMode {
    /// PIN'i **ajanın kendi penceresi** alır; tarayıcı hiç görmez. KentOS CAD'de varsayılan (docs/adr/9002 §4).
    Native,
    /// PIN'i tarayıcı alır ve yalnız geri döngü üzerinden yollar.
    ///
    /// **Sapma**: PIN tarayıcıdan geçer. Yalnız geliştirme içindir ve `/info` bunu `insecurePinEntry: true` ile
    /// açıkça bildirir — gizlenen bir sapma, sapma değil kusurdur.
    Client,
}

/// Yayınlanan olay.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub event_id: String,
    pub event_type: String,
    pub timestamp: String,
    pub transaction_id: Option<String>,
    pub batch_id: Option<String>,
    pub package_id: Option<String>,
    pub file_id: Option<String>,
    pub signature_method: Option<String>,
    pub client_type: Option<String>,
    pub payload: serde_json::Value,
}

/// Paylaşılan servis durumu.
#[derive(Clone)]
pub struct AppState {
    inner: Arc<Inner>,
}

struct Inner {
    batches: Mutex<HashMap<String, BatchRecord>>,
    replay: Mutex<ReplayGuard>,
    audit: Mutex<AuditChain>,
    providers: SignatureProviderRegistry,
    events: broadcast::Sender<Event>,
    pin_entry: PinEntryMode,
    /// KentOS sunucusunun açık anahtarı — `None` ise batch oluşturma REDDEDİLİR.
    trust: Option<kentos_imza_security::TrustConfig>,
    /// Kart: gerçek PKCS#11 modülü ya da (yalnız deneme derlemesinde) yazılım kartı.
    token: Option<Arc<dyn Token>>,

    /// PIN bekleyen batch'ler — **`native` kipin kalbi**.
    ///
    /// `POST …/sign` bir kanal açar, `pin_required` yayınlar ve **ajanın** PIN'i getirmesini bekler. Ajan
    /// `POST …/pin` çağırdığında kanal kapanır ve imzalama başlar.
    ///
    /// **PIN burada SAKLANMAZ** — kanaldan geçer, imzalama fonksiyonuna girer ve düşer. Bu harita yalnız *"kim
    /// bekliyor"* bilgisini tutar.
    pin_bekleyenler: Mutex<HashMap<String, tokio::sync::oneshot::Sender<PinCevabi>>>,

    /// Ajanın son işareti — `interactiveAgent` bundan türer.
    ///
    /// Bir **damga**, bir bayrak değil: ajan çökerse bayrak `true` kalırdı ve istemci hiç açılmayacak bir pencereyi
    /// beklerdi. Damga eskiyince ajan yok sayılır.
    agent_son_gorulme: Mutex<Option<std::time::Instant>>,
}

/// Ajanın PIN penceresinden dönen cevap.
///
/// `Debug` **elle yazıldı**: türetilmiş bir `Debug` PIN'i olduğu gibi basar ve tek bir `tracing::debug!` satırı onu
/// loga düşürür.
pub enum PinCevabi {
    Girildi {
        pin: String,
        /// Kullanıcı ajan penceresinde **başka bir cihaz/sertifika seçtiyse**.
        ///
        /// Oradaki seçim **bağlayıcıdır**: kullanıcının imzayı ONAYLADIĞI yüzey orasıdır. Tarayıcının söylediğine
        /// körlemesine uymak, güvenilir pencerede gösterilen imzalayan ile gerçekte imzalayanın ayrışmasına kapı
        /// açardı.
        secim: Option<CihazSecimi>,
    },
    /// Kullanıcı vazgeçti: tekrar DENENMEZ.
    Iptal,
}

/// Ajan penceresinden gelen cihaz/sertifika seçimi.
#[derive(Debug, Clone)]
pub struct CihazSecimi {
    pub slot_id: u64,
    pub certificate_id: String,
}

impl std::fmt::Debug for PinCevabi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Girildi { secim, .. } => {
                write!(
                    f,
                    "PinCevabi::Girildi {{ pin: <gizlendi>, secim: {secim:?} }}"
                )
            }
            Self::Iptal => f.write_str("PinCevabi::Iptal"),
        }
    }
}

/// Ajanın "yaşıyorum" işareti bu süre içinde gelmemişse ajan YOK sayılır.
const AGENT_TAZELIK: std::time::Duration = std::time::Duration::from_secs(15);

/// PIN penceresi bu süre içinde cevap vermezse imza `KSI-1301` ile düşer.
///
/// Süre **cömert olmalı**: kullanıcı pencereyi görmek için başka bir uygulamadan çıkmak zorunda kalabilir. Ama sonsuz
/// olamaz — cevapsız bir batch, karta dokunmadan asılı kalırdı.
pub const PIN_BEKLEME: std::time::Duration = std::time::Duration::from_secs(180);

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("pin_entry", &self.inner.pin_entry)
            .field("trust_configured", &self.inner.trust.is_some())
            .field("token_loaded", &self.inner.token.is_some())
            .finish_non_exhaustive()
    }
}

impl AppState {
    #[must_use]
    pub fn new(
        providers: SignatureProviderRegistry,
        pin_entry: PinEntryMode,
        trust: Option<kentos_imza_security::TrustConfig>,
        token: Option<Arc<dyn Token>>,
    ) -> Self {
        // Kapasite 256: yavaş bir WebSocket istemcisi yayını TIKAMASIN. Dolduğunda en eski olay düşer ve o istemci
        //    `Lagged` alır — imza akışı DEVAM EDER. Sınırsız bir kanal, tarayıcı sekmesi donduğunda belleği şişirirdi.
        let (events, _) = broadcast::channel(256);
        Self {
            inner: Arc::new(Inner {
                batches: Mutex::new(HashMap::new()),
                replay: Mutex::new(ReplayGuard::new()),
                audit: Mutex::new(AuditChain::new()),
                providers,
                events,
                pin_entry,
                trust,
                token,
                pin_bekleyenler: Mutex::new(HashMap::new()),
                agent_son_gorulme: Mutex::new(None),
            }),
        }
    }

    #[must_use]
    pub fn pin_entry(&self) -> PinEntryMode {
        self.inner.pin_entry
    }

    #[must_use]
    pub fn trust_configured(&self) -> bool {
        self.inner.trust.is_some()
    }

    #[must_use]
    pub fn trust(&self) -> Option<&kentos_imza_security::TrustConfig> {
        self.inner.trust.as_ref()
    }

    #[must_use]
    pub fn providers(&self) -> &SignatureProviderRegistry {
        &self.inner.providers
    }

    #[must_use]
    pub fn token(&self) -> Option<&Arc<dyn Token>> {
        self.inner.token.as_ref()
    }

    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.inner.events.subscribe()
    }

    /// Olay yayınlar ve **denetim zincirine** yazar.
    ///
    /// İkisi birlikte: yayınlanan ama denetime girmeyen bir olay, sonradan *"bu ne zaman oldu"* sorusunu cevapsız
    /// bırakırdı.
    pub fn emit(&self, event_type: &str, batch: Option<&SigningBatch>, payload: serde_json::Value) {
        let now = now_rfc3339();
        let event = Event {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type: event_type.to_owned(),
            timestamp: now.clone(),
            transaction_id: batch.map(|b| b.transaction_id.clone()),
            batch_id: batch.map(|b| b.id.clone()),
            package_id: payload
                .get("packageId")
                .and_then(|v| v.as_str())
                .map(str::to_owned),
            file_id: payload
                .get("fileId")
                .and_then(|v| v.as_str())
                .map(str::to_owned),
            signature_method: batch.map(|b| format!("{:?}", b.signature_method)),
            client_type: batch.map(|b| format!("{:?}", b.client_type)),
            payload: payload.clone(),
        };

        if let Some(b) = batch
            && let Ok(mut zincir) = self.inner.audit.lock()
        {
            zincir.append(
                event.event_id.clone(),
                b.transaction_id.clone(),
                b.id.clone(),
                event.package_id.clone(),
                event.file_id.clone(),
                format!("{:?}", b.signature_method),
                format!("{:?}", b.client_type),
                event_type,
                now,
                payload,
            );
        }

        // Abone yoksa `send` hata döner ve bu NORMALDİR: kimse dinlemiyorsa olay kaybolur. Denetim zinciri zaten
        //    yazıldı.
        let _ = self.inner.events.send(event);
    }

    /// Batch ekler; replay kapısından geçirir.
    ///
    /// # Errors
    /// Aynı işlem/nonce ikinci kez → `KSI-1503`.
    pub fn insert_batch(&self, batch: SigningBatch, nonce: &str) -> Result<(), ImzaError> {
        {
            let mut g = self
                .inner
                .replay
                .lock()
                .map_err(|_| kilit_hatasi("replay"))?;
            g.consume(&batch.transaction_id, nonce, &now_rfc3339())?;
        }
        let mut b = self
            .inner
            .batches
            .lock()
            .map_err(|_| kilit_hatasi("batch"))?;
        b.insert(batch.id.clone(), BatchRecord::new(batch));
        Ok(())
    }

    /// Batch'i okur (kopyası).
    ///
    /// # Errors
    /// Yoksa `KSI-1500`.
    pub fn batch(&self, id: &str) -> Result<BatchRecord, ImzaError> {
        self.inner
            .batches
            .lock()
            .map_err(|_| kilit_hatasi("batch"))?
            .get(id)
            .cloned()
            .ok_or_else(|| {
                ImzaError::new(ErrorCode::RequestError, format!("batch {id} bulunamadı"))
            })
    }

    /// Batch üzerinde değişiklik yapar.
    ///
    /// # Errors
    /// Batch yoksa `KSI-1500`; `f`in hatası.
    pub fn update_batch<T>(
        &self,
        id: &str,
        f: impl FnOnce(&mut BatchRecord) -> Result<T, ImzaError>,
    ) -> Result<T, ImzaError> {
        let mut kilit = self
            .inner
            .batches
            .lock()
            .map_err(|_| kilit_hatasi("batch"))?;
        let girdi = kilit.get_mut(id).ok_or_else(|| {
            ImzaError::new(ErrorCode::RequestError, format!("batch {id} bulunamadı"))
        })?;
        f(girdi)
    }

    /// Batch durumunu **durum makinesinden geçirerek** değiştirir; doğrudan atama YOK.
    ///
    /// # Errors
    /// Geçiş izinli değilse `KSI-1000`.
    pub fn transition(&self, id: &str, to: BatchStatus) -> Result<(), ImzaError> {
        self.update_batch(id, |r| r.transition(to))
    }

    /// Bir paketin durumunu değiştirir.
    ///
    /// # Errors
    /// Batch/paket yoksa `KSI-1500`; geçiş izinli değilse `KSI-1000`.
    pub fn set_package_status(
        &self,
        batch_id: &str,
        package_id: &str,
        to: PackageStatus,
    ) -> Result<(), ImzaError> {
        self.update_batch(batch_id, |r| r.set_package_status(package_id, to))
    }

    /// Bir dosyanın durumunu değiştirir.
    ///
    /// # Errors
    /// Batch/dosya yoksa `KSI-1500`; geçiş izinli değilse `KSI-1000`.
    pub fn set_file_status(
        &self,
        batch_id: &str,
        file_id: &str,
        to: FileStatus,
    ) -> Result<(), ImzaError> {
        self.update_batch(batch_id, |r| r.set_file_status(file_id, to))
    }

    /// **Doğrulanmış** bir imza sonucunu kaydeder; yalnız motor çağırır.
    ///
    /// # Errors
    /// Batch yoksa `KSI-1500`.
    pub fn put_result(
        &self,
        batch_id: &str,
        file_id: &str,
        result: SignatureResult,
    ) -> Result<(), ImzaError> {
        self.update_batch(batch_id, |r| {
            r.results.insert(file_id.to_owned(), result);
            Ok(())
        })
    }

    /// İmzalamada kullanılan sertifikayı kaydeder (sonuçla teslim edilir).
    ///
    /// # Errors
    /// Batch yoksa `KSI-1500`.
    pub fn set_signer(
        &self,
        batch_id: &str,
        certificate: &CertificateInfo,
    ) -> Result<(), ImzaError> {
        self.update_batch(batch_id, |r| {
            r.signer = Some(certificate.clone());
            Ok(())
        })
    }

    /// Kullanıcıya gösterilecek son hatayı saklar.
    ///
    /// # Errors
    /// Batch yoksa `KSI-1500`.
    pub fn set_last_error(&self, batch_id: &str, error: ImzaError) -> Result<(), ImzaError> {
        self.update_batch(batch_id, |r| {
            r.last_error = Some(error);
            Ok(())
        })
    }

    /// **PIN bekleme kanalı açar** ve alıcısını döndürür.
    ///
    /// # Errors
    /// Kilit zehirlenmişse `KSI-1000`.
    pub fn pin_bekle(
        &self,
        batch_id: &str,
    ) -> Result<tokio::sync::oneshot::Receiver<PinCevabi>, ImzaError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.inner
            .pin_bekleyenler
            .lock()
            .map_err(|_| kilit_hatasi("pin"))?
            .insert(batch_id.to_owned(), tx);
        Ok(rx)
    }

    /// Bekleyen bir batch'e PIN cevabını **teslim eder**.
    ///
    /// # Errors
    /// O batch için bekleyen yoksa `KSI-1500` — geç kalmış ya da yanlış bir cevap sessizce yutulmaz.
    pub fn pin_ver(&self, batch_id: &str, cevap: PinCevabi) -> Result<(), ImzaError> {
        let tx = self
            .inner
            .pin_bekleyenler
            .lock()
            .map_err(|_| kilit_hatasi("pin"))?
            .remove(batch_id)
            .ok_or_else(|| {
                ImzaError::new(
                    ErrorCode::RequestError,
                    format!("batch {batch_id} için PIN bekleyen yok"),
                )
            })?;
        // Alıcı düşmüşse (istek iptal edildi) hata YUTULUR: cevabı verecek kimse kalmadıysa yapacak bir şey yok.
        let _ = tx.send(cevap);
        Ok(())
    }

    /// Bekleyen bir PIN isteğini iptal eder (batch iptal edilirse).
    pub fn pin_bekleyeni_dusur(&self, batch_id: &str) {
        if let Ok(mut m) = self.inner.pin_bekleyenler.lock() {
            m.remove(batch_id);
        }
    }

    /// Ajan "yaşıyorum" dedi.
    pub fn agent_isareti(&self) {
        if let Ok(mut g) = self.inner.agent_son_gorulme.lock() {
            *g = Some(std::time::Instant::now());
        }
    }

    /// Ajan şu anda bağlı mı (`interactiveAgent`): damgadan türer, bayraktan DEĞİL.
    #[must_use]
    pub fn agent_bagli(&self) -> bool {
        self.inner
            .agent_son_gorulme
            .lock()
            .ok()
            .and_then(|g| *g)
            .is_some_and(|t| t.elapsed() < AGENT_TAZELIK)
    }

    /// Denetim zincirinin anlık kopyası — tanılama için.
    #[must_use]
    pub fn audit_events(&self) -> Vec<kentos_imza_audit::AuditEvent> {
        self.inner
            .audit
            .lock()
            .map(|z| z.events().to_vec())
            .unwrap_or_default()
    }

    /// Denetim zinciri sağlam mı.
    #[must_use]
    pub fn audit_valid(&self) -> bool {
        self.inner.audit.lock().is_ok_and(|z| z.verify().is_ok())
    }
}

fn kilit_hatasi(ne: &str) -> ImzaError {
    ImzaError::new(
        ErrorCode::ServiceError,
        format!("{ne} kilidi zehirlendi (bir iş parçacığı panikledi)"),
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test içinde başarısızlık zaten çökmeli")]
mod tests {
    use kentos_imza_signature::MockElectronicSignatureProvider;

    use super::*;

    fn durum() -> AppState {
        AppState::new(
            SignatureProviderRegistry::new().with(Box::new(MockElectronicSignatureProvider)),
            PinEntryMode::Native,
            None,
            None,
        )
    }

    fn batch(id: &str, tx: &str) -> SigningBatch {
        SigningBatch {
            id: id.into(),
            transaction_id: tx.into(),
            signature_method: kentos_imza_domain::SignatureMethod::ElectronicSignature,
            client_type: kentos_imza_domain::SigningClientType::Web,
            title: "test".into(),
            description: None,
            source_application: None,
            source_module: None,
            source_context: None,
            packages: vec![],
            status: BatchStatus::Created,
            created_at: now_rfc3339(),
            expires_at: "2099-01-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn ayni_islem_ikinci_kez_eklenemez() {
        let d = durum();
        assert!(d.insert_batch(batch("b1", "tx1"), "n1").is_ok());
        let hata = d.insert_batch(batch("b2", "tx1"), "n2").unwrap_err();
        assert_eq!(hata.code, "KSI-1503");
    }

    #[test]
    /// Durum **doğrudan atanamaz** — geçersiz geçiş reddedilir.
    fn gecersiz_gecis_reddedilir() {
        let d = durum();
        d.insert_batch(batch("b1", "tx1"), "n1").unwrap();
        assert!(d.transition("b1", BatchStatus::Completed).is_err());
        assert!(d.transition("b1", BatchStatus::Validating).is_ok());
        assert_eq!(d.batch("b1").unwrap().batch.status, BatchStatus::Validating);
    }

    #[test]
    fn olaylar_denetim_zincirine_yazilir_ve_zincir_saglam() {
        let d = durum();
        let b = batch("b1", "tx1");
        d.insert_batch(b.clone(), "n1").unwrap();
        d.emit("batch_created", Some(&b), serde_json::json!({}));
        d.emit(
            "batch_opened",
            Some(&b),
            serde_json::json!({ "packageId": "p1" }),
        );

        let olaylar = d.audit_events();
        assert_eq!(olaylar.len(), 2);
        assert!(olaylar[0].previous_event_hash.is_none());
        assert_eq!(
            olaylar[1].previous_event_hash.as_deref(),
            Some(olaylar[0].event_hash.as_str())
        );
        assert!(d.audit_valid());
        assert_eq!(olaylar[1].package_id.as_deref(), Some("p1"));
    }

    #[test]
    fn olmayan_batch_1500_doner() {
        assert_eq!(durum().batch("yok").unwrap_err().code, "KSI-1500");
    }
}
