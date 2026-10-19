//! A STAC endpoint's address (docs/adr/0528 §3): an http(s) address of a host with no login, query or fragment,
//! its trailing slashes dropped, its scheme and host in lower case; and the origin a link must share with it.

/// The endpoint's text normalized, or why it is not one.
pub fn normalize(text: &str) -> Result<String, String> {
    let t = text.trim();
    if t.is_empty() {
        return Err("STAC adresi boş.".into());
    }
    if t.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err("STAC adresi boşluk ya da denetim karakteri içeremez.".into());
    }
    let not_http = || {
        format!("“{t}” bir http ya da https adresi değil (http:// ya da https:// ile başlamalı).")
    };
    let Some((scheme, rest)) = t.split_once("://") else {
        return Err(not_http());
    };
    let scheme = scheme.to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(not_http());
    }
    if rest.contains('?') || rest.contains('#') {
        return Err(
            "STAC adresi sorgu ya da parça içeremez; arama parametreleri komutun alanlarıdır."
                .into(),
        );
    }
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    if authority.contains('@') {
        return Err(
            "STAC adresi kullanıcı adı ya da parola içeremez; gizli değerler adrese yazılmaz."
                .into(),
        );
    }
    if authority.is_empty() || authority.starts_with(':') {
        return Err("STAC adresinin sunucu adı yok.".into());
    }
    let path = path.trim_end_matches('/');
    Ok(format!(
        "{scheme}://{}{path}",
        authority.to_ascii_lowercase()
    ))
}

/// The scheme and authority of an address in lower case (`https://host:8443`), none when it has none.
pub fn origin(url: &str) -> Option<String> {
    let (scheme, rest) = url.trim().split_once("://")?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.is_empty() || authority.contains('@') {
        return None;
    }
    Some(format!(
        "{}://{}",
        scheme.to_ascii_lowercase(),
        authority.to_ascii_lowercase()
    ))
}

/// Whether `href` leads to the same origin as the (normalized) `endpoint`.
pub fn same_origin(endpoint: &str, href: &str) -> bool {
    match (origin(endpoint), origin(href)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

/// A link's address made absolute: `/path` is taken from the endpoint's origin, `path` from the endpoint, an
/// absolute address stays. None for an empty one.
pub fn resolve(endpoint: &str, href: &str) -> Option<String> {
    let h = href.trim();
    if h.contains("://") {
        return Some(h.to_owned());
    }
    if let Some(path) = h.strip_prefix('/') {
        return Some(format!("{}/{path}", origin(endpoint)?));
    }
    (!h.is_empty()).then(|| format!("{}/{h}", endpoint.trim_end_matches('/')))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_are_normalized() {
        assert_eq!(
            normalize("HTTPS://Earth-Search.AWS.element84.com/v1/").as_deref(),
            Ok("https://earth-search.aws.element84.com/v1")
        );
        assert_eq!(
            normalize(" http://localhost:8080 ").as_deref(),
            Ok("http://localhost:8080")
        );
        assert!(normalize("").is_err());
        assert!(normalize("ftp://x/y").is_err());
        assert!(normalize("https://x/y?a=1").is_err());
        assert!(normalize("https://u:p@x/y").is_err());
        assert!(normalize("https:///v1").is_err());
        assert!(normalize("https://x /v1").is_err());
    }

    #[test]
    fn origins_and_links() {
        let e = "https://a.example/v1";
        assert!(same_origin(e, "https://A.example/v1/search"));
        assert!(!same_origin(e, "https://b.example/v1/search"));
        assert!(!same_origin(e, "http://a.example/v1/search"));
        assert!(!same_origin(e, "https://a.example:8443/x"));
        assert_eq!(
            resolve(e, "/v1/search").as_deref(),
            Some("https://a.example/v1/search")
        );
        assert_eq!(
            resolve(e, "search").as_deref(),
            Some("https://a.example/v1/search")
        );
        assert_eq!(resolve(e, "https://c/x").as_deref(), Some("https://c/x"));
        assert_eq!(resolve(e, ""), None);
    }
}
