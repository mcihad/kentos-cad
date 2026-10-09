//! The map services' proxy (docs/adr/0208 §13): which addresses it reaches,
//! the headers it sends and passes back, and a real round trip against a
//! server of the test's own on the loopback (opened for it).

use std::net::IpAddr;

use axum::Router;
use axum::body::Bytes;
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Redirect};
use axum::routing::{any, get};
use reqwest::Url;

use super::proxy::{HEADER_PREFIX, MOST_BYTES, Proxy, Reach, Stop, outgoing, public};

fn ip(text: &str) -> IpAddr {
    text.parse().expect("an address")
}

#[test]
fn only_the_public_internet_is_public() {
    for (text, open) in [
        ("8.8.8.8", true),
        ("172.32.0.1", true),
        ("100.128.0.1", true),
        ("2606:4700::1111", true),
        ("::ffff:8.8.8.8", true),
        ("127.0.0.1", false),
        ("10.1.2.3", false),
        ("169.254.169.254", false),
        ("100.64.0.1", false),
        ("172.16.0.1", false),
        ("192.168.1.1", false),
        ("192.0.2.7", false),
        ("198.18.0.1", false),
        ("224.0.0.1", false),
        ("240.0.0.1", false),
        ("255.255.255.255", false),
        ("0.0.0.0", false),
        ("::", false),
        ("::1", false),
        ("::ffff:127.0.0.1", false),
        ("::ffff:169.254.169.254", false),
        ("64:ff9b::7f00:1", false),
        ("2002:7f00:1::", false),
        ("2001:db8::1", false),
        ("2001::1", false),
        ("fe80::1", false),
        ("fd00::1", false),
        ("ff02::1", false),
    ] {
        assert_eq!(public(ip(text)), open, "{text}");
    }
}

#[test]
fn an_organisation_opens_its_own_networks() {
    let r = Reach::parse(Some("10.20.0.0/16, fd00::/8 192.168.1.5")).expect("reads");
    assert_eq!(r.open.len(), 3);
    assert!(r.allows(ip("10.20.3.4")));
    assert!(!r.allows(ip("10.21.0.1")));
    assert!(r.allows(ip("fd12::5")));
    assert!(r.allows(ip("192.168.1.5")));
    assert!(!r.allows(ip("192.168.1.6")));
    assert!(r.allows(ip("8.8.8.8")));
    assert!(Reach::parse(None).expect("none").open.is_empty());
    for bad in ["10.0.0.0/33", "fd00::/129", "ağ", "10.0.0/8"] {
        assert!(Reach::parse(Some(bad)).is_err(), "{bad}");
    }
}

#[test]
fn only_the_prefixed_headers_go_and_never_a_cookie_or_the_host() {
    let mut h = HeaderMap::new();
    for (k, v) in [
        ("x-kentos-header-x-api-key", "anahtar"),
        ("x-kentos-header-authorization", "Bearer b"),
        ("x-kentos-header-cookie", "a=b"),
        ("x-kentos-header-host", "ic.ornek"),
        ("x-kentos-header-proxy-authorization", "x"),
        ("x-kentos-header-sec-fetch-mode", "cors"),
        ("authorization", "Bearer oturum"),
        ("cookie", "kentos_session=s"),
        ("content-type", "application/x-www-form-urlencoded"),
    ] {
        h.insert(
            header::HeaderName::from_static(k),
            HeaderValue::from_static(v),
        );
    }
    let out = outgoing(&h);
    assert_eq!(out.get("x-api-key").expect("the key"), "anahtar");
    assert_eq!(
        out.get(header::AUTHORIZATION).expect("the token"),
        "Bearer b"
    );
    assert!(out.get(header::COOKIE).is_none());
    assert!(out.get(header::HOST).is_none());
    assert!(out.get("proxy-authorization").is_none());
    assert!(out.get("sec-fetch-mode").is_none());
    assert_eq!(
        out.get(header::CONTENT_TYPE).expect("the body's type"),
        "application/x-www-form-urlencoded"
    );
    assert_eq!(
        out.get(header::ACCEPT_ENCODING).expect("plain bytes"),
        "identity"
    );
    assert!(out.keys().all(|k| !k.as_str().starts_with(HEADER_PREFIX)));
}

#[test]
fn an_address_is_refused_before_anything_is_sent() {
    let p = Proxy::new(Reach::default()).expect("a proxy");
    let refused = |text: &str| {
        matches!(
            p.check(&Url::parse(text).expect("an address")),
            Err(Stop::Address(_))
        )
    };
    assert!(refused("ftp://ornek.org/veri"));
    assert!(refused("https://ad:parola@ornek.org/wms"));
    assert!(refused("http://127.0.0.1:8787/v1/me"));
    assert!(refused("http://[::1]/"));
    // Every spelling of an address is read as itself.
    assert!(refused("http://2130706433/"));
    assert!(refused("http://0x7f.1/"));
    assert!(refused("http://169.254.169.254/latest/meta-data/"));
    // A name is checked when it is resolved.
    assert!(
        p.check(&Url::parse("https://tile.openstreetmap.org/0/0/0.png").expect("an address"))
            .is_ok()
    );
}

/// A server on the loopback: a tile, redirects, a loop, an answer too large and an echo.
async fn serve() -> String {
    async fn tile() -> impl IntoResponse {
        let mut h = HeaderMap::new();
        h.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/png"));
        h.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("max-age=86400"),
        );
        h.insert(header::SET_COOKIE, HeaderValue::from_static("izleme=1"));
        h.insert("x-powered-by", HeaderValue::from_static("deneme"));
        (h, Bytes::from_static(b"\x89PNG karo"))
    }
    async fn echo(method: Method, headers: HeaderMap, body: Bytes) -> String {
        let key = headers
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("-")
            .to_owned();
        let cookie = headers.contains_key(header::COOKIE);
        format!("{method} {key} {cookie} {}", String::from_utf8_lossy(&body))
    }
    let app = Router::new()
        .route("/karo", get(tile))
        .route("/bir", get(|| async { Redirect::to("/iki") }))
        .route("/iki", get(|| async { Redirect::temporary("/karo") }))
        .route("/dongu", get(|| async { Redirect::to("/dongu") }))
        .route(
            "/ic",
            get(|| async { Redirect::to("http://127.0.0.2:9/gizli") }),
        )
        .route(
            "/gor",
            any(|| async { (StatusCode::SEE_OTHER, [(header::LOCATION, "/yanki")]) }),
        )
        .route("/buyuk", get(|| async { vec![0u8; MOST_BYTES + 1] }))
        .route("/yok", get(|| async { (StatusCode::NOT_FOUND, "yok") }))
        .route("/yanki", any(echo));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("binds");
    let at = listener.local_addr().expect("an address");
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    format!("http://{at}")
}

fn url(base: &str, path: &str) -> Url {
    Url::parse(&format!("{base}{path}")).expect("an address")
}

#[tokio::test]
async fn a_round_trip_follows_redirects_and_passes_back_only_safe_headers() {
    let base = serve().await;
    let p = Proxy::new(Reach::parse(Some("127.0.0.1/32")).expect("reads")).expect("a proxy");
    let up = p
        .fetch(
            Method::GET,
            url(&base, "/bir"),
            HeaderMap::new(),
            Bytes::new(),
        )
        .await
        .expect("answers");
    assert_eq!(up.status, StatusCode::OK);
    assert_eq!(&up.body[..], b"\x89PNG karo");
    assert_eq!(
        up.headers.get(header::CONTENT_TYPE).expect("its type"),
        "image/png"
    );
    assert_eq!(
        up.headers.get(header::CACHE_CONTROL).expect("its caching"),
        "max-age=86400"
    );
    assert!(up.headers.get(header::SET_COOKIE).is_none());
    assert!(up.headers.get("x-powered-by").is_none());
    assert_eq!(up.host, "127.0.0.1");
    // A target's own refusal passes back as it is.
    let up = p
        .fetch(
            Method::GET,
            url(&base, "/yok"),
            HeaderMap::new(),
            Bytes::new(),
        )
        .await
        .expect("answers");
    assert_eq!(up.status, StatusCode::NOT_FOUND);
    // A POST goes with its body and the target's headers; a 303 goes on as a GET without the body.
    let mut h = HeaderMap::new();
    h.insert(
        "x-kentos-header-x-api-key",
        HeaderValue::from_static("anahtar"),
    );
    h.insert(header::COOKIE, HeaderValue::from_static("kentos_session=s"));
    let up = p
        .fetch(
            Method::POST,
            url(&base, "/yanki"),
            outgoing(&h),
            Bytes::from_static(b"f=json"),
        )
        .await
        .expect("answers");
    assert_eq!(&up.body[..], b"POST anahtar false f=json");
    let up = p
        .fetch(
            Method::POST,
            url(&base, "/gor"),
            outgoing(&h),
            Bytes::from_static(b"f=json"),
        )
        .await
        .expect("answers");
    assert_eq!(&up.body[..], b"GET anahtar false ");
}

#[tokio::test]
async fn too_many_redirects_too_large_an_answer_and_the_inside_are_refused() {
    let base = serve().await;
    let p = Proxy::new(Reach::parse(Some("127.0.0.1/32")).expect("reads")).expect("a proxy");
    let stop = |r: Result<super::proxy::Upstream, Stop>| match r {
        Err(Stop::Upstream(m)) => m,
        other => panic!("refused upstream: {other:?}"),
    };
    assert!(
        stop(
            p.fetch(
                Method::GET,
                url(&base, "/dongu"),
                HeaderMap::new(),
                Bytes::new()
            )
            .await
        )
        .contains("yönlendirdi")
    );
    assert!(
        stop(
            p.fetch(
                Method::GET,
                url(&base, "/buyuk"),
                HeaderMap::new(),
                Bytes::new()
            )
            .await
        )
        .contains("32 MB")
    );
    // A redirect is checked again: 127.0.0.2 is not opened.
    assert!(matches!(
        p.fetch(
            Method::GET,
            url(&base, "/ic"),
            HeaderMap::new(),
            Bytes::new()
        )
        .await,
        Err(Stop::Address(_))
    ));
    // Without the loopback opened, a name that resolves to it is refused when resolved.
    let closed = Proxy::new(Reach::default()).expect("a proxy");
    let port = base.rsplit(':').next().expect("a port");
    let m = stop(
        closed
            .fetch(
                Method::GET,
                Url::parse(&format!("http://localhost:{port}/karo")).expect("an address"),
                HeaderMap::new(),
                Bytes::new(),
            )
            .await,
    );
    assert!(m.contains("iç ağ"), "{m}");
}

#[test]
fn a_user_has_sixteen_requests_on_their_way_at_most() {
    let p = Proxy::new(Reach::default()).expect("a proxy");
    let (a, b) = (uuid::Uuid::now_v7(), uuid::Uuid::now_v7());
    let mut held: Vec<_> = (0..super::proxy::PER_USER)
        .map(|_| p.slot(a).expect("a slot"))
        .collect();
    assert!(p.slot(a).is_none());
    // Another user is not held up.
    assert!(p.slot(b).is_some());
    held.pop();
    assert!(p.slot(a).is_some());
}
