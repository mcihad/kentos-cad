//! The cloud API over HTTP (docs/adr/0040): the routes the web's
//! `HttpCloudApi` uses (apps/web/src/app/cloud/api.ts), for the desktop.
//!
//! - **Signing in** with a local account's login and password. The server's
//!   session token is kept in memory only, never written to a file or a log
//!   (TODOS.md SYNC-13), and goes with every request as the session cookie
//!   together with `x-kentos-client: desktop` (apps/api/src/http/auth.rs).
//!   A 401 forgets it: the account signs in again.
//! - **Plain http only to this computer:** anywhere else the session would
//!   cross the network readable, so a server elsewhere needs https.
//! - Every call returns a future that runs on the cloud's own runtime
//!   (runtime.rs) and can be awaited on any executor; dropping it cancels
//!   the request. Failures are [`ApiFailure`], with the server's message.

use std::fmt;
use std::future::Future;
use std::net::IpAddr;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use kentos_contracts::{
    ApiError, AuthConfig, CatalogSort, CatalogView, CommandEnvelope, EventPage, FeaturePage,
    FileRevisions, FileUpload, FileUploadBegin, LoginRequest, Me, ProjectCreate, ProjectInfo,
    ProjectPage, ProjectType,
};
use reqwest::header::{self, HeaderMap};
use reqwest::{Method, RequestBuilder, Response, Url};
use serde::Serialize;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::failure::ApiFailure;
use crate::runtime::run;

/// How long an ordinary request may take (the web's too).
const TIMEOUT: Duration = Duration::from_secs(30);
/// How long a file's bytes may take either way (the server's upload limit, docs/adr/0031).
pub const FILE_TIMEOUT: Duration = Duration::from_secs(600);
/// What the desktop says it is (the server's `x-kentos-client`).
pub const CLIENT: &str = "desktop";
const CLIENT_HEADER: &str = "x-kentos-client";
const SESSION_COOKIE: &str = "kentos_session";

/// Where a long transfer stands: bytes done, and all of them when known (0 when not).
pub type Progress = Arc<dyn Fn(u64, u64) + Send + Sync>;

/// The session token: sent with requests, never shown.
#[derive(Clone, PartialEq, Eq)]
struct Token(String);

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Token(gizli)")
    }
}

struct Inner {
    /// The server's address without a trailing slash (a path prefix is kept).
    base: String,
    http: reqwest::Client,
    session: RwLock<Option<Token>>,
}

impl fmt::Debug for Inner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cloud")
            .field("base", &self.base)
            .field("signed_in", &self.token().is_some())
            .finish()
    }
}

/// A connection to one KentOS server, signed in or not. Cheap to clone:
/// the clones share the connection pool and the session.
#[derive(Clone, Debug)]
pub struct Cloud {
    inner: Arc<Inner>,
}

/// One page of a view of the account's catalog (`GET /v1/me/catalog`, docs/adr/0028).
#[derive(Clone, Debug, PartialEq)]
pub struct CatalogQuery {
    pub view: CatalogView,
    /// The workspace of the `organization` view; for the others, keeps the list to it.
    pub tenant: Option<Uuid>,
    /// Words that must all be in the name, description or tags.
    pub text: Option<String>,
    pub project_type: Option<ProjectType>,
    pub sort: Option<CatalogSort>,
    pub limit: Option<u32>,
    /// The `next` of the page before.
    pub after: Option<String>,
}

impl CatalogQuery {
    pub fn new(view: CatalogView) -> Self {
        Self {
            view,
            tenant: None,
            text: None,
            project_type: None,
            sort: None,
            limit: None,
            after: None,
        }
    }
}

/// A `.kcad` file from the server, checked against the SHA-256 it came with.
#[derive(Clone, Debug, PartialEq)]
pub struct Download {
    pub bytes: Vec<u8>,
    pub sha256: String,
    /// The revision it is (`x-kentos-revision`): a file project's revision,
    /// or the data revision of a database project's snapshot.
    pub revision: Option<String>,
    /// A snapshot's event cursor of the same moment (docs/adr/0033).
    pub event_cursor: Option<String>,
}

/// An enum of the contracts as its query text (`mine`, `updated` …).
fn query_text<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(s)) => s,
        _ => String::new(),
    }
}

/// The session token in a login answer's cookies.
fn session_of(headers: &HeaderMap) -> Option<Token> {
    headers
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find_map(|cookie| {
            let (name, value) = cookie.split(';').next()?.split_once('=')?;
            let value = value.trim();
            (name.trim() == SESSION_COOKIE && !value.is_empty()).then(|| Token(value.to_string()))
        })
}

/// The server's address, checked: http or https with a host and nothing
/// after the path; plain http only to this computer.
fn server_address(server: &str) -> Result<String, ApiFailure> {
    let bad =
        |why: &str| ApiFailure::local(format!("Sunucu adresi kullanılamaz ({server}): {why}"));
    let url = Url::parse(server.trim()).map_err(|e| bad(&e.to_string()))?;
    if !url.username().is_empty() || url.password().is_some() {
        return Err(bad("adreste kullanıcı adı ya da parola olmamalı"));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(bad("adres ? ya da # içermemeli"));
    }
    let Some(host) = url.host_str() else {
        return Err(bad("sunucunun adı yok"));
    };
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    let local = bare.eq_ignore_ascii_case("localhost")
        || bare.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback());
    match url.scheme() {
        "https" => {}
        "http" if local => {}
        "http" => {
            return Err(bad(
                "şifresiz http yalnız bu bilgisayardaki sunucuya kullanılır; oturumunuz ağda okunabilirdi. https ile girin",
            ));
        }
        other => return Err(bad(&format!("{other} değil, http ya da https olmalı"))),
    }
    Ok(url.as_str().trim_end_matches('/').to_string())
}

impl Inner {
    fn token(&self) -> Option<Token> {
        self.session.read().ok().and_then(|s| s.clone())
    }

    fn set_token(&self, token: Option<Token>) {
        if let Ok(mut s) = self.session.write() {
            *s = token;
        }
    }

    fn url(&self, path: &str) -> Result<Url, ApiFailure> {
        Url::parse(&format!("{}{path}", self.base))
            .map_err(|e| ApiFailure::local(format!("İstek adresi kurulamadı ({path}): {e}")))
    }

    fn project_url(&self, tenant: Uuid, project: Uuid, rest: &str) -> Result<Url, ApiFailure> {
        self.url(&format!("/v1/tenants/{tenant}/projects/{project}{rest}"))
    }

    /// A request with the desktop's headers and, when signed in, the session.
    fn request(&self, method: Method, url: Url, timeout: Duration) -> RequestBuilder {
        let b = self
            .http
            .request(method, url)
            .timeout(timeout)
            .header(header::ACCEPT, "application/json")
            .header(CLIENT_HEADER, CLIENT);
        match self.token() {
            Some(Token(t)) => b.header(header::COOKIE, format!("{SESSION_COOKIE}={t}")),
            None => b,
        }
    }

    fn json_body<B: Serialize>(b: RequestBuilder, body: &B) -> Result<RequestBuilder, ApiFailure> {
        let bytes = serde_json::to_vec(body)
            .map_err(|e| ApiFailure::local(format!("İstek yazılamadı: {e}")))?;
        Ok(b.header(header::CONTENT_TYPE, "application/json")
            .body(bytes))
    }

    /// Sends it; an error status becomes the server's failure. A 401 on a
    /// request made with the session forgets the session.
    async fn send(&self, b: RequestBuilder, waited: Duration) -> Result<Response, ApiFailure> {
        let res = b
            .send()
            .await
            .map_err(|e| ApiFailure::unreachable(&e, waited))?;
        if res.status().is_success() {
            return Ok(res);
        }
        let failure = Self::failure(res).await;
        if failure.status == 401 {
            self.set_token(None);
        }
        Err(failure)
    }

    async fn failure(res: Response) -> ApiFailure {
        let status = res.status().as_u16();
        let retry_header = res
            .headers()
            .get(header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse().ok());
        match res.bytes().await {
            Ok(body) => match serde_json::from_slice::<ApiError>(&body) {
                Ok(error) => ApiFailure::answered(status, error, retry_header),
                Err(_) => ApiFailure::status_only(status),
            },
            Err(_) => ApiFailure::status_only(status),
        }
    }

    async fn json<T: DeserializeOwned>(
        &self,
        b: RequestBuilder,
        waited: Duration,
    ) -> Result<T, ApiFailure> {
        let res = self.send(b, waited).await?;
        let status = res.status().as_u16();
        let body = res
            .bytes()
            .await
            .map_err(|e| ApiFailure::unreachable(&e, waited))?;
        serde_json::from_slice(&body).map_err(|e| ApiFailure::unreadable(status, "JSON", e))
    }

    /// A `.kcad` file read in parts, checked against its entity tag (SHA-256).
    async fn download(&self, url: Url, progress: Option<Progress>) -> Result<Download, ApiFailure> {
        let mut res = self
            .send(self.request(Method::GET, url, FILE_TIMEOUT), FILE_TIMEOUT)
            .await?;
        let text = |name: &str| {
            res.headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(|v| v.trim().to_string())
        };
        let tag = text("etag").map(|t| t.trim_matches('"').to_string());
        let revision = text("x-kentos-revision");
        let event_cursor = text("x-kentos-event-cursor");
        let total = res.content_length().unwrap_or(0);
        let mut bytes = Vec::with_capacity(usize::try_from(total).unwrap_or(0).min(1 << 28));
        while let Some(part) = res
            .chunk()
            .await
            .map_err(|e| ApiFailure::unreachable(&e, FILE_TIMEOUT))?
        {
            bytes.extend_from_slice(&part);
            if let Some(p) = &progress {
                p(bytes.len() as u64, total);
            }
        }
        let sha256 = hex_sha256(&bytes);
        match tag {
            Some(tag) if tag == sha256 => Ok(Download {
                bytes,
                sha256,
                revision,
                event_cursor,
            }),
            Some(tag) => Err(ApiFailure::local(format!(
                "İndirilen dosya bozuk geldi (SHA-256 {sha256}, sunucunun bildirdiği {tag}); yeniden indirin."
            ))
            .with_code("corrupt")),
            None => Err(ApiFailure::unreadable(
                200,
                "dosya",
                "SHA-256'sı (ETag) bildirilmemiş",
            )),
        }
    }
}

/// Lowercase hex SHA-256 of `bytes`.
pub fn hex_sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

impl Cloud {
    /// A connection to the server at `server` (`https://kentos.kurum.gov.tr`,
    /// `http://127.0.0.1:8787`), not signed in. Nothing is sent yet.
    pub fn new(server: &str) -> Result<Self, ApiFailure> {
        let base = server_address(server)?;
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            // A redirect would take the session to an address nobody chose.
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("KentOS-CAD/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| ApiFailure::local(format!("Bulut bağlantısı kurulamadı: {e}")))?;
        Ok(Self {
            inner: Arc::new(Inner {
                base,
                http,
                session: RwLock::new(None),
            }),
        })
    }

    /// The server's address as it is used.
    pub fn server(&self) -> &str {
        &self.inner.base
    }

    /// Whether a session is held (the server may still have ended it).
    pub fn signed_in(&self) -> bool {
        self.inner.token().is_some()
    }

    fn call<T, F>(
        &self,
        work: impl FnOnce(Arc<Inner>) -> F,
    ) -> impl Future<Output = Result<T, ApiFailure>> + Send + 'static
    where
        T: Send + 'static,
        F: Future<Output = Result<T, ApiFailure>> + Send + 'static,
    {
        run(work(self.inner.clone()))
    }

    fn get<T: DeserializeOwned + Send + 'static>(
        &self,
        url: Result<Url, ApiFailure>,
    ) -> impl Future<Output = Result<T, ApiFailure>> + Send + 'static {
        self.call(move |inner| async move {
            let b = inner.request(Method::GET, url?, TIMEOUT);
            inner.json(b, TIMEOUT).await
        })
    }

    /// How this server signs people in (`GET /v1/auth/config`).
    pub fn auth_config(
        &self,
    ) -> impl Future<Output = Result<AuthConfig, ApiFailure>> + Send + 'static {
        self.get(self.inner.url("/v1/auth/config"))
    }

    /// Signs in with a local account; the session is kept for the requests after.
    pub fn sign_in(
        &self,
        login: &str,
        password: &str,
    ) -> impl Future<Output = Result<Me, ApiFailure>> + Send + 'static {
        let body = LoginRequest {
            login: login.trim().to_string(),
            password: password.to_string(),
        };
        self.call(move |inner| async move {
            let url = inner.url("/v1/auth/login")?;
            // Without the old session: a wrong password must not sign the account out.
            let b = inner
                .http
                .post(url)
                .timeout(TIMEOUT)
                .header(header::ACCEPT, "application/json")
                .header(CLIENT_HEADER, CLIENT);
            let res = Inner::json_body(b, &body)?
                .send()
                .await
                .map_err(|e| ApiFailure::unreachable(&e, TIMEOUT))?;
            if !res.status().is_success() {
                return Err(Inner::failure(res).await);
            }
            let token = session_of(res.headers()).ok_or_else(|| {
                ApiFailure::unreadable(200, "giriş", "sunucu oturum çerezi göndermedi")
            })?;
            let status = res.status().as_u16();
            let body = res
                .bytes()
                .await
                .map_err(|e| ApiFailure::unreachable(&e, TIMEOUT))?;
            let me: Me = serde_json::from_slice(&body)
                .map_err(|e| ApiFailure::unreadable(status, "hesap", e))?;
            inner.set_token(Some(token));
            Ok(me)
        })
    }

    /// Signs out: the session is forgotten here at once, and ended on the
    /// server when it answers (otherwise it ends by itself when it expires).
    pub fn sign_out(&self) -> impl Future<Output = Result<(), ApiFailure>> + Send + 'static {
        let token = self.inner.token();
        self.inner.set_token(None);
        self.call(move |inner| async move {
            let Some(Token(t)) = token else {
                return Ok(());
            };
            let b = inner
                .http
                .post(inner.url("/v1/auth/logout")?)
                .timeout(TIMEOUT)
                .header(CLIENT_HEADER, CLIENT)
                .header(header::COOKIE, format!("{SESSION_COOKIE}={t}"));
            let res = b
                .send()
                .await
                .map_err(|e| ApiFailure::unreachable(&e, TIMEOUT))?;
            if res.status().is_success() {
                Ok(())
            } else {
                Err(Inner::failure(res).await)
            }
        })
    }

    /// The signed-in account and its workspaces (`GET /v1/me`).
    pub fn me(&self) -> impl Future<Output = Result<Me, ApiFailure>> + Send + 'static {
        self.get(self.inner.url("/v1/me"))
    }

    /// One page of a catalog view (`GET /v1/me/catalog`).
    pub fn catalog(
        &self,
        query: &CatalogQuery,
    ) -> impl Future<Output = Result<ProjectPage, ApiFailure>> + Send + 'static {
        let url = self.inner.url("/v1/me/catalog").map(|mut url| {
            {
                let mut q = url.query_pairs_mut();
                q.append_pair("view", &query_text(&query.view));
                if let Some(t) = query.tenant {
                    q.append_pair("tenant", &t.to_string());
                }
                if let Some(text) = query
                    .text
                    .as_deref()
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
                {
                    q.append_pair("q", text);
                }
                if let Some(t) = &query.project_type {
                    q.append_pair("type", &query_text(t));
                }
                if let Some(s) = &query.sort {
                    q.append_pair("sort", &query_text(s));
                }
                if let Some(l) = query.limit {
                    q.append_pair("limit", &l.to_string());
                }
                if let Some(a) = &query.after {
                    q.append_pair("after", a);
                }
            }
            url
        });
        self.get(url)
    }

    /// What opening a project needs before its objects (`GET …/projects/{project}`).
    pub fn project(
        &self,
        tenant: Uuid,
        project: Uuid,
    ) -> impl Future<Output = Result<ProjectInfo, ApiFailure>> + Send + 'static {
        self.get(self.inner.project_url(tenant, project, ""))
    }

    /// A page of a database project's objects in id order (`GET …/features`).
    pub fn features(
        &self,
        tenant: Uuid,
        project: Uuid,
        after: Option<&str>,
        limit: u32,
    ) -> impl Future<Output = Result<FeaturePage, ApiFailure>> + Send + 'static {
        let url = self
            .inner
            .project_url(tenant, project, "/features")
            .map(|mut url| {
                {
                    let mut q = url.query_pairs_mut();
                    q.append_pair("limit", &limit.to_string());
                    if let Some(a) = after {
                        q.append_pair("after", a);
                    }
                }
                url
            });
        self.get(url)
    }

    /// Some objects of a database project by id (`GET …/features?ids=`).
    pub fn features_by_id(
        &self,
        tenant: Uuid,
        project: Uuid,
        ids: &[Uuid],
    ) -> impl Future<Output = Result<FeaturePage, ApiFailure>> + Send + 'static {
        let list = ids
            .iter()
            .map(Uuid::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let url = self
            .inner
            .project_url(tenant, project, "/features")
            .map(|mut url| {
                url.query_pairs_mut().append_pair("ids", &list);
                url
            });
        self.get(url)
    }

    /// A product command on the project the envelope names; the answer is the
    /// command's own output (`CommitResult` of `project.changes`, …).
    pub fn command<O: DeserializeOwned + Send + 'static>(
        &self,
        envelope: CommandEnvelope,
    ) -> impl Future<Output = Result<O, ApiFailure>> + Send + 'static {
        self.call(move |inner| async move {
            let tenant = Uuid::parse_str(&envelope.tenant_id)
                .map_err(|_| ApiFailure::local("Komutun kurum kimliği geçersiz."))?;
            let project = Uuid::parse_str(&envelope.project_id)
                .map_err(|_| ApiFailure::local("Komutun proje kimliği geçersiz."))?;
            let url = inner.project_url(tenant, project, "/commands")?;
            let b = Inner::json_body(inner.request(Method::POST, url, TIMEOUT), &envelope)?;
            inner.json(b, TIMEOUT).await
        })
    }

    /// A new project in `tenant` (`POST …/projects`); `key` makes a retry
    /// answer with the same project instead of making a second one.
    pub fn create_project(
        &self,
        tenant: Uuid,
        input: ProjectCreate,
        key: Uuid,
    ) -> impl Future<Output = Result<ProjectInfo, ApiFailure>> + Send + 'static {
        self.call(move |inner| async move {
            let url = inner.url(&format!("/v1/tenants/{tenant}/projects"))?;
            let b = inner
                .request(Method::POST, url, TIMEOUT)
                .header("idempotency-key", key.to_string());
            inner.json(Inner::json_body(b, &input)?, TIMEOUT).await
        })
    }

    /// The committed events after `after` (`GET …/events`).
    pub fn events(
        &self,
        tenant: Uuid,
        project: Uuid,
        after: &str,
    ) -> impl Future<Output = Result<EventPage, ApiFailure>> + Send + 'static {
        let url = self
            .inner
            .project_url(tenant, project, "/events")
            .map(|mut url| {
                url.query_pairs_mut().append_pair("after", after);
                url
            });
        self.get(url)
    }

    /// The committed events after `after`, waiting up to `wait` for one when
    /// none is new (a long poll, docs/adr/0044): the answer comes as soon as
    /// a commit lands, or empty when the wait ends.
    pub fn events_waiting(
        &self,
        tenant: Uuid,
        project: Uuid,
        after: &str,
        wait: Duration,
    ) -> impl Future<Output = Result<EventPage, ApiFailure>> + Send + 'static {
        let url = self
            .inner
            .project_url(tenant, project, "/events")
            .map(|mut url| {
                url.query_pairs_mut()
                    .append_pair("after", after)
                    .append_pair("wait", &wait.as_secs().max(1).to_string());
                url
            });
        // The request may take the whole wait, and a little more.
        let timeout = wait + Duration::from_secs(10);
        self.call(move |inner| async move {
            let b = inner.request(Method::GET, url?, timeout);
            inner.json(b, timeout).await
        })
    }

    /// A file project's revisions, newest first (`GET …/files`).
    pub fn file_revisions(
        &self,
        tenant: Uuid,
        project: Uuid,
    ) -> impl Future<Output = Result<FileRevisions, ApiFailure>> + Send + 'static {
        self.get(self.inner.project_url(tenant, project, "/files"))
    }

    /// One revision of a file project (`GET …/files/{n}`), checked against its SHA-256.
    pub fn download_revision(
        &self,
        tenant: Uuid,
        project: Uuid,
        revision: u64,
        progress: Option<Progress>,
    ) -> impl Future<Output = Result<Download, ApiFailure>> + Send + 'static {
        self.call(move |inner| async move {
            let url = inner.project_url(tenant, project, &format!("/files/{revision}"))?;
            inner.download(url, progress).await
        })
    }

    /// A database project as one `.kcad` file of one moment (`GET …/snapshot`, docs/adr/0033).
    pub fn snapshot(
        &self,
        tenant: Uuid,
        project: Uuid,
        progress: Option<Progress>,
    ) -> impl Future<Output = Result<Download, ApiFailure>> + Send + 'static {
        self.call(move |inner| async move {
            let url = inner.project_url(tenant, project, "/snapshot")?;
            inner.download(url, progress).await
        })
    }

    /// Opens an upload for a file of this size and SHA-256 (`POST …/uploads`).
    pub fn begin_upload(
        &self,
        tenant: Uuid,
        project: Uuid,
        begin: FileUploadBegin,
    ) -> impl Future<Output = Result<FileUpload, ApiFailure>> + Send + 'static {
        self.call(move |inner| async move {
            let url = inner.project_url(tenant, project, "/uploads")?;
            let b = Inner::json_body(inner.request(Method::POST, url, TIMEOUT), &begin)?;
            inner.json(b, TIMEOUT).await
        })
    }

    /// The upload's bytes (`PUT …/uploads/{upload}`); every try of the same
    /// upload shares them without a copy.
    pub fn send_upload(
        &self,
        tenant: Uuid,
        project: Uuid,
        upload: Uuid,
        bytes: bytes::Bytes,
    ) -> impl Future<Output = Result<FileUpload, ApiFailure>> + Send + 'static {
        self.call(move |inner| async move {
            let url = inner.project_url(tenant, project, &format!("/uploads/{upload}"))?;
            let b = inner
                .request(Method::PUT, url, FILE_TIMEOUT)
                .header(header::CONTENT_TYPE, "application/octet-stream")
                .body(bytes);
            inner.json(b, FILE_TIMEOUT).await
        })
    }

    /// How an upload stands (`GET …/uploads/{upload}`): whether its bytes
    /// arrived, asked after the answer to them was lost.
    pub fn upload_status(
        &self,
        tenant: Uuid,
        project: Uuid,
        upload: Uuid,
    ) -> impl Future<Output = Result<FileUpload, ApiFailure>> + Send + 'static {
        self.get(
            self.inner
                .project_url(tenant, project, &format!("/uploads/{upload}")),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderValue;

    #[test]
    fn plain_http_goes_only_to_this_computer() {
        assert_eq!(
            server_address("http://127.0.0.1:8787/").unwrap(),
            "http://127.0.0.1:8787"
        );
        assert_eq!(
            server_address("http://localhost:8787").unwrap(),
            "http://localhost:8787"
        );
        assert_eq!(
            server_address("http://[::1]:8787").unwrap(),
            "http://[::1]:8787"
        );
        assert_eq!(
            server_address(" https://kentos.kurum.gov.tr/cad/ ").unwrap(),
            "https://kentos.kurum.gov.tr/cad"
        );
        for refused in [
            "http://192.168.1.10:8787",
            "http://kentos.kurum.gov.tr",
            "ftp://kentos.kurum.gov.tr",
            "https://ayse:parola@kentos.kurum.gov.tr",
            "https://kentos.kurum.gov.tr/?x=1",
            "kentos.kurum.gov.tr",
        ] {
            let e = server_address(refused).unwrap_err();
            assert_eq!(e.code, "local", "{refused}");
            assert!(!e.transient(), "{refused}");
        }
    }

    #[test]
    fn the_session_is_read_from_the_login_answer_and_never_shown() {
        let mut h = HeaderMap::new();
        h.append(
            header::SET_COOKIE,
            HeaderValue::from_static("baska=1; Path=/"),
        );
        h.append(
            header::SET_COOKIE,
            HeaderValue::from_static(
                "kentos_session=abc123; Path=/; HttpOnly; SameSite=Strict; Max-Age=3600",
            ),
        );
        let token = session_of(&h).unwrap();
        assert_eq!(token, Token("abc123".into()));
        assert_eq!(format!("{token:?}"), "Token(gizli)");
        // An emptied cookie (signing out) is no session.
        let mut cleared = HeaderMap::new();
        cleared.append(
            header::SET_COOKIE,
            HeaderValue::from_static("kentos_session=; Max-Age=0"),
        );
        assert_eq!(session_of(&cleared), None);
        // The connection's debug text says whether it is signed in, never the token.
        let cloud = Cloud::new("http://127.0.0.1:1").unwrap();
        cloud.inner.set_token(Some(Token("gizli-belirteç".into())));
        let shown = format!("{cloud:?}");
        assert!(
            shown.contains("signed_in: true") && !shown.contains("belirteç"),
            "{shown}"
        );
    }

    #[test]
    fn catalog_queries_carry_only_what_is_asked() {
        assert_eq!(query_text(&CatalogView::Organization), "organization");
        assert_eq!(query_text(&CatalogSort::Updated), "updated");
    }
}
