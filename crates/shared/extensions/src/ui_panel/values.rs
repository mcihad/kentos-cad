//! A panel component's worked-out values (plan/eklenti.md ADIM 22, PY-UI-08; docs/adr/8057 §35): `calculate` and
//! `defaultExpr`, expressions of KentOS's expression language over the panel's inputs by their ids, as a form's
//! calculated fields and defaults are written. The host works them out with no Python.
//!
//! - **`calculate`:** the component's value is its expression's, worked out again whenever an input it reads changes;
//!   the user does not type it (a field shows it read-only), its code does not set it. On a `text` component it gives
//!   the words, and the component's `text` shows while it gives nothing. A calculation may read another; one that
//!   reads itself, alone or round others, is refused at Kapı 1 ([`calc_order`]).
//! - **`defaultExpr`:** a field's first value, worked out once when the panel opens (after the fields above it got
//!   theirs); the user changes it as any value.
//! - **The moment:** both read `bugün()` and `şimdi()` (`@tarih`, `@simdi`), the moment the panel opened; no other `@`
//!   value. A calculation reading the moment does not move with the clock.
//! - **What a value is read as:** as a rule reads the inputs (rules.rs); the result is written to the field in its
//!   kind's words by the host (a number as a number, a date as a date), empty when it is not one.

use kentos_expression::{Expr, FieldType, Value, Variable};

use super::rules::{Values, schema_of};

/// The keys a component's worked-out values are written under.
pub const VALUE_KEYS: [&str; 2] = ["calculate", "defaultExpr"];

/// The `@` values a worked-out value reads: the day and the moment the panel opened.
pub const MOMENT: [&str; 2] = ["tarih", "simdi"];

/// When the panel opened, as the expressions read it: the day ISO (`2026-11-15`) and the moment ISO
/// (`2026-11-15T14:30:05`), as a project's `@tarih` and `@simdi`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Moment {
    pub today: String,
    pub now: String,
}

impl Moment {
    /// A moment to compile against where none is (Kapı 1): the expressions are checked, not worked out.
    pub fn checking() -> Moment {
        Moment {
            today: "2026-01-01".into(),
            now: "2026-01-01T00:00:00".into(),
        }
    }
}

/// Whether component `kind` takes worked-out value `key`: a calculation every component that shows one value (a text
/// its words), a default every typed field.
pub fn takes_value(kind: &str, key: &str) -> bool {
    let typed = matches!(
        kind,
        "textField" | "numberField" | "dateField" | "timeField" | "dateTimeField"
    );
    match key {
        "calculate" => typed || kind == "text",
        "defaultExpr" => typed,
        _ => false,
    }
}

/// A worked-out value compiled against the panel's inputs and the moment, or why not (Kapı 1's words).
pub fn compile(
    source: &str,
    inputs: &[(String, FieldType)],
    moment: &Moment,
) -> Result<Expr, String> {
    let mut schema = schema_of(inputs);
    schema.variables = vec![
        Variable {
            name: "tarih".into(),
            value: Value::text(moment.today.clone()),
            description: "Panelin açıldığı gün".into(),
        },
        Variable {
            name: "simdi".into(),
            value: Value::text(moment.now.clone()),
            description: "Panelin açıldığı an".into(),
        },
    ];
    let expr = kentos_expression::compile_with(source, &schema)
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
            "“@{name}” burada yok: hesap panelin girdilerini, bugün() ve şimdi()'yi okur."
        ));
    }
    Ok(expr)
}

/// `expr` worked out from the panel's values (`value` gives an input's as a rule reads it; none is empty); none when
/// it gives nothing.
pub fn value_of(expr: &Expr, value: impl Fn(&str) -> Option<String>) -> Option<Value<'static>> {
    let texts: Vec<Option<String>> = expr.fields.iter().map(|f| value(f)).collect();
    let out = expr.evaluate(&Values(&texts)).into_owned();
    (!kentos_expression::value::is_empty(&out)).then_some(out)
}

/// The order the calculations are worked out in, each after those it reads: `calculated` gives each its id (none for
/// a text without one) and the ids it reads. A calculation reading itself, alone or round others, is refused: its
/// place and the round in words.
pub fn calc_order(
    calculated: &[(Option<String>, Vec<String>)],
) -> Result<Vec<usize>, (usize, String)> {
    let mut order: Vec<usize> = Vec::with_capacity(calculated.len());
    let mut state = vec![0u8; calculated.len()]; // 0 not seen, 1 on the path, 2 placed
    fn visit(
        at: usize,
        calculated: &[(Option<String>, Vec<String>)],
        state: &mut [u8],
        path: &mut Vec<usize>,
        order: &mut Vec<usize>,
    ) -> Result<(), (usize, String)> {
        match state[at] {
            2 => return Ok(()),
            1 => {
                let from = path.iter().position(|p| *p == at).unwrap_or(0);
                let mut names: Vec<&str> = path[from..]
                    .iter()
                    .filter_map(|p| calculated[*p].0.as_deref())
                    .collect();
                names.extend(calculated[at].0.as_deref());
                let round = names.join(" → ");
                let first = calculated[at].0.as_deref().unwrap_or_default();
                return Err((
                    at,
                    if names.len() <= 2 && path.last() == Some(&at) {
                        format!("“{first}” hesabı kendi değerini okuyor.")
                    } else {
                        format!(
                            "Hesaplar birbirini okuyor: {round}; biri ötekinden önce hesaplanamaz."
                        )
                    },
                ));
            }
            _ => {}
        }
        state[at] = 1;
        path.push(at);
        for read in &calculated[at].1 {
            if let Some(next) = calculated
                .iter()
                .position(|(id, _)| id.as_deref() == Some(read.as_str()))
            {
                visit(next, calculated, state, path, order)?;
            }
        }
        path.pop();
        state[at] = 2;
        order.push(at);
        Ok(())
    }
    for at in 0..calculated.len() {
        visit(at, calculated, &mut state, &mut Vec::new(), &mut order)?;
    }
    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui_panel::rules::inputs_of;
    use serde_json::json;

    fn inputs() -> Vec<(String, FieldType)> {
        inputs_of(&[
            json!({ "type": "textField", "id": "onek" }),
            json!({ "type": "numberField", "id": "ilk" }),
            json!({ "type": "numberField", "id": "adet" }),
            json!({ "type": "checkbox", "id": "tamam" }),
            json!({ "type": "dateField", "id": "gun" }),
        ])
    }

    /// A calculation reads the inputs typed (a number as a number), joins text, and gives nothing when empty; the
    /// moment is the one it was compiled with.
    #[test]
    fn a_value_is_worked_out_from_the_panels_values_and_the_moment() {
        let inputs = inputs();
        let moment = Moment {
            today: "2026-11-15".into(),
            now: "2026-11-15T14:30:05".into(),
        };
        let values = |id: &str| match id {
            "onek" => Some("B-".to_owned()),
            "ilk" => Some("10".to_owned()),
            "adet" => Some("5".to_owned()),
            "tamam" => Some("true".to_owned()),
            _ => None,
        };
        let work = |source: &str| {
            let expr =
                compile(source, &inputs, &moment).unwrap_or_else(|e| panic!("{source}: {e}"));
            value_of(&expr, values).map(|v| kentos_expression::value::to_text(&v).into_owned())
        };
        assert_eq!(work("ilk + adet - 1").as_deref(), Some("14"));
        assert_eq!(work("onek || doldur(ilk, 2)").as_deref(), Some("B-10"));
        assert_eq!(
            work("eğer(tamam, 'evet', 'hayır')").as_deref(),
            Some("evet")
        );
        assert_eq!(work("bugün()").as_deref(), Some("2026-11-15"));
        assert_eq!(work("şimdi()").as_deref(), Some("2026-11-15T14:30:05"));
        assert_eq!(work("@tarih").as_deref(), Some("2026-11-15"));
        assert_eq!(
            work("eğer(boş(gun), boş, gun)"),
            None,
            "nothing: the text shows"
        );
    }

    /// Kapı 1's words: a name that is not an input, an `@` value other than the moment, an expression that does not
    /// compile; which components take which key.
    #[test]
    fn a_value_that_names_no_input_or_another_at_value_is_said() {
        let inputs = inputs();
        let moment = Moment::checking();
        assert!(
            compile("mahalle || 'x'", &inputs, &moment)
                .unwrap_err()
                .starts_with("“mahalle” paneldeki bir girdinin kimliği değil; girdiler: onek, ilk, adet, tamam, gun")
        );
        assert_eq!(
            compile("@proje_adi", &inputs, &moment).unwrap_err(),
            "“@proje_adi” burada yok: hesap panelin girdilerini, bugün() ve şimdi()'yi okur."
        );
        assert!(compile("ilk +", &inputs, &moment).is_err());
        assert!(takes_value("text", "calculate") && !takes_value("text", "defaultExpr"));
        assert!(
            takes_value("numberField", "defaultExpr") && takes_value("dateTimeField", "calculate")
        );
        assert!(!takes_value("select", "calculate") && !takes_value("button", "defaultExpr"));
    }

    /// Kapı 1 on a whole panel: calculations and defaults that fit pass; one that is typed, set first or asked of the
    /// user, a component that takes none, a round and an `@` value are said where they are written.
    #[test]
    fn kapi_1_says_where_a_worked_out_value_does_not_fit() {
        let problems = |body: serde_json::Value| -> Vec<(String, String)> {
            let panel = json!({ "format": "kentos.ui.panel", "schemaVersion": 1, "title": "Hesap", "body": body });
            crate::ui_panel::check("ui/hesap.panel.json", &panel, &["yenile"])
                .into_iter()
                .map(|p| (p.at.unwrap_or_default(), p.message))
                .collect()
        };
        let fits = problems(json!([
            { "type": "textField", "id": "onek", "label": "Önek", "value": "B-" },
            { "type": "numberField", "id": "ilk", "label": "İlk", "value": 10 },
            { "type": "numberField", "id": "adet", "label": "Adet", "defaultExpr": "ilk - 5" },
            { "type": "textField", "id": "son", "label": "Son", "calculate": "onek || doldur(ilk + adet - 1, 2)" },
            { "type": "dateField", "id": "gun", "label": "Gün", "defaultExpr": "bugün()" },
            { "type": "text", "id": "ozet", "text": "Bir önek yazın.", "calculate": "son || ' yazılır.'" },
            { "type": "text", "text": "Kimliksiz de olur.", "calculate": "eğer(boş(onek), boş, onek)" }
        ]));
        assert!(fits.is_empty(), "{fits:#?}");
        let found = problems(json!([
            { "type": "numberField", "id": "ilk", "label": "İlk", "value": 1, "required": true, "calculate": "2" },
            { "type": "textField", "id": "a", "label": "A", "value": "x", "defaultExpr": "'y'" },
            { "type": "select", "id": "s", "label": "S", "options": [{ "value": "a", "label": "A" }], "calculate": "'a'" },
            { "type": "textField", "id": "b", "label": "B", "calculate": "c" },
            { "type": "textField", "id": "c", "label": "C", "calculate": "b" },
            { "type": "textField", "id": "d", "label": "D", "defaultExpr": "@proje_adi" },
            { "type": "dateField", "id": "e", "label": "E", "calculate": "bugün()", "onChange": "yenile" }
        ]));
        let at = |pointer: &str| {
            found
                .iter()
                .filter(|(p, _)| p == pointer)
                .map(|(_, m)| m.as_str())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            at("/body/0/value"),
            ["calculate ile “value” birlikte olmaz: hesaplanan bileşen değerini hesabı yazar."]
        );
        assert_eq!(
            at("/body/0/required"),
            ["calculate ile “required” birlikte olmaz: hesaplanan bileşen kullanıcıya sorulmaz."]
        );
        assert_eq!(
            at("/body/1/value"),
            ["defaultExpr ile “value” birlikte olmaz: ilk değeri biri verir."]
        );
        assert!(
            at("/body/2/calculate")[0]
                .starts_with("“calculate” select bileşeninde bilinmeyen bir alan")
        );
        assert_eq!(
            at("/body/3/calculate"),
            ["Hesaplar birbirini okuyor: b → c → b; biri ötekinden önce hesaplanamaz."]
        );
        assert!(at("/body/5/defaultExpr")[0].starts_with("“@proje_adi” burada yok"));
        assert_eq!(
            at("/body/6/onChange"),
            [
                "calculate ile “onChange” birlikte olmaz: hesaplanan bileşen kullanıcı değiştirmez, onChange çalışmaz."
            ]
        );
        assert_eq!(found.len(), 7, "{found:#?}");
    }

    /// The calculations come each after those it reads; one that reads itself, or a round of them, is refused with
    /// the round named.
    #[test]
    fn calculations_come_after_what_they_read_and_a_round_is_refused() {
        let id = |s: &str| Some(s.to_owned());
        let reads = |r: &[&str]| r.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        // c reads b, b reads a: a, b, c whatever the body's order; a text without id reads b.
        let order = calc_order(&[
            (id("c"), reads(&["b"])),
            (None, reads(&["b"])),
            (id("b"), reads(&["a", "onek"])),
            (id("a"), reads(&["ilk"])),
        ])
        .unwrap();
        assert_eq!(order, vec![3, 2, 0, 1]);
        assert_eq!(
            calc_order(&[(id("a"), reads(&["a"]))]).unwrap_err(),
            (0, "“a” hesabı kendi değerini okuyor.".to_owned())
        );
        let (at, words) = calc_order(&[
            (id("a"), reads(&["b"])),
            (id("b"), reads(&["c"])),
            (id("c"), reads(&["a"])),
        ])
        .unwrap_err();
        assert_eq!(at, 0);
        assert_eq!(
            words,
            "Hesaplar birbirini okuyor: a → b → c → a; biri ötekinden önce hesaplanamaz."
        );
    }
}
