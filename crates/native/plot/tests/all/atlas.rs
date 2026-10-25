//! The atlas as one PDF (docs/adr/0534): a hundred parcels, a page each, from a pinned state, with only what is
//! shown and permitted going out. The pages' facts are the independent script's
//! (`scripts/fixtures/atlas_cases.py`); here the planner's pages, the pin's rules and the file's structure.

use kentos_plot::atlas::{AtlasInput, plan};
use serde_json::Value;

use crate::atlas_drawing::{self as drawing, CANCELLED, COLUMNS, ROWS};
use crate::files::pdf_pages;

fn input(sheet: &str) -> AtlasInput<'_> {
    AtlasInput {
        sheet,
        user: "kabul",
        date: "2026-10-19",
        ..AtlasInput::default()
    }
}

#[test]
fn a_hundred_parcels_make_a_hundred_pages_by_the_sheet_s_rules() {
    let doc = drawing::document();
    let planned = plan(&doc, &input("atlas-paftasi")).expect("a plan");
    let total = COLUMNS * ROWS;
    assert_eq!(planned.pages.len(), total - CANCELLED.len());
    assert_eq!(planned.pages.len(), 100);
    assert_eq!(planned.filtered_out as usize, CANCELLED.len());
    // Sorted by the parcel number, named by ada and number, each centred on its parcel at a scale that holds it.
    let kept: Vec<usize> = (1..=total).filter(|n| !CANCELLED.contains(n)).collect();
    for (page, n) in planned.pages.iter().zip(&kept) {
        let (num, ada, (x, y), (w, h)) = drawing::parcel(n - 1);
        assert_eq!(page.name, format!("{ada}/{num}"));
        assert_eq!(page.center.x, x + w / 2.0);
        assert_eq!(page.center.y, y + h / 2.0);
        assert_eq!(page.bbox, [x, y, x + w, y + h]);
    }
    // The parcels are alike but the big one: 1/250 holds them, the big one wants 1/1000.
    let (last, rest) = planned.pages.split_last().expect("pages");
    assert!(
        rest.iter().all(|p| p.scale == 250),
        "{:?}",
        rest.iter().map(|p| p.scale).collect::<Vec<_>>()
    );
    assert_eq!(last.scale, 1000);
    // The permitted attributes: the declared fields less the owner a form marks `redact`.
    assert_eq!(planned.fields, ["Ada", "Parsel", "Durum"]);
    assert!(
        planned.withheld.contains(&"Malik".to_owned()),
        "{:?}",
        planned.withheld
    );
    assert!(
        planned.withheld.contains(&"Sır".to_owned()),
        "{:?}",
        planned.withheld
    );
    assert_eq!(planned.bindings.srid, 5253);
    assert!(planned.bindings.geo);
}

#[test]
fn the_pdf_has_the_pages_and_says_what_it_was_made_from() {
    let doc = drawing::document();
    let planned = plan(&doc, &input("atlas-paftasi")).expect("a plan");
    let made = planned.pdf(&doc).expect("a PDF");
    assert!(made.bytes.starts_with(b"%PDF-"));
    let pages = pdf_pages(&made.bytes);
    assert_eq!(pages.len(), 100, "a page for each object");
    assert!(
        pages
            .iter()
            .all(|p| (p.width - 210.0).abs() < 0.01 && (p.height - 297.0).abs() < 0.01)
    );
    // The keywords carry the state's digest.
    let text = String::from_utf8_lossy(&made.bytes);
    assert!(text.contains("kentos.atlas/1"), "the keywords");
    assert!(text.contains(&planned.pin.digest), "the pin's digest");
    // No attribute that is held back is in any object of the file, and no object of the filter's outside.
    assert!(made.findings.is_empty(), "{:?}", made.findings);
}

/// An object its layer's filter leaves out is on no map: the page of a parcel beside a cancelled one has fewer lines
/// than the same page of a drawing with no filter.
#[test]
fn objects_outside_the_layers_filter_are_on_no_map() {
    let pages_of = |doc: &kentos_domain::Document, name: &str| {
        let planned = plan(doc, &input("atlas-paftasi")).expect("a plan");
        let k = planned
            .pages
            .iter()
            .position(|p| p.name == name)
            .expect("the page");
        let made = planned.pdf(doc).expect("a PDF");
        pdf_pages(&made.bytes).swap_remove(k).segments.len()
    };
    let filtered = drawing::document();
    let unfiltered = drawing::document_of(drawing::drawing_with(false));
    // Parcels 4 and 6 are the cancelled 5's neighbours.
    for name in ["A1/4", "A1/6"] {
        let (f, u) = (pages_of(&filtered, name), pages_of(&unfiltered, name));
        assert!(f < u, "{name}: {f} lines with the filter, {u} without");
    }
    // The cancelled parcel is no page, and without the filter it is one.
    assert_eq!(
        plan(&unfiltered, &input("atlas-paftasi"))
            .expect("a plan")
            .pages
            .len(),
        105
    );
}

#[test]
fn a_pin_that_is_not_the_state_is_refused_and_the_same_state_is_not() {
    let mut doc = drawing::document();
    let planned = plan(&doc, &input("atlas-paftasi")).expect("a plan");
    let pin = planned.pin.clone();
    let again = AtlasInput {
        pin: Some(&pin),
        ..input("atlas-paftasi")
    };
    assert!(plan(&doc, &again).is_ok(), "the same state");
    // A change to a parcel is a different state.
    let entity: kentos_contracts::Entity = serde_json::from_value(serde_json::json!({
        "kind": "line", "id": 0, "layerId": "parsel", "attrs": {},
        "a": {"x": 1.0, "y": 1.0}, "b": {"x": 2.0, "y": 2.0}
    }))
    .expect("a line");
    doc.add(entity).expect("added");
    let e = plan(&doc, &again).expect_err("the drawing moved on");
    assert_eq!(e.code, "pin_mismatch");
    assert!(e.message.contains("Parsel"), "{}", e.message);
}

#[test]
fn the_bindings_are_explicit_and_refused_when_they_cannot_be_kept() {
    let doc = drawing::document();
    let wrong_crs = AtlasInput {
        crs: Some(4326),
        ..input("atlas-paftasi")
    };
    assert_eq!(
        plan(&doc, &wrong_crs).expect_err("another system").code,
        "crs_unsupported"
    );
    let project_crs = AtlasInput {
        crs: Some(5253),
        ..input("atlas-paftasi")
    };
    assert!(plan(&doc, &project_crs).is_ok());
    let theme = AtlasInput {
        theme: Some("yok"),
        ..input("atlas-paftasi")
    };
    assert_eq!(
        plan(&doc, &theme).expect_err("no theme").code,
        "unknown_theme"
    );
    let time = AtlasInput {
        time: Some("dün"),
        ..input("atlas-paftasi")
    };
    assert_eq!(
        plan(&doc, &time).expect_err("not a moment").code,
        "invalid_time"
    );
    let field = AtlasInput {
        fields: Some(&["Yok".to_owned()]),
        ..input("atlas-paftasi")
    };
    let e = plan(&doc, &field).expect_err("no such field");
    assert_eq!(
        (e.code.as_str(), e.path.as_str()),
        ("field_not_found", "fields[0]")
    );
    assert_eq!(
        plan(&doc, &input("yok")).expect_err("no sheet").code,
        "unknown_sheet"
    );
}

#[test]
fn a_redacted_field_is_never_carried_even_when_it_is_asked_for() {
    let doc = drawing::document();
    let fields = ["Ada".to_owned(), "Malik".to_owned()];
    let asked = AtlasInput {
        fields: Some(&fields),
        ..input("atlas-paftasi")
    };
    let planned = plan(&doc, &asked).expect("a plan");
    assert_eq!(planned.fields, ["Ada"]);
    assert!(planned.withheld.contains(&"Malik".to_owned()));
}

/// A bound file that changes while the pages are made refuses the run: nothing mixes two states.
#[test]
fn a_bound_file_that_changes_while_the_pages_are_made_refuses_the_run() {
    use kentos_contracts::{SourceKind, SourceLink};
    let dir = std::env::temp_dir().join(format!("kentos-atlas-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("kaynak.gpkg");
    std::fs::write(&file, b"bir").expect("a file");
    let sha = |b: &[u8]| kentos_contracts::encoding::sha256_hex(b);
    let mut snapshot =
        kentos_contracts::DocumentSnapshotV1::from_json(&drawing::drawing().to_string())
            .expect("reads");
    let link = SourceLink {
        id: "kaynak-1".into(),
        kind: SourceKind::Geopackage,
        path: file.display().to_string(),
        relative: None,
        table: "parsel".into(),
        layer: "parsel".into(),
        revision: sha(b"bir"),
        digest: String::new(),
        srid: Some(5253),
        extent: None,
        key: None,
        parked: Vec::new(),
        writable: true,
    };
    snapshot.settings.sources.push(link);
    let mut doc = kentos_domain::Document::from_snapshot(snapshot).expect("opens");
    let kept =
        kentos_sheet::project::project_sheets(&drawing::sheet_book(), |_| None).expect("kept");
    doc.set_sheets(kept).expect("the book");
    let planned = plan(&doc, &input("atlas-paftasi")).expect("a plan");
    let source = planned
        .pin
        .data
        .iter()
        .find_map(|l| l.source.clone())
        .expect("the bound layer's source");
    assert_eq!(source.revision, sha(b"bir"));
    assert_eq!(source.current.as_deref(), Some(sha(b"bir").as_str()));
    // The same file: the pages are made.
    assert!(planned.pdf(&doc).is_ok());
    // The file changes after the pages are made and before they are put in place.
    let e = planned
        .pdf_with(&doc, &|| std::fs::write(&file, b"iki").expect("changed"))
        .expect_err("refused");
    assert_eq!(e.code, "source_changed");
    // Planned again, the file is no longer the one the layer was read from: told, and made from the layer.
    let later = plan(&doc, &input("atlas-paftasi")).expect("a plan");
    assert!(
        later.warnings.iter().any(|w| w.code == "source_stale"),
        "{:?}",
        later.warnings
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_sheet_without_an_atlas_and_an_atlas_without_pages_say_so() {
    let mut doc = drawing::document();
    // A filter that no object passes.
    let mut book = drawing::sheet_book();
    book.sheets[0].atlas.as_mut().expect("atlas").filter = "Parsel > 100000".into();
    let kept = kentos_sheet::project::project_sheets(&book, |_| None).expect("kept");
    doc.set_sheets(kept).expect("the book");
    assert_eq!(
        plan(&doc, &input("atlas-paftasi")).expect_err("empty").code,
        "atlas_empty"
    );
    book.sheets[0].atlas = None;
    let kept = kentos_sheet::project::project_sheets(&book, |_| None).expect("kept");
    doc.set_sheets(kept).expect("the book");
    assert_eq!(
        plan(&doc, &input("atlas-paftasi")).expect_err("none").code,
        "no_atlas"
    );
    book.sheets[0].atlas = Some(kentos_sheet::model::Atlas {
        layer: "yok".into(),
        filter: String::new(),
        sort: Vec::new(),
        page_name: "Ada".into(),
    });
    let kept = kentos_sheet::project::project_sheets(&book, |_| None).expect("kept");
    doc.set_sheets(kept).expect("the book");
    assert_eq!(
        plan(&doc, &input("atlas-paftasi"))
            .expect_err("no layer")
            .code,
        "atlas_no_layer"
    );
}

/// The file written for the independent readers (poppler, GDAL): `KENTOS_ATLAS_OUT=dir`.
#[test]
fn the_file_for_the_independent_readers() {
    let Some(dir) = std::env::var_os("KENTOS_ATLAS_OUT") else {
        return;
    };
    let doc = drawing::document();
    let planned = plan(&doc, &input("atlas-paftasi")).expect("a plan");
    let made = planned.pdf(&doc).expect("a PDF");
    let dir = std::path::Path::new(&dir);
    std::fs::write(dir.join("parsel-atlasi.pdf"), &made.bytes).expect("written");
    let pages: Value = serde_json::to_value(&planned.pages).expect("json");
    std::fs::write(
        dir.join("parsel-atlasi.pages.json"),
        serde_json::to_vec_pretty(&pages).expect("json"),
    )
    .expect("written");
}

/// Pages in another system (docs/adr/0638 §1): the bindings say both, the pages' middles are on the paper's ground,
/// their boxes on the project's; the file's viewports are in the paper's system and its keywords say the project's.
#[test]
fn pages_drawn_in_another_system_say_both_systems() {
    let doc = drawing::document();
    let tm30 = AtlasInput {
        crs: Some(5254),
        ..input("atlas-paftasi")
    };
    let planned = plan(&doc, &tm30).expect("a plan");
    assert_eq!(planned.bindings.srid, 5254);
    assert_eq!(planned.bindings.project_srid, Some(5253));
    assert_eq!(planned.bindings.crs_name, "TUREF / TM30");
    assert!(planned.bindings.geo);
    let own = plan(&doc, &input("atlas-paftasi")).expect("a plan");
    assert_eq!(planned.pages.len(), own.pages.len());
    for (p, o) in planned.pages.iter().zip(&own.pages) {
        assert_eq!(p.bbox, o.bbox, "the object's box stays the project's");
        // TM30 is 3° east of TM27: the ground moves some 257 km west.
        assert!(o.center.x - p.center.x > 250_000.0, "{p:?}");
    }
    let made = planned.pdf(&doc).expect("a PDF");
    let text = String::from_utf8_lossy(&made.bytes);
    assert!(
        text.contains("koordinat=EPSG:5254; proje=EPSG:5253"),
        "the keywords"
    );
    assert!(text.contains("/EPSG 5254"), "the viewports' system");
    // A system in degrees, and one the registry does not have, are refused with words.
    for (srid, words) in [(4326, "derece"), (9999, "kaydında yok")] {
        let e = plan(
            &doc,
            &AtlasInput {
                crs: Some(srid),
                ..input("atlas-paftasi")
            },
        )
        .expect_err("refused");
        assert_eq!(e.code, "crs_unsupported");
        assert!(e.message.contains(words), "{}", e.message);
    }
}

/// A map cut to its object (docs/adr/0638 §2): the map's clip is its parcel's outline, not the frame: the clips'
/// areas on the paper are far smaller.
#[test]
fn a_map_cut_to_its_object_writes_the_object_s_outline_as_a_clip() {
    use kentos_sheet::kinds::{ItemKind, MapView};
    let clips_of = |cut: bool| {
        let mut doc = drawing::document();
        let mut book = drawing::sheet_book();
        for it in &mut book.sheets[0].items {
            if let ItemKind::Map(m) = &mut it.kind
                && matches!(m.view, MapView::Atlas(_))
            {
                m.clip_to_atlas_feature = cut;
            }
        }
        let kept = kentos_sheet::project::project_sheets(&book, |_| None).expect("kept");
        doc.set_sheets(kept).expect("the book");
        let planned = plan(&doc, &input("atlas-paftasi")).expect("a plan");
        let made = planned.pdf(&doc).expect("a PDF");
        assert!(
            !made.notes.iter().any(|n| n.contains("kırp")),
            "{:?}",
            made.notes
        );
        // Each clip's area: the path written before `W n`, its points' shoelace.
        let mut areas = Vec::new();
        for stream in crate::files::pdf_streams(&made.bytes) {
            let text = String::from_utf8_lossy(&stream).into_owned();
            let mut ring: Vec<(f64, f64)> = Vec::new();
            for line in text.lines() {
                let words: Vec<&str> = line.split_whitespace().collect();
                match words.as_slice() {
                    [x, y, "m"] => {
                        ring.clear();
                        ring.push((x.parse().unwrap_or(0.0), y.parse().unwrap_or(0.0)));
                    }
                    [x, y, "l"] => ring.push((x.parse().unwrap_or(0.0), y.parse().unwrap_or(0.0))),
                    ["W"] => {
                        let n = ring.len();
                        let a: f64 = (0..n)
                            .map(|i| {
                                let (p, q) = (ring[i], ring[(i + 1) % n]);
                                p.0 * q.1 - q.0 * p.1
                            })
                            .sum();
                        areas.push(a.abs() / 2.0);
                    }
                    _ => {}
                }
            }
        }
        areas
    };
    let (cut, whole) = (clips_of(true), clips_of(false));
    assert_eq!(cut.len(), whole.len(), "one clip for one");
    let (a, b): (f64, f64) = (cut.iter().sum(), whole.iter().sum());
    assert!(a < b * 0.9, "{a} < {b}");
}
