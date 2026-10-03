//! The `.kpafta` file (design §9, §10): a book of sheets and the bytes of its
//! pictures, JSON — `{format: "kentos.sheet.file", version: 1, book, assets}`.
//! One codec for both platforms: the web reads and writes it through the
//! WASM binding (`encodeKpafta`, `decodeKpafta`), the desktop directly.
//!
//! Reading checks the format and its version, reads the book as
//! [`read_book`](crate::validate::read_book) does (checked and normalised)
//! and holds every picture to the book's metadata: bytes that are not
//! base64, a digest or size that does not match, a picture the book does not
//! list or one given twice is an error, never a half-read file. Writing
//! checks the book the same way and carries the bytes of each of its
//! pictures the host has (one it lacks is left out: the preflight of the
//! sheet that shows it says so on the other side).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::error::{Result, SheetError};
use crate::model::SheetBook;
use crate::template::{AssetWithBytes, base64_decode, sha256_hex};
use crate::validate::{read_book, validate_book};

pub const FORMAT: &str = "kentos.sheet.file";
pub const VERSION: u32 = 1;

/// A `.kpafta` file as read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct KpaftaFile {
    /// `kentos.sheet.file`.
    pub format: String,
    pub version: u32,
    pub book: SheetBook,
    /// The book's pictures with their bytes, in the book's order.
    pub assets: Vec<AssetWithBytes>,
}

/// A picture's bytes held to its metadata; the bytes when they match.
fn check(a: &AssetWithBytes, path: &str) -> Result<Vec<u8>> {
    let bytes = base64_decode(&a.data).ok_or_else(|| {
        SheetError::at(
            "bad_asset",
            format!("{path}.data"),
            format!(
                "“{}” resminin baytları base64 olarak okunamadı.",
                a.meta.name
            ),
        )
    })?;
    if sha256_hex(&bytes) != a.meta.sha256 || bytes.len() as u64 != u64::from(a.meta.bytes) {
        return Err(SheetError::at(
            "bad_asset",
            path,
            format!(
                "“{}” resminin baytları kimliğine (SHA-256) ya da boyuna uymuyor; dosya bozulmuş.",
                a.meta.name
            ),
        ));
    }
    Ok(bytes)
}

/// The file's text for a book and the bytes the host has of its pictures.
pub fn encode(book: &SheetBook, assets: &[AssetWithBytes]) -> Result<String> {
    validate_book(book)?;
    let mut carried = Vec::new();
    for (i, meta) in book.assets.iter().enumerate() {
        if let Some(a) = assets.iter().find(|a| a.meta.sha256 == meta.sha256) {
            let a = AssetWithBytes {
                meta: meta.clone(),
                data: a.data.clone(),
            };
            check(&a, &format!("assets[{i}]"))?;
            carried.push(a);
        }
    }
    let file = KpaftaFile {
        format: FORMAT.to_owned(),
        version: VERSION,
        book: book.clone(),
        assets: carried,
    };
    serde_json::to_string(&file)
        .map_err(|e| SheetError::new("bad_json", format!("Pafta dosyası yazılamadı: {e}")))
}

/// A file's text read: its book and its pictures, every one checked.
pub fn decode(text: &str) -> Result<KpaftaFile> {
    let raw: serde_json::Value = serde_json::from_str(text)
        .map_err(|_| SheetError::new("bad_json", "Dosya JSON değil: bir .kpafta dosyası seçin."))?;
    if raw.get("format").and_then(|f| f.as_str()) != Some(FORMAT) {
        return Err(SheetError::at(
            "unknown_schema",
            "format",
            "Bu bir KentOS pafta dosyası (.kpafta) değil.",
        ));
    }
    match raw.get("version").and_then(serde_json::Value::as_u64) {
        Some(v) if v == u64::from(VERSION) => {}
        Some(v) if v > u64::from(VERSION) => {
            return Err(SheetError::at(
                "newer_schema",
                "version",
                format!("Dosya bu sürümün bilmediği {v}. biçimde yazılmış; KentOS'u güncelleyin."),
            ));
        }
        _ => {
            return Err(SheetError::at(
                "unknown_schema",
                "version",
                "Pafta dosyasının sürümü okunamadı.",
            ));
        }
    }
    if let Some(k) = raw.as_object().and_then(|o| {
        o.keys()
            .find(|k| !matches!(k.as_str(), "format" | "version" | "book" | "assets"))
    }) {
        return Err(SheetError::at(
            "bad_json",
            k.clone(),
            format!("Pafta dosyasında bilinmeyen alan: “{k}”."),
        ));
    }
    let book_json = raw
        .get("book")
        .ok_or_else(|| SheetError::at("bad_json", "book", "Dosyada pafta kitabı yok."))?
        .to_string();
    let book = read_book(&book_json)?;
    let assets: Vec<AssetWithBytes> = match raw.get("assets") {
        None => Vec::new(),
        Some(v) => serde_json::from_value(v.clone())
            .map_err(|e| SheetError::json("Pafta dosyasının resimleri", &e))?,
    };
    let mut seen = std::collections::BTreeSet::new();
    for (i, a) in assets.iter().enumerate() {
        let path = format!("assets[{i}]");
        if !seen.insert(a.meta.sha256.clone()) {
            return Err(SheetError::at(
                "bad_asset",
                path,
                format!("“{}” resmi dosyada iki kez var.", a.meta.name),
            ));
        }
        let Some(meta) = book.asset(&a.meta.sha256) else {
            return Err(SheetError::at(
                "bad_asset",
                path,
                format!("“{}” resmi kitabın resimleri arasında yok.", a.meta.name),
            ));
        };
        if *meta != a.meta {
            return Err(SheetError::at(
                "bad_asset",
                path,
                format!("“{}” resminin bilgileri kitabınkine uymuyor.", a.meta.name),
            ));
        }
        check(a, &path)?;
    }
    Ok(KpaftaFile {
        format: FORMAT.to_owned(),
        version: VERSION,
        book,
        assets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_goes_round_and_a_broken_one_is_refused_whole() {
        let mut book = SheetBook::default();
        let bytes = b"\x89PNG kucuk resim".to_vec();
        let meta = crate::model::AssetMeta {
            sha256: sha256_hex(&bytes),
            kind: crate::model::AssetKind::Png,
            name: "logo.png".into(),
            width: 1,
            height: 1,
            bytes: bytes.len() as u32,
            dpi: None,
        };
        book.assets.push(meta.clone());
        let asset = AssetWithBytes {
            meta: meta.clone(),
            data: crate::template::base64_encode(&bytes),
        };
        let text = encode(&book, std::slice::from_ref(&asset)).unwrap();
        let file = decode(&text).unwrap();
        assert_eq!(
            (file.book, file.assets),
            (book.clone(), vec![asset.clone()])
        );
        // A picture the host lacks is left out.
        assert!(
            decode(&encode(&book, &[]).unwrap())
                .unwrap()
                .assets
                .is_empty()
        );
        // Bytes that do not match.
        let mut bad = serde_json::from_str::<serde_json::Value>(&text).unwrap();
        bad["assets"][0]["data"] = crate::template::base64_encode(b"baska").into();
        assert_eq!(decode(&bad.to_string()).unwrap_err().code, "bad_asset");
        assert_eq!(decode("{").unwrap_err().code, "bad_json");
        assert_eq!(
            decode(r#"{"format":"x","version":1}"#).unwrap_err().code,
            "unknown_schema"
        );
        assert_eq!(
            decode(r#"{"format":"kentos.sheet.file","version":2}"#)
                .unwrap_err()
                .code,
            "newer_schema"
        );
    }
}
