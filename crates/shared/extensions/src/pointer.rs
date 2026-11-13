//! The JSON pointer (RFC 6901) of the place a serde error points at: `/entrypoints/processing/0/handler`, so a person or
//! an agent mends the field, not "line 41, column 7". serde_json gives the line and column only; the text up to there
//! is walked once, keeping the open objects' current keys and the open arrays' indices.

/// An open container while walking.
enum Frame {
    Object { key: Option<String>, want_key: bool },
    Array { index: usize, started: bool },
}

/// Escapes a JSON pointer's segment.
pub(crate) fn segment(text: &str) -> String {
    text.replace('~', "~0").replace('/', "~1")
}

/// The pointer of the place at byte `end`: the keys and indices open there. With `whole_object`, the innermost open
/// object itself, not its current key (a missing field is the object's).
pub(crate) fn pointer(text: &str, end: usize, whole_object: bool) -> String {
    let b = text.as_bytes();
    let end = end.min(b.len());
    let mut stack: Vec<Frame> = Vec::new();
    let value_starts = |stack: &mut Vec<Frame>| {
        if let Some(Frame::Array { started, .. }) = stack.last_mut() {
            *started = true;
        }
    };
    let mut i = 0;
    while i < end {
        match b[i] {
            b'{' => {
                value_starts(&mut stack);
                stack.push(Frame::Object {
                    key: None,
                    want_key: true,
                });
                i += 1;
            }
            b'[' => {
                value_starts(&mut stack);
                stack.push(Frame::Array {
                    index: 0,
                    started: false,
                });
                i += 1;
            }
            b'}' | b']' => {
                stack.pop();
                i += 1;
            }
            b',' => {
                match stack.last_mut() {
                    Some(Frame::Object { want_key, .. }) => *want_key = true,
                    Some(Frame::Array { index, .. }) => *index += 1,
                    None => {}
                }
                i += 1;
            }
            b':' => {
                if let Some(Frame::Object { want_key, .. }) = stack.last_mut() {
                    *want_key = false;
                }
                i += 1;
            }
            b'"' => {
                let start = i + 1;
                let mut j = start;
                while j < b.len() && b[j] != b'"' {
                    j += if b[j] == b'\\' { 2 } else { 1 };
                }
                let j = j.min(b.len());
                match stack.last_mut() {
                    Some(Frame::Object {
                        key,
                        want_key: true,
                    }) => {
                        let raw = text.get(start..j).unwrap_or("");
                        *key = Some(
                            serde_json::from_str::<String>(&format!("\"{raw}\""))
                                .unwrap_or_else(|_| raw.to_owned()),
                        );
                    }
                    _ => value_starts(&mut stack),
                }
                i = j + 1;
            }
            c if c.is_ascii_whitespace() => i += 1,
            _ => {
                value_starts(&mut stack);
                while i < end
                    && !matches!(b[i], b',' | b'}' | b']' | b':')
                    && !b[i].is_ascii_whitespace()
                {
                    i += 1;
                }
            }
        }
    }
    if whole_object && let Some(Frame::Object { key, .. }) = stack.last_mut() {
        *key = None;
    }
    let mut out = String::new();
    for frame in &stack {
        match frame {
            Frame::Object { key: Some(key), .. } => {
                out.push('/');
                out.push_str(&segment(key));
            }
            Frame::Object { key: None, .. } => break,
            Frame::Array {
                index,
                started: true,
            } => {
                out.push('/');
                out.push_str(&index.to_string());
            }
            Frame::Array { started: false, .. } => break,
        }
    }
    out
}

/// The pointer of the place serde_json's error is at: the field for a wrong value or an unknown field, the object
/// for a missing one.
pub(crate) fn at_error(text: &str, e: &serde_json::Error) -> String {
    let line_start: usize = text
        .split_inclusive('\n')
        .take(e.line().saturating_sub(1))
        .map(str::len)
        .sum();
    let end = (line_start + e.column()).min(text.len());
    if e.to_string().starts_with("missing field") {
        // Reported after the object's `}`: the object is the place.
        pointer(text, end.saturating_sub(1), true)
    } else {
        pointer(text, end, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields, rename_all = "camelCase")]
    #[allow(dead_code)]
    struct Outer {
        name: String,
        #[serde(default)]
        items: Vec<Inner>,
        #[serde(default)]
        nested: Option<Box<Outer>>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields, rename_all = "camelCase")]
    #[allow(dead_code)]
    struct Inner {
        id: String,
        #[serde(default)]
        size: Option<u32>,
        #[serde(default)]
        kind: Option<Kind>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    enum Kind {
        Large,
    }

    fn at(text: &str) -> String {
        let e = serde_json::from_str::<Outer>(text).unwrap_err();
        at_error(text, &e)
    }

    #[test]
    fn serde_errors_point_at_the_field() {
        assert_eq!(at(r#"{"name": "a", "bogus": 1}"#), "/bogus");
        assert_eq!(
            at(
                "{\n  \"name\": \"a\",\n  \"items\": [\n    {\"id\": \"x\"},\n    {\"id\": \"y\", \"size\": \"big\"}\n  ]\n}"
            ),
            "/items/1/size"
        );
        assert_eq!(
            at(r#"{"name": "a", "items": [{"id": "x", "kind": "huge"}]}"#),
            "/items/0/kind"
        );
        assert_eq!(
            at(r#"{"name": "a", "items": [{"id": "x"}, {"size": 2}]}"#),
            "/items/1"
        );
        assert_eq!(at(r#"{"items": []}"#), "");
        assert_eq!(
            at(r#"{"name": "a", "nested": {"name": "b", "items": [{"id": 5}]}}"#),
            "/nested/items/0/id"
        );
        assert_eq!(
            at(r#"{"name": "a", "nested": {"name": "b", "items": [{"i\"d": 5}]}}"#),
            "/nested/items/0/i\"d"
        );
        assert_eq!(
            at(r#"{"name": "a", "items": [{"id": "x", "a/b": 1}]}"#),
            "/items/0/a~1b"
        );
    }
}
