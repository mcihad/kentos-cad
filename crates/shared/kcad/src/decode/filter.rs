//! Schema 35's layer filter (docs/specs/kcad-v2.md §6.5, docs/adr/0211 §2):
//! a layer's `filter`, read field by field and checked whole by the
//! contract's rules (`LayerFilter::problem`), as the writers check it. The
//! tree's own rule (`filters_problem`: not on a group, not on a layer drawn
//! from a service) is checked once the tree is read.

use kentos_contracts::{EntityId, LayerFilter};

use super::{id16, list, map, text, unknown};
use crate::cbor::Reader;
use crate::error::{Code, KcadError};

pub(super) fn layer_filter(r: &mut Reader<'_>) -> Result<LayerFilter, KcadError> {
    let at = r.position();
    let mut filter = LayerFilter::default();
    map(r, |r, k| {
        match k {
            "objects" => {
                let list_at = r.position();
                filter.objects = list(r, |r, _| id16(r).map(EntityId))?;
                // An empty list is not written (the field is left out).
                if filter.objects.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        list_at,
                        "süzgecin nesne listesi boş; boş liste yazılmaz",
                    ));
                }
            }
            "expression" => filter.expression = Some(text(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    match filter.problem() {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(filter),
    }
}
