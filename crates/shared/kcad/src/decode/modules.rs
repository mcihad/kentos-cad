//! The module versions in the payload's root (`modules`, schema 254; docs/specs/kcad-v2.md §6.1, docs/adr/8054 §10):
//! each module's and each `.kek` extension's own version of the data it adds. They come before `version` and the
//! document, so a file whose module data this build does not know is refused as written by a newer KentOS before its
//! document is read, not called broken over a field it cannot know.

use super::*;
use crate::{MODULE_VERSIONS, ModuleVersions, SCHEMA_WITH_MODULE_VERSIONS};

/// A root's `modules` as read, with where it starts in the payload.
pub(super) struct Read {
    pub(super) versions: ModuleVersions,
    pub(super) at: usize,
}

/// `modules`: a map of module keys to versions, the extensions' under `ext`; neither it nor `ext` is empty, no version
/// is 0 (a module that did not raise its version is not written).
pub(super) fn read(r: &mut Reader<'_>) -> Result<Read, KcadError> {
    let at = r.position();
    let mut versions = ModuleVersions::default();
    map(r, |r, key| {
        if key == "ext" {
            let ext_at = r.position();
            map(r, |r, id| {
                versions.extensions.push((id.to_owned(), version(r)?));
                Ok(())
            })?;
            if versions.extensions.is_empty() {
                return Err(r.fail_at(Code::BadValue, ext_at, "boş eklenti sürümleri yazılmaz"));
            }
        } else {
            versions.modules.push((key.to_owned(), version(r)?));
        }
        Ok(())
    })?;
    if versions.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, "boş modül sürümleri yazılmaz"));
    }
    Ok(Read { versions, at })
}

fn version(r: &mut Reader<'_>) -> Result<u16, KcadError> {
    let at = r.position();
    let v = r.uint(u64::from(u16::MAX))?;
    if v == 0 {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "sürüm 0 yazılmaz: sürümünü yükseltmemiş modül yazılmaz",
        ));
    }
    Ok(v as u16)
}

/// The versions against the document schema and this build: `modules` needs schema 254; a newer module version, a
/// module this build does not know and an extension's data are written by a newer KentOS (or need the extension).
pub(super) fn check(r: &Reader<'_>, schema: u32, read: &Read) -> Result<(), KcadError> {
    if schema < SCHEMA_WITH_MODULE_VERSIONS {
        return Err(r.fail_at(
            Code::UnknownField,
            read.at,
            &format!("modül sürümleri şema {SCHEMA_WITH_MODULE_VERSIONS} ister"),
        ));
    }
    for (key, v) in &read.versions.modules {
        match MODULE_VERSIONS.iter().find(|(k, _, _)| k == key) {
            Some((_, name, known)) if v > known => {
                return Err(KcadError::new(
                    Code::SchemaVersion,
                    format!(
                        "Bu dosyanın {name} verisi KentOS'un daha yeni bir sürümüyle yazılmış ({name} verisi sürüm {v}; \
                         bu sürümün okuduğu: {known}). KentOS'u güncelleyip yeniden açın."
                    ),
                ));
            }
            Some(_) => {}
            None => {
                return Err(KcadError::new(
                    Code::SchemaVersion,
                    format!(
                        "Bu dosya bu KentOS sürümünün tanımadığı bir modülün (`{key}`, sürüm {v}) verisini taşıyor. \
                         KentOS'u güncelleyip yeniden açın."
                    ),
                ));
            }
        }
    }
    if let Some((id, v)) = read.versions.extensions.first() {
        return Err(KcadError::new(
            Code::SchemaVersion,
            format!(
                "Bu dosya `{id}` eklentisinin verisini taşıyor (sürüm {v}); bu KentOS onu okuyamıyor. Eklentiyi kurup ya \
                 da KentOS'u güncelleyip yeniden açın."
            ),
        ));
    }
    Ok(())
}
