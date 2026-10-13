//! Schema 49's user coordinate systems (docs/specs/kcad-v2.md §6.4.11,
//! docs/adr/0261 §1): the settings' `ucs` and `namedUcs`, read field by
//! field and then checked by the contract's rule (`ucs_problem`), as the
//! writers check them; the world's UCS and an empty list are refused (a
//! writer leaves them out).

use kentos_contracts::{NamedUcs, Ucs, ucs_problem};

use super::{list, map, point, required, text, unknown};
use crate::cbor::Reader;
use crate::error::{Code, KcadError};

/// A frame's fields, with a name when `named`.
fn frame(r: &mut Reader<'_>, named: bool) -> Result<(Option<String>, Ucs), KcadError> {
    let (mut name, mut origin, mut angle) = (None, None, None);
    map(r, |r, key| {
        match key {
            "name" if named => name = Some(text(r)?),
            "origin" => origin = Some(point(r)?),
            "angle" => angle = Some(r.float()?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let ucs = Ucs {
        origin: required(r, origin, "origin")?,
        angle: required(r, angle, "angle")?,
    };
    let name = if named {
        Some(required(r, name, "name")?)
    } else {
        None
    };
    Ok((name, ucs))
}

/// The current UCS: never the world's.
pub(super) fn ucs(r: &mut Reader<'_>) -> Result<Ucs, KcadError> {
    let at = r.position();
    let (_, u) = frame(r, false)?;
    if let Some(problem) = ucs_problem(Some(&u), &[]) {
        return Err(r.fail_at(Code::BadValue, at, &problem));
    }
    Ok(u)
}

/// The named UCSs: not empty, then checked whole.
pub(super) fn named_ucs(r: &mut Reader<'_>) -> Result<Vec<NamedUcs>, KcadError> {
    let at = r.position();
    let all = list(r, |r, _| {
        let (name, u) = frame(r, true)?;
        Ok(NamedUcs {
            name: name.unwrap_or_default(),
            origin: u.origin,
            angle: u.angle,
        })
    })?;
    if all.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, "boş adlı UCS listesi yazılmaz"));
    }
    if let Some(problem) = ucs_problem(None, &all) {
        return Err(r.fail_at(Code::BadValue, at, &problem));
    }
    Ok(all)
}
