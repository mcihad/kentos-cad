//! A connection's proof added to a request (docs/adr/0208 §12): parameters
//! in the address, headers, HTTP Basic, a bearer token, an ArcGIS token, an
//! OAuth 2 access token, Google's key. Proof goes to the connection's origin
//! only: a request to any other origin is left as it is. A token is asked
//! for with [`token_request`] and read with [`read_token`]; the host keeps
//! it until it is about to expire.

use kentos_contracts::{AuthKind, ConnectionSecret, ServiceConnection, origin_of};

use crate::query::with_params;
use crate::request::Request;

/// A token a connection was given: its text and when it stops (ms since the epoch).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub value: String,
    pub expires_ms: u64,
}

impl Token {
    /// Whether it should be asked for again: within a minute of its end.
    pub fn stale(&self, now_ms: u64) -> bool {
        now_ms.saturating_add(60_000) >= self.expires_ms
    }
}

/// Base64 (RFC 4648, with padding).
pub fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        out.push(char::from(ABC[(n >> 18) as usize & 63]));
        out.push(char::from(ABC[(n >> 12) as usize & 63]));
        out.push(if chunk.len() > 1 {
            char::from(ABC[(n >> 6) as usize & 63])
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            char::from(ABC[n as usize & 63])
        } else {
            '='
        });
    }
    out
}

/// Whether `url` goes to the connection's origin.
pub fn same_origin(conn: &ServiceConnection, url: &str) -> bool {
    origin_of(url).is_some_and(|o| o == conn.origin)
}

/// Why a connection's secrets cannot prove it (a value missing), in the
/// windows' words; none when they can.
pub fn missing(conn: &ServiceConnection, secret: Option<&ConnectionSecret>) -> Option<String> {
    let Some(s) = secret else {
        return (conn.auth != AuthKind::None).then(|| {
            format!(
                "“{}” bağlantısının kimlik bilgileri bu cihazda yok: Bağlantılar'dan tamamlayın.",
                conn.name
            )
        });
    };
    let blank = |v: &Option<String>| v.as_deref().is_none_or(|t| t.trim().is_empty());
    let lacks = match conn.auth {
        AuthKind::None => false,
        AuthKind::Query | AuthKind::Header => {
            s.values.len() < conn.names.len() || s.values.iter().any(|v| v.is_empty())
        }
        AuthKind::Basic | AuthKind::Arcgis => blank(&s.user) || blank(&s.password),
        AuthKind::Bearer | AuthKind::Google => blank(&s.token),
        AuthKind::Oauth2 => blank(&s.client_id) || blank(&s.client_secret),
    };
    lacks.then(|| {
        format!(
            "“{}” bağlantısının {} eksik: Bağlantılar'dan tamamlayın.",
            conn.name,
            conn.auth.label().to_lowercase()
        )
    })
}

/// `req` with the connection's proof when it goes to the connection's
/// origin. An ArcGIS or OAuth 2 connection needs its `token`; without one
/// the request stays as it is (the host asks for a token first).
pub fn apply(
    conn: &ServiceConnection,
    secret: &ConnectionSecret,
    token: Option<&Token>,
    req: &mut Request,
) {
    if !same_origin(conn, &req.url) {
        return;
    }
    match conn.auth {
        AuthKind::None => {}
        AuthKind::Query => {
            let pairs: Vec<(&str, &str)> = conn
                .names
                .iter()
                .zip(&secret.values)
                .map(|(n, v)| (n.as_str(), v.as_str()))
                .collect();
            req.url = with_params(&req.url, &pairs);
        }
        AuthKind::Header => {
            for (n, v) in conn.names.iter().zip(&secret.values) {
                req.headers.push((n.clone(), v.clone()));
            }
        }
        AuthKind::Basic => {
            let pair = format!(
                "{}:{}",
                secret.user.as_deref().unwrap_or(""),
                secret.password.as_deref().unwrap_or("")
            );
            req.headers.push((
                "Authorization".into(),
                format!("Basic {}", base64(pair.as_bytes())),
            ));
        }
        AuthKind::Bearer => {
            if let Some(t) = &secret.token {
                req.headers
                    .push(("Authorization".into(), format!("Bearer {}", t.trim())));
            }
        }
        AuthKind::Arcgis => {
            if let Some(t) = token {
                req.url = with_params(&req.url, &[("token", &t.value)]);
            }
        }
        AuthKind::Oauth2 => {
            if let Some(t) = token {
                req.headers
                    .push(("Authorization".into(), format!("Bearer {}", t.value)));
            }
        }
        AuthKind::Google => {
            if let Some(k) = &secret.token {
                req.url = with_params(&req.url, &[("key", k.trim())]);
            }
        }
    }
}

/// The names of the parameters that carry secrets for this connection
/// (left out of cache keys and log lines).
pub fn secret_params(conn: &ServiceConnection) -> Vec<String> {
    match conn.auth {
        AuthKind::Query => conn.names.clone(),
        AuthKind::Arcgis => vec!["token".into()],
        AuthKind::Google => vec!["key".into(), "session".into()],
        _ => Vec::new(),
    }
}

/// The request that gets an ArcGIS or OAuth 2 connection its token; none for
/// the other kinds. An ArcGIS token address absent is the server's own
/// (`<origin>/arcgis/tokens/generateToken`); `referer` is the address the
/// token is asked for (ArcGIS binds the token to it).
pub fn token_request(
    conn: &ServiceConnection,
    secret: &ConnectionSecret,
    referer: &str,
) -> Option<Request> {
    match conn.auth {
        AuthKind::Arcgis => {
            let url = conn
                .token_url
                .clone()
                .unwrap_or_else(|| format!("{}/arcgis/tokens/generateToken", conn.origin));
            Some(Request::form(
                url,
                &[
                    ("username", secret.user.as_deref().unwrap_or("")),
                    ("password", secret.password.as_deref().unwrap_or("")),
                    ("client", "referer"),
                    ("referer", referer),
                    ("expiration", "120"),
                    ("f", "json"),
                ],
            ))
        }
        AuthKind::Oauth2 => {
            let mut fields = vec![
                ("grant_type", "client_credentials"),
                ("client_id", secret.client_id.as_deref().unwrap_or("")),
                (
                    "client_secret",
                    secret.client_secret.as_deref().unwrap_or(""),
                ),
            ];
            if let Some(s) = &conn.scope {
                fields.push(("scope", s));
            }
            Some(Request::form(conn.token_url.clone()?, &fields))
        }
        _ => None,
    }
}

/// The token an answer to [`token_request`] gives, or why there is none
/// (the service's own words when it says them).
pub fn read_token(conn: &ServiceConnection, body: &str, now_ms: u64) -> Result<Token, String> {
    let v: serde_json::Value = serde_json::from_str(body)
        .map_err(|_| "Belirteç adresinin yanıtı JSON değil.".to_owned())?;
    let said = |v: &serde_json::Value| {
        v["error"]["message"]
            .as_str()
            .or_else(|| v["error_description"].as_str())
            .or_else(|| v["error"].as_str())
            .map(str::to_owned)
    };
    match conn.auth {
        AuthKind::Arcgis => {
            let value = v["token"].as_str().ok_or_else(|| {
                said(&v).map_or_else(
                    || "ArcGIS belirteç vermedi.".to_owned(),
                    |m| format!("ArcGIS belirteç vermedi: {m}"),
                )
            })?;
            // `expires` is ms since the epoch.
            let expires_ms = v["expires"].as_u64().unwrap_or(now_ms + 60 * 60_000);
            Ok(Token {
                value: value.to_owned(),
                expires_ms,
            })
        }
        AuthKind::Oauth2 => {
            let value = v["access_token"].as_str().ok_or_else(|| {
                said(&v).map_or_else(
                    || "Belirteç adresi erişim belirteci vermedi.".to_owned(),
                    |m| format!("Belirteç adresi erişim belirteci vermedi: {m}"),
                )
            })?;
            let seconds = v["expires_in"].as_u64().unwrap_or(3600);
            Ok(Token {
                value: value.to_owned(),
                expires_ms: now_ms + seconds * 1000,
            })
        }
        _ => Err("Bu bağlantı belirteç istemez.".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn(auth: AuthKind, names: &[&str]) -> ServiceConnection {
        ServiceConnection {
            id: "c".into(),
            name: "Kurum".into(),
            origin: "https://kbs.ornek.bel.tr".into(),
            auth,
            names: names.iter().map(|n| (*n).to_owned()).collect(),
            token_url: None,
            scope: None,
        }
    }

    #[test]
    fn base64_as_rfc_4648() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(
            base64("kullanıcı:şifre".as_bytes()),
            "a3VsbGFuxLFjxLE6xZ9pZnJl"
        );
    }

    #[test]
    fn proof_goes_only_to_the_connections_origin() {
        let c = conn(AuthKind::Query, &["apikey"]);
        let s = ConnectionSecret {
            id: "c".into(),
            values: vec!["S3CR3T".into()],
            ..ConnectionSecret::default()
        };
        let mut r = Request::get("https://kbs.ornek.bel.tr/wms?SERVICE=WMS");
        apply(&c, &s, None, &mut r);
        assert_eq!(
            r.url,
            "https://kbs.ornek.bel.tr/wms?SERVICE=WMS&apikey=S3CR3T"
        );
        let mut other = Request::get("https://tile.openstreetmap.org/1/1/1.png");
        apply(&c, &s, None, &mut other);
        assert_eq!(other.url, "https://tile.openstreetmap.org/1/1/1.png");
        let b = conn(AuthKind::Basic, &[]);
        let s = ConnectionSecret {
            id: "c".into(),
            user: Some("ali".into()),
            password: Some("p".into()),
            ..ConnectionSecret::default()
        };
        let mut r = Request::get("https://kbs.ornek.bel.tr:443/x");
        apply(&b, &s, None, &mut r);
        assert_eq!(
            r.headers,
            vec![("Authorization".to_owned(), "Basic YWxpOnA=".to_owned())]
        );
        assert_eq!(missing(&b, Some(&s)), None);
        assert!(missing(&b, None).unwrap().contains("bu cihazda yok"));
    }

    #[test]
    fn tokens_are_asked_for_and_read() {
        let a = conn(AuthKind::Arcgis, &[]);
        let s = ConnectionSecret {
            id: "c".into(),
            user: Some("ali".into()),
            password: Some("p&q".into()),
            ..ConnectionSecret::default()
        };
        let r = token_request(&a, &s, "https://kentos.app").expect("a request");
        assert_eq!(
            r.url,
            "https://kbs.ornek.bel.tr/arcgis/tokens/generateToken"
        );
        assert!(r.body.unwrap().0.contains("password=p%26q"));
        let t = read_token(&a, r#"{"token":"T","expires":1800000000000}"#, 0).unwrap();
        assert_eq!(
            t,
            Token {
                value: "T".into(),
                expires_ms: 1_800_000_000_000
            }
        );
        assert!(
            read_token(&a, r#"{"error":{"message":"Invalid credentials"}}"#, 0)
                .unwrap_err()
                .contains("Invalid credentials")
        );
        let o = ServiceConnection {
            token_url: Some("https://auth.ornek/token".into()),
            ..conn(AuthKind::Oauth2, &[])
        };
        let t = read_token(&o, r#"{"access_token":"A","expires_in":300}"#, 1000).unwrap();
        assert_eq!(t.expires_ms, 301_000);
        assert!(t.stale(250_000) && !t.stale(200_000));
    }
}
