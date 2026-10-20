//! A text's letters and formats through every output (docs/adr/0324 §6; plan/cad.md CAD-ANNO-18): Turkish letters,
//! symbols, a line break, bold and coloured runs, a raised letter, a width factor and a slant, a list's indents and
//! tabs and a stacked fraction, written into the project file, a DXF and a sheet's PDF and read back: the file and the
//! DXF give the same text, runs and layout; the PDF's words (poppler's `pdftotext`) are the letters as typed.

use kentos_commands::{Entity, TextEntity, TextScript};
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::document::Document;
use crate::files_testing::{drive, scratch};

const SCENARIO: &str = include_str!("../../../fixtures/acceptance/v1/room.json");
const TURKISH: &str = "İĞÜŞÖÇ ığüşöç\n45° ±0.05 Ø20 m²";
const LIST: &str = "Liste\n•\tKum\n•\tÇakıl 1/2";

fn command(app: &mut App, name: &str, input: Value) -> Value {
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        name,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    app.follow_sheets();
    answer
}

fn texts(list: &[Entity]) -> Vec<TextEntity> {
    let mut out: Vec<TextEntity> = list
        .iter()
        .filter_map(|e| match e {
            Entity::Text(t) => Some(t.clone()),
            _ => None,
        })
        .collect();
    out.sort_by(|a, b| a.text.cmp(&b.text));
    out
}

/// A text's letters, formats, width, slant and layout.
type Kept = (
    String,
    Vec<kentos_commands::TextRun>,
    Option<f64>,
    Option<f64>,
    Option<Box<kentos_commands::ParagraphLayout>>,
);

/// What a text should keep through a file.
fn kept(t: &TextEntity) -> Kept {
    (
        t.text.clone(),
        t.paragraph.runs.clone(),
        t.width_factor,
        t.face.oblique,
        t.paragraph.layout.clone(),
    )
}

/// The words on page `page` of the PDF at `path`.
fn pdf_words(path: &std::path::Path, page: usize) -> Vec<String> {
    let n = page.to_string();
    let run = std::process::Command::new("pdftotext")
        .args(["-f", &n, "-l", &n])
        .arg(path)
        .arg("-")
        .output()
        .expect("pdftotext runs");
    String::from_utf8_lossy(&run.stdout)
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

#[cfg(unix)]
#[test]
fn a_text_s_letters_and_formats_come_back_from_every_output() {
    let scenario: Value = serde_json::from_str(SCENARIO).expect("the scenario");
    let dir = scratch("yazi-ciktilari");
    let choices = kentos_headless::new_drawing(&scenario["project"]).expect("the choices");
    let made = kentos_headless::Session::new(&choices).expect("a new project");
    let path = dir.join("oda.kcad");
    std::fs::write(&path, made.to_bytes().expect("its bytes")).expect("written");
    let (mut app, _) = App::boot(None);
    let doc = Document::read(&path).expect("opens");
    let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    drive(&mut app, task);
    let layer = app
        .document
        .as_ref()
        .map(|d| d.model.layers().active().to_owned())
        .expect("a layer");
    // 5 mm on paper at the project's 1:50.
    command(
        &mut app,
        "cad.entities.create",
        json!({ "layerId": layer, "objects": [
            { "geometry": { "kind": "text", "p": { "x": 0.5, "y": 3.5 }, "text": TURKISH, "height": 0.25, "rotation": 0.0,
                "align": "topLeft", "widthFactor": 0.8, "font": "arimo", "oblique": 15.0,
                "runs": [{ "start": 0, "end": 6, "bold": true }, { "start": 7, "end": 13, "color": "#E5484D" },
                         { "start": 29, "end": 30, "script": "super" }] } },
            { "geometry": { "kind": "text", "p": { "x": 0.5, "y": 2.0 }, "text": LIST, "height": 0.25, "rotation": 0.0,
                "align": "topLeft", "boxWidth": 3.0,
                "runs": [{ "start": 20, "end": 23, "script": "stack" }],
                "layout": { "indent": -1.5, "leftIndent": 1.5 } } }
        ] }),
    );
    let model = &app.document.as_ref().expect("a drawing").model;
    let written = texts(&model.entities().cloned().collect::<Vec<_>>());
    assert_eq!(written.len(), 2);
    // Sorted by their letters: the list (L) before İ.
    assert_eq!(written[0].paragraph.runs[0].script, Some(TextScript::Stack));

    // The project file.
    let bytes = kentos_kcad::encode(&model.to_snapshot_v2()).expect("encodes");
    let back = kentos_kcad::decode(&bytes).expect("decodes");
    let read = texts(&back.entities);
    assert_eq!(
        read.iter().map(kept).collect::<Vec<_>>(),
        written.iter().map(kept).collect::<Vec<_>>()
    );

    // A DXF: MTEXT's own notation, the slant and face in KentOS's data.
    let all: Vec<&Entity> = model.entities().collect();
    let input = kentos_headless::export::dxf_input(model, &[(layer.clone(), all)]);
    let (dxf, _) = kentos_formats::dxf::write(&input);
    let imported = kentos_formats::dxf::read(&dxf, &kentos_commands::DxfReadOptions::default())
        .expect("reads");
    let read = texts(&imported.entities);
    assert_eq!(
        read.iter().map(kept).collect::<Vec<_>>(),
        written.iter().map(kept).collect::<Vec<_>>()
    );

    // A sheet's PDF: the letters as typed.
    if std::process::Command::new("pdftotext")
        .arg("-v")
        .output()
        .is_err()
    {
        eprintln!("pdftotext yok: yazının PDF'i denetlenmedi (poppler kurun).");
        return;
    }
    let sheets = scenario["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .find(|s| s["command"] == "cad.sheets.edit")
        .expect("the sheets' step");
    command(&mut app, "cad.sheets.edit", sheets["input"].clone());
    use kentos_sheet_ui::message::{PdfMessage, PdfScope};
    let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::Export(
        kentos_sheet_ui::ExportKind::Pdf,
    )));
    let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::Pdf(
        PdfMessage::Scope(PdfScope::All),
    )));
    let pdf = dir.join("yazilar.pdf");
    app.sheet_export_to(kentos_sheet_ui::ExportKind::Pdf, Some(pdf.clone()));
    let words = pdf_words(&pdf, 1);
    for want in [
        "İĞÜŞÖÇ",
        "ığüşöç",
        "45°",
        "±0.05",
        "Ø20",
        "Liste",
        "Kum",
        "Çakıl",
    ] {
        assert!(
            words.iter().any(|w| w == want),
            "“{want}” not in the PDF: {words:?}"
        );
    }
}
