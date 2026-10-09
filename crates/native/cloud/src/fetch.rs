//! Map services' requests (docs/adr/0208 §6): a tile, a capabilities
//! document, a style, a page of objects, a token. They run on the cloud's
//! runtime, many at once without a thread each: at most [`TOTAL`] in
//! flight, and at most the host's own limit to one host (two for
//! OpenStreetMap's servers, as their tile policy asks; six otherwise). A
//! request whose `cancel` is set before it starts never leaves, and one set
//! while it waits for an answer is dropped. Redirects are followed five
//! times, a request gets 30 seconds. The answer is whatever the server said:
//! the caller reads the status (a tile's 404 is an empty tile, not a fault).
//! A connection's proof is in the request's headers or address already
//! (`kentos_services::auth::apply`); nothing here keeps cookies.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::Duration;

use reqwest::Client;
use reqwest::header::{HeaderName, HeaderValue};
use tokio::sync::Semaphore;

/// The most requests in flight at once.
pub const TOTAL: usize = 24;

/// A request's method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
}

/// A request to a map service.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
}

/// What the server answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    /// Header names in lower case, as HTTP/2 writes them.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    /// The address answered, after redirects.
    pub url: String,
}

impl Response {
    /// A header's value by its name in any case.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// Why no answer came, in the user's words, and whether trying again may help.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    pub message: String,
    pub retryable: bool,
    /// The request was cancelled before it got an answer.
    pub cancelled: bool,
}

fn client() -> Result<&'static Client, Failure> {
    static CLIENT: OnceLock<Result<Client, String>> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(30))
                .redirect(reqwest::redirect::Policy::limited(5))
                // OpenStreetMap's tile policy asks for an agent that names the
                // program and where to find it (kentos_services::user_agent).
                .user_agent(concat!(
                    "KentOS-CAD/",
                    env!("CARGO_PKG_VERSION"),
                    " (+https://github.com/mcihad/kentos-cad)"
                ))
                .build()
                .map_err(|e| format!("Harita servisi istemcisi kurulamadı: {e}."))
        })
        .as_ref()
        .map_err(|message| Failure {
            message: message.clone(),
            retryable: false,
            cancelled: false,
        })
}

/// A host's own semaphore, made with `limit` permits the first time it is asked for.
fn host(name: &str, limit: usize) -> Arc<Semaphore> {
    static HOSTS: OnceLock<Mutex<HashMap<String, Arc<Semaphore>>>> = OnceLock::new();
    let hosts = HOSTS.get_or_init(Default::default);
    let mut hosts = hosts.lock().unwrap_or_else(PoisonError::into_inner);
    hosts
        .entry(name.to_owned())
        .or_insert_with(|| Arc::new(Semaphore::new(limit.clamp(1, TOTAL))))
        .clone()
}

fn total() -> &'static Arc<Semaphore> {
    static TOTAL_PERMITS: OnceLock<Arc<Semaphore>> = OnceLock::new();
    TOTAL_PERMITS.get_or_init(|| Arc::new(Semaphore::new(TOTAL)))
}

/// The host of an address (its authority without a user and a port), lower case.
fn host_of(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let authority = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let host = if authority.starts_with('[') {
        authority
            .split(']')
            .next()
            .map_or(authority, |h| h.trim_start_matches('['))
    } else {
        authority.split(':').next().unwrap_or(authority)
    };
    host.to_ascii_lowercase()
}

const CANCELLED: &str = "İstek bırakıldı.";

async fn send(req: Request, limit: usize, cancel: Arc<AtomicBool>) -> Result<Response, Failure> {
    let stop = || Failure {
        message: CANCELLED.into(),
        retryable: false,
        cancelled: true,
    };
    let _all = total().clone().acquire_owned().await.map_err(|_| stop())?;
    let _one = host(&host_of(&req.url), limit)
        .acquire_owned()
        .await
        .map_err(|_| stop())?;
    if cancel.load(Ordering::Relaxed) {
        return Err(stop());
    }
    let c = client()?;
    let mut builder = match req.method {
        Method::Get => c.get(&req.url),
        Method::Post => c.post(&req.url),
    };
    for (k, v) in &req.headers {
        let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(k.as_bytes()),
            HeaderValue::from_str(v),
        ) else {
            return Err(Failure {
                message: format!("“{k}” başlığı gönderilemiyor: adı ya da değeri HTTP'ye uymuyor."),
                retryable: false,
                cancelled: false,
            });
        };
        builder = builder.header(name, value);
    }
    if let Some(body) = req.body {
        builder = builder.body(body);
    }
    let host = host_of(&req.url);
    let res = builder.send().await.map_err(|e| Failure {
        message: if e.is_timeout() {
            format!("{host} 30 saniyede cevap vermedi.")
        } else if e.is_connect() {
            format!("{host} sunucusuna bağlanılamadı; ağ bağlantısını denetleyin.")
        } else {
            format!("{host} sunucusuna istek gitmedi: {e}.")
        },
        retryable: e.is_timeout() || e.is_connect() || e.is_request(),
        cancelled: false,
    })?;
    let status = res.status().as_u16();
    let url = res.url().to_string();
    let headers = res
        .headers()
        .iter()
        .filter_map(|(k, v)| {
            v.to_str()
                .ok()
                .map(|v| (k.as_str().to_owned(), v.to_owned()))
        })
        .collect();
    let body = res.bytes().await.map_err(|e| Failure {
        message: format!("{host} sunucusunun cevabı yarıda kesildi: {e}."),
        retryable: true,
        cancelled: false,
    })?;
    Ok(Response {
        status,
        headers,
        body: body.to_vec(),
        url,
    })
}

/// Sends `req` on the cloud's runtime, at most `limit` at once to its host;
/// `done` gets the answer on one of the runtime's threads (it must not block
/// long). Setting `cancel` drops it if it has not got its answer yet.
pub fn spawn(
    req: Request,
    limit: usize,
    cancel: Arc<AtomicBool>,
    done: impl FnOnce(Result<Response, Failure>) + Send + 'static,
) {
    let watch = cancel.clone();
    let work = async move {
        let stopped = async move {
            while !watch.load(Ordering::Relaxed) {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        };
        tokio::select! {
            answer = send(req, limit, cancel) => answer,
            () = stopped => Err(Failure { message: CANCELLED.into(), retryable: false, cancelled: true }),
        }
    };
    match crate::runtime::handle() {
        Ok(rt) => {
            rt.spawn(async move { done(work.await) });
        }
        Err(f) => done(Err(Failure {
            message: f.message.clone(),
            retryable: false,
            cancelled: false,
        })),
    }
}

/// Sends `req` and waits on this thread (one of the desktop's own, never the
/// runtime's): capabilities, a style, a page of objects.
pub fn wait(req: Request, limit: usize) -> Result<Response, Failure> {
    let (tx, rx) = std::sync::mpsc::channel();
    spawn(
        req,
        limit,
        Arc::new(AtomicBool::new(false)),
        move |answer| {
            let _ = tx.send(answer);
        },
    );
    rx.recv().unwrap_or_else(|_| {
        Err(Failure {
            message: "İstek yarıda kaldı.".into(),
            retryable: true,
            cancelled: false,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts_are_the_authority_without_user_and_port() {
        assert_eq!(
            host_of("https://a.tile.openstreetmap.org/1/2/3.png"),
            "a.tile.openstreetmap.org"
        );
        assert_eq!(
            host_of("https://ali:gizli@CBS.Example.gov.tr:8443/wms?x=1"),
            "cbs.example.gov.tr"
        );
        assert_eq!(host_of("http://[::1]:8080/a"), "::1");
    }

    #[test]
    fn a_cancelled_request_never_leaves() {
        let cancel = Arc::new(AtomicBool::new(true));
        let (tx, rx) = std::sync::mpsc::channel();
        spawn(
            Request {
                method: Method::Get,
                url: "http://127.0.0.1:9/hic".into(),
                headers: Vec::new(),
                body: None,
            },
            2,
            cancel,
            move |a| {
                let _ = tx.send(a);
            },
        );
        let answer = rx.recv_timeout(Duration::from_secs(5)).expect("an answer");
        assert!(answer.is_err_and(|f| f.cancelled));
    }
}
