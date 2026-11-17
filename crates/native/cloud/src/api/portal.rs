//! The cloud portal's dossiers as the desktop works with them (docs/adr/9000 §5–8, docs/adr/9001): who the account is
//! on the portal, the catalog's names, a professional's and an organisation's lists, one dossier with the steps the
//! server allows now, a step taken with the status the person saw, a message, the open cloud project's present state
//! added to the package being prepared, a pending file taken out of it, a file's bytes checked against their SHA-256,
//! and a KentOS project file opened as a new cloud project.
//!
//! - `GET /v1/portal/me`, `GET /v1/portal/catalog`;
//! - `GET /v1/me/dossiers?…` (a professional's), `GET /v1/tenants/{tenant}/dossiers?…` (an organisation's);
//! - `GET /v1/dossiers/{id}`, `POST …/actions`, `POST …/messages`;
//! - `POST …/files/from-project`, `DELETE …/files/{file}`, `GET …/files/{file}`, `POST …/files/{file}/import`.
//!
//! The server decides every step and words every name and sentence; the desktop shows what it answers.

use std::future::Future;

use kentos_contracts::{
    DossierActionRequest, DossierDetail, DossierDirection, DossierFileImport, DossierFileImported,
    DossierFileView, DossierList, DossierMessageView, DossierStatus, FromProject, MessagePost,
    PortalCatalog, PortalMe,
};
use reqwest::{Method, Url};
use uuid::Uuid;

use super::{Cloud, Download, Inner, Progress, TIMEOUT, query_text};
use crate::failure::ApiFailure;

/// Which dossiers a list asks for: the server's `direction`, `status` (comma separated), `q`, `awaiting`, `limit` and
/// `offset`. Empty fields are left out of the address.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DossierQuery {
    pub direction: Option<DossierDirection>,
    pub statuses: Vec<DossierStatus>,
    /// Words of its number, title, place or the other side.
    pub text: Option<String>,
    /// Only those waiting for the asking side.
    pub awaiting: bool,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl DossierQuery {
    /// The query written onto `url`.
    fn written(&self, mut url: Url) -> Url {
        {
            let mut q = url.query_pairs_mut();
            if let Some(d) = &self.direction {
                q.append_pair("direction", &query_text(d));
            }
            if !self.statuses.is_empty() {
                let statuses: Vec<String> = self.statuses.iter().map(query_text).collect();
                q.append_pair("status", &statuses.join(","));
            }
            if let Some(text) = self
                .text
                .as_deref()
                .map(str::trim)
                .filter(|t| !t.is_empty())
            {
                q.append_pair("q", text);
            }
            if self.awaiting {
                q.append_pair("awaiting", "true");
            }
            if let Some(limit) = self.limit {
                q.append_pair("limit", &limit.to_string());
            }
            if let Some(offset) = self.offset {
                q.append_pair("offset", &offset.to_string());
            }
        }
        // `query_pairs_mut` leaves a bare `?` when nothing was written.
        if url.query() == Some("") {
            url.set_query(None);
        }
        url
    }
}

impl Inner {
    /// `/v1/dossiers/{id}` and the path after it.
    fn dossier_url(&self, id: Uuid, rest: &str) -> Result<Url, ApiFailure> {
        self.url(&format!("/v1/dossiers/{id}{rest}"))
    }
}

impl Cloud {
    /// Who the account is on the portal: a professional, a member of organisations, a platform administrator
    /// (`GET /v1/portal/me`).
    pub fn portal_me(&self) -> impl Future<Output = Result<PortalMe, ApiFailure>> + Send + 'static {
        self.get(self.inner.url("/v1/portal/me"))
    }

    /// The names the portal speaks in: kinds, documents, professions, statuses and their tones
    /// (`GET /v1/portal/catalog`).
    pub fn portal_catalog(
        &self,
    ) -> impl Future<Output = Result<PortalCatalog, ApiFailure>> + Send + 'static {
        self.get(self.inner.url("/v1/portal/catalog"))
    }

    /// A professional's dossiers: the applications they sent and the transfers sent to them (`GET /v1/me/dossiers`).
    pub fn my_dossiers(
        &self,
        query: &DossierQuery,
    ) -> impl Future<Output = Result<DossierList, ApiFailure>> + Send + 'static {
        let query = query.clone();
        self.get(self.inner.url("/v1/me/dossiers").map(|u| query.written(u)))
    }

    /// An organisation's dossiers, as one of its members sees them (`GET /v1/tenants/{tenant}/dossiers`).
    pub fn organization_dossiers(
        &self,
        tenant: Uuid,
        query: &DossierQuery,
    ) -> impl Future<Output = Result<DossierList, ApiFailure>> + Send + 'static {
        let query = query.clone();
        self.get(
            self.inner
                .url(&format!("/v1/tenants/{tenant}/dossiers"))
                .map(|u| query.written(u)),
        )
    }

    /// One dossier from the caller's side: its packages, messages, timeline and the steps allowed now
    /// (`GET /v1/dossiers/{id}`).
    pub fn dossier(
        &self,
        id: Uuid,
    ) -> impl Future<Output = Result<DossierDetail, ApiFailure>> + Send + 'static {
        self.get(self.inner.dossier_url(id, ""))
    }

    /// A step on the dossier with the status the person saw (`POST …/actions`): the dossier as it is after it. Another's
    /// step in between is the server's conflict (409), never an overwrite.
    pub fn dossier_step(
        &self,
        id: Uuid,
        step: DossierActionRequest,
    ) -> impl Future<Output = Result<DossierDetail, ApiFailure>> + Send + 'static {
        self.call(move |inner| async move {
            let url = inner.dossier_url(id, "/actions")?;
            let b = Inner::json_body(inner.request(Method::POST, url, TIMEOUT), &step)?;
            inner.json(b, TIMEOUT).await
        })
    }

    /// A message to the other side (`POST …/messages`).
    pub fn dossier_message(
        &self,
        id: Uuid,
        post: MessagePost,
    ) -> impl Future<Output = Result<DossierMessageView, ApiFailure>> + Send + 'static {
        self.call(move |inner| async move {
            let url = inner.dossier_url(id, "/messages")?;
            let b = Inner::json_body(inner.request(Method::POST, url, TIMEOUT), &post)?;
            inner.json(b, TIMEOUT).await
        })
    }

    /// A cloud project's present state as a file of the package being prepared (`POST …/files/from-project`): a file
    /// project's newest revision, a database project's snapshot of now.
    pub fn dossier_add_project(
        &self,
        id: Uuid,
        from: FromProject,
    ) -> impl Future<Output = Result<DossierFileView, ApiFailure>> + Send + 'static {
        self.call(move |inner| async move {
            let url = inner.dossier_url(id, "/files/from-project")?;
            // A database project's snapshot is written on the server while the request waits.
            let b = Inner::json_body(inner.request(Method::POST, url, super::FILE_TIMEOUT), &from)?;
            inner.json(b, super::FILE_TIMEOUT).await
        })
    }

    /// A file of the package being prepared taken out of it (`DELETE …/files/{file}`); a sent one stays.
    pub fn dossier_remove_file(
        &self,
        id: Uuid,
        file: Uuid,
    ) -> impl Future<Output = Result<(), ApiFailure>> + Send + 'static {
        self.call(move |inner| async move {
            let url = inner.dossier_url(id, &format!("/files/{file}"))?;
            inner
                .send(inner.request(Method::DELETE, url, TIMEOUT), TIMEOUT)
                .await
                .map(|_| ())
        })
    }

    /// A file's bytes (`GET …/files/{file}`), checked against the SHA-256 the server keeps for it; a file never
    /// changes, so a download cut short goes on from where it stopped.
    pub fn dossier_file(
        &self,
        id: Uuid,
        file: Uuid,
        progress: Option<Progress>,
    ) -> impl Future<Output = Result<Download, ApiFailure>> + Send + 'static {
        self.call(move |inner| async move {
            let url = inner.dossier_url(id, &format!("/files/{file}"))?;
            inner.download(url, progress, true).await
        })
    }

    /// A KentOS project file of the dossier as a new file project of the caller (`POST …/files/{file}/import`): in
    /// their personal space, or in `into`'s organisation. The answer names the project to open.
    pub fn dossier_import(
        &self,
        id: Uuid,
        file: Uuid,
        into: DossierFileImport,
    ) -> impl Future<Output = Result<DossierFileImported, ApiFailure>> + Send + 'static {
        self.call(move |inner| async move {
            let url = inner.dossier_url(id, &format!("/files/{file}/import"))?;
            let b = Inner::json_body(inner.request(Method::POST, url, super::FILE_TIMEOUT), &into)?;
            inner.json(b, super::FILE_TIMEOUT).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_query_carries_only_what_is_asked() {
        let base = || Url::parse("http://127.0.0.1:8787/v1/me/dossiers").unwrap();
        assert_eq!(
            DossierQuery::default().written(base()).as_str(),
            "http://127.0.0.1:8787/v1/me/dossiers"
        );
        let asked = DossierQuery {
            direction: Some(DossierDirection::Transfer),
            statuses: vec![DossierStatus::Sent, DossierStatus::RevisionRequested],
            text: Some("  412 ada ".into()),
            awaiting: true,
            limit: Some(100),
            offset: None,
        };
        assert_eq!(
            asked.written(base()).as_str(),
            "http://127.0.0.1:8787/v1/me/dossiers?direction=transfer&status=sent%2Crevision_requested\
             &q=412+ada&awaiting=true&limit=100"
        );
        let blank = DossierQuery {
            text: Some("   ".into()),
            ..DossierQuery::default()
        };
        assert_eq!(blank.written(base()).query(), None);
    }
}
