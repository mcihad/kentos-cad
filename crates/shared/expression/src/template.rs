//! Templates (docs/adr/0225 §6): a layer's action targets and its
//! information card's title and text. A template is plain text in which each
//! `{…}` is an expression of the language, evaluated on an object and put in
//! as text (an empty value as nothing); `{{` and `}}` are plain braces. A
//! brace inside a quoted text (`'…'`, `"…"`) or a bracketed field name
//! (`[…]`) of an expression is the expression's. Alone in its braces,
//! `{$enlem}` and `{$boylam}` are the object's place in WGS 84 (the caller
//! gives it), not the language's.
//!
//! An action's target is an address when its text before the first
//! expression starts with `http://`, `https://` or `mailto:` (the scheme in
//! any case): every value put in it is percent-encoded (RFC 3986: all but
//! letters, digits and `-._~`, each byte of its UTF-8). Otherwise the values
//! go in as they are (a file's path, or an address an attribute holds
//! whole). What a filled target is (`target_kind`): an address, a file, or
//! neither (another scheme: `javascript:`, `data:`, `file:` … are refused).
//!
//! Positions are 1-based UTF-16 code units, as the language's errors are
//! (the dialogs count them so). The web reaches this through
//! `kentos-geometry-wasm` (`templateParse`, `templateFill`); both platforms
//! pass `fixtures/expression/v2/template.json`, which
//! `scripts/fixtures/template_cases.py` writes from the rules alone.

use kentos_geometry_core::display;

/// One piece of a template, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum Piece {
    /// Plain text, its doubled braces already single.
    Text(String),
    /// An expression's source and where it starts in the template.
    Expr { source: String, at: usize },
    /// `{$enlem}`: the object's latitude in WGS 84.
    Latitude,
    /// `{$boylam}`: the object's longitude in WGS 84.
    Longitude,
}

/// A template split into its pieces.
#[derive(Clone, Debug, PartialEq)]
pub struct Template {
    pub pieces: Vec<Piece>,
    /// Whether the values are percent-encoded (an address's template).
    pub encoded: bool,
}

/// Why a template does not split, and where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemplateError {
    /// 1-based UTF-16 position of the brace at fault.
    pub at: usize,
    pub message: String,
}

impl TemplateError {
    /// As a dialog says it.
    pub fn text(&self) -> String {
        format!("{}. karakterde: {}", self.at, self.message)
    }
}

/// What a filled action target is (docs/adr/0225 §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetKind {
    /// An `http`, `https` or `mailto` address.
    Address,
    /// A file's path (absolute, or relative to the drawing's folder).
    File,
    /// Another scheme, or nothing: not opened.
    Refused,
}

const OPEN: u16 = b'{' as u16;
const CLOSE: u16 = b'}' as u16;
const SINGLE: u16 = b'\'' as u16;
const DOUBLE: u16 = b'"' as u16;
const BRACKET: u16 = b'[' as u16;
const BRACKET_END: u16 = b']' as u16;

/// The schemes whose templates are addresses.
const ADDRESS_SCHEMES: [&str; 3] = ["http://", "https://", "mailto:"];

/// Whether `text` starts with one of `ADDRESS_SCHEMES`, in any case.
fn starts_address(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    ADDRESS_SCHEMES.iter().any(|s| lower.starts_with(s))
}

/// The unit of the closing brace of the expression whose first unit is
/// `start`; `Err` when an opening brace comes first or none closes it.
fn expression_end(u: &[u16], start: usize) -> Result<usize, ()> {
    let mut i = start;
    while i < u.len() {
        let c = u[i];
        if c == SINGLE || c == DOUBLE {
            // A quoted text: a doubled quote is one quote.
            let mut j = i + 1;
            loop {
                if j >= u.len() {
                    return Err(());
                }
                if u[j] == c {
                    if u.get(j + 1) == Some(&c) {
                        j += 2;
                        continue;
                    }
                    break;
                }
                j += 1;
            }
            i = j + 1;
            continue;
        }
        if c == BRACKET {
            match u[i..].iter().position(|&x| x == BRACKET_END) {
                Some(k) => {
                    i += k + 1;
                    continue;
                }
                None => return Err(()),
            }
        }
        if c == CLOSE {
            return Ok(i);
        }
        if c == OPEN {
            return Err(());
        }
        i += 1;
    }
    Err(())
}

/// White space as the language trims a name (JavaScript's `trim`).
fn trimmed(s: &str) -> &str {
    crate::js::text::trim(s)
}

/// Splits `text` into its pieces.
pub fn parse(text: &str) -> Result<Template, TemplateError> {
    let u: Vec<u16> = text.encode_utf16().collect();
    let mut pieces = Vec::new();
    let mut plain: Vec<u16> = Vec::new();
    let mut i = 0;
    while i < u.len() {
        let c = u[i];
        if c == OPEN && u.get(i + 1) == Some(&OPEN) {
            plain.push(OPEN);
            i += 2;
            continue;
        }
        if c == CLOSE && u.get(i + 1) == Some(&CLOSE) {
            plain.push(CLOSE);
            i += 2;
            continue;
        }
        if c == CLOSE {
            return Err(TemplateError {
                at: i + 1,
                message: "“}” tek başına; düz ayraç için }} yazın.".into(),
            });
        }
        if c == OPEN {
            // An opening brace before the closing one, or none: this one does not close.
            let end = expression_end(&u, i + 1).map_err(|_| TemplateError {
                at: i + 1,
                message: "“{” kapanmamış: ifadeyi “}” ile kapatın (düz ayraç için {{ yazın)."
                    .into(),
            })?;
            let source = String::from_utf16_lossy(&u[i + 1..end]);
            let inner = trimmed(&source);
            if inner.is_empty() {
                return Err(TemplateError {
                    at: i + 1,
                    message: "“{}” boş: araya bir ifade yazın.".into(),
                });
            }
            if !plain.is_empty() {
                pieces.push(Piece::Text(String::from_utf16_lossy(&plain)));
                plain.clear();
            }
            pieces.push(match inner {
                "$enlem" => Piece::Latitude,
                "$boylam" => Piece::Longitude,
                _ => Piece::Expr { source, at: i + 2 },
            });
            i = end + 1;
            continue;
        }
        plain.push(c);
        i += 1;
    }
    if !plain.is_empty() {
        pieces.push(Piece::Text(String::from_utf16_lossy(&plain)));
    }
    let lead = match pieces.first() {
        Some(Piece::Text(t)) => t.as_str(),
        _ => "",
    };
    let encoded = starts_address(trimmed(lead));
    Ok(Template { pieces, encoded })
}

impl Template {
    /// Its expressions' sources and places, in order.
    pub fn expressions(&self) -> impl Iterator<Item = (&str, usize)> {
        self.pieces.iter().filter_map(|p| match p {
            Piece::Expr { source, at } => Some((source.as_str(), *at)),
            _ => None,
        })
    }

    /// Whether it has `{$enlem}` or `{$boylam}`.
    pub fn needs_place(&self) -> bool {
        self.pieces
            .iter()
            .any(|p| matches!(p, Piece::Latitude | Piece::Longitude))
    }

    /// Whether it is only plain text.
    pub fn is_plain(&self) -> bool {
        self.pieces.iter().all(|p| matches!(p, Piece::Text(_)))
    }

    /// The text with its expressions' values (`values`, in their order; a
    /// missing one is empty) and the object's place (`place`: latitude and
    /// longitude in degrees; none: empty) put in, percent-encoded when the
    /// template is an address's.
    pub fn fill(&self, values: &[Option<String>], place: Option<(f64, f64)>) -> String {
        let mut out = String::new();
        let mut k = 0;
        let put = |out: &mut String, v: &str| {
            if self.encoded {
                percent_encode_into(out, v);
            } else {
                out.push_str(v);
            }
        };
        for p in &self.pieces {
            match p {
                Piece::Text(t) => out.push_str(t),
                Piece::Expr { .. } => {
                    if let Some(Some(v)) = values.get(k) {
                        put(&mut out, v);
                    }
                    k += 1;
                }
                Piece::Latitude => {
                    if let Some((lat, _)) = place {
                        put(&mut out, &degrees(lat));
                    }
                }
                Piece::Longitude => {
                    if let Some((_, lon)) = place {
                        put(&mut out, &degrees(lon));
                    }
                }
            }
        }
        out
    }
}

/// A coordinate in decimal degrees, eight decimals (about a millimetre) by
/// the display rule (docs/adr/0149).
pub fn degrees(v: f64) -> String {
    display::fixed(v, 8)
}

/// `s` percent-encoded (RFC 3986): every byte of its UTF-8 but the
/// unreserved letters, digits and `-._~` as `%XX` (upper case).
pub fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    percent_encode_into(&mut out, s);
    out
}

fn percent_encode_into(out: &mut String, s: &str) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(b));
        } else {
            out.push('%');
            out.push(char::from(HEX[usize::from(b >> 4)]));
            out.push(char::from(HEX[usize::from(b & 15)]));
        }
    }
}

/// What a filled action target is: an `http`, `https` or `mailto` address
/// (the scheme in any case, something after it, no white space); a file's
/// path (no scheme, or a drive letter: `C:\…`, `C:/…`); else refused (an
/// empty target, another scheme).
pub fn target_kind(target: &str) -> TargetKind {
    let t = trimmed(target);
    if t.is_empty() {
        return TargetKind::Refused;
    }
    let lower = t.to_ascii_lowercase();
    if let Some(scheme) = ADDRESS_SCHEMES.iter().find(|s| lower.starts_with(*s)) {
        let rest = &t[scheme.len()..];
        return if rest.is_empty() || t.chars().any(char::is_whitespace) {
            TargetKind::Refused
        } else {
            TargetKind::Address
        };
    }
    // A scheme: a letter, then letters, digits, `+`, `-` or `.`, then `:`.
    let b = t.as_bytes();
    if let Some(colon) = t.find(':')
        && colon > 0
        && b[0].is_ascii_alphabetic()
        && b[..colon]
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'+' | b'-' | b'.'))
    {
        // A drive letter is a path's.
        let drive = colon == 1 && matches!(b.get(2), Some(b'\\' | b'/'));
        if !drive {
            return TargetKind::Refused;
        }
    }
    TargetKind::File
}

#[cfg(test)]
mod tests {
    use super::*;

    /// fixtures/expression/v2/template.json, which
    /// scripts/fixtures/template_cases.py writes from the rules alone.
    #[test]
    fn the_shared_cases_pass() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../fixtures/expression/v2/template.json"
        );
        let text = std::fs::read_to_string(path).expect("the fixture");
        let cases: serde_json::Value = serde_json::from_str(&text).expect("JSON");
        let piece_json = |p: &Piece| match p {
            Piece::Text(t) => serde_json::json!({ "kind": "text", "text": t }),
            Piece::Expr { source, at } => {
                serde_json::json!({ "kind": "expr", "text": source, "at": at })
            }
            Piece::Latitude => serde_json::json!({ "kind": "lat" }),
            Piece::Longitude => serde_json::json!({ "kind": "lon" }),
        };
        for c in cases["parse"].as_array().expect("parse") {
            let t = c["template"].as_str().expect("template");
            match parse(t) {
                Ok(got) => {
                    assert!(c.get("error").is_none(), "{t:?} splits: {got:?}");
                    let pieces: Vec<_> = got.pieces.iter().map(piece_json).collect();
                    assert_eq!(serde_json::Value::from(pieces), c["pieces"], "{t:?}");
                    assert_eq!(
                        got.encoded,
                        c["encoded"].as_bool().expect("encoded"),
                        "{t:?}"
                    );
                }
                Err(e) => {
                    assert_eq!(e.at as u64, c["error"]["at"].as_u64().expect("at"), "{t:?}");
                    assert_eq!(
                        e.message,
                        c["error"]["message"].as_str().expect("message"),
                        "{t:?}"
                    );
                }
            }
        }
        for c in cases["fill"].as_array().expect("fill") {
            let t = c["template"].as_str().expect("template");
            let values: Vec<Option<String>> = c["values"]
                .as_array()
                .expect("values")
                .iter()
                .map(|v| v.as_str().map(str::to_owned))
                .collect();
            let place = c.get("place").map(|p| {
                (
                    p[0].as_f64().expect("latitude"),
                    p[1].as_f64().expect("longitude"),
                )
            });
            let got = parse(t).expect("splits").fill(&values, place);
            assert_eq!(got, c["text"].as_str().expect("text"), "{t:?}");
        }
        for c in cases["target"].as_array().expect("target") {
            let t = c["target"].as_str().expect("target");
            let kind = match target_kind(t) {
                TargetKind::Address => "address",
                TargetKind::File => "file",
                TargetKind::Refused => "refused",
            };
            assert_eq!(kind, c["kind"].as_str().expect("kind"), "{t:?}");
        }
    }

    fn pieces(t: &str) -> Vec<Piece> {
        parse(t).expect("splits").pieces
    }

    #[test]
    fn braces_split_text_and_expressions() {
        assert_eq!(
            pieces("Parsel {Ada}/{Parsel}"),
            vec![
                Piece::Text("Parsel ".into()),
                Piece::Expr {
                    source: "Ada".into(),
                    at: 9
                },
                Piece::Text("/".into()),
                Piece::Expr {
                    source: "Parsel".into(),
                    at: 15
                },
            ]
        );
        assert_eq!(pieces("{{düz}}"), vec![Piece::Text("{düz}".into())]);
        assert_eq!(
            pieces("{'}' || [a}b]}"),
            vec![Piece::Expr {
                source: "'}' || [a}b]".into(),
                at: 2
            }]
        );
        assert_eq!(pieces(" { $enlem } "), {
            vec![
                Piece::Text(" ".into()),
                Piece::Latitude,
                Piece::Text(" ".into()),
            ]
        });
    }

    #[test]
    fn broken_braces_say_where() {
        assert_eq!(parse("a {Ada").unwrap_err().at, 3);
        assert_eq!(parse("a } b").unwrap_err().at, 3);
        assert_eq!(parse("x{ }").unwrap_err().at, 2);
        assert_eq!(parse("{a {b}").unwrap_err().at, 1);
        assert_eq!(parse("{'a}").unwrap_err().at, 1);
    }

    #[test]
    fn an_address_encodes_its_values() {
        let t = parse("https://a.org/?q={Ad}&n={$enlem}").expect("splits");
        assert!(t.encoded);
        assert_eq!(
            t.fill(&[Some("Ayşe & Ali".into())], Some((41.0, 29.000000005))),
            "https://a.org/?q=Ay%C5%9Fe%20%26%20Ali&n=41.00000000"
        );
        let file = parse("belgeler/{Ada}.pdf").expect("splits");
        assert!(!file.encoded);
        assert_eq!(file.fill(&[Some("1 2".into())], None), "belgeler/1 2.pdf");
        assert_eq!(file.fill(&[None], None), "belgeler/.pdf");
    }

    #[test]
    fn targets_are_addresses_files_or_refused() {
        assert_eq!(target_kind("https://a.org"), TargetKind::Address);
        assert_eq!(target_kind("MAILTO:a@b.org"), TargetKind::Address);
        assert_eq!(target_kind("https://"), TargetKind::Refused);
        assert_eq!(target_kind("javascript:alert(1)"), TargetKind::Refused);
        assert_eq!(target_kind("file:///etc/passwd"), TargetKind::Refused);
        assert_eq!(target_kind("C:\\belgeler\\a.pdf"), TargetKind::File);
        assert_eq!(target_kind("belgeler/a.pdf"), TargetKind::File);
        assert_eq!(target_kind("  "), TargetKind::Refused);
    }
}
