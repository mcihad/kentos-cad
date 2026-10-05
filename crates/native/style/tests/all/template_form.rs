//! The Şablon düzenleyici's form rules (docs/adr/0176 §4) as
//! `fixtures/style/v1/template-form.json` has them, written from the rules by
//! `scripts/fixtures/template_form_cases.py` (`model/templateForm.test.ts`
//! runs the same cases).

use std::path::PathBuf;

use kentos_native_style::template_form::{TemplateForm, from_form, to_form};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/style/v1/template-form.json");
    serde_json::from_str(&std::fs::read_to_string(&path).expect("template-form.json"))
        .expect("JSON")
}

/// What a form makes, as the cases write it.
fn made(form: &TemplateForm) -> Value {
    match from_form(form) {
        Ok(item) => {
            let mut out =
                json!({ "name": item.name, "path": item.path, "template": item.template });
            if let Some(description) = item.description {
                out["description"] = Value::from(description);
            }
            out
        }
        Err(issues) => json!({ "issues": issues }),
    }
}

#[test]
fn a_form_makes_the_webs_template_or_says_the_webs_problems() {
    let fixture = fixture();
    assert_eq!(fixture["format"], "kentos.template-form-cases");
    let cases = fixture["toTemplate"].as_array().expect("cases");
    assert!(cases.len() >= 15, "{} cases", cases.len());
    for c in cases {
        let name = c["name"].as_str().unwrap_or("?");
        let form: TemplateForm = serde_json::from_value(c["form"].clone()).expect("a form");
        assert_eq!(made(&form), c["result"], "{name}");
    }
}

#[test]
fn a_template_shows_the_webs_form_and_the_form_gives_it_back() {
    let fixture = fixture();
    for c in fixture["toForm"].as_array().expect("cases") {
        let name = c["name"].as_str().unwrap_or("?");
        let form = to_form(&c["item"]);
        let want: TemplateForm = serde_json::from_value(c["form"].clone()).expect("a form");
        assert_eq!(form, want, "{name}");
        let back = from_form(&form).expect("a template");
        assert_eq!(back.template, c["item"]["template"], "{name}");
    }
}
