//! A form's scope (Kapsam, docs/adr/0539): which records of its layer a Veri girişi reads and writes
//! through it. A layer's records are scoped by its live forms (published, not withdrawn): a record is
//! within reach when one of them has no scope or its scope holds true for it. A layer no live form
//! edits is not scoped. A scope that does not compile, or whose answer is empty or false, gives
//! nothing: what is not shown to be allowed is not.

use std::collections::BTreeMap;

use kentos_contracts::{FormBody, LayerField, ProjectForm};
use kentos_expression::{Expr, Unit, Variable, compile_with};

use crate::record::{Attrs, Object, evaluate, schema_in};

/// A scope compiled over the layer's fields, or why it is not one.
pub fn compile(
    scope: &str,
    fields: &[LayerField],
    variables: &[Variable],
    unit: Unit,
) -> Result<Expr, String> {
    let schema = schema_in(fields, variables, unit);
    let e = compile_with(scope, &schema)
        .map_err(|e| format!("Formun kapsamı okunamadı: {}", e.text()))?;
    if let Some(name) = e.fields.iter().find(|n| !fields.iter().any(|f| &f.name == *n)) {
        return Err(format!("Formun kapsamı katmanda olmayan “{name}” alanını okuyor."));
    }
    Ok(e)
}

/// Whether a compiled scope holds for a record: a true answer (an empty one does not).
pub fn holds(scope: &Expr, attrs: &Attrs, object: &Object) -> bool {
    let v = evaluate(scope, attrs, object);
    !kentos_expression::value::is_empty(&v) && kentos_expression::value::truthy(&v)
}

/// What one live form gives on its layer: every record, the ones its scope holds for, or none (a
/// scope that does not compile).
#[derive(Clone, Debug)]
enum Reach {
    All,
    Where(Box<Expr>),
    Nothing,
}

/// The scopes of a project's layers, from its forms' latest releases.
#[derive(Clone, Debug, Default)]
pub struct Scopes {
    layers: BTreeMap<String, Vec<Reach>>,
}

impl Scopes {
    /// The live forms' scopes, each compiled over its layer's fields (`fields_of`).
    pub fn new<'a>(
        forms: &[ProjectForm],
        fields_of: impl Fn(&str) -> &'a [LayerField],
        variables: &[Variable],
        unit: Unit,
    ) -> Scopes {
        let mut layers: BTreeMap<String, Vec<Reach>> = BTreeMap::new();
        for form in forms.iter().filter(|f| !f.withdrawn) {
            let Some(release) = form.releases.last() else {
                continue;
            };
            let reach = match &release.body.scope {
                None => Reach::All,
                Some(s) => match compile(s, fields_of(&form.layer), variables, unit) {
                    Ok(e) => Reach::Where(Box::new(e)),
                    Err(_) => Reach::Nothing,
                },
            };
            layers.entry(form.layer.clone()).or_default().push(reach);
        }
        Scopes { layers }
    }

    /// Whether any layer is narrowed (a layer every form of which reaches all is not).
    pub fn narrows(&self) -> bool {
        self.layers
            .values()
            .any(|r| !r.iter().any(|x| matches!(x, Reach::All)))
    }

    /// The layers narrowed: none of their forms reaches every record.
    pub fn scoped_layers(&self) -> Vec<String> {
        self.layers
            .iter()
            .filter(|(_, r)| !r.iter().any(|x| matches!(x, Reach::All)))
            .map(|(l, _)| l.clone())
            .collect()
    }

    /// Whether a record of `layer` is within reach.
    pub fn sees(&self, layer: &str, attrs: &Attrs, object: &Object) -> bool {
        let Some(reaches) = self.layers.get(layer) else {
            return true;
        };
        reaches.iter().any(|r| match r {
            Reach::All => true,
            Reach::Where(e) => holds(e, attrs, object),
            Reach::Nothing => false,
        })
    }
}

/// Why a body's scope keeps it from being published: it does not compile over the layer's fields.
pub fn scope_problem(
    body: &FormBody,
    fields: &[LayerField],
    variables: &[Variable],
    unit: Unit,
) -> Option<String> {
    body.scope
        .as_deref()
        .and_then(|s| compile(s, fields, variables, unit).err())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_expression::Value;

    fn fields() -> Vec<LayerField> {
        serde_json::from_value(serde_json::json!([
            { "name": "Sorumlu", "kind": "text" }, { "name": "Kat", "kind": "integer" }
        ]))
        .unwrap()
    }

    fn me(email: &str) -> Vec<Variable> {
        vec![Variable {
            name: "kullanici_eposta".into(),
            value: Value::text(email.to_owned()),
            description: String::new(),
        }]
    }

    fn attrs(pairs: &[(&str, &str)]) -> Attrs {
        pairs.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect()
    }

    fn form(layer: &str, scope: Option<&str>, withdrawn: bool) -> ProjectForm {
        let body = serde_json::json!({ "revision": 1, "items": [], "scope": scope });
        let typed: FormBody = serde_json::from_value(body.clone()).unwrap();
        serde_json::from_value(serde_json::json!({ "id": format!("f-{layer}-{}", scope.unwrap_or("")),
            "name": "F", "layer": layer, "draft": body, "withdrawn": withdrawn,
            "releases": [{ "version": 1, "digest": kentos_contracts::body_digest(&typed), "body": body }] }))
        .unwrap()
    }

    fn scopes(forms: &[ProjectForm], email: &str) -> Scopes {
        let f = fields();
        Scopes::new(forms, |_| f.as_slice(), &me(email), Unit::METRE)
    }

    #[test]
    fn a_scope_reaches_the_records_it_holds_true_for() {
        let s = scopes(&[form("bina", Some("Sorumlu = @kullanici_eposta"), false)], "ali@k.test");
        let o = Object::default();
        assert!(s.narrows());
        assert_eq!(s.scoped_layers(), vec!["bina".to_owned()]);
        assert!(s.sees("bina", &attrs(&[("Sorumlu", "ali@k.test")]), &o));
        assert!(!s.sees("bina", &attrs(&[("Sorumlu", "ayse@k.test")]), &o));
        // An empty answer does not reach: what is not shown to be allowed is not.
        assert!(!s.sees("bina", &attrs(&[]), &o));
        // A layer no form edits is not scoped.
        assert!(s.sees("yol", &attrs(&[]), &o));
    }

    #[test]
    fn a_form_without_a_scope_reaches_all_and_one_that_does_not_compile_nothing() {
        let o = Object::default();
        let all = scopes(
            &[form("bina", Some("Kat > 3"), false), form("bina", None, false)],
            "ali@k.test",
        );
        assert!(!all.narrows());
        assert!(all.sees("bina", &attrs(&[("Kat", "1")]), &o));
        let broken = scopes(&[form("bina", Some("Yok = 1"), false)], "ali@k.test");
        assert!(!broken.sees("bina", &attrs(&[("Kat", "9")]), &o));
        // A withdrawn form gives nothing, and a layer only it edits is no longer scoped by it.
        let gone = scopes(&[form("bina", Some("Kat > 3"), true)], "ali@k.test");
        assert!(!gone.narrows());
        // Two scopes: either reaches.
        let two = scopes(
            &[form("bina", Some("Kat > 3"), false), form("bina", Some("Sorumlu = @kullanici_eposta"), false)],
            "ali@k.test",
        );
        assert!(two.sees("bina", &attrs(&[("Kat", "9")]), &o));
        assert!(two.sees("bina", &attrs(&[("Kat", "1"), ("Sorumlu", "ali@k.test")]), &o));
        assert!(!two.sees("bina", &attrs(&[("Kat", "1"), ("Sorumlu", "x")]), &o));
    }

    #[test]
    fn a_changed_scope_is_said_between_versions() {
        let body = |scope: Option<&str>| -> FormBody {
            serde_json::from_value(serde_json::json!({ "revision": 1, "items": [], "scope": scope })).unwrap()
        };
        let words = |a: &FormBody, b: &FormBody| -> Vec<String> {
            crate::compare::compare(a, b).into_iter().map(|c| c.words).collect()
        };
        assert_eq!(
            words(&body(None), &body(Some("Kat > 3"))),
            ["Kapsam: Kat > 3."]
        );
        assert_eq!(
            words(&body(Some("Kat > 3")), &body(None)),
            ["Kapsam kaldırıldı: bütün kayıtlar."]
        );
        assert!(words(&body(Some("Kat > 3")), &body(Some("Kat > 3"))).is_empty());
    }

    #[test]
    fn the_preflight_says_a_scope_that_does_not_compile() {
        let body: FormBody = serde_json::from_value(
            serde_json::json!({ "revision": 1, "items": [], "scope": "Yok = @kullanici_eposta" }),
        )
        .unwrap();
        let issues = crate::preflight(&body, &fields(), &me("a@k.test"));
        assert!(
            issues
                .iter()
                .any(|i| i.code == crate::codes::SCOPE_INVALID && i.rule.as_deref() == Some("scope")),
            "{issues:?}"
        );
        assert!(scope_problem(&body, &fields(), &me("a@k.test"), Unit::METRE).is_some());
    }
}
