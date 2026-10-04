//! Proje ayarları' Datum dönüşümleri (docs/adr/0168 §3, §6; the web's
//! `model/choiceForm.ts`): for each of the registry's three datum pairs,
//! what EPSG's way is, and what is typed turned into the project's choice
//! (`DatumTransform`) or, field by field, what is wrong. The shared cases
//! are fixtures/crs/v1/choice-form.json
//! (scripts/fixtures/crs_choice_form_cases.py, from the ADR's rules).

use std::collections::BTreeMap;

use kentos_contracts::{Convention, DatumTransform, GridChoice, Helmert, RegistryDatum};

/// The registry's datum pairs, in the order Datum dönüşümleri lists them.
pub const PAIRS: [(RegistryDatum, RegistryDatum); 3] = [
    (RegistryDatum::Ed50, RegistryDatum::Turef),
    (RegistryDatum::Ed50, RegistryDatum::Wgs84),
    (RegistryDatum::Turef, RegistryDatum::Wgs84),
];

/// EPSG's way between a pair, as the transforms say it (docs/adr/0167 §3).
pub fn epsg_text(pair: (RegistryDatum, RegistryDatum)) -> &'static str {
    match pair {
        (RegistryDatum::Ed50, RegistryDatum::Turef) => {
            "±2.1 m, EPSG:1783 + EPSG:5260; resmî dönüşüm değil"
        }
        (RegistryDatum::Ed50, RegistryDatum::Wgs84) => "±2 m, EPSG:1784; resmî dönüşüm değil",
        _ => "±1 m, EPSG:5261",
    }
}

/// A datum's name as Datum dönüşümleri writes it.
pub fn datum_name(d: RegistryDatum) -> &'static str {
    match d {
        RegistryDatum::Turef => "TUREF",
        RegistryDatum::Ed50 => "ED50",
        RegistryDatum::Wgs84 => "WGS 84",
    }
}

/// How a pair goes: EPSG's way, the project's seven parameters, or a grid of
/// the device's library.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Method {
    #[default]
    Epsg,
    Helmert,
    Grid,
}

/// The seven parameters' fields, in their order: the translations (m), the
/// rotations (″), the scale difference (ppm).
pub const PARAMETERS: [&str; 7] = ["tx", "ty", "tz", "rx", "ry", "rz", "ds"];

/// What Datum dönüşümleri holds for a pair, as typed.
#[derive(Clone, Debug, PartialEq)]
pub struct Form {
    pub method: Method,
    /// From the pair's second datum to its first.
    pub reversed: bool,
    pub name: String,
    /// `PARAMETERS`' texts.
    pub parameters: [String; 7],
    pub convention: Convention,
    /// The grid's SHA-256.
    pub grid: String,
    pub accuracy: String,
}

impl Default for Form {
    fn default() -> Self {
        Self {
            method: Method::Epsg,
            reversed: false,
            name: String::new(),
            parameters: Default::default(),
            convention: Convention::PositionVector,
            grid: String::new(),
            accuracy: String::new(),
        }
    }
}

/// A grid the form may name: one of the device's library, or the one the
/// project's choice names already.
#[derive(Clone, Debug, PartialEq)]
pub struct GridRef {
    pub id: String,
    pub file: String,
    pub size: u64,
}

/// What is wrong, by field: `name`, `tx` … `ds`, `grid`, `accuracy`.
pub type Problems = BTreeMap<&'static str, &'static str>;

pub const NAME: &str = "Adını yazın: değerlerin yanında dayanağı olarak görünür.";
pub const NUMBER: &str = "Sayı yazın, ör. -84.1.";
pub const ACCURACY: &str = "0 ya da büyük bir sayı yazın; bilinmiyorsa boş bırakın.";
pub const GRID: &str = "Bir ızgara seçin; listede yoksa Izgaralar'dan ekleyin.";

/// A number as the Hesap windows read one: trimmed, its first comma a
/// point, `^[-+]?(\d+(\.\d*)?|\.\d+)(e[-+]?\d+)?$`; none for anything else.
fn number(text: &str) -> Option<f64> {
    let t = text.trim().replacen(',', ".", 1);
    let b = t.as_bytes();
    let mut i = usize::from(matches!(b.first(), Some(b'-' | b'+')));
    let digits = |from: usize| b[from..].iter().take_while(|c| c.is_ascii_digit()).count();
    let whole = digits(i);
    i += whole;
    let mut fraction = 0;
    if b.get(i) == Some(&b'.') {
        fraction = digits(i + 1);
        i += 1 + fraction;
    }
    if whole == 0 && fraction == 0 {
        return None;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        i += 1;
        i += usize::from(matches!(b.get(i), Some(b'-' | b'+')));
        let exponent = digits(i);
        if exponent == 0 {
            return None;
        }
        i += exponent;
    }
    (i == b.len()).then(|| t.parse().ok()).flatten()
}

/// The choice a pair's form gives, none for EPSG's way; or what is wrong.
pub fn build(
    pair: (RegistryDatum, RegistryDatum),
    form: &Form,
    grids: &[GridRef],
) -> Result<Option<DatumTransform>, Problems> {
    if form.method == Method::Epsg {
        return Ok(None);
    }
    let (from, to) = if form.reversed {
        (pair.1, pair.0)
    } else {
        pair
    };
    let mut problems = Problems::new();
    let name = form.name.trim();
    if name.is_empty() {
        problems.insert("name", NAME);
    }
    let accuracy = match form.accuracy.trim() {
        "" => None,
        text => match number(text) {
            Some(a) if a >= 0.0 => Some(a),
            _ => {
                problems.insert("accuracy", ACCURACY);
                None
            }
        },
    };
    let mut choice = DatumTransform {
        from,
        to,
        name: name.to_owned(),
        helmert: None,
        grid: None,
    };
    match form.method {
        Method::Helmert => {
            let mut values = [0.0; 7];
            for (i, key) in PARAMETERS.iter().enumerate() {
                let text = form.parameters[i].trim();
                // An empty rotation or scale difference is 0: three parameters.
                if text.is_empty() && i >= 3 {
                    continue;
                }
                match number(text) {
                    Some(v) => values[i] = v,
                    None => {
                        problems.insert(key, NUMBER);
                    }
                }
            }
            choice.helmert = Some(Helmert {
                translation: [values[0], values[1], values[2]],
                rotation: [values[3], values[4], values[5]],
                scale: values[6],
                convention: form.convention,
                accuracy,
            });
        }
        Method::Grid => match grids.iter().find(|g| g.id == form.grid) {
            Some(g) => {
                choice.grid = Some(GridChoice {
                    id: g.id.clone(),
                    file: g.file.clone(),
                    size: g.size,
                    accuracy,
                });
            }
            None => {
                problems.insert("grid", GRID);
            }
        },
        Method::Epsg => {}
    }
    if problems.is_empty() {
        Ok(Some(choice))
    } else {
        Err(problems)
    }
}

/// The pair a choice is for, either way round.
fn same_pair(c: &DatumTransform, pair: (RegistryDatum, RegistryDatum)) -> bool {
    (c.from, c.to) == pair || (c.to, c.from) == pair
}

/// The form of a pair's choice among the project's; EPSG's way without one.
pub fn form_of(pair: (RegistryDatum, RegistryDatum), choices: &[DatumTransform]) -> Form {
    let Some(c) = choices.iter().find(|c| same_pair(c, pair)) else {
        return Form::default();
    };
    let text = |v: f64| format!("{v}");
    let mut form = Form {
        reversed: c.from == pair.1,
        name: c.name.clone(),
        ..Form::default()
    };
    match (&c.helmert, &c.grid) {
        (Some(h), _) => {
            let v = [
                h.translation[0],
                h.translation[1],
                h.translation[2],
                h.rotation[0],
                h.rotation[1],
                h.rotation[2],
                h.scale,
            ];
            form.method = Method::Helmert;
            form.parameters = v.map(text);
            form.convention = h.convention;
            form.accuracy = h.accuracy.map(text).unwrap_or_default();
        }
        (None, Some(g)) => {
            form.method = Method::Grid;
            form.grid = g.id.clone();
            form.accuracy = g.accuracy.map(text).unwrap_or_default();
        }
        (None, None) => {}
    }
    form
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn datum(v: &Value) -> RegistryDatum {
        serde_json::from_value(v.clone()).expect("a datum")
    }

    fn form(v: &Value) -> Form {
        let text = |v: &Value| v.as_str().expect("a text").to_owned();
        Form {
            method: match v["method"].as_str() {
                Some("helmert") => Method::Helmert,
                Some("grid") => Method::Grid,
                _ => Method::Epsg,
            },
            reversed: v["reversed"].as_bool().expect("reversed"),
            name: text(&v["name"]),
            parameters: PARAMETERS.map(|k| text(&v["parameters"][k])),
            convention: serde_json::from_value(v["convention"].clone()).expect("a convention"),
            grid: text(&v["grid"]),
            accuracy: text(&v["accuracy"]),
        }
    }

    /// What is typed turns into the choice, or the problems by field, as the
    /// shared cases say (the web reads the same file, `model/choiceForm.test.ts`);
    /// a choice's form gives it back.
    #[test]
    fn the_forms_are_the_shared_cases() {
        let file: Value =
            serde_json::from_str(include_str!("../../../../fixtures/crs/v1/choice-form.json"))
                .expect("the cases read");
        assert_eq!(file["format"], "kentos.crs-choice-form");
        for (key, text) in [
            ("name", NAME),
            ("number", NUMBER),
            ("accuracy", ACCURACY),
            ("grid", GRID),
        ] {
            assert_eq!(file["texts"][key], text, "{key}");
        }
        for (i, e) in file["epsg"].as_array().expect("epsg").iter().enumerate() {
            assert_eq!(PAIRS[i], (datum(&e["pair"][0]), datum(&e["pair"][1])));
            assert_eq!(epsg_text(PAIRS[i]), e["text"], "{i}");
        }
        let grids: Vec<GridRef> = file["library"]
            .as_array()
            .expect("library")
            .iter()
            .map(|g| GridRef {
                id: g["id"].as_str().expect("id").to_owned(),
                file: g["file"].as_str().expect("file").to_owned(),
                size: g["size"].as_u64().expect("size"),
            })
            .collect();
        let cases = file["cases"].as_array().expect("cases");
        assert!(cases.len() >= 8);
        for case in cases {
            let name = case["name"].as_str().expect("a name");
            let pair = (datum(&case["pair"][0]), datum(&case["pair"][1]));
            let typed = form(&case["form"]);
            let got = build(pair, &typed, &grids);
            match case.get("problems") {
                Some(p) => {
                    let want: Problems = p
                        .as_object()
                        .expect("problems")
                        .iter()
                        .map(|(k, v)| {
                            let key = [
                                "name", "tx", "ty", "tz", "rx", "ry", "rz", "ds", "grid",
                                "accuracy",
                            ]
                            .into_iter()
                            .find(|f| f == k)
                            .expect("a field");
                            let text = [NAME, NUMBER, ACCURACY, GRID]
                                .into_iter()
                                .find(|t| v == t)
                                .expect("a text");
                            (key, text)
                        })
                        .collect();
                    assert_eq!(got, Err(want), "{name}");
                }
                None => {
                    let want: Option<DatumTransform> =
                        serde_json::from_value(case["choice"].clone()).expect("a choice");
                    assert_eq!(got, Ok(want.clone()), "{name}");
                    // Its form gives it back.
                    if let Some(c) = want {
                        let again = build(pair, &form_of(pair, std::slice::from_ref(&c)), &grids);
                        assert_eq!(again, Ok(Some(c)), "{name}");
                    }
                }
            }
        }
    }
}
