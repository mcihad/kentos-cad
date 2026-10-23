//! Real searchable PDF fields and a live index use the same metadata as the sheet editor.
use kentos_sheet::{
    Item, ItemKind, RectUm, TableColumn, TableSource,
    drawing_set::{DrawingIndex, DrawingInfo},
    ops::{Op, apply_all},
};
use serde_json::{Value};
#[test]
fn drawing_set_title_fields_and_live_index_follow_a_revision_in_the_real_pdf() {
    let fx: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/drawing-set.json"
    ))
    .unwrap();
    let mut book: kentos_sheet::SheetBook = serde_json::from_value(fx["book"].clone()).unwrap();
    book = apply_all(
        &book,
        &serde_json::from_value::<Vec<Op>>(fx["cases"][0]["ops"].clone()).unwrap(),
    )
    .unwrap()
    .book;
    let mut table = match kentos_sheet::kinds::default_kind("table").unwrap() {
        ItemKind::Table(t) => t,
        _ => unreachable!(),
    };
    table.source = TableSource::DrawingSet(DrawingIndex::default());
    table.columns = ["No", "Başlık", "Rev", "Durum", "Alt küme", "Görünüm"]
        .into_iter()
        .map(|v| TableColumn {
            heading: v.into(),
            value: String::new(),
            width: 0,
            align: Default::default(),
            decimals: None,
        })
        .collect();
    book.sheets[0].items.push(Item::new(
        "index",
        "Pafta dizini",
        RectUm {
            left: 20_000,
            top: 140_000,
            width: 380_000,
            height: 90_000,
        },
        ItemKind::Table(table),
    ));
    let mut text = kentos_sheet::kinds::default_kind("text").unwrap();
    if let ItemKind::Text(t) = &mut text {
        t.content="[% @pafta_no %] | [% @pafta_basligi %] | [% @revizyon %] | [% @pafta_durumu %] | [% @alt_kume %]".into();
    }
    book.sheets[0].items.push(Item::new(
        "fields",
        "Antet alanları",
        RectUm {
            left: 20_000,
            top: 245_000,
            width: 380_000,
            height: 30_000,
        },
        text,
    ));
    let mut doc = kentos_domain::Document::from_snapshot(
        serde_json::from_value(fx["model"].clone()).unwrap(),
    )
    .unwrap();
    let root = std::env::temp_dir().join(format!("kentos-drawing-set-pdf-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    for (n, title, rev) in [(0, "Zemin kat", "R02"), (1, "Yenilenen kat", "R03")] {
        if n == 1 {
            book.sheets[0].drawing_info = Some(DrawingInfo {
                number: "A-101".into(),
                title: title.into(),
                revision: rev.into(),
                status: "Kontrol".into(),
                subset: vec!["Mimari".into(), "Katlar".into()],
            });
        }
        doc.set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
            .unwrap();
        let before = doc.to_snapshot_v2();
        let pdf = kentos_plot::book::book_pdf(&doc, &[], "kabul", "2026-10-23").unwrap();
        let path = root.join(format!("{n}.pdf"));
        std::fs::write(&path, &pdf.bytes).unwrap();
        let result = std::process::Command::new("pdftotext")
            .args(["-layout"])
            .arg(path)
            .arg("-")
            .output()
            .unwrap();
        assert!(result.status.success());
        let words = String::from_utf8(result.stdout).unwrap();
        assert!(
            words.matches(title).count() >= 2,
            "live index and title field: {words}"
        );
        for word in ["A-101", rev, "Kontrol", "Mimari / Katlar"] {
            assert!(words.contains(word), "{word}: {words}");
        }
        if n == 1 {
            assert!(!words.contains("Zemin kat"), "no stale index cells");
        }
        assert_eq!(doc.to_snapshot_v2(), before);
        assert!(crate::files::pdf_pages(&pdf.bytes)[0].ink.iter().any(|p| {
            ((p.segment[2] - p.segment[0]).hypot(p.segment[3] - p.segment[1]) - 100.).abs() < 0.002
        }));
    }
    std::fs::remove_dir_all(root).unwrap();
}
