//! The request ledger (document schema 45, docs/specs/kcad-v2.md §6.13,
//! docs/adr/0253 §6): the requests a drawing executed, each its id, the
//! call's fingerprint, the command, its version, the revision after it, its
//! status and the answer then, held to the contract's rules
//! (`requests_problem`) as the readers hold them.

use kentos_contracts::{CallStatus, RequestRecord, RequestsFault, requests_problem};

use super::Encoder;
use crate::cbor::Seg;
use crate::error::{Code, KcadError};

/// 32 lowercase hexadecimal digits as their 16 bytes.
fn fingerprint_bytes(hex: &str) -> Option<[u8; 16]> {
    let digits = hex.as_bytes();
    if digits.len() != 32 {
        return None;
    }
    let value = |c: u8| match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    };
    let mut out = [0u8; 16];
    for (i, pair) in digits.chunks(2).enumerate() {
        *out.get_mut(i)? = (value(*pair.first()?)? << 4) | value(*pair.get(1)?)?;
    }
    Some(out)
}

impl<'d> Encoder<'d> {
    pub(super) fn requests(&mut self, list: &'d [RequestRecord]) -> Result<(), KcadError> {
        if let Some((at, why)) = requests_problem(list) {
            if let RequestsFault::Field(i, field) = at {
                self.path.push(Seg::Index(i));
                self.path.push(Seg::Name(field));
            }
            return Err(self.fail(Code::BadValue, why));
        }
        self.open(list.len(), false)?;
        for (i, r) in list.iter().enumerate() {
            self.at(Seg::Index(i), |e| e.request(r))?;
        }
        self.close();
        Ok(())
    }

    fn request(&mut self, r: &'d RequestRecord) -> Result<(), KcadError> {
        self.open(6 + usize::from(r.answer.is_some()), true)?;
        // answer status (6), command version (7), revision (8), requestId (9), fingerprint (11).
        if let Some(answer) = &r.answer {
            self.key("answer");
            self.at(Seg::Name("answer"), |e| e.text(answer))?;
        }
        self.key("status");
        self.at(Seg::Name("status"), |e| {
            e.text(match r.status {
                CallStatus::Done => "done",
                CallStatus::Undone => "undone",
            })
        })?;
        self.key("command");
        self.at(Seg::Name("command"), |e| e.text(&r.command))?;
        self.key("version");
        self.w.uint(u64::from(r.version));
        self.key("revision");
        self.at(Seg::Name("revision"), |e| e.text(&r.revision))?;
        self.key("requestId");
        self.at(Seg::Name("requestId"), |e| e.text(&r.request_id))?;
        self.key("fingerprint");
        self.at(Seg::Name("fingerprint"), |e| {
            let bytes = fingerprint_bytes(&r.fingerprint).ok_or_else(|| {
                e.fail(Code::BadValue, "parmak izi 32 küçük onaltılık hane olmalı")
            })?;
            e.w.bytes(&bytes);
            Ok(())
        })?;
        self.close();
        Ok(())
    }
}
