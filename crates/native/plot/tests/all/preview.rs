//! Independent page-setup reference and an independent PDF rasterizer must agree with print preview.
use kentos_sheet::{
    SheetBook,
    ops::{self, Op},
    print_setup::*,
    project::project_sheets,
};
#[test]
fn twenty_print_previews_keep_the_pdf_physical_transform_and_ink() {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/page-setups.json"
    ))
    .unwrap();
    let out = std::env::temp_dir().join(format!("kentos-preview-{}", std::process::id()));
    std::fs::create_dir_all(&out).unwrap();
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
        let preview =
            kentos_plot::preview::page(&doc, "page", "Test", "2026-10-23", None, 96).unwrap();
        std::fs::write(out.join(format!("{n}-preview.png")), &preview.png).unwrap();
        let pdf = kentos_plot::book::book_pdf(&doc, &[], "Test", "2026-10-23").unwrap();
        let path = out.join(format!("{n}.pdf"));
        std::fs::write(&path, &pdf.bytes).unwrap();
        let stem = out.join(format!("{n}"));
        assert!(
            std::process::Command::new("pdftoppm")
                .args(["-r", "96", "-singlefile", "-png"])
                .arg(&path)
                .arg(&stem)
                .status()
                .unwrap()
                .success()
        );
        let decoder = png::Decoder::new(std::io::BufReader::new(
            std::fs::File::open(stem.with_extension("png")).unwrap(),
        ));
        let mut reader = decoder.read_info().unwrap();
        let mut bytes = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut bytes).unwrap();
        assert_eq!([preview.width, preview.height], [info.width, info.height]);
        let colour = |p: &[u8]| p[2] > p[0].saturating_add(8) && p[2] > p[1].saturating_add(5);
        // Independent rasterizers differ in antialiasing; every strong blue sample must have
        // counterpart within one pixel, and at least 90% of strong ink must survive each way.
        let mask = |bytes: &[u8], channels: usize| {
            bytes.chunks_exact(channels).map(colour).collect::<Vec<_>>()
        };
        let a = mask(&preview.rgba, 4);
        let b = mask(&bytes[..info.buffer_size()], 3);
        for (from, to) in [(&a, &b), (&b, &a)] {
            let mut count = 0;
            let mut found = 0;
            for (i, &ink) in from.iter().enumerate() {
                if !ink {
                    continue;
                }
                count += 1;
                let x = (i % info.width as usize) as i32;
                let y = (i / info.width as usize) as i32;
                if (-1..=1).any(|dy| {
                    (-1..=1).any(|dx| {
                        let (x, y) = (x + dx, y + dy);
                        x >= 0
                            && y >= 0
                            && x < info.width as i32
                            && y < info.height as i32
                            && to[y as usize * info.width as usize + x as usize]
                    })
                }) {
                    found += 1;
                }
            }
            assert!(
                count > 0 && found * 10 >= count * 9,
                "{} {found}/{count}",
                c["name"]
            );
        }
        let factor = c["expected"]["factor"].as_f64().unwrap();
        assert!(
            preview
                .svg
                .contains(&format!("matrix({factor} 0 0 {factor} "))
        );
        assert_eq!(doc.to_snapshot_v2(), before);
    }
    std::fs::remove_dir_all(out).unwrap();
}

#[test]
fn a_master_page_preview_has_real_paper_and_model_ink() {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/multipage-sets.json"
    ))
    .unwrap();
    let c = &data["cases"][8];
    let mut source = c["model"].clone();
    source["entities"].as_array_mut().unwrap().push(serde_json::json!({"kind":"line","id":99999,"layerId":"cizim","attrs":{},"a":{"x":-2.5,"y":-1.},"b":{"x":2.5,"y":-1.}}));
    let mut doc =
        kentos_domain::Document::from_snapshot(serde_json::from_value(source).unwrap()).unwrap();
    let book: SheetBook = serde_json::from_value(c["book"].clone()).unwrap();
    doc.set_sheets(project_sheets(&book, |_| None).unwrap())
        .unwrap();
    let image =
        kentos_plot::preview::page(&doc, &book.sheets[0].id, "Test", "2026-10-23", None, 96)
            .unwrap();
    std::fs::write(
        std::env::temp_dir().join("kentos-sheet23-master.png"),
        &image.png,
    )
    .unwrap();
    std::fs::write(
        std::env::temp_dir().join("kentos-sheet23-master.svg"),
        &image.svg,
    )
    .unwrap();
    assert!(
        image
            .rgba
            .chunks_exact(4)
            .filter(|p| p[0] > 248 && p[1] > 248 && p[2] > 248 && p[3] == 255)
            .count()
            > 300_000
    );
    assert!(
        image
            .rgba
            .chunks_exact(4)
            .filter(|p| p[0] < 200 || p[1] < 200 || p[2] < 200)
            .count()
            > 100
    );
}
