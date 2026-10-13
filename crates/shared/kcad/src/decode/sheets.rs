//! Schema 44's sheet book (docs/specs/kcad-v2.md §6.12, docs/adr/0251): the
//! book as an opaque value, a map; its pictures' bytes each held to the
//! SHA-256 it is given under, none given twice; an empty picture list is
//! refused (a writer leaves it out). The book's own rules are the sheet
//! core's, which reads it when the project is opened.

use std::collections::BTreeSet;

use kentos_contracts::{ProjectSheets, SheetPicture, encoding::sha256_hex};

use super::{list, map, required, unknown};
use crate::cbor::Reader;
use crate::error::{Code, KcadError};

/// 32 bytes as 64 lowercase hexadecimal digits.
fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from(b"0123456789abcdef"[usize::from(b >> 4)]));
        s.push(char::from(b"0123456789abcdef"[usize::from(b & 15)]));
    }
    s
}

fn picture(r: &mut Reader<'_>) -> Result<(SheetPicture, usize), KcadError> {
    let (mut bytes, mut sha) = (None, None);
    map(r, |r, key| {
        match key {
            "bytes" => bytes = Some(r.bytes()?.0.to_vec()),
            "sha256" => {
                let (raw, at) = r.bytes()?;
                if raw.len() != 32 {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("{} bayt; SHA-256 32 bayt olmalı", raw.len()),
                    ));
                }
                sha = Some((hex(raw), at));
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let bytes = required(r, bytes, "bytes")?;
    let (sha256, at) = required(r, sha, "sha256")?;
    Ok((SheetPicture { sha256, bytes }, at))
}

pub(super) fn sheets(r: &mut Reader<'_>) -> Result<ProjectSheets, KcadError> {
    let (mut book, mut pictures) = (None, None);
    map(r, |r, key| {
        match key {
            "book" => {
                let at = r.position();
                let value = r.opaque()?;
                if !value.is_object() {
                    return Err(r.fail_at(Code::WrongType, at, "pafta kitabı bir harita olmalı"));
                }
                book = Some(value);
            }
            "pictures" => {
                let at = r.position();
                let mut seen = BTreeSet::new();
                let list = list(r, |r, _| {
                    let (p, at) = picture(r)?;
                    if sha256_hex(&p.bytes) != p.sha256 {
                        return Err(r.fail_at(
                            Code::BadValue,
                            at,
                            "resmin baytları SHA-256'sına uymuyor",
                        ));
                    }
                    if !seen.insert(p.sha256.clone()) {
                        return Err(r.fail_at(Code::BadValue, at, "aynı resim iki kez verilmiş"));
                    }
                    Ok(p)
                })?;
                if list.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "boş resim listesi yazılmaz"));
                }
                pictures = Some(list);
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(ProjectSheets {
        book: required(r, book, "book")?,
        pictures: pictures.unwrap_or_default(),
    })
}
