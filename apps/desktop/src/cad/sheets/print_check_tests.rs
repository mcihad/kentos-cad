//! The municipal report's 15th item in the sheet mode (docs/adr/9147): Yeni pafta's extent chosen in the window of the
//! sheet's questions, from the desktop's own context (its view, its layers' boxes, the drawing's box): the whole
//! drawing fits and the print check says nothing of a cut, the fixed scale keeps the cut as a warning; two sheets with
//! Turkish letters in their titles pass the check's glyphs and go into one PDF of two pages; the legend carries the
//! shown layers alone, by their names.

use kentos_sheet_ui::Message as Sheet;
use kentos_sheet_ui::new_extent::Extent;
use kentos_sheet_ui::questions::QuestionsMessage;

use crate::app::{App, Message};
use crate::files_testing::app_with_drawing;

/// The sample drawing at 1:100 with the drawing area zoomed in on one corner of it: the plot scale leaves the rest
/// out.
fn zoomed_in() -> App {
    let mut app = app_with_drawing();
    let doc = app.document.as_mut().expect("open");
    let mut settings = doc.settings().clone();
    settings.plot_scale = 100.0;
    doc.model.set_settings(settings);
    let b = kentos_native_application::sheets_fit::object_extent(
        &app.document.as_ref().expect("open").model,
        None,
    )
    .ok()
    .flatten()
    .expect("the drawing's extent");
    app.viewport.camera.set_size(1200.0, 800.0);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: b[0] - 2.0,
            min_y: b[1] - 2.0,
            max_x: b[0] + 2.0,
            max_y: b[1] + 2.0,
        },
        0.0,
    );
    app
}

fn ask(app: &mut App, m: QuestionsMessage) {
    let _ = app.update(Message::Sheet(Sheet::Questions(m)));
}

fn cut(app: &App) -> bool {
    app.sheets
        .findings()
        .iter()
        .any(|f| f.code == "drawing_cut")
}

#[test]
fn the_whole_drawing_chosen_in_the_window_leaves_nothing_out_and_the_fixed_scale_keeps_the_cut() {
    // The fixed scale, as before: the cut kept, as a warning the print check says.
    let mut app = zoomed_in();
    let _ = app.update(Message::Sheet(Sheet::NewSheet));
    let q = app.sheets.questions().expect("the questions");
    assert_eq!(q.extent, Some(Extent::Fixed));
    assert!(
        q.placed.1.contains("Kırpma bilerek korunur."),
        "{}",
        q.placed.1
    );
    ask(&mut app, QuestionsMessage::Make);
    assert!(cut(&app), "the cut kept: {:?}", app.sheets.findings());
    // The whole drawing: fitted, nothing left out.
    let mut app = zoomed_in();
    let _ = app.update(Message::Sheet(Sheet::NewSheet));
    ask(&mut app, QuestionsMessage::Extent(Extent::Drawing));
    let said = app
        .sheets
        .questions()
        .expect("the questions")
        .placed
        .1
        .clone();
    assert!(
        said.starts_with("Çizimin tamamı %5 payla sığar: ana harita 1:"),
        "{said}"
    );
    ask(&mut app, QuestionsMessage::Make);
    assert!(!cut(&app), "fitted: {:?}", app.sheets.findings());
    // A layer's box comes from the desktop: the parcel layer is one of them.
    let mut app = zoomed_in();
    let _ = app.update(Message::Sheet(Sheet::NewSheet));
    assert!(
        app.sheets
            .context()
            .layer_bounds
            .iter()
            .any(|(id, _)| id == "parsel"),
        "{:?}",
        app.sheets.context().layer_bounds
    );
    ask(&mut app, QuestionsMessage::Extent(Extent::Layer));
    let said = app
        .sheets
        .questions()
        .expect("the questions")
        .placed
        .1
        .clone();
    assert!(
        said.contains("katmanının nesneleri %5 payla sığar: ana harita 1:"),
        "{said}"
    );
}

#[test]
fn two_sheets_with_turkish_titles_pass_the_glyph_check_and_print_as_two_pages() {
    use kentos_sheet_ui::message::{PdfMessage, PdfScope};
    let mut app = app_with_drawing();
    for title in [
        "Çiğdem Sokağı İmar Değişikliği",
        "ĞÜŞİÖÇ ğüşıöç — Kuzey Paftası",
    ] {
        let _ = app.update(Message::Sheet(Sheet::NewSheet));
        ask(&mut app, QuestionsMessage::Extent(Extent::Drawing));
        let q = app.sheets.questions().expect("the questions");
        let first = q
            .template_variables()
            .iter()
            .position(|v| v.kind == kentos_sheet::model::VarKind::Text)
            .expect("a text question");
        ask(&mut app, QuestionsMessage::Text(first, title.to_owned()));
        ask(&mut app, QuestionsMessage::Make);
        assert!(
            !app.sheets
                .findings()
                .iter()
                .any(|f| f.code == "glyph_missing"),
            "{title}: {:?}",
            app.sheets.findings()
        );
    }
    assert_eq!(app.sheets.book().sheets.len(), 2);
    let _ = app.update(Message::Sheet(Sheet::Export(
        kentos_sheet_ui::ExportKind::Pdf,
    )));
    let _ = app.update(Message::Sheet(Sheet::Pdf(PdfMessage::Scope(PdfScope::All))));
    let file = crate::files_testing::scratch("pafta-iki").join("paftalar.pdf");
    app.sheet_export_to(kentos_sheet_ui::ExportKind::Pdf, Some(file.clone()));
    let pdf = std::fs::read(&file).expect("the PDF is written");
    let text = String::from_utf8_lossy(&pdf);
    let pages = text.matches("/Type /Page").count() - text.matches("/Type /Pages").count();
    assert_eq!(pages, 2, "one page a sheet");
}

/// The legend of a new sheet carries the shown layers alone, by their names: the sample drawing's Çizim is hidden,
/// Parsel and Bina are shown.
#[test]
fn the_legend_carries_the_shown_layers_by_name() {
    let mut app = app_with_drawing();
    let _ = app.update(Message::Sheet(Sheet::NewSheet));
    ask(&mut app, QuestionsMessage::Extent(Extent::Drawing));
    ask(&mut app, QuestionsMessage::Make);
    let legend = |app: &App| -> Vec<(String, String)> {
        app.sheets
            .data()
            .legends
            .iter()
            .flat_map(|l| l.entries.iter().map(|e| (e.layer.clone(), e.label.clone())))
            .collect()
    };
    let rows = legend(&app);
    assert!(
        rows.iter()
            .any(|(id, label)| id == "parsel" && label == "Parsel"),
        "{rows:?}"
    );
    // Bina holds areas and lines: a row each, by the layer's name and the kind.
    let bina: Vec<&str> = rows
        .iter()
        .filter(|(id, _)| id == "bina")
        .map(|(_, l)| l.as_str())
        .collect();
    assert_eq!(bina, ["Bina (alan)", "Bina (çizgi)"], "{rows:?}");
    assert!(
        !rows.iter().any(|(id, _)| id == "cizim"),
        "a hidden layer is not in the legend: {rows:?}"
    );
    assert!(
        rows.iter().all(|(id, label)| label != id),
        "names, not ids: {rows:?}"
    );
}

/// A reprint with a data layer's source gone (docs/adr/9187): the print check says the layer out of reach on the map,
/// the PDF is still written (without its rows); the file back, nothing is said.
#[test]
fn a_reprint_with_a_data_layers_source_gone_says_it_and_prints_once_back() {
    crate::data_layers::register();
    let path = crate::data_layers_tests::parcels("yeniden-basim", 4);
    let mut app = app_with_drawing();
    app.add_data_layers(vec![
        crate::data_layers::describe(&path.display().to_string(), "parseller").expect("described"),
    ])
    .expect("added");
    let _ = app.update(Message::Sheet(Sheet::NewSheet));
    ask(&mut app, QuestionsMessage::Extent(Extent::Drawing));
    ask(&mut app, QuestionsMessage::Make);
    let unreachable = |app: &App| -> Vec<String> {
        app.sheets
            .findings()
            .iter()
            .filter(|f| f.code == "layer_unreachable")
            .map(|f| f.message.clone())
            .collect()
    };
    assert!(unreachable(&app).is_empty(), "{:?}", unreachable(&app));
    // The source moved away: said, as a warning on the map.
    let away = path.with_extension("tasindi");
    std::fs::rename(&path, &away).expect("moved");
    app.follow_sheet_data();
    let said = unreachable(&app);
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(
        said[0].starts_with("“parseller” katmanı basılamıyor: kaynağı olan “")
            && said[0].ends_with("” dosyası yok. Haritada boş kalır."),
        "{said:?}"
    );
    let file = crate::files_testing::scratch("yeniden-basim-pdf").join("pafta.pdf");
    app.sheet_export_to(kentos_sheet_ui::ExportKind::Pdf, Some(file.clone()));
    assert!(
        std::fs::read(&file)
            .expect("still printed")
            .starts_with(b"%PDF-")
    );
    // Back: nothing said.
    std::fs::rename(&away, &path).expect("moved back");
    app.follow_sheet_data();
    assert!(unreachable(&app).is_empty(), "{:?}", unreachable(&app));
}

/// An offline map service (docs/adr/9187): a shown basemap whose service failed is out of reach, said with why; a
/// hidden one is not, nor one that answers.
#[test]
fn an_offline_map_service_is_out_of_reach_while_shown() {
    let mut app = app_with_drawing();
    let _ = app.update(Message::Run("basemap.osmStandard"));
    let doc = &app.document.as_ref().expect("open").model;
    let offline =
        super::mode::unreachable_layers(Some(doc), &|_| None, &|_| Some("çevrimdışı".into()));
    assert_eq!(
        offline
            .iter()
            .map(|u| (u.name.as_str(), u.why.as_str()))
            .collect::<Vec<_>>(),
        [("OSM Standart", "harita servisi yanıt vermiyor (çevrimdışı)")]
    );
    assert!(
        super::mode::unreachable_layers(Some(doc), &|_| None, &|_| None).is_empty(),
        "answering"
    );
    let id = doc
        .layers()
        .leaves()
        .iter()
        .find(|n| n.service.is_some())
        .map(|n| n.id.clone())
        .expect("the basemap");
    let doc = &mut app.document.as_mut().expect("open").model;
    let mut node = doc.layers().get(&id).expect("it").clone();
    node.visible = false;
    let mut snapshot = doc.to_snapshot_v2();
    for n in &mut snapshot.layers {
        if n.id == id {
            *n = node.clone();
        }
    }
    let hidden = kentos_domain::Document::from_snapshot_v2(snapshot).expect("reads");
    assert!(
        super::mode::unreachable_layers(Some(&hidden), &|_| None, &|_| Some("çevrimdışı".into()))
            .is_empty(),
        "hidden"
    );
}

/// The new sheet window's pictures (`.run/gui-evidence/kullanim-9147/`): the fixed scale saying the cut and what the
/// whole drawing needs, and Katman choosing the parcels' box.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    crate::files_testing::usage_shots("9147", "01-sabit-olcek-kirpma", || {
        let mut app = zoomed_in();
        let _ = app.update(Message::Sheet(Sheet::NewSheet));
        app
    });
    crate::files_testing::usage_shots("9147", "02-katman-kapsami", || {
        let mut app = zoomed_in();
        let _ = app.update(Message::Sheet(Sheet::NewSheet));
        ask(&mut app, QuestionsMessage::Extent(Extent::Layer));
        app
    });
}

/// Pictures for the owner (`.run/gui-evidence/kullanim-9187/`): the new sheet's picture at the fixed scale (the cut) and
/// with the whole drawing fitted; the print check of a reprint whose data layer's source is gone.
#[test]
#[ignore = "pictures for the owner"]
fn extent_picture_screens() {
    crate::files_testing::usage_shots("9187", "1-sabit-olcek-resim", || {
        let mut app = zoomed_in();
        let _ = app.update(Message::Sheet(Sheet::NewSheet));
        app
    });
    crate::files_testing::usage_shots("9187", "2-cizimin-tamami-resim", || {
        let mut app = zoomed_in();
        let _ = app.update(Message::Sheet(Sheet::NewSheet));
        ask(&mut app, QuestionsMessage::Extent(Extent::Drawing));
        app
    });
    crate::data_layers::register();
    let path = crate::data_layers_tests::parcels("yeniden-basim-resim", 4);
    let away = path.with_extension("tasindi");
    crate::files_testing::usage_shots("9187", "3-yeniden-basim-kaynak-yok", || {
        if away.exists() {
            std::fs::rename(&away, &path).expect("back");
        }
        let mut app = app_with_drawing();
        app.add_data_layers(vec![
            crate::data_layers::describe(&path.display().to_string(), "parseller")
                .expect("described"),
        ])
        .expect("added");
        let _ = app.update(Message::Sheet(Sheet::NewSheet));
        ask(&mut app, QuestionsMessage::Extent(Extent::Drawing));
        ask(&mut app, QuestionsMessage::Make);
        std::fs::rename(&path, &away).expect("moved");
        app.follow_sheet_data();
        let _ = app.update(Message::Sheet(Sheet::InspectorTab(
            kentos_sheet_ui::message::InspectorTab::Preflight,
        )));
        app
    });
}
