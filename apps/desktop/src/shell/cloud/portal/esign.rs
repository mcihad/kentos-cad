//! E-signature in this process (docs/adr/9002 §7; the server's side is docs/adr/9004): the files of the package being
//! prepared in Portal dosyaları are signed by talking to the card from this process (PKCS#11), without the local
//! service.
//!
//! The way is the local service's contract: the server works the files' digests out itself and signs the signing
//! request with Ed25519 → this process verifies the envelope with the trust key, checks that it is used once and opens
//! the batch → the user picks the card and the certificate and types the PIN (the window is `kentos-imza-ui`'s, the
//! PIN agent's too) → the engine (`sign_batch`) signs in one card session and verifies each signature here → the
//! result (`BatchResult`) is handed to the server, which verifies it again against its own bytes → the window says the
//! server's verdict.
//!
//! This file is the window's side; the envelope's and the card's side, on threads of their own, is `esign_local.rs`.

use std::sync::Arc;

use iced::Task;
use kentos_commands::{
    DossierAction, DossierDetail, DossierSignatureView, SigningRequestCreated, SigningTrust,
    SigningVerdict,
};
use kentos_imza_domain::{BatchRecord, CertificateInfo, DeviceInfo};
use kentos_imza_security::TrustConfig;
use kentos_imza_signature::Choice;
use kentos_imza_ui::{Action, Link, Msg, Outcome, Phase, Request, SigningDialog};

use super::esign_local::{open_envelope, read_cards, sign_task, token, trust_config};
use super::esign_server::EsignServer;
use super::{Event, msg};
use crate::app::{App, Message};

/// The cards read: each card plugged in with its certificates.
#[derive(Debug, Clone)]
pub struct Cards {
    /// The software test card: its signatures have no legal value and the window says so.
    pub mock: bool,
    pub list: Vec<(DeviceInfo, Vec<CertificateInfo>)>,
}

/// What comes from the engine's thread.
#[derive(Debug, Clone)]
pub enum Worker {
    /// An engine event (`file_signed` …), with its time and payload.
    Event {
        name: String,
        at: String,
        payload: serde_json::Value,
    },
    /// Done: the record and, when it stopped before the batch started (a wrong PIN, no card), why.
    Done {
        record: Box<BatchRecord>,
        error: Option<kentos_imza_core::ImzaError>,
    },
}

/// The e-signature's events.
#[derive(Debug, Clone)]
pub enum EsignEvent {
    Trusted(Result<SigningTrust, String>),
    Requested(Result<SigningRequestCreated, String>),
    Cards(Result<Cards, String>),
    Dialog(Msg),
    Worker(Worker),
    Delivered(Result<SigningVerdict, String>),
    Cancelled,
    Signatures {
        dossier: String,
        result: Result<Vec<DossierSignatureView>, String>,
    },
}

/// A signing job, shown over the window.
pub struct Esign {
    pub dossier: String,
    /// Gönder's question follows the signing (E-imzayla gönder).
    pub then_send: bool,
    pub files: Vec<String>,
    pub dialog: SigningDialog,
    /// The key the envelope is verified with, from the trust answer on.
    trust: Option<TrustConfig>,
    pub transaction: Option<String>,
    pub record: Option<BatchRecord>,
    /// The server's verdict, file by file.
    pub verdict: Option<SigningVerdict>,
}

impl std::fmt::Debug for Esign {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Esign")
            .field("dossier", &self.dossier)
            .field("transaction", &self.transaction)
            .field("dialog", &self.dialog)
            .finish_non_exhaustive()
    }
}

impl Esign {
    /// Stopped before a request could be shown: the window says why, with Kapat.
    fn stop(&mut self, why: String) {
        self.dialog.show_request(Request {
            id: String::new(),
            title: "E-imza".into(),
            reference: None,
            documents: Vec::new(),
        });
        self.dialog.finish(Outcome::Failed(why));
    }
}

#[cfg(test)]
thread_local! {
    /// Each test thread its own test server: one test's signatures do not show in another's window.
    static TEST_SERVER: super::esign_server::MockServer = super::esign_server::MockServer::new();
}

/// The in-process test server: one per test thread in tests, one per process in an `esign-mock` build.
#[cfg(any(test, feature = "esign-mock"))]
pub(super) fn mock_server() -> super::esign_server::MockServer {
    #[cfg(test)]
    {
        TEST_SERVER.with(Clone::clone)
    }
    #[cfg(not(test))]
    {
        static ONE: std::sync::OnceLock<super::esign_server::MockServer> =
            std::sync::OnceLock::new();
        ONE.get_or_init(super::esign_server::MockServer::new)
            .clone()
    }
}

/// The server: the signed-in cloud client; in a build with `esign-mock`, `KENTOS_IMZA_DENEME_SUNUCU=1` gives the
/// in-process test server instead (tests always use it).
pub fn server(app: &App) -> Option<Arc<dyn EsignServer>> {
    #[cfg(any(test, feature = "esign-mock"))]
    if cfg!(test) || std::env::var("KENTOS_IMZA_DENEME_SUNUCU").as_deref() == Ok("1") {
        let s = mock_server();
        if let Some(d) = app.cloud.portal.as_ref().and_then(|p| p.chosen_detail()) {
            s.know(d.pending.iter().map(|f| (f.id.clone(), f.name.clone())));
        }
        return Some(Arc::new(s));
    }
    super::esign_cloud::server(app)
}

/// The files of the package being prepared that can be signed: the reader's side's pending files.
pub fn signable(d: &DossierDetail) -> Vec<String> {
    d.pending.iter().map(|f| f.id.clone()).collect()
}

/// Whether the reader signs the chosen dossier's files now: the server says this reader e-signs them (`can_sign`: the
/// side preparing the package, an office's editors, the municipality's editors and above; docs/adr/9004) and the
/// package being prepared has files.
pub fn signs(d: &DossierDetail) -> bool {
    d.can_sign && !d.pending.is_empty()
}

/// What the step row offers: `Some(true)` for E-imzayla gönder (Gönder is one of the steps), `Some(false)` for
/// E-imzala, `None` while the switch is off or the reader does not sign these files ([`signs`]).
pub fn offer(on: bool, d: &DossierDetail) -> Option<bool> {
    (on && signs(d)).then(|| d.actions.contains(&DossierAction::Send))
}

/// The window's message of an e-signature event.
pub(super) fn ev(e: EsignEvent) -> Message {
    msg(Event::Esign(Box::new(e)))
}

impl App {
    /// E-imzala / E-imzayla gönder: the trust key first, then the signing request. Only with the switch on.
    pub(super) fn esign_start(&mut self, then_send: bool) -> Task<Message> {
        let Some(server) = server(self) else {
            return Task::none();
        };
        let Some(p) = self
            .cloud
            .portal
            .as_mut()
            .filter(|p| p.esign_on && !p.working() && p.esign.is_none())
        else {
            return Task::none();
        };
        // The message alone does not start what the row does not offer.
        let Some(d) = p.chosen_detail().filter(|d| signs(d)) else {
            return Task::none();
        };
        let files = signable(d);
        let mut dialog = SigningDialog::default();
        dialog.link = Link::InProcess;
        dialog.phase = Phase::Waiting;
        p.esign = Some(Esign {
            dossier: d.summary.id.clone(),
            then_send,
            files,
            dialog,
            trust: None,
            transaction: None,
            record: None,
            verdict: None,
        });
        Task::perform(server.trust(), |r| ev(EsignEvent::Trusted(r)))
    }

    pub(super) fn esign_event(&mut self, e: EsignEvent) -> Task<Message> {
        match e {
            EsignEvent::Trusted(r) => self.esign_trusted(r),
            EsignEvent::Requested(r) => self.esign_requested(r),
            EsignEvent::Cards(r) => {
                let Some(s) = self.esign_mut() else {
                    return Task::none();
                };
                match r {
                    Ok(cards) => {
                        s.dialog.mock = cards.mock;
                        let options = kentos_imza_ui::convert::options(&cards.list);
                        s.dialog.set_certificates(options, None);
                        // The PIN is typed without reaching for the mouse.
                        kentos_imza_ui::focus_pin()
                    }
                    Err(why) => {
                        s.dialog.card_failed(why);
                        Task::none()
                    }
                }
            }
            EsignEvent::Dialog(m) => self.esign_dialog(m),
            EsignEvent::Worker(w) => self.esign_worker(w),
            EsignEvent::Delivered(r) => self.esign_delivered(r),
            EsignEvent::Cancelled => Task::none(),
            EsignEvent::Signatures { dossier, result } => {
                if let (Some(p), Ok(list)) = (self.cloud.portal.as_mut(), result) {
                    p.signatures.insert(dossier, list);
                }
                Task::none()
            }
        }
    }

    /// Esc over the signing window is its Kapat, as a press on the dimmed area is: a request the server never got a
    /// result for is given up on the server there, not left until it expires; while the card works Esc does nothing.
    /// `None` without a signing window (Esc then closes Portal dosyaları as before).
    pub(crate) fn esign_escape(&mut self) -> Option<Task<Message>> {
        self.cloud.portal.as_ref()?.esign.as_ref()?;
        Some(self.esign_dialog(Msg::Close))
    }

    fn esign_mut(&mut self) -> Option<&mut Esign> {
        self.cloud.portal.as_mut()?.esign.as_mut()
    }

    fn esign_trusted(&mut self, r: Result<SigningTrust, String>) -> Task<Message> {
        let Some(server) = server(self) else {
            return Task::none();
        };
        let Some(s) = self.esign_mut() else {
            return Task::none();
        };
        match trust_config(r) {
            Ok(trust) => {
                s.trust = Some(trust);
                Task::perform(server.request(s.dossier.clone(), s.files.clone()), |r| {
                    ev(EsignEvent::Requested(r))
                })
            }
            Err(why) => {
                s.stop(why);
                Task::none()
            }
        }
    }

    fn esign_requested(&mut self, r: Result<SigningRequestCreated, String>) -> Task<Message> {
        let server = server(self);
        let Some(s) = self.esign_mut() else {
            return Task::none();
        };
        let created = match r {
            Ok(c) => c,
            Err(why) => {
                s.stop(why);
                return Task::none();
            }
        };
        let opened = s
            .trust
            .as_ref()
            .ok_or_else(|| "Sunucunun e-imza anahtarı alınamadı.".to_owned())
            .and_then(|trust| open_envelope(trust, &created));
        match opened {
            Ok(record) => {
                s.dialog.show_request(Request::from_batch(&record.batch));
                s.dialog.reading = true;
                s.transaction = Some(created.transaction_id);
                s.record = Some(record);
                read_cards()
            }
            Err(why) => {
                s.stop(why);
                // The server's request is not left hanging until it expires: the answer is "not signed".
                match server {
                    Some(server) => Task::perform(server.cancel(created.transaction_id), |_| {
                        ev(EsignEvent::Cancelled)
                    }),
                    None => Task::none(),
                }
            }
        }
    }

    fn esign_dialog(&mut self, m: Msg) -> Task<Message> {
        let server = server(self);
        let Some(p) = self.cloud.portal.as_mut() else {
            return Task::none();
        };
        let Some(s) = p.esign.as_mut() else {
            return Task::none();
        };
        match s.dialog.update(m) {
            Some(Action::Sign {
                slot_id,
                certificate_id,
                pin,
            }) => {
                // The card first: a card that cannot be used leaves the opened batch for the next try.
                let card = match token() {
                    Ok(card) => card,
                    Err(why) => {
                        s.dialog.pin_refused(why);
                        return kentos_imza_ui::focus_pin();
                    }
                };
                let Some(record) = s.record.take() else {
                    s.dialog.pin_refused("İmza isteği açık değil.");
                    return Task::none();
                };
                let choice = Choice {
                    slot_id,
                    certificate_id,
                };
                sign_task(card, record, choice, pin)
            }
            Some(Action::Cancel) => {
                let transaction = s.transaction.take();
                p.esign = None;
                match (transaction, server) {
                    (Some(tx), Some(server)) => {
                        Task::perform(server.cancel(tx), |_| ev(EsignEvent::Cancelled))
                    }
                    _ => Task::none(),
                }
            }
            Some(Action::Close) => {
                let send = s.then_send && s.verdict.as_ref().is_some_and(|v| v.accepted);
                let dossier = s.dossier.clone();
                // A request the server never got a result for (nothing could be signed, or the delivery failed) is
                // given up, not left hanging until it expires.
                let unanswered = if s.verdict.is_none() {
                    s.transaction.take()
                } else {
                    None
                };
                p.esign = None;
                let mut tasks = vec![self.esign_signatures(dossier)];
                if let (Some(tx), Some(server)) = (unanswered, server) {
                    tasks.push(Task::perform(server.cancel(tx), |_| {
                        ev(EsignEvent::Cancelled)
                    }));
                }
                if send {
                    tasks.push(Task::done(msg(Event::Step(DossierAction::Send))));
                }
                Task::batch(tasks)
            }
            Some(Action::Rescan) => read_cards(),
            None => Task::none(),
        }
    }

    fn esign_worker(&mut self, w: Worker) -> Task<Message> {
        let server = server(self);
        let Some(s) = self.esign_mut() else {
            return Task::none();
        };
        match w {
            Worker::Event { name, at, payload } => {
                s.dialog.event(&name, &at, &payload);
                Task::none()
            }
            Worker::Done { record, error } => {
                let record = *record;
                if let Some(e) = error {
                    // Stopped before the batch started (a wrong PIN, no card): the batch stays READY and is tried
                    // again in the same window.
                    s.record = Some(record);
                    s.dialog.pin_refused(e.user_message);
                    return kentos_imza_ui::focus_pin();
                }
                let result = record.result();
                s.record = Some(record);
                if result.signed == 0 {
                    let why = result.last_error.as_ref().map_or_else(
                        || "Hiçbir belge imzalanamadı.".to_owned(),
                        |e| e.user_message.clone(),
                    );
                    s.dialog.finish(Outcome::Failed(why));
                    return Task::none();
                }
                let (Some(tx), Some(server)) = (s.transaction.clone(), server) else {
                    return Task::none();
                };
                Task::perform(server.deliver(tx, result), |r| ev(EsignEvent::Delivered(r)))
            }
        }
    }

    fn esign_delivered(&mut self, r: Result<SigningVerdict, String>) -> Task<Message> {
        let Some(s) = self.esign_mut() else {
            return Task::none();
        };
        let total = s.dialog.request.as_ref().map_or(0, Request::to_sign);
        match r {
            Ok(v) => {
                if let Some(record) = s.record.as_mut() {
                    let _ = record.acknowledge(v.accepted);
                }
                let signed = v.signed as usize;
                let outcome = if !v.accepted {
                    Outcome::Failed(
                        v.reason
                            .clone()
                            .unwrap_or_else(|| "Sunucu imzaları kabul etmedi.".into()),
                    )
                } else if v.signed == v.required && signed == total {
                    Outcome::Completed { signed }
                } else {
                    Outcome::Partial {
                        signed,
                        total: v.required as usize,
                    }
                };
                s.dialog.finish(outcome);
                s.verdict = Some(v);
            }
            Err(why) => s.dialog.finish(Outcome::Failed(format!(
                "Sonuç sunucuya teslim edilemedi: {why}"
            ))),
        }
        Task::none()
    }

    /// The dossier's recorded signatures, for the file lines' "E-imzalı".
    pub(super) fn esign_signatures(&mut self, dossier: String) -> Task<Message> {
        let Some(server) = server(self) else {
            return Task::none();
        };
        Task::perform(server.signatures(dossier.clone()), move |result| {
            ev(EsignEvent::Signatures {
                dossier: dossier.clone(),
                result,
            })
        })
    }
}
