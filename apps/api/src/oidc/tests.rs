//! OpenID against a fake provider in this process (discovery, keys, token
//! endpoint), signing with a test-only RSA key (tests/fixtures).

use std::sync::{Arc, Mutex as StdMutex};

use axum::extract::{Form, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use jsonwebtoken::jwk::{Jwk, JwkSet};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use kentos_application::identity;
use kentos_postgres::testing::TestDb;
use serde_json::{Value, json};

use super::*;

const KEY: &str = include_str!("../../tests/fixtures/oidc-test-rsa.pem");
const CLIENT: &str = "kentos-cad";

#[derive(Default)]
struct Fake {
    issuer: String,
    kid: String,
    /// What the token endpoint returns next, and the form it last received.
    next_id_token: Option<String>,
    last_form: Option<Vec<(String, String)>>,
    jwks_calls: usize,
}

type Shared = Arc<StdMutex<Fake>>;

fn key() -> EncodingKey {
    EncodingKey::from_rsa_pem(KEY.as_bytes()).unwrap()
}

fn sign(claims: Value, alg: Algorithm, kid: &str) -> String {
    let mut header = Header::new(alg);
    header.kid = Some(kid.into());
    encode(&header, &claims, &key()).unwrap()
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

async fn start_fake() -> (Shared, String) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let shared: Shared = Arc::new(StdMutex::new(Fake {
        issuer: base.clone(),
        kid: "test-1".into(),
        ..Default::default()
    }));
    let app = Router::new()
        .route(
            "/.well-known/openid-configuration",
            get(|State(s): State<Shared>| async move {
                let iss = s.lock().unwrap().issuer.clone();
                Json(json!({
                    "issuer": iss,
                    "authorization_endpoint": format!("{iss}/auth"),
                    "token_endpoint": format!("{iss}/token"),
                    "jwks_uri": format!("{iss}/jwks"),
                }))
            }),
        )
        .route(
            "/jwks",
            get(|State(s): State<Shared>| async move {
                let mut f = s.lock().unwrap();
                f.jwks_calls += 1;
                let mut jwk = Jwk::from_encoding_key(&key(), Algorithm::RS256).unwrap();
                jwk.common.key_id = Some(f.kid.clone());
                Json(JwkSet { keys: vec![jwk] })
            }),
        )
        .route(
            "/token",
            post(|State(s): State<Shared>, Form(form): Form<Vec<(String, String)>>| async move {
                let mut f = s.lock().unwrap();
                f.last_form = Some(form);
                Json(json!({ "id_token": f.next_id_token.take().unwrap_or_default(), "token_type": "Bearer" }))
            }),
        )
        .with_state(shared.clone());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (shared, base)
}

fn settings(issuer: &str) -> OidcSettings {
    OidcSettings {
        issuer: issuer.into(),
        client_id: CLIENT.into(),
        client_secret: Some("gizli".into()),
        audience: "kentos-api".into(),
        label: "Kurum".into(),
    }
}

fn query(url: &str) -> HashMap<String, String> {
    reqwest::Url::parse(url)
        .unwrap()
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect()
}

fn id_claims(iss: &str, nonce: &str) -> Value {
    json!({ "iss": iss, "aud": CLIENT, "sub": "kisi-42", "exp": now() + 300, "iat": now(), "nonce": nonce,
            "name": "Ayla Kaya", "email": "ayla@example.org" })
}

#[tokio::test]
async fn code_and_pkce_sign_in_opens_a_session() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (fake, iss) = start_fake().await;
    let oidc = Oidc::new(settings(&iss), "http://app.test").unwrap();

    let url = oidc.start(&db.app, "/proje").await.unwrap();
    assert!(url.starts_with(&format!("{iss}/auth?")));
    let q = query(&url);
    assert_eq!(q["client_id"], CLIENT);
    assert_eq!(q["redirect_uri"], "http://app.test/v1/auth/oidc/callback");
    assert_eq!(q["code_challenge_method"], "S256");
    assert_eq!(q["scope"], "openid profile email");

    let token = { sign(id_claims(&iss, &q["nonce"]), Algorithm::RS256, "test-1") };
    fake.lock().unwrap().next_id_token = Some(token);
    let (session, return_to) = oidc.finish(&db.app, "kod-1", &q["state"]).await.unwrap();
    assert_eq!(return_to, "/proje");

    // The code was redeemed with the verifier whose S256 hash is the challenge sent earlier.
    let form: HashMap<String, String> = fake
        .lock()
        .unwrap()
        .last_form
        .clone()
        .unwrap()
        .into_iter()
        .collect();
    assert_eq!(
        (
            form["grant_type"].as_str(),
            form["code"].as_str(),
            form["client_secret"].as_str()
        ),
        ("authorization_code", "kod-1", "gizli")
    );
    let challenge: String = sqlx::query_scalar(
        "select translate(rtrim(encode(public.digest($1, 'sha256'), 'base64'), '='), '+/', '-_')",
    )
    .bind(&form["code_verifier"])
    .fetch_one(&db.owner)
    .await
    .unwrap();
    assert_eq!(challenge, q["code_challenge"]);

    let actor = identity::session_actor(&db.app, &session)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (actor.display_name.as_str(), actor.method),
        ("Ayla Kaya", SignInMethod::Oidc)
    );
    let (issuer, subject): (String, String) =
        sqlx::query_as("select issuer, subject from kentos.app_user where id = $1")
            .bind(actor.user_id)
            .fetch_one(&db.owner)
            .await
            .unwrap();
    assert_eq!(
        (issuer.as_str(), subject.as_str()),
        (iss.as_str(), "kisi-42")
    );

    // A state is good once.
    assert!(matches!(
        oidc.finish(&db.app, "kod-1", &q["state"]).await,
        Err(AppError::Invalid { .. })
    ));
    db.close().await;
}

#[tokio::test]
async fn tokens_that_do_not_belong_are_refused() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (fake, iss) = start_fake().await;
    let oidc = Oidc::new(settings(&iss), "http://app.test").unwrap();
    let attempt = |claims: Value, alg: Algorithm, kid: &'static str| {
        let fake = fake.clone();
        let oidc = &oidc;
        let db = &db;
        async move {
            let q = query(&oidc.start(&db.app, "/").await.unwrap());
            let mut claims = claims;
            if claims["nonce"] == "@" {
                claims["nonce"] = json!(q["nonce"]);
            }
            let token = if alg == Algorithm::HS256 {
                let mut h = Header::new(Algorithm::HS256);
                h.kid = Some(kid.into());
                encode(
                    &h,
                    &claims,
                    &EncodingKey::from_secret(b"public-key-as-secret"),
                )
                .unwrap()
            } else {
                sign(claims, alg, kid)
            };
            fake.lock().unwrap().next_id_token = Some(token);
            oidc.finish(&db.app, "kod", &q["state"]).await
        }
    };
    let ok = id_claims(&iss, "@");
    let mut wrong_nonce = ok.clone();
    wrong_nonce["nonce"] = json!("baska");
    let mut wrong_aud = ok.clone();
    wrong_aud["aud"] = json!("baska-uygulama");
    let mut wrong_iss = ok.clone();
    wrong_iss["iss"] = json!("https://sahte.example.org");
    let mut expired = ok.clone();
    expired["exp"] = json!(now() - 3600);
    for (claims, alg, kid) in [
        (wrong_nonce, Algorithm::RS256, "test-1"),
        (wrong_aud, Algorithm::RS256, "test-1"),
        (wrong_iss, Algorithm::RS256, "test-1"),
        (expired, Algorithm::RS256, "test-1"),
        (ok.clone(), Algorithm::HS256, "test-1"),
        (ok.clone(), Algorithm::RS256, "bilinmeyen"),
    ] {
        assert!(matches!(
            attempt(claims, alg, kid).await,
            Err(AppError::Unauthenticated(_))
        ));
    }
    // An unknown key id refreshed the keys once; more unknown ids within a minute do not hammer the provider.
    let calls = fake.lock().unwrap().jwks_calls;
    assert!(
        attempt(ok.clone(), Algorithm::RS256, "baska-bilinmeyen")
            .await
            .is_err()
    );
    assert_eq!(fake.lock().unwrap().jwks_calls, calls);
    // The good token still works.
    assert!(attempt(ok, Algorithm::RS256, "test-1").await.is_ok());
    db.close().await;
}

#[tokio::test]
async fn bearer_tokens_identify_api_clients() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (_fake, iss) = start_fake().await;
    let oidc = Oidc::new(settings(&iss), "http://app.test").unwrap();
    let access = |aud: &str| {
        sign(
            json!({ "iss": iss, "aud": aud, "sub": "servis-7", "exp": now() + 300, "preferred_username": "rapor" }),
            Algorithm::RS256,
            "test-1",
        )
    };
    let actor = oidc
        .bearer_actor(&db.app, &access("kentos-api"))
        .await
        .unwrap();
    assert_eq!(
        (actor.display_name.as_str(), actor.method),
        ("rapor", SignInMethod::Bearer)
    );
    let again = oidc
        .bearer_actor(&db.app, &access("kentos-api"))
        .await
        .unwrap();
    assert_eq!(again.user_id, actor.user_id);
    // An ID token meant for the browser client is not an API access token.
    assert!(oidc.bearer_actor(&db.app, &access(CLIENT)).await.is_err());
    // A disabled account is refused even with a valid token.
    sqlx::query("update kentos.app_user set status = 'disabled' where id = $1")
        .bind(actor.user_id)
        .execute(&db.owner)
        .await
        .unwrap();
    assert!(matches!(
        oidc.bearer_actor(&db.app, &access("kentos-api")).await,
        Err(AppError::Forbidden(_))
    ));
    db.close().await;
}

#[test]
fn return_paths_stay_inside_the_app() {
    assert_eq!(safe_return_to(Some("/proje/1?x=2")), "/proje/1?x=2");
    for evil in [
        Some("//evil.example"),
        Some("https://evil.example"),
        Some("/\\evil"),
        Some("proje"),
        None,
    ] {
        assert_eq!(safe_return_to(evil), "/");
    }
}
