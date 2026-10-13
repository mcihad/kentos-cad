//! A project's sheet book in its snapshot (docs/adr/0251): the book of
//! sheets, master pages and variables the sheet core keeps (`kentos.sheet/1`,
//! crates/shared/sheet) and the bytes of its pictures. The file format and
//! the documents carry the book as it is, an opaque value like a project
//! style's (docs/specs/kcad-v2.md §6.7); the sheet core reads and checks it
//! (`kentos_sheet::validate::read_book`) as it reads a `.kpafta`'s.
//!
//! A picture is named by the SHA-256 of its bytes, as the book names it; the
//! snapshot's JSON writes the bytes as base64, the `.kcad` as a byte string.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::encoding::sha256_hex;

/// The sheet book of a project and its pictures' bytes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectSheets {
    /// The sheet core's book (`kentos.sheet/1`): an object, opaque here.
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>"))]
    pub book: serde_json::Value,
    /// The bytes of the book's pictures; none when it has none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<SheetPicture>>"))]
    pub pictures: Vec<SheetPicture>,
}

/// A picture's bytes under the SHA-256 the book names it by.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SheetPicture {
    /// SHA-256 of `bytes`, 64 lowercase hexadecimal digits.
    pub sha256: String,
    /// The file's bytes; base64 (RFC 4648, with padding) in JSON.
    #[serde(with = "crate::encoding::base64_text")]
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub bytes: Vec<u8>,
}

/// Whether `text` is a SHA-256 as the book writes it: 64 lowercase hexadecimal digits.
pub fn is_sha256_text(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Where `sheets` breaks the file format's rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SheetsFault {
    /// The book is not an object.
    Book,
    /// The picture at this index: its digest is not its bytes', or it is given twice.
    Picture(usize),
}

/// Why `sheets` cannot be kept by the file format's rules (docs/specs/kcad-v2.md
/// §6.12), and where: the book not an object, a picture whose digest is not its
/// bytes' or that is given twice. The book's own rules are the sheet core's.
pub fn sheets_problem(sheets: &ProjectSheets) -> Option<(SheetsFault, &'static str)> {
    if !sheets.book.is_object() {
        return Some((SheetsFault::Book, "pafta kitabı bir nesne (harita) olmalı"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for (i, p) in sheets.pictures.iter().enumerate() {
        let at = SheetsFault::Picture(i);
        if !is_sha256_text(&p.sha256) {
            return Some((at, "SHA-256, 64 küçük onaltılık hane olmalı"));
        }
        if sha256_hex(&p.bytes) != p.sha256 {
            return Some((at, "resmin baytları SHA-256'sına uymuyor"));
        }
        if !seen.insert(p.sha256.as_str()) {
            return Some((at, "aynı resim iki kez verilmiş"));
        }
    }
    None
}

impl std::fmt::Display for SheetsFault {
    /// The place in words: “Pafta kitabı” or “Pafta kitabı › 2. resim”.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SheetsFault::Book => f.write_str("Pafta kitabı"),
            SheetsFault::Picture(i) => write!(f, "Pafta kitabı › {}. resim", i + 1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn picture(bytes: &[u8]) -> SheetPicture {
        SheetPicture {
            sha256: sha256_hex(bytes),
            bytes: bytes.to_vec(),
        }
    }

    #[test]
    fn pictures_go_as_base64_and_are_held_to_their_digests() {
        let sheets = ProjectSheets {
            book: json!({ "schema": "kentos.sheet/1" }),
            pictures: vec![picture(b"pafta")],
        };
        let text = serde_json::to_value(&sheets).expect("writes");
        assert_eq!(text["pictures"][0]["bytes"], "cGFmdGE=");
        let back: ProjectSheets = serde_json::from_value(text).expect("reads");
        assert_eq!(back, sheets);
        assert_eq!(sheets_problem(&sheets), None);

        let mut twice = sheets.clone();
        twice.pictures.push(picture(b"pafta"));
        assert_eq!(
            sheets_problem(&twice).map(|(at, _)| at),
            Some(SheetsFault::Picture(1))
        );
        let mut wrong = sheets.clone();
        wrong.pictures[0].bytes = b"baska".to_vec();
        assert!(sheets_problem(&wrong).is_some());
        let not_a_book = ProjectSheets {
            book: json!([1]),
            pictures: Vec::new(),
        };
        assert_eq!(
            sheets_problem(&not_a_book).map(|(at, _)| at),
            Some(SheetsFault::Book)
        );
        let none = serde_json::to_value(&not_a_book).expect("writes");
        assert!(none.get("pictures").is_none());
    }
}
