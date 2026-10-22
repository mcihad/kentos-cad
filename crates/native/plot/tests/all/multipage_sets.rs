use kentos_domain::Document;
use kentos_sheet::{SheetBook, project::project_sheets};
use serde_json::Value;
#[test]
fn twenty_six_page_sets_keep_each_master_map_and_architectural_view_in_its_own_page() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/multipage-sets.json"
    ))
    .unwrap();
    let out = std::env::temp_dir().join(format!("kentos-sheet17-{}", std::process::id()));
    std::fs::create_dir_all(&out).unwrap();
    for (index, c) in data["cases"].as_array().unwrap().iter().enumerate() {
        let mut doc =
            Document::from_snapshot(serde_json::from_value(c["model"].clone()).unwrap()).unwrap();
        let book: SheetBook = serde_json::from_value(c["book"].clone()).unwrap();
        doc.set_sheets(project_sheets(&book, |_| None).unwrap())
            .unwrap();
        let before = doc.to_snapshot_v2();
        let pdf = kentos_plot::book::book_pdf(&doc, &[], "Test", "2026-10-22").unwrap();
        assert_eq!(
            serde_json::to_value(&pdf.sheets).unwrap(),
            c["expected"]["pages"]
        );
        assert!(pdf.notes.is_empty(), "{:?}", pdf.notes);
        let pages = super::files::pdf_pages(&pdf.bytes);
        assert_eq!(pages.len(), 6);
        assert!((pages[0].width - 210.).abs() < 0.001);
        assert!((pages[1].width - 420.).abs() < 0.001);
        let path = out.join(format!("{index}.pdf"));
        std::fs::write(&path, &pdf.bytes).unwrap();
        let read = std::process::Command::new("pdftotext")
            .args(["-layout"])
            .arg(&path)
            .arg("-")
            .output()
            .expect("Poppler is required for independent PDF acceptance");
        assert!(read.status.success());
        let text = String::from_utf8(read.stdout).unwrap();
        let texts = text.split('\u{c}').collect::<Vec<_>>();
        assert!(!texts[0].contains("FAR"), "{} small page {text}", c["name"]);
        for (page, words) in c["expected"]["words"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            for word in words.as_array().unwrap() {
                assert!(
                    texts[page].contains(word.as_str().unwrap()),
                    "{} page {} missing {}: {text}",
                    c["name"],
                    page + 1,
                    word
                );
            }
        }
        assert_eq!(doc.to_snapshot_v2(), before);
        assert_eq!(doc.entities().count(), c["expected"]["sourceCount"].as_u64().unwrap() as usize);
    }
    std::fs::remove_dir_all(out).unwrap();
}
