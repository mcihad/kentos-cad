//! XML documents for the formats that are XML (Trimble JobXML; GPX next):
//! roxmltree's parser, guarded. The parser recurses once per nested
//! element, so a document nested deeper than [`DEPTH`] is refused before it
//! is parsed: a hostile or broken file must not overflow the stack, on the
//! desktop or in the browser's formats worker (docs/adr/0009). The readers'
//! independent references (Python's expat) count the same depth.

use roxmltree::Document;

/// The deepest element a document may hold (the root is level 1).
pub const DEPTH: usize = 256;

/// Why a text is not parsed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unparsed {
    /// Not well-formed XML.
    NotXml,
    /// An element deeper than [`DEPTH`].
    TooDeep,
}

/// The text's deepest element's level, as a scan of its tags sees it:
/// comments, CDATA, processing instructions and declarations (with an
/// internal subset) skipped, quoted attribute values skipped, an empty
/// element one level below its parent too.
fn depth(text: &str) -> usize {
    let b = text.as_bytes();
    // Just after the next `pat` from `from`, or the end.
    let after = |from: usize, pat: &[u8]| -> usize {
        b.get(from..)
            .and_then(|rest| rest.windows(pat.len()).position(|w| w == pat))
            .map_or(b.len(), |p| from + p + pat.len())
    };
    let (mut i, mut level, mut most) = (0usize, 0usize, 0usize);
    while i < b.len() {
        if b[i] != b'<' {
            i += 1;
            continue;
        }
        let rest = &b[i..];
        if rest.starts_with(b"<!--") {
            i = after(i + 4, b"-->");
        } else if rest.starts_with(b"<![CDATA[") {
            i = after(i + 9, b"]]>");
        } else if rest.starts_with(b"<?") {
            i = after(i + 2, b"?>");
        } else if rest.starts_with(b"<!") {
            let (mut j, mut bracket) = (i + 2, 0usize);
            while j < b.len() {
                match b[j] {
                    b'[' => bracket += 1,
                    b']' => bracket = bracket.saturating_sub(1),
                    b'>' if bracket == 0 => break,
                    _ => {}
                }
                j += 1;
            }
            i = j + 1;
        } else if rest.starts_with(b"</") {
            level = level.saturating_sub(1);
            i = after(i + 2, b">");
        } else {
            let (mut j, mut quote) = (i + 1, 0u8);
            while j < b.len() {
                let c = b[j];
                if quote != 0 {
                    if c == quote {
                        quote = 0;
                    }
                } else if c == b'"' || c == b'\'' {
                    quote = c;
                } else if c == b'>' {
                    break;
                }
                j += 1;
            }
            let empty = j > i + 1 && b[j - 1] == b'/';
            most = most.max(level + 1);
            if !empty {
                level += 1;
            }
            i = j + 1;
        }
    }
    most
}

/// The text's document, or why it is not one.
pub fn document(text: &str) -> Result<Document<'_>, Unparsed> {
    if depth(text) > DEPTH {
        return Err(Unparsed::TooDeep);
    }
    Document::parse(text).map_err(|_| Unparsed::NotXml)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_is_the_deepest_elements_level() {
        assert_eq!(depth("<a/>"), 1);
        assert_eq!(depth("<a><b/><c><d x='>'/></c></a>"), 3);
        assert_eq!(depth("<!-- <a><b> --><a><![CDATA[<b><c>]]></a>"), 1);
        assert_eq!(
            depth("<?xml version=\"1.0\"?><!DOCTYPE a [<!ENTITY e \"<x>\">]><a/>"),
            1
        );
        let deep = format!("{}{}", "<a>".repeat(300), "</a>".repeat(300));
        assert_eq!(document(&deep).err(), Some(Unparsed::TooDeep));
        let fine = format!("{}{}", "<a>".repeat(DEPTH), "</a>".repeat(DEPTH));
        assert!(document(&fine).is_ok());
        assert_eq!(document("<a>").err(), Some(Unparsed::NotXml));
    }
}
