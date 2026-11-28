//! Künyenin kişisel bilgileri (docs/adr/8100; plan/bim-plan.md BIM-15; 6698 sayılı Kanun md. 3/1-d): the permit set's
//! künye variables that name or identify people — the building's owner and its land registry record, the contractor, the
//! architect and the office, those who designed, drew and checked it, the supervisors — wherever a project keeps `@`
//! variables (its settings, its sheet book and each sheet), and the name of whoever delivered a package
//! (`settings.arch.deliveries[].by`). A cloud project gives them only to whoever works on its objects; anyone else reads
//! them cut: a text value as [`CUT_PARTICULAR`], any other value as none.
//!
//! Only variables are cut: a name typed onto a sheet as plain text is the sheet's content, not a particular.

use serde_json::Value;

use crate::{ProjectSettings, ProjectSheets, VariableKind, VariableValue};

/// The künye's variables that name or identify people, by the names the permit set gives them
/// (`kentos_sheet::permit::PARTICULARS`).
pub const PERSONAL_PARTICULARS: [&str; 14] = [
    "yapi_sahibi",
    "tapu_kaydi",
    "muteahhit",
    "mimar",
    "mimar_soyadi",
    "sicil_no",
    "btb_no",
    "buro_adi",
    "buro_adres",
    "vergi_no",
    "tasarlayan",
    "cizen",
    "kontrol_eden",
    "denetciler",
];

/// What a cut text particular reads.
pub const CUT_PARTICULAR: &str = "Gizli";

/// Whether `name` is a personal particular.
pub fn is_personal(name: &str) -> bool {
    PERSONAL_PARTICULARS.contains(&name)
}

/// Whether a value says something: not none, not what a cut leaves.
fn says(value: &VariableValue) -> bool {
    !value.is_null() && *value != VariableValue::Text(CUT_PARTICULAR.to_owned())
}

/// The same of a sheet variable's value.
fn says_json(value: Option<&Value>) -> bool {
    value.is_some_and(|x| !x.is_null() && x.as_str() != Some(CUT_PARTICULAR))
}

impl ProjectSettings {
    /// Whether it carries a personal particular: a value of one of [`PERSONAL_PARTICULARS`], or who delivered a
    /// package.
    pub fn has_particulars(&self) -> bool {
        self.variables
            .iter()
            .any(|v| is_personal(&v.name) && says(&v.value))
            || self
                .arch
                .as_ref()
                .is_some_and(|a| a.deliveries.iter().any(|d| d.by.is_some()))
    }

    /// Its personal particulars cut; how many were.
    pub fn cut_particulars(&mut self) -> usize {
        let mut cut = 0;
        for v in self
            .variables
            .iter_mut()
            .filter(|v| is_personal(&v.name) && says(&v.value))
        {
            v.value = match v.kind {
                VariableKind::Text => VariableValue::Text(CUT_PARTICULAR.to_owned()),
                _ => VariableValue::Null,
            };
            cut += 1;
        }
        if let Some(arch) = self.arch.as_mut() {
            for d in arch.deliveries.iter_mut().filter(|d| d.by.is_some()) {
                d.by = None;
                cut += 1;
            }
        }
        cut
    }
}

/// A list of sheet variables (`[{name, kind, value}]`) with its personal particulars cut; how many were.
fn cut_list(list: Option<&mut Value>) -> usize {
    let Some(Value::Array(list)) = list else {
        return 0;
    };
    let mut cut = 0;
    for v in list.iter_mut() {
        let personal = v
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(is_personal);
        if !personal || !says_json(v.get("value")) {
            continue;
        }
        let text = v
            .get("kind")
            .and_then(Value::as_str)
            .is_none_or(|k| k == "text");
        v["value"] = if text {
            Value::String(CUT_PARTICULAR.to_owned())
        } else {
            Value::Null
        };
        cut += 1;
    }
    cut
}

/// Whether a list of sheet variables carries a personal particular.
fn list_has(list: Option<&Value>) -> bool {
    list.and_then(Value::as_array).is_some_and(|list| {
        list.iter().any(|v| {
            v.get("name")
                .and_then(Value::as_str)
                .is_some_and(is_personal)
                && says_json(v.get("value"))
        })
    })
}

impl ProjectSheets {
    /// Whether the book or one of its sheets carries a personal particular.
    pub fn has_particulars(&self) -> bool {
        list_has(self.book.get("variables"))
            || self
                .book
                .get("sheets")
                .and_then(Value::as_array)
                .is_some_and(|s| s.iter().any(|sheet| list_has(sheet.get("variables"))))
    }

    /// The book's and its sheets' personal particulars cut; how many were.
    pub fn cut_particulars(&mut self) -> usize {
        let mut cut = cut_list(self.book.get_mut("variables"));
        if let Some(Value::Array(sheets)) = self.book.get_mut("sheets") {
            for sheet in sheets {
                cut += cut_list(sheet.get_mut("variables"));
            }
        }
        cut
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{ArchSettings, Delivery, DeliveryPackage, ProjectVariable};

    fn var(name: &str, kind: VariableKind, value: VariableValue) -> ProjectVariable {
        ProjectVariable {
            name: name.into(),
            label: String::new(),
            kind,
            value,
        }
    }

    #[test]
    fn the_kunye_s_personal_particulars_are_cut_and_the_rest_kept() {
        let mut s: ProjectSettings =
            serde_json::from_value(json!({"srid": 5256, "lengthDecimals": 3,
            "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 100.0}))
            .expect("settings");
        s = ProjectSettings {
            variables: vec![
                var(
                    "proje_no",
                    VariableKind::Text,
                    VariableValue::Text("2026/14".into()),
                ),
                var(
                    "yapi_sahibi",
                    VariableKind::Text,
                    VariableValue::Text("Ayşe Yılmaz".into()),
                ),
                var(
                    "sicil_no",
                    VariableKind::Number,
                    VariableValue::Number(12345.0),
                ),
                var("mimar", VariableKind::Text, VariableValue::Null),
                var("ada", VariableKind::Text, VariableValue::Text("118".into())),
            ],
            arch: Some(ArchSettings {
                deliveries: vec![Delivery {
                    id: "0192f5a1-8099-7000-8000-000000000001".into(),
                    number: 1,
                    revision: "000".into(),
                    package: DeliveryPackage {
                        manifest: "a".repeat(64),
                        files: 1,
                        path: None,
                        source: None,
                    },
                    to: "Belediye".into(),
                    channel: Default::default(),
                    at: "2026-11-02".into(),
                    by: Some("Mimar A".into()),
                    receipt: None,
                    response: None,
                }],
                ..ArchSettings::default()
            }),
            ..s
        };
        assert!(s.has_particulars());
        assert_eq!(
            s.cut_particulars(),
            3,
            "the owner, the registration number and who delivered"
        );
        let values: Vec<(&str, &VariableValue)> = s
            .variables
            .iter()
            .map(|v| (v.name.as_str(), &v.value))
            .collect();
        assert_eq!(
            values,
            [
                ("proje_no", &VariableValue::Text("2026/14".into())),
                ("yapi_sahibi", &VariableValue::Text(CUT_PARTICULAR.into())),
                ("sicil_no", &VariableValue::Null),
                ("mimar", &VariableValue::Null),
                ("ada", &VariableValue::Text("118".into())),
            ]
        );
        assert!(
            s.variables.iter().all(|v| v.problem().is_none()),
            "a cut value is a value of its kind"
        );
        assert_eq!(s.arch.as_ref().unwrap().deliveries[0].by, None);
        assert!(
            !s.has_particulars(),
            "a cut text reads Gizli, which is no one's"
        );
        let mut again = s.clone();
        assert_eq!(
            again.cut_particulars(),
            0,
            "a cut copy has nothing more to cut"
        );
        assert_eq!(again, s);
    }

    #[test]
    fn the_sheet_book_s_and_each_sheet_s_particulars_are_cut() {
        let mut sheets = ProjectSheets {
            book: json!({
                "variables": [{"name": "mimar", "value": "Ali Kaya"}, {"name": "proje_no", "value": "2026/14"}],
                "sheets": [
                    {"id": "a", "variables": [{"name": "vergi_no", "kind": "number", "value": 1234567890}]},
                    {"id": "b"}
                ]
            }),
            pictures: Vec::new(),
        };
        assert!(sheets.has_particulars());
        assert_eq!(sheets.cut_particulars(), 2);
        assert_eq!(sheets.book["variables"][0]["value"], CUT_PARTICULAR);
        assert_eq!(sheets.book["variables"][1]["value"], "2026/14");
        assert_eq!(
            sheets.book["sheets"][0]["variables"][0]["value"],
            Value::Null
        );
        assert!(!sheets.has_particulars());
        assert_eq!(sheets.cut_particulars(), 0);
    }
}
