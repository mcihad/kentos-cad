//! A layer bound to its source (docs/adr/0503, plan/gis.md CBS-01, CBS-02, CBS-36): the project's `sources`, each a
//! layer of the drawing kept as the working copy of a table of a file (a GeoPackage today), with the file's revision
//! when the layer last matched it. The objects stay in the drawing, so a project whose file is gone still opens with
//! them; writing back and reloading go through `cad.sources.*`, which compare the file with this revision first. No
//! password or token is kept: a GeoPackage needs none.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// At most this many bound layers in a project.
pub const MAX_SOURCES: usize = 1000;

/// What a source is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SourceKind {
    Geopackage,
    /// A PostGIS table (docs/adr/0505): `path` is the database's address without a password.
    Postgis,
}

/// A layer bound to a table of a file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourceLink {
    /// Its own id, `kaynak-N`.
    pub id: String,
    pub kind: SourceKind,
    /// The file, an absolute path.
    pub path: String,
    /// The table in it.
    pub table: String,
    /// The drawing's layer that is its working copy.
    pub layer: String,
    /// The SHA-256 (64 hex digits) of the file when the layer last matched it: what a write or a reload compares with.
    pub revision: String,
    /// The layer's digest when it last matched the file (64 hex digits: the SHA-256 of its objects' kinds, geometries
    /// and attributes by lasting id, `kentos_native_application::sources::layer_digest`): a reload compares with it to
    /// tell the layer's own edits from the file's.
    pub digest: String,
    /// The EPSG code the table declares, none for an undefined system.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    /// Whether the table can be written: a feature or attribute table of a file KentOS may write.
    pub writable: bool,
}

/// What is wrong with the project's sources against its layers' ids, if anything.
pub fn sources_problem(list: &[SourceLink], layer_ids: &[&str]) -> Option<String> {
    if list.len() > MAX_SOURCES {
        return Some(format!(
            "bir projede en çok {MAX_SOURCES} bağlı katman olur"
        ));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut layers = std::collections::BTreeSet::new();
    for (i, s) in list.iter().enumerate() {
        let at = format!("{}. kaynak", i + 1);
        if s.id.trim().is_empty() || !ids.insert(s.id.as_str()) {
            return Some(format!(
                "{at}: kimliği boş ya da başka bir kaynağın (“{}”)",
                s.id
            ));
        }
        match s.kind {
            SourceKind::Geopackage => {
                if s.path.trim().is_empty()
                    || !(s.path.starts_with('/') || s.path.get(1..3) == Some(":\\"))
                {
                    return Some(format!("{at}: dosya yolu mutlak değil (“{}”)", s.path));
                }
            }
            SourceKind::Postgis => {
                if !is_database(&s.path) {
                    return Some(format!("{at}: veritabanı adresi değil (“{}”)", s.path));
                }
                if let Some(why) = address_problem(&s.path) {
                    return Some(format!("{at}: {why}"));
                }
            }
        }
        if s.table.trim().is_empty() {
            return Some(format!("{at}: tablosu boş"));
        }
        if !layers.insert(s.layer.as_str()) {
            return Some(format!(
                "{at}: “{}” katmanı başka bir kaynağa da bağlı",
                s.layer
            ));
        }
        if !layer_ids.is_empty() && !layer_ids.contains(&s.layer.as_str()) {
            return Some(format!("{at}: “{}” katmanı çizimde yok", s.layer));
        }
        if s.revision.len() != 64
            || !s
                .revision
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Some(format!("{at}: sürümü 64 küçük onaltılı hane değil"));
        }
        if s.digest.len() != 64
            || !s
                .digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Some(format!("{at}: katmanın özeti 64 küçük onaltılı hane değil"));
        }
    }
    None
}

/// Whether a source's address is a database's (`postgresql://…`) rather than a file's path (docs/adr/0505).
pub fn is_database(address: &str) -> bool {
    address.starts_with("postgresql://") || address.starts_with("postgres://")
}

/// What is wrong with a database address, if anything (docs/adr/0505): a password in it (passwords stay on the
/// device: the environment's `PGPASSWORD` or `~/.pgpass`, never the project), no host or no database.
pub fn address_problem(address: &str) -> Option<String> {
    const PASSWORD: &str = "adreste parola var; parola projeye yazılmaz: PGPASSWORD ortam değişkenine ya da ~/.pgpass dosyasına koyun";
    let rest = address
        .split_once("://")
        .map(|(_, r)| r)
        .unwrap_or_default();
    let (authority, tail) = rest.split_once('/').unwrap_or((rest, ""));
    if let Some((user, _)) = authority.rsplit_once('@')
        && user.contains(':')
    {
        return Some(PASSWORD.into());
    }
    let host = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    if host.trim().is_empty() {
        return Some("adreste sunucu yok (postgresql://kullanıcı@sunucu:5432/veritabanı)".into());
    }
    let (database, query) = tail.split_once('?').unwrap_or((tail, ""));
    if database.trim().is_empty() {
        return Some(
            "adreste veritabanı yok (postgresql://kullanıcı@sunucu:5432/veritabanı)".into(),
        );
    }
    if query.split('&').any(|kv| {
        kv.split('=')
            .next()
            .is_some_and(|k| k.eq_ignore_ascii_case("password"))
    }) {
        return Some(PASSWORD.into());
    }
    None
}
