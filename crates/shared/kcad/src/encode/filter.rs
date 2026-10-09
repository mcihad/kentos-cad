//! A layer's filter (document schema 35, docs/specs/kcad-v2.md §6.5,
//! docs/adr/0211 §2): checked by the contract's rules, as the readers check
//! it, its keys sorted as RFC 8949 sorts them (`objects`, `expression`). Each
//! is written only when it has a value; the objects are 16-byte ids (§6.8).

use kentos_contracts::{LayerFilter, LayerNodeType};

use super::Encoder;
use crate::cbor::Seg;
use crate::error::{Code, KcadError};

impl<'d> Encoder<'d> {
    /// A layer's filter; a group has none.
    pub(super) fn layer_filter(
        &mut self,
        kind: LayerNodeType,
        filter: &'d LayerFilter,
    ) -> Result<(), KcadError> {
        if kind == LayerNodeType::Group {
            return Err(self.fail(
                Code::BadValue,
                "grubun süzgeci olmaz; süzgeç yalnız katmanındır",
            ));
        }
        if let Some(problem) = filter.problem() {
            return Err(self.fail(Code::BadValue, &problem));
        }
        let objects = !filter.objects.is_empty();
        self.open(
            usize::from(objects) + usize::from(filter.expression.is_some()),
            true,
        )?;
        if objects {
            self.key("objects");
            self.at(Seg::Name("objects"), |e| {
                e.open(filter.objects.len(), false)?;
                for (i, id) in filter.objects.iter().enumerate() {
                    e.at(Seg::Index(i), |e| e.id(&id.0))?;
                }
                e.close();
                Ok(())
            })?;
        }
        if let Some(expression) = &filter.expression {
            self.key("expression");
            self.at(Seg::Name("expression"), |e| e.text(expression))?;
        }
        self.close();
        Ok(())
    }
}
