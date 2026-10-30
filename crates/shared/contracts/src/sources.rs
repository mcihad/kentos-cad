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
    /// The file's path from the project file's folder, `/` between its parts (docs/adr/0506): written on saving when
    /// the file lies on the same disk, so a project moved with its data (or packed with Paketle) finds it again; read
    /// only when `path` is not there and the file it names has the same content. A database has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub relative: Option<String>,
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
    /// The area the working copy holds (minimum east, north, maximum east, north, in the project's system; docs/adr/0510):
    /// only the table's rows whose box meets it are read, and a write leaves the rows outside as they are. None: the
    /// whole table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub extent: Option<[f64; 4]>,
    /// The attribute that names a row the same whatever its place in the table (docs/adr/0533 §1): its value is unique
    /// among the rows and not empty. A reload follows a label pin to the row of the same value when the table was
    /// written again with other row ids, and says the pins whose row is gone. None: a row is its row id, the rule
    /// of every bound layer before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub key: Option<String>,
    /// The label pins of objects that left the part (docs/adr/0578 §3): the object's row is in the table but outside
    /// `extent` (it was moved out, or the part was narrowed), so the layer holds no object for its pins. They are kept
    /// here with the row's lasting id and its key's value, not drawn, and go back to the object when the row comes
    /// into the part again; a row gone from the table takes its parked pins with it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ParkedPin>>", optional))]
    pub parked: Vec<ParkedPin>,
    /// The table's CHECK constraints as the link last read them (docs/adr/0755 §1): every write of a record is asked them
    /// with SQL's own meaning, and one that ties a column to a list of values makes the field's value list. Binding and
    /// every reading of the table write them anew.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TableCheck>>", optional))]
    pub checks: Vec<TableCheck>,
    /// The table's unique constraints and plain unique indexes as the link last read them (docs/adr/0755 §1): no two
    /// records of the layer hold the same values in their columns.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TableUnique>>", optional))]
    pub uniques: Vec<TableUnique>,
    /// Whether the table can be written: a feature or attribute table of a file KentOS may write.
    pub writable: bool,
}

/// The most CHECK constraints and the most unique constraints a link keeps (docs/adr/0755 §1).
pub const MAX_TABLE_CONSTRAINTS: usize = 256;
/// The most characters of a CHECK constraint's text a link keeps.
pub const MAX_CHECK_SQL: usize = 8000;
/// The most characters of a constraint's and of its columns' names.
pub const MAX_CONSTRAINT_NAME: usize = 256;

/// A CHECK constraint of a bound table (docs/adr/0755 §1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TableCheck {
    /// Its name; none for SQLite's constraint without one, which the database names by its text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// Its condition as the table gives it: the inside of PostgreSQL's `CHECK (…)` (`pg_get_constraintdef`), SQLite's
    /// as its `CREATE TABLE` text writes it.
    pub sql: String,
    /// The table's columns it reads, in the table's order.
    pub columns: Vec<String>,
    /// Only the table judges it (docs/adr/0755 §3): it reads a column whose comparisons KentOS cannot follow (a
    /// collation other than byte order, SQLite's affinity other than the field kind's, PostgreSQL's blank-padded
    /// `character(n)`); written only as `true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub table_only: bool,
}

/// A unique constraint or a plain unique index of a bound table (docs/adr/0755 §1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TableUnique {
    /// Its name; none for SQLite's constraint without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// Its columns, in its order.
    pub columns: Vec<String>,
    /// Two empty (NULL) values count as the same (PostgreSQL's `NULLS NOT DISTINCT`); written only as `true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub nulls_equal: bool,
}

impl TableCheck {
    /// The name the constraint is told by: its own, else its text (as SQLite names a constraint without one).
    pub fn said(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.sql)
    }
}

fn bad_name(s: &str) -> bool {
    s.trim().is_empty()
        || s.trim() != s
        || s.chars().count() > MAX_CONSTRAINT_NAME
        || s.chars().any(char::is_control)
}

fn columns_problem(columns: &[String], need: bool) -> Option<&'static str> {
    if need && columns.is_empty() {
        return Some("sütunu yok");
    }
    if columns.len() > MAX_TABLE_CONSTRAINTS {
        return Some("çok sütunu var");
    }
    if columns.iter().any(|c| bad_name(c)) {
        return Some("sütunlarından birinin adı boş, çok uzun ya da denetim karakteri taşıyor");
    }
    if columns
        .iter()
        .enumerate()
        .any(|(i, c)| columns[..i].contains(c))
    {
        return Some("bir sütunu iki kez geçiyor");
    }
    None
}

/// What is wrong with a link's constraints, if anything (docs/adr/0755 §1): at most [`MAX_TABLE_CONSTRAINTS`] of each, a
/// check's text not empty and at most [`MAX_CHECK_SQL`] characters, names and columns' names not blank, a unique
/// constraint with a column and each column once.
pub fn table_constraints_problem(checks: &[TableCheck], uniques: &[TableUnique]) -> Option<String> {
    if checks.len() > MAX_TABLE_CONSTRAINTS || uniques.len() > MAX_TABLE_CONSTRAINTS {
        return Some(format!(
            "en çok {MAX_TABLE_CONSTRAINTS} CHECK ve {MAX_TABLE_CONSTRAINTS} tekil kısıt tutulur"
        ));
    }
    for (i, c) in checks.iter().enumerate() {
        let at = format!("{}. CHECK kısıtı", i + 1);
        if c.name.as_deref().is_some_and(bad_name) {
            return Some(format!(
                "{at}: adı boş, çok uzun ya da denetim karakteri taşıyor"
            ));
        }
        if c.sql.trim().is_empty() || c.sql.trim() != c.sql || c.sql.chars().count() > MAX_CHECK_SQL
        {
            return Some(format!(
                "{at}: metni boş, başında ya da sonunda boşluk var ya da {MAX_CHECK_SQL} karakteri aşıyor"
            ));
        }
        if let Some(why) = columns_problem(&c.columns, false) {
            return Some(format!("{at}: {why}"));
        }
    }
    for (i, u) in uniques.iter().enumerate() {
        let at = format!("{}. tekil kısıt", i + 1);
        if u.name.as_deref().is_some_and(bad_name) {
            return Some(format!(
                "{at}: adı boş, çok uzun ya da denetim karakteri taşıyor"
            ));
        }
        if let Some(why) = columns_problem(&u.columns, true) {
            return Some(format!("{at}: {why}"));
        }
    }
    None
}

/// The most label pins a bound layer parks, objects counted.
pub const MAX_PARKED: usize = 100_000;

/// The pins of a row outside the part (docs/adr/0578 §3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParkedPin {
    /// The object's lasting id (the row's).
    pub uid: String,
    /// The key's value of the row when the link has a key and the row had a value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub key: Option<String>,
    /// The pins, as the object held them.
    pub pins: Vec<crate::LabelPin>,
}

/// What is wrong with a link's parked pins, if anything: at most [`MAX_PARKED`], each object once with a lasting id
/// and at least one pin, a key value not empty.
pub fn parked_problem(list: &[ParkedPin]) -> Option<String> {
    if list.len() > MAX_PARKED {
        return Some(format!("en çok {MAX_PARKED} nesnenin iğneleri park edilir"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for p in list {
        if p.uid.trim().is_empty() || !seen.insert(p.uid.as_str()) {
            return Some(format!(
                "park edilen iğnenin nesne kimliği boş ya da iki kez geçiyor (“{}”)",
                p.uid
            ));
        }
        if p.pins.is_empty() {
            return Some(format!("“{}” nesnesinin park edilen iğnesi yok", p.uid));
        }
        if p.key.as_deref().is_some_and(|k| k.is_empty()) {
            return Some(format!("“{}” nesnesinin anahtar değeri boş", p.uid));
        }
    }
    None
}

/// Why an area is not one: four finite numbers, the minimum below the maximum on both axes.
pub fn extent_problem(e: &[f64; 4]) -> Option<&'static str> {
    if e.iter().any(|v| !v.is_finite()) {
        Some("sayıları sonlu olmalı")
    } else if e[0] >= e[2] || e[1] >= e[3] {
        Some("en küçüğü en büyüğünden küçük olmalı")
    } else {
        None
    }
}

/// What is wrong with a key's name, if anything: a column's name, not empty, no control letters, at most 128 letters.
pub fn key_problem(key: &str) -> Option<&'static str> {
    if key.trim().is_empty() || key.chars().count() > 128 || key.chars().any(char::is_control) {
        Some("boş, çok uzun ya da denetim karakteri taşıyor")
    } else {
        None
    }
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
        if let Some(k) = &s.key
            && let Some(why) = key_problem(k)
        {
            return Some(format!("{at}: anahtar alanı {why}"));
        }
        if let Some(why) = parked_problem(&s.parked) {
            return Some(format!("{at}: {why}"));
        }
        if let Some(why) = table_constraints_problem(&s.checks, &s.uniques) {
            return Some(format!("{at}: {why}"));
        }
        match s.kind {
            SourceKind::Geopackage => {
                if let Some(e) = &s.extent
                    && let Some(why) = extent_problem(e)
                {
                    return Some(format!("{at}: alan sınırının {why}"));
                }
                if s.path.trim().is_empty()
                    || !(s.path.starts_with('/') || s.path.get(1..3) == Some(":\\"))
                {
                    return Some(format!("{at}: dosya yolu mutlak değil (“{}”)", s.path));
                }
                if let Some(r) = &s.relative
                    && let Some(why) = relative_problem(r)
                {
                    return Some(format!("{at}: göreli yolu {why} (“{r}”)"));
                }
            }
            SourceKind::Postgis => {
                if !is_database(&s.path) {
                    return Some(format!("{at}: veritabanı adresi değil (“{}”)", s.path));
                }
                if let Some(why) = address_problem(&s.path) {
                    return Some(format!("{at}: {why}"));
                }
                if s.relative.is_some() {
                    return Some(format!("{at}: veritabanının göreli yolu olmaz"));
                }
                if s.extent.is_some() {
                    return Some(format!(
                        "{at}: veritabanı tablosunun alan sınırı henüz olmaz"
                    ));
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

/// The longest a source's relative path may be, in letters.
pub const MAX_RELATIVE: usize = 4096;

/// What is wrong with a source's relative path, if anything (docs/adr/0506): not empty, not absolute, its parts
/// separated by `/`, none empty or `.`; `..` only at the start.
pub fn relative_problem(r: &str) -> Option<&'static str> {
    if r.is_empty() || r.chars().count() > MAX_RELATIVE || r.contains('\0') {
        return Some("boş, çok uzun ya da geçersiz");
    }
    if r.starts_with('/') || r.contains('\\') || r.get(1..2) == Some(":") {
        return Some("göreli değil ya da / dışında ayırıcı taşıyor");
    }
    let mut up = true;
    for part in r.split('/') {
        match part {
            "" | "." => return Some("boş ya da . parçası taşıyor"),
            ".." if up => {}
            ".." => return Some("ortasında .. taşıyor"),
            _ => up = false,
        }
    }
    if up {
        return Some("bir dosyayı göstermiyor");
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

/// The most objects a bound layer's base is kept for (docs/adr/0538 §1): a layer of more has none, and its two sides'
/// changes cannot be told apart (`cad.sources.reconcile` refuses, `no_source_base`).
pub const MAX_BASE_ROWS: usize = 200_000;
/// The bytes of one row of a base: the lasting id (16), the layer's hash (8) and the table's (8), big-endian.
pub const BASE_ROW: usize = 32;

/// A bound layer's base (docs/adr/0538 §1): each object's hashes when the layer and its table last matched, so a
/// later reconcile tells what the layer changed from what the table did, object by object. The layer's hash is that of
/// the object's kind, geometry and attributes (the first 8 bytes of the SHA-256 of their canonical JSON, the
/// essence `layer_digest` hashes); the table's that of the object its row reads as. The two differ only where a row
/// does not read back bit for bit as the object written (a coordinate system moved through). A base belongs to the
/// link's `revision` and `digest` it was taken at: with others it is stale and not used.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourceBase {
    /// The link's id (`kaynak-N`).
    pub source: String,
    /// The file's SHA-256 and the layer's digest it was taken at.
    pub revision: String,
    pub digest: String,
    /// The rows, `BASE_ROW` bytes each in the lasting ids' byte order, in base64 (RFC 4648, with padding).
    pub rows: String,
    /// What each row's object held besides its hashes (docs/adr/0553 §1), one record per row in the same order, in
    /// base64: the hashes of its kind and geometry on each side, of each attribute (name and value) and of its
    /// vertices. Without it two sides' changes to one object cannot be told apart field by field (a conflict as a
    /// whole, docs/adr/0538), nor related changes found. Written only when it fits [`MAX_FIELD_BASE_BYTES`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fields: Option<String>,
}

/// The most bytes a base's field records take (decoded): a layer whose records are more keeps none, and its
/// objects conflict as a whole (docs/adr/0553 §1).
pub const MAX_FIELD_BASE_BYTES: usize = 8 * 1024 * 1024;
/// The most vertices a record keeps hashes of; an object of more has none (no related changes through it).
pub const MAX_BASE_VERTICES: usize = 128;
/// A record's flag: its attribute hashes are those of the layer and of the table alike.
pub const FIELDS_FLAG_ATTRS: u8 = 1;
/// A record's flag: its vertex hashes are those of the layer and of the table alike.
pub const FIELDS_FLAG_VERTICES: u8 = 2;

/// What a base keeps of one object besides the two hashes (docs/adr/0553 §1).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BaseDetail {
    /// The hash of the object's kind and geometry (no attributes) in the layer and in the table.
    pub shape: (u64, u64),
    /// Each attribute's name hash and value hash, by rising name hash; none where the table's attributes read
    /// otherwise than the layer's (the object conflicts as a whole).
    pub attrs: Option<Vec<(u32, u64)>>,
    /// The hashes of the object's vertices, rising and once each; none past [`MAX_BASE_VERTICES`] or where the table's
    /// geometry reads otherwise than the layer's.
    pub vertices: Option<Vec<u32>>,
}

/// A base's field records read: `count` of them exactly; none when they are not base64, not whole, out of order or
/// too many bytes.
pub fn base_details(count: usize, text: &str) -> Option<Vec<BaseDetail>> {
    let bytes = crate::encoding::base64_decode(text)?;
    if bytes.len() > MAX_FIELD_BASE_BYTES {
        return None;
    }
    let mut at = 0usize;
    let mut take = |n: usize| -> Option<&[u8]> {
        let part = bytes.get(at..at + n)?;
        at += n;
        Some(part)
    };
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let a = u64::from_be_bytes(take(8)?.try_into().ok()?);
        let b = u64::from_be_bytes(take(8)?.try_into().ok()?);
        let flags = take(1)?[0];
        if flags & !(FIELDS_FLAG_ATTRS | FIELDS_FLAG_VERTICES) != 0 {
            return None;
        }
        let na = usize::from(u16::from_be_bytes(take(2)?.try_into().ok()?));
        let mut attrs: Vec<(u32, u64)> = Vec::with_capacity(na);
        for _ in 0..na {
            let n = u32::from_be_bytes(take(4)?.try_into().ok()?);
            let v = u64::from_be_bytes(take(8)?.try_into().ok()?);
            if attrs.last().is_some_and(|(last, _)| *last >= n) {
                return None;
            }
            attrs.push((n, v));
        }
        let nv = usize::from(u16::from_be_bytes(take(2)?.try_into().ok()?));
        if nv > MAX_BASE_VERTICES {
            return None;
        }
        let mut vertices: Vec<u32> = Vec::with_capacity(nv);
        for _ in 0..nv {
            let v = u32::from_be_bytes(take(4)?.try_into().ok()?);
            if vertices.last().is_some_and(|last| *last >= v) {
                return None;
            }
            vertices.push(v);
        }
        if flags & FIELDS_FLAG_ATTRS == 0 && !attrs.is_empty() {
            return None;
        }
        if flags & FIELDS_FLAG_VERTICES == 0 && !vertices.is_empty() {
            return None;
        }
        out.push(BaseDetail {
            shape: (a, b),
            attrs: (flags & FIELDS_FLAG_ATTRS != 0).then_some(attrs),
            vertices: (flags & FIELDS_FLAG_VERTICES != 0).then_some(vertices),
        });
    }
    (at == bytes.len()).then_some(out)
}

/// The field records written as a base's text; none when they take more than [`MAX_FIELD_BASE_BYTES`].
pub fn base_details_text(details: &[BaseDetail]) -> Option<String> {
    let mut bytes = Vec::new();
    for d in details {
        bytes.extend_from_slice(&d.shape.0.to_be_bytes());
        bytes.extend_from_slice(&d.shape.1.to_be_bytes());
        let flags = (u8::from(d.attrs.is_some()) * FIELDS_FLAG_ATTRS)
            | (u8::from(d.vertices.is_some()) * FIELDS_FLAG_VERTICES);
        bytes.push(flags);
        let attrs = d.attrs.as_deref().unwrap_or(&[]);
        bytes.extend_from_slice(&u16::try_from(attrs.len()).ok()?.to_be_bytes());
        for (n, v) in attrs {
            bytes.extend_from_slice(&n.to_be_bytes());
            bytes.extend_from_slice(&v.to_be_bytes());
        }
        let vertices = d.vertices.as_deref().unwrap_or(&[]);
        bytes.extend_from_slice(&u16::try_from(vertices.len()).ok()?.to_be_bytes());
        for v in vertices {
            bytes.extend_from_slice(&v.to_be_bytes());
        }
        if bytes.len() > MAX_FIELD_BASE_BYTES {
            return None;
        }
    }
    Some(crate::encoding::base64_encode(&bytes))
}

/// One object of a base: its lasting id's bytes, the layer's hash and the table's.
pub type BaseRow = ([u8; 16], u64, u64);

/// A base's rows read; none when they are not base64, not whole rows, not in strictly rising id order or too many.
/// One object's row of a base by its lasting id, without decoding the rest (docs/adr/0651 §1): the rows are sorted by
/// id, so a binary search decodes only the rows it probes. None when the base has no row of it or is not a base's
/// text (whitespace-free base64 of whole rows).
pub fn base_row_of(rows: &str, uid: &[u8; 16]) -> Option<BaseRow> {
    let t = rows.as_bytes();
    if !t.len().is_multiple_of(4) || t.iter().any(u8::is_ascii_whitespace) {
        return None;
    }
    let pad = t.iter().rev().take(2).filter(|c| **c == b'=').count();
    let bytes = (t.len() / 4 * 3).checked_sub(pad)?;
    if !bytes.is_multiple_of(BASE_ROW) {
        return None;
    }
    // The bytes [start, start + BASE_ROW) from the groups of four characters that hold them.
    let row_at = |k: usize| -> Option<BaseRow> {
        let start = k * BASE_ROW;
        let first = start / 3;
        let last = (start + BASE_ROW).div_ceil(3);
        let chunk = std::str::from_utf8(&t[first * 4..last * 4]).ok()?;
        let decoded = crate::encoding::base64_decode(chunk)?;
        let row = decoded.get(start - first * 3..start - first * 3 + BASE_ROW)?;
        let mut u = [0u8; 16];
        u.copy_from_slice(&row[..16]);
        let mut a = [0u8; 8];
        a.copy_from_slice(&row[16..24]);
        let mut b = [0u8; 8];
        b.copy_from_slice(&row[24..]);
        Some((u, u64::from_be_bytes(a), u64::from_be_bytes(b)))
    };
    let (mut lo, mut hi) = (0usize, bytes / BASE_ROW);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let row = row_at(mid)?;
        match row.0.cmp(uid) {
            std::cmp::Ordering::Equal => return Some(row),
            std::cmp::Ordering::Less => lo = mid + 1,
            std::cmp::Ordering::Greater => hi = mid,
        }
    }
    None
}

pub fn base_rows(rows: &str) -> Option<Vec<BaseRow>> {
    let bytes = crate::encoding::base64_decode(rows)?;
    if bytes.len() % BASE_ROW != 0 || bytes.len() / BASE_ROW > MAX_BASE_ROWS {
        return None;
    }
    let mut out: Vec<BaseRow> = Vec::with_capacity(bytes.len() / BASE_ROW);
    for row in bytes.chunks_exact(BASE_ROW) {
        let mut uid = [0u8; 16];
        uid.copy_from_slice(&row[..16]);
        let mut a = [0u8; 8];
        a.copy_from_slice(&row[16..24]);
        let mut b = [0u8; 8];
        b.copy_from_slice(&row[24..]);
        if out.last().is_some_and(|(last, _, _)| *last >= uid) {
            return None;
        }
        out.push((uid, u64::from_be_bytes(a), u64::from_be_bytes(b)));
    }
    Some(out)
}

/// The rows written as a base's text (in rising id order: they are sorted here).
pub fn base_text(mut rows: Vec<BaseRow>) -> String {
    rows.sort_by_key(|r| r.0);
    let mut bytes = Vec::with_capacity(rows.len() * BASE_ROW);
    for (uid, a, b) in rows {
        bytes.extend_from_slice(&uid);
        bytes.extend_from_slice(&a.to_be_bytes());
        bytes.extend_from_slice(&b.to_be_bytes());
    }
    crate::encoding::base64_encode(&bytes)
}

fn hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// What is wrong with the project's bases, if anything: each of a link the project has, at most one a link, its
/// revision and digest 64 lower-case hex digits, its rows readable (`base_rows`).
pub fn bases_problem(bases: &[SourceBase], sources: &[SourceLink]) -> Option<String> {
    let mut seen = std::collections::BTreeSet::new();
    for (i, b) in bases.iter().enumerate() {
        let at = format!("{}. taban", i + 1);
        if !sources.iter().any(|s| s.id == b.source) {
            return Some(format!("{at}: “{}” kimlikli bir bağ yok", b.source));
        }
        if !seen.insert(b.source.as_str()) {
            return Some(format!("{at}: “{}” bağının ikinci tabanı", b.source));
        }
        if !hex64(&b.revision) || !hex64(&b.digest) {
            return Some(format!("{at}: sürümü ya da özeti 64 küçük onaltılı hane değil"));
        }
        let Some(rows) = base_rows(&b.rows) else {
            return Some(format!(
                "{at}: satırları okunmuyor (base64, {BASE_ROW} baytlık satırlar, artan kimlik sırası, en çok {MAX_BASE_ROWS})"
            ));
        };
        if let Some(fields) = &b.fields
            && base_details(rows.len(), fields).is_none()
        {
            return Some(format!(
                "{at}: alan kayıtları okunmuyor (base64, satır başına bir kayıt, artan özet sırası, en çok {MAX_FIELD_BASE_BYTES} bayt)"
            ));
        }
    }
    None
}

#[cfg(test)]
mod base_row_tests {
    use super::*;

    /// docs/adr/0651 §1: a row found by binary search over the text is the row the whole decoding gives, for every
    /// count of rows (each of the three base64 alignments) and every id, and none for an id not there.
    #[test]
    fn a_row_is_found_without_decoding_the_rest() {
        for n in [0usize, 1, 2, 3, 4, 5, 17, 100] {
            let rows: Vec<BaseRow> = (0..n)
                .map(|i| {
                    let mut u = [0u8; 16];
                    u[0] = (i * 2 + 1) as u8;
                    u[15] = 7;
                    (u, i as u64 * 31 + 1, i as u64 * 17)
                })
                .collect();
            let text = base_text(rows.clone());
            assert_eq!(base_rows(&text).as_deref(), Some(rows.as_slice()));
            for r in &rows {
                assert_eq!(base_row_of(&text, &r.0), Some(*r), "{n} rows");
            }
            let mut missing = [0u8; 16];
            missing[0] = 2;
            assert_eq!(base_row_of(&text, &missing), None);
        }
        assert_eq!(base_row_of("abc", &[0; 16]), None);
    }
}
