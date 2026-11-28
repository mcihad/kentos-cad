//! Tests of `geopackage.rs`, in a file of their own (the file is over the size limit and may not grow).

use super::*;

#[test]
fn a_columns_literal_default_is_read_as_postgresql_and_sqlite_write_it() {
    // PostgreSQL's pg_get_expr.
    assert_eq!(
        literal_default("'İlçe'::character varying").as_deref(),
        Some("İlçe")
    );
    assert_eq!(literal_default("'{}'::jsonb").as_deref(), Some("{}"));
    assert_eq!(literal_default("false").as_deref(), Some("false"));
    assert_eq!(literal_default("(-1)").as_deref(), Some("-1"));
    assert_eq!(
        literal_default("'2020-01-01'::date").as_deref(),
        Some("2020-01-01")
    );
    assert_eq!(literal_default("'a::b'::text").as_deref(), Some("a::b"));
    assert_eq!(literal_default("NULL::character varying"), None);
    for computed in [
        "now()",
        "uuidv7()",
        "gen_random_uuid()",
        "nextval('ilceler_id_seq'::regclass)",
        "CURRENT_TIMESTAMP",
        "('a' || 'b')",
        "(now() + '1 day'::interval)",
    ] {
        assert_eq!(literal_default(computed), None, "{computed}");
    }
    // SQLite's PRAGMA table_info.
    assert_eq!(literal_default("'Ali''nin'").as_deref(), Some("Ali'nin"));
    assert_eq!(literal_default("7").as_deref(), Some("7"));
    assert_eq!(literal_default("-1.5").as_deref(), Some("-1.5"));
    assert_eq!(literal_default("upper('ilce')"), None);
    assert_eq!(literal_default("NULL"), None);
    // A literal in its field's kind; one that does not read as the kind gives none.
    assert_eq!(
        default_as(LayerFieldKind::Boolean, "0").as_deref(),
        Some("false")
    );
    assert_eq!(
        default_as(LayerFieldKind::Boolean, "1").as_deref(),
        Some("true")
    );
    assert_eq!(
        default_as(LayerFieldKind::Integer, "3").as_deref(),
        Some("3")
    );
    assert_eq!(default_as(LayerFieldKind::Integer, "yok"), None);
    assert_eq!(default_as(LayerFieldKind::Text, ""), None);
}

#[test]
fn a_not_null_column_without_a_default_is_a_required_field() {
    let column =
        |name: &str, kind: ColumnKind, not_null: bool, default: Option<&str>| gpkg::Column {
            name: name.into(),
            decl: String::new(),
            kind,
            not_null,
            default: default.map(str::to_owned),
        };
    let ad = field_of(
        &column("ad", ColumnKind::Text(None), true, None),
        Reading::Plain,
    )
    .expect("a field");
    assert!(ad.required && ad.default.is_none());
    let tur = field_of(
        &column("tur", ColumnKind::Text(None), true, Some("'İlçe'")),
        Reading::Plain,
    )
    .expect("a field");
    assert!(!tur.required);
    assert_eq!(tur.default.as_deref(), Some("İlçe"));
    let made = field_of(
        &column("zaman", ColumnKind::DateTime, true, Some("now()")),
        Reading::Plain,
    )
    .expect("a field");
    assert!(
        !made.required && made.default.is_none(),
        "the database gives it"
    );
    let free = field_of(
        &column("notu", ColumnKind::Text(None), false, None),
        Reading::Plain,
    )
    .expect("a field");
    assert!(!free.required && free.default.is_none());
}

fn column(name: &str, kind: ColumnKind, decl: &str) -> gpkg::Column {
    gpkg::Column {
        name: name.into(),
        decl: decl.into(),
        kind,
        not_null: false,
        default: None,
    }
}

fn rows(values: &[Value]) -> Vec<gpkg::Row> {
    values
        .iter()
        .enumerate()
        .map(|(i, v)| gpkg::Row {
            fid: i as i64 + 1,
            geometry: None,
            values: vec![v.clone()],
        })
        .collect()
}

/// docs/adr/0629 §4: what a column's values and type say of its field.
#[test]
fn a_columns_values_say_its_kind() {
    let at = |c: &gpkg::Column, vs: &[Value], hint| {
        let r = reading_of(c, 0, &rows(vs), hint);
        field_of(c, r).map(|f| f.kind)
    };
    let int = column("n", ColumnKind::Integer, "");
    assert_eq!(
        at(&int, &[Value::Integer(7)], None),
        Some(LayerFieldKind::Integer)
    );
    assert_eq!(
        at(&int, &[Value::Integer(1 << 53)], None),
        Some(LayerFieldKind::BigInt)
    );
    assert_eq!(
        at(&int, &[Value::Integer(7)], Some(LayerFieldKind::BigInt)),
        Some(LayerFieldKind::BigInt)
    );
    let when = column("t", ColumnKind::DateTime, "");
    let text = |t: &str| Value::Text(t.into());
    assert_eq!(
        at(&when, &[text("2024-03-01T10:20:30Z")], None),
        Some(LayerFieldKind::DateTime)
    );
    assert_eq!(
        at(&when, &[text("2024-03-01T10:20:30+03:00")], None),
        Some(LayerFieldKind::Instant)
    );
    assert_eq!(
        at(&when, &[text("2024-03-01T10:20:30.5Z")], None),
        Some(LayerFieldKind::Instant)
    );
    // A local time keeps being one: its fraction makes no moment.
    let local = column("t", ColumnKind::DateTime, "timestamp");
    assert_eq!(
        at(&local, &[text("2024-03-01T10:20:30.5")], None),
        Some(LayerFieldKind::DateTime)
    );
    assert_eq!(
        at(
            &column("b", ColumnKind::Blob(None), ""),
            &[Value::Blob(vec![1])],
            None
        ),
        Some(LayerFieldKind::Binary)
    );
    let id = column("u", ColumnKind::Other("uuid".into()), "uuid");
    assert_eq!(
        at(&id, &[text("0192F5A0-7C3E-7D4A-9B1E-4C2F8A6D0001")], None),
        Some(LayerFieldKind::Uuid)
    );
    // A value that does not keep the hint's kind leaves the column as text.
    assert_eq!(at(&id, &[text("yok")], None), Some(LayerFieldKind::Text));
    let json = column("j", ColumnKind::Other("jsonb".into()), "jsonb");
    assert_eq!(
        at(&json, &[text("{\"a\": 1}")], None),
        Some(LayerFieldKind::Json)
    );
}

/// The import's values of the kinds, and what a cell cannot keep said.
#[test]
fn what_a_cell_cannot_keep_is_said() {
    let mut losses = BTreeSet::new();
    let blob = ColumnKind::Blob(None);
    assert_eq!(
        value_text(
            &Value::Blob(b"ABC".to_vec()),
            &blob,
            Reading::Binary,
            &mut losses,
            "b"
        )
        .as_deref(),
        Some("QUJD")
    );
    assert!(losses.is_empty());
    let big = Value::Blob(vec![0; MAX_BINARY + 1]);
    assert_eq!(
        value_text(&big, &blob, Reading::Binary, &mut losses, "b"),
        None
    );
    assert!(
        losses.iter().any(|l| l.contains("1 MiB'ı aşan")),
        "{losses:?}"
    );
    assert_eq!(
        value_text(
            &Value::Real(f64::NAN),
            &ColumnKind::Real,
            Reading::Plain,
            &mut losses,
            "r"
        ),
        None
    );
    assert!(
        losses.iter().any(|l| l.contains("sonlu olmayan")),
        "{losses:?}"
    );
    let when = ColumnKind::DateTime;
    assert_eq!(
        value_text(
            &Value::Text("2024-03-01 10:20:30.500+03".into()),
            &when,
            Reading::Instant,
            &mut losses,
            "t"
        )
        .as_deref(),
        Some("2024-03-01T10:20:30.5+03:00")
    );
    assert_eq!(
        in_thousandths("2024-03-01T07:20:30.5Z"),
        "2024-03-01T07:20:30.500Z"
    );
    assert_eq!(
        in_thousandths("2024-03-01T07:20:30.1234Z"),
        "2024-03-01T07:20:30.123400Z"
    );
    assert_eq!(
        in_thousandths("2024-03-01T07:20:30Z"),
        "2024-03-01T07:20:30Z"
    );
    // No offset in a time stamp column: UTC's.
    assert_eq!(
        value_text(
            &Value::Text("2024-03-01T10:20:30.25".into()),
            &when,
            Reading::Instant,
            &mut losses,
            "t"
        )
        .as_deref(),
        Some("2024-03-01T10:20:30.25Z")
    );
}

/// docs/adr/0629 §4: a cell the layer would write is the table's when it means the same to the layer's field.
#[test]
fn a_time_stamp_is_the_same_moment() {
    let when = ColumnKind::DateTime;
    let text = |t: &str| Value::Text(t.into());
    let (a, b) = (
        text("2024-03-01T07:20:30.5Z"),
        text("2024-03-01T10:20:30.500+03:00"),
    );
    assert!(same_cell_as(&a, &b, &when, Some(LayerFieldKind::Instant)));
    assert!(!same_cell_as(
        &a,
        &text("2024-03-01T07:20:31Z"),
        &when,
        Some(LayerFieldKind::Instant)
    ));
    let id = ColumnKind::Text(Some(36));
    assert!(same_cell_as(
        &text("0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0001"),
        &text("0192F5A0-7C3E-7D4A-9B1E-4C2F8A6D0001"),
        &id,
        Some(LayerFieldKind::Uuid)
    ));
    assert!(same_cell_as(
        &text("{\"a\":1}"),
        &text("{ \"a\" : 1 }"),
        &ColumnKind::Text(None),
        Some(LayerFieldKind::Json)
    ));
    let blob = ColumnKind::Blob(None);
    assert!(same_cell_as(
        &Value::Blob(vec![1, 2]),
        &Value::Blob(vec![1, 2]),
        &blob,
        Some(LayerFieldKind::Binary)
    ));
    assert!(!same_cell_as(
        &Value::Blob(vec![1, 2]),
        &Value::Blob(vec![1]),
        &blob,
        Some(LayerFieldKind::Binary)
    ));
}

/// docs/adr/0629 §4: KentOS's document says the kinds a GeoPackage's types do not.
#[test]
fn the_fields_document_names_the_wide_kinds_only() {
    let fields = vec![
        LayerField::new("Ad", LayerFieldKind::Text),
        LayerField::new("Cihaz", LayerFieldKind::Uuid),
        LayerField::new("Ek", LayerFieldKind::Json),
    ];
    let columns = vec![
        ("Ad".to_owned(), "ad".to_owned()),
        ("Cihaz".to_owned(), "cihaz".to_owned()),
        ("Ek".to_owned(), "ek".to_owned()),
    ];
    assert_eq!(
        field_document(&fields, &columns).as_deref(),
        Some(r#"{"kinds":{"cihaz":"uuid","ek":"json"}}"#)
    );
    assert_eq!(field_document(&fields[..1], &columns), None);
}
