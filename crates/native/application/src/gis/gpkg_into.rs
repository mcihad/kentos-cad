//! A delivery written into a GeoPackage that is there (docs/adr/9191): only its tables written again, or its rows
//! added to the file's tables of the same names, the file's other tables untouched.
//!
//! The caller writes into a copy of the file (`copy`: SQLite's online backup, page for page, so what another program
//! holds in a write-ahead log is copied too and no table's row ids change) and puts it in place at the end: a stopped
//! or failed write leaves the file as it was. Rows
//! are added only where the table can hold them: the same geometry family, multi-part and curves where the column
//! takes them, the same system, Z and M as the column says, every NOT NULL column filled. A layer's attribute the
//! table has no column for is left out and said; an object whose lasting id the table has is not added again.

use std::collections::{BTreeSet, HashSet};
use std::path::Path;

use kentos_domain::Document;
use kentos_geopackage as gpkg;
use kentos_geopackage::GeoPackage;

use crate::codes;
use crate::geopackage::{BATCH_ROWS, Clash, Flow, Row, TableWritten, WriteStop, Written, row_of, write_in};

/// A layer's rows added to a table of the file's: the table as the file has it, the columns written with the attribute
/// each reads, the lasting ids it has, the attributes it has no column for.
#[derive(Clone, Debug)]
pub(crate) struct Append {
    pub have: gpkg::Layer,
    pub columns: Vec<gpkg::Column>,
    pub keys: Vec<String>,
    pub uids: HashSet<String>,
    pub left_out: Vec<String>,
}

/// Copies the GeoPackage at `from` to `to` (a file not there) by SQLite's online backup: page for page, what another
/// program wrote and holds in a write-ahead log in the copy too, every table's rows with their ids as they are.
/// Refused for a file that does not read as a GeoPackage.
pub fn copy(from: &Path, to: &Path) -> Result<(), String> {
    let gp = GeoPackage::open(from, false).map_err(|e| e.0)?;
    gp.layers().map_err(|e| e.0)?;
    gp.copy_to(to).map_err(|e| e.0)
}

/// The layers written into the GeoPackage at `path` (docs/adr/9191), their tables in one transaction:
/// `Clash::Replace` writes them again, `Clash::Append` adds their rows to the file's tables of the same names; a table
/// the file lacks is made, the file's other tables are not touched. A relation the file declares on a table written
/// again goes (its mapping rows were the old rows'); the layers' own relations and looks are written for the tables
/// written anew, an appended table keeps the file's. `path` is a copy the caller puts in place.
pub fn write_into(
    doc: &Document,
    layers: &[String],
    path: &Path,
    clash: Clash,
    flow: &Flow<'_>,
) -> Result<Written, WriteStop> {
    let mut gp = GeoPackage::open(path, true)?;
    let side = gp.side_tables()?;
    let before: Vec<String> = gp
        .layers()?
        .into_iter()
        .map(|l| l.name)
        .filter(|n| !side.contains(n))
        .collect();
    let session = gp.session()?;
    let mut out = write_in(doc, layers, &session, Some(clash), BATCH_ROWS, flow)?;
    session.commit()?;
    let replaced: BTreeSet<String> =
        out.tables.iter().filter(|t| t.replaced).map(|t| t.table.to_lowercase()).collect();
    let mut gone = Vec::new();
    for r in gp.relations()? {
        if replaced.contains(&r.base_table.to_lowercase()) || replaced.contains(&r.related_table.to_lowercase()) {
            gp.drop_relation(&r)?;
            gone.push(r);
        }
    }
    let fresh: Vec<TableWritten> = out.tables.iter().filter(|t| !t.appended).cloned().collect();
    let mut profile =
        crate::gpkg_profile::write_profile(doc, &fresh, &mut gp, &crate::gis_derive::utc_now()).map_err(WriteStop::Failed)?;
    // The project's relations with an appended side: the file's table keeps its own (its mapping is its rows').
    let appended: BTreeSet<&str> = out.tables.iter().filter(|t| t.appended).filter_map(|t| t.layer.as_deref()).collect();
    let written: BTreeSet<&str> = out.tables.iter().filter_map(|t| t.layer.as_deref()).collect();
    for r in &doc.settings().relations {
        let (child, parent) = (r.child.as_str(), r.parent.as_str());
        if !(appended.contains(child) || appended.contains(parent)) || !(written.contains(child) && written.contains(parent)) {
            continue;
        }
        let said = format!("“{}” ilişkisi yazılmadı:", r.name);
        profile.left.retain(|l| !l.starts_with(&said));
        profile.left.push(format!(
            "{said} satırları var olan tabloya eklenen katmanın ilişkileri dosyadakilerdir, eşleme eklenmez"
        ));
    }
    for r in gone {
        let again = profile.relations.iter().any(|(_, base, related, _)| {
            base.eq_ignore_ascii_case(&r.base_table) && related.eq_ignore_ascii_case(&r.related_table)
        });
        if !again {
            profile.left.push(format!(
                "dosyanın “{}” ile “{}” arasındaki ilişkisi çıktı: tablo yeniden yazıldı, eşlemesi eski satırlara gidiyordu",
                r.base_table, r.related_table
            ));
        }
    }
    out.profile = profile;
    let touched: BTreeSet<String> = out.tables.iter().map(|t| t.table.to_lowercase()).collect();
    out.kept = before.into_iter().filter(|t| !touched.contains(&t.to_lowercase())).collect();
    gp.close()?;
    Ok(out)
}

/// A geometry column's type as (family: 0 point, 1 line, 2 surface, none any; multi-part taken; curves taken). A
/// collection column takes no layer's rows: `Some(3)`.
fn shape(type_name: &str) -> (Option<u8>, bool, bool) {
    match type_name.to_ascii_uppercase().as_str() {
        "POINT" => (Some(0), false, false),
        "MULTIPOINT" => (Some(0), true, false),
        "LINESTRING" => (Some(1), false, false),
        "MULTILINESTRING" => (Some(1), true, false),
        "COMPOUNDCURVE" | "CURVE" => (Some(1), false, true),
        "MULTICURVE" => (Some(1), true, true),
        "POLYGON" => (Some(2), false, false),
        "MULTIPOLYGON" => (Some(2), true, false),
        "CURVEPOLYGON" | "SURFACE" => (Some(2), false, true),
        "MULTISURFACE" => (Some(2), true, true),
        "GEOMETRY" => (None, true, true),
        // A circular string column takes only arcs, a collection column only collections: no layer's rows are made so.
        _ => (Some(3), true, true),
    }
}

fn family_words(family: Option<u8>) -> &'static str {
    match family {
        Some(0) => "nokta",
        Some(1) => "çizgi",
        Some(2) => "alan",
        Some(_) => "özel geometri",
        None => "karışık geometri",
    }
}

fn unfit(code: &'static str, message: String) -> WriteStop {
    WriteStop::Unfit { code, message }
}

/// Why `rows` (the layer's column type: [`crate::geopackage`]'s `Families`' name) cannot go into a column of `have`;
/// none when they can.
fn geometry_unfit(have: &str, rows: &str) -> Option<String> {
    let (tf, tm, tc) = shape(have);
    let (rf, rm, rc) = shape(rows);
    if tf.is_none() {
        return None;
    }
    if rf != tf {
        return Some(format!(
            "tablo {} geometri tutuyor, katmanın nesneleri {}",
            family_words(tf),
            family_words(rf)
        ));
    }
    if rm && !tm {
        return Some("katmanda çok parçalı nesneler var, tablonun sütunu tek parçalı".to_owned());
    }
    if rc && !tc {
        return Some("katmanda yay içeren nesneler var, tablonun sütunu yalnız doğru parçalı".to_owned());
    }
    None
}

/// Why rows whose Z (or M) `rows` (0 none, 1 every row, 2 some rows) cannot go into a column saying `have` (0
/// prohibited, 1 mandatory, 2 optional); none when they can.
fn dims_unfit(have: u8, rows: u8, what: &str) -> Option<String> {
    match (have, rows) {
        (0, 1 | 2) => Some(format!("katmanda {what} değeri olan nesneler var, tablonun sütunu {what} almıyor")),
        (1, 0 | 2) => Some(format!("tablonun sütunu her satırda {what} istiyor, katmanda {what} değeri olmayan nesneler var")),
        _ => None,
    }
}

/// A system's words: its organization and code (EPSG:5253), else its id.
fn system_words(session: &gpkg::Session<'_>, id: Option<i32>) -> Result<String, WriteStop> {
    let Some(id) = id else {
        return Ok("tanımsız".to_owned());
    };
    Ok(match session.srs(id)? {
        Some(s) if s.organization_id > 0 => format!("{}:{}", s.organization.to_ascii_uppercase(), s.organization_id),
        _ => format!("{id} kimlikli sistem"),
    })
}

/// A layer's rows checked against the file's `table` they would be added to (docs/adr/9191): the columns they fill,
/// the lasting ids the table has (and how many of the layer's objects it has already), or why they cannot go in.
/// `type_name`, `z`, `m`, `columns` and `keys` are the layer's table as it would be written new.
#[allow(clippy::too_many_arguments)]
pub(crate) fn append_plan(
    session: &gpkg::Session<'_>,
    doc: &Document,
    layer: &str,
    table: &str,
    type_name: Option<&str>,
    z: u8,
    m: u8,
    columns: &[gpkg::Column],
    keys: &[String],
    srs_id: i32,
) -> Result<(Append, u32), WriteStop> {
    let name = doc.layers().get(layer).map_or_else(|| layer.to_owned(), |n| n.name.clone());
    let Some(have) = session.table(table)? else {
        return Err(WriteStop::Failed(format!("“{table}” tablosu dosyada okunamadı")));
    };
    let again = "Tabloyu yeniden yazmayı seçin ya da başka bir dosyaya yazın.";
    match (&have.geometry, type_name) {
        (None, Some(_)) => {
            return Err(unfit(
                codes::GEOMETRY_MISMATCH,
                format!("“{table}” dosyada geometrisiz bir tablo; “{name}” katmanının geometrileri ona eklenemez. {again}"),
            ));
        }
        (Some(g), Some(rows)) => {
            let why = geometry_unfit(&g.type_name, rows)
                .or_else(|| dims_unfit(g.z, z, "Z"))
                .or_else(|| dims_unfit(g.m, m, "M"));
            if let Some(why) = why {
                return Err(unfit(
                    codes::GEOMETRY_MISMATCH,
                    format!("“{name}” katmanının satırları “{table}” tablosuna eklenemez: {why}. {again}"),
                ));
            }
            if have.srs_id != Some(srs_id) {
                return Err(unfit(
                    codes::CRS_MISMATCH,
                    format!(
                        "“{table}” tablosunun sistemi {}, çizimin {}; başka sistemdeki tabloya satır eklenmez. {again}",
                        system_words(session, have.srs_id)?,
                        system_words(session, Some(srs_id))?
                    ),
                ));
            }
        }
        _ => {}
    }
    // Each column the layer's attribute goes to: the table's of the same name (case aside) that takes values.
    let mut out_columns = Vec::new();
    let mut out_keys = Vec::new();
    let mut left_out = Vec::new();
    for (c, k) in columns.iter().zip(keys) {
        match have
            .columns
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case(&c.name) && gpkg::Making::of(h).written())
        {
            Some(h) if !out_columns.iter().any(|o: &gpkg::Column| o.name == h.name) => {
                out_columns.push(h.clone());
                out_keys.push(k.clone());
            }
            _ => left_out.push(k.clone()),
        }
    }
    let uids = session.uids(table)?;
    let objects: Vec<_> = doc
        .by_layer_with_uids(layer)
        .filter(|(_, e)| !matches!(row_of(e), Row::Left(_)))
        .collect();
    let already = objects.iter().filter(|(u, _)| uids.contains(&u.to_string())).count() as u32;
    // A NOT NULL column without a default: every row added fills it.
    for h in have.columns.iter().filter(|h| h.not_null && h.default.is_none() && gpkg::Making::of(h) == gpkg::Making::Plain) {
        let Some(i) = out_columns.iter().position(|o| o.name == h.name) else {
            return Err(unfit(
                codes::SOURCE_NOT_NULL,
                format!(
                    "“{table}” tablosunun “{}” sütunu boş geçilemez (NOT NULL) ve “{name}” katmanında karşılığı yok; satırlar eklenemez. {again}",
                    h.name
                ),
            ));
        };
        let empty = objects
            .iter()
            .filter(|(u, _)| !uids.contains(&u.to_string()))
            .filter(|(_, e)| e.base().attrs.get(&out_keys[i]).is_none_or(|v| v.trim().is_empty()))
            .count();
        if empty > 0 {
            return Err(unfit(
                codes::SOURCE_NOT_NULL,
                format!(
                    "“{table}” tablosunun “{}” sütunu boş geçilemez (NOT NULL); “{name}” katmanının {empty} nesnesinde “{}” boş. Değerlerini girin ya da tabloyu yeniden yazmayı seçin.",
                    h.name, out_keys[i]
                ),
            ));
        }
    }
    Ok((
        Append {
            have,
            columns: out_columns,
            keys: out_keys,
            uids,
            left_out,
        },
        already,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_column_takes_its_family_and_multi_parts_and_curves_only_where_it_says() {
        assert_eq!(geometry_unfit("MULTIPOLYGON", "POLYGON"), None);
        assert_eq!(geometry_unfit("MULTIPOLYGON", "MULTIPOLYGON"), None);
        assert_eq!(geometry_unfit("GEOMETRY", "POINT"), None);
        assert_eq!(geometry_unfit("MULTICURVE", "LINESTRING"), None);
        assert_eq!(geometry_unfit("COMPOUNDCURVE", "LINESTRING"), None);
        assert!(geometry_unfit("POLYGON", "POINT").is_some_and(|w| w.contains("alan") && w.contains("nokta")));
        assert!(geometry_unfit("POLYGON", "MULTIPOLYGON").is_some_and(|w| w.contains("çok parçalı")));
        assert!(geometry_unfit("LINESTRING", "COMPOUNDCURVE").is_some_and(|w| w.contains("yay")));
        assert!(geometry_unfit("POINT", "GEOMETRY").is_some(), "mixed rows into one family");
        assert!(geometry_unfit("GEOMETRYCOLLECTION", "POINT").is_some());
    }

    #[test]
    fn z_and_m_go_only_where_the_column_says() {
        assert_eq!(dims_unfit(2, 1, "Z"), None);
        assert_eq!(dims_unfit(0, 0, "Z"), None);
        assert_eq!(dims_unfit(1, 1, "Z"), None);
        assert!(dims_unfit(0, 2, "Z").is_some());
        assert!(dims_unfit(1, 0, "M").is_some());
    }
}
