//! An asset's address as the project keeps it (docs/adr/0528 §4): a signed address is a temporary proof, not a
//! place. When the query holds a signature or a token the whole query is dropped (the signature's own
//! parameters, an expiry and a policy go with it), and a login in the authority is dropped; `signed` says it
//! was there. An address with a query that is none of these (`?versionId=…`) stays as it is.

/// Query parameter names that carry a signature, a credential or a token (lower case).
const PROOF: &[&str] = &[
    "sig",
    "signature",
    "x-amz-signature",
    "x-amz-credential",
    "x-amz-security-token",
    "awsaccesskeyid",
    "x-goog-signature",
    "x-goog-credential",
    "token",
    "sas_token",
    "access_token",
    "api_key",
    "apikey",
    "key-pair-id",
    "policy",
];

/// `href` as the project keeps it, and whether a signature or a login was dropped.
pub fn stable(href: &str) -> (String, bool) {
    let mut signed = false;
    let mut rest = href.trim();
    let mut head = String::new();
    // A login in the authority: `https://user:pass@host/…`.
    if let Some((scheme, after)) = rest.split_once("://") {
        let authority_end = after.find(['/', '?', '#']).unwrap_or(after.len());
        let (authority, tail) = after.split_at(authority_end);
        if let Some((_, host)) = authority.rsplit_once('@') {
            signed = true;
            head = format!("{scheme}://{host}");
        } else {
            head = format!("{scheme}://{authority}");
        }
        rest = tail;
    }
    let (path, query) = match rest.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (rest, None),
    };
    let (query, fragment) = match query {
        Some(q) => match q.split_once('#') {
            Some((q, f)) => (Some(q), Some(f)),
            None => (Some(q), None),
        },
        None => (None, path.split_once('#').map(|(_, f)| f)),
    };
    let path = path.split('#').next().unwrap_or(path);
    let proof = query.is_some_and(|q| {
        q.split('&').any(|pair| {
            let key = pair.split('=').next().unwrap_or("").to_ascii_lowercase();
            PROOF.contains(&key.as_str())
        })
    });
    let mut out = format!("{head}{path}");
    if proof {
        signed = true;
    } else if let Some(q) = query.filter(|q| !q.is_empty()) {
        out.push('?');
        out.push_str(q);
    }
    if let Some(f) = fragment.filter(|f| !f.is_empty()) {
        out.push('#');
        out.push_str(f);
    }
    (out, signed)
}

/// Whether an address can be read by a raster: http or https.
pub fn is_http(href: &str) -> bool {
    let l = href.trim().to_ascii_lowercase();
    (l.starts_with("http://") && l.len() > 7) || (l.starts_with("https://") && l.len() > 8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signatures_never_stay() {
        assert_eq!(
            stable(
                "https://a.blob.core.windows.net/c/x.tif?st=2024&se=2024&sp=r&sv=2021&sr=c&sig=abc%3D"
            ),
            ("https://a.blob.core.windows.net/c/x.tif".into(), true)
        );
        assert_eq!(
            stable(
                "https://b.s3.amazonaws.com/x.tif?X-Amz-Algorithm=AWS4&X-Amz-Credential=k&X-Amz-Signature=s"
            ),
            ("https://b.s3.amazonaws.com/x.tif".into(), true)
        );
        assert_eq!(
            stable("https://u:p@h.example/x.tif"),
            ("https://h.example/x.tif".into(), true)
        );
        assert_eq!(
            stable("https://h.example/x.tif?token=abc"),
            ("https://h.example/x.tif".into(), true)
        );
    }

    #[test]
    fn plain_addresses_stay() {
        assert_eq!(
            stable("https://h.example/x.tif?versionId=3"),
            ("https://h.example/x.tif?versionId=3".into(), false)
        );
        assert_eq!(
            stable("s3://bucket/x.TIF"),
            ("s3://bucket/x.TIF".into(), false)
        );
        assert_eq!(
            stable("  https://h.example/a/b.tif "),
            ("https://h.example/a/b.tif".into(), false)
        );
    }

    #[test]
    fn only_http_is_readable() {
        assert!(is_http("https://h/x"));
        assert!(is_http("HTTP://h/x"));
        assert!(!is_http("s3://b/x"));
        assert!(!is_http("gs://b/x"));
        assert!(!is_http("https://"));
        assert!(!is_http("/local/x.tif"));
    }
}
