//! Addresses as services take them (RFC 3986): a query's values
//! percent-encoded, parameters added to an address that may have its own
//! (OGC's parameter names are not case-sensitive: one given again replaces
//! the address's own), a parameter read back, and parameters left out of a
//! cache key.

/// Whether a byte stays as it is in a query value: RFC 3986's unreserved
/// characters and those a query may hold that a service reads as they are
/// (`,` in a BBOX or a layer list, `:` in a CRS, `/` in a media type, `*`
/// in ArcGIS's `outFields`).
fn kept(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~' | b',' | b':' | b'/' | b'*')
}

/// `text` percent-encoded for a query value (UTF-8, upper-case hex).
pub fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for &b in text.as_bytes() {
        if kept(b) {
            out.push(char::from(b));
        } else {
            out.push('%');
            out.push(char::from(b"0123456789ABCDEF"[usize::from(b >> 4)]));
            out.push(char::from(b"0123456789ABCDEF"[usize::from(b & 15)]));
        }
    }
    out
}

/// A number as a request writes it: the shortest text that reads back as
/// the same number, without an exponent; −0 as 0.
pub fn number(x: f64) -> String {
    if x == 0.0 {
        "0".to_owned()
    } else {
        format!("{x}")
    }
}

/// `text` percent-decoded (`%41` → `A`); a `%` without two hex digits stays.
/// A `+` stays a `+`: the addresses here are written, not sent by a form.
pub fn decode(text: &str) -> String {
    let b = text.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let hex = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
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

/// An address split at its query and fragment: what comes before `?`, the
/// query's pairs (name, value, and the pair as written), the fragment with
/// its `#`.
fn split(url: &str) -> (&str, Vec<(&str, &str, &str)>, &str) {
    let (rest, fragment) = match url.find('#') {
        Some(i) => (&url[..i], &url[i..]),
        None => (url, ""),
    };
    let (base, query) = match rest.find('?') {
        Some(i) => (&rest[..i], &rest[i + 1..]),
        None => (rest, ""),
    };
    let pairs = query
        .split('&')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (n, v) = p.split_once('=').unwrap_or((p, ""));
            (n, v, p)
        })
        .collect();
    (base, pairs, fragment)
}

/// `url` with `params` added after its own query; a parameter it has
/// already (its name in any case) is replaced where it stands. Values are
/// encoded here; the address's own stay as written.
pub fn with_params(url: &str, params: &[(&str, &str)]) -> String {
    let (base, own, fragment) = split(url);
    let mut out = String::with_capacity(url.len() + params.len() * 16);
    out.push_str(base);
    let mut first = true;
    let mut sep = |out: &mut String| {
        out.push(if first { '?' } else { '&' });
        first = false;
    };
    let mut used = vec![false; params.len()];
    for (name, _, raw) in &own {
        match params
            .iter()
            .position(|(n, _)| n.eq_ignore_ascii_case(name))
        {
            Some(k) => {
                if !used[k] {
                    used[k] = true;
                    sep(&mut out);
                    out.push_str(params[k].0);
                    out.push('=');
                    out.push_str(&encode(params[k].1));
                }
            }
            None => {
                sep(&mut out);
                out.push_str(raw);
            }
        }
    }
    for (k, (name, value)) in params.iter().enumerate() {
        if !used[k] {
            sep(&mut out);
            out.push_str(name);
            out.push('=');
            out.push_str(&encode(value));
        }
    }
    out.push_str(fragment);
    out
}

/// The value of the parameter `name` (any case) of `url`, decoded.
pub fn param(url: &str, name: &str) -> Option<String> {
    let (_, own, _) = split(url);
    own.iter()
        .find(|(n, _, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v, _)| decode(v))
}

/// `url` without the parameters named (any case): a cache key or a log
/// line without a connection's secrets.
pub fn without_params(url: &str, names: &[&str]) -> String {
    let (base, own, fragment) = split(url);
    let mut out = String::from(base);
    let mut first = true;
    for (name, _, raw) in own {
        if names.iter().any(|n| n.eq_ignore_ascii_case(name)) {
            continue;
        }
        out.push(if first { '?' } else { '&' });
        first = false;
        out.push_str(raw);
    }
    out.push_str(fragment);
    out
}

/// `url` without its query and fragment: a service's base address.
pub fn base(url: &str) -> &str {
    split(url).0
}

/// `path` added to `url`'s path (one `/` between them), its query kept:
/// an OGC API's or an ArcGIS service's resource.
pub fn join(url: &str, path: &str) -> String {
    let (base, own, fragment) = split(url);
    let mut out = String::from(base.trim_end_matches('/'));
    out.push('/');
    out.push_str(path.trim_start_matches('/'));
    let mut first = true;
    for (_, _, raw) in own {
        out.push(if first { '?' } else { '&' });
        first = false;
        out.push_str(raw);
    }
    out.push_str(fragment);
    out
}

/// `href` resolved against the address `against` was read from: an absolute
/// address as it is, `//host/…` with `against`'s scheme, `/…` on its
/// origin, anything else beside its last `/`.
pub fn resolve(against: &str, href: &str) -> String {
    let href = href.trim();
    if href.contains("://") {
        return href.to_owned();
    }
    let scheme_end = against.find("://").map_or(0, |i| i + 3);
    if let Some(rest) = href.strip_prefix("//") {
        return format!("{}{rest}", &against[..scheme_end]);
    }
    let host_end = against[scheme_end..]
        .find('/')
        .map_or(against.len(), |i| scheme_end + i);
    if href.starts_with('/') {
        return format!("{}{href}", &against[..host_end]);
    }
    let path = base(against);
    let dir = match path.rfind('/') {
        Some(i) if i >= host_end => &path[..=i],
        _ => {
            return format!("{}/{href}", &against[..host_end]);
        }
    };
    format!("{dir}{href}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_encoded_and_decoded() {
        assert_eq!(encode("a b&c=d/ş"), "a%20b%26c%3Dd/%C5%9F");
        assert_eq!(encode("1,2:3"), "1,2:3");
        assert_eq!(decode("a%20b%26c%3Dd%2F%C5%9F"), "a b&c=d/ş");
        assert_eq!(decode("100%"), "100%");
    }

    #[test]
    fn parameters_go_after_the_addresss_own_and_replace_its_names() {
        assert_eq!(
            with_params("https://x/wms", &[("SERVICE", "WMS"), ("LAYERS", "a,b")]),
            "https://x/wms?SERVICE=WMS&LAYERS=a,b"
        );
        assert_eq!(
            with_params(
                "https://x/wms?map=/m/a.map&service=wms",
                &[("SERVICE", "WMS")]
            ),
            "https://x/wms?map=/m/a.map&SERVICE=WMS"
        );
        assert_eq!(
            with_params("https://x/wms?", &[("A", "1")]),
            "https://x/wms?A=1"
        );
        assert_eq!(
            param("https://x/?Key=a%20b&b=2", "key").as_deref(),
            Some("a b")
        );
        assert_eq!(
            without_params("https://x/t?apikey=S&x=1#f", &["APIKEY"]),
            "https://x/t?x=1#f"
        );
    }

    #[test]
    fn paths_join_and_references_resolve() {
        assert_eq!(
            join("https://x/ogc/?f=json", "collections"),
            "https://x/ogc/collections?f=json"
        );
        assert_eq!(
            resolve("https://x/a/b/c.json", "d.json"),
            "https://x/a/b/d.json"
        );
        assert_eq!(
            resolve("https://x/a/b/c.json", "/d.json"),
            "https://x/d.json"
        );
        assert_eq!(resolve("https://x/a", "//y/z"), "https://y/z");
        assert_eq!(resolve("https://x", "d"), "https://x/d");
        assert_eq!(resolve("https://x/a", "https://z/q"), "https://z/q");
    }
}
