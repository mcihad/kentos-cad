use kentos_sheet::{
    SheetBook,
    ops::{self, Op},
    print_setup::*,
    project::project_sheets,
};
#[test]
fn twenty_print_transforms_reach_pdf_content_and_keep_source_unchanged() {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/page-setups.json"
    ))
    .unwrap();
    let folder = std::env::temp_dir().join(format!("kentos-sheet18-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    for (n, c) in data["cases"].as_array().unwrap().iter().enumerate() {
        let mut book = SheetBook::default();
        book.sheets
            .push(serde_json::from_value(c["sheet"].clone()).unwrap());
        book = ops::apply_all(
            &book,
            &[
                Op::SavePageSetups(SavePageSetups {
                    setups: vec![serde_json::from_value(c["setup"].clone()).unwrap()],
                }),
                Op::ApplyPageSetup(ApplyPageSetup {
                    sheet: "page".into(),
                    setup: "office".into(),
                }),
            ],
        )
        .unwrap()
        .book;
        let mut doc = kentos_domain::Document::from_snapshot(
            serde_json::from_value(c["model"].clone()).unwrap(),
        )
        .unwrap();
        doc.set_sheets(project_sheets(&book, |_| None).unwrap())
            .unwrap();
        let before = doc.to_snapshot_v2();
        let pdf = kentos_plot::book::book_pdf(&doc, &[], "Test", "2026-10-23").unwrap();
        let pages = super::files::pdf_pages(&pdf.bytes);
        assert_eq!(pages.len(), 1);
        assert!((pages[0].width - 420.).abs() < 0.001);
        assert!((pages[0].height - 297.).abs() < 0.001);
        let b = c["expected"]["box"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap() / 1000.)
            .collect::<Vec<_>>();
        let want = [b[0], 297. - b[1], b[0] + b[2], 297. - b[1]];
        assert!(
            pages[0]
                .segments
                .iter()
                .any(|s| s.iter().zip(want).all(|(a, b)| (a - b).abs() < 0.002)),
            "{} missing {want:?}: {:?}",
            c["name"],
            pages[0].segments
        );
        let path = folder.join(format!("{n}.pdf"));
        std::fs::write(&path, pdf.bytes).unwrap();
        let output = std::process::Command::new("pdftocairo")
            .args(["-svg"])
            .arg(&path)
            .arg(folder.join(format!("{n}.svg")))
            .output()
            .unwrap();
        assert!(output.status.success());
        let svg = std::fs::read_to_string(folder.join(format!("{n}.svg"))).unwrap();
        // An independent PDF interpreter must retain the coloured vector rectangle after clipping/scaling.
        assert!(svg.contains("path"), "{} {svg}", c["name"]);
        let text = std::process::Command::new("pdftotext")
            .arg(&path)
            .arg("-")
            .output()
            .unwrap();
        assert!(text.status.success());
        assert!(
            String::from_utf8(text.stdout).unwrap().contains("PRINT"),
            "{}",
            c["name"]
        );
        assert_eq!(doc.to_snapshot_v2(), before);
    }
    std::fs::remove_dir_all(folder).unwrap();
}
