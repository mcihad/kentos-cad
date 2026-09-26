//! OpenID Connect (docs/adr/0007-authentication.md).
//!
//! - Browser sign-in: authorization code + PKCE (S256), run by this server.
//!   State, nonce and the code verifier come from `gen_random_bytes` and sit
//!   in `kentos.oidc_login` for ten minutes; the callback consumes them once.
//!   A verified ID token opens the same kind of session as a local login.
//! - API clients: `Authorization: Bearer <access token>`, verified against
//!   the provider's keys (issuer, audience, expiry; asymmetric algorithms only).
//!
//! The provider's discovery document and keys are cached for an hour; an
//! unknown key id triggers at most one refresh a minute (key rotation).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use kentos_application::AppError;
use kentos_application::identity::{self, Actor};
use kentos_contracts::SignInMethod;
use kentos_postgres::Db;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::config::OidcSettings;

const CACHE_FOR: Duration = Duration::from_secs(3600);
const REFRESH_AT_MOST: Duration = Duration::from_secs(60);
const IDENTITY_CACHE: Duration = Duration::from_secs(300);
/// Only asymmetric signatures: a shared-secret (HS*) token could be forged with a public key.
const ALLOWED: [Algorithm; 8] = [
    Algorithm::RS256,
    Algorithm::RS384,
    Algorithm::RS512,
    Algorithm::PS256,
    Algorithm::PS384,
    Algorithm::PS512,
    Algorithm::ES256,
    Algorithm::ES384,
];

#[derive(Deserialize)]
struct Discovery {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    jwks_uri: String,
}

struct Provider {
    authorization_endpoint: String,
    token_endpoint: String,
    keys: JwkSet,
    fetched: Instant,
}

#[derive(Deserialize)]
struct TokenResponse {
    id_token: String,
}

#[derive(Deserialize)]
struct Claims {
    sub: String,
    #[serde(default)]
    nonce: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    preferred_username: Option<String>,
    #[serde(default)]
    email: Option<String>,
    /// A boolean in the standard; some providers send the text "true".
    #[serde(default)]
    email_verified: Option<serde_json::Value>,
}

impl Claims {
    /// The provider says the e-mail is the person's (docs/adr/0035).
    fn email_verified(&self) -> bool {
        match &self.email_verified {
            Some(serde_json::Value::Bool(b)) => *b,
            Some(serde_json::Value::String(s)) => s.eq_ignore_ascii_case("true"),
            _ => false,
        }
    }

    fn display_name(&self) -> &str {
        self.name
            .as_deref()
            .or(self.preferred_username.as_deref())
            .unwrap_or(&self.sub)
    }
}

pub struct Oidc {
    settings: OidcSettings,
    redirect_uri: String,
    http: reqwest::Client,
    provider: RwLock<Option<Arc<Provider>>>,
    refreshed: Mutex<Option<Instant>>,
    /// (subject → account) for bearer tokens, so each API call does not write the account row.
    identities: Mutex<HashMap<String, (Uuid, Instant)>>,
}

fn unavailable(what: &str, e: impl std::fmt::Display) -> AppError {
    tracing::warn!(error = %e, "OpenID: {what}");
    AppError::Unauthenticated(format!(
        "Kimlik sağlayıcısına ulaşılamadı ({what}); birazdan yeniden deneyin."
    ))
}

/// A path inside this app only, so the callback cannot be turned into an open redirect.
pub fn safe_return_to(path: Option<&str>) -> String {
    match path {
        Some(p)
            if p.starts_with('/')
                && !p.starts_with("//")
                && !p.contains('\\')
                && p.len() <= 512 =>
        {
            p.to_string()
        }
        _ => "/".into(),
    }
}

impl Oidc {
    pub fn new(settings: OidcSettings, public_url: &str) -> Result<Self, String> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| format!("OpenID istemcisi kurulamadı: {e}"))?;
        Ok(Self {
            redirect_uri: format!("{public_url}/v1/auth/oidc/callback"),
            settings,
            http,
            provider: RwLock::new(None),
            refreshed: Mutex::new(None),
            identities: Mutex::new(HashMap::new()),
        })
    }

    async fn fetch_provider(&self) -> Result<Arc<Provider>, AppError> {
        let url = format!("{}/.well-known/openid-configuration", self.settings.issuer);
        let d: Discovery = self
            .http
            .get(&url)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| unavailable("yapılandırma", e))?
            .json()
            .await
            .map_err(|e| unavailable("yapılandırma", e))?;
        // The document must speak for the configured issuer, character for character.
        if d.issuer.trim_end_matches('/') != self.settings.issuer {
            return Err(unavailable(
                "yapılandırma",
                format!("issuer uyuşmuyor: {}", d.issuer),
            ));
        }
        let keys: JwkSet = self
            .http
            .get(&d.jwks_uri)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| unavailable("anahtarlar", e))?
            .json()
            .await
            .map_err(|e| unavailable("anahtarlar", e))?;
        let p = Arc::new(Provider {
            authorization_endpoint: d.authorization_endpoint,
            token_endpoint: d.token_endpoint,
            keys,
            fetched: Instant::now(),
        });
        *self.provider.write().await = Some(p.clone());
        *self.refreshed.lock().await = Some(Instant::now());
        Ok(p)
    }

    async fn provider(&self) -> Result<Arc<Provider>, AppError> {
        if let Some(p) = self
            .provider
            .read()
            .await
            .as_ref()
            .filter(|p| p.fetched.elapsed() < CACHE_FOR)
        {
            return Ok(p.clone());
        }
        self.fetch_provider().await
    }

    /// Verifies a signed token for `audience` and reads its claims; refreshes keys once for an unknown key id.
    async fn verify<C: DeserializeOwned>(
        &self,
        token: &str,
        audience: &str,
    ) -> Result<C, AppError> {
        let bad =
            |why: String| AppError::Unauthenticated(format!("Kimlik belirteci geçersiz: {why}."));
        let header = decode_header(token).map_err(|e| bad(e.to_string()))?;
        if !ALLOWED.contains(&header.alg) {
            return Err(bad(format!("{:?} imzası kabul edilmiyor", header.alg)));
        }
        let kid = header
            .kid
            .clone()
            .ok_or_else(|| bad("anahtar kimliği (kid) yok".into()))?;
        let mut provider = self.provider().await?;
        if provider.keys.find(&kid).is_none() {
            let recent = self
                .refreshed
                .lock()
                .await
                .is_some_and(|t| t.elapsed() < REFRESH_AT_MOST);
            if !recent {
                provider = self.fetch_provider().await?;
            }
        }
        let jwk = provider
            .keys
            .find(&kid)
            .ok_or_else(|| bad("imza anahtarı tanınmıyor".into()))?;
        let key = DecodingKey::from_jwk(jwk).map_err(|e| bad(e.to_string()))?;
        let mut validation = Validation::new(header.alg);
        validation.set_issuer(&[
            self.settings.issuer.as_str(),
            format!("{}/", self.settings.issuer).as_str(),
        ]);
        validation.set_audience(&[audience]);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        validation.validate_nbf = true;
        validation.leeway = 60;
        decode::<C>(token, &key, &validation)
            .map(|d| d.claims)
            .map_err(|e| bad(e.to_string()))
    }

    /// Starts a browser sign-in; returns the provider URL to redirect to.
    pub async fn start(&self, db: &Db, return_to: &str) -> Result<String, AppError> {
        let provider = self.provider().await?;
        sqlx::query("delete from kentos.oidc_login where expires_at < now()")
            .execute(&db.pool)
            .await?;
        let (state, nonce, challenge): (String, String, String) = sqlx::query_as(
            "insert into kentos.oidc_login (state, nonce, code_verifier, return_to, expires_at)
             values (encode(public.gen_random_bytes(24), 'hex'), encode(public.gen_random_bytes(24), 'hex'),
                     encode(public.gen_random_bytes(32), 'hex'), $1, now() + interval '10 minutes')
             returning state, nonce, translate(rtrim(encode(public.digest(code_verifier, 'sha256'), 'base64'), '='), '+/', '-_')",
        )
        .bind(return_to)
        .fetch_one(&db.pool)
        .await?;
        let mut url = reqwest::Url::parse(&provider.authorization_endpoint)
            .map_err(|e| unavailable("yetkilendirme adresi", e))?;
        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.settings.client_id)
            .append_pair("redirect_uri", &self.redirect_uri)
            .append_pair("scope", "openid profile email")
            .append_pair("state", &state)
            .append_pair("nonce", &nonce)
            .append_pair("code_challenge", &challenge)
            .append_pair("code_challenge_method", "S256");
        Ok(url.into())
    }

    /// Finishes a sign-in: consumes the state, redeems the code, verifies the ID token, opens a session.
    /// Returns the session token and where to send the browser.
    pub async fn finish(
        &self,
        db: &Db,
        code: &str,
        state: &str,
    ) -> Result<(String, String), AppError> {
        let row: Option<(String, String, String)> = sqlx::query_as(
            "delete from kentos.oidc_login where state = $1 and expires_at > now() returning nonce, code_verifier, return_to",
        )
        .bind(state)
        .fetch_optional(&db.pool)
        .await?;
        let Some((nonce, verifier, return_to)) = row else {
            return Err(AppError::invalid(
                "Giriş isteği bulunamadı ya da süresi doldu; girişi yeniden başlatın.",
            ));
        };
        let provider = self.provider().await?;
        let mut form = vec![
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", self.redirect_uri.as_str()),
            ("client_id", self.settings.client_id.as_str()),
            ("code_verifier", verifier.as_str()),
        ];
        if let Some(secret) = &self.settings.client_secret {
            form.push(("client_secret", secret.as_str()));
        }
        let tokens: TokenResponse = self
            .http
            .post(&provider.token_endpoint)
            .form(&form)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| unavailable("belirteç", e))?
            .json()
            .await
            .map_err(|e| unavailable("belirteç", e))?;
        let claims: Claims = self
            .verify(&tokens.id_token, &self.settings.client_id)
            .await?;
        if claims.nonce.as_deref() != Some(nonce.as_str()) {
            return Err(AppError::Unauthenticated(
                "Kimlik belirteci bu giriş isteğine ait değil (nonce).".into(),
            ));
        }
        let user = identity::resolve_identity(
            db,
            &self.settings.issuer,
            &claims.sub,
            claims.display_name(),
            claims.email.as_deref(),
            claims.email_verified(),
        )
        .await?
        .ok_or_else(|| AppError::forbidden("Hesabınız devre dışı; kurum yöneticinize başvurun."))?;
        let token = identity::open_session(db, user, SignInMethod::Oidc).await?;
        Ok((token, return_to))
    }

    /// The account behind a bearer access token.
    pub async fn bearer_actor(&self, db: &Db, token: &str) -> Result<Actor, AppError> {
        let claims: Claims = self.verify(token, &self.settings.audience).await?;
        let cached = self
            .identities
            .lock()
            .await
            .get(&claims.sub)
            .filter(|(_, at)| at.elapsed() < IDENTITY_CACHE)
            .map(|(u, _)| *u);
        let user = match cached {
            Some(u) => u,
            None => {
                let u = identity::resolve_identity(
                    db,
                    &self.settings.issuer,
                    &claims.sub,
                    claims.display_name(),
                    claims.email.as_deref(),
                    claims.email_verified(),
                )
                .await?
                .ok_or_else(|| {
                    AppError::forbidden("Hesabınız devre dışı; kurum yöneticinize başvurun.")
                })?;
                self.identities
                    .lock()
                    .await
                    .insert(claims.sub.clone(), (u, Instant::now()));
                u
            }
        };
        identity::actor_of(db, user, SignInMethod::Bearer)
            .await?
            .ok_or_else(|| {
                AppError::forbidden("Hesabınız devre dışı; kurum yöneticinize başvurun.")
            })
    }
}

#[cfg(test)]
mod tests;
