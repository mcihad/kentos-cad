//! `cad.addresses.compare` (docs/adr/0736): a project's address data compared with an address register, read only.
//! The register is a table file (CSV or text, XLSX, ODS: the shared table reader's, İşlemler's limits) or a layer of the
//! drawing (its objects in the drawing's order, their attributes the columns, a point its position); its first row
//! names the columns. The locator is built as the Adres tab builds it ([`crate::addresses`]) and compared by the shared
//! core (`ops::geocode::register`). A neighbourhood or district column is compared only when the address data's doors
//! have that field. The doors are given by their objects' persistent ids, a layer's rows by theirs too.

use std::path::{Path, PathBuf};

use kentos_commands::{
    ADDRESS_COMPARE_LIMIT, ADDRESS_COMPARE_MAX_LIMIT, ADDRESS_COMPARE_MAX_TOLERANCE,
    ADDRESS_COMPARE_TOLERANCE,
    AddressDifference, AddressDistance, AddressPart, AddressPartDifference, AddressRegister,
    AddressRegisterRepeat, AddressUndrawn, AddressUnregistered, AddressesCompare,
    AddressesCompared, CommandResult, CommandWarning, Entity,
};
use kentos_domain::{Document, Slot};
use kentos_formats::table_file::{Limits, read_with};
use kentos_geometry_core::ops::geocode::register::{Part, RegisterRow, compare};
use kentos_geometry_core::vec2::Vec2;

use crate::ExecutionContext;
use crate::addresses;
use crate::addresses_check::uids_by_place;
use crate::checks::{Stop, error};
use crate::codes;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

fn count(n: usize) -> u32 {
    n.min(u32::MAX as usize) as u32
}

/// A number as a register writes it: an optional sign, digits with one decimal point (a lone comma is one), an
/// optional exponent; nothing else, and finite.
fn number(text: &str) -> Option<f64> {
    let t = text.trim();
    let t = if t.matches(',').count() == 1 && !t.contains('.') {
        t.replace(',', ".")
    } else {
        t.to_owned()
    };
    let b = t.as_bytes();
    let mut i = usize::from(matches!(b.first(), Some(b'+' | b'-')));
    let digits = |i: &mut usize| {
        let start = *i;
        while *i < b.len() && b[*i].is_ascii_digit() {
            *i += 1;
        }
        *i - start
    };
    let mut seen = digits(&mut i);
    if i < b.len() && b[i] == b'.' {
        i += 1;
        seen += digits(&mut i);
    }
    if seen == 0 {
        return None;
    }
    if i < b.len() && matches!(b[i], b'e' | b'E') {
        i += 1;
        if i < b.len() && matches!(b[i], b'+' | b'-') {
            i += 1;
        }
        if digits(&mut i) == 0 {
            return None;
        }
    }
    if i != b.len() {
        return None;
    }
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// The register read: its column names and its rows' cells (each a row of the register), with each row's object for a
/// layer; whether rows were cut.
struct Table {
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
    /// A layer's: each row's object and its point.
    objects: Vec<(String, Option<Vec2>)>,
    cut: bool,
    /// What a column is called in the words: a file's column, a layer's field.
    layer: Option<String>,
}

/// The register file's path: absolute, or from the drawing's folder.
fn register_path(text: &str, drawing: Option<&str>) -> Result<PathBuf, Stop> {
    let t = text.trim();
    if t.is_empty() {
        return Err(fail(
            codes::INVALID_FILE_PATH,
            "Adres kaydının dosya yolu boş; bir tablo dosyasının yolunu verin.".into(),
            "register.path",
        ));
    }
    let p = Path::new(t);
    if p.is_absolute() {
        return Ok(p.to_path_buf());
    }
    match drawing
        .map(str::trim)
        .map(Path::new)
        .filter(|d| d.is_absolute())
        .and_then(Path::parent)
    {
        Some(folder) => Ok(crate::source_places::resolve(folder, &t.replace('\\', "/"))),
        None => Err(fail(
            codes::INVALID_FILE_PATH,
            format!(
                "“{t}” göreli bir yol, ama çizimin dosyası yok; mutlak yolu verin ya da çizimi önce kaydedin."
            ),
            "register.path",
        )),
    }
}

fn file_table(path: &Path, sheet: Option<&str>) -> Result<Table, Stop> {
    if !path.is_file() {
        return Err(fail(
            codes::FILE_NOT_FOUND,
            format!("{} dosyası yok.", path.display()),
            "register.path",
        ));
    }
    let bytes = std::fs::read(path).map_err(|e| {
        fail(
            codes::REGISTER_UNREADABLE,
            format!("{} okunamadı: {e}", path.display()),
            "register.path",
        )
    })?;
    let read = read_with(&bytes, Limits::PROCESSING);
    if let Some(problem) = read.problem {
        return Err(fail(codes::REGISTER_UNREADABLE, problem, "register.path"));
    }
    let chosen = match sheet {
        None => read.sheets.into_iter().next(),
        Some(name) => {
            let names: Vec<String> = read.sheets.iter().filter_map(|s| s.name.clone()).collect();
            if names.is_empty() {
                return Err(fail(
                    codes::REGISTER_UNREADABLE,
                    "Adres kaydı tek sayfalı bir metin tablosu; sayfa adı verilmez.".into(),
                    "register.sheet",
                ));
            }
            match read
                .sheets
                .into_iter()
                .find(|s| s.name.as_deref() == Some(name))
            {
                Some(s) => Some(s),
                None => {
                    return Err(fail(
                        codes::REGISTER_UNREADABLE,
                        format!(
                            "Adres kaydında “{name}” sayfası yok; sayfaları: {}.",
                            names.join(", ")
                        ),
                        "register.sheet",
                    ));
                }
            }
        }
    };
    let Some(sheet) = chosen.filter(|s| !s.rows.is_empty()) else {
        return Err(fail(
            codes::REGISTER_UNREADABLE,
            "Adres kaydında satır yok.".into(),
            "register.path",
        ));
    };
    let mut rows = sheet.rows.into_iter();
    let columns: Vec<String> = rows
        .next()
        .unwrap_or_default()
        .into_iter()
        .map(|c| c.trim().to_owned())
        .collect();
    Ok(Table {
        columns,
        rows: rows.collect(),
        objects: Vec::new(),
        cut: sheet.cut,
        layer: None,
    })
}

fn layer_table(doc: &Document, layer: &str) -> Result<Table, Stop> {
    let Some(node) = doc.layers().get(layer) else {
        return Err(fail(
            codes::LAYER_NOT_FOUND,
            format!("“{layer}” katmanı çizimde yok."),
            "register.layer",
        ));
    };
    let columns = addresses::field_names(doc, layer);
    let mut objects: Vec<(u64, &Entity, String)> = doc
        .entities_with_uids()
        .filter(|(_, e)| e.base().layer_id == layer)
        .map(|(uid, e)| {
            let place = doc.place(Slot(e.base().id)).unwrap_or(u64::MAX);
            (place, e, uid.to_string())
        })
        .collect();
    objects.sort_by_key(|(place, _, _)| *place);
    let rows = objects
        .iter()
        .map(|(_, e, _)| {
            columns
                .iter()
                .map(|c| e.base().attrs.get(c).cloned().unwrap_or_default())
                .collect()
        })
        .collect();
    let objects = objects
        .into_iter()
        .map(|(_, e, uid)| {
            let at = match e {
                Entity::Point(p) => Some(Vec2::new(p.p.x, p.p.y)),
                _ => None,
            };
            (uid, at)
        })
        .collect();
    Ok(Table {
        columns,
        rows,
        objects,
        cut: false,
        layer: Some(node.name.clone()),
    })
}

/// A column's place in the table, or why there is none.
fn column(table: &Table, name: &str, path: &str) -> Result<usize, Stop> {
    let name = name.trim();
    if let Some(i) = table.columns.iter().position(|c| c == name) {
        return Ok(i);
    }
    let mut listed: Vec<&str> = table.columns.iter().take(20).map(String::as_str).collect();
    if table.columns.len() > 20 {
        listed.push("…");
    }
    let listed = if listed.is_empty() {
        "yok".to_owned()
    } else {
        listed.join(", ")
    };
    Err(fail(
        codes::REGISTER_COLUMN_MISSING,
        match &table.layer {
            Some(layer) => {
                format!("“{layer}” katmanında “{name}” alanı yok; alanları: {listed}.")
            }
            None => format!("Adres kaydında “{name}” sütunu yok; sütunları: {listed}."),
        },
        path,
    ))
}

/// The answer and its warnings; why not.
pub(crate) fn compare_in(
    doc: &Document,
    input: &AddressesCompare,
) -> Result<(AddressesCompared, Vec<CommandWarning>), Stop> {
    let limit = input.limit.unwrap_or(ADDRESS_COMPARE_LIMIT);
    if limit == 0 || limit > ADDRESS_COMPARE_MAX_LIMIT {
        return Err(fail(
            codes::INVALID_LIMIT,
            "Liste sınırı 1 ile 10 000 arasında bir tam sayı olmalı.".into(),
            "limit",
        ));
    }
    let limit = limit as usize;
    let tolerance = input.tolerance.unwrap_or(ADDRESS_COMPARE_TOLERANCE);
    if !(tolerance > 0.0 && tolerance <= ADDRESS_COMPARE_MAX_TOLERANCE) {
        return Err(fail(
            codes::INVALID_TOLERANCE,
            "Tolerans 0'dan büyük, en çok 10 000 m olmalı.".into(),
            "tolerance",
        ));
    }
    let reg: &AddressRegister = &input.register;
    let missing = || {
        fail(
            codes::REGISTER_MISSING,
            "Karşılaştırılacak adres kaydını verin: bir tablo dosyası (path) ya da çizimin bir katmanı (layer), ikisinden biri."
                .into(),
            "register",
        )
    };
    if reg.path.is_some() == reg.layer.is_some() {
        return Err(missing());
    }
    if reg.layer.is_some() && reg.sheet.is_some() {
        return Err(fail(
            codes::INVALID_INPUT,
            "Sayfa (sheet) yalnız tablo dosyasıyla verilir; katmandan okunan kayıtta sayfa yoktur.".into(),
            "register.sheet",
        ));
    }
    if reg.code.trim().is_empty() {
        return Err(fail(
            codes::INVALID_INPUT,
            "Adres kodunun sütununu (code) verin.".into(),
            "register.code",
        ));
    }
    let locators = &doc.settings().address_locators;
    let Some(first) = locators.first() else {
        return Err(Stop::failed(error(
            codes::NO_ADDRESS_DATA,
            "Projede adres verisi yok; önce Adres verisi penceresinde ya da cad.addresses.define ile tanımlayın."
                .to_owned(),
            None,
        )));
    };
    let locator = match &input.locator {
        None => first,
        Some(id) => addresses::locator(doc, id).ok_or_else(|| {
            fail(
                codes::UNKNOWN_LOCATOR,
                format!("Projede “{id}” kimlikli adres verisi yok."),
                "locator",
            )
        })?,
    };
    let Some(doors) = locator.doors.as_ref().filter(|d| d.code.is_some()) else {
        return Err(fail(
            codes::NO_ADDRESS_CODES,
            format!(
                "“{}” adres verisinin kapılarında adres kodu alanı yok; Adres verisi penceresinde kapıların Adres kodu alanını seçin.",
                locator.name
            ),
            "locator",
        ));
    };
    let table = match (&reg.path, &reg.layer) {
        (Some(path), None) => {
            file_table(&register_path(path, input.drawing.as_deref())?, reg.sheet.as_deref())?
        }
        (None, Some(layer)) => layer_table(doc, layer)?,
        _ => return Err(missing()),
    };
    let code = column(&table, &reg.code, "register.code")?;
    let named = |name: &Option<String>, path: &str| -> Result<Option<usize>, Stop> {
        name.as_deref()
            .map(|n| column(&table, n, path))
            .transpose()
    };
    let street = named(&reg.street, "register.street")?;
    let door = named(&reg.door, "register.door")?;
    let hood = named(&reg.neighbourhood, "register.neighbourhood")?;
    let district = named(&reg.district, "register.district")?;
    let x = named(&reg.x, "register.x")?;
    let y = named(&reg.y, "register.y")?;
    // A neighbourhood or a district the doors do not say is not compared.
    let hood = hood.filter(|_| doors.neighbourhood.is_some());
    let district = district.filter(|_| doors.district.is_some());
    let cell = |row: &[String], at: Option<usize>| -> Option<String> {
        at.map(|i| row.get(i).map(|c| c.trim().to_owned()).unwrap_or_default())
    };
    let mut rows = Vec::new();
    let mut uncoded = 0usize;
    for (i, row) in table.rows.iter().enumerate() {
        let code_text = row.get(code).map(|c| c.trim()).unwrap_or_default();
        if code_text.is_empty() {
            uncoded += 1;
            continue;
        }
        let at = match (x, y) {
            (Some(xi), Some(yi)) => {
                match (
                    row.get(xi).and_then(|t| number(t)),
                    row.get(yi).and_then(|t| number(t)),
                ) {
                    (Some(px), Some(py)) => Some(Vec2::new(px, py)),
                    _ => None,
                }
            }
            _ => table.objects.get(i).and_then(|(_, at)| *at),
        };
        rows.push(RegisterRow {
            row: count(i + 1),
            code: code_text.to_owned(),
            street: cell(row, street),
            door: cell(row, door),
            neighbourhood: cell(row, hood),
            district: cell(row, district),
            at,
        });
    }
    let found = compare(&addresses::build(doc, locator), &rows, tolerance);
    let by_place = uids_by_place(doc);
    let uid_of = |order: u32| by_place.get(&order).cloned().unwrap_or_default();
    let row_uids = |rows: &[u32]| -> Vec<String> {
        rows.iter()
            .filter_map(|r| table.objects.get((*r as usize).wrapping_sub(1)))
            .map(|(uid, _)| uid.clone())
            .collect()
    };
    let part = |p: Part| match p {
        Part::Street => AddressPart::Street,
        Part::Door => AddressPart::Door,
        Part::Neighbourhood => AddressPart::Neighbourhood,
        Part::District => AddressPart::District,
    };
    let mut warnings = Vec::new();
    if table.cut {
        warnings.push(CommandWarning {
            code: codes::REGISTER_CUT.into(),
            message: format!(
                "Adres kaydının ilk {} satırı karşılaştırıldı; gerisi okunmadı.",
                kentos_formats::table_file::PROCESSING_ROWS
            ),
            path: Some("register.path".into()),
        });
    }
    Ok((
        AddressesCompared {
            locator: locator.id.clone(),
            doors: count(found.doors),
            coded: count(found.coded),
            matched: count(found.matched),
            rows: count(rows.len()),
            uncoded_rows: count(uncoded),
            unregistered: count(found.unregistered.len()),
            unregistered_doors: found
                .unregistered
                .iter()
                .take(limit)
                .map(|u| AddressUnregistered {
                    uid: uid_of(u.order),
                    code: u.code.clone(),
                    street: u.street.clone(),
                    door: u.door.clone(),
                })
                .collect(),
            differing: count(found.differences.len()),
            differences: found
                .differences
                .iter()
                .take(limit)
                .map(|d| AddressDifference {
                    uid: uid_of(d.order),
                    code: d.code.clone(),
                    row: d.row,
                    parts: d
                        .parts
                        .iter()
                        .map(|p| AddressPartDifference {
                            part: part(p.part),
                            drawing: p.drawing.clone(),
                            register: p.register.clone(),
                        })
                        .collect(),
                })
                .collect(),
            distant: count(found.distant.len()),
            distances: found
                .distant
                .iter()
                .take(limit)
                .map(|d| AddressDistance {
                    uid: uid_of(d.order),
                    code: d.code.clone(),
                    row: d.row,
                    distance: d.distance,
                })
                .collect(),
            undrawn: count(found.undrawn.len()),
            undrawn_addresses: found
                .undrawn
                .iter()
                .take(limit)
                .map(|u| AddressUndrawn {
                    street: u.street.clone(),
                    door: u.door.clone(),
                    neighbourhood: u.neighbourhood.clone(),
                    codes: u.codes.clone(),
                    rows: u.rows.clone(),
                    uids: row_uids(&u.rows),
                })
                .collect(),
            repeated: count(found.repeats.len()),
            register_repeats: found
                .repeats
                .iter()
                .take(limit)
                .map(|r| AddressRegisterRepeat {
                    code: r.code.clone(),
                    rows: r.rows.clone(),
                    uids: row_uids(&r.rows),
                })
                .collect(),
            revision: doc.revision().to_string(),
        },
        warnings,
    ))
}

/// The answer on a drawing read only (a window asks without a context).
pub fn plan_in(doc: &Document, input: &AddressesCompare) -> CommandResult<AddressesCompared> {
    match compare_in(doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &AddressesCompare) -> CommandResult<()> {
    match compare_in(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed {
            output: (),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &AddressesCompare) -> CommandResult<AddressesCompared> {
    plan_in(cx.doc, input)
}

/// Read-only: the same answer as the plan.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: AddressesCompare,
) -> CommandResult<AddressesCompared> {
    plan(cx, &input)
}

#[cfg(test)]
mod tests {
    use super::number;

    #[test]
    fn a_register_writes_numbers_plainly() {
        assert_eq!(number(" 500010.25 "), Some(500_010.25));
        assert_eq!(number("500010,25"), Some(500_010.25));
        assert_eq!(number("-1e3"), Some(-1000.0));
        assert_eq!(number(".5"), Some(0.5));
        for bad in ["", "1.000,5", "1_000", "inf", "NaN", "12a", "1e", "+", "1,2,3"] {
            assert_eq!(number(bad), None, "{bad}");
        }
    }
}
