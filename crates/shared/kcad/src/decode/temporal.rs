//! Schema 34's layer time settings and scenarios (docs/specs/kcad-v2.md
//! §6.5, docs/adr/0210 §2): a layer's `time`, a group's `scenario`, a
//! scenario layer's `replaces`. Each is read field by field and checked whole
//! by the contract's rules (`LayerTime::problem`, `ScenarioInfo::problem`),
//! as the writers check it; Birikimli written false is refused (a writer
//! leaves it out). The tree's own rules (`scenarios_problem`) are checked
//! once the tree is read.

use kentos_contracts::{LayerTime, ScenarioInfo};

use super::{map, required, text, unknown};
use crate::cbor::Reader;
use crate::error::{Code, KcadError};

pub(super) fn layer_time(r: &mut Reader<'_>) -> Result<LayerTime, KcadError> {
    let at = r.position();
    let (mut start, mut end, mut key, mut cumulative) = (None, None, None, false);
    map(r, |r, k| {
        match k {
            "end" => end = Some(text(r)?),
            "key" => key = Some(text(r)?),
            "start" => start = Some(text(r)?),
            "cumulative" => {
                let flag = r.position();
                if !r.bool()? {
                    return Err(r.fail_at(
                        Code::BadValue,
                        flag,
                        "cumulative yalnız true yazılır; birikimli olmayan katmanda yazılmaz",
                    ));
                }
                cumulative = true;
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let time = LayerTime {
        start: required(r, start, "start")?,
        end,
        key,
        cumulative,
    };
    match time.problem() {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(time),
    }
}

pub(super) fn scenario(r: &mut Reader<'_>) -> Result<ScenarioInfo, KcadError> {
    let at = r.position();
    let mut note = None;
    map(r, |r, k| {
        match k {
            "note" => note = Some(text(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let info = ScenarioInfo { note };
    match info.problem() {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(info),
    }
}
