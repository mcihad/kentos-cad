//! Reaching a STAC asset (docs/adr/0644 §1): an `s3://` address as the HTTPS address a range reader asks, the AWS
//! signature (Signature Version 4) a request to a bucket carries when the user has AWS credentials (a requester-pays
//! bucket, Landsat's, wants it and `x-amz-request-payer`), and an Azure blob address with a shared access signature
//! (SAS; the Planetary Computer gives one per storage account and container). Pure: the host keeps the credentials
//! and the tokens, sends the requests and asks these for the headers and addresses. The shared cases are
//! `fixtures/services/v1/stac-access.json` (`scripts/fixtures/stac_access_cases.py`, checked against botocore).

use sha2::{Digest, Sha256};

/// The SHA-256 of nothing: a GET's payload hash.
pub const EMPTY_HASH: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// The region an `s3://` address without one is asked in.
pub const DEFAULT_REGION: &str = "us-east-1";

const UNRESERVED: &[u8] = b"-_.~";

/// AWS's URI encoding: letters, digits and `-_.~` kept, every other byte `%XX` (upper case); `/` kept in a path.
pub fn encode(text: &str, slash: bool) -> String {
    let mut out = String::with_capacity(text.len());
    for &b in text.as_bytes() {
        if b.is_ascii_alphanumeric() || UNRESERVED.contains(&b) || (slash && b == b'/') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// `%XX` decoded (a malformed escape kept as it is).
fn decode(text: &str) -> String {
    let b = text.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2]))
        {
            out.push(h * 16 + l);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// `s3://bucket/key` as the HTTPS address a range reader asks: `https://bucket.s3.<region>.amazonaws.com/<key>`, a
/// bucket with a dot in its name path style (`https://s3.<region>.amazonaws.com/<bucket>/<key>`: TLS names no dotted
/// bucket); none for another address.
pub fn s3_url(href: &str, region: Option<&str>) -> Option<String> {
    let rest = href.trim().strip_prefix("s3://")?;
    let (bucket, key) = rest.split_once('/').unwrap_or((rest, ""));
    if bucket.is_empty() {
        return None;
    }
    let region = region
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .unwrap_or(DEFAULT_REGION);
    let path = encode(key, true);
    Some(if bucket.contains('.') {
        format!("https://s3.{region}.amazonaws.com/{bucket}/{path}")
    } else {
        format!("https://{bucket}.s3.{region}.amazonaws.com/{path}")
    })
}

/// An S3 host's region (`bucket.s3.us-west-2.amazonaws.com`, `s3.eu-west-1.amazonaws.com`, `s3-us-west-2…`); the
/// global `s3.amazonaws.com` is us-east-1; none for another host.
pub fn s3_region(host: &str) -> Option<String> {
    let host = host.to_ascii_lowercase();
    let stem = host.strip_suffix(".amazonaws.com")?;
    let labels: Vec<&str> = stem.split('.').collect();
    let at = labels
        .iter()
        .rposition(|l| *l == "s3" || l.starts_with("s3-"))?;
    let label = labels[at];
    if let Some(r) = label.strip_prefix("s3-") {
        return Some(r.to_owned());
    }
    match labels.get(at + 1) {
        Some(r) if !r.is_empty() => Some((*r).to_owned()),
        _ => Some(DEFAULT_REGION.to_owned()),
    }
}

/// AWS credentials: the access key, the secret and, for temporary ones, the session token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Credentials {
    pub access: String,
    pub secret: String,
    pub token: Option<String>,
}

/// A request to sign: its method, address, headers (name, value) and time (`YYYYMMDDTHHMMSSZ`).
#[derive(Clone, Debug)]
pub struct Request<'a> {
    pub method: &'a str,
    pub url: &'a str,
    pub headers: &'a [(&'a str, &'a str)],
    pub time: &'a str,
    pub region: &'a str,
}

/// What signing made: the canonical request, the string to sign, the signature and the headers to send (lower-case
/// names: `authorization`, `x-amz-content-sha256`, `x-amz-date` and, with a token, `x-amz-security-token`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signed {
    pub canonical_request: String,
    pub string_to_sign: String,
    pub signature: String,
    pub signed_headers: String,
    pub authorization: String,
    pub headers: Vec<(String, String)>,
}

fn sha256_hex(data: &[u8]) -> String {
    hex_of(&Sha256::digest(data))
}

fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// HMAC-SHA256 (RFC 2104).
fn hmac(key: &[u8], message: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut k = [0u8; BLOCK];
    if key.len() > BLOCK {
        k[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let mut inner = Sha256::new();
    inner.update(k.map(|b| b ^ 0x36));
    inner.update(message);
    let mut outer = Sha256::new();
    outer.update(k.map(|b| b ^ 0x5c));
    outer.update(inner.finalize());
    outer.finalize().into()
}

/// The parts of an address: host (with its port when given), path, query.
fn parts(url: &str) -> Option<(&str, &str, &str)> {
    let rest = url.split_once("://")?.1;
    let (authority, rest) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let rest = rest.split('#').next().unwrap_or("");
    let (path, query) = rest.split_once('?').unwrap_or((rest, ""));
    Some((authority, path, query))
}

fn canonical_query(query: &str) -> String {
    if query.is_empty() {
        return String::new();
    }
    let mut pairs: Vec<(String, String)> = query
        .split('&')
        .map(|p| {
            let (k, v) = p.split_once('=').unwrap_or((p, ""));
            (encode(&decode(k), false), encode(&decode(v), false))
        })
        .collect();
    pairs.sort();
    pairs
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&")
}

/// The request signed for S3 with `c` (AWS Signature Version 4, an unsigned-body GET or HEAD); none for an address
/// that does not read.
pub fn sign(c: &Credentials, r: &Request<'_>) -> Option<Signed> {
    let (host, path, query) = parts(r.url)?;
    let mut headers: Vec<(String, String)> = r
        .headers
        .iter()
        .map(|(n, v)| {
            (
                n.to_ascii_lowercase(),
                v.split_whitespace().collect::<Vec<_>>().join(" "),
            )
        })
        .collect();
    headers.retain(|(n, _)| n != "host" && n != "x-amz-content-sha256" && n != "x-amz-date");
    headers.push(("host".into(), host.to_owned()));
    headers.push(("x-amz-content-sha256".into(), EMPTY_HASH.into()));
    headers.push(("x-amz-date".into(), r.time.to_owned()));
    if let Some(t) = &c.token {
        headers.retain(|(n, _)| n != "x-amz-security-token");
        headers.push(("x-amz-security-token".into(), t.clone()));
    }
    headers.sort();
    let canonical_headers: String = headers.iter().map(|(n, v)| format!("{n}:{v}\n")).collect();
    let signed_headers = headers
        .iter()
        .map(|(n, _)| n.as_str())
        .collect::<Vec<_>>()
        .join(";");
    let path = if path.is_empty() { "/" } else { path };
    let canonical_request = [
        r.method,
        path,
        &canonical_query(query),
        &canonical_headers,
        &signed_headers,
        EMPTY_HASH,
    ]
    .join("\n");
    let day = r.time.get(..8)?;
    let scope = format!("{day}/{}/s3/aws4_request", r.region);
    let string_to_sign = [
        "AWS4-HMAC-SHA256",
        r.time,
        &scope,
        &sha256_hex(canonical_request.as_bytes()),
    ]
    .join("\n");
    let mut k = hmac(format!("AWS4{}", c.secret).as_bytes(), day.as_bytes());
    for part in [r.region, "s3", "aws4_request"] {
        k = hmac(&k, part.as_bytes());
    }
    let signature = hex_of(&hmac(&k, string_to_sign.as_bytes()));
    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signed_headers}, Signature={signature}",
        c.access
    );
    let mut send = vec![
        ("authorization".to_owned(), authorization.clone()),
        ("x-amz-content-sha256".to_owned(), EMPTY_HASH.to_owned()),
        ("x-amz-date".to_owned(), r.time.to_owned()),
    ];
    if let Some(t) = &c.token {
        send.push(("x-amz-security-token".to_owned(), t.clone()));
    }
    Some(Signed {
        canonical_request,
        string_to_sign,
        signature,
        signed_headers,
        authorization,
        headers: send,
    })
}

/// An Azure blob address's storage account and container; none for another address (or a plain `http` one: a SAS
/// travels only over TLS).
pub fn blob_parts(url: &str) -> Option<(String, String)> {
    let rest = url.trim().strip_prefix("https://")?;
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let host = host.split(':').next().unwrap_or(host).to_ascii_lowercase();
    let account = host.strip_suffix(".blob.core.windows.net")?;
    if account.is_empty() || account.contains('.') {
        return None;
    }
    let path = path.split(['?', '#']).next().unwrap_or("");
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() < 2 {
        return None;
    }
    Some((account.to_owned(), segments[0].to_owned()))
}

/// A blob address with a SAS token after its own query.
pub fn with_sas(url: &str, token: &str) -> String {
    let token = token.trim_start_matches('?');
    if url.contains('?') {
        format!("{url}&{token}")
    } else {
        format!("{url}?{token}")
    }
}

/// The Planetary Computer's address that gives a container's SAS token.
pub fn planetary_token_url(account: &str, container: &str) -> String {
    format!(
        "https://planetarycomputer.microsoft.com/api/sas/v1/token/{}/{}",
        encode(account, false),
        encode(container, false)
    )
}

/// A token answer (`{"msft:expiry": "…", "token": "…"}`): the token and its expiry as the answer writes it.
pub fn read_token(bytes: &[u8]) -> Result<(String, Option<String>), String> {
    let v: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| format!("SAS cevabı okunamadı: {e}"))?;
    let token = v
        .get("token")
        .and_then(serde_json::Value::as_str)
        .filter(|t| !t.trim().is_empty())
        .ok_or("SAS cevabında token yok.")?;
    let expiry = v
        .get("msft:expiry")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    Ok((token.to_owned(), expiry))
}

/// A section of an AWS shared credentials file (`~/.aws/credentials`, INI): its keys, none when the profile is not
/// there or has no key pair.
pub fn read_credentials(text: &str, profile: &str) -> Option<Credentials> {
    let mut on = false;
    let (mut access, mut secret, mut token) = (None, None, None);
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            on = name.trim() == profile;
            continue;
        }
        if !on {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let v = v.trim().to_owned();
        match k.trim().to_ascii_lowercase().as_str() {
            "aws_access_key_id" => access = Some(v),
            "aws_secret_access_key" => secret = Some(v),
            "aws_session_token" => token = Some(v),
            _ => {}
        }
    }
    Some(Credentials {
        access: access.filter(|a| !a.is_empty())?,
        secret: secret.filter(|s| !s.is_empty())?,
        token: token.filter(|t| !t.is_empty()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cases() -> serde_json::Value {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../fixtures/services/v1/stac-access.json"
        );
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn signatures_are_awss_and_botocores() {
        let doc = cases();
        let all = doc["sigv4"].as_array().unwrap();
        assert!(all.len() >= 10);
        for c in all {
            let headers: Vec<(String, String)> = c["headers"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_owned()))
                .collect();
            let headers: Vec<(&str, &str)> = headers
                .iter()
                .map(|(a, b)| (a.as_str(), b.as_str()))
                .collect();
            let creds = Credentials {
                access: c["access"].as_str().unwrap().into(),
                secret: c["secret"].as_str().unwrap().into(),
                token: c["token"].as_str().map(str::to_owned),
            };
            let got = sign(
                &creds,
                &Request {
                    method: c["method"].as_str().unwrap(),
                    url: c["url"].as_str().unwrap(),
                    headers: &headers,
                    time: c["time"].as_str().unwrap(),
                    region: c["region"].as_str().unwrap(),
                },
            )
            .unwrap();
            let e = &c["expect"];
            let name = c["name"].as_str().unwrap();
            assert_eq!(
                got.canonical_request,
                e["canonicalRequest"].as_str().unwrap(),
                "{name}"
            );
            assert_eq!(
                got.string_to_sign,
                e["stringToSign"].as_str().unwrap(),
                "{name}"
            );
            assert_eq!(got.signature, e["signature"].as_str().unwrap(), "{name}");
            assert_eq!(
                got.authorization,
                e["authorization"].as_str().unwrap(),
                "{name}"
            );
        }
    }

    #[test]
    fn s3_addresses_and_sas_tokens_are_the_references() {
        let doc = cases();
        for c in doc["s3"].as_array().unwrap() {
            assert_eq!(
                s3_url(c["href"].as_str().unwrap(), c["region"].as_str()).as_deref(),
                c["expect"].as_str(),
                "{}",
                c["name"]
            );
        }
        for c in doc["sas"].as_array().unwrap() {
            let url = c["url"].as_str().unwrap();
            let parts = blob_parts(url);
            let want = &c["expect"]["parts"];
            match parts {
                Some((a, k)) => {
                    assert_eq!(want["account"], a, "{}", c["name"]);
                    assert_eq!(want["container"], k, "{}", c["name"]);
                    assert_eq!(
                        with_sas(url, c["token"].as_str().unwrap()),
                        c["expect"]["signed"].as_str().unwrap()
                    );
                }
                None => assert!(want.is_null(), "{}", c["name"]),
            }
        }
    }

    #[test]
    fn regions_tokens_and_credentials_read() {
        assert_eq!(
            s3_region("usgs-landsat.s3.us-west-2.amazonaws.com").as_deref(),
            Some("us-west-2")
        );
        assert_eq!(
            s3_region("s3.eu-west-1.amazonaws.com").as_deref(),
            Some("eu-west-1")
        );
        assert_eq!(
            s3_region("kova.s3.amazonaws.com").as_deref(),
            Some("us-east-1")
        );
        assert_eq!(
            s3_region("s3-ap-south-1.amazonaws.com").as_deref(),
            Some("ap-south-1")
        );
        assert_eq!(s3_region("example.com"), None);
        let (t, e) =
            read_token(br#"{"msft:expiry":"2026-10-27T10:00:00Z","token":"st=1&sig=x"}"#).unwrap();
        assert_eq!(
            (t.as_str(), e.as_deref()),
            ("st=1&sig=x", Some("2026-10-27T10:00:00Z"))
        );
        assert!(read_token(b"{}").is_err());
        let ini = "[default]\naws_access_key_id = AK\naws_secret_access_key=SK\n\n[is]\naws_access_key_id=B\n";
        assert_eq!(
            read_credentials(ini, "default"),
            Some(Credentials {
                access: "AK".into(),
                secret: "SK".into(),
                token: None
            })
        );
        assert_eq!(read_credentials(ini, "is"), None);
        assert_eq!(
            planetary_token_url("sentinel2l2a01", "sentinel2-l2"),
            "https://planetarycomputer.microsoft.com/api/sas/v1/token/sentinel2l2a01/sentinel2-l2"
        );
    }
}
