//! E-signing a dossier's files from the desktop (docs/adr/9002 §7, docs/adr/9004): the server's key, a signing request
//! for the package's files, the result delivered for the server's own check, a request given up, and the signatures
//! kept on a dossier's files.
//!
//! - `GET /v1/signing/trust`;
//! - `POST /v1/dossiers/{id}/signing`, `POST /v1/signing/{tx}/result`, `POST /v1/signing/{tx}/cancel`;
//! - `GET /v1/dossiers/{id}/signatures`.
//!
//! The envelope and the delivered result are the signing service's wire types (`kentos-imza-domain`); the answers
//! around them are the portal's contracts.

use std::future::Future;
use std::time::Duration;

use kentos_contracts::{
    DossierSignatureView, SigningClientType, SigningRequestCreate, SigningRequestCreated,
    SigningTrust, SigningVerdict,
};
use kentos_imza_domain::BatchResult;
use reqwest::Method;
use uuid::Uuid;

use super::{Cloud, FILE_TIMEOUT, TIMEOUT};
use crate::failure::ApiFailure;

/// A signing request and a delivered result make the server read every file's stored bytes for its digest: a package
/// of large project files can take longer than an ordinary call's 30 s, and a delivery cut short by the client would
/// tell the signer it failed while the server went on to keep the signatures.
const DIGESTS: Duration = FILE_TIMEOUT;

impl Cloud {
    /// The key the server signs its signing requests with (`GET /v1/signing/trust`).
    pub fn signing_trust(
        &self,
    ) -> impl Future<Output = Result<SigningTrust, ApiFailure>> + Send + 'static {
        self.get(self.inner.url("/v1/signing/trust"))
    }

    /// A signing request for files of the dossier, signed in this process (`POST /v1/dossiers/{id}/signing`): the
    /// server reads their stored bytes, computes the digests itself and answers the signed envelope.
    pub fn signing_request(
        &self,
        dossier: Uuid,
        file_ids: Vec<String>,
    ) -> impl Future<Output = Result<SigningRequestCreated, ApiFailure>> + Send + 'static {
        let body = SigningRequestCreate {
            client_type: SigningClientType::Desktop,
            file_ids,
        };
        self.call(move |inner| async move {
            let url = inner.url(&format!("/v1/dossiers/{dossier}/signing"))?;
            let b = super::Inner::json_body(inner.request(Method::POST, url, DIGESTS), &body)?;
            inner.json(b, DIGESTS).await
        })
    }

    /// The signed result delivered for the server's own check (`POST /v1/signing/{tx}/result`).
    pub fn signing_result(
        &self,
        transaction: String,
        result: BatchResult,
    ) -> impl Future<Output = Result<SigningVerdict, ApiFailure>> + Send + 'static {
        self.call(move |inner| async move {
            let url = inner.url(&format!("/v1/signing/{transaction}/result"))?;
            let b = super::Inner::json_body(inner.request(Method::POST, url, DIGESTS), &result)?;
            inner.json(b, DIGESTS).await
        })
    }

    /// A request given up before it was signed (`POST /v1/signing/{tx}/cancel`).
    pub fn signing_cancel(
        &self,
        transaction: String,
    ) -> impl Future<Output = Result<(), ApiFailure>> + Send + 'static {
        self.call(move |inner| async move {
            let url = inner.url(&format!("/v1/signing/{transaction}/cancel"))?;
            inner
                .send(inner.request(Method::POST, url, TIMEOUT), TIMEOUT)
                .await
                .map(|_| ())
        })
    }

    /// The verified signatures kept on the dossier's files (`GET /v1/dossiers/{id}/signatures`).
    pub fn dossier_signatures(
        &self,
        dossier: Uuid,
    ) -> impl Future<Output = Result<Vec<DossierSignatureView>, ApiFailure>> + Send + 'static {
        self.get(
            self.inner
                .url(&format!("/v1/dossiers/{dossier}/signatures")),
        )
    }
}
