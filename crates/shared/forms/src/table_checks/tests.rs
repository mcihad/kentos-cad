//! The constraints' judgement against SQLite's and PostgreSQL's own answers: `fixtures/table-checks/v1/cases.json`,
//! written by `scripts/fixtures/table_checks_cases.py` (SQLite's answers from SQLite itself, PostgreSQL's from a
//! PostgreSQL server), each record judged as KentOS judges it.

use std::collections::BTreeMap;

use kentos_contracts::{LayerField, TableCheck};
use serde_json::Value;

use super::*;

fn check(sql: &str, columns: &[&str]) -> TableCheck {
    TableCheck {
        name: Some("k".into()),
        sql: sql.into(),
        columns: columns.iter().map(|c| (*c).to_owned()).collect(),
        table_only: false,
    }
}

fn field(name: &str, kind: LayerFieldKind) -> LayerField {
    LayerField::new(name, kind)
}

fn attrs(pairs: &[(&str, &str)]) -> Attrs {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

fn breaks(sql: &str, columns: &[&str], fields: &[LayerField], d: Dialect, a: &Attrs) -> bool {
    let checks = TableChecks::new(&[check(sql, columns)], fields, d);
    !checks.broken(fields, None, a).is_empty()
}

#[test]
fn a_list_and_a_range() {
    let f = [field("tur", LayerFieldKind::Text)];
    let pg = "((tur)::text = ANY ((ARRAY['is'::character varying, 'personel'::character varying])::text[]))";
    assert!(!breaks(
        pg,
        &["tur"],
        &f,
        Dialect::Postgres,
        &attrs(&[("tur", "is")])
    ));
    assert!(breaks(
        pg,
        &["tur"],
        &f,
        Dialect::Postgres,
        &attrs(&[("tur", "Is")])
    ));
    // NULL holds.
    assert!(!breaks(pg, &["tur"], &f, Dialect::Postgres, &attrs(&[])));
    let o = [field("oncelik", LayerFieldKind::Integer)];
    let range = "((oncelik >= 1) AND (oncelik <= 3))";
    assert!(breaks(
        range,
        &["oncelik"],
        &o,
        Dialect::Postgres,
        &attrs(&[("oncelik", "4")])
    ));
    assert!(!breaks(
        range,
        &["oncelik"],
        &o,
        Dialect::Postgres,
        &attrs(&[("oncelik", "3")])
    ));
    let lite = "oncelik BETWEEN 1 AND 3";
    assert!(breaks(
        lite,
        &["oncelik"],
        &o,
        Dialect::Sqlite,
        &attrs(&[("oncelik", "0")])
    ));
}

#[test]
fn what_kentos_cannot_follow_holds() {
    let t = [field("ad", LayerFieldKind::Text)];
    // PostgreSQL's order of texts hangs on its collation.
    assert!(!breaks(
        "(ad > 'm'::text)",
        &["ad"],
        &t,
        Dialect::Postgres,
        &attrs(&[("ad", "a")])
    ));
    // SQLite's is the bytes'.
    assert!(breaks(
        "ad > 'm'",
        &["ad"],
        &t,
        Dialect::Sqlite,
        &attrs(&[("ad", "a")])
    ));
    // A function KentOS has not.
    assert!(!breaks(
        "(jsonb_array_length(ad) > 0)",
        &["ad"],
        &t,
        Dialect::Postgres,
        &attrs(&[("ad", "x")])
    ));
    // But a conjunction false on its known side is false.
    assert!(breaks(
        "((jsonb_array_length(ad) > 0) AND (ad <> ''::text))",
        &["ad"],
        &t,
        Dialect::Postgres,
        &attrs(&[("ad", "")])
    ));
    let checks = TableChecks::new(
        &[check("(jsonb_array_length(ad) > 0)", &["ad"])],
        &t,
        Dialect::Postgres,
    );
    assert_eq!(checks.judged(0), Judged::Partly);
    let only = TableCheck {
        table_only: true,
        ..check("ad <> ''", &["ad"])
    };
    let checks = TableChecks::new(&[only], &t, Dialect::Sqlite);
    assert_eq!(checks.judged(0), Judged::TableOnly);
    assert!(checks.broken(&t, None, &attrs(&[("ad", "")])).is_empty());
}

#[test]
fn a_check_is_asked_when_a_column_it_reads_changed() {
    let f = [
        field("a", LayerFieldKind::Integer),
        field("b", LayerFieldKind::Integer),
    ];
    let checks = TableChecks::new(&[check("a > 0", &["a"])], &f, Dialect::Sqlite);
    let before = attrs(&[("a", "0"), ("b", "1")]);
    let after = attrs(&[("a", "0"), ("b", "2")]);
    assert!(checks.broken(&f, Some(&before), &after).is_empty());
    assert_eq!(checks.broken(&f, None, &after).len(), 1);
}

#[test]
fn sentences() {
    let said = |sql: &str, cols: &[&str], d: Dialect| {
        let f: Vec<LayerField> = cols
            .iter()
            .map(|c| field(c, LayerFieldKind::Text))
            .collect();
        TableChecks::new(&[check(sql, cols)], &f, d).said(0)
    };
    assert_eq!(
        said(
            "((tur)::text = ANY ((ARRAY['is'::character varying, 'personel'::character varying, 'butce_disi'::character varying])::text[]))",
            &["tur"],
            Dialect::Postgres
        ),
        "tur şunlardan biri olmalı: is, personel, butce_disi"
    );
    assert_eq!(
        said(
            "((oncelik >= 1) AND (oncelik <= 3))",
            &["oncelik"],
            Dialect::Postgres
        ),
        "oncelik 1 ile 3 arasında olmalı"
    );
    assert_eq!(
        said(
            "((bitis IS NULL) OR (bitis > baslangic))",
            &["baslangic", "bitis"],
            Dialect::Postgres
        ),
        "bitis, baslangic alanından büyük olmalı"
    );
    assert_eq!(
        said(
            "((kod)::text ~ '^[0-9]{2}$'::text)",
            &["kod"],
            Dialect::Postgres
        ),
        "kod şu kalıba uymalı: ^[0-9]{2}$"
    );
    assert_eq!(
        said(
            "(length(btrim(gerekce)) >= 3)",
            &["gerekce"],
            Dialect::Postgres
        ),
        "gerekce baştaki ve sondaki boşluklar sayılmadan en az 3 karakter olmalı"
    );
    assert_eq!(
        said("x <> ''", &["x"], Dialect::Sqlite),
        "x boş metin olmamalı"
    );
}

#[test]
fn value_lists() {
    let pg = check(
        "((tur)::text = ANY ((ARRAY['is'::character varying, 'personel'::character varying])::text[]))",
        &["tur"],
    );
    assert_eq!(
        value_list(&pg, Dialect::Postgres, LayerFieldKind::Text),
        Some(("tur".into(), vec!["is".into(), "personel".into()]))
    );
    let lite = check("durum IS NULL OR durum IN ('A', 'P', 'A')", &["durum"]);
    assert_eq!(
        value_list(&lite, Dialect::Sqlite, LayerFieldKind::Text),
        Some(("durum".into(), vec!["A".into(), "P".into()]))
    );
    let ints = check("(seviye = ANY (ARRAY[1, 2, 3]))", &["seviye"]);
    assert_eq!(
        value_list(&ints, Dialect::Postgres, LayerFieldKind::Integer).map(|(_, v)| v),
        Some(vec!["1".into(), "2".into(), "3".into()])
    );
    // A NULL in the list holds for every value; not a list.
    assert_eq!(
        value_list(
            &check("x IN ('a', NULL)", &["x"]),
            Dialect::Sqlite,
            LayerFieldKind::Text
        ),
        None
    );
    assert_eq!(
        value_list(
            &check("x NOT IN ('a')", &["x"]),
            Dialect::Sqlite,
            LayerFieldKind::Text
        ),
        None
    );
    assert_eq!(
        value_list(
            &check("x IN (1.5)", &["x"]),
            Dialect::Sqlite,
            LayerFieldKind::Text
        ),
        None
    );
    assert_eq!(
        value_list(
            &check("(x = ANY ('{a,b}'::text[]))", &["x"]),
            Dialect::Postgres,
            LayerFieldKind::Text
        )
        .map(|(_, v)| v),
        Some(vec!["a".into(), "b".into()])
    );
}

#[test]
fn columns_a_check_reads() {
    let cols = [
        "fid".to_owned(),
        "Tur".to_owned(),
        "b".to_owned(),
        "c".to_owned(),
    ];
    assert_eq!(
        columns_of("tur IN ('a') AND C > b", Dialect::Sqlite, &cols),
        vec!["Tur", "b", "c"]
    );
    assert_eq!(
        columns_of("(\"Tur\" <> ''::text)", Dialect::Postgres, &cols),
        vec!["Tur"]
    );
    assert!(columns_of("(tur <> ''::text)", Dialect::Postgres, &cols).is_empty());
}

/// The shared cases: a record's values, the constraint and what the database answered (`ok`: the write went in).
#[test]
fn the_shared_cases() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/table-checks/v1/cases.json");
    let text = std::fs::read_to_string(&path).expect("the cases");
    let doc: Value = serde_json::from_str(&text).expect("json");
    let mut seen = 0;
    for case in doc["cases"].as_array().expect("cases") {
        let name = case["name"].as_str().unwrap_or_default();
        let d = match case["dialect"].as_str() {
            Some("sqlite") => Dialect::Sqlite,
            _ => Dialect::Postgres,
        };
        let fields: Vec<LayerField> = serde_json::from_value(case["fields"].clone())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let c: TableCheck =
            serde_json::from_value(case["check"].clone()).unwrap_or_else(|e| panic!("{name}: {e}"));
        let checks = TableChecks::new(std::slice::from_ref(&c), &fields, d);
        if let Some(j) = case["judged"].as_str() {
            let want = match j {
                "whole" => Judged::Whole,
                "partly" => Judged::Partly,
                _ => Judged::TableOnly,
            };
            assert_eq!(checks.judged(0), want, "{name}");
        }
        if let Some(s) = case["said"].as_str() {
            assert_eq!(checks.said(0), s, "{name}");
        }
        if let Some(list) = case.get("list") {
            let kind = fields
                .iter()
                .find(|f| c.columns.first() == Some(&f.name))
                .map(|f| f.kind);
            let got = kind.and_then(|k| value_list(&c, d, k)).map(|(_, v)| v);
            let want: Option<Vec<String>> = serde_json::from_value(list.clone()).expect("list");
            assert_eq!(got, want, "{name}");
        }
        for row in case["rows"].as_array().expect("rows") {
            let values: BTreeMap<String, String> = serde_json::from_value(row["values"].clone())
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            let ok = row["ok"].as_bool().expect("ok");
            let broken = !checks.broken(&fields, None, &values).is_empty();
            // KentOS refuses only what the table refused.
            if broken {
                assert!(
                    !ok,
                    "{name}: KentOS refused {values:?}, which the table took"
                );
            }
            if let Some(kentos) = row["kentos"].as_str() {
                assert_eq!(
                    if broken { "refuses" } else { "takes" },
                    kentos,
                    "{name}: {values:?}"
                );
            }
            seen += 1;
        }
    }
    assert!(seen > 0);
}

/// How long the constraints take (docs/adr/0755 Doğrulama): six common checks made once, then asked of 100 000 new
/// records and of 100 000 edits of one column, as a layer's rules ask each written record. Release:
/// `cargo test --release -p kentos-forms --lib table_checks::tests::timing -- --ignored --nocapture`.
#[test]
#[ignore = "a timing; run in release"]
fn timing() {
    use std::time::Instant;
    let fields = [
        field("tur", LayerFieldKind::Text),
        field("oncelik", LayerFieldKind::Integer),
        field("tutar", LayerFieldKind::Decimal),
        field("kod", LayerFieldKind::Text),
        field("baslangic", LayerFieldKind::Date),
        field("bitis", LayerFieldKind::Date),
        field("gerekce", LayerFieldKind::Text),
    ];
    let checks = [
        check(
            "((tur)::text = ANY ((ARRAY['is'::character varying, 'personel'::character varying, 'butce_disi'::character varying])::text[]))",
            &["tur"],
        ),
        check("((oncelik >= 1) AND (oncelik <= 3))", &["oncelik"]),
        check("(tutar >= 0.01)", &["tutar"]),
        check("((kod)::text ~ '^[0-9]{2}$'::text)", &["kod"]),
        check(
            "((bitis IS NULL) OR (bitis > baslangic))",
            &["baslangic", "bitis"],
        ),
        check("(length(btrim(gerekce)) >= 3)", &["gerekce"]),
    ];
    let rounds = 1000;
    let t = Instant::now();
    let mut made = TableChecks::default();
    for _ in 0..rounds {
        made = TableChecks::new(&checks, &fields, Dialect::Postgres);
    }
    let make = t.elapsed() / rounds;
    let n = 100_000usize;
    let records: Vec<Attrs> = (0..n)
        .map(|i| {
            let mut a = Attrs::new();
            let tur = ["is", "personel", "butce_disi", "x"][i % 4];
            a.insert("tur".into(), tur.into());
            a.insert("oncelik".into(), (i % 4).to_string());
            a.insert("tutar".into(), format!("{}.{:02}", i % 1000, i % 100));
            let kod = if i % 7 == 0 {
                "abc".to_owned()
            } else {
                format!("{:02}", i % 100)
            };
            a.insert("kod".into(), kod);
            a.insert("baslangic".into(), "2026-01-15".into());
            let bitis = if i % 9 == 0 {
                "2025-12-01"
            } else {
                "2026-02-01"
            };
            a.insert("bitis".into(), bitis.into());
            let gerekce = if i % 11 == 0 { "  ab  " } else { "açıklama" };
            a.insert("gerekce".into(), gerekce.into());
            a
        })
        .collect();
    let t = Instant::now();
    let broken_new: usize = records
        .iter()
        .map(|r| made.broken(&fields, None, r).len())
        .sum();
    let new = t.elapsed();
    let edits: Vec<(Attrs, Attrs)> = records
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut after = r.clone();
            after.insert("oncelik".into(), ((i + 1) % 4).to_string());
            (r.clone(), after)
        })
        .collect();
    let t = Instant::now();
    let broken_edits: usize = edits
        .iter()
        .map(|(before, after)| made.broken(&fields, Some(before), after).len())
        .sum();
    let edit = t.elapsed();
    println!(
        "altı kısıt kurulur: {:.1} µs; 100 000 yeni kayıt: {:.1} ms ({broken_new} bozuk); \
         100 000 tek alan düzenlemesi: {:.1} ms ({broken_edits} bozuk)",
        make.as_secs_f64() * 1e6,
        new.as_secs_f64() * 1e3,
        edit.as_secs_f64() * 1e3,
    );
    assert!(broken_new > 0 && broken_edits > 0);
}
