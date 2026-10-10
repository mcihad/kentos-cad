//! Schema 37's variables (docs/specs/kcad-v2.md §6.4.9, docs/adr/0214 §2.3):
//! the settings' `variables`, read field by field and then checked whole by
//! the contract's rules (`variables_problem`), as the writers check them; an
//! empty list is refused (a writer leaves it out).

use kentos_contracts::{ProjectVariable, VariableKind, VariableValue, variables_problem};

use super::{list, map, named, required, text, unknown};
use crate::cbor::Reader;
use crate::error::{Code, KcadError};

/// A value: null, true/false, a float64 or text.
fn value(r: &mut Reader<'_>) -> Result<VariableValue, KcadError> {
    Ok(match r.peek() {
        Some(0xf4 | 0xf5) => VariableValue::Bool(r.bool()?),
        Some(b) if b >> 5 == 3 => VariableValue::Text(text(r)?),
        _ => match r.float_or_null()? {
            Some(x) => VariableValue::Number(x),
            None => VariableValue::Null,
        },
    })
}

pub(super) fn variables(r: &mut Reader<'_>) -> Result<Vec<ProjectVariable>, KcadError> {
    let at = r.position();
    let kinds = [
        ("text", VariableKind::Text),
        ("number", VariableKind::Number),
        ("bool", VariableKind::Bool),
        ("date", VariableKind::Date),
    ];
    let all = list(r, |r, _| {
        let (mut name, mut label, mut kind, mut v) = (None, None, None, None);
        map(r, |r, key| {
            match key {
                "name" => name = Some(text(r)?),
                "label" => {
                    let at = r.position();
                    let t = text(r)?;
                    if t.is_empty() {
                        return Err(r.fail_at(Code::BadValue, at, "boş etiket yazılmaz"));
                    }
                    label = Some(t);
                }
                "kind" => kind = Some(named(r, &kinds)?),
                "value" => v = Some(value(r)?),
                _ => return Err(unknown(r)),
            }
            Ok(())
        })?;
        Ok(ProjectVariable {
            name: required(r, name, "name")?,
            label: label.unwrap_or_default(),
            kind: required(r, kind, "kind")?,
            value: v.unwrap_or_default(),
        })
    })?;
    if all.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, "boş değişken listesi yazılmaz"));
    }
    if let Some(problem) = variables_problem(&all) {
        return Err(r.fail_at(Code::BadValue, at, &problem));
    }
    Ok(all)
}
