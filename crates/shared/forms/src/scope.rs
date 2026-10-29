//! A form's scope (Kapsam, docs/adr/0539): which records of its layer a Veri girişi reads and writes
//! through it. A layer's records are scoped by its live forms (published, not withdrawn): a record is
//! within reach when one of them has no scope or its scope holds true for it. A layer no live form
//! edits is not scoped. A scope that does not compile, or whose answer is empty or false, gives
//! nothing: what is not shown to be allowed is not.

use std::collections::{BTreeMap, BTreeSet};

use kentos_contracts::{FormBody, LayerField, LayerFieldKind, ProjectForm};
use kentos_expression::pushdown::{ColumnSort, Dialect, Pushed, push_after};
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
    /// Each scoped layer's fields the language reads as text: the ones a source may be sent an equality on.
    texts: BTreeMap<String, BTreeSet<String>>,
}

/// What a layer's scopes read of a record (docs/adr/0738 §3): what a change's earlier copy must keep for a
/// Veri girişi's reach of it to be told later.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScopeReads {
    /// The layer is not scoped: every record is within reach.
    Unscoped,
    /// These attributes alone: a record is judged by their values.
    Fields(BTreeSet<String>),
    /// More than its attributes (its shape, kind, label, other layers): only the whole record tells.
    More,
}

/// What a source is sent of a layer's scope (docs/adr/0649 §1). The records it hands over are judged
/// again with the scope, so the SQL only has to keep every record within reach.
#[derive(Clone, Debug, PartialEq)]
pub enum ScopeSql {
    /// No record of the layer is within reach: none is read.
    Nothing,
    /// The scope has no part the source can check: every record is read and judged.
    Judge,
    /// The records the SQL keeps are read and judged: a superset of the ones within reach.
    Where(Pushed),
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
        let mut texts: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
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
            texts.entry(form.layer.clone()).or_insert_with(|| {
                fields_of(&form.layer)
                    .iter()
                    .filter(|f| reads_as_text(f.kind))
                    .map(|f| f.name.clone())
                    .collect()
            });
        }
        Scopes { layers, texts }
    }

    /// The SQL a source can check of `layer`'s scope (docs/adr/0649 §1); none: the layer is not scoped
    /// (every record is within reach). `column` writes a field's SQL (quoted) and `taken` is how many
    /// values the query binds before these. The forms' scopes are joined with `OR`: a record is within
    /// reach when one of them holds; a form whose scope has no part to send makes the whole layer judged.
    pub fn sql_of(
        &self,
        layer: &str,
        column: &dyn Fn(&str) -> String,
        dialect: Dialect,
        taken: usize,
    ) -> Option<ScopeSql> {
        let reaches = self.layers.get(layer)?;
        if reaches.iter().any(|r| matches!(r, Reach::All)) {
            return None;
        }
        let texts = self.texts.get(layer);
        let columns = |name: &str| {
            texts
                .is_some_and(|t| t.contains(name))
                .then(|| (column(name), ColumnSort::Text))
        };
        let mut parts = Vec::new();
        let mut params = Vec::new();
        let mut exact = true;
        for r in reaches {
            let Reach::Where(e) = r else { continue };
            let p = push_after(e, &columns, dialect, taken + params.len());
            if p.sql.is_empty() {
                return Some(ScopeSql::Judge);
            }
            parts.push(p.sql);
            params.extend(p.params);
            exact &= p.exact;
        }
        Some(match parts.len() {
            0 => ScopeSql::Nothing,
            1 => ScopeSql::Where(Pushed {
                sql: parts.remove(0),
                params,
                exact,
            }),
            _ => ScopeSql::Where(Pushed {
                sql: format!("({})", parts.join(" OR ")),
                params,
                exact,
            }),
        })
    }

    /// What `layer`'s scopes read of a record (docs/adr/0738 §3): a scope that does not compile reads nothing (it
    /// reaches no record).
    pub fn reads(&self, layer: &str) -> ScopeReads {
        let Some(reaches) = self.layers.get(layer) else {
            return ScopeReads::Unscoped;
        };
        if reaches.iter().any(|r| matches!(r, Reach::All)) {
            return ScopeReads::Unscoped;
        }
        let mut fields = BTreeSet::new();
        for r in reaches {
            if let Reach::Where(e) = r {
                if e.needs != kentos_expression::Needs::default() || !e.world.is_empty() {
                    return ScopeReads::More;
                }
                fields.extend(e.fields.iter().cloned());
            }
        }
        ScopeReads::Fields(fields)
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

/// Whether the language reads a field of `kind` as its stored text (docs/adr/0412 §3, 0629 §3): an
/// equality on it is the stored text's.
fn reads_as_text(kind: LayerFieldKind) -> bool {
    matches!(
        kind,
        LayerFieldKind::Text
            | LayerFieldKind::Time
            | LayerFieldKind::BigInt
            | LayerFieldKind::Instant
            | LayerFieldKind::Uuid
            | LayerFieldKind::Json
            | LayerFieldKind::Binary
    )
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

    /// docs/adr/0738 §3: what a layer's scopes read, for a change's earlier copy to keep.
    #[test]
    fn a_scope_says_what_it_reads() {
        let set = |names: &[&str]| -> BTreeSet<String> {
            names.iter().map(|n| (*n).to_owned()).collect()
        };
        let two = scopes(
            &[
                form("bina", Some("Kat > 3"), false),
                form("bina", Some("Sorumlu = @kullanici_eposta"), false),
            ],
            "ali@k.test",
        );
        assert_eq!(
            two.reads("bina"),
            ScopeReads::Fields(set(&["Kat", "Sorumlu"]))
        );
        // A layer no form scopes, and one a form without a scope reaches whole, are not scoped.
        assert_eq!(two.reads("yol"), ScopeReads::Unscoped);
        let all = scopes(
            &[
                form("bina", Some("Kat > 3"), false),
                form("bina", None, false),
            ],
            "a",
        );
        assert_eq!(all.reads("bina"), ScopeReads::Unscoped);
        // The shape read: only the whole record tells.
        let shape = scopes(&[form("bina", Some("$alan > 100"), false)], "a");
        assert_eq!(shape.reads("bina"), ScopeReads::More);
        // A scope that does not compile reaches nothing and reads nothing.
        let broken = scopes(&[form("bina", Some("Yok = 1"), false)], "a");
        assert_eq!(broken.reads("bina"), ScopeReads::Fields(BTreeSet::new()));
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

    fn pg(name: &str) -> String {
        format!("(properties->>'{}')", name.replace('\'', "''"))
    }

    #[test]
    fn a_scope_is_sent_as_sql_on_its_text_fields() {
        let s = scopes(&[form("bina", Some("Sorumlu = @kullanici_eposta"), false)], "ali@k.test");
        let Some(ScopeSql::Where(p)) = s.sql_of("bina", &pg, Dialect::Postgres, 3) else {
            panic!("{:?}", s.sql_of("bina", &pg, Dialect::Postgres, 3));
        };
        assert_eq!(p.sql, "(properties->>'Sorumlu') = $4");
        assert_eq!(p.params, vec![kentos_expression::pushdown::Param::Text("ali@k.test".into())]);
        assert!(p.exact);
        // A layer no form edits, or one a form reaches all of, is not narrowed.
        assert_eq!(s.sql_of("yol", &pg, Dialect::Postgres, 3), None);
        let all = scopes(&[form("bina", Some("Kat > 3"), false), form("bina", None, false)], "a@k.test");
        assert_eq!(all.sql_of("bina", &pg, Dialect::Postgres, 0), None);
    }

    #[test]
    fn two_forms_are_joined_and_what_cannot_be_sent_is_judged() {
        let two = scopes(
            &[
                form("bina", Some("Sorumlu = 'a@k.test'"), false),
                form("bina", Some("Sorumlu in ('b@k.test', 'c@k.test')"), false),
            ],
            "x@k.test",
        );
        let Some(ScopeSql::Where(p)) = two.sql_of("bina", &pg, Dialect::Postgres, 3) else {
            panic!()
        };
        assert_eq!(
            p.sql,
            "((properties->>'Sorumlu') = $4 OR ((properties->>'Sorumlu') = $5 OR (properties->>'Sorumlu') = $6))"
        );
        assert_eq!(p.params.len(), 3);
        // A number field is read as a number: its stored text is not compared in SQL.
        let number = scopes(&[form("bina", Some("Kat > 3"), false)], "x@k.test");
        assert_eq!(number.sql_of("bina", &pg, Dialect::Postgres, 0), Some(ScopeSql::Judge));
        // One form with nothing to send makes the layer judged, whatever the other sends.
        let mixed = scopes(
            &[form("bina", Some("Sorumlu = 'a@k.test'"), false), form("bina", Some("Kat > 3"), false)],
            "x@k.test",
        );
        assert_eq!(mixed.sql_of("bina", &pg, Dialect::Postgres, 0), Some(ScopeSql::Judge));
        // A part of an `and` the source cannot check is left out: the SQL keeps more, not fewer.
        let and = scopes(&[form("bina", Some("Sorumlu = 'a@k.test' and Kat > 3"), false)], "x@k.test");
        let Some(ScopeSql::Where(p)) = and.sql_of("bina", &pg, Dialect::Postgres, 0) else { panic!() };
        assert_eq!(p.sql, "(properties->>'Sorumlu') = $1");
        assert!(!p.exact);
    }

    #[test]
    fn a_scope_that_does_not_compile_reads_nothing() {
        let s = scopes(&[form("bina", Some("Yok = 1"), false)], "x@k.test");
        assert_eq!(s.sql_of("bina", &pg, Dialect::Postgres, 0), Some(ScopeSql::Nothing));
        // A field name with a quote is quoted, not run.
        assert_eq!(pg("a'b"), "(properties->>'a''b')");
    }
}
