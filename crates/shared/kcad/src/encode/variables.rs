//! The project's variables (document schema 37, docs/specs/kcad-v2.md §6.4.9,
//! docs/adr/0214 §2.3): checked whole by the contract's rules
//! (`variables_problem`), as the readers check them; each map's keys sorted
//! as RFC 8949 sorts them (`kind`, `name`, `label`, `value`). A label is
//! written only when it is not empty, a value only when there is one.

use kentos_contracts::{ProjectVariable, VariableKind, VariableValue, variables_problem};

use super::Encoder;
use crate::cbor::Seg;
use crate::error::{Code, KcadError};

fn kind_name(k: VariableKind) -> &'static str {
    match k {
        VariableKind::Text => "text",
        VariableKind::Number => "number",
        VariableKind::Bool => "bool",
        VariableKind::Date => "date",
    }
}

impl<'d> Encoder<'d> {
    pub(super) fn variables(&mut self, list: &'d [ProjectVariable]) -> Result<(), KcadError> {
        if let Some(problem) = variables_problem(list) {
            return Err(self.fail(Code::BadValue, &problem));
        }
        self.open(list.len(), false)?;
        for (i, v) in list.iter().enumerate() {
            self.at(Seg::Index(i), |e| {
                let has_value = !matches!(v.value, VariableValue::Null);
                e.open(
                    2 + usize::from(!v.label.is_empty()) + usize::from(has_value),
                    true,
                )?;
                e.key("kind");
                e.text(kind_name(v.kind))?;
                e.key("name");
                e.at(Seg::Name("name"), |e| e.text(&v.name))?;
                if !v.label.is_empty() {
                    e.key("label");
                    e.at(Seg::Name("label"), |e| e.text(&v.label))?;
                }
                if has_value {
                    e.key("value");
                    e.at(Seg::Name("value"), |e| match &v.value {
                        VariableValue::Null => Ok(()),
                        VariableValue::Bool(b) => {
                            e.w.bool(*b);
                            Ok(())
                        }
                        VariableValue::Number(x) => e.float(*x),
                        VariableValue::Text(t) => e.text(t),
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
