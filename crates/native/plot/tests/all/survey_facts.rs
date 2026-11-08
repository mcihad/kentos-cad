//! The survey quality on a sheet (docs/adr/2400; plan/map-cad.md MAP-18): the map project of
//! `fixtures/kcad/v2/map-survey-network.json`, whose GNSS archive accepted kn-1 as a mean (no σ) and kn-2 by the
//! network adjustment da-1 (σ north 4.0 mm, east 3.5 mm, up 9.0 mm; class C3, the rule package bohhbuy-2018-gnss@1
//! held, m0 0.98, 3 degrees of freedom). Its sheet's texts read `@konum_dogrulugu`, `@yukseklik_dogrulugu`,
//! `@ag_dengelemesi` and `@olcme_kalitesi`. By hand: √(4.0² + 3.5²) = 5.315 mm, shown to the millimetre as ±0,5 cm;
//! the greatest σ up ±0,9 cm. A project's own variable of a name comes first.

use kentos_contracts::DocumentSnapshotV2;
use kentos_domain::Document;
use kentos_plot::book::base_inputs;
use kentos_sheet::display::{Prim, display_list};
use kentos_sheet::model::SheetBook;
use serde_json::{Value, json};

fn text(id: &str, top: i64, content: &str) -> Value {
    json!({"id": id, "name": id, "frame": {"left": 20000, "top": top, "width": 380000, "height": 12000},
           "kind": {"type": "text", "content": content}})
}

fn drawing(own: bool) -> Document {
    drawing_of(own, true)
}

/// The project with its sheet; `archive` false: without its GNSS archive.
fn drawing_of(own: bool, archive: bool) -> Document {
    let mut v: Value =
        serde_json::from_str(include_str!("../../../../../fixtures/kcad/v2/map-survey-network.json")).expect("the file");
    if !archive {
        v["settings"].as_object_mut().expect("settings").remove("mapSurvey");
    }
    if own {
        v["settings"]["variables"] = json!([
            {"name": "konum_dogrulugu", "label": "Kurumun yazımı", "kind": "text", "value": "±1 cm"}
        ]);
    }
    let snapshot: DocumentSnapshotV2 = serde_json::from_value(v).expect("a snapshot");
    let mut doc = Document::from_snapshot_v2(snapshot).expect("opens");
    let ops: Vec<kentos_sheet::ops::Op> = serde_json::from_value(json!([{"op": "addSheet", "sheet": {
        "id": "pafta", "name": "Pafta",
        "page": {"paper": "a3", "orientation": "landscape", "size": {"width": 420000, "height": 297000},
                 "margins": {"top": 10000, "right": 10000, "bottom": 10000, "left": 20000}},
        "items": [
            text("konum", 20000, "Konum: [% @konum_dogrulugu %]"),
            text("yukseklik", 40000, "Yükseklik: [% @yukseklik_dogrulugu %]"),
            text("ag", 60000, "[% @ag_dengelemesi %]"),
            text("not", 80000, "[% @olcme_kalitesi %]")
        ]}}]))
    .expect("ops");
    let book = kentos_sheet::ops::apply_all(&SheetBook::default(), &ops).expect("the sheet").book;
    doc.set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).expect("kept")).expect("the book");
    doc
}

/// Each text's words, its lines joined.
fn texts(doc: &Document) -> Vec<(String, String)> {
    let (book, _) = kentos_sheet::project::read_project_sheets(doc.sheets().expect("sheets")).expect("the book");
    let list = display_list(&book, "pafta", &base_inputs(doc, "Ayşe", "2026-11-08")).expect("drawn");
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
fn a_sheet_reads_the_archive_s_survey_quality() {
    let all = texts(&drawing(false));
    assert_eq!(said(&all, "konum"), "Konum: ±0,5 cm");
    assert_eq!(said(&all, "yukseklik"), "Yükseklik: ±0,9 cm");
    assert_eq!(said(&all, "ag"), "da-1, C3 sınıfı, kurallar sağlandı (bohhbuy-2018-gnss@1), m0 0,98, serbestlik 3");
    assert_eq!(
        said(&all, "not"),
        "GNSS ile 2 nokta: en büyük nokta konum hatası ±0,5 cm, yükseklik hatası ±0,9 cm (ortalama hata); ağ dengelemesi da-1 (C3, \
         kurallar sağlandı). 1 noktanın doğruluğu kayıtlı değil."
    );
    // Every glyph drawn: a sign the sheet's face lacks would print “?”.
    assert!(all.iter().all(|(_, w)| !w.contains('?')), "{all:?}");
}

#[test]
fn a_project_s_own_variable_comes_first() {
    let all = texts(&drawing(true));
    assert_eq!(said(&all, "konum"), "Konum: ±1 cm");
}

/// The print check says each text that reads a quality the project does not have; with the archive, or with a
/// project variable of the name, it says none.
#[test]
fn the_print_check_says_a_quality_the_project_lacks() {
    let found = |doc: &Document| -> Vec<String> {
        let (book, _) = kentos_sheet::project::read_project_sheets(doc.sheets().expect("sheets")).expect("the book");
        let inputs = base_inputs(doc, "Ayşe", "2026-11-08");
        let all = kentos_plot::preflight::inspect(doc, &book, "pafta", &inputs, None).expect("checked");
        let mut items: Vec<String> =
            all.iter().filter(|f| f.code == "survey_quality_missing").filter_map(|f| f.item.clone()).collect();
        items.sort();
        items
    };
    assert!(found(&drawing(false)).is_empty());
    assert_eq!(found(&drawing_of(false, false)), ["ag", "konum", "not", "yukseklik"]);
    assert_eq!(found(&drawing_of(true, false)), ["ag", "not", "yukseklik"], "the project's own konum_dogrulugu");
}

/// `.run/gui-evidence/olcme-kalitesi-2400/pafta.pdf`, the sheet as printed:
///
/// ```text
/// cargo test -p kentos-plot --test all survey_facts::picture -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn picture() {
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../.run/gui-evidence/olcme-kalitesi-2400");
    std::fs::create_dir_all(&out).expect("a folder");
    let made = kentos_plot::book::book_pdf(&drawing(false), &[], "Ayşe", "2026-11-08").expect("a PDF");
    std::fs::write(out.join("pafta.pdf"), &made.bytes).expect("writes");
    println!("{}", out.join("pafta.pdf").display());
}
