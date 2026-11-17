//! The e-signature's server side as the window sees it (docs/adr/9002 §7; the server is docs/adr/9004): the trust key,
//! the signing request, the delivery, giving up and a dossier's signatures. The window looks at this trait; the real
//! one is the signed-in cloud client (`esign_cloud.rs`), and the test server ([`MockServer`]) keeps the same rules in
//! this process (it signs the envelope and checks the delivery with the server's rule, `verify_delivery`). The answers
//! are the portal's contract types (`kentos-contracts`).

use std::future::Future;
use std::pin::Pin;

use kentos_commands::{DossierSignatureView, SigningRequestCreated, SigningTrust, SigningVerdict};
use kentos_imza_domain::BatchResult;

/// A call's answer, or its short reason in Turkish.
pub type Answer<T> = Pin<Box<dyn Future<Output = Result<T, String>> + Send + 'static>>;

/// The server's e-signature endpoints.
pub trait EsignServer: Send + Sync {
    /// `GET /v1/signing/trust`.
    fn trust(&self) -> Answer<SigningTrust>;
    /// `POST /v1/dossiers/{id}/signing`.
    fn request(&self, dossier: String, file_ids: Vec<String>) -> Answer<SigningRequestCreated>;
    /// `POST /v1/signing/{tx}/result`.
    fn deliver(&self, transaction: String, result: BatchResult) -> Answer<SigningVerdict>;
    /// `POST /v1/signing/{tx}/cancel`.
    fn cancel(&self, transaction: String) -> Answer<()>;
    /// `GET /v1/dossiers/{id}/signatures`.
    fn signatures(&self, dossier: String) -> Answer<Vec<DossierSignatureView>>;
}

#[cfg(any(test, feature = "esign-mock"))]
pub use mock::MockServer;

#[cfg(any(test, feature = "esign-mock"))]
mod mock {
    //! The test server: signs the envelope in this process and checks the delivery with the server's rules
    //! (`verify_delivery`). A file's bytes come from its id; the digests are computed from those bytes.

    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};

    use kentos_commands::{
        DossierSide, DossierSignatureView, SigningFile, SigningFileVerdict, SigningLegalCheck,
        SigningOutcome, SigningRequestCreated, SigningSigner, SigningTrust, SigningVerdict,
    };
    use kentos_imza_domain::{
        BatchResult, DigestAlgorithm, PayloadFile, PayloadPackage, SignedRequestPayload,
    };
    use kentos_imza_security::digest;
    use kentos_imza_security::issue::{ServerKey, issue};
    use kentos_imza_signature::delivery::verify_delivery;

    use super::{Answer, EsignServer};

    #[derive(Default)]
    struct State {
        /// File id → name.
        names: HashMap<String, String>,
        requests: HashMap<String, SignedRequestPayload>,
        signatures: HashMap<String, Vec<DossierSignatureView>>,
    }

    /// The requests' numbers, one count for the process: the window's replay gate is the process's too, so two test
    /// servers (one per test thread) must never issue the same transaction and nonce (measured: the second test's
    /// first request was refused as a replay).
    static ISSUED: AtomicU64 = AtomicU64::new(0);

    /// The test server.
    #[derive(Clone)]
    pub struct MockServer {
        key: Arc<ServerKey>,
        state: Arc<Mutex<State>>,
    }

    /// A file's test bytes.
    pub fn bytes_of(file_id: &str) -> Vec<u8> {
        format!("KentOS deneme belgesi {file_id}").into_bytes()
    }

    impl MockServer {
        /// A new test server, with a key of its own.
        ///
        /// # Panics
        /// When the system's random source cannot be read (a test build; never in the product's binary).
        #[must_use]
        pub fn new() -> Self {
            Self {
                key: Arc::new(ServerKey::generate().expect("a random source")),
                state: Arc::new(Mutex::new(State::default())),
            }
        }

        /// Makes the files' names known (the window sends only their ids in the signing request).
        pub fn know(&self, files: impl IntoIterator<Item = (String, String)>) {
            if let Ok(mut state) = self.state.lock() {
                state.names.extend(files);
            }
        }

        /// Forgets the signatures kept so far (a picture of a dossier before it was signed).
        #[cfg(test)]
        pub fn forget(&self) {
            if let Ok(mut state) = self.state.lock() {
                state.signatures.clear();
            }
        }

        /// The signing requests issued and neither delivered nor given up.
        #[cfg(test)]
        pub fn open(&self) -> usize {
            self.state.lock().map_or(0, |state| state.requests.len())
        }
    }

    fn ready<T: Send + 'static>(t: Result<T, String>) -> Answer<T> {
        Box::pin(async move { t })
    }

    impl EsignServer for MockServer {
        fn trust(&self) -> Answer<SigningTrust> {
            ready(Ok(SigningTrust {
                public_key: self.key.public_base64(),
                issuer: kentos_imza_security::DEFAULT_ISSUER.to_owned(),
                audience: kentos_imza_security::DEFAULT_AUDIENCE.to_owned(),
            }))
        }

        fn request(&self, dossier: String, file_ids: Vec<String>) -> Answer<SigningRequestCreated> {
            let Ok(mut state) = self.state.lock() else {
                return ready(Err("deneme sunucusu kullanılamıyor".to_owned()));
            };
            let n = ISSUED.fetch_add(1, Ordering::Relaxed) + 1;
            let tx = format!("deneme-{n}");
            let files: Vec<PayloadFile> = file_ids
                .iter()
                .map(|id| {
                    let b = bytes_of(id);
                    PayloadFile {
                        id: id.clone(),
                        name: state
                            .names
                            .get(id)
                            .cloned()
                            .unwrap_or_else(|| format!("{id}.pdf")),
                        role: "MAIN_DOCUMENT".into(),
                        mime_type: "application/pdf".into(),
                        size: b.len() as u64,
                        digest_algorithm: "SHA-384".into(),
                        digest: digest::compute(DigestAlgorithm::Sha384, &b),
                        signing_required: true,
                    }
                })
                .collect();
            let payload = SignedRequestPayload {
                transaction_id: tx.clone(),
                batch_id: format!("{tx}-batch"),
                user_id: "deneme".into(),
                session_id: "deneme".into(),
                signature_method: "ELECTRONIC_SIGNATURE".into(),
                client_type: "DESKTOP".into(),
                packages: vec![PayloadPackage {
                    id: dossier.clone(),
                    title: "Deneme dosyası, hazırlanan paket".into(),
                    reference_number: Some("2026/0007".into()),
                    document_type: None,
                    metadata: serde_json::json!({ "dossierId": dossier }),
                    files: files.clone(),
                }],
                created_at: kentos_imza_signature::verify::now_rfc3339(),
                expires_at: kentos_imza_signature::verify::rfc3339_after(15 * 60),
                nonce: format!("{n:032x}"),
                issuer: kentos_imza_security::DEFAULT_ISSUER.into(),
                audience: kentos_imza_security::DEFAULT_AUDIENCE.into(),
            };
            let envelope = issue(&payload, &self.key, None);
            state.requests.insert(tx.clone(), payload.clone());
            ready(envelope.map_err(|e| e.user_message).map(|signed_request| {
                SigningRequestCreated {
                    transaction_id: tx,
                    batch_id: payload.batch_id.clone(),
                    signed_request,
                    expires_at: payload.expires_at.clone(),
                    files: files
                        .into_iter()
                        .map(|f| SigningFile {
                            file_id: f.id,
                            name: f.name,
                            size: f.size,
                            digest_algorithm: f.digest_algorithm,
                            digest: f.digest,
                        })
                        .collect(),
                }
            }))
        }

        fn deliver(&self, transaction: String, result: BatchResult) -> Answer<SigningVerdict> {
            let Ok(mut state) = self.state.lock() else {
                return ready(Err("deneme sunucusu kullanılamıyor".to_owned()));
            };
            let Some(issued) = state.requests.remove(&transaction) else {
                return ready(Err(
                    "Bu imza isteği tamamlanmış ya da hiç açılmamış.".to_owned()
                ));
            };
            let h = verify_delivery(&issued, &result, |id| {
                Some(digest::compute(DigestAlgorithm::Sha384, &bytes_of(id)))
            });
            if let Some(s) = &h.signer {
                let dossier = issued
                    .packages
                    .first()
                    .map(|p| p.id.clone())
                    .unwrap_or_default();
                let now = kentos_imza_signature::verify::now_rfc3339();
                let kept: Vec<DossierSignatureView> = h
                    .files
                    .iter()
                    .filter(|f| f.verified)
                    .map(|f| DossierSignatureView {
                        id: f.signature_id.clone().unwrap_or_default(),
                        file_id: f.file_id.clone(),
                        file_name: state.names.get(&f.file_id).cloned().unwrap_or_default(),
                        round: 0,
                        side: DossierSide::Professional,
                        signer_name: "Deneme kullanıcısı".into(),
                        subject: s.subject.clone(),
                        issuer: s.issuer.clone(),
                        serial_number: s.serial_number.clone(),
                        fingerprint_sha256: s.fingerprint_sha256.clone(),
                        key_algorithm: s.key_algorithm.clone(),
                        signature_algorithm: s.signature_algorithm.clone(),
                        digest_algorithm: "SHA-384".into(),
                        signed_at: result
                            .results
                            .iter()
                            .find(|r| r.file_id == f.file_id)
                            .map(|r| r.signature.signed_at.clone())
                            .unwrap_or_default(),
                        verified_at: now.clone(),
                        still_matches: true,
                        legal_check: SigningLegalCheck::NotDone,
                    })
                    .collect();
                // A file signed again with the same certificate keeps the later signature only.
                let list = state.signatures.entry(dossier).or_default();
                list.retain(|old| {
                    !kept.iter().any(|k| {
                        k.file_id == old.file_id && k.fingerprint_sha256 == old.fingerprint_sha256
                    })
                });
                list.extend(kept);
            }
            let status = match h.status() {
                "completed" => SigningOutcome::Completed,
                "partially" => SigningOutcome::Partially,
                _ => SigningOutcome::Failed,
            };
            ready(Ok(SigningVerdict {
                accepted: h.accepted,
                reason: h.reason.clone(),
                status,
                signed: u32::try_from(h.signed).unwrap_or(u32::MAX),
                required: u32::try_from(h.required).unwrap_or(u32::MAX),
                files: h
                    .files
                    .iter()
                    .map(|f| SigningFileVerdict {
                        file_id: f.file_id.clone(),
                        verified: f.verified,
                        error: f.error.clone(),
                    })
                    .collect(),
                signer: h.signer.map(|s| SigningSigner {
                    subject: s.subject,
                    issuer: s.issuer,
                    serial_number: s.serial_number,
                    fingerprint_sha256: s.fingerprint_sha256,
                }),
                legal_check: SigningLegalCheck::NotDone,
            }))
        }

        fn cancel(&self, transaction: String) -> Answer<()> {
            if let Ok(mut state) = self.state.lock() {
                state.requests.remove(&transaction);
            }
            ready(Ok(()))
        }

        fn signatures(&self, dossier: String) -> Answer<Vec<DossierSignatureView>> {
            let list = self
                .state
                .lock()
                .map(|state| state.signatures.get(&dossier).cloned().unwrap_or_default())
                .unwrap_or_default();
            ready(Ok(list))
        }
    }
}
