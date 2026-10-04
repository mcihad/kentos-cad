//! Every family of `fixtures/sheet/v1` (its README says what each holds and
//! where its expectations come from). The web runs the same files through
//! `kentos-sheet-wasm`.

use crate::common;

use std::collections::BTreeSet;

use common::*;
use kentos_contracts::{ProjectType, Workspace};
use kentos_sheet::atlas::atlas_plan;
use kentos_sheet::display::{AtlasFeature, AtlasMapView, RenderInputs, display_list};
use kentos_sheet::hit::{HitQuery, hit_test};
use kentos_sheet::ops::{Op, SetPage, apply};
use kentos_sheet::preflight::preflight;
use kentos_sheet::profile::{Capabilities, profile_for, rank_templates, tool_availability};
use kentos_sheet::snap::{SnapOptions, SnapSession, snap_rotation};
use kentos_sheet::sync::{LocalTemplate, RemoteTemplate, plan_sync};
use kentos_sheet::template::{read_template, system_templates};
use kentos_sheet::validate::read_book;
use kentos_sheet::*;
use serde_json::{Value, json};

fn load(path: &str) -> Value {
    let text =
        std::fs::read_to_string(fixtures().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn book_of(v: &Value) -> SheetBook {
    read_book(&v.to_string()).unwrap()
}

fn files(dir: &str) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(fixtures().join(dir))
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".json"))
        .collect();
    out.sort();
    out
}

#[test]
fn relayout() {
    for f in files("relayout") {
        let fx = load(&format!("relayout/{f}"));
        let book = book_of(&fx["book"]);
        let sheet = fx["sheet"].as_str().unwrap();
        for case in fx["cases"].as_array().unwrap() {
            let page: Page = serde_json::from_value(case["page"].clone()).unwrap();
            let op = Op::SetPage(SetPage {
                owner: Owner::sheet(sheet),
                page,
                relayout: true,
            });
            let out = apply(&book, &op).unwrap().book;
            for (id, want) in case["expect"].as_object().unwrap() {
                let want: RectUm = serde_json::from_value(want.clone()).unwrap();
                let have = out.item(id).unwrap().frame;
                assert_eq!(have, want, "{f}: {}: {id}", case["description"]);
            }
            if let Some(v) = case.get("expectVariant") {
                let have = out.sheet(sheet).unwrap().active_variant.clone();
                assert_eq!(
                    serde_json::to_value(have).unwrap(),
                    *v,
                    "{f}: {}",
                    case["description"]
                );
            }
        }
    }
}

#[test]
fn snap() {
    for f in files("snap") {
        let fx = load(&format!("snap/{f}"));
        let book = book_of(&fx["book"]);
        let sheet = fx["sheet"].as_str().unwrap();
        let moving: Vec<String> = serde_json::from_value(fx["moving"].clone()).unwrap();
        let options: SnapOptions =
            serde_json::from_value(fx.get("options").cloned().unwrap_or(json!({}))).unwrap();
        let session = SnapSession::new(&book, sheet, &moving, options).unwrap();
        for q in fx["queries"].as_array().unwrap() {
            let delta: PointUm = serde_json::from_value(q["delta"].clone()).unwrap();
            let r = session.query(delta, q["tolerance"].as_i64().unwrap() as i32);
            assert_eq!(
                serde_json::to_value(&r).unwrap(),
                q["expect"],
                "{f}: {}",
                q["description"]
            );
        }
        for q in fx["resize"].as_array().unwrap() {
            let handle = serde_json::from_value(q["handle"].clone()).unwrap();
            let to: PointUm = serde_json::from_value(q["to"].clone()).unwrap();
            let r =
                session.query_resize(handle, to, q["tolerance"].as_i64().unwrap() as i32, false);
            assert_eq!(
                serde_json::to_value(&r).unwrap(),
                q["expect"],
                "{f}: {}",
                q["description"]
            );
        }
        for q in fx["rotation"].as_array().unwrap() {
            let got = snap_rotation(
                q["angle"].as_i64().unwrap() as i32,
                q["step"].as_bool().unwrap(),
            );
            assert_eq!(i64::from(got), q["expect"].as_i64().unwrap(), "{f}: {q}");
        }
    }
}

#[test]
fn sync() {
    for f in files("sync") {
        let fx = load(&format!("sync/{f}"));
        for case in fx["cases"].as_array().unwrap() {
            let local: Vec<LocalTemplate> = serde_json::from_value(case["local"].clone()).unwrap();
            let remote: Vec<RemoteTemplate> =
                serde_json::from_value(case["remote"].clone()).unwrap();
            let plan = plan_sync(&local, &remote);
            assert_eq!(
                serde_json::to_value(&plan.actions).unwrap(),
                case["expect"],
                "{f}: {}",
                case["description"]
            );
        }
    }
}

#[test]
fn profiles() {
    let fx = load("profiles/modes.json");
    for case in fx["profiles"].as_array().unwrap() {
        let ws: Option<Workspace> = serde_json::from_value(case["workspace"].clone()).unwrap();
        let caps: Capabilities = serde_json::from_value(case["capabilities"].clone()).unwrap();
        let d = case["description"].as_str().unwrap();
        let p = profile_for(ws, &caps);
        let e = &case["expect"];
        assert_eq!(p.id, e["profile"].as_str().unwrap(), "{d}");
        assert_eq!(
            p.default_template,
            e["defaultTemplate"].as_str().unwrap(),
            "{d}"
        );
        let tools = tool_availability(ws, &caps);
        for (id, want) in e["tools"].as_object().unwrap() {
            let t = tools
                .iter()
                .find(|t| t.id == *id)
                .unwrap_or_else(|| panic!("{d}: {id} yok"));
            let have = serde_json::to_value(t).unwrap();
            for (k, v) in want.as_object().unwrap() {
                if k == "presets" {
                    let ids: Vec<&str> = t.presets.iter().map(|p| p.id.as_str()).collect();
                    let want: Vec<&str> = v
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|x| x.as_str().unwrap())
                        .collect();
                    assert_eq!(ids, want, "{d}: {id}");
                } else {
                    assert_eq!(&have[k], v, "{d}: {id}.{k}");
                }
            }
        }
        for id in e["absent"].as_array().unwrap() {
            assert!(
                !tools.iter().any(|t| t.id == id.as_str().unwrap()),
                "{d}: {id} olmamalı"
            );
        }
    }
    let metas: Vec<_> = system_templates().iter().map(|t| t.meta.clone()).collect();
    for case in fx["ranking"].as_array().unwrap() {
        let ws: Option<Workspace> = serde_json::from_value(case["workspace"].clone()).unwrap();
        let pt: Option<ProjectType> = serde_json::from_value(case["projectType"].clone()).unwrap();
        let ranked = rank_templates(&metas, ws, pt);
        assert_eq!(
            serde_json::to_value(&ranked).unwrap(),
            case["expect"],
            "{}",
            case["description"]
        );
    }
}

#[test]
fn atlas() {
    let fx = load("atlas/plan.json");
    let book = book_of(&fx["book"]);
    let features: Vec<AtlasFeature> = serde_json::from_value(fx["features"].clone()).unwrap();
    let plan = atlas_plan(&book, fx["sheet"].as_str().unwrap(), &features).unwrap();
    let e = &fx["expect"];
    assert_eq!(serde_json::to_value(&plan.warnings).unwrap(), e["warnings"]);
    let pages = e["pages"].as_array().unwrap();
    assert_eq!(plan.pages.len(), pages.len());
    for (have, want) in plan.pages.iter().zip(pages) {
        assert_eq!(have.feature.id, want["feature"].as_str().unwrap());
        assert_eq!(i64::from(have.index), want["index"].as_i64().unwrap());
        assert_eq!(i64::from(have.count), want["count"].as_i64().unwrap());
        assert_eq!(have.name, want["name"].as_str().unwrap());
        let maps: Vec<AtlasMapView> = serde_json::from_value(want["maps"].clone()).unwrap();
        assert_eq!(have.maps, maps, "{}", have.name);
    }
}

#[test]
fn preflight_findings() {
    for f in files("preflight") {
        let fx = load(&format!("preflight/{f}"));
        let book = book_of(&fx["book"]);
        let inputs: RenderInputs = serde_json::from_value(fx["inputs"].clone()).unwrap();
        let found = preflight(&book, fx["sheet"].as_str().unwrap(), &inputs).unwrap();
        let have: BTreeSet<(String, String, String)> = found
            .iter()
            .map(|x| {
                (
                    serde_json::to_value(x.severity)
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_owned(),
                    x.code.clone(),
                    x.item.clone().unwrap_or_default(),
                )
            })
            .collect();
        let want: BTreeSet<(String, String, String)> = fx["expect"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| {
                (
                    x["severity"].as_str().unwrap().to_owned(),
                    x["code"].as_str().unwrap().to_owned(),
                    x["item"].as_str().unwrap_or_default().to_owned(),
                )
            })
            .collect();
        assert_eq!(have, want, "{f}");
        // Errors first, and every finding says why and how.
        assert!(
            found.windows(2).all(|w| w[0].severity <= w[1].severity),
            "{f}: sıra"
        );
        assert!(
            found
                .iter()
                .all(|x| !x.message.is_empty() && !x.fix.is_empty()),
            "{f}: ileti"
        );
    }
}

#[test]
fn hit() {
    let fx = load("hit/basic.json");
    let book = book_of(&fx["book"]);
    for q in fx["queries"].as_array().unwrap() {
        let query: HitQuery = serde_json::from_value(q["query"].clone()).unwrap();
        let hits = hit_test(&book, fx["sheet"].as_str().unwrap(), &query).unwrap();
        assert_eq!(
            serde_json::to_value(&hits).unwrap(),
            q["expect"],
            "{}",
            q["description"]
        );
    }
}

#[test]
fn templates() {
    for f in files("templates/valid") {
        let text = std::fs::read_to_string(fixtures().join("templates/valid").join(&f)).unwrap();
        read_template(&text).unwrap_or_else(|e| panic!("{f}: {e}"));
    }
    for f in files("templates/invalid") {
        let fx = load(&format!("templates/invalid/{f}"));
        let err = read_template(&fx["template"].to_string()).expect_err(&f);
        assert_eq!(
            err.code,
            fx["expect"]["code"].as_str().unwrap(),
            "{f}: {}",
            err.message
        );
    }
}

/// `.kpafta`: the files read (and read again after writing), the broken ones refused with their codes.
#[test]
fn kpafta() {
    let dir = fixtures().join("kpafta/valid");
    let mut valid: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    valid.sort();
    assert!(!valid.is_empty());
    for path in valid {
        let text = std::fs::read_to_string(&path).unwrap();
        let file = kentos_sheet::kpafta::decode(&text)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(
            (file.format.as_str(), file.version),
            (kentos_sheet::kpafta::FORMAT, kentos_sheet::kpafta::VERSION)
        );
        let again = kentos_sheet::kpafta::decode(
            &kentos_sheet::kpafta::encode(&file.book, &file.assets).unwrap(),
        )
        .unwrap();
        assert_eq!(again, file, "{}", path.display());
    }
    for f in files("kpafta/invalid") {
        let fx = load(&format!("kpafta/invalid/{f}"));
        let text = match &fx["file"] {
            Value::String(s) => s.clone(),
            v => v.to_string(),
        };
        let err = kentos_sheet::kpafta::decode(&text).expect_err(&f);
        assert_eq!(
            err.code,
            fx["expect"]["code"].as_str().unwrap(),
            "{f}: {}",
            err.message
        );
    }
}

/// The display lists: recorded (`KENTOS_WRITE_SHEET=1`), held byte for byte; the WASM binding is held to the same.
#[test]
fn display() {
    for f in files("display") {
        let path = fixtures().join("display").join(&f);
        let mut fx = load(&format!("display/{f}"));
        let book = match &fx["book"] {
            Value::String(p) => {
                read_book(&std::fs::read_to_string(fixtures().join("display").join(p)).unwrap())
                    .unwrap()
            }
            v => book_of(v),
        };
        let inputs: RenderInputs = serde_json::from_value(fx["inputs"].clone()).unwrap();
        let list = display_list(&book, fx["sheet"].as_str().unwrap(), &inputs).unwrap();
        let have = serde_json::to_value(&list).unwrap();
        if writing() {
            fx["expect"] = have;
            std::fs::write(&path, serde_json::to_string_pretty(&fx).unwrap() + "\n").unwrap();
        } else {
            assert!(
                fx["expect"] == have,
                "{f}: çizim planı değişti (KENTOS_WRITE_SHEET=1 ile yazın, farkı okuyun)"
            );
        }
    }
}

#[test]
fn every_fixture_family_has_its_files() {
    for dir in [
        "ops",
        "relayout",
        "snap",
        "display",
        "svg",
        "templates/valid",
        "templates/invalid",
        "kpafta/valid",
        "kpafta/invalid",
        "sync",
        "preflight",
        "atlas",
        "profiles",
        "hit",
        "pdf",
        "glyphs",
        "wmm",
    ] {
        let n = std::fs::read_dir(fixtures().join(dir)).unwrap().count();
        assert!(n > 0, "{dir} boş");
    }
}

/// The missing-glyph rule (design §6, `glyphs/`): the pieces each face draws, the display list's
/// “?”, the preflight's findings, the faces a PDF embeds and the SVG's `<tspan>`s.
#[test]
fn glyphs() {
    use kentos_sheet::display::Prim;
    use kentos_sheet::pdf::{PdfFace, PdfInputs, PdfOptions, fonts_needed, to_pdf};
    use kentos_sheet::svg::{SvgOptions, to_svg};
    use kentos_sheet::text::{TextRun, text_runs};
    let fx = load("glyphs/runs.json");
    for case in fx["cases"].as_array().unwrap() {
        let have = text_runs(
            case["font"].as_str().unwrap(),
            case["weight"].as_u64().unwrap() as u16,
            case["italic"].as_bool().unwrap(),
            case["text"].as_str().unwrap(),
        );
        let want: Vec<TextRun> = serde_json::from_value(case["runs"].clone()).unwrap();
        assert_eq!(have, want, "{}", case["why"]);
    }
    let fx = load("glyphs/sheet.json");
    let book = book_of(&fx["book"]);
    let sheet = fx["sheet"].as_str().unwrap();
    let inputs: RenderInputs = serde_json::from_value(fx["inputs"].clone()).unwrap();
    let found = preflight(&book, sheet, &inputs).unwrap();
    let have: BTreeSet<(String, String, String)> = found
        .iter()
        .map(|x| {
            (
                serde_json::to_value(x.severity)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_owned(),
                x.code.clone(),
                x.item.clone().unwrap_or_default(),
            )
        })
        .collect();
    let want: BTreeSet<(String, String, String)> = fx["expect"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| {
            (
                x["severity"].as_str().unwrap().to_owned(),
                x["code"].as_str().unwrap().to_owned(),
                x["item"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    assert_eq!(have, want);
    // The findings name the characters and the faces.
    let named = |item: &str| {
        found
            .iter()
            .find(|f| f.code == "glyph_missing" && f.item.as_deref() == Some(item))
            .map(|f| f.message.clone())
            .unwrap()
    };
    assert!(
        named("kot").contains("“↑”")
            && named("kot").contains("Barlow 400")
            && named("kot").contains("Arimo 400")
    );
    assert!(named("taks").contains("“≤”") && named("taks").contains("“?”"));
    assert!(named("bedel").contains("Courier Prime 700") && named("bedel").contains("Barlow 600"));
    // The display list writes what is drawn.
    let list = display_list(&book, sheet, &inputs).unwrap();
    for (item, text) in fx["expectTexts"].as_object().unwrap() {
        let drawn: String = list
            .prims
            .iter()
            .filter_map(|p| match p {
                Prim::Text(t) if &t.item == item => Some(t.text.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(drawn, text.as_str().unwrap(), "{item}");
    }
    // The PDF embeds every face a piece is drawn with.
    let pdf_inputs = PdfInputs {
        render: inputs.clone(),
        fonts: pdf_fonts(),
        assets: Vec::new(),
        maps: Vec::new(),
        crs: None,
    };
    let faces = fonts_needed(&book, &pdf_inputs, &PdfOptions::default()).unwrap();
    let want: Vec<PdfFace> = serde_json::from_value(fx["expectFaces"].clone()).unwrap();
    assert_eq!(
        faces.iter().collect::<BTreeSet<_>>(),
        want.iter().collect::<BTreeSet<_>>()
    );
    let pdf = to_pdf(&book, &pdf_inputs, &PdfOptions::default()).unwrap();
    let t = String::from_utf8_lossy(&pdf);
    assert!(
        t.contains("+Arimo-Regular") && t.contains("+CourierPrime-Bold"),
        "the faces the pieces need"
    );
    // The SVG names the face of a borrowed letter.
    let svg = to_svg(&list, &SvgOptions::default());
    for s in fx["expectSvg"].as_array().unwrap() {
        assert!(svg.contains(s.as_str().unwrap()), "{s}");
    }
}
