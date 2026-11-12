//! A bound table's columns its layer's fields mirror (docs/adr/0753): the default the database works out for a new row
//! (`column.made`), and a single-column foreign key, whose referenced table's rows become the field's value list
//! (`column.refers`, `values`) as QGIS's Relation Reference offers a parent's rows by its display expression. Binding the
//! table writes them, every reading of it writes them again; a field with a list of its own (or a project's list) keeps
//! it. The independent reference is `scripts/fixtures/source_references_cases.py`.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use kentos_commands::{
    ColumnRefers, CommandWarning, FieldChoice, FieldColumn, LayerField, LayerFieldKind, MAX_MADE,
    MAX_REFERS_NAME, TableCheck, check_value, field_problem, fold, js_trim,
};
use kentos_forms::table_checks::{self, Dialect};
use kentos_geometry_core::text::natural::natural_cmp;
use kentos_geopackage::{self as gpkg, GeoPackage};

use crate::codes;
use crate::geopackage::literal_default;
use crate::provider::{self, ProviderError, Reference, label_column};

/// The most rows of a referenced table a field's list takes (docs/adr/0753 §2); a table with more gives none.
pub(crate) const MOST_REFERENCE_ROWS: usize = 10_000;
/// [`MOST_REFERENCE_ROWS`] as a sentence says it.
const MOST_SAID: &str = "10 000";

/// A GeoPackage table's single-column foreign keys with the referenced tables' rows.
pub(crate) fn of_file(path: &Path, table: &str) -> Result<Vec<Reference>, String> {
    let file = GeoPackage::open(path, false).map_err(|e| e.0)?;
    let keys = file.foreign_keys(table).map_err(|e| e.0)?;
    let mut out = Vec::with_capacity(keys.len());
    for k in keys {
        let label = label_column(&k.texts);
        let rows = file
            .reference_rows(&k.table, &k.to, label.as_deref(), MOST_REFERENCE_ROWS)
            .map_err(|e| e.0)?;
        out.push(Reference {
            column: k.column,
            table: k.table,
            to: k.to,
            label,
            rows: (rows.len() <= MOST_REFERENCE_ROWS).then_some(rows),
        });
    }
    Ok(out)
}

/// A database table's, by the host's provider; none where the host registered none.
pub(crate) fn of_database(address: &str, table: &str) -> Result<Vec<Reference>, String> {
    let Some(p) = provider::postgis() else {
        return Ok(Vec::new());
    };
    p.references(address, table, MOST_REFERENCE_ROWS)
        .map_err(|e| match e {
            ProviderError::Unreachable(w)
            | ProviderError::NoTable(w)
            | ProviderError::Refused(w)
            | ProviderError::Unreadable(w) => w,
            other => format!("{other:?}"),
        })
}

/// Whether a default is NULL (`NULL`, `(NULL)`, PostgreSQL's `NULL::character varying`).
fn is_null(sql: &str) -> bool {
    let mut t = sql.trim();
    while t.len() >= 2 && t.starts_with('(') && t.ends_with(')') {
        t = t[1..t.len() - 1].trim();
    }
    let up = t.to_ascii_uppercase();
    up == "NULL" || up.starts_with("NULL::")
}

/// The default the database works out for a column, as its text (docs/adr/0753 §1), or the clause of a column it makes
/// itself (an identity, a generated column; docs/adr/0754): none without a default, for NULL, and for a literal (the
/// field's own default, docs/adr/0750); a run of blanks and control characters one space (a generation expression
/// written over lines reads on one), a text past [`MAX_MADE`] characters shortened with “…”.
pub(crate) fn made_of(c: &gpkg::Column) -> Option<String> {
    let d = c.default.as_deref()?;
    if gpkg::Making::of(c) == gpkg::Making::Plain && (literal_default(d).is_some() || is_null(d)) {
        return None;
    }
    let mut plain = String::with_capacity(d.len());
    for ch in d.chars() {
        if ch.is_whitespace() || ch.is_control() {
            if !plain.ends_with(' ') {
                plain.push(' ');
            }
        } else {
            plain.push(ch);
        }
    }
    let text = js_trim(&plain);
    if text.is_empty() {
        return None;
    }
    if text.chars().count() <= MAX_MADE {
        return Some(text.to_owned());
    }
    let cut: String = text.chars().take(MAX_MADE - 1).collect();
    Some(format!("{}…", js_trim(&cut)))
}

/// Whether a reference's names fit a field's column (docs/adr/0753 §1).
fn names_fit(r: &Reference) -> bool {
    let fits = |t: &str| {
        !js_trim(t).is_empty()
            && js_trim(t) == t
            && t.chars().count() <= MAX_REFERS_NAME
            && !t.chars().any(char::is_control)
    };
    fits(&r.table) && fits(&r.to) && r.label.as_deref().is_none_or(fits)
}

/// Whether a field's kind holds a value list: a text or a number, not a multiple one.
fn takes_list(f: &LayerField) -> bool {
    !f.multiple
        && matches!(
            f.kind,
            LayerFieldKind::Text
                | LayerFieldKind::Integer
                | LayerFieldKind::BigInt
                | LayerFieldKind::Decimal
        )
}

/// A referenced table's rows as `f`'s value list (docs/adr/0753 §2), and how many were left out. The rows in the natural
/// order of their keys, then their labels; each key in the field's kind (one that does not fit left out, a key met again
/// once), its label trimmed (an empty or missing one the code). A label no other repeats, folded the Turkish way, stays;
/// one that repeats says its code beside it, again while that is taken. The list in the natural order of its labels,
/// then its codes.
pub(crate) fn listed(
    f: &LayerField,
    rows: &[(String, Option<String>)],
) -> (Vec<FieldChoice>, usize) {
    let mut probe = LayerField::new(f.name.clone(), f.kind);
    probe.length = f.length;
    probe.scale = f.scale;
    let mut rows: Vec<&(String, Option<String>)> = rows.iter().collect();
    rows.sort_by(|a, b| {
        natural_cmp(&a.0, &b.0).then_with(|| {
            natural_cmp(
                a.1.as_deref().unwrap_or_default(),
                b.1.as_deref().unwrap_or_default(),
            )
        })
    });
    let mut left = 0;
    let mut codes = HashSet::with_capacity(rows.len());
    let mut items: Vec<(String, String)> = Vec::with_capacity(rows.len());
    for (key, label) in rows {
        match check_value(&probe, key) {
            Ok(code) if !code.is_empty() => {
                if codes.insert(code.clone()) {
                    let label = label
                        .as_deref()
                        .map(js_trim)
                        .filter(|l| !l.is_empty())
                        .unwrap_or(&code)
                        .to_owned();
                    items.push((code, label));
                }
            }
            _ => left += 1,
        }
    }
    let mut count: HashMap<String, usize> = HashMap::with_capacity(items.len());
    for (_, l) in &items {
        *count.entry(fold(l)).or_default() += 1;
    }
    let mut taken: HashSet<String> = count
        .iter()
        .filter(|(_, n)| **n == 1)
        .map(|(l, _)| l.clone())
        .collect();
    let mut out = Vec::with_capacity(items.len());
    for (code, l) in items {
        let label = if count[&fold(&l)] == 1 {
            l
        } else {
            let mut label = format!("{l} ({code})");
            while !taken.insert(fold(&label)) {
                label = format!("{label} ({code})");
            }
            label
        };
        out.push(FieldChoice::new(code, label));
    }
    out.sort_by(|a, b| natural_cmp(&a.label, &b.label).then_with(|| natural_cmp(&a.code, &b.code)));
    (out, left)
}

fn warning(message: String, at: &str) -> CommandWarning {
    CommandWarning {
        code: codes::REFERENCE_VALUES_MISSING.into(),
        message,
        path: Some(at.to_owned()),
    }
}

/// The values a one-column CHECK of the table ties `f` to (docs/adr/0755 §4) as its list, with the constraint's name:
/// each value one the field takes as it is (a text trimmed and not empty, a number of the field's kind), else none.
fn check_list(
    f: &LayerField,
    checks: &[TableCheck],
    d: Dialect,
) -> Option<(Vec<FieldChoice>, String)> {
    if !takes_list(f) {
        return None;
    }
    let mut probe = LayerField::new(f.name.clone(), f.kind);
    probe.length = f.length;
    probe.scale = f.scale;
    checks.iter().find_map(|c| {
        let (column, codes) = table_checks::value_list(c, d, f.kind)?;
        (column == f.name).then_some(())?;
        let fits = codes.iter().all(|code| match check_value(&probe, code) {
            Ok(v) if f.kind == LayerFieldKind::Text => v == *code && !v.is_empty(),
            Ok(v) => !v.is_empty(),
            Err(_) => false,
        });
        if !fits || c.said().chars().count() > kentos_commands::MAX_CHECK_SQL {
            return None;
        }
        let rows: Vec<(String, Option<String>)> = codes.into_iter().map(|c| (c, None)).collect();
        let (list, _) = listed(f, &rows);
        (!list.is_empty()).then(|| (list, c.said().to_owned()))
    })
}

/// The table's own fields (as they are read from its columns) with what its columns say besides (docs/adr/0753 §1):
/// each column's made default, and each single-column foreign key's table with its rows as the field's value list, when
/// the field's kind holds one; without a key, a one-column CHECK's list of values (docs/adr/0755 §4). `refs` an error:
/// the references could not be read (said; no field gets one). The warnings, at `at`: lists not made or short.
pub(crate) fn table_fields(
    fields: &mut [LayerField],
    columns: &[gpkg::Column],
    refs: &Result<Vec<Reference>, String>,
    checks: &[TableCheck],
    d: Dialect,
    table: &str,
    at: &str,
) -> Vec<CommandWarning> {
    let mut warnings = Vec::new();
    if let Err(why) = refs {
        warnings.push(warning(
            format!(
                "“{table}” tablosunun başvurduğu tablolar okunamadı ({why}); alanlar değer listesi almadı."
            ),
            at,
        ));
    }
    let refs = refs.as_deref().unwrap_or_default();
    for f in fields.iter_mut() {
        let made = columns.iter().find(|c| c.name == f.name).and_then(made_of);
        let mut refers = None;
        let mut values = None;
        if let Some(r) = refs
            .iter()
            .find(|r| r.column == f.name)
            .filter(|r| names_fit(r))
        {
            refers = Some(ColumnRefers {
                table: r.table.clone(),
                column: r.to.clone(),
                label: r.label.clone(),
            });
            if takes_list(f) {
                match &r.rows {
                    None => warnings.push(warning(
                        format!(
                            "“{}” alanının değerleri “{}” tablosundan listelenmedi: tabloda {MOST_SAID} satırdan çok var; alan serbest kalır.",
                            f.name, r.table
                        ),
                        at,
                    )),
                    Some(rows) => {
                        let (list, left) = listed(f, rows);
                        if left > 0 {
                            warnings.push(warning(
                                format!(
                                    "“{}” alanının değer listesine “{}” tablosunun {left} satırı girmedi: anahtarları alanın türüne uymuyor.",
                                    f.name, r.table
                                ),
                                at,
                            ));
                        }
                        values = (!list.is_empty()).then_some(list);
                    }
                }
            }
        }
        let mut check = None;
        if refers.is_none()
            && let Some((list, said)) = check_list(f, checks, d)
        {
            values = Some(list);
            check = Some(said);
        }
        let mirrored = refers.is_some() || check.is_some();
        f.column = (made.is_some() || mirrored).then_some(FieldColumn {
            made,
            refers,
            check,
        });
        if mirrored {
            f.values = values;
        }
    }
    warnings
}

/// A layer's field after a reading of its table (docs/adr/0753 §1), `t` the table's (from [`table_fields`]): the table's
/// made default; its foreign key's table and list, or its CHECK's list (docs/adr/0755 §4), when the field holds that list
/// already or has none of its own (nor a project's), the list dropped with the key or the constraint. `known`: the
/// references and the constraints were read (else the field's stay). A field the mirror would make wrong stays as it was.
pub(crate) fn follow(f: &mut LayerField, t: &LayerField, known: bool) {
    let before = f.clone();
    let mirrored = f
        .column
        .as_ref()
        .is_some_and(|c| c.refers.is_some() || c.check.is_some());
    let own = !mirrored && (f.values.is_some() || f.domain.is_some());
    let made = t.column.as_ref().and_then(|c| c.made.clone());
    let from = if !known || own { &f.column } else { &t.column };
    let refers = from.as_ref().and_then(|c| c.refers.clone());
    let check = from.as_ref().and_then(|c| c.check.clone());
    if known && !own && (mirrored || refers.is_some() || check.is_some()) {
        let same = f.kind == t.kind && f.length == t.length && f.scale == t.scale;
        f.values = if (refers.is_some() || check.is_some()) && same && takes_list(f) {
            t.values.clone()
        } else {
            None
        };
    }
    f.column = (made.is_some() || refers.is_some() || check.is_some()).then_some(FieldColumn {
        made,
        refers,
        check,
    });
    if field_problem(f).is_some() {
        *f = before;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(r: &[(&str, Option<&str>)]) -> Vec<(String, Option<String>)> {
        r.iter()
            .map(|(k, l)| ((*k).to_owned(), l.map(str::to_owned)))
            .collect()
    }

    fn pairs(list: &[FieldChoice]) -> Vec<(&str, &str)> {
        list.iter()
            .map(|c| (c.code.as_str(), c.label.as_str()))
            .collect()
    }

    #[test]
    fn a_list_is_in_its_labels_order_with_repeats_told_apart() {
        let f = LayerField::new("il_id", LayerFieldKind::BigInt);
        let (list, left) = listed(
            &f,
            &rows(&[
                ("58", Some("Sivas")),
                ("6", Some("Ankara")),
                ("34", Some(" İstanbul ")),
                ("07", Some("Antalya")),
                ("7", Some("Tekrar")),
                ("x", Some("Yanlış")),
                ("9", None),
                ("10", Some("sivas")),
            ]),
        );
        assert_eq!(left, 1, "x is no number");
        assert_eq!(
            pairs(&list),
            [
                ("9", "9"),
                ("6", "Ankara"),
                ("7", "Antalya"),
                ("34", "İstanbul"),
                ("10", "sivas (10)"),
                ("58", "Sivas (58)")
            ]
        );
        assert!(
            field_problem(&LayerField {
                values: Some(list),
                ..f
            })
            .is_none()
        );
    }

    #[test]
    fn a_label_still_taken_says_its_code_again() {
        let f = LayerField::new("ust", LayerFieldKind::Text);
        let (list, _) = listed(
            &f,
            &rows(&[("A", Some("Ev")), ("B", Some("Ev")), ("C", Some("Ev (A)"))]),
        );
        assert_eq!(
            pairs(&list),
            [("C", "Ev (A)"), ("A", "Ev (A) (A)"), ("B", "Ev (B)")]
        );
        assert!(
            field_problem(&LayerField {
                values: Some(list),
                ..f
            })
            .is_none()
        );
    }

    /// docs/adr/0753's timing: 10 000 rows made a list, every tenth label repeated (`cargo test --release -p
    /// kentos-native-application --lib references::tests::timing -- --ignored --nocapture`).
    #[test]
    #[ignore = "a timing, run by hand"]
    fn timing() {
        let rows: Vec<(String, Option<String>)> = (1..=10_000)
            .map(|i| {
                let label = if i % 10 == 0 {
                    format!("İl {}", i - 1)
                } else {
                    format!("İl {i}")
                };
                (i.to_string(), Some(label))
            })
            .collect();
        let f = LayerField::new("il_id", LayerFieldKind::BigInt);
        let mut best = f64::MAX;
        for _ in 0..20 {
            let t = std::time::Instant::now();
            let (list, left) = listed(&f, &rows);
            assert_eq!((list.len(), left), (10_000, 0));
            best = best.min(t.elapsed().as_secs_f64() * 1000.0);
        }
        let mut field = f.clone();
        field.values = Some(listed(&f, &rows).0);
        let t = std::time::Instant::now();
        assert!(field_problem(&field).is_none());
        let check = t.elapsed().as_secs_f64() * 1000.0;
        println!(
            "10 000 satırın listesi: {best:.2} ms (20 denemenin en iyisi); alanın denetimi {check:.2} ms"
        );
    }

    #[test]
    fn the_most_is_said_as_it_is() {
        assert_eq!(MOST_SAID.replace(' ', ""), MOST_REFERENCE_ROWS.to_string());
    }

    #[test]
    fn a_made_default_is_neither_null_nor_a_literal() {
        let c = |d: &str| gpkg::Column {
            name: "x".into(),
            decl: String::new(),
            kind: gpkg::ColumnKind::Text(None),
            not_null: false,
            default: Some(d.into()),
        };
        assert_eq!(made_of(&c("now()")).as_deref(), Some("now()"));
        assert_eq!(
            made_of(&c("CURRENT_TIMESTAMP")).as_deref(),
            Some("CURRENT_TIMESTAMP")
        );
        assert_eq!(made_of(&c("NULL")), None);
        assert_eq!(made_of(&c("(NULL)")), None);
        assert_eq!(made_of(&c("NULL::character varying")), None);
        assert_eq!(made_of(&c("'İlçe'::text")), None);
        assert_eq!(made_of(&c("7")), None);
        assert_eq!(
            made_of(&c("upper(\n'ilce')")).as_deref(),
            Some("upper( 'ilce')")
        );
        let long = format!("f({})", "a".repeat(2000));
        let made = made_of(&c(&long)).expect("made");
        assert_eq!(made.chars().count(), MAX_MADE);
        assert!(made.ends_with('…'));
    }

    #[test]
    fn a_reading_follows_the_key_and_keeps_an_own_list() {
        let mut t = LayerField::new("il_id", LayerFieldKind::BigInt);
        let refers = ColumnRefers {
            table: "kbs.iller".into(),
            column: "id".into(),
            label: Some("ad".into()),
        };
        t.column = Some(FieldColumn {
            made: None,
            refers: Some(refers.clone()),
            check: None,
        });
        t.values = Some(vec![FieldChoice::new("6", "Ankara")]);
        // A field without a list takes it.
        let mut f = LayerField::new("il_id", LayerFieldKind::BigInt);
        follow(&mut f, &t, true);
        assert_eq!(f.values, t.values);
        assert_eq!(f.column, t.column);
        // Its own list stays.
        let mut own = LayerField::new("il_id", LayerFieldKind::BigInt);
        own.values = Some(vec![FieldChoice::new("1", "Bir")]);
        let kept = own.clone();
        follow(&mut own, &t, true);
        assert_eq!(own, kept);
        // The key gone: the list goes with it.
        let plain = LayerField::new("il_id", LayerFieldKind::BigInt);
        follow(&mut f, &plain, true);
        assert_eq!((f.values, f.column), (None, None));
        // Not read: as it was.
        let mut f = LayerField::new("il_id", LayerFieldKind::BigInt);
        follow(&mut f, &t, true);
        let was = f.clone();
        follow(&mut f, &plain, false);
        assert_eq!(f, was);
    }
}
