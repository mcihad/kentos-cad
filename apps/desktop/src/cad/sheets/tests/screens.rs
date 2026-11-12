//! Sheets' pictures and PDF files for the owner, run by hand: each step's screens, the comparison with
//! the web, the PDFs written.

use super::*;

/// Pictures of the sheet mode in the desktop for the owner: the demo
/// drawing's sheet, its map painted from the drawing's own scene, in the
/// Pafta (light) and Grafit (dark) themes, 1440 and 1100 pixels wide.
/// `cargo test -p kentos-desktop sheet_screens -- --ignored --nocapture`
/// writes `.run/shots/sheet-desktop/masaustu-*.png`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn sheet_screens() {
    let out =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots/sheet-desktop");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    for (theme, tag) in [("light", "pafta"), ("dark", "grafit")] {
        for (w, h) in [(1440.0, 900.0), (1100.0, 720.0)] {
            let mut app = app_with_drawing();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            // The map looks at the drawing: the sheet is made where the drawing area looks.
            if let Some(b) = scene::extents(app.document.as_ref().unwrap()) {
                app.viewport.camera.center =
                    Vec2::new((b.min_x + b.max_x) / 2.0, (b.min_y + b.max_y) / 2.0);
            }
            new_sheet(&mut app);
            let map = app
                .sheets
                .open_sheet()
                .unwrap()
                .items
                .iter()
                .find(|i| i.name == "Harita")
                .map(|i| i.id.clone());
            let _ = app.update(Message::Sheet(Sheet::Select(map.into_iter().collect())));
            let mut snapshot =
                kentos_ui::snapshot::Snapshot::software(Size::new(w, h)).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("masaustu-{tag}-{}.png", w as u32));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}

/// Step 5's windows on the desktop, for the owner beside the web's: the PDF export window
/// over the ifraz sheet, Kullan's questions over the gallery, and a sheet whose template has
/// a newer revision (the tab's dot, the inspector's “Yeni sürüm var”). Light theme first.
///
/// ```text
/// KENTOS_DEMO_DRAWING=$PWD/.run/demo/ornek-1244-1249-ada.json \
///   cargo test -p kentos-desktop sheet_step5_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn sheet_step5_screens() {
    use kentos_sheet_ui::gallery::{GalleryMessage, Source};
    use kentos_sheet_ui::save_template::SaveMessage;
    let out =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots/sheet-desktop");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let (mut app, ifraz) = demo_ifraz("pafta-adim5");
    let sheet = |app: &mut App, m: Sheet| {
        let task = app.update(Message::Sheet(m));
        crate::files_testing::drive(app, task);
    };
    let shoot = |app: &mut App, name: &str| {
        for (theme, tag) in [("light", "pafta"), ("dark", "grafit")] {
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let mut snapshot =
                kentos_ui::snapshot::Snapshot::software(Size::new(1440.0, 900.0))
                    .expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("masaustu-{name}-{tag}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    };
    sheet(&mut app, Sheet::ZoomPage);
    // The PDF window over the ifraz sheet.
    sheet(&mut app, Sheet::Export(kentos_sheet_ui::ExportKind::Pdf));
    shoot(&mut app, "c5-pdf-penceresi");
    sheet(&mut app, Sheet::CloseDialog);
    // Kullan's questions over the gallery (the imar plan asks fifteen).
    for m in [
        GalleryMessage::Open,
        GalleryMessage::AllModes(true),
        GalleryMessage::Pick("sys:imar-plani".into()),
        GalleryMessage::Use,
    ] {
        sheet(&mut app, Sheet::Gallery(m));
    }
    if app.sheets.gallery_asking().is_some() {
        sheet(&mut app, Sheet::Gallery(GalleryMessage::Answer(true)));
    }
    assert!(app.sheets.questions().is_some(), "the questions");
    sheet(
        &mut app,
        Sheet::Questions(QuestionsMessage::Text(0, "Suşehri Belediyesi".into())),
    );
    shoot(&mut app, "c5-sorular");
    sheet(&mut app, Sheet::CloseDialog);
    sheet(&mut app, Sheet::Gallery(GalleryMessage::Close));
    // The ifraz sheet kept as the user's template; a sheet made from it; the template saved
    // again from that sheet: revision 2, and the sheet says “Yeni sürüm var”.
    sheet(&mut app, Sheet::SaveTemplate(SaveMessage::Open));
    sheet(
        &mut app,
        Sheet::SaveTemplate(SaveMessage::Name("Belediye ifraz paftası".into())),
    );
    sheet(&mut app, Sheet::SaveTemplate(SaveMessage::Save));
    let mine = app
        .sheet_store
        .as_ref()
        .and_then(|s| s.templates().into_iter().next())
        .map(|r| r.id)
        .expect("the user's template");
    for m in [
        GalleryMessage::Open,
        GalleryMessage::Source(Source::Mine),
        GalleryMessage::Pick(mine.clone()),
        GalleryMessage::Use,
    ] {
        sheet(&mut app, Sheet::Gallery(m));
    }
    if app.sheets.gallery_asking().is_some() {
        sheet(&mut app, Sheet::Gallery(GalleryMessage::Answer(true)));
    }
    sheet(&mut app, Sheet::Questions(QuestionsMessage::Make));
    let made = app
        .sheets
        .open_sheet()
        .map(|s| s.id.clone())
        .expect("the sheet");
    assert_ne!(made, ifraz);
    sheet(&mut app, Sheet::SaveTemplate(SaveMessage::Open));
    sheet(&mut app, Sheet::SaveTemplate(SaveMessage::Save));
    let s = app.sheets.open_sheet().expect("the sheet").clone();
    assert!(
        app.sheets.template_newer(&s),
        "revision 2 kept, the sheet made from 1"
    );
    sheet(&mut app, Sheet::Select(Vec::new()));
    sheet(&mut app, Sheet::InspectorTab(InspectorTab::Page));
    // The page tab's first sections folded: its Şablon section in sight.
    for section in ["paper", "margins", "variant"] {
        sheet(&mut app, Sheet::Section(section));
    }
    sheet(&mut app, Sheet::ZoomPage);
    shoot(&mut app, "c5-yeni-surum");
}

/// Step 6's north diagram on the desktop (design §8a): the ifraz sheet's north arrow as the
/// Turkish topographic sheets' diagram, the declination from WMM2025 at the map's centre on
/// today's date; the whole sheet, then the diagram close up. Light theme first.
///
/// ```text
/// KENTOS_DEMO_DRAWING=$PWD/.run/demo/ornek-1244-1249-ada.json \
///   cargo test -p kentos-desktop sheet_step6_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn sheet_step6_screens() {
    use kentos_sheet::ops::{Op, SetItemProps};
    let out =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots/sheet-desktop");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let (mut app, ifraz) = demo_ifraz("pafta-adim6");
    let sheet = |app: &mut App, m: Sheet| {
        let task = app.update(Message::Sheet(m));
        crate::files_testing::drive(app, task);
    };
    let arrow = app
        .sheets
        .book()
        .sheet(&ifraz)
        .and_then(|s| {
            s.items
                .iter()
                .find(|i| matches!(i.kind, kentos_sheet::kinds::ItemKind::NorthArrow(_)))
        })
        .map(|i| i.id.clone())
        .expect("the ifraz sheet's north arrow");
    assert!(app.sheets.apply(
        vec![Op::SetItemProps(SetItemProps {
            id: arrow.clone(),
            patch: serde_json::json!({ "kind": { "type": "northArrow", "style": "diagram" } }),
        })],
        "Kuzey çizelgesi",
    ));
    let texts: Vec<String> = app
        .sheets
        .display_list()
        .map(|l| {
            l.prims
                .iter()
                .filter_map(|p| match p {
                    kentos_sheet::display::Prim::Text(t) if t.item == arrow => {
                        Some(t.text.clone())
                    }
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    println!("{texts:?}");
    assert!(texts.iter().any(|t| t.contains("(WMM2025, ")), "{texts:?}");
    let shoot = |app: &mut App, name: &str, zoom: Option<Sheet>| {
        for (theme, tag) in [("light", "pafta"), ("dark", "grafit")] {
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let mut snapshot =
                kentos_ui::snapshot::Snapshot::software(Size::new(1440.0, 900.0))
                    .expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            // The window at this size first (the stage learns its size), then the zoom.
            snapshot.settle(app, App::view, &mut update);
            if let Some(z) = zoom.clone() {
                let task = app.update(Message::Sheet(z));
                crate::files_testing::drive(app, task);
                snapshot.settle(app, App::view, &mut update);
            }
            let file = out.join(format!("masaustu-{name}-{tag}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    };
    sheet(&mut app, Sheet::Select(Vec::new()));
    shoot(&mut app, "c6-kuzey-cizelgesi", Some(Sheet::ZoomPage));
    sheet(&mut app, Sheet::Select(vec![arrow]));
    shoot(
        &mut app,
        "c6-kuzey-cizelgesi-yakin",
        Some(Sheet::ZoomSelection),
    );
}

/// The files the acceptance tools read (gdalinfo, pdffonts, pdftotext, pdfinfo, pdftoppm):
/// the ifraz system template on the web's demo drawing, written by the desktop's own export.
///
/// ```text
/// KENTOS_DEMO_DRAWING=$PWD/.run/demo/ornek-1244-1249-ada.json \
///   cargo test -p kentos-desktop sheet_pdf_files -- --ignored --nocapture
/// ```
#[test]
#[ignore = "files for the acceptance tools, run by hand"]
fn sheet_pdf_files() {
    let out =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots/sheet-pdf");
    std::fs::create_dir_all(&out).expect("a folder for the files");
    let (mut app, ifraz) = demo_ifraz("pafta-pdf-kabul");
    assert_eq!(
        app.sheets.open_sheet().map(|s| s.id.clone()),
        Some(ifraz.clone())
    );
    let _ = app.update(Message::Sheet(Sheet::Export(
        kentos_sheet_ui::ExportKind::Pdf,
    )));
    let file = out.join("masaustu-ifraz.pdf");
    app.sheet_export_to(kentos_sheet_ui::ExportKind::Pdf, Some(file.clone()));
    assert!(
        std::fs::read(&file)
            .expect("written")
            .starts_with(b"%PDF-1.7")
    );
    // The map frames as the sheet places them, for the corner check against `gdalinfo`.
    let list = kentos_sheet::display::display_list(
        app.sheets.book(),
        &ifraz,
        &app.sheets.export_inputs(),
    )
    .expect("the display list");
    let maps: Vec<&kentos_sheet::display::MapPrim> = list
        .prims
        .iter()
        .filter_map(|p| match p {
            kentos_sheet::display::Prim::Map(m) => Some(m),
            _ => None,
        })
        .collect();
    std::fs::write(
        out.join("masaustu-ifraz-haritalar.json"),
        serde_json::to_string_pretty(&maps).expect("the maps"),
    )
    .expect("written");
    println!("{}", file.display());
}

/// The web's demo drawing (`Ornek_1244-1249_Ada`, written by
/// apps/web/scripts/style/demo-drawing.test.ts) open in the CAD mode, the drawing area on
/// its home view, and the ifraz sheet from the system template with the web's values and
/// parcel table columns (sheet-shots.mjs `VALUES`, `COLUMNS`).
pub(super) fn demo_ifraz(store: &str) -> (App, String) {
    use kentos_sheet::ops::Op;
    use kentos_sheet_ui::gallery::GalleryMessage;
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let path = std::env::var("KENTOS_DEMO_DRAWING").unwrap_or_else(|_| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../.run/demo/ornek-1244-1249-ada.json"
        )
        .to_owned()
    });
    let text =
        std::fs::read_to_string(&path).expect("the web's demo drawing (see the comment)");
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&text).expect("reads");
    let mut doc = Document::new(snapshot, None).expect("opens");
    doc.model.set_name("Ornek_1244-1249_Ada.kcad");
    let mut settings = doc.settings().clone();
    settings.workspace = Some(kentos_commands::Workspace::Cad);
    doc.model.set_settings(settings);
    let home = doc.model.home_view();
    let (mut app, _) = App::boot(None);
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.sheet_store = Some(
        kentos_sheet_ui::Store::open(&crate::files_testing::scratch(store)).expect("a store"),
    );
    // The sheets look where the web's do: the drawing's home view.
    if let Some(h) = home {
        app.viewport.camera.center =
            Vec2::new((h.min_x + h.max_x) / 2.0, (h.min_y + h.max_y) / 2.0);
    }
    let sheet = |app: &mut App, m: Sheet| {
        let task = app.update(Message::Sheet(m));
        crate::files_testing::drive(app, task);
    };
    let gallery = |app: &mut App, m: GalleryMessage| sheet(app, Sheet::Gallery(m));
    let from = |app: &mut App, id: &str| {
        gallery(app, GalleryMessage::Open);
        gallery(app, GalleryMessage::Pick(id.into()));
        gallery(app, GalleryMessage::Use);
        if app.sheets.gallery_asking().is_some() {
            gallery(app, GalleryMessage::Answer(true));
        }
        // The template's questions, answered with its own values (set below as the web's).
        if app.sheets.questions().is_some() {
            sheet(app, Sheet::Questions(QuestionsMessage::Make));
        }
        app.sheets
            .open_sheet()
            .map(|s| s.id.clone())
            .expect("a sheet")
    };
    let ifraz = from(&mut app, "sys:ifraz-paftasi");
    // The web's values and the parcel table's columns (sheet-shots.mjs `VALUES`, `COLUMNS`).
    let values = serde_json::json!({ "il": "Sivas", "ilce": "Suşehri", "mahalle": "Kızılırmak", "ada": "1245", "pafta_no": "P-12" });
    let s = app.sheets.open_sheet().expect("the ifraz sheet").clone();
    let variables: Vec<serde_json::Value> = s
        .variables
        .iter()
        .map(|v| {
            let mut j = serde_json::to_value(v).expect("a variable");
            if let Some(x) = values.get(&v.name) {
                j["value"] = x.clone();
            }
            j
        })
        .collect();
    let table = s
        .items
        .iter()
        .find(|i| matches!(i.kind, kentos_sheet::kinds::ItemKind::Table(_)))
        .map(|i| i.id.clone())
        .expect("the parcel table");
    let ops: Vec<Op> = serde_json::from_value(serde_json::json!([
        { "op": "saveVariables", "sheet": ifraz, "variables": variables },
        { "op": "setItemProps", "id": table, "patch": { "kind": {
            "columns": [
                { "heading": "Parsel", "value": "Parsel", "width": 13000, "align": "center" },
                { "heading": "Yüzölçümü (m²)", "value": "$alan", "width": 0, "align": "right", "decimals": 2 },
                { "heading": "Tapu alanı (m²)", "value": "[Tapu alanı (m²)]", "width": 0, "align": "right", "decimals": 2 },
                { "heading": "Nitelik", "value": "Nitelik", "width": 0, "align": "left" }
            ],
            "source": { "sort": [{ "expression": "to_int(Parsel)", "descending": false }] }
        } } }
    ]))
    .expect("the web's operations");
    assert!(
        app.sheets.apply(ops, "Değişkenler ve parsel tablosu"),
        "{:?}",
        app.sheets.error()
    );
    (app, ifraz)
}

/// Pictures of the desktop for setting beside the web's
/// (apps/web/scripts/e2e/sheet-shots.mjs, `b1`, `b4`): the web's demo
/// drawing (`Ornek_1244-1249_Ada`, written by
/// apps/web/scripts/style/demo-drawing.test.ts), in the CAD mode, the ifraz
/// sheet from the system template with the web's values and parcel table
/// columns, an aplikasyon sketch beside it, a template of the user's own
/// saved from it; the sheet, then the gallery's Sistem and Benim, light
/// (Pafta) first, then Grafit, 1440 × 900 and 1100 × 650.
///
/// ```text
/// KENTOS_DEMO_DRAWING=$PWD/.run/demo/ornek-1244-1249-ada.json \
///   cargo test -p kentos-desktop sheet_comparison_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn sheet_comparison_screens() {
    use kentos_sheet::ops::Op;
    use kentos_sheet_ui::gallery::{GalleryMessage, Source};
    use kentos_sheet_ui::save_template::SaveMessage;
    let out =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots/sheet-desktop");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let (mut app, ifraz) = demo_ifraz("pafta-karsilastirma");
    let sheet = |app: &mut App, m: Sheet| {
        let task = app.update(Message::Sheet(m));
        crate::files_testing::drive(app, task);
    };
    let gallery = |app: &mut App, m: GalleryMessage| sheet(app, Sheet::Gallery(m));
    let from = |app: &mut App, id: &str| {
        gallery(app, GalleryMessage::Open);
        gallery(app, GalleryMessage::Pick(id.into()));
        gallery(app, GalleryMessage::Use);
        if app.sheets.gallery_asking().is_some() {
            gallery(app, GalleryMessage::Answer(true));
        }
        // The template's questions, answered with its own values (set below as the web's).
        if app.sheets.questions().is_some() {
            sheet(app, Sheet::Questions(QuestionsMessage::Make));
        }
        app.sheets
            .open_sheet()
            .map(|s| s.id.clone())
            .expect("a sheet")
    };
    // A template of the user's own, as the web's run saves one.
    sheet(&mut app, Sheet::SaveTemplate(SaveMessage::Open));
    sheet(
        &mut app,
        Sheet::SaveTemplate(SaveMessage::Name("Belediye ifraz paftası".into())),
    );
    sheet(&mut app, Sheet::SaveTemplate(SaveMessage::Description(
        "Belediyenin ifraz paftası: Kızılırmak mahallesi düzeni, parsel tablosu ve imzalar.".into(),
    )));
    sheet(
        &mut app,
        Sheet::SaveTemplate(SaveMessage::Category("kadastro".into())),
    );
    sheet(
        &mut app,
        Sheet::SaveTemplate(SaveMessage::Tags("ifraz, belediye".into())),
    );
    sheet(&mut app, Sheet::SaveTemplate(SaveMessage::Save));
    let _aplikasyon = from(&mut app, "sys:aplikasyon-krokisi");
    let at = app
        .sheets
        .book()
        .sheets
        .iter()
        .position(|s| s.id == ifraz)
        .expect("the ifraz sheet");
    sheet(&mut app, Sheet::Tab(at + 1));
    sheet(&mut app, Sheet::Select(Vec::new()));
    sheet(&mut app, Sheet::ZoomPage);
    // What the preflight finds on the ifraz sheet, for the comparison with the web's.
    for f in app.sheets.findings() {
        println!("Ön denetim: {:?} {} — {}", f.severity, f.code, f.message);
    }
    let mine = app
        .sheet_store
        .as_ref()
        .and_then(|s| s.templates().into_iter().next())
        .map(|r| r.id)
        .expect("the user's template");
    // A picture frame in the title row given an institution's logo, as Resim seç… gives
    // it; then Seçim (the frame fills the stage) so the picture is seen close.
    let logo = logo_png(512);
    let title = app
        .sheets
        .book()
        .sheet(&ifraz)
        .and_then(|s| s.items.iter().find(|i| i.name == "Başlık ve yer"))
        .map(|i| i.frame)
        .expect("the ifraz sheet's title block");
    let ifraz_sheet = ifraz.clone();
    let with_logo = move |app: &mut App| {
        use kentos_sheet::kinds::{ItemKind, PictureFit, PictureItem};
        let frame = kentos_sheet::units::RectUm {
            left: title.left + 2_000,
            top: title.top + 1_500,
            width: 10_000,
            height: 10_000,
        };
        let picture = kentos_sheet::model::Item::new(
            "logo",
            "Kurum logosu",
            frame,
            ItemKind::Picture(PictureItem {
                asset: None,
                fit: PictureFit::Contain,
                clip: true,
            }),
        );
        assert!(app.sheets.apply(
            vec![Op::AddItems(kentos_sheet::ops::AddItems {
                to: kentos_sheet::model::Owner::sheet(&ifraz_sheet),
                items: vec![picture],
                index: None,
            })],
            "Resim çerçevesi",
        ));
        sheet(app, Sheet::Select(vec!["logo".into()]));
        sheet(
            app,
            Sheet::PictureFile("kurum-logosu.png".into(), logo.clone()),
        );
        sheet(app, Sheet::ZoomSelection);
    };
    let without_logo = move |app: &mut App| {
        sheet(app, Sheet::Undo);
        sheet(app, Sheet::Undo);
        sheet(app, Sheet::Select(Vec::new()));
        sheet(app, Sheet::ZoomPage);
    };
    let close_gallery = |app: &mut App| {
        let t = app.update(Message::Sheet(Sheet::Gallery(GalleryMessage::Close)));
        crate::files_testing::drive(app, t);
    };
    type Scene = (&'static str, Box<dyn Fn(&mut App)>, Box<dyn Fn(&mut App)>);
    let scenes: Vec<Scene> = vec![
        (
            "b1-ifraz-sistem-sablonu",
            Box::new(|_: &mut App| {}),
            Box::new(close_gallery),
        ),
        (
            "b4-galeri-sistem",
            Box::new(move |app: &mut App| {
                let t = app.update(Message::Sheet(Sheet::Gallery(GalleryMessage::Open)));
                crate::files_testing::drive(app, t);
                let t = app.update(Message::Sheet(Sheet::Gallery(GalleryMessage::Pick(
                    "sys:ifraz-paftasi".into(),
                ))));
                crate::files_testing::drive(app, t);
            }),
            Box::new(close_gallery),
        ),
        (
            "b4-galeri-benim",
            Box::new(move |app: &mut App| {
                for m in [
                    GalleryMessage::Open,
                    GalleryMessage::Source(Source::Mine),
                    GalleryMessage::Pick(mine.clone()),
                ] {
                    let t = app.update(Message::Sheet(Sheet::Gallery(m)));
                    crate::files_testing::drive(app, t);
                }
            }),
            Box::new(close_gallery),
        ),
        ("resim-logo", Box::new(with_logo), Box::new(without_logo)),
    ];
    for (theme, tag) in [("light", "pafta"), ("dark", "grafit")] {
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        for (scene, open, close) in &scenes {
            for (w, h) in [(1440.0, 900.0), (1100.0, 650.0)] {
                let mut snapshot = kentos_ui::snapshot::Snapshot::software(Size::new(w, h))
                    .expect("a renderer");
                let mut update = |app: &mut App, m: Message| {
                    let _ = app.update(m);
                };
                // The window at this size first (the stage learns its size), then the scene.
                snapshot.settle(&mut app, App::view, &mut update);
                open(&mut app);
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("masaustu-{scene}-{tag}-{}.png", w as u32));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
                // Each scene from the sheet in front.
                close(&mut app);
            }
        }
    }
}

/// Step 8's pictures (design, open question 6): in the ifraz sheet's title row a small
/// logo (a 64-pixel PNG) and the KentOS sign as an SVG, then seen close (Seçim: the two
/// frames fill the stage): smooth, not squares; the SVG drawn by resvg. The PNG export of the
/// same sheet, cut round the two pictures, beside it.
///
/// ```text
/// cargo test -p kentos-desktop sheet_step8_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn sheet_step8_screens() {
    use kentos_sheet::kinds::{ItemKind, PictureFit, PictureItem};
    use kentos_sheet::ops::Op;
    let out =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots/sheet-desktop");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let (mut app, ifraz) = demo_ifraz("pafta-adim8");
    let sheet = |app: &mut App, m: Sheet| {
        let task = app.update(Message::Sheet(m));
        crate::files_testing::drive(app, task);
    };
    let title = app
        .sheets
        .book()
        .sheet(&ifraz)
        .and_then(|s| s.items.iter().find(|i| i.name == "Başlık ve yer"))
        .map(|i| i.frame)
        .expect("the ifraz sheet's title block");
    let sign = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/public/favicon.svg"),
    )
    .expect("the KentOS sign");
    let pictures = [
        (
            "logo",
            "Kurum logosu",
            2_000,
            "kurum-logosu.png",
            logo_png(64),
        ),
        ("isaret", "KentOS işareti", 14_000, "kentos.svg", sign),
    ];
    for (id, name, dx, file, bytes) in pictures {
        let frame = kentos_sheet::units::RectUm {
            left: title.left + dx,
            top: title.top + 1_500,
            width: 10_000,
            height: 10_000,
        };
        let item = kentos_sheet::model::Item::new(
            id,
            name,
            frame,
            ItemKind::Picture(PictureItem {
                asset: None,
                fit: PictureFit::Contain,
                clip: true,
            }),
        );
        assert!(app.sheets.apply(
            vec![Op::AddItems(kentos_sheet::ops::AddItems {
                to: kentos_sheet::model::Owner::sheet(&ifraz),
                items: vec![item],
                index: None,
            })],
            "Resim çerçevesi",
        ));
        sheet(&mut app, Sheet::Select(vec![id.into()]));
        sheet(&mut app, Sheet::PictureFile(file.into(), bytes));
    }
    let kinds: Vec<_> = app
        .sheets
        .book()
        .assets
        .iter()
        .map(|a| (a.kind, a.width, a.height))
        .collect();
    println!("{kinds:?}");
    assert!(kinds.contains(&(kentos_sheet::model::AssetKind::Png, 64, 64)));
    assert!(kinds.contains(&(kentos_sheet::model::AssetKind::Svg, 32, 32)));
    sheet(
        &mut app,
        Sheet::Select(vec!["logo".into(), "isaret".into()]),
    );
    for (theme, tag) in [("light", "pafta"), ("dark", "grafit")] {
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = kentos_ui::snapshot::Snapshot::software(Size::new(1440.0, 900.0))
            .expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let task = app.update(Message::Sheet(Sheet::ZoomSelection));
        crate::files_testing::drive(&mut app, task);
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("masaustu-c8-resim-yakin-{tag}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());
    }
    // The PNG export at 400 dpi (the pictures magnified there too), cut round the two pictures.
    sheet(&mut app, Sheet::ExportDpi(400));
    let png = out.join("masaustu-c8-resim-disa-aktarim.png");
    app.sheet_export_to(ExportKind::Png, Some(png.clone()));
    let bytes = std::fs::read(&png).expect("the export");
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().expect("a PNG");
    let mut buf = vec![0; reader.output_buffer_size().expect("a size")];
    let info = reader.next_frame(&mut buf).expect("its pixels");
    let page = app
        .sheets
        .book()
        .sheet(&ifraz)
        .map(|s| s.page.size)
        .expect("a page");
    let k = f64::from(info.width) / f64::from(page.width);
    let px = |um: i32| (f64::from(um) * k).round() as u32;
    let (x0, y0) = (px(title.left + 1_000), px(title.top + 500));
    let (cw, ch) = (px(24_000), px(12_000));
    let channels = buf.len() / (info.width as usize * info.height as usize);
    let mut crop = Vec::with_capacity((cw * ch * 4) as usize);
    for y in y0..y0 + ch {
        for x in x0..x0 + cw {
            let i = (y as usize * info.width as usize + x as usize) * channels;
            let p = &buf[i..i + channels];
            crop.extend_from_slice(&[p[0], p[1], p[2], if channels == 4 { p[3] } else { 255 }]);
        }
    }
    let cut = out.join("masaustu-c8-resim-disa-aktarim-kesit.png");
    let mut file = std::io::BufWriter::new(std::fs::File::create(&cut).expect("a file"));
    let mut e = png::Encoder::new(&mut file, cw, ch);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.write_header()
        .and_then(|mut w| w.write_image_data(&crop))
        .expect("writes the cut");
    println!(
        "{} ({}×{} px, {} dpi)",
        cut.display(),
        info.width,
        info.height,
        (k * 25_400.0).round()
    );
}

/// Step 9's pictures: the north arrow's declination in the inspector (the ifraz sheet's arrow
/// on magnetic north: what the paper writes and from what, the date and where it comes from);
/// the ifraz map with the demo's buildings in a pattern fill (no vector form: the styled
/// shaders' CPU twin draws the map on the screen) and the same map cut from its PDF; an SVG
/// logo in the title row cut from the PDF (drawn here as a PNG, the core draws no SVG).
///
/// ```text
/// cargo test -p kentos-desktop sheet_step9_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn sheet_step9_screens() {
    use kentos_sheet::kinds::{ItemKind, PictureFit, PictureItem};
    use kentos_sheet::ops::{Op, SetItemProps};
    let out =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots/sheet-desktop");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let (mut app, ifraz) = demo_ifraz("pafta-adim9");
    let sheet = |app: &mut App, m: Sheet| {
        let task = app.update(Message::Sheet(m));
        crate::files_testing::drive(app, task);
    };
    let shoot = |app: &mut App, name: &str, zoom: Option<Sheet>| {
        for (theme, tag) in [("light", "pafta"), ("dark", "grafit")] {
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let mut snapshot =
                kentos_ui::snapshot::Snapshot::software(Size::new(1440.0, 900.0))
                    .expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(app, App::view, &mut update);
            if let Some(z) = zoom.clone() {
                let task = app.update(Message::Sheet(z));
                crate::files_testing::drive(app, task);
                snapshot.settle(app, App::view, &mut update);
            }
            let file = out.join(format!("masaustu-{name}-{tag}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from("light"))]);
        app.apply_settings();
    };
    let items = |app: &App| {
        app.sheets
            .book()
            .sheet(&ifraz)
            .map(|s| s.items.clone())
            .unwrap_or_default()
    };
    // 1. The north arrow on magnetic north, chosen: its declination in the inspector.
    let arrow = items(&app)
        .iter()
        .find(|i| matches!(i.kind, ItemKind::NorthArrow(_)))
        .map(|i| i.id.clone())
        .expect("the ifraz sheet's north arrow");
    assert!(app.sheets.apply(
        vec![Op::SetItemProps(SetItemProps {
            id: arrow.clone(),
            patch: serde_json::json!({ "kind": { "type": "northArrow", "north": "magnetic" } }),
        })],
        "Kuzey",
    ));
    sheet(&mut app, Sheet::Select(vec![arrow]));
    // The frame's and the constraints' sections closed: the arrow's own fields in view.
    sheet(&mut app, Sheet::Section("frame"));
    sheet(&mut app, Sheet::Section("constraints"));
    let info = app
        .sheets
        .north_info()
        .cloned()
        .expect("what the arrow shows");
    println!(
        "Kuzey: {:?} {:?} {:?} {:?}",
        info.declination, info.model, info.date, info.date_source
    );
    assert_eq!(
        info.source,
        Some(kentos_sheet::display::DeclinationSource::Model)
    );
    shoot(&mut app, "c9-kuzey-denetci", Some(Sheet::ZoomPage));
    sheet(&mut app, Sheet::Section("frame"));
    sheet(&mut app, Sheet::Section("constraints"));
    sheet(&mut app, Sheet::Select(Vec::new()));
    // 2. The demo's buildings in a pattern fill: a light fill, small crosses, the outline.
    {
        let doc = app.document.as_mut().expect("the demo drawing");
        let mut style = doc
            .model
            .layers()
            .get("yapi")
            .map(|l| l.style.clone())
            .expect("the Yapı layer");
        style.renderer = Some(
            serde_json::json!({ "type": "single", "symbols": { "fill": { "type": "fill", "layers": [
            { "id": "f", "type": "simpleFill", "color": "#DCE8F6" },
            { "id": "p", "type": "patternFill", "spacingX": 1.6, "spacingY": 1.6, "stagger": true,
              "marker": { "type": "marker", "layers": [{ "id": "x", "type": "shape", "shape": "x", "size": 0.7, "fill": "#1F4A96", "stroke": null }] } },
            { "id": "o", "type": "simpleLine", "color": "#1F4A96", "width": 0.3 }
        ] } } }),
        );
        assert!(doc.model.set_layer_style("yapi", style, "Desenli yapılar"));
    }
    app.sheet_maps.sync(app.document.as_ref());
    let ways = app.sheets.pdf_ways(&app.sheet_painter());
    println!("{ways:?}");
    assert!(
        ways.iter()
            .any(|w| !w.vector && w.why.contains("desen dolgusu"))
    );
    let map = items(&app)
        .iter()
        .find(|i| matches!(i.kind, ItemKind::Map(_)))
        .map(|i| (i.id.clone(), i.frame))
        .expect("the ifraz sheet's map");
    sheet(&mut app, Sheet::Select(vec![map.0.clone()]));
    shoot(&mut app, "c9-desenli-harita", Some(Sheet::ZoomSelection));
    sheet(&mut app, Sheet::Select(Vec::new()));
    // 3. An SVG logo in the title row (the KentOS sign).
    let title = items(&app)
        .iter()
        .find(|i| i.name == "Başlık ve yer")
        .map(|i| i.frame)
        .expect("the ifraz sheet's title block");
    let sign = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/public/favicon.svg"),
    )
    .expect("the KentOS sign");
    let logo = kentos_sheet::units::RectUm {
        left: title.left + 2_000,
        top: title.top + 1_500,
        width: 10_000,
        height: 10_000,
    };
    assert!(app.sheets.apply(
        vec![Op::AddItems(kentos_sheet::ops::AddItems {
            to: kentos_sheet::model::Owner::sheet(&ifraz),
            items: vec![kentos_sheet::model::Item::new(
                "isaret",
                "KentOS işareti",
                logo,
                ItemKind::Picture(PictureItem {
                    asset: None,
                    fit: PictureFit::Contain,
                    clip: true,
                }),
            )],
            index: None,
        })],
        "Resim çerçevesi",
    ));
    sheet(&mut app, Sheet::Select(vec!["isaret".into()]));
    sheet(&mut app, Sheet::PictureFile("kentos.svg".into(), sign));
    sheet(&mut app, Sheet::Select(Vec::new()));
    // The PDF at 200 dpi (the map's picture and the SVG's PNG at that resolution).
    sheet(&mut app, Sheet::ExportDpi(200));
    let _ = app.update(Message::Sheet(Sheet::Export(ExportKind::Pdf)));
    let pdf = out.join("masaustu-c9.pdf");
    app.sheet_export_to(ExportKind::Pdf, Some(pdf.clone()));
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    println!("{:?}", &said[said.len().saturating_sub(4)..]);
    assert!(
        said.iter()
            .any(|l| l.contains("SVG resim PDF'e resim olarak gömülür")),
        "{said:?}"
    );
    // Cuts of the page at 200 dpi: the map's south-west corner with buildings, and the logo.
    let cut = |name: &str, r: kentos_sheet::units::RectUm| {
        let px = |um: i32| (f64::from(um) / 25_400.0 * 200.0).round() as i64;
        let file = out.join(name);
        let made = std::process::Command::new("pdftoppm")
            .args(["-r", "200", "-png", "-singlefile", "-f", "1", "-l", "1"])
            .args(["-x", &px(r.left).to_string(), "-y", &px(r.top).to_string()])
            .args([
                "-W",
                &px(r.width).to_string(),
                "-H",
                &px(r.height).to_string(),
            ])
            .arg(&pdf)
            .arg(file.with_extension(""))
            .status();
        if made.is_ok_and(|s| s.success()) {
            println!("{}", file.display());
        }
    };
    let (_, m) = map;
    cut(
        "masaustu-c9-desenli-harita-pdf.png",
        kentos_sheet::units::RectUm {
            left: m.left + m.width / 4,
            top: m.top + m.height / 4,
            width: m.width / 2,
            height: m.height / 2,
        },
    );
    cut(
        "masaustu-c9-svg-pdf.png",
        kentos_sheet::units::RectUm {
            left: logo.left - 2_000,
            top: logo.top - 1_500,
            width: logo.width + 4_000,
            height: logo.height + 3_000,
        },
    );
}

/// An institution's emblem for the picture scene: a disc shading from navy
/// to teal, a white ring and a north arrow, the edges smoothed (16 samples
/// a pixel), the corners transparent.
pub(super) fn logo_png(side: u32) -> Vec<u8> {
    let s = f64::from(side);
    let c = s / 2.0;
    // Inside triangle a-b-c (either winding).
    let tri = |p: [f64; 2], a: [f64; 2], b: [f64; 2], d: [f64; 2]| {
        let side = |u: [f64; 2], v: [f64; 2]| {
            (v[0] - u[0]) * (p[1] - u[1]) - (v[1] - u[1]) * (p[0] - u[0])
        };
        let (x, y, z) = (side(a, b), side(b, d), side(d, a));
        (x >= 0.0 && y >= 0.0 && z >= 0.0) || (x <= 0.0 && y <= 0.0 && z <= 0.0)
    };
    let (apex, left, notch, right) = (
        [c, 0.2 * s],
        [c - 0.15 * s, 0.72 * s],
        [c, 0.6 * s],
        [c + 0.15 * s, 0.72 * s],
    );
    let mut px = Vec::with_capacity((side * side * 4) as usize);
    for j in 0..side {
        for i in 0..side {
            let mut sum = [0.0f64; 4];
            for k in 0..16 {
                let p = [
                    f64::from(i) + (f64::from(k % 4) + 0.5) / 4.0,
                    f64::from(j) + (f64::from(k / 4) + 0.5) / 4.0,
                ];
                let r = ((p[0] - c).powi(2) + (p[1] - c).powi(2)).sqrt();
                if r > 0.47 * s {
                    continue;
                }
                let white = (0.38 * s..0.41 * s).contains(&r)
                    || tri(p, apex, left, notch)
                    || tri(p, apex, notch, right);
                let t = p[1] / s;
                let rgb = if white {
                    [255.0, 255.0, 255.0]
                } else {
                    [29.0 - 3.0 * t, 59.0 + 68.0 * t, 114.0 + 24.0 * t]
                };
                sum = [
                    sum[0] + rgb[0],
                    sum[1] + rgb[1],
                    sum[2] + rgb[2],
                    sum[3] + 1.0,
                ];
            }
            let n = sum[3].max(1.0);
            px.extend([
                (sum[0] / n).round() as u8,
                (sum[1] / n).round() as u8,
                (sum[2] / n).round() as u8,
                (sum[3] / 16.0 * 255.0).round() as u8,
            ]);
        }
    }
    let mut out = Vec::new();
    {
        let mut e = png::Encoder::new(&mut out, side, side);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        let mut w = e.write_header().expect("a header");
        w.write_image_data(&px).expect("the pixels");
    }
    out
}
