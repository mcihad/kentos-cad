//! A template's rules (docs/adr/0176 §1) as `fixtures/style/v1/object-templates.json` holds
//! them, written by hand from the ADR: every problem in the order the fields
//! are read, in the same words as the web's `model/objectTemplate.ts`
//! (`model/objectTemplate.test.ts` runs the same cases).

use std::path::PathBuf;

use kentos_native_style::library::{Source, StyleLibrary, TreeFilter};
use kentos_native_style::object_template::{member_issues, preview_symbol, template_issues};
use serde_json::{Value, json};

#[test]
fn every_template_case_says_what_the_web_says() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/style/v1/object-templates.json");
    let fixture: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("object-templates.json"))
            .expect("JSON");
    assert_eq!(fixture["format"], "kentos.object-template-cases");
    assert_eq!(fixture["version"], 1);
    let cases = fixture["cases"].as_array().expect("cases");
    assert!(cases.len() > 40, "{} cases", cases.len());
    let mut wrong = Vec::new();
    for c in cases {
        let id = c["id"].as_str().unwrap_or("?");
        let place = c["where"].as_str().unwrap_or("şablon");
        let want: Vec<String> = c["issues"]
            .as_array()
            .expect("issues")
            .iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect();
        let got = template_issues(&c["template"], place);
        if got != want {
            wrong.push(format!("{id}: {got:?}, beklenen {want:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Object templates in the style library (docs/adr/0176 §2), as the web's
/// `style/templateLibrary.test.ts` holds them: found by a search of their
/// description, copied into a project with the user symbol they draw with
/// and that symbol's drawings, pictured by their symbol or their look.
#[test]
fn templates_in_the_library_are_found_copied_with_what_they_draw_with_and_pictured() {
    let drawing = json!({ "kind": "asset", "id": "a-agac", "name": "Ağaç", "path": ["Çizimler"], "format": "svg",
        "data": "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 4 4\"/>", "width": 4, "height": 4 });
    let tree = json!({ "kind": "symbol", "id": "s-agac", "name": "Ağaç", "path": ["Semboller"],
        "symbol": { "type": "marker", "layers": [{ "id": "v", "type": "svg", "asset": "a-agac", "size": 4 }] } });
    let template = json!({ "kind": "template", "id": "t-agac", "name": "Ağaç", "path": ["Şablonlar"], "description": "Yeşil alanın ağaçları",
        "template": { "tool": "point", "layer": { "path": ["Peyzaj"], "name": "Ağaç" }, "symbol": "s-agac", "attrs": { "Tür": "Çınar" } } });
    let mut lib = StyleLibrary::default();
    lib.load(Source::User, &[drawing, tree.clone(), template], &[]);
    let found: Vec<String> = lib
        .tree(TreeFilter {
            query: Some("yeşil alan"),
            ..TreeFilter::default()
        })
        .iter()
        .flat_map(|c| c.all_items())
        .map(|(i, _)| i.id().to_owned())
        .collect();
    assert_eq!(found, ["t-agac"]);
    let copy = lib
        .copy("t-agac", Source::Project, None, None)
        .expect("a copy");
    let mut project: Vec<String> = lib
        .items(Some(Source::Project))
        .iter()
        .map(|(i, _)| i.id().to_owned())
        .collect();
    project.sort();
    let mut want = vec![
        "a-agac".to_owned(),
        copy.id().to_owned(),
        "s-agac".to_owned(),
    ];
    want.sort();
    assert_eq!(project, want);
    // Its own symbol, else its look in its tool's shape.
    let own = preview_symbol(
        &json!({ "tool": "point", "layer": { "path": [], "name": "A" }, "symbol": "s-agac" }),
        &lib,
    );
    assert_eq!(own, tree["symbol"]);
    let line = preview_symbol(
        &json!({ "tool": "polyline", "layer": { "path": [], "name": "Yol", "lineWeight": 0.5 }, "color": "#F5A524" }),
        &lib,
    );
    assert_eq!(
        line,
        json!({ "type": "line", "layers": [{ "id": "l", "type": "simpleLine", "color": "#F5A524", "width": 0.5 }] })
    );
    let area = preview_symbol(
        &json!({ "tool": "polygon", "layer": { "path": [], "name": "Parsel", "color": "#E5484D" }, "symbol": "yok" }),
        &lib,
    );
    assert_eq!(
        area,
        json!({ "type": "fill", "layers": [{ "id": "l", "type": "simpleLine", "color": "#E5484D", "width": 0.35 }] })
    );
}

/// The Şablonlar panel's list (docs/adr/0176 §4) as
/// `fixtures/style/v1/template-list.json` has it, written by hand from the
/// rule: the groups by their category paths in Turkish order, the templates
/// by their names, a search in the name, description, category, tool and
/// layer (`style/templateList.test.ts` runs the same cases).
#[test]
fn the_templates_panel_lists_what_the_web_lists() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/style/v1/template-list.json");
    let fixture: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("template-list.json"))
            .expect("JSON");
    assert_eq!(fixture["format"], "kentos.template-list-cases");
    let items = |source: &str| -> Vec<Value> {
        fixture["library"][source]
            .as_array()
            .expect("items")
            .clone()
    };
    let system = items("system")
        .into_iter()
        .filter_map(kentos_native_style::library::Item::from_value)
        .collect();
    let mut lib = StyleLibrary::with_system(system, Vec::new());
    lib.load(Source::User, &items("user"), &[]);
    lib.load(Source::Project, &items("project"), &[]);
    let cases = fixture["cases"].as_array().expect("cases");
    assert!(cases.len() >= 8, "{} cases", cases.len());
    for c in cases {
        let name = c["name"].as_str().unwrap_or("?");
        let got: Vec<Value> = kentos_native_style::object_template::listed(
            &lib,
            c["query"].as_str().expect("a query"),
        )
        .into_iter()
        .map(|g| {
            json!({
                "path": g.path,
                "ids": g.items.into_iter().map(|(id, _)| id).collect::<Vec<_>>(),
            })
        })
        .collect();
        assert_eq!(Value::from(got), c["groups"], "{name}");
    }
}

/// A template made from a drawn object (docs/adr/0176 §4) as
/// `fixtures/style/v1/template-from-object.json` has it, written from the
/// rule by an independent script (`model/objectTemplate.test.ts` runs the
/// same cases).
#[test]
fn a_template_made_from_an_object_is_the_webs() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/style/v1/template-from-object.json");
    let fixture: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("template-from-object.json"))
            .expect("JSON");
    assert_eq!(fixture["format"], "kentos.template-from-object-cases");
    let layers: Vec<kentos_contracts::LayerNode> =
        serde_json::from_value(fixture["layers"].clone()).expect("layers");
    let blocks = fixture["blocks"].as_array().expect("blocks").clone();
    let block_name = |id: &kentos_contracts::BlockId| {
        blocks
            .iter()
            .find(|b| b["id"].as_str() == Some(id.to_text().as_str()))
            .and_then(|b| b["name"].as_str())
            .map(str::to_owned)
    };
    let cases = fixture["cases"].as_array().expect("cases");
    assert!(cases.len() >= 12, "{} cases", cases.len());
    let plot_scale = fixture["plotScale"].as_f64().expect("plotScale");
    for c in cases {
        let name = c["name"].as_str().unwrap_or("?");
        let entity: kentos_contracts::Entity =
            serde_json::from_value(c["entity"].clone()).expect("an entity");
        let scale = c["plotScale"].as_f64().unwrap_or(plot_scale);
        let got = match kentos_native_style::object_template::from_object(
            &entity, &layers, block_name, scale,
        ) {
            Ok((name, template)) => {
                // What it makes is a template the app draws with.
                assert!(template_issues(&template, "şablon").is_empty(), "{name}");
                json!({ "name": name, "template": template })
            }
            Err(refused) => json!({ "refused": refused }),
        };
        assert_eq!(got, c["result"], "{name}");
    }
}

/// A group template's members in the library (docs/adr/0176 §5), as
/// `fixtures/style/v1/template-groups.json` has them: what keeps each case
/// from starting, in the web's words.
#[test]
fn a_group_templates_members_are_found_as_the_web_finds_them() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/style/v1/template-groups.json");
    let fixture: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("template-groups.json"))
            .expect("JSON");
    assert_eq!(fixture["format"], "kentos.template-group-cases");
    assert_eq!(fixture["version"], 1);
    let library = fixture["library"].as_array().expect("library");
    let find = |id: &str| {
        library
            .iter()
            .find(|i| i["id"].as_str() == Some(id))
            .and_then(|i| Some((i["name"].as_str()?, &i["template"])))
    };
    let cases = fixture["cases"].as_array().expect("cases");
    assert!(cases.len() >= 10, "{} cases", cases.len());
    for c in cases {
        let name = c["name"].as_str().unwrap_or("?");
        // The template itself has no problems: what is said is the library's.
        assert_eq!(
            template_issues(&c["template"], "şablon"),
            Vec::<String>::new(),
            "{name}"
        );
        let want: Vec<String> = c["issues"]
            .as_array()
            .expect("issues")
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect();
        assert_eq!(member_issues(&c["template"], find), want, "{name}");
    }
}
