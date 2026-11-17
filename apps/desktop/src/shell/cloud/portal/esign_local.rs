//! The e-signature's local side (docs/adr/9002 §7): the server's envelope verified here, the card layer loaded once,
//! the cards read and the batch signed. Card calls are blocking C calls, so they run on threads of their own; the PIN
//! goes only to the signing thread and enters no message, record or `Debug` output.
//!
//! The trust key can be pinned with `KENTOS_IMZA_TRUST_KEY`; without it the key the server names is used (it comes
//! over the same session, so it guards only against local tampering and replays; docs/adr/9002 §7).

use std::sync::{Arc, Mutex, OnceLock};

use iced::Task;
use iced::futures::{SinkExt as _, StreamExt as _};
use kentos_commands::{SigningRequestCreated, SigningTrust};
use kentos_imza_domain::BatchRecord;
use kentos_imza_security::{ReplayGuard, TrustConfig};
use kentos_imza_signature::{Choice, RecordSink, Token, sign_batch};

use super::esign::{Cards, EsignEvent, Worker, ev};
use crate::app::Message;

/// The card layer, loaded once for the process's life: PKCS#11's `C_Initialize` is called once per process. In tests
/// every call is a new software card (a wrong PIN's count does not carry from test to test).
pub(super) fn token() -> Result<Arc<dyn Token>, String> {
    #[cfg(test)]
    {
        Ok(Arc::new(kentos_imza_signature::mock_token::MockToken::new()))
    }
    #[cfg(not(test))]
    {
        static CARD: OnceLock<Result<Arc<dyn Token>, String>> = OnceLock::new();
        CARD.get_or_init(load_token).clone()
    }
}

#[cfg(not(test))]
fn load_token() -> Result<Arc<dyn Token>, String> {
    #[cfg(feature = "esign-mock")]
    if std::env::var("KENTOS_IMZA_DENEME_KARTI").as_deref() == Ok("1") {
        return Ok(Arc::new(
            kentos_imza_signature::mock_token::MockToken::new()
                .with_delay(std::time::Duration::from_millis(600)),
        ));
    }
    let library = std::env::var_os("KENTOS_IMZA_PKCS11")
        .map(std::path::PathBuf::from)
        .or_else(|| kentos_imza_pkcs11::well_known_libraries().into_iter().next())
        .ok_or_else(|| {
            "E-imza kartının sürücüsü bulunamadı. Kartın sürücüsünü kurun ya da KENTOS_IMZA_PKCS11 ile yolunu verin."
                .to_owned()
        })?;
    kentos_imza_pkcs11::ProviderConfig::new("masaustu", library)
        .and_then(kentos_imza_pkcs11::Pkcs11Module::load)
        .map(|m| Arc::new(m) as Arc<dyn Token>)
        .map_err(|e| e.user_message)
}

/// One replay gate for the process's life: the same envelope is not opened twice.
fn replay() -> &'static Mutex<ReplayGuard> {
    static GATE: OnceLock<Mutex<ReplayGuard>> = OnceLock::new();
    GATE.get_or_init(|| Mutex::new(ReplayGuard::new()))
}

/// The trust key: the one pinned here when there is one (refused when the server names another), else the server's.
pub(super) fn trust_config(r: Result<SigningTrust, String>) -> Result<TrustConfig, String> {
    let pinned = std::env::var("KENTOS_IMZA_TRUST_KEY")
        .ok()
        .filter(|k| !k.trim().is_empty());
    let (key, issuer, audience) = match (pinned, r) {
        (Some(k), Ok(t)) if t.public_key.trim() != k.trim() => {
            return Err(
                "Sunucunun e-imza anahtarı bu bilgisayarda sabitlenen anahtarla uyuşmuyor; imza başlatılmadı.".into(),
            );
        }
        (Some(k), Ok(t)) => (k, t.issuer, t.audience),
        (Some(k), Err(_)) => (
            k,
            kentos_imza_security::DEFAULT_ISSUER.to_owned(),
            kentos_imza_security::DEFAULT_AUDIENCE.to_owned(),
        ),
        (None, Ok(t)) => (t.public_key, t.issuer, t.audience),
        (None, Err(why)) => return Err(format!("E-imza bu sunucuda kullanılamıyor: {why}")),
    };
    TrustConfig::from_base64(&key, issuer, audience).map_err(|e| e.user_message)
}

/// Opens the server's answer: the envelope's signature first, then its content (an unverified body is not looked
/// at), then that it is used once, then the batch.
pub(super) fn open_envelope(
    trust: &TrustConfig,
    created: &SigningRequestCreated,
) -> Result<BatchRecord, String> {
    let now = kentos_imza_signature::verify::now_rfc3339();
    let payload = kentos_imza_security::verify(&created.signed_request, trust, &now)
        .map_err(|e| e.user_message)?;
    if payload.transaction_id != created.transaction_id {
        return Err("Sunucunun cevabı imzalı istekle uyuşmuyor.".to_owned());
    }
    replay()
        .lock()
        .map_err(|_| "İmza isteğinin tek kullanımlık olduğu denetlenemedi.".to_owned())?
        .consume(&payload.transaction_id, &payload.nonce, &now)
        .map_err(|e| e.user_message)?;
    let mut record = BatchRecord::new(payload.to_batch().map_err(|e| e.user_message)?);
    record.open().map_err(|e| e.user_message)?;
    Ok(record)
}

/// Runs card work on a thread of its own.
fn on_thread<T: Send + 'static>(
    f: impl FnOnce() -> T + Send + 'static,
) -> impl Future<Output = Option<T>> {
    let (tx, rx) = iced::futures::channel::oneshot::channel();
    let started = std::thread::Builder::new()
        .name("kentos-imza".into())
        .spawn(move || {
            let _ = tx.send(f());
        })
        .is_ok();
    async move { if started { rx.await.ok() } else { None } }
}

/// Reads the cards plugged in and their certificates.
pub(super) fn read_cards() -> Task<Message> {
    Task::perform(
        on_thread(|| -> Result<Cards, String> {
            let card = token()?;
            let devices = card.devices().map_err(|e| e.user_message)?;
            if devices.is_empty() {
                return Err(
                    "Elektronik imza cihazı bulunamadı. Kartı takıp yeniden tarayın.".into(),
                );
            }
            let list = devices
                .into_iter()
                .map(|d| {
                    let certificates = card.certificates(d.slot_id).unwrap_or_default();
                    (d, certificates)
                })
                .collect();
            Ok(Cards {
                mock: card.is_mock(),
                list,
            })
        }),
        |r| {
            ev(EsignEvent::Cards(
                r.unwrap_or_else(|| Err("Kart okunamadı.".into())),
            ))
        },
    )
}

/// Runs the engine on a thread of its own; its events and its end flow to the window. The PIN goes only to this
/// thread and is dropped there.
pub(super) fn sign_task(
    card: Arc<dyn Token>,
    mut record: BatchRecord,
    choice: Choice,
    pin: String,
) -> Task<Message> {
    let flow = iced::stream::channel(64, async move |mut out| {
        let (tx, mut rx) = iced::futures::channel::mpsc::unbounded::<Worker>();
        let started = std::thread::Builder::new()
            .name("kentos-imza-motor".into())
            .spawn(move || {
                let batch = record.batch.clone();
                let events = tx.clone();
                let mut emit = |name: &str, payload: &serde_json::Value| {
                    let _ = events.unbounded_send(Worker::Event {
                        name: name.to_owned(),
                        at: kentos_imza_signature::verify::now_rfc3339(),
                        payload: payload.clone(),
                    });
                };
                let error = {
                    let mut sink = RecordSink {
                        record: &mut record,
                        on_event: &mut emit,
                    };
                    sign_batch(&batch, card.as_ref(), &choice, &pin, &mut sink).err()
                };
                drop(pin);
                let _ = tx.unbounded_send(Worker::Done {
                    record: Box::new(record),
                    error,
                });
            })
            .is_ok();
        if !started {
            return;
        }
        while let Some(w) = rx.next().await {
            if out.send(w).await.is_err() {
                break;
            }
        }
    });
    Task::run(flow, |w| ev(EsignEvent::Worker(w)))
}
