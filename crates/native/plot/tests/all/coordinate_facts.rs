//! The coordinate facts on a sheet (docs/adr/1680; plan/map-cad.md MAP-18): a TUREF / TM36 project (EPSG:5256) with
//! orthometric heights on the TG-20 geoid and the epoch 2005.0, its sheet's texts reading `@datum`, `@elipsoit`,
//! `@projeksiyon`, `@kot_sistemi`, `@epok`, `@epsg_kodu`. The expected words are EPSG's record of 5256 (TUREF, GRS 1980,
//! Transverse Mercator, natural origin 36° E, scale factor 1), not KentOS's. A text that names ED50 or another EPSG
//! code is found by the print check; one that names the project's datum beside another (a conversion note) is not; a
//! project's own variable `datum` comes before the fact.

use kentos_contracts::{DocumentSnapshotV1, DrawingFont, GeoidModel, HeightKind, HeightReference, Workspace};
use kentos_domain::Document;
use kentos_plot::book::base_inputs;
use kentos_project::new_project::{NewProject, new_project};
use kentos_sheet::display::{Prim, display_list};
use kentos_sheet::model::SheetBook;
use serde_json::json;

fn text(id: &str, top: i64, content: &str) -> serde_json::Value {
    json!({"id": id, "name": id, "frame": {"left": 20000, "top": top, "width": 380000, "height": 12000},
           "kind": {"type": "text", "content": content}})
}

fn drawing(own_datum: bool) -> Document {
    let mut snapshot: DocumentSnapshotV1 = new_project(&NewProject {
        name: "Ada 101".into(),
        srid: 5256,
        plot_scale: 1000.0,
        workspace: Workspace::Cad,
        drawing_font: DrawingFont::Barlow,
        province: None,
        drawing_unit: None,
        annotation: None,
        cad_work: None,
        heights: None,
    })
    .expect("a new project");
    snapshot.settings.heights = Some(HeightReference {
        kind: HeightKind::Orthometric,
        vertical_srid: None,
        geoid: Some(GeoidModel { name: "TG-20".into(), sha256: None }),
    });
    snapshot.settings.coordinate_epoch = Some(2005.0);
    if own_datum {
        snapshot.settings.variables = serde_json::from_value(json!([
            {"name": "datum", "label": "Kurumun yazımı", "kind": "text", "value": "ITRF96 (TUREF)"}
        ]))
        .expect("a variable");
    }
    let mut doc = Document::from_snapshot(snapshot).expect("opens");
    let ops: Vec<kentos_sheet::ops::Op> = serde_json::from_value(json!([{"op": "addSheet", "sheet": {
        "id": "pafta", "name": "Pafta",
        "page": {"paper": "a3", "orientation": "landscape", "size": {"width": 420000, "height": 297000},
                 "margins": {"top": 10000, "right": 10000, "bottom": 10000, "left": 20000}},
        "items": [
            text("datum", 20000, "Datum: [% @datum %]"),
            text("elipsoit", 40000, "Elipsoit: [% @elipsoit %]"),
            text("projeksiyon", 60000, "[% @projeksiyon %]"),
            text("kot", 80000, "Kot: [% @kot_sistemi %]"),
            text("epok", 100000, "Epok: [% @epok %]"),
            text("kod", 120000, "[% @koordinat_sistemi %] ([% @epsg_kodu %])"),
            text("eski", 140000, "Koordinatlar ED50 / UTM 36 (EPSG:23036)"),
            text("not", 160000, "ED50'den TUREF'e dönüştürülmüştür."),
            text("kod-eski", 180000, "Sistem: EPSG 2320")
        ]}}]))
    .expect("ops");
    let book = kentos_sheet::ops::apply_all(&SheetBook::default(), &ops).expect("the sheet").book;
    doc.set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).expect("kept")).expect("the book");
    doc
}

fn texts(doc: &Document) -> Vec<(String, String)> {
    let (book, _) = kentos_sheet::project::read_project_sheets(doc.sheets().expect("sheets")).expect("the book");
    let list = display_list(&book, "pafta", &base_inputs(doc, "Ayşe", "2026-11-07")).expect("drawn");
    let mut out: Vec<(String, String)> = Vec::new();
    for p in &list.prims {
        if let Prim::Text(t) = p {
            match out.iter_mut().find(|(id, _)| *id == t.item) {
                Some((_, words)) => {
                    words.push(' ');
                    words.push_str(&t.text);
                }
                None => out.push((t.item.clone(), t.text.clone())),
            }
        }
    }
    out
}

fn said<'a>(all: &'a [(String, String)], id: &str) -> &'a str {
    all.iter().find(|(i, _)| i == id).map_or("", |(_, w)| w.as_str())
}

#[test]
fn a_sheet_reads_the_project_s_datum_projection_heights_and_epoch() {
    let all = texts(&drawing(false));
    assert_eq!(said(&all, "datum"), "Datum: TUREF (ITRF96)");
    assert_eq!(said(&all, "elipsoit"), "Elipsoit: GRS80");
    // Words the sheet's faces draw (a subscript zero would be “?”).
    assert_eq!(said(&all, "projeksiyon"), "Transverse Mercator, DOM 36°, ölçek katsayısı 1");
    assert_eq!(said(&all, "kot"), "Kot: Ortometrik yükseklik (geoit TG-20)");
    assert!(said(&all, "epok").starts_with("Epok: 2005"), "{}", said(&all, "epok"));
    assert_eq!(said(&all, "kod"), "TUREF / TM36 (EPSG:5256)");
}

#[test]
fn a_project_s_own_variable_comes_first() {
    let all = texts(&drawing(true));
    assert_eq!(said(&all, "datum"), "Datum: ITRF96 (TUREF)");
}

#[test]
fn the_print_check_finds_a_text_that_names_another_system() {
    let doc = drawing(false);
    let (book, _) = kentos_sheet::project::read_project_sheets(doc.sheets().expect("sheets")).expect("the book");
    let inputs = base_inputs(&doc, "Ayşe", "2026-11-07");
    let found = kentos_plot::preflight::inspect(&doc, &book, "pafta", &inputs, None).expect("checked");
    let mut wrong: Vec<&str> =
        found.iter().filter(|f| f.code == "crs_text_mismatch").filter_map(|f| f.item.as_deref()).collect();
    wrong.sort();
    assert_eq!(wrong, ["eski", "kod-eski"], "the conversion note names TUREF too; the @ texts read the project");
}

/// `.run/gui-evidence/pafta-koordinat-1680/pafta.pdf`, the sheet as printed (its findings beside it):
///
/// ```text
/// cargo test -p kentos-plot --test all coordinate_facts::picture -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn picture() {
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../.run/gui-evidence/pafta-koordinat-1680");
    std::fs::create_dir_all(&out).expect("a folder");
    let made = kentos_plot::book::book_pdf(&drawing(false), &[], "Ayşe", "2026-11-07").expect("a PDF");
    std::fs::write(out.join("pafta.pdf"), &made.bytes).expect("writes");
    let words: Vec<String> =
        made.sheet_findings.iter().map(|(s, f)| format!("{s} {} {:?}: {}", f.code, f.item, f.message)).collect();
    std::fs::write(out.join("bulgular.txt"), words.join("\n")).expect("writes");
    println!("{}", out.join("pafta.pdf").display());
}
