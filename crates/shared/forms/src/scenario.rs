//! A form's scenarios (docs/adr/0420): values typed onto an empty record (a new one takes the
//! defaults), and what the form made of them: the record after it, the items shown, required and
//! read-only, and what it said. A scenario is captured from the designer's preview and run again
//! whenever the form changes; its differences say what changed, one sentence each, in a fixed order:
//! the record's values, then shown, required and read-only items, then the issues.

use kentos_contracts::{FormTest, FormTestExpect};

use crate::record::{Attrs, Object};
use crate::session::Prepared;

/// What the form makes of `values` typed onto an empty record (`new`: with its defaults).
pub fn outcome(
    prepared: &Prepared,
    values: &std::collections::BTreeMap<String, String>,
    new: bool,
) -> FormTestExpect {
    let object = Object::default();
    let typed: Vec<(String, String)> = values.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    let (after, issues) = prepared.assess(&Attrs::new(), &typed, &object, new);
    let states = prepared.described(&after, &object);
    let ids = |keep: &dyn Fn(&kentos_contracts::FormItemState) -> bool| {
        states
            .iter()
            .filter(|s| keep(s))
            .map(|s| s.id.clone())
            .collect::<Vec<_>>()
    };
    FormTestExpect {
        attrs: after,
        shown: ids(&|s| s.shown),
        required: ids(&|s| s.required),
        read_only: ids(&|s| s.read_only),
        issues: issues
            .iter()
            .map(|i| match &i.item {
                Some(item) => format!("{}@{item}", i.code),
                None => i.code.clone(),
            })
            .collect(),
    }
}

/// A scenario of `values` named `name`, expecting what the form makes of them now.
pub fn capture(
    prepared: &Prepared,
    name: &str,
    values: std::collections::BTreeMap<String, String>,
    new: bool,
) -> FormTest {
    let expect = outcome(prepared, &values, new);
    FormTest {
        name: name.to_owned(),
        values,
        new,
        expect,
    }
}

fn list(items: &[String]) -> String {
    if items.is_empty() {
        "hiçbiri".to_owned()
    } else {
        items.join(", ")
    }
}

/// How the form's outcome differs from what `test` expects; empty when it passes.
pub fn run(prepared: &Prepared, test: &FormTest) -> Vec<String> {
    let got = outcome(prepared, &test.values, test.new);
    let want = &test.expect;
    let mut out = Vec::new();
    let mut names: Vec<&String> = want.attrs.keys().chain(got.attrs.keys()).collect();
    names.sort();
    names.dedup();
    for name in names {
        let (w, g) = (want.attrs.get(name), got.attrs.get(name));
        if w != g {
            let say = |v: Option<&String>| v.map_or_else(|| "boş".to_owned(), |v| format!("“{v}”"));
            out.push(format!(
                "“{name}” değeri: beklenen {}, bulunan {}.",
                say(w),
                say(g)
            ));
        }
    }
    for (words, w, g) in [
        ("Gösterilen öğeler", &want.shown, &got.shown),
        ("Zorunlu öğeler", &want.required, &got.required),
        ("Salt okunur öğeler", &want.read_only, &got.read_only),
        ("Sorunlar", &want.issues, &got.issues),
    ] {
        if w != g {
            out.push(format!(
                "{words}: beklenen {}, bulunan {}.",
                list(w),
                list(g)
            ));
        }
    }
    out
}
