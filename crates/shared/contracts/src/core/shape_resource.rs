//! Portable, explicitly attached SHX assets (ADR 0930). No path lookup or
//! external URL is permitted; one canonical filename identifies one resource.
use crate::encoding::{base64_decode, base64_encode, sha256_hex};
use serde_json::{Value, json};

pub const MAX_BYTES: usize = 4 * 1024 * 1024;
const PREFIX: &str = "data:application/x-autocad-shx;base64,";

pub fn is_resource(item: &Value) -> bool {
    item["kind"] == "asset" && item["format"] == "shx"
}

/// Imports may preserve an absolute source filename. Resolution uses only its
/// basename, never the source path, on every host platform.
pub fn source_name(file: &str) -> &str {
    file.rsplit(['/', '\\']).next().unwrap_or(file)
}
pub fn id(file: &str) -> String {
    format!("shx:{}", file.to_lowercase())
}
pub fn uses(types: &[crate::LineTypeDef], file: &str) -> usize {
    types
        .iter()
        .filter(|t| {
            t.elements.iter().any(|e| {
                e.shape
                    .as_ref()
                    .is_some_and(|s| id(source_name(&s.file)) == id(file))
            })
        })
        .count()
}

/// Text styles whose SHX font or Big Font is this resource (ADR 1120 §4): a
/// resource they draw with is in use like a linetype's shape file.
pub fn font_uses(styles: &[crate::TextStyleDef], file: &str) -> usize {
    let named = |f: &Option<String>| {
        f.as_deref()
            .is_some_and(|f| id(source_name(f)) == id(file))
    };
    styles
        .iter()
        .filter(|s| named(&s.font_file) || named(&s.big_font_file))
        .count()
}

pub fn read(item: &Value) -> Result<Vec<u8>, String> {
    if !is_resource(item) {
        return Err("SHX kaynak öğesi gerekli.".into());
    }
    let file = item["file"].as_str().ok_or("SHX dosya adı yok.")?;
    if file.is_empty()
        || file.len() > 255
        || source_name(file) != file
        || file.chars().any(|c| c.is_control() || c == ':')
        || !file.to_ascii_lowercase().ends_with(".shx")
        || item["id"].as_str() != Some(id(file).as_str())
    {
        return Err("SHX kaynağının dosya adı veya kimliği geçersiz.".into());
    }
    let data = item["data"]
        .as_str()
        .and_then(|s| s.strip_prefix(PREFIX))
        .ok_or("SHX kaynağı gömülü base64 veri olmalı.")?;
    if data.len() > MAX_BYTES.div_ceil(3) * 4 {
        return Err("SHX kaynağı 4 MiB sınırını aşıyor.".into());
    }
    let bytes = base64_decode(data).ok_or("SHX base64 verisi okunamadı.")?;
    if bytes.len() > MAX_BYTES || base64_encode(&bytes) != data {
        return Err("SHX base64 verisi standart biçimde değil.".into());
    }
    if item["sha256"].as_str() != Some(sha256_hex(&bytes).as_str()) {
        return Err("SHX kaynak özeti uyuşmuyor.".into());
    }
    embeddable(&bytes)?;
    Ok(bytes)
}

const UNIFONT: &[u8] = b"AutoCAD-86 unifont 1.0\r\n\x1a";
const BIGFONT: &[u8] = b"AutoCAD-86 bigfont 1.0\r\n\x1a";

/// Whether a valid resource is a unifont or a Big Font: KCAD schema 184 and
/// `.kstil` readers before it do not know these forms (ADR 1120 §4).
pub fn is_font_form(item: &Value) -> bool {
    read(item).is_ok_and(|b| b.starts_with(UNIFONT) || b.starts_with(BIGFONT))
}

/// The forms KentOS reads (ADR 0930, ADR 1120). A unifont whose definition
/// record says it may not be embedded (type bit 1, Autodesk's Unicode font
/// description) is not put into a project; read-only embedding (bit 2) is
/// what a project resource is, since its bytes are never edited.
fn embeddable(bytes: &[u8]) -> Result<(), String> {
    if bytes.starts_with(b"AutoCAD-86 shapes 1.0\r\n\x1a")
        || bytes.starts_with(b"AutoCAD-86 shapes 1.1\r\n\x1a")
        || bytes.starts_with(BIGFONT)
    {
        return Ok(());
    }
    if !bytes.starts_with(UNIFONT) {
        return Err(
            "Bu SHX biçimi desteklenmiyor; shapes 1.0/1.1, unifont 1.0 veya bigfont 1.0 gerekli."
                .into(),
        );
    }
    let at = UNIFONT.len() + 4;
    let def = bytes
        .get(at..at + 2)
        .map(|b| usize::from(u16::from_le_bytes([b[0], b[1]])))
        .ok_or("SHX unifont tanımı yarım kalmış.")?;
    let record = bytes
        .get(at + 2..at + 2 + def)
        .ok_or("SHX unifont tanımı yarım kalmış.")?;
    let name = record
        .iter()
        .position(|b| *b == 0)
        .ok_or("SHX unifont tanımı geçersiz.")?;
    match &record[name + 1..] {
        [_, _, _, _, embed, 0] if embed & 1 == 0 => Ok(()),
        [_, _, _, _, _, 0] => Err(
            "Bu SHX unifont yazı tipinin tanımı gömülmesine izin vermiyor (tür 1); projeye eklenmedi."
                .into(),
        ),
        _ => Err("SHX unifont tanımı geçersiz.".into()),
    }
}

pub fn item(file: &str, bytes: &[u8]) -> Result<Value, String> {
    if bytes.len() > MAX_BYTES {
        return Err("SHX kaynağı 4 MiB sınırını aşıyor.".into());
    }
    let out = json!({"id":id(file),"kind":"asset","name":file,"path":["CAD","SHX"],"format":"shx",
        "file":file,"data":format!("{PREFIX}{}",base64_encode(bytes)),"sha256":sha256_hex(bytes)});
    read(&out)?;
    Ok(out)
}

pub fn check_items(items: &[Value]) -> Result<(), String> {
    let mut seen = std::collections::BTreeSet::new();
    for item in items.iter().filter(|i| is_resource(i)) {
        read(item)?;
        if !seen.insert(item["id"].as_str()) {
            return Err("Aynı dosya adına bağlı birden fazla SHX kaynağı var.".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portable_resource_has_a_checked_identity_and_no_path_or_url_lookup() {
        let bytes = b"AutoCAD-86 shapes 1.0\r\n\x1a";
        let a = item("Fence.SHX", bytes).unwrap();
        assert_eq!(read(&a).unwrap(), bytes);
        assert_eq!(a["id"], "shx:fence.shx");
        assert_eq!(source_name("C:\\fonts\\Fence.SHX"), "Fence.SHX");
        for f in ["", "../fence.shx", "C:\\fence.shx", "a.txt", "https:a.shx"] {
            assert!(item(f, bytes).is_err());
        }
        let mut bad = a.clone();
        bad["sha256"] = json!("0".repeat(64));
        assert!(read(&bad).is_err());
        bad = a.clone();
        bad["data"] = json!("https://example.org/fence.shx");
        assert!(read(&bad).is_err());
        assert!(check_items(&[a.clone(), a]).is_err());
    }
    #[test]
    fn unifont_and_bigfont_are_resources_unless_the_font_forbids_embedding() {
        let unifont = |embed: u8| {
            let mut b = UNIFONT.to_vec();
            b.extend(2u32.to_le_bytes());
            b.extend(8u16.to_le_bytes());
            b.extend([b'U', 0, 4, 1, 0, 0, embed, 0]);
            b
        };
        assert!(item("u.shx", &unifont(0)).is_ok());
        assert!(item("u.shx", &unifont(2)).is_ok(), "read-only embedding");
        for forbidden in [1, 3] {
            assert!(item("u.shx", &unifont(forbidden)).unwrap_err().contains("izin vermiyor"));
        }
        assert!(item("u.shx", &unifont(0)[..UNIFONT.len() + 7]).is_err());
        assert!(item("b.shx", b"AutoCAD-86 bigfont 1.0\r\n\x1a").is_ok());
        assert!(item("x.shx", b"AutoCAD-86 shapes 2.0\r\n\x1a").is_err());
    }
}
