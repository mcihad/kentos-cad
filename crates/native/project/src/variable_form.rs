//! Proje ayarları › Değişkenler's form (docs/adr/0214 §2.3, §4): the
//! project's variables as rows of texts (name, label, kind, value), the
//! rows read back into variables with what is said of a row that does not
//! hold, the row a new variable starts as and a row whose kind changes. A
//! name is a letter or `_`, then letters, digits and `_`, compared with
//! Turkish letters and case aside; a value is typed as its kind reads it
//! (a number as the Hesap windows read one, a date as YYYY-AA-GG or
//! GG.AA.YYYY, true/false from the switch). The web's twin is
//! `apps/web/src/model/variableForm.ts`; both pass
//! `fixtures/project/v1/variable-form.json`, which
//! `scripts/fixtures/variable_form_cases.py` writes from the rules alone.

use kentos_contracts::{
    ProjectVariable, VARIABLE_TEXT_MAX, VARIABLES_MAX, VariableKind, VariableValue, is_date,
    variable_key, variable_name_problem,
};

use crate::definition_form::number;

/// What is said of a row that does not hold.
pub const NAME_EMPTY: &str = "Bir ad yazın: harf ya da _ ile başlar, harf, rakam ve _ içerir.";
pub const NUMBER: &str = "Sayı yazın (1.5 ya da 1,5); değeri yoksa boş bırakın.";
pub const DATE: &str = "Tarihi YYYY-AA-GG ya da GG.AA.YYYY yazın; değeri yoksa boş bırakın.";
pub const BOOL: &str = "Doğru ya da yanlış seçin.";
pub const TOO_MANY: &str = "En çok 200 değişken olabilir.";

/// The switch's two values, as a row keeps them.
pub const TRUE: &str = "doğru";
pub const FALSE: &str = "yanlış";

/// One variable as the form types it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row {
    pub name: String,
    pub label: String,
    pub kind: VariableKind,
    pub value: String,
}

/// What does not hold in a row: its name, its value.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Problems {
    pub name: Option<String>,
    pub value: Option<String>,
}

/// The rows read back.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Read {
    /// The rows that hold, in order.
    pub variables: Vec<ProjectVariable>,
    /// Each row's problems, in order.
    pub problems: Vec<Problems>,
    /// Too many rows.
    pub list: Option<&'static str>,
}

impl Read {
    /// Whether a row has something to put right: Kaydet waits.
    pub fn blocked(&self) -> bool {
        self.list.is_some()
            || self
                .problems
                .iter()
                .any(|p| p.name.is_some() || p.value.is_some())
    }
}

/// A number as the form writes it: the shortest text that reads back as
/// it, without an exponent; -0 is "0".
pub fn number_text(x: f64) -> String {
    if x == 0.0 {
        "0".to_owned()
    } else {
        format!("{x}")
    }
}

/// A variable's value as the form writes it for its kind.
pub fn value_text(value: &VariableValue) -> String {
    match value {
        VariableValue::Null => String::new(),
        VariableValue::Bool(true) => TRUE.to_owned(),
        VariableValue::Bool(false) => FALSE.to_owned(),
        VariableValue::Number(x) => number_text(*x),
        VariableValue::Text(t) => t.clone(),
    }
}

/// The form's rows of a project's variables.
pub fn rows(list: &[ProjectVariable]) -> Vec<Row> {
    list.iter()
        .map(|v| Row {
            name: v.name.clone(),
            label: v.label.clone(),
            kind: v.kind,
            value: value_text(&v.value),
        })
        .collect()
}

/// `G.A.YYYY` or `GG.AA.YYYY` as `YYYY-AA-GG`; none for anything else.
fn dotted(t: &str) -> Option<String> {
    let parts: Vec<&str> = t.split('.').collect();
    let [d, m, y] = parts.as_slice() else {
        return None;
    };
    let digits = |s: &str, min: usize, max: usize| {
        (min..=max).contains(&s.len()) && s.bytes().all(|c| c.is_ascii_digit())
    };
    if !(digits(d, 1, 2) && digits(m, 1, 2) && digits(y, 4, 4)) {
        return None;
    }
    Some(format!("{y}-{m:0>2}-{d:0>2}"))
}

/// A value typed for a kind, or what is said of it; a blank text is no value.
pub fn read_value(kind: VariableKind, typed: &str) -> Result<VariableValue, &'static str> {
    let t = typed.trim();
    if t.is_empty() {
        return Ok(VariableValue::Null);
    }
    match kind {
        VariableKind::Text if t.chars().count() > VARIABLE_TEXT_MAX => Err(TEXT_LONG),
        VariableKind::Text => Ok(VariableValue::Text(t.to_owned())),
        VariableKind::Number => number(t)
            .filter(|x| x.is_finite())
            .map(VariableValue::Number)
            .ok_or(NUMBER),
        VariableKind::Date => {
            let iso = if is_date(t) {
                Some(t.to_owned())
            } else {
                dotted(t).filter(|d| is_date(d))
            };
            iso.map(VariableValue::Text).ok_or(DATE)
        }
        VariableKind::Bool => match t {
            TRUE => Ok(VariableValue::Bool(true)),
            FALSE => Ok(VariableValue::Bool(false)),
            _ => Err(BOOL),
        },
    }
}

/// What is said of a text value longer than a variable keeps.
pub const TEXT_LONG: &str = "Değer 4000 karakterden uzun.";

/// The rows read back: the variables that hold and every row's problems.
pub fn read(rows: &[Row]) -> Read {
    let mut out = Read {
        list: (rows.len() > VARIABLES_MAX).then_some(TOO_MANY),
        ..Read::default()
    };
    let mut seen = std::collections::HashSet::new();
    for r in rows {
        let trimmed = r.name.trim();
        let name = trimmed.strip_prefix('@').unwrap_or(trimmed);
        let mut named = if name.is_empty() {
            Some(NAME_EMPTY.to_owned())
        } else {
            variable_name_problem(name).map(|p| format!("{p}."))
        };
        if named.is_none() && !seen.insert(variable_key(name)) {
            named = Some(format!("@{name} adı yukarıda var; başka ad seçin."));
        }
        let value = read_value(r.kind, &r.value);
        if named.is_none()
            && let Ok(value) = &value
        {
            out.variables.push(ProjectVariable {
                name: name.to_owned(),
                label: r.label.trim().to_owned(),
                kind: r.kind,
                value: value.clone(),
            });
        }
        out.problems.push(Problems {
            name: named,
            value: value.err().map(str::to_owned),
        });
    }
    out
}

/// The row a new variable starts as: the first free name of `degisken1`,
/// `degisken2` …, no label, text, no value.
pub fn new_row(rows: &[Row]) -> Row {
    let taken: std::collections::HashSet<String> = rows
        .iter()
        .map(|r| {
            let t = r.name.trim();
            variable_key(t.trim_start_matches('@'))
        })
        .collect();
    let name = (1..)
        .map(|n| format!("degisken{n}"))
        .find(|n| !taken.contains(&variable_key(n)))
        .unwrap_or_default();
    Row {
        name,
        ..Row::default()
    }
}

/// A row whose kind changes: to true/false its value is "doğru" when it was,
/// else "yanlış"; from true/false it is emptied; between the others the text
/// stays (reading it says whether it holds).
pub fn with_kind(row: &Row, kind: VariableKind) -> Row {
    let value = if kind == VariableKind::Bool {
        if row.value.trim() == TRUE {
            TRUE
        } else {
            FALSE
        }
        .to_owned()
    } else if row.kind == VariableKind::Bool {
        String::new()
    } else {
        row.value.clone()
    };
    Row {
        kind,
        value,
        ..row.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const FIXTURE: &str = include_str!("../../../../fixtures/project/v1/variable-form.json");

    fn row_of(v: &Value) -> Row {
        Row {
            name: v["name"].as_str().unwrap_or("").to_owned(),
            label: v["label"].as_str().unwrap_or("").to_owned(),
            kind: serde_json::from_value(v["kind"].clone()).expect("a kind"),
            value: v["value"].as_str().unwrap_or("").to_owned(),
        }
    }

    fn row_json(r: &Row) -> Value {
        serde_json::json!({
            "name": r.name,
            "label": r.label,
            "kind": r.kind,
            "value": r.value,
        })
    }

    fn variables_of(v: &Value) -> Vec<ProjectVariable> {
        serde_json::from_value(v.clone()).expect("variables")
    }

    #[test]
    fn the_form_keeps_the_reference_s_rules() {
        let file: Value = serde_json::from_str(FIXTURE).expect("the fixture reads");
        assert_eq!(file["format"], "kentos.variable-form");
        let messages = &file["messages"];
        assert_eq!(messages["nameEmpty"], NAME_EMPTY);
        assert_eq!(messages["number"], NUMBER);
        assert_eq!(messages["date"], DATE);
        assert_eq!(messages["bool"], BOOL);
        assert_eq!(messages["tooMany"], TOO_MANY);
        let mut checked = 0;
        for case in file["reads"].as_array().expect("reads") {
            let rows: Vec<Row> = case["rows"]
                .as_array()
                .expect("rows")
                .iter()
                .map(row_of)
                .collect();
            let r = read(&rows);
            assert_eq!(
                r.variables,
                variables_of(&case["variables"]),
                "{}",
                case["case"]
            );
            let problems: Vec<Problems> = case["problems"]
                .as_array()
                .expect("problems")
                .iter()
                .map(|p| Problems {
                    name: p["name"].as_str().map(str::to_owned),
                    value: p["value"].as_str().map(str::to_owned),
                })
                .collect();
            assert_eq!(r.problems, problems, "{}", case["case"]);
            assert_eq!(r.list, None);
            checked += rows.len();
        }
        for case in file["rows"].as_array().expect("rows") {
            let got: Vec<Value> = rows(&variables_of(&case["variables"]))
                .iter()
                .map(row_json)
                .collect();
            assert_eq!(Value::Array(got), case["rows"], "{}", case["case"]);
            // Written and read back, the variables are the same.
            let written = rows(&variables_of(&case["variables"]));
            let mut back = variables_of(&case["variables"]);
            for v in &mut back {
                if v.value == VariableValue::Number(-0.0) {
                    v.value = VariableValue::Number(0.0);
                }
            }
            assert_eq!(read(&written).variables, back);
        }
        for case in file["newRows"].as_array().expect("new rows") {
            let rows: Vec<Row> = case["rows"]
                .as_array()
                .expect("rows")
                .iter()
                .map(row_of)
                .collect();
            assert_eq!(row_json(&new_row(&rows)), case["row"], "{}", case["case"]);
        }
        for case in file["kinds"].as_array().expect("kinds") {
            let kind: VariableKind = serde_json::from_value(case["kind"].clone()).expect("a kind");
            assert_eq!(
                row_json(&with_kind(&row_of(&case["row"]), kind)),
                case["result"],
                "{}",
                case["case"]
            );
        }
        let many: Vec<Row> = (0..201)
            .map(|i| Row {
                name: format!("v{i}"),
                ..Row::default()
            })
            .collect();
        assert_eq!(read(&many).list, file["tooMany"].as_str());
        assert!(read(&many).blocked());
        assert!(checked >= 30, "{checked}");
    }

    /// The built-in values beside the project's (docs/adr/0214 §2.3): `crate::variables`.
    #[test]
    fn the_built_in_values_are_the_reference_s() {
        use kentos_contracts::ProjectSettings;
        use kentos_expression::Value as V;
        let file: Value = serde_json::from_str(FIXTURE).expect("the fixture reads");
        for case in file["builtins"].as_array().expect("builtins") {
            let s = &case["settings"];
            let mut settings: ProjectSettings = serde_json::from_value(serde_json::json!({
                "srid": s["srid"],
                "lengthDecimals": 3,
                "areaDecimals": 2,
                "areaUnit": "m2",
                "angleUnit": "grad",
                "plotScale": s["plotScale"],
            }))
            .expect("settings");
            settings.variables = variables_of(&s["variables"]);
            // A definition of the project's own: the registry's TM30 under another name.
            if let Some(custom) = s.get("customCrs") {
                let base = crate::crs::system(5254)
                    .and_then(|r| r.transform_system())
                    .expect("TUREF / TM30");
                settings.custom_crs =
                    crate::systems::definition_from(custom["name"].as_str().unwrap_or(""), &base);
                assert!(settings.custom_crs.is_some());
            }
            let now = case["now"].as_str().expect("now");
            let wall = kentos_geometry_core::time::read(now)
                .moment()
                .expect("a moment");
            let got = crate::variables::expression_variables(
                case["name"].as_str().unwrap_or(""),
                &settings,
                wall,
                case["user"].as_str().unwrap_or(""),
            );
            let want = case["variables"].as_array().expect("variables");
            assert_eq!(got.len(), want.len(), "{}", case["case"]);
            for (g, w) in got.iter().zip(want) {
                assert_eq!(g.name, w["name"].as_str().unwrap_or(""), "{}", case["case"]);
                let value = match &w["value"] {
                    Value::Null => V::Null,
                    Value::Bool(b) => V::Bool(*b),
                    Value::Number(n) => V::Num(n.as_f64().unwrap_or(f64::NAN)),
                    Value::String(t) => V::text(t.clone()),
                    _ => V::Null,
                };
                assert_eq!(g.value, value, "{} @{}", case["case"], g.name);
                assert_eq!(
                    g.description,
                    w["description"].as_str().unwrap_or(""),
                    "{}",
                    case["case"]
                );
            }
        }
    }
}
