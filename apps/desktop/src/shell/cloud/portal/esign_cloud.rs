//! The server's e-signature endpoints over the signed-in cloud client (docs/adr/9002 §7; the server's side is
//! docs/adr/9004): `GET /v1/signing/trust`, `POST /v1/dossiers/{id}/signing`, `POST /v1/signing/{tx}/result`,
//! `POST /v1/signing/{tx}/cancel`, `GET /v1/dossiers/{id}/signatures`. A failure is said in the server's own words.

use std::sync::Arc;

use kentos_cloud::{ApiFailure, Cloud};
use kentos_commands::{DossierSignatureView, SigningRequestCreated, SigningTrust, SigningVerdict};
use kentos_imza_domain::BatchResult;

use super::esign_server::{Answer, EsignServer};
use crate::app::App;

/// The signed-in client's e-signature endpoints; none without a session.
pub fn server(app: &App) -> Option<Arc<dyn EsignServer>> {
    app.cloud
        .signed_in()
        .map(|c| Arc::new(CloudServer(c.clone())) as Arc<dyn EsignServer>)
}

struct CloudServer(Cloud);

fn words(f: ApiFailure) -> String {
    f.to_string()
}

fn uuid(id: &str) -> Result<kentos_domain::Uuid, String> {
    crate::cloud::uuid(id).ok_or_else(|| format!("“{id}” bir dosya kimliği değil."))
}

impl EsignServer for CloudServer {
    fn trust(&self) -> Answer<SigningTrust> {
        let f = self.0.signing_trust();
        Box::pin(async move { f.await.map_err(words) })
    }

    fn request(&self, dossier: String, file_ids: Vec<String>) -> Answer<SigningRequestCreated> {
        let id = match uuid(&dossier) {
            Ok(id) => id,
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        let f = self.0.signing_request(id, file_ids);
        Box::pin(async move { f.await.map_err(words) })
    }

    fn deliver(&self, transaction: String, result: BatchResult) -> Answer<SigningVerdict> {
        let f = self.0.signing_result(transaction, result);
        Box::pin(async move { f.await.map_err(words) })
    }

    fn cancel(&self, transaction: String) -> Answer<()> {
        let f = self.0.signing_cancel(transaction);
        Box::pin(async move { f.await.map_err(words) })
    }

    fn signatures(&self, dossier: String) -> Answer<Vec<DossierSignatureView>> {
        let id = match uuid(&dossier) {
            Ok(id) => id,
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        let f = self.0.dossier_signatures(id);
        Box::pin(async move { f.await.map_err(words) })
    }
}
