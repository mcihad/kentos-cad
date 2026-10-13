//! The project's sheet book (document schema 44, docs/specs/kcad-v2.md
//! §6.12, docs/adr/0251): the book as an opaque value, as the sheet core
//! keeps it, and its pictures' bytes under their SHA-256, held to the
//! contract's rules (`sheets_problem`) as the readers hold them.

use kentos_contracts::{ProjectSheets, SheetsFault, sheets_problem};

use super::Encoder;
use crate::cbor::{MAX_STRING, Seg};
use crate::error::{Code, KcadError};

/// A SHA-256 written as 64 lowercase hexadecimal digits, as its 32 bytes.
fn digest_bytes(hex: &str) -> Option<[u8; 32]> {
    let mut out = [0u8; 32];
    let digits = hex.as_bytes();
    if digits.len() != 64 {
        return None;
    }
    for (i, pair) in digits.chunks(2).enumerate() {
        let value = |c: u8| match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            _ => None,
        };
        *out.get_mut(i)? = (value(*pair.first()?)? << 4) | value(*pair.get(1)?)?;
    }
    Some(out)
}

impl<'d> Encoder<'d> {
    pub(super) fn sheets(&mut self, s: &'d ProjectSheets) -> Result<(), KcadError> {
        if let Some((at, why)) = sheets_problem(s) {
            match at {
                SheetsFault::Book => self.path.push(Seg::Name("book")),
                SheetsFault::Picture(i) => {
                    self.path.push(Seg::Name("pictures"));
                    self.path.push(Seg::Index(i));
                    self.path.push(Seg::Name("sha256"));
                }
            }
            return Err(self.fail(Code::BadValue, why));
        }
        self.open(1 + usize::from(!s.pictures.is_empty()), true)?;
        // book (4), pictures (8).
        self.key("book");
        self.at(Seg::Name("book"), |e| e.opaque(&s.book))?;
        if !s.pictures.is_empty() {
            self.key("pictures");
            self.at(Seg::Name("pictures"), |e| {
                e.open(s.pictures.len(), false)?;
                for (i, p) in s.pictures.iter().enumerate() {
                    e.at(Seg::Index(i), |e| {
                        e.open(2, true)?;
                        // bytes (5), sha256 (6).
                        e.key("bytes");
                        e.at(Seg::Name("bytes"), |e| {
                            if p.bytes.len() as u64 > MAX_STRING {
                                return Err(e.fail(
                                    Code::TooLong,
                                    &format!(
                                        "resim {} bayt, en çok {MAX_STRING} olabilir",
                                        p.bytes.len()
                                    ),
                                ));
                            }
                            e.w.bytes(&p.bytes);
                            Ok(())
                        })?;
                        e.key("sha256");
                        e.at(Seg::Name("sha256"), |e| {
                            let digest = digest_bytes(&p.sha256).ok_or_else(|| {
                                e.fail(Code::BadValue, "SHA-256, 64 küçük onaltılık hane olmalı")
                            })?;
                            e.w.bytes(&digest);
                            Ok(())
                        })?;
                        e.close();
                        Ok(())
                    })?;
                }
                e.close();
                Ok(())
            })?;
        }
        self.close();
        Ok(())
    }
}
