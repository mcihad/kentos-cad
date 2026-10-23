//! Independent rational physical dimensions, source metre/millimetre metadata and large origins.
use kentos_sheet::{SheetBook, project::project_sheets};
#[test]
fn twenty_physical_calibration_pages_match_model_and_paper_rulers() {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/calibration.json"
    ))
    .unwrap();
    for c in data["cases"].as_array().unwrap() {
        let mut book = SheetBook::default();
        book.sheets
            .push(serde_json::from_value(c["sheet"].clone()).unwrap());
        let mut doc = kentos_domain::Document::from_snapshot(
            serde_json::from_value(c["model"].clone()).unwrap(),
        )
        .unwrap();
        doc.set_sheets(project_sheets(&book, |_| None).unwrap())
            .unwrap();
        let before = doc.to_snapshot_v2();
        let pdf = kentos_plot::book::book_pdf(&doc, &[], "Test", "2026-10-23").unwrap();
        let pages = super::files::pdf_pages(&pdf.bytes);
        let page = &pages[0];
        for rgb in [
            [34. / 255., 85. / 255., 170. / 255.],
            [204. / 255., 51. / 255., 68. / 255.],
        ] {
            let lengths: Vec<_> = page
                .ink
                .iter()
                .filter(|line| {
                    line.color
                        .iter()
                        .zip(rgb)
                        .all(|(a, b)| (a - b).abs() < 0.002)
                })
                .map(|line| {
                    (line.segment[2] - line.segment[0]).hypot(line.segment[3] - line.segment[1])
                })
                .collect();
            assert!(
                lengths.iter().any(|length| (length - 100.).abs() < 0.002),
                "{} {rgb:?}: {lengths:?}",
                c["name"]
            );
        }
        assert_eq!(doc.to_snapshot_v2(), before);
    }
}

#[test]
fn the_calibration_template_keeps_100_mm_marks_and_a_bound_1_to_50_view_on_every_paper() {
    use kentos_sheet::{
        template::*,
        display::{self, Prim},
        kinds::MapView,
    };
    let t = system_template("sys:baski-kalibrasyonu").unwrap();
    for (paper, orientation) in [
        (kentos_sheet::Paper::A4, kentos_sheet::Orientation::Portrait),
        (
            kentos_sheet::Paper::A3,
            kentos_sheet::Orientation::Landscape,
        ),
        (
            kentos_sheet::Paper::A0,
            kentos_sheet::Orientation::Landscape,
        ),
    ] {
        let ids = InstanceIds {
            sheet: "page".into(),
            master: None,
            items: t.sheet.items.iter().map(|i| i.id.clone()).collect(),
            master_items: Vec::new(),
        };
        let inst = instantiate(
            t,
            &ids,
            &InstanceOptions {
                center: Some(kentos_sheet::GroundPoint { x: 2.5, y: 0. }),
                scale: Some(200),
                paper: Some(PaperChoice { paper, orientation }),
                ..Default::default()
            },
        )
        .unwrap();
        let mut book = SheetBook::default();
        book.sheets.push(inst.sheet);
        let model: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../fixtures/sheet/v1/calibration.json"
        ))
        .unwrap();
        let mut source = model["cases"][6]["model"].clone();
        source["entities"][0]["a"] = serde_json::json!({"x":0.,"y":0.});
        source["entities"][0]["b"] = serde_json::json!({"x":5.,"y":0.});
        let mut doc =
            kentos_domain::Document::from_snapshot(serde_json::from_value(source).unwrap())
                .unwrap();
        doc.set_sheets(project_sheets(&book, |_| None).unwrap())
            .unwrap();
        let pdf = kentos_plot::book::book_pdf(&doc, &[], "Test", "2026-10-23").unwrap();
        let pages = super::files::pdf_pages(&pdf.bytes);
        for rgb in [
            [34. / 255., 85. / 255., 170. / 255.],
            [204. / 255., 51. / 255., 68. / 255.],
        ] {
            assert!(pages[0].ink.iter().any(|p| {
                p.color.iter().zip(rgb).all(|(a, b)| (a - b).abs() < 0.002)
                    && ((p.segment[2] - p.segment[0]).hypot(p.segment[3] - p.segment[1]) - 100.)
                        .abs()
                        < 0.002
            }));
        }
        let red=pages[0].ink.iter().filter(|p|(p.color[0]-0.8).abs()<0.002&&(p.color[1]-0.2).abs()<0.002).collect::<Vec<_>>();
        assert!(red.iter().any(|p|((p.segment[2]-p.segment[0]).abs()-100.).abs()<0.002&&(p.segment[3]-p.segment[1]).abs()<0.002));
        assert!(red.iter().any(|p|((p.segment[3]-p.segment[1]).abs()-100.).abs()<0.002&&(p.segment[2]-p.segment[0]).abs()<0.002));
        let list = display::display_list(&book, "page", &Default::default()).unwrap();
        assert!(
            list.prims
                .iter()
                .any(|p| matches!(p,Prim::Map(m) if (m.view.scale-50.).abs()<1e-9))
        );
        let viewport = book.sheets[0]
            .items
            .iter()
            .find(|i| i.id == "model")
            .unwrap();
        assert!(
            matches!(&viewport.kind,kentos_sheet::ItemKind::Map(m) if m.view_locked&&matches!(m.view,MapView::Fixed(_)))
        );
    }
}

#[test]
fn exported_png_records_its_physical_dpi() {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/calibration.json"
    ))
    .unwrap();
    let c = &data["cases"][6];
    let mut doc =
        kentos_domain::Document::from_snapshot(serde_json::from_value(c["model"].clone()).unwrap())
            .unwrap();
    let mut book = SheetBook::default();
    book.sheets
        .push(serde_json::from_value(c["sheet"].clone()).unwrap());
    doc.set_sheets(project_sheets(&book, |_| None).unwrap())
        .unwrap();
    for dpi in [72, 96, 150, 200, 300] {
        let page =
            kentos_plot::preview::page(&doc, "page", "Test", "2026-10-23", None, dpi).unwrap();
        let reader = png::Decoder::new(std::io::Cursor::new(page.png))
            .read_info()
            .unwrap();
        let dims = reader.info().pixel_dims.unwrap();
        assert_eq!(dims.unit, png::Unit::Meter);
        assert_eq!(dims.xppu, (f64::from(dpi) / 0.0254).round() as u32);
        assert_eq!(dims.xppu, dims.yppu);
    }
}
