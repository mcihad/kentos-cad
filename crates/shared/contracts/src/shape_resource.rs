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
    if !bytes.starts_with(b"AutoCAD-86 shapes 1.0\r\n\x1a")
        && !bytes.starts_with(b"AutoCAD-86 shapes 1.1\r\n\x1a")
    {
        return Err("Bu SHX biçimi desteklenmiyor; shapes 1.0/1.1 gerekli.".into());
    }
    Ok(bytes)
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
}
