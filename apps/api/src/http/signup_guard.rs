//! What keeps a machine from opening accounts (docs/adr/9000 §20; plan BUL-12), on `POST /v1/portal/signup`:
//!
//! - **A ticket.** `GET /v1/portal/signup` gives the form a ticket: when it was issued, a nonce and the server's
//!   HMAC of both. The sign-up must bring it back no sooner than `MIN_FILL` (a person reads and types a form; a script
//!   posts at once) and no later than `MAX_AGE`, and only once.
//! - **A field no person fills.** The form has a `website` field its people do not see; filled, the request is a
//!   machine's.
//! - **A limit per address.** Tries and accounts per client address and window (`RateLimiter`); the accounts per hour
//!   are `KENTOS_SIGNUP_PER_HOUR` (5 by default).
//!
//! The secret is made when the process starts: a restart only makes the open forms ask for a new ticket.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::request::Parts;

use super::limit::RateLimiter;
use crate::delivery::{hex, hmac_sha256};

/// The least time between giving the form its ticket and taking the sign-up.
pub const MIN_FILL: u64 = 3;
/// The longest a form may stay open.
pub const MAX_AGE: u64 = 2 * 3600;
/// Tries per address in `TRY_WINDOW`.
pub const MAX_TRIES: u32 = 20;
pub const TRY_WINDOW: Duration = Duration::from_secs(10 * 60);

pub struct SignupGuard {
    secret: [u8; 32],
    used: Mutex<HashMap<String, Instant>>,
    pub tries: RateLimiter,
    pub accounts: RateLimiter,
}

impl Default for SignupGuard {
    fn default() -> Self {
        Self::new(5)
    }
}

impl SignupGuard {
    pub fn new(accounts_per_hour: u32) -> Self {
        let mut secret = [0u8; 32];
        secret[..16].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
        secret[16..].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
        Self {
            secret,
            used: Mutex::new(HashMap::new()),
            tries: RateLimiter::new(MAX_TRIES, TRY_WINDOW),
            accounts: RateLimiter::new(accounts_per_hour.max(1), Duration::from_secs(3600)),
        }
    }

    fn mac(&self, issued: u64, nonce: &str) -> String {
        hex(&hmac_sha256(
            &self.secret,
            format!("signup:{issued}.{nonce}").as_bytes(),
        ))
    }

    /// A ticket issued at `now` (Unix seconds).
    pub fn ticket(&self, now: u64) -> String {
        let nonce = uuid::Uuid::new_v4().simple().to_string();
        format!("{now}.{nonce}.{}", self.mac(now, &nonce))
    }

    /// Takes the ticket back at `now`: why not, in the person's words, when it does not do.
    pub fn redeem(&self, ticket: Option<&str>, now: u64) -> Result<(), &'static str> {
        const REOPEN: &str = "Kayıt formunu yeniden açın (sayfayı yenileyin).";
        let ticket = ticket
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .ok_or(REOPEN)?;
        let mut parts = ticket.splitn(3, '.');
        let (Some(issued), Some(nonce), Some(mac)) = (parts.next(), parts.next(), parts.next())
        else {
            return Err(REOPEN);
        };
        let issued: u64 = issued.parse().map_err(|_| REOPEN)?;
        let expected = self.mac(issued, nonce);
        // Every byte compared, so the time taken says nothing about where it differs.
        let same = expected.len() == mac.len()
            && expected
                .bytes()
                .zip(mac.bytes())
                .fold(0u8, |acc, (a, b)| acc | (a ^ b))
                == 0;
        if !same {
            return Err(REOPEN);
        }
        let age = now.saturating_sub(issued);
        if issued > now + 60 || age > MAX_AGE {
            return Err("Kayıt formunun süresi doldu; sayfayı yenileyip yeniden doldurun.");
        }
        if age < MIN_FILL {
            return Err("Form çok çabuk gönderildi; birkaç saniye bekleyip yeniden gönderin.");
        }
        let mut used = self.used.lock().expect("ticket lock");
        if used.len() > 10_000 {
            used.retain(|_, at| at.elapsed() < Duration::from_secs(MAX_AGE));
        }
        if used.insert(nonce.to_owned(), Instant::now()).is_some() {
            return Err(REOPEN);
        }
        Ok(())
    }
}

/// The address a request came from: the connection's, or — when the connection is a reverse proxy on this machine —
/// the last address that proxy put in `X-Forwarded-For`. A header from anyone else is not believed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClientAddr(pub Option<IpAddr>);

impl ClientAddr {
    /// The key a limit counts under.
    pub fn key(&self) -> String {
        self.0
            .map_or_else(|| "bilinmeyen".to_owned(), |ip| ip.to_string())
    }
}

impl<S: Send + Sync> FromRequestParts<S> for ClientAddr {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let peer = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|c| c.0.ip());
        let forwarded = peer.filter(IpAddr::is_loopback).and_then(|_| {
            parts
                .headers
                .get("x-forwarded-for")?
                .to_str()
                .ok()?
                .rsplit(',')
                .next()?
                .trim()
                .parse::<IpAddr>()
                .ok()
        });
        Ok(Self(forwarded.or(peer)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ticket_is_taken_back_once_after_a_person_s_while_and_before_it_grows_old() {
        let g = SignupGuard::default();
        let t = g.ticket(1_000);
        assert!(g.redeem(Some(&t), 1_001).unwrap_err().contains("çok çabuk"));
        assert_eq!(g.redeem(Some(&t), 1_000 + MIN_FILL), Ok(()));
        assert!(g.redeem(Some(&t), 1_010).is_err(), "only once");
        let old = g.ticket(1_000);
        assert!(
            g.redeem(Some(&old), 1_000 + MAX_AGE + 1)
                .unwrap_err()
                .contains("süresi doldu")
        );
        // Another server's ticket, a changed one, none at all.
        let other = SignupGuard::default().ticket(1_000);
        assert!(
            g.redeem(Some(&other), 1_010)
                .unwrap_err()
                .contains("yeniden açın")
        );
        let forged = g.ticket(1_000).replacen("1000.", "900.", 1);
        assert!(g.redeem(Some(&forged), 1_010).is_err());
        assert!(g.redeem(None, 1_010).is_err() && g.redeem(Some(""), 1_010).is_err());
    }
}
