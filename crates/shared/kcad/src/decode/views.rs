//! Schema 52's named views (docs/specs/kcad-v2.md §6.4.12, docs/adr/0285
//! §1): the settings' `namedViews`, read field by field and then checked by
//! the contract's rule (`views_problem`), as the writers check them; an
//! empty list is refused (a writer leaves it out).

use kentos_contracts::{NamedView, views_problem};

use super::{list, map, point, required, text, unknown};
use crate::cbor::Reader;
use crate::error::{Code, KcadError};

/// The named views: not empty, then checked whole.
pub(super) fn named_views(r: &mut Reader<'_>) -> Result<Vec<NamedView>, KcadError> {
    let at = r.position();
    let all = list(r, |r, _| {
        let (mut name, mut center, mut width, mut height, mut twist) =
            (None, None, None, None, None);
        map(r, |r, key| {
            match key {
                "name" => name = Some(text(r)?),
                "center" => center = Some(point(r)?),
                "width" => width = Some(r.float()?),
                "height" => height = Some(r.float()?),
                "twist" => twist = Some(r.float()?),
                _ => return Err(unknown(r)),
            }
            Ok(())
        })?;
        Ok(NamedView {
            name: required(r, name, "name")?,
            center: required(r, center, "center")?,
            width: required(r, width, "width")?,
            height: required(r, height, "height")?,
            twist: required(r, twist, "twist")?,
        })
    })?;
    if all.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, "boş adlı görünüm listesi yazılmaz"));
    }
    if let Some(problem) = views_problem(&all) {
        return Err(r.fail_at(Code::BadValue, at, &problem));
    }
    Ok(all)
}
