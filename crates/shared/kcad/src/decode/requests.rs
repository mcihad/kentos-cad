//! Schema 45's request ledger (docs/specs/kcad-v2.md §6.13, docs/adr/0253
//! §6): the requests a drawing executed; an empty list is refused (a writer
//! leaves it out), and the contract's rules hold (`requests_problem`): at
//! most 256, each id by its rule and once, a command and a version, the
//! revision decimal text, an answer of at most 64 KiB.

use kentos_contracts::{CallStatus, RequestRecord, RequestsFault, requests_problem};

use super::{list, map, named, required, text, unknown};
use crate::cbor::{Reader, Seg};
use crate::error::{Code, KcadError};

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from(b"0123456789abcdef"[usize::from(b >> 4)]));
        s.push(char::from(b"0123456789abcdef"[usize::from(b & 15)]));
    }
    s
}

fn request(r: &mut Reader<'_>) -> Result<RequestRecord, KcadError> {
    let (mut answer, mut status, mut command, mut version) = (None, None, None, None);
    let (mut revision, mut request_id, mut fingerprint) = (None, None, None);
    map(r, |r, key| {
        match key {
            "answer" => answer = Some(text(r)?),
            "status" => {
                status = Some(named(
                    r,
                    &[("done", CallStatus::Done), ("undone", CallStatus::Undone)],
                )?)
            }
            "command" => command = Some(text(r)?),
            "version" => {
                version = Some(u32::try_from(r.uint(u64::from(u32::MAX))?).unwrap_or(u32::MAX))
            }
            "revision" => revision = Some(text(r)?),
            "requestId" => request_id = Some(text(r)?),
            "fingerprint" => {
                let (raw, at) = r.bytes()?;
                if raw.len() != 16 {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("{} bayt; parmak izi 16 bayt olmalı", raw.len()),
                    ));
                }
                fingerprint = Some(hex(raw));
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(RequestRecord {
        request_id: required(r, request_id, "requestId")?,
        fingerprint: required(r, fingerprint, "fingerprint")?,
        command: required(r, command, "command")?,
        version: required(r, version, "version")?,
        revision: required(r, revision, "revision")?,
        status: required(r, status, "status")?,
        answer,
    })
}

pub(super) fn requests(r: &mut Reader<'_>) -> Result<Vec<RequestRecord>, KcadError> {
    let at = r.position();
    let out = list(r, |r, _| request(r))?;
    if out.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, "boş istek listesi yazılmaz"));
    }
    if let Some((fault, why)) = requests_problem(&out) {
        if let RequestsFault::Field(i, field) = fault {
            r.push(Seg::Index(i));
            r.push(Seg::Name(field));
            let e = r.fail_at(Code::BadValue, at, why);
            r.pop();
            r.pop();
            return Err(e);
        }
        return Err(r.fail_at(Code::BadValue, at, why));
    }
    Ok(out)
}
