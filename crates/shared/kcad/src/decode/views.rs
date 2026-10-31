//! Schema 52's named views (docs/specs/kcad-v2.md §6.4.12, docs/adr/0285
//! §1): the settings' `namedViews`, read field by field and then checked by
//! the contract's rule (`views_problem`), as the writers check them; an
//! empty list is refused (a writer leaves it out).

use kentos_contracts::{NamedView, ViewRecipe, ViewTemplate, views_problem};

use super::{list, map, point, required, text, unknown};
use crate::cbor::Reader;
use crate::error::{Code, KcadError};

/// The named views: not empty, then checked whole.
/// Schema 81 (docs/adr/0345 §2) adds a view's `id`; schema 141 (docs/adr/0774 §1) its `recipe`, read whole as the
/// contract's JSON and written as the writer writes it (no field at its default); schema 143 (docs/adr/0781) its `detail`
/// and `crop`.
pub(super) fn named_views(
    r: &mut Reader<'_>,
    ids: bool,
    recipes: bool,
    detail: bool,
    templates: bool,
    options: bool,
) -> Result<Vec<NamedView>, KcadError> {
    let at = r.position();
    let all = list(r, |r, _| {
        let (mut id, mut name, mut center, mut width, mut height, mut twist) =
            (None, None, None, None, None, None);
        let mut recipe = None;
        map(r, |r, key| {
            match key {
                "id" if ids => {
                    let at = r.position();
                    let t = text(r)?;
                    if t.is_empty() {
                        return Err(r.fail_at(Code::BadValue, at, "boş görünüm kimliği yazılmaz"));
                    }
                    id = Some(t);
                }
                "name" => name = Some(text(r)?),
                "center" => center = Some(point(r)?),
                "width" => width = Some(r.float()?),
                "height" => height = Some(r.float()?),
                "twist" => twist = Some(r.float()?),
                "recipe" if recipes => {
                    let at = r.position();
                    let value = r.opaque()?;
                    let read: ViewRecipe = serde_json::from_value(value.clone()).map_err(|e| {
                        let words = e.to_string();
                        let code = if words.contains("unknown field") || words.contains("unknown variant") {
                            Code::UnknownField
                        } else {
                            Code::BadValue
                        };
                        r.fail_at(code, at, &format!("görünüş tarifi okunamadı: {words}"))
                    })?;
                    if serde_json::to_value(&read).ok().as_ref() != Some(&value) {
                        return Err(r.fail_at(
                            Code::BadValue,
                            at,
                            "görünüş tarifi yazıcının biçiminde değil (varsayılan değerinde bir alan yazılmış)",
                        ));
                    }
                    if !detail && (read.detail.is_some() || read.crop) {
                        return Err(r.fail_at(
                            Code::UnknownField,
                            at,
                            "görünüş tarifinin duvar ayrıntısı ve kırpması şema 143'ten beri yazılır",
                        ));
                    }
                    if !templates && (!read.pens.is_empty() || read.mono || read.template.is_some()) {
                        return Err(r.fail_at(
                            Code::UnknownField,
                            at,
                            "görünüş tarifinin kalemleri, siyah beyazı ve şablonu şema 146'dan beri yazılır",
                        ));
                    }
                    if !options && !read.options.is_empty() {
                        return Err(r.fail_at(
                            Code::UnknownField,
                            at,
                            "görünüş tarifinin tasarım seçenekleri şema 153'ten beri yazılır",
                        ));
                    }
                    recipe = Some(read);
                }
                _ => return Err(unknown(r)),
            }
            Ok(())
        })?;
        let id = id.unwrap_or_default();
        Ok(NamedView {
            id,
            name: required(r, name, "name")?,
            center: required(r, center, "center")?,
            width: required(r, width, "width")?,
            height: required(r, height, "height")?,
            twist: required(r, twist, "twist")?,
            recipe,
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

/// Schema 146 (docs/adr/0783 §2): the project's view templates, the contract's JSON as the writer writes it (no field at
/// its default), not empty, checked by the contract's rule on their own (the views' templates are checked with the
/// settings).
pub(super) fn view_templates(r: &mut Reader<'_>) -> Result<Vec<ViewTemplate>, KcadError> {
    let at = r.position();
    let value = r.opaque()?;
    let read: Vec<ViewTemplate> = serde_json::from_value(value.clone()).map_err(|e| {
        let words = e.to_string();
        let code = if words.contains("unknown field") || words.contains("unknown variant") {
            Code::UnknownField
        } else {
            Code::BadValue
        };
        r.fail_at(code, at, &format!("görünüş şablonları okunamadı: {words}"))
    })?;
    if serde_json::to_value(&read).ok().as_ref() != Some(&value) {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "görünüş şablonları yazıcının biçiminde değil (varsayılan değerinde bir alan yazılmış)",
        ));
    }
    if read.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, "boş görünüş şablonu listesi yazılmaz"));
    }
    if let Some(problem) = kentos_contracts::templates_problem(&read, &[]) {
        return Err(r.fail_at(Code::BadValue, at, &problem));
    }
    Ok(read)
}
