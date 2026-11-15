//! A panel component's rules (plan/eklenti.md ADIM 22, PY-UI-08; docs/adr/8057 §33): `visibleWhen`, `requiredWhen`
//! and `enabledWhen`, expressions of KentOS's expression language over the panel's inputs by their ids, the profile a
//! form's rules speak (`visibleWhen`, `requiredWhen` there too). The host works them out as the user changes a value,
//! with no Python: a field shown, asked or usable by the panel's own values.
//!
//! - **Checked at Kapı 1:** a rule compiles against the panel's inputs, each by its id with its type (a number field a
//!   number, a check yes or no, a date or moment field a date, the rest text); a name that is not an input's id, an `@`
//!   value and a function that looks at other layers are said where they are written.
//! - **An id with a hyphen is written in brackets** (`[en-sik]`): bare, `en-sik` reads as a subtraction.
//! - **What a rule reads** ([`holds`]): an input's value as the expression reads it: a number as `4.5`, a check
//!   `true` or `false`, a date ISO, a multiple choice's chosen values joined (`içerir(bilgiler, 'tarih')`), the rest as
//!   written; an empty one is empty. A rule that is not true (empty too) does not hold.

use kentos_expression::{Expr, FieldDef, FieldSource, FieldType, Measured, Schema, Scope};
use serde_json::Value as Json;

/// The keys a component's rules are written under.
pub const RULE_KEYS: [&str; 3] = ["visibleWhen", "requiredWhen", "enabledWhen"];

/// A component's rules and worked-out values (values.rs), as written: the expressions read beside its keys.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rules {
    /// Shown only while this holds.
    pub visible_when: Option<String>,
    /// Asked of the user (as `required`) while this holds.
    pub required_when: Option<String>,
    /// Usable (a button pressed, a field typed in) only while this holds.
    pub enabled_when: Option<String>,
    /// Its value, worked out again as the inputs it reads change (`calculate`).
    pub calculate: Option<String>,
    /// Its first value, worked out when the panel opens (`defaultExpr`).
    pub default_expr: Option<String>,
    /// The tab whose page it is on (tabs.rs); none for every page.
    pub tab: Option<String>,
}

impl Rules {
    /// The rules a component's JSON writes; a key that is not text is Kapı 1's to say.
    pub fn of(component: &Json) -> Rules {
        let text = |key: &str| {
            component
                .get(key)
                .and_then(Json::as_str)
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(str::to_owned)
        };
        Rules {
            visible_when: text("visibleWhen"),
            required_when: text("requiredWhen"),
            enabled_when: text("enabledWhen"),
            calculate: text("calculate"),
            default_expr: text("defaultExpr"),
            tab: super::tabs::tab_of(component),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.visible_when.is_none()
            && self.required_when.is_none()
            && self.enabled_when.is_none()
            && self.calculate.is_none()
            && self.default_expr.is_none()
            && self.tab.is_none()
    }
}

/// The expression type an input of `kind` is read as; none for a component that holds no value.
pub fn input_type(kind: &str) -> Option<FieldType> {
    Some(match kind {
        "numberField" => FieldType::Number,
        "checkbox" => FieldType::Bool,
        // ISO text, which compares in time order (a form's dates and moments alike).
        "dateField" | "dateTimeField" => FieldType::Date,
        "textField" | "timeField" | "select" | "multiSelect" | "layerPicker" | "fieldPicker"
        | "featurePicker" | "filePicker" => FieldType::Text,
        _ => return None,
    })
}

/// The panel's inputs, (id, type), in the body's order.
pub fn inputs_of(body: &[Json]) -> Vec<(String, FieldType)> {
    body.iter()
        .filter_map(|c| {
            let ty = input_type(c.get("type").and_then(Json::as_str)?)?;
            Some((c.get("id").and_then(Json::as_str)?.to_owned(), ty))
        })
        .collect()
}

/// The schema a panel's rules compile against: its inputs, each a typed field of its id.
pub fn schema_of(inputs: &[(String, FieldType)]) -> Schema {
    Schema {
        fields: inputs
            .iter()
            .map(|(id, ty)| FieldDef {
                name: id.clone(),
                ty: *ty,
                source: FieldSource::User,
                description: String::new(),
            })
            .collect(),
        ..Schema::default()
    }
}

/// A rule compiled against the panel's inputs, or why not (Kapı 1's words).
pub fn compile(source: &str, inputs: &[(String, FieldType)]) -> Result<Expr, String> {
    let expr = kentos_expression::compile_with(source, &schema_of(inputs))
        .map_err(|e| format!("{} ({}. karakter).", e.message, e.at))?;
    if let Some(unknown) = expr
        .fields
        .iter()
        .find(|f| !inputs.iter().any(|(id, _)| id == *f))
    {
        let ids: Vec<&str> = inputs.iter().map(|(id, _)| id.as_str()).collect();
        return Err(format!(
            "“{unknown}” paneldeki bir girdinin kimliği değil; girdiler: {} (tireli kimlik köşeli parantezle: [en-sik]).",
            if ids.is_empty() {
                "yok".to_owned()
            } else {
                ids.join(", ")
            }
        ));
    }
    if let Some(name) = expr.unknown_variables.first() {
        return Err(format!(
            "“@{name}” panel kuralında yok: kural yalnız panelin girdilerini okur."
        ));
    }
    Ok(expr)
}

/// Whether `expr` holds for the panel's values: `value` gives an input's value by its id as the expression reads it
/// (module doc); none is empty.
pub fn holds(expr: &Expr, value: impl Fn(&str) -> Option<String>) -> bool {
    let texts: Vec<Option<String>> = expr.fields.iter().map(|f| value(f)).collect();
    kentos_expression::value::truthy(&expr.evaluate(&Values(&texts)))
}

/// The panel's values as one object an expression reads: its inputs only, no geometry (a worked-out value too,
/// values.rs).
pub(super) struct Values<'a>(pub(super) &'a [Option<String>]);

impl Scope for Values<'_> {
    fn field(&self, i: usize) -> Option<&str> {
        self.0.get(i)?.as_deref()
    }
    fn measured(&self) -> Measured {
        Measured::default()
    }
    fn vertices(&self) -> Option<f64> {
        None
    }
    fn kind(&self) -> &str {
        ""
    }
    fn layer(&self) -> &str {
        ""
    }
    fn label(&self) -> Option<&str> {
        None
    }
    fn index(&self) -> f64 {
        1.0
    }
    fn id(&self) -> f64 {
        0.0
    }
    fn scale(&self) -> Option<f64> {
        None
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn inputs() -> Vec<(String, FieldType)> {
        inputs_of(&[
            json!({ "type": "select", "id": "tur" }),
            json!({ "type": "numberField", "id": "adet" }),
            json!({ "type": "checkbox", "id": "tamam" }),
            json!({ "type": "multiSelect", "id": "bilgiler" }),
            json!({ "type": "dateField", "id": "gun" }),
            json!({ "type": "text", "text": "yazı" }),
            json!({ "type": "button", "id": "ara" }),
        ])
    }

    /// A rule reads the panel's values by id, typed as its inputs: a number compared as a number, a check as yes or
    /// no, a multiple choice's values as text, a date as ISO; empty does not hold.
    #[test]
    fn a_rule_reads_the_panels_values_by_their_ids_and_types() {
        let inputs = inputs();
        assert_eq!(inputs.len(), 5, "inputs only: no text, no button");
        let values = |id: &str| match id {
            "tur" => Some("Okul".to_owned()),
            "adet" => Some("12".to_owned()),
            "tamam" => Some("true".to_owned()),
            "bilgiler" => Some("alan, tarih".to_owned()),
            "gun" => Some("2026-11-15".to_owned()),
            _ => None,
        };
        for (rule, want) in [
            ("tur = 'Okul'", true),
            ("adet > 9", true),
            ("adet > 100", false),
            ("tamam", true),
            ("içerir(bilgiler, 'tarih')", true),
            ("gun >= '2026-11-01'", true),
            ("tur = 'Lise' or tamam and adet < 20", true),
        ] {
            let expr = compile(rule, &inputs).unwrap_or_else(|e| panic!("{rule}: {e}"));
            assert_eq!(holds(&expr, values), want, "{rule}");
        }
        let empty = compile("adet > 0", &inputs).unwrap();
        assert!(!holds(&empty, |_| None), "an empty value does not hold");
    }

    /// Kapı 1's words: a name that is not an input's id (with the inputs and the bracket hint), an `@` value, an
    /// expression that does not compile.
    #[test]
    fn a_rule_that_names_no_input_or_does_not_compile_is_said() {
        let inputs = inputs();
        let e = compile("mahalle = 'Merkez'", &inputs).unwrap_err();
        assert!(
            e.starts_with("“mahalle” paneldeki bir girdinin kimliği değil; girdiler: tur, adet, tamam, bilgiler, gun"),
            "{e}"
        );
        assert!(
            compile("@proje = 'x'", &inputs)
                .unwrap_err()
                .contains("@proje")
        );
        assert!(compile("tur = ", &inputs).is_err());
        assert!(
            compile("[en-sik] = 'x'", &[("en-sik".to_owned(), FieldType::Text)]).is_ok(),
            "a hyphened id in brackets"
        );
        assert_eq!(
            Rules::of(&json!({ "visibleWhen": " tamam ", "enabledWhen": "" })),
            Rules {
                visible_when: Some("tamam".into()),
                ..Rules::default()
            }
        );
    }
}
