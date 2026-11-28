//! A stored file as a response (docs/adr/9000 §13): the same for a dossier's file, a project's attachment and a
//! picture, so the desktop, a phone and an agent resume and cache every download the same way.
//!
//! - `ETag` is the file's SHA-256 (its bytes never change under its id): `If-None-Match` with it answers 304.
//! - `Range: bytes=a-b`, `bytes=a-` and `bytes=-n` answer 206 with that part, `Content-Range` saying where it lies;
//!   a list of ranges is answered whole, as is an `If-Range` naming another tag; a range past the end answers 416.
//! - A document or a picture that runs nothing (`viewable`) is shown in place when asked (`?inline=1`); anything else
//!   is always a download. Every answer says `X-Content-Type-Options: nosniff` and a `Content-Security-Policy` that
//!   runs nothing, so an uploaded page or picture never acts as the application.

use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use kentos_application::error::AppError;

use super::files::{FileBody, disposition};

/// A file to answer with.
pub struct Stored<'a> {
    pub file: tokio::fs::File,
    pub size: u64,
    pub sha256: &'a str,
    pub name: &'a str,
    pub media_type: &'a str,
    /// Shown in place rather than saved: asked for, and the kind runs nothing.
    pub inline: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum Wanted {
    Whole,
    Part(u64, u64),
    Unsatisfiable,
}

/// The part a request asks for of a file `size` bytes long whose tag is `tag`.
fn wanted(headers: &HeaderMap, size: u64, tag: &str) -> Wanted {
    let Some(range) = headers.get(header::RANGE).and_then(|v| v.to_str().ok()) else {
        return Wanted::Whole;
    };
    if let Some(if_range) = headers.get(header::IF_RANGE).and_then(|v| v.to_str().ok())
        && if_range.trim() != tag
    {
        return Wanted::Whole;
    }
    let Some(spec) = range.trim().strip_prefix("bytes=") else {
        return Wanted::Whole;
    };
    let Some((a, b)) = spec.split_once('-').filter(|_| !spec.contains(',')) else {
        return Wanted::Whole;
    };
    let number = |t: &str| t.trim().parse::<u64>().ok();
    let (start, end) = match (a.trim().is_empty(), b.trim().is_empty()) {
        (true, false) => match number(b) {
            Some(n) if n > 0 && size > 0 => (size.saturating_sub(n), size - 1),
            Some(_) => return Wanted::Unsatisfiable,
            None => return Wanted::Whole,
        },
        (false, true) => match number(a) {
            Some(s) => (s, size.saturating_sub(1)),
            None => return Wanted::Whole,
        },
        (false, false) => match (number(a), number(b)) {
            (Some(s), Some(e)) if s <= e => (s, e.min(size.saturating_sub(1))),
            (Some(_), Some(_)) => return Wanted::Unsatisfiable,
            _ => return Wanted::Whole,
        },
        (true, true) => return Wanted::Whole,
    };
    if size == 0 || start >= size {
        Wanted::Unsatisfiable
    } else {
        Wanted::Part(start, end)
    }
}

/// The headers every stored file's answer carries.
fn common(h: &mut HeaderMap, tag: &str) {
    if let Ok(v) = HeaderValue::from_str(tag) {
        h.insert(header::ETAG, v);
    }
    h.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    h.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    h.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'none'; img-src 'self' data:; style-src 'unsafe-inline'; sandbox",
        ),
    );
    // Private: kept by the caller's own client, which asks again with the tag (a 304 is a few bytes).
    h.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-cache"),
    );
}

/// The answer for `stored` to a request with `headers`.
pub async fn respond(headers: &HeaderMap, stored: Stored<'_>) -> Result<Response, AppError> {
    let tag = format!("\"{}\"", stored.sha256);
    if kentos_application::preview::already_has(
        headers
            .get(header::IF_NONE_MATCH)
            .and_then(|v| v.to_str().ok()),
        &tag,
    ) {
        let mut response = StatusCode::NOT_MODIFIED.into_response();
        common(response.headers_mut(), &tag);
        return Ok(response);
    }
    let size = stored.size;
    let (status, start, len) = match wanted(headers, size, &tag) {
        Wanted::Whole => (StatusCode::OK, 0, size),
        Wanted::Part(s, e) => (StatusCode::PARTIAL_CONTENT, s, e - s + 1),
        Wanted::Unsatisfiable => {
            let mut response = StatusCode::RANGE_NOT_SATISFIABLE.into_response();
            if let Ok(v) = HeaderValue::from_str(&format!("bytes */{size}")) {
                response.headers_mut().insert(header::CONTENT_RANGE, v);
            }
            common(response.headers_mut(), &tag);
            return Ok(response);
        }
    };
    let mut file = stored.file;
    if start > 0 {
        use tokio::io::AsyncSeekExt as _;
        file.seek(std::io::SeekFrom::Start(start))
            .await
            .map_err(AppError::Storage)?;
    }
    let mut response = Body::new(FileBody::part(file, len)).into_response();
    *response.status_mut() = status;
    let h = response.headers_mut();
    common(h, &tag);
    h.insert(header::CONTENT_LENGTH, HeaderValue::from(len));
    if status == StatusCode::PARTIAL_CONTENT
        && let Ok(v) = HeaderValue::from_str(&format!("bytes {start}-{}/{size}", start + len - 1))
    {
        h.insert(header::CONTENT_RANGE, v);
    }
    let shown =
        stored.inline && kentos_application::attachments::VIEWABLE.contains(&stored.media_type);
    let media = if shown {
        stored.media_type
    } else {
        "application/octet-stream"
    };
    if let Ok(v) = HeaderValue::from_str(media) {
        h.insert(header::CONTENT_TYPE, v);
    }
    if let Some(v) = disposition(if shown { "inline" } else { "attachment" }, stored.name) {
        h.insert(header::CONTENT_DISPOSITION, v);
    }
    Ok(response)
}

/// A drawn picture (an SVG the server made, which runs nothing) with its tag.
pub fn picture(tag: &str, svg: Option<String>, drawn: usize, skipped: usize) -> Response {
    let mut response = match svg {
        Some(svg) => {
            let mut r = Body::from(svg).into_response();
            r.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("image/svg+xml; charset=utf-8"),
            );
            r.headers_mut()
                .insert("x-kentos-drawn", HeaderValue::from(drawn));
            r.headers_mut()
                .insert("x-kentos-skipped", HeaderValue::from(skipped));
            r
        }
        None => StatusCode::NOT_MODIFIED.into_response(),
    };
    let h = response.headers_mut();
    if let Ok(v) = HeaderValue::from_str(tag) {
        h.insert(header::ETAG, v);
    }
    h.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    h.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'; style-src 'unsafe-inline'; sandbox"),
    );
    h.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-cache"),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ranged(range: &str, if_range: Option<&str>) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(header::RANGE, HeaderValue::from_str(range).unwrap());
        if let Some(t) = if_range {
            h.insert(header::IF_RANGE, HeaderValue::from_str(t).unwrap());
        }
        h
    }

    #[test]
    fn a_range_is_read_as_one_part_or_the_whole() {
        let tag = "\"ab\"";
        assert_eq!(wanted(&HeaderMap::new(), 100, tag), Wanted::Whole);
        assert_eq!(
            wanted(&ranged("bytes=0-9", None), 100, tag),
            Wanted::Part(0, 9)
        );
        assert_eq!(
            wanted(&ranged("bytes=90-", None), 100, tag),
            Wanted::Part(90, 99)
        );
        assert_eq!(
            wanted(&ranged("bytes=-10", None), 100, tag),
            Wanted::Part(90, 99)
        );
        assert_eq!(
            wanted(&ranged("bytes=95-500", None), 100, tag),
            Wanted::Part(95, 99)
        );
        assert_eq!(
            wanted(&ranged("bytes=100-", None), 100, tag),
            Wanted::Unsatisfiable
        );
        assert_eq!(
            wanted(&ranged("bytes=9-3", None), 100, tag),
            Wanted::Unsatisfiable
        );
        assert_eq!(
            wanted(&ranged("bytes=0-1,5-6", None), 100, tag),
            Wanted::Whole
        );
        assert_eq!(wanted(&ranged("items=0-1", None), 100, tag), Wanted::Whole);
        assert_eq!(
            wanted(&ranged("bytes=0-9", Some("\"ab\"")), 100, tag),
            Wanted::Part(0, 9)
        );
        assert_eq!(
            wanted(&ranged("bytes=0-9", Some("\"cd\"")), 100, tag),
            Wanted::Whole
        );
    }
}
