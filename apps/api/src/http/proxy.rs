//! The map services' proxy (docs/adr/0208 §13): `GET` and `POST /v1/proxy?url=…`
//! for a signed-in user whose browser cannot reach a service itself (CORS).
//! The target's name is resolved here and the connection made to the
//! addresses resolved (a resolver of our own, so no second lookup can lead
//! elsewhere); loopback, private, link-local (the clouds' metadata), CGNAT,
//! multicast and the reserved ranges are refused unless `KENTOS_PROXY_ALLOW`
//! opens an organisation's own networks. At most three redirects, each
//! checked again; an answer of at most 32 MB within 30 seconds; no cookie
//! either way, no proxy of the environment (it would resolve the target
//! itself); only safe headers back. The headers for the target come as
//! `x-kentos-header-<name>`. At most 16 requests of a user at a time. The log
//! has the target's host and a count, never its path or query (a key may
//! be there).

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use kentos_application::AppError;
use kentos_contracts::ApiError;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use reqwest::{Url, redirect};
use serde::Deserialize;
use uuid::Uuid;

use super::AppState;
use super::auth::Caller;
use super::error::{Failure, request_id};

/// The largest answer passed on.
pub const MOST_BYTES: usize = 32 * 1024 * 1024;
/// How long an answer may take, redirects included.
pub const TIMEOUT: Duration = Duration::from_secs(30);
/// Redirects followed at most.
pub const MOST_REDIRECTS: usize = 3;
/// A user's requests on their way at most.
pub const PER_USER: usize = 16;
/// The prefix of a header meant for the target.
pub const HEADER_PREFIX: &str = "x-kentos-header-";

/// Headers never sent to a target: the connection's own, and what would let
/// the request act as someone else's (a cookie, a forwarded address).
const NEVER_SENT: [&str; 13] = [
    "host",
    "cookie",
    "connection",
    "content-length",
    "transfer-encoding",
    "te",
    "trailer",
    "upgrade",
    "keep-alive",
    "proxy-authorization",
    "proxy-connection",
    "forwarded",
    "x-forwarded-for",
];

/// The answer's headers passed back.
const PASSED_BACK: [HeaderName; 6] = [
    header::CONTENT_TYPE,
    header::CACHE_CONTROL,
    header::ETAG,
    header::LAST_MODIFIED,
    header::EXPIRES,
    header::CONTENT_ENCODING,
];

/// Whether `ip` is on the public internet: not this host's, not a private
/// or shared network's, not link-local (where the clouds keep their
/// metadata), not multicast, not reserved or for documentation; an IPv4
/// address inside an IPv6 one (mapped, NAT64, 6to4) is judged as itself.
pub fn public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => public_v4(v4),
        IpAddr::V6(v6) => public_v6(v6),
    }
}

fn public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(a == 0
        || a == 10
        || a == 127
        || (a == 100 && (64..=127).contains(&b))
        || (a == 169 && b == 254)
        || (a == 172 && (16..=31).contains(&b))
        || (a == 192 && b == 0 && c == 0)
        || (a == 192 && b == 0 && c == 2)
        || (a == 192 && b == 88 && c == 99)
        || (a == 192 && b == 168)
        || (a == 198 && (b == 18 || b == 19))
        || (a == 198 && b == 51 && c == 100)
        || (a == 203 && b == 0 && c == 113)
        || a >= 224)
}

fn public_v6(ip: Ipv6Addr) -> bool {
    let s = ip.segments();
    // IPv4 inside: mapped (::ffff:a.b.c.d), NAT64 (64:ff9b::/96), 6to4 (2002::/16).
    if s[..5] == [0, 0, 0, 0, 0] && s[5] == 0xffff {
        return public_v4(Ipv4Addr::new(
            (s[6] >> 8) as u8,
            s[6] as u8,
            (s[7] >> 8) as u8,
            s[7] as u8,
        ));
    }
    if s[0] == 0x64 && s[1] == 0xff9b && s[2..6] == [0, 0, 0, 0] {
        return public_v4(Ipv4Addr::new(
            (s[6] >> 8) as u8,
            s[6] as u8,
            (s[7] >> 8) as u8,
            s[7] as u8,
        ));
    }
    if s[0] == 0x2002 {
        return public_v4(Ipv4Addr::new(
            (s[1] >> 8) as u8,
            s[1] as u8,
            (s[2] >> 8) as u8,
            s[2] as u8,
        ));
    }
    !(ip.is_unspecified()
        || ip.is_loopback()
        // IPv4-compatible (deprecated) and the rest of ::/96.
        || s[..6] == [0, 0, 0, 0, 0, 0]
        // Discard-only (100::/64).
        || (s[0] == 0x100 && s[1..4] == [0, 0, 0])
        // Teredo (2001::/32) and documentation (2001:db8::/32).
        || (s[0] == 0x2001 && (s[1] == 0 || s[1] == 0xdb8))
        // Unique local (fc00::/7), link-local (fe80::/10), multicast (ff00::/8).
        || (s[0] & 0xfe00) == 0xfc00
        || (s[0] & 0xffc0) == 0xfe80
        || (s[0] & 0xff00) == 0xff00)
}

/// A network `KENTOS_PROXY_ALLOW` opens: an address and its prefix length.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Net {
    pub addr: IpAddr,
    pub bits: u8,
}

impl Net {
    fn contains(&self, ip: IpAddr) -> bool {
        match (self.addr, ip) {
            (IpAddr::V4(n), IpAddr::V4(a)) => {
                let mask = u32::MAX.checked_shl(32 - u32::from(self.bits)).unwrap_or(0);
                u32::from(n) & mask == u32::from(a) & mask
            }
            (IpAddr::V6(n), IpAddr::V6(a)) => {
                let mask = u128::MAX
                    .checked_shl(128 - u32::from(self.bits))
                    .unwrap_or(0);
                u128::from(n) & mask == u128::from(a) & mask
            }
            _ => false,
        }
    }
}

/// What the proxy may reach: the public internet and the networks opened.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reach {
    pub open: Vec<Net>,
}

impl Reach {
    /// `KENTOS_PROXY_ALLOW`: networks with their prefix (`10.20.0.0/16`, `fd00::/8`) or single
    /// addresses, separated by commas or spaces; absent, none.
    pub fn parse(text: Option<&str>) -> Result<Reach, String> {
        let mut open = Vec::new();
        for item in text
            .unwrap_or("")
            .split([',', ' ', '\t', '\n'])
            .filter(|t| !t.is_empty())
        {
            let (addr, bits) = match item.split_once('/') {
                Some((a, b)) => (a, Some(b)),
                None => (item, None),
            };
            let addr: IpAddr = addr.parse().map_err(|_| {
                format!(
                    "KENTOS_PROXY_ALLOW: “{item}” bir adres ya da ağ değil (örnek: 10.20.0.0/16)."
                )
            })?;
            let most = if addr.is_ipv4() { 32 } else { 128 };
            let bits = match bits {
                None => most,
                Some(b) => b
                    .parse::<u8>()
                    .ok()
                    .filter(|b| *b <= most)
                    .ok_or_else(|| format!("KENTOS_PROXY_ALLOW: “{item}” ağın önek uzunluğu 0 ile {most} arasında olmalı."))?,
            };
            open.push(Net { addr, bits });
        }
        Ok(Reach { open })
    }

    pub fn allows(&self, ip: IpAddr) -> bool {
        public(ip) || self.open.iter().any(|n| n.contains(ip))
    }
}

/// The resolver the proxy's client connects through: a name's addresses the
/// proxy may not reach are dropped; none left, the name is refused.
struct Guard {
    reach: Arc<Reach>,
}

#[derive(Debug)]
struct Refused(String);

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} yalnız iç ağ adreslerine çözülüyor", self.0)
    }
}

impl std::error::Error for Refused {}

impl Resolve for Guard {
    fn resolve(&self, name: Name) -> Resolving {
        let reach = self.reach.clone();
        let host = name.as_str().to_owned();
        Box::pin(async move {
            let found: Vec<SocketAddr> =
                tokio::net::lookup_host((host.as_str(), 0)).await?.collect();
            let kept: Vec<SocketAddr> =
                found.into_iter().filter(|a| reach.allows(a.ip())).collect();
            if kept.is_empty() {
                return Err(Box::new(Refused(host)) as Box<dyn std::error::Error + Send + Sync>);
            }
            Ok(Box::new(kept.into_iter()) as Addrs)
        })
    }
}

/// The proxy: its client, what it may reach and each user's requests on their way.
pub struct Proxy {
    client: reqwest::Client,
    reach: Arc<Reach>,
    busy: Mutex<HashMap<Uuid, usize>>,
}

/// Why a request was not passed on, in the words the web's windows show.
#[derive(Debug, PartialEq, Eq)]
pub enum Stop {
    /// The address itself is refused (422).
    Address(String),
    /// The target could not be reached or answered too much (502).
    Upstream(String),
}

/// An answer to pass back.
#[derive(Debug)]
pub struct Upstream {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Bytes,
    /// The host that answered (for the log).
    pub host: String,
}

impl Proxy {
    pub fn new(reach: Reach) -> Result<Proxy, String> {
        let reach = Arc::new(reach);
        let client = reqwest::Client::builder()
            .redirect(redirect::Policy::none())
            .no_proxy()
            .timeout(TIMEOUT)
            .connect_timeout(Duration::from_secs(10))
            .user_agent(concat!("KentOS/", env!("CARGO_PKG_VERSION")))
            .dns_resolver(Arc::new(Guard {
                reach: reach.clone(),
            }))
            .build()
            .map_err(|e| format!("Vekilin istemcisi kurulamadı: {e}"))?;
        Ok(Proxy {
            client,
            reach,
            busy: Mutex::new(HashMap::new()),
        })
    }

    /// One of `user`'s slots, or none when 16 are taken.
    pub(super) fn slot(&self, user: Uuid) -> Option<Slot<'_>> {
        let mut busy = self.busy.lock().unwrap_or_else(|e| e.into_inner());
        let n = busy.entry(user).or_insert(0);
        if *n >= PER_USER {
            return None;
        }
        *n += 1;
        Some(Slot { proxy: self, user })
    }

    /// Whether `url` may be asked for: `http` or `https`, no user in it, and
    /// a literal address only where the proxy may reach (a name is checked
    /// when it is resolved).
    pub fn check(&self, url: &Url) -> Result<(), Stop> {
        if !matches!(url.scheme(), "http" | "https") {
            return Err(Stop::Address(
                "Vekil yalnız http ve https adreslerine gider.".into(),
            ));
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(Stop::Address(
                "Adreste kullanıcı bilgisi olmamalı: kullanıcı adı ve parola bağlantının değerleriyle gider.".into(),
            ));
        }
        let Some(host) = url.host_str().filter(|h| !h.is_empty()) else {
            return Err(Stop::Address("Adreste makine adı yok.".into()));
        };
        // The address parser writes every form of an IP address (`0x7f.1`, `2130706433`) as itself.
        if let Ok(ip) = host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<IpAddr>()
            && !self.reach.allows(ip)
        {
            return Err(Stop::Address(format!(
                "Vekil iç ağ adreslerine gitmez ({ip}); kurumun iç ağı KENTOS_PROXY_ALLOW ile açılabilir."
            )));
        }
        Ok(())
    }

    /// `method` on `url` with `headers` and `body`, redirects followed (each
    /// checked again; a 303, or a 301 or 302 after a POST, goes on as a GET
    /// without the body, as a browser does), the answer read up to 32 MB.
    pub async fn fetch(
        &self,
        method: Method,
        url: Url,
        headers: HeaderMap,
        body: Bytes,
    ) -> Result<Upstream, Stop> {
        let mut method = method;
        let mut url = url;
        let mut body = (!body.is_empty()).then_some(body);
        for hop in 0..=MOST_REDIRECTS {
            self.check(&url)?;
            let mut req = self
                .client
                .request(method.clone(), url.clone())
                .headers(headers.clone());
            if let Some(b) = &body {
                req = req.body(b.clone());
            }
            let res = req.send().await.map_err(|e| Stop::Upstream(reached(&e)))?;
            let status = res.status();
            if status.is_redirection()
                && let Some(to) = res
                    .headers()
                    .get(header::LOCATION)
                    .and_then(|v| v.to_str().ok())
            {
                if hop == MOST_REDIRECTS {
                    return Err(Stop::Upstream(format!(
                        "Servis {MOST_REDIRECTS}'ten çok yönlendirdi."
                    )));
                }
                url = url.join(to).map_err(|_| {
                    Stop::Upstream("Servisin yönlendirdiği adres okunamadı.".into())
                })?;
                if status == StatusCode::SEE_OTHER
                    || (method == Method::POST && matches!(status.as_u16(), 301 | 302))
                {
                    method = Method::GET;
                    body = None;
                }
                continue;
            }
            let host = url.host_str().unwrap_or("").to_owned();
            let mut back = HeaderMap::new();
            for name in &PASSED_BACK {
                if let Some(v) = res.headers().get(name) {
                    back.insert(name.clone(), v.clone());
                }
            }
            let mut res = res;
            let mut bytes: Vec<u8> = Vec::new();
            while let Some(chunk) = res.chunk().await.map_err(|e| Stop::Upstream(reached(&e)))? {
                if bytes.len() + chunk.len() > MOST_BYTES {
                    return Err(Stop::Upstream("Servisin yanıtı 32 MB'tan büyük.".into()));
                }
                bytes.extend_from_slice(&chunk);
            }
            return Ok(Upstream {
                status,
                headers: back,
                body: Bytes::from(bytes),
                host,
            });
        }
        Err(Stop::Upstream(format!(
            "Servis {MOST_REDIRECTS}'ten çok yönlendirdi."
        )))
    }
}

/// Why a target was not reached, without its address (a key may be in it).
fn reached(e: &reqwest::Error) -> String {
    let mut source = std::error::Error::source(e);
    let mut refused = false;
    while let Some(s) = source {
        if s.downcast_ref::<Refused>().is_some() {
            refused = true;
            break;
        }
        source = s.source();
    }
    if refused {
        "Servisin adresi yalnız iç ağa çözülüyor; vekil oraya gitmez (kurumun iç ağı KENTOS_PROXY_ALLOW ile açılabilir).".into()
    } else if e.is_timeout() {
        "Servis 30 saniyede yanıt vermedi.".into()
    } else {
        "Servise ulaşılamadı: adresi ve servisin çalıştığını denetleyin.".into()
    }
}

/// A user's request on its way; gone when dropped.
pub(super) struct Slot<'a> {
    proxy: &'a Proxy,
    user: Uuid,
}

impl Drop for Slot<'_> {
    fn drop(&mut self) {
        let mut busy = self.proxy.busy.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(n) = busy.get_mut(&self.user) {
            *n -= 1;
            if *n == 0 {
                busy.remove(&self.user);
            }
        }
    }
}

/// The headers for the target: the `x-kentos-header-` ones without their
/// prefix (never a connection's or a cookie), the body's type, and plain
/// bytes asked for (the answer passes back as it comes).
pub fn outgoing(headers: &HeaderMap) -> HeaderMap {
    let mut out = HeaderMap::new();
    for (name, value) in headers {
        let Some(rest) = name.as_str().strip_prefix(HEADER_PREFIX) else {
            continue;
        };
        if rest.is_empty()
            || NEVER_SENT.contains(&rest)
            || rest.starts_with("proxy-")
            || rest.starts_with("sec-")
        {
            continue;
        }
        if let Ok(n) = HeaderName::from_bytes(rest.as_bytes()) {
            out.append(n, value.clone());
        }
    }
    if let Some(t) = headers.get(header::CONTENT_TYPE) {
        out.insert(header::CONTENT_TYPE, t.clone());
    }
    out.insert(
        header::ACCEPT_ENCODING,
        HeaderValue::from_static("identity"),
    );
    out
}

#[derive(Deserialize)]
pub struct Target {
    pub url: String,
}

/// A refusal of the proxy's own: an `ApiError` with its status.
fn refusal(status: StatusCode, code: &str, message: String, headers: &HeaderMap) -> Response {
    let body = ApiError {
        error: code.into(),
        message,
        request_id: request_id(headers),
        conflicts: None,
        path: Some("url".into()),
        revision: None,
        retryable: status == StatusCode::BAD_GATEWAY,
        retry_after: None,
    };
    (status, Json(body)).into_response()
}

/// `GET`, `POST /v1/proxy?url=…`.
pub async fn forward(
    State(state): State<AppState>,
    Caller(actor): Caller,
    Query(target): Query<Target>,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, Failure> {
    let Some(proxy) = &state.proxy else {
        return Err(Failure::with(
            AppError::forbidden("Bu sunucuda servis vekili kapalı."),
            &headers,
        ));
    };
    let Ok(url) = Url::parse(target.url.trim()) else {
        return Ok(refusal(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid",
            "Vekile verilen adres okunamadı.".into(),
            &headers,
        ));
    };
    let Some(_slot) = proxy.slot(actor.user_id) else {
        return Err(Failure::with(
            AppError::Limited {
                message: format!(
                    "Aynı anda en çok {PER_USER} servis isteği sunucu üzerinden gider; biraz bekleyin."
                ),
                retry_after: 1,
            },
            &headers,
        ));
    };
    match proxy.fetch(method, url, outgoing(&headers), body).await {
        Ok(up) => {
            tracing::debug!(host = %up.host, status = up.status.as_u16(), bytes = up.body.len(), "servis vekili");
            let mut res = (up.status, up.body).into_response();
            *res.headers_mut() = up.headers;
            Ok(res)
        }
        Err(Stop::Address(m)) => Ok(refusal(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid",
            m,
            &headers,
        )),
        Err(Stop::Upstream(m)) => Ok(refusal(StatusCode::BAD_GATEWAY, "upstream", m, &headers)),
    }
}
