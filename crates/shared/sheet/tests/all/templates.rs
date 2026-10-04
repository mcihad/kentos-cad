//! The system templates (design §12): each reads and validates, works on
//! A4, A3, A1 and A0 in both orientations without a preflight error other
//! than the values only a user gives, and draws its recommended paper as
//! the golden SVG in `fixtures/sheet/v1/svg` (`KENTOS_WRITE_SHEET=1`
//! rewrites them; read the difference). `template_shots` (ignored) writes
//! the pages the PNG renders are taken from, into `.run/shots/sheet/templates`.

use crate::common;

use common::*;
use kentos_sheet::display::{RenderInputs, display_list};
use kentos_sheet::preflight::{PLACEHOLDERS, Severity, preflight};
use kentos_sheet::svg::{SvgOptions, to_svg};
use kentos_sheet::template::*;
use kentos_sheet::*;
use std::collections::BTreeSet;

pub const SYSTEM_IDS: [&str; 10] = [
    "sys:genel-a4-dikey",
    "sys:genel-a3-yatay",
    "sys:rapor-sayfasi",
    "sys:cad-teknik",
    "sys:cad-mimari",
    "sys:aplikasyon-krokisi",
    "sys:ifraz-paftasi",
    "sys:imar-plani",
    "sys:gis-tematik",
    "sys:gis-atlas",
];

/// CAD templates for projects that may have no coordinate system.
fn georeferenced(id: &str) -> bool {
    !matches!(id, "sys:cad-teknik" | "sys:cad-mimari")
}

/// A sheet from a template, as a host makes it: ids from the template's, the sample answers, the sample place.
pub fn instance(t: &Template, paper: Option<PaperChoice>, answers: bool) -> SheetBook {
    let ids = InstanceIds {
        sheet: "s1".into(),
        master: t.master.as_ref().map(|_| "m1".into()),
        items: t.sheet.items.iter().map(|i| i.id.clone()).collect(),
        master_items: t
            .master
            .iter()
            .flat_map(|m| m.items.iter().map(|i| format!("m-{}", i.id)))
            .collect(),
    };
    let options = InstanceOptions {
        paper,
        name: None,
        center: answers.then_some(CENTER),
        scale: None,
        values: if answers { sample_values() } else { Vec::new() },
    };
    let inst = instantiate(t, &ids, &options).expect("instantiate");
    SheetBook {
        sheets: vec![inst.sheet],
        masters: inst.master.into_iter().collect(),
        assets: inst.assets,
        ..SheetBook::default()
    }
}

fn atlas_input(book: &SheetBook, inputs: &mut RenderInputs) {
    if book.sheets[0].atlas.is_none() {
        return;
    }
    let features: Vec<display::AtlasFeature> = (0..3)
        .map(|i| display::AtlasFeature {
            id: format!("1234/{}", i + 5),
            bbox: [
                487_120.0 + f64::from(i) * 40.0,
                4_417_790.0,
                487_170.0 + f64::from(i) * 40.0,
                4_417_850.0,
            ],
            attributes: vec![display::Attribute {
                name: "ad".into(),
                value: VarValue::Text(format!("1234 ada {} parsel", i + 5)),
            }],
        })
        .collect();
    let plan = kentos_sheet::atlas::atlas_plan(book, "s1", &features).expect("atlas plan");
    inputs.atlas = plan.pages.into_iter().next();
}

#[test]
fn system_templates_read() {
    assert!(
        system_template_errors().is_empty(),
        "{:?}",
        system_template_errors()
    );
    let ids: Vec<&str> = system_templates()
        .iter()
        .map(|t| t.meta.id.as_str())
        .collect();
    assert_eq!(ids, SYSTEM_IDS);
    for t in system_templates() {
        assert!(!t.meta.papers.is_empty(), "{}", t.meta.id);
        let json = serde_json::to_string(t).unwrap();
        // A system template is not a user's: the user reader refuses its `sys:` id.
        assert_eq!(read_template(&json).unwrap_err().code, "reserved_id");
    }
}

#[test]
fn system_templates_pass_preflight_on_every_paper() {
    let papers = [Paper::A4, Paper::A3, Paper::A1, Paper::A0];
    let orientations = [Orientation::Portrait, Orientation::Landscape];
    let mut failures = Vec::new();
    for t in system_templates() {
        for paper in papers {
            for orientation in orientations {
                let book = instance(t, Some(PaperChoice { paper, orientation }), false);
                let mut inputs = sample_inputs(&book, "s1", georeferenced(&t.meta.id));
                // A template as it is: no legend, table or coordinate data, no answers, no place.
                inputs.legends.clear();
                inputs.tables.clear();
                inputs.coordinates.clear();
                for f in preflight(&book, "s1", &inputs).unwrap() {
                    if f.severity == Severity::Error && !PLACEHOLDERS.contains(&f.code.as_str()) {
                        failures.push(format!(
                            "{} {:?} {:?}: {} {}",
                            t.meta.id, paper, orientation, f.code, f.message
                        ));
                    }
                }
                // With the sample answers and data: no error at all.
                let book = instance(t, Some(PaperChoice { paper, orientation }), true);
                let mut inputs = sample_inputs(&book, "s1", georeferenced(&t.meta.id));
                atlas_input(&book, &mut inputs);
                for f in preflight(&book, "s1", &inputs).unwrap() {
                    if f.severity == Severity::Error && f.code != "atlas_no_layer" {
                        failures.push(format!(
                            "{} {:?} {:?} (dolu): {} {}",
                            t.meta.id, paper, orientation, f.code, f.message
                        ));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// On the papers a template names, filled in, nothing is even a warning: no overlap, no overflow, nothing outside the margins.
#[test]
fn system_templates_are_clean_on_their_papers() {
    let mut failures = Vec::new();
    for t in system_templates() {
        for choice in &t.meta.papers {
            let book = instance(t, Some(*choice), true);
            let mut inputs = sample_inputs(&book, "s1", georeferenced(&t.meta.id));
            atlas_input(&book, &mut inputs);
            for f in preflight(&book, "s1", &inputs).unwrap() {
                if f.severity != Severity::Info && f.code != "atlas_no_layer" {
                    failures.push(format!(
                        "{} {:?} {:?}: {} {}",
                        t.meta.id, choice.paper, choice.orientation, f.code, f.message
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The SVG of a system template on its recommended paper, with the sample answers and data;
/// `maps`: the synthetic map pictures in the map frames (the renders), or the writer's grey placeholder (the goldens: what the core makes, not what a host paints).
fn render(t: &Template, answers: bool, maps: bool) -> (String, f64, f64) {
    let book = instance(t, None, answers);
    let mut inputs = sample_inputs(&book, "s1", georeferenced(&t.meta.id));
    if !answers {
        inputs.legends.clear();
        inputs.tables.clear();
        inputs.coordinates.clear();
    }
    atlas_input(&book, &mut inputs);
    let list = display_list(&book, "s1", &inputs).unwrap();
    let options = SvgOptions {
        maps: if answers && maps {
            map_images(&list)
        } else {
            Vec::new()
        },
        title: Some(t.meta.name.clone()),
        ..SvgOptions::default()
    };
    let size = book.sheets[0].page.size;
    (
        to_svg(&list, &options),
        f64::from(size.width) / 1000.0,
        f64::from(size.height) / 1000.0,
    )
}

#[test]
fn system_template_svgs_are_golden() {
    let dir = fixtures().join("svg");
    let mut stale = Vec::new();
    for t in system_templates() {
        let (svg, ..) = render(t, true, false);
        let file = dir.join(format!("{}.svg", t.meta.id.trim_start_matches("sys:")));
        if writing() {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(&file, &svg).unwrap();
        } else if std::fs::read_to_string(&file).ok().as_deref() != Some(svg.as_str()) {
            stale.push(file.display().to_string());
        }
    }
    assert!(
        stale.is_empty(),
        "değişen SVG'ler (KENTOS_WRITE_SHEET=1 ile yazın, farkı okuyun): {stale:?}"
    );
}

/// The same, on another paper: the items follow their constraints there.
fn render_on(t: &Template, paper: PaperChoice) -> (String, f64, f64) {
    let book = instance(t, Some(paper), true);
    let mut inputs = sample_inputs(&book, "s1", georeferenced(&t.meta.id));
    atlas_input(&book, &mut inputs);
    let list = display_list(&book, "s1", &inputs).unwrap();
    let options = SvgOptions {
        maps: map_images(&list),
        title: Some(t.meta.name.clone()),
        ..SvgOptions::default()
    };
    let size = book.sheets[0].page.size;
    (
        to_svg(&list, &options),
        f64::from(size.width) / 1000.0,
        f64::from(size.height) / 1000.0,
    )
}

#[test]
#[ignore]
fn template_shots() {
    let dir = root().join(".run/shots/sheet/templates");
    std::fs::create_dir_all(&dir).unwrap();
    let mut list = String::new();
    for t in system_templates() {
        let name = t.meta.id.trim_start_matches("sys:");
        let mut shots = vec![
            (name.to_owned(), render(t, true, true)),
            (format!("{name}-bos"), render(t, false, false)),
        ];
        // Two other papers: the constraints at work.
        let recommended = t.meta.papers[0];
        let others = if recommended.orientation == Orientation::Portrait {
            [
                (Paper::A3, Orientation::Landscape),
                (Paper::A1, Orientation::Portrait),
            ]
        } else {
            [
                (Paper::A4, Orientation::Portrait),
                (Paper::A2, Orientation::Landscape),
            ]
        };
        for (paper, orientation) in others {
            let tag = format!(
                "{}-{}",
                paper.id(),
                if orientation == Orientation::Portrait {
                    "dikey"
                } else {
                    "yatay"
                }
            );
            shots.push((
                format!("{name}-{tag}"),
                render_on(t, PaperChoice { paper, orientation }),
            ));
        }
        for (file, (svg, w, h)) in shots {
            std::fs::write(dir.join(format!("{file}.svg")), &svg).unwrap();
            std::fs::write(dir.join(format!("{file}.html")), html(&svg, w, h)).unwrap();
            list.push_str(&format!(
                "{file} {} {}\n",
                (w * 10.0).round(),
                (h * 10.0).round()
            ));
        }
    }
    std::fs::write(dir.join("list.txt"), list).unwrap();
}

/// Every character the templates print (and the sample answers) is in the metrics table, so none is measured as an average letter.
#[test]
fn every_character_of_the_templates_is_measured() {
    fn walk(v: &serde_json::Value, out: &mut BTreeSet<char>) {
        match v {
            serde_json::Value::String(s) => out.extend(s.chars()),
            serde_json::Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            serde_json::Value::Object(o) => o.values().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut chars = BTreeSet::new();
    for t in system_templates() {
        walk(&serde_json::to_value(t).unwrap(), &mut chars);
    }
    walk(&serde_json::to_value(sample_values()).unwrap(), &mut chars);
    let missing: Vec<char> = chars
        .into_iter()
        .filter(|c| !c.is_control() && !kentos_sheet::text::measures(*c))
        .collect();
    assert!(
        missing.is_empty(),
        "ölçüsü olmayan karakterler: {missing:?}"
    );
}

/// “Şablon olarak kaydet” and back: the maps lose their place (not their scale), the sheet's
/// answers become questions, the pictures travel with their bytes, the master page goes along;
/// the template reads as a user's, and used with the same ids, place and answers it gives the
/// sheet again.
#[test]
fn a_sheet_saved_as_a_template_comes_back() {
    let meta = |id: &str| TemplateMeta {
        id: id.into(),
        revision: 1,
        name: "Büromun paftası".into(),
        description: "Deneme".into(),
        category: "genel".into(),
        tags: vec!["büro".into()],
        papers: vec![PaperChoice {
            paper: Paper::A4,
            orientation: Orientation::Landscape,
        }],
        workspaces: vec![
            kentos_contracts::Workspace::Cad,
            kentos_contracts::Workspace::Gis,
        ],
        project_types: Vec::new(),
        created: "2026-10-02T09:00:00Z".into(),
        updated: "2026-10-02T09:00:00Z".into(),
        author: "M. Demir".into(),
    };
    let read = |path: &str| {
        kentos_sheet::validate::read_book(&std::fs::read_to_string(fixtures().join(path)).unwrap())
            .unwrap()
    };
    let center = |s: &Sheet| {
        s.items.iter().find_map(|i| match &i.kind {
            ItemKind::Map(m) => match &m.view {
                MapView::Fixed(f) => f.center,
                _ => None,
            },
            _ => None,
        })
    };

    // A sheet with a picture: its bytes are asked for, and carried.
    let book = read("display/books/every-kind.json");
    let valid: Template = serde_json::from_str(
        &std::fs::read_to_string(fixtures().join("templates/valid/kullanici-sablonu.json"))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        extract(&book, "s1", &meta("u:buro"), &[]).unwrap_err().code,
        "missing_asset"
    );
    let t = extract(&book, "s1", &meta("u:buro"), &valid.assets).unwrap();
    assert_eq!(t.assets.len(), 1);
    assert_eq!(center(&t.sheet), None, "şablon yer taşımaz");
    let again = read_template(&serde_json::to_string(&t).unwrap()).unwrap();
    assert_eq!(again, t);
    let ids = InstanceIds {
        sheet: "s1".into(),
        master: None,
        items: book.sheets[0].items.iter().map(|i| i.id.clone()).collect(),
        master_items: Vec::new(),
    };
    let options = InstanceOptions {
        center: center(&book.sheets[0]),
        ..InstanceOptions::default()
    };
    let inst = instantiate(&t, &ids, &options).unwrap();
    let mut sheet = inst.sheet;
    assert_eq!(
        sheet.origin.take().map(|o| (o.template_id, o.revision)),
        Some(("u:buro".to_owned(), 1))
    );
    assert_eq!(sheet, book.sheets[0]);
    assert_eq!(inst.asset_bytes, valid.assets);

    // A sheet on a master page with an answered question.
    let mut book = read("ops/book.json");
    book.sheets[0].items.retain(|i| i.id != "p1");
    book.assets.clear();
    let t = extract(&book, "s1", &meta("u:ana-sayfali"), &[]).unwrap();
    assert!(t.sheet.variables.is_empty());
    assert_eq!(
        t.variables
            .iter()
            .map(|v| (v.name.as_str(), v.value.is_null()))
            .collect::<Vec<_>>(),
        [("ada", true)]
    );
    let master = t.master.as_ref().unwrap();
    assert_eq!(master.items.len(), 2);
    read_template(&serde_json::to_string(&t).unwrap()).unwrap();
    let ids = InstanceIds {
        sheet: "s1".into(),
        master: Some("M1".into()),
        items: book.sheets[0].items.iter().map(|i| i.id.clone()).collect(),
        master_items: book.masters[0].items.iter().map(|i| i.id.clone()).collect(),
    };
    let options = InstanceOptions {
        center: center(&book.sheets[0]),
        values: serde_json::from_value(serde_json::json!([{ "name": "ada", "value": "101" }]))
            .unwrap(),
        ..InstanceOptions::default()
    };
    let inst = instantiate(&t, &ids, &options).unwrap();
    let mut sheet = inst.sheet;
    sheet.origin = None;
    assert_eq!(sheet, book.sheets[0]);
    assert_eq!(inst.master.as_ref(), Some(&book.masters[0]));
    // Fewer ids than items: refused, nothing guessed.
    let short = InstanceIds {
        items: ids.items[1..].to_vec(),
        ..ids
    };
    assert_eq!(
        instantiate(&t, &short, &options).unwrap_err().code,
        "ids_missing"
    );
}

/// A0 … A4, both ways: the matrix of design §3.2a.
const MATRIX: [Paper; 5] = [Paper::A4, Paper::A3, Paper::A2, Paper::A1, Paper::A0];

fn orientation_tag(o: Orientation) -> &'static str {
    if o == Orientation::Portrait {
        "dikey"
    } else {
        "yatay"
    }
}

/// A template on a paper as a user has it: answered, placed, with its data (an atlas's coverage layer chosen).
fn filled(t: &Template, choice: PaperChoice) -> (SheetBook, RenderInputs) {
    let mut book = instance(t, Some(choice), true);
    if let Some(a) = book.sheets[0].atlas.as_mut() {
        a.layer = "parsel".into();
    }
    let mut inputs = sample_inputs(&book, "s1", georeferenced(&t.meta.id));
    atlas_input(&book, &mut inputs);
    (book, inputs)
}

/// Every system template on A4 … A0 in both orientations, answered and filled: no error, and nothing
/// that a layout should have prevented — no overlap, nothing off the paper or in the margins, no text
/// that does not fit (design §3.2a). Any other warning is listed too: the layouts are meant to be clean.
#[test]
fn every_template_on_every_paper_both_ways() {
    let mut failures = Vec::new();
    for t in system_templates() {
        for paper in MATRIX {
            for orientation in [Orientation::Portrait, Orientation::Landscape] {
                let (book, inputs) = filled(t, PaperChoice { paper, orientation });
                for f in preflight(&book, "s1", &inputs).unwrap() {
                    if f.severity != Severity::Info {
                        failures.push(format!(
                            "{} {} {}: {} {} {}",
                            t.meta.id,
                            paper.id(),
                            orientation_tag(orientation),
                            f.code,
                            f.item.as_deref().unwrap_or("-"),
                            f.message
                        ));
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} bulgu:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Every north arrow of a book made the north diagram, magnetic (design §8a): the sheet's own
/// and its master page's.
fn with_north_diagram(book: &SheetBook) -> SheetBook {
    with_north(
        book,
        serde_json::json!({ "type": "northArrow", "style": "diagram", "north": "magnetic" }),
    )
}

/// Every north arrow of a book changed by `kind` (a merge of its kind's properties).
fn with_north(book: &SheetBook, kind: serde_json::Value) -> SheetBook {
    use kentos_sheet::ops::{Op, SetItemProps, apply};
    let arrows: Vec<ItemId> = book.sheets[0]
        .items
        .iter()
        .chain(book.masters.iter().flat_map(|m| m.items.iter()))
        .filter(|i| matches!(i.kind, ItemKind::NorthArrow(_)))
        .map(|i| i.id.clone())
        .collect();
    let mut book = book.clone();
    for id in arrows {
        let op = Op::SetItemProps(SetItemProps {
            id,
            patch: serde_json::json!({ "kind": kind }),
        });
        book = apply(&book, &op).expect("applied").book;
    }
    book
}

/// The same matrix with every north arrow of magnetic north, with its note and as the north
/// diagram: the note and the angles fit the arrow's frame on every paper (wrapped, at worst made
/// smaller to the legible size).
#[test]
fn every_template_on_every_paper_both_ways_with_the_north_diagram() {
    let mut failures = Vec::new();
    let mut arrows = 0;
    for t in system_templates() {
        for paper in MATRIX {
            for orientation in [Orientation::Portrait, Orientation::Landscape] {
                let (book, inputs) = filled(t, PaperChoice { paper, orientation });
                // The arrow with its note of magnetic north, then the diagram.
                let noted = with_north(
                    &book,
                    serde_json::json!({ "type": "northArrow", "north": "magnetic", "note": true }),
                );
                for f in preflight(&noted, "s1", &inputs).unwrap() {
                    if f.severity != Severity::Info {
                        failures.push(format!(
                            "{} {} {} (not): {} {} {}",
                            t.meta.id,
                            paper.id(),
                            orientation_tag(orientation),
                            f.code,
                            f.item.as_deref().unwrap_or("-"),
                            f.message
                        ));
                    }
                }
                let book = with_north_diagram(&book);
                arrows += book.sheets[0]
                    .items
                    .iter()
                    .filter(|i| matches!(&i.kind, ItemKind::NorthArrow(n) if n.style == NorthArrowStyle::Diagram))
                    .count();
                for f in preflight(&book, "s1", &inputs).unwrap() {
                    if f.severity != Severity::Info {
                        failures.push(format!(
                            "{} {} {}: {} {} {}",
                            t.meta.id,
                            paper.id(),
                            orientation_tag(orientation),
                            f.code,
                            f.item.as_deref().unwrap_or("-"),
                            f.message
                        ));
                    }
                }
            }
        }
    }
    assert!(arrows > 0, "no template has a north arrow");
    assert!(
        failures.is_empty(),
        "{} bulgu:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Every template on its recommended paper with the north diagram of magnetic north, for review:
/// `.run/shots/sheet/templates/north-diagram/<id>.html` (and the narrowest papers, A4 both ways).
#[test]
#[ignore]
fn north_diagram_shots() {
    let dir = root().join(".run/shots/sheet/templates/north-diagram");
    std::fs::create_dir_all(&dir).unwrap();
    let mut list = String::new();
    for t in system_templates() {
        let name = t.meta.id.trim_start_matches("sys:");
        let mut choices = vec![(name.to_owned(), t.meta.papers[0])];
        for o in [Orientation::Portrait, Orientation::Landscape] {
            choices.push((
                format!("{name}-a4-{}", orientation_tag(o)),
                PaperChoice {
                    paper: Paper::A4,
                    orientation: o,
                },
            ));
        }
        for (file, choice) in choices {
            let (book, inputs) = filled(t, choice);
            let book = with_north_diagram(&book);
            let list_ = display_list(&book, "s1", &inputs).unwrap();
            let options = SvgOptions {
                maps: map_images(&list_),
                title: Some(t.meta.name.clone()),
                ..SvgOptions::default()
            };
            let svg = to_svg(&list_, &options);
            let size = book.sheets[0].page.size;
            let (w, h) = (
                f64::from(size.width) / 1000.0,
                f64::from(size.height) / 1000.0,
            );
            std::fs::write(dir.join(format!("{file}.svg")), &svg).unwrap();
            std::fs::write(dir.join(format!("{file}.html")), html(&svg, w, h)).unwrap();
            list.push_str(&format!(
                "{file} {} {}\n",
                (w * 10.0).round(),
                (h * 10.0).round()
            ));
        }
    }
    std::fs::write(dir.join("list.txt"), list).unwrap();
}

/// The pages of the matrix for the coordinator's review: `.run/shots/sheet/templates/matrix/<id>-<paper>-<yön>.html`.
#[test]
#[ignore]
fn template_matrix_shots() {
    let dir = root().join(".run/shots/sheet/templates/matrix");
    std::fs::create_dir_all(&dir).unwrap();
    let mut list = String::new();
    for t in system_templates() {
        let name = t.meta.id.trim_start_matches("sys:");
        for paper in MATRIX {
            for orientation in [Orientation::Portrait, Orientation::Landscape] {
                let (book, inputs) = filled(t, PaperChoice { paper, orientation });
                let list_ = display_list(&book, "s1", &inputs).unwrap();
                let options = SvgOptions {
                    maps: map_images(&list_),
                    title: Some(t.meta.name.clone()),
                    ..SvgOptions::default()
                };
                let svg = to_svg(&list_, &options);
                let size = book.sheets[0].page.size;
                let (w, h) = (
                    f64::from(size.width) / 1000.0,
                    f64::from(size.height) / 1000.0,
                );
                let variant = book.sheets[0]
                    .active_variant
                    .clone()
                    .unwrap_or_else(|| "temel".into());
                let file = format!("{name}-{}-{}", paper.id(), orientation_tag(orientation));
                std::fs::write(dir.join(format!("{file}.html")), html(&svg, w, h)).unwrap();
                list.push_str(&format!(
                    "{file} {} {} {variant}\n",
                    (w * 10.0).round(),
                    (h * 10.0).round()
                ));
            }
        }
    }
    std::fs::write(dir.join("list.txt"), list).unwrap();
}
