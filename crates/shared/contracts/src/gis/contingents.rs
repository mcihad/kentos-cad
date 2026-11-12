//! A layer's value combinations (Değer bileşimleri, docs/adr/0423; ArcGIS's contingent values): a
//! group of listed fields whose values go together; its rows are the combinations allowed, each cell a
//! code, any value, or empty. A record that fills a group's fields must keep one of its rows (of the
//! record's subtype, when a row names one); a restrictive group refuses another, an advisory one warns.
//! The choices of a group's field are the codes some row allows with the record's other values.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::fields::{LayerField, js_trim};

/// A layer holds at most this many groups, a group this many fields and rows.
pub const MOST_GROUPS: usize = 50;
pub const MOST_GROUP_FIELDS: usize = 8;
pub const MOST_COMBINATIONS: usize = 10_000;

/// A group of fields whose values go together.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldGroup {
    /// The group's name, once in the layer.
    pub name: String,
    /// Its fields' keys, in its order: 2–8 single-valued listed fields.
    pub fields: Vec<String>,
    /// A combination none of its rows allows is refused (advisory: warned).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub restrictive: bool,
    /// The combinations allowed.
    pub rows: Vec<Combination>,
}

/// One combination: a cell for each field of its group, in the group's order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Combination {
    pub cells: Vec<Cell>,
    /// The subtype whose records it is for (docs/adr/0422); none: every record's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub subtype: Option<String>,
}

/// A cell of a combination: a code of its field's list, any value (empty too), or empty.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum Cell {
    Any,
    Empty,
    Code { code: String },
}

impl Cell {
    /// Whether the cell allows `value` (trimmed; empty: no value).
    pub fn allows(&self, value: &str) -> bool {
        match self {
            Cell::Any => true,
            Cell::Empty => value.is_empty(),
            Cell::Code { code } => code == value,
        }
    }

    /// What a message calls it.
    pub fn words(&self) -> String {
        match self {
            Cell::Any => "herhangi".into(),
            Cell::Empty => "boş".into(),
            Cell::Code { code } => code.clone(),
        }
    }
}

/// What a group says of a record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unmet<'g> {
    pub group: &'g FieldGroup,
    /// The record's values of the group's fields, in its order (empty: none).
    pub values: Vec<String>,
}

impl FieldGroup {
    /// The rows that apply to a record of `subtype`.
    fn rows_for<'a>(&'a self, subtype: Option<&'a str>) -> impl Iterator<Item = &'a Combination> {
        self.rows
            .iter()
            .filter(move |r| r.subtype.as_deref().is_none_or(|s| Some(s) == subtype))
    }

    /// The record's values of the group's fields, by `value_of` (a field's name), in its order.
    pub fn values_of(
        &self,
        fields: &[LayerField],
        value_of: &impl Fn(&str) -> Option<String>,
    ) -> Vec<String> {
        self.fields
            .iter()
            .map(|k| {
                fields
                    .iter()
                    .find(|f| f.key() == k)
                    .and_then(|f| value_of(&f.name))
                    .map(|v| js_trim(&v).to_owned())
                    .unwrap_or_default()
            })
            .collect()
    }

    /// Whether a record with `values` (in the group's order) keeps a row for `subtype`; a record
    /// that fills none of the group's fields is not asked.
    pub fn holds(&self, values: &[String], subtype: Option<&str>) -> bool {
        values.iter().all(String::is_empty)
            || self
                .rows_for(subtype)
                .any(|r| r.cells.iter().zip(values).all(|(c, v)| c.allows(v)))
    }

    /// The codes the field at `at` (in the group) may take with the record's other `values`: those a
    /// row for `subtype` allows there while it allows the other filled values; none when some such row
    /// allows any value there.
    pub fn offered(
        &self,
        at: usize,
        values: &[String],
        subtype: Option<&str>,
    ) -> Option<Vec<String>> {
        let mut out: Vec<String> = Vec::new();
        for r in self.rows_for(subtype) {
            let fits = r
                .cells
                .iter()
                .zip(values)
                .enumerate()
                .all(|(i, (c, v))| i == at || v.is_empty() || c.allows(v));
            if !fits {
                continue;
            }
            match r.cells.get(at) {
                Some(Cell::Any) => return None,
                Some(Cell::Code { code }) if !out.contains(code) => out.push(code.clone()),
                _ => {}
            }
        }
        Some(out)
    }
}

/// The groups a record with `value_of` does not keep, in their order.
pub fn unmet<'g>(
    groups: &'g [FieldGroup],
    fields: &[LayerField],
    subtype: Option<&str>,
    value_of: impl Fn(&str) -> Option<String>,
) -> Vec<Unmet<'g>> {
    groups
        .iter()
        .filter_map(|g| {
            let values = g.values_of(fields, &value_of);
            (!g.holds(&values, subtype)).then_some(Unmet { group: g, values })
        })
        .collect()
}

/// The words of a record's values in a group, by the lists' labels: “Tür = Sanayi, Malzeme = boş”.
pub fn combination_words(group: &FieldGroup, fields: &[LayerField], values: &[String]) -> String {
    group
        .fields
        .iter()
        .zip(values)
        .map(|(k, v)| {
            let field = fields.iter().find(|f| f.key() == k);
            let name = field.map_or(k.as_str(), |f| f.label());
            let v = if v.is_empty() {
                "boş"
            } else {
                field
                    .and_then(|f| f.values.as_ref())
                    .and_then(|l| l.iter().find(|c| c.code == *v))
                    .map_or(v.as_str(), |c| c.label.as_str())
            };
            format!("{name} = {v}")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn bad_name(name: &str) -> bool {
    name.is_empty()
        || name.chars().count() > 64
        || name.trim() != name
        || name.chars().any(char::is_control)
}

/// Why `groups` cannot be the groups of a layer with `fields` (docs/adr/0423 §1), with the path of
/// the first fault; `subtypes` gives the codes a row's subtype may name.
pub fn contingents_problem(
    fields: &[LayerField],
    subtypes: Option<&crate::subtypes::LayerSubtypes>,
    groups: &[FieldGroup],
) -> Option<(String, String)> {
    if groups.len() > MOST_GROUPS {
        return Some((
            String::new(),
            format!(
                "Bir katmanın en çok {MOST_GROUPS} değer grubu olur; {} var.",
                groups.len()
            ),
        ));
    }
    for (i, g) in groups.iter().enumerate() {
        let at = i.to_string();
        if bad_name(&g.name) {
            return Some((
                format!("{at}/name"),
                format!(
                    "“{}” bir grup adı olamaz: 1–64 karakter, başında ve sonunda boşluk olmadan.",
                    g.name
                ),
            ));
        }
        if groups[..i].iter().any(|o| o.name == g.name) {
            return Some((
                format!("{at}/name"),
                format!("“{}” grubu iki kez var.", g.name),
            ));
        }
        if g.fields.len() < 2 || g.fields.len() > MOST_GROUP_FIELDS {
            return Some((
                format!("{at}/fields"),
                format!(
                    "“{}” grubunda 2–{MOST_GROUP_FIELDS} alan olur; {} var.",
                    g.name,
                    g.fields.len()
                ),
            ));
        }
        for (j, k) in g.fields.iter().enumerate() {
            let Some(f) = fields.iter().find(|f| f.key() == k) else {
                return Some((
                    format!("{at}/fields/{j}"),
                    format!("Katmanda “{k}” alanı yok."),
                ));
            };
            if f.values.as_ref().is_none_or(Vec::is_empty) || f.holds_many() {
                return Some((
                    format!("{at}/fields/{j}"),
                    format!(
                        "“{}” alanı gruba giremez: değer listeli, tek değerli bir alan olmalı.",
                        f.label()
                    ),
                ));
            }
            if g.fields[..j].contains(k) {
                return Some((
                    format!("{at}/fields/{j}"),
                    format!("“{}” alanı “{}” grubunda iki kez var.", f.label(), g.name),
                ));
            }
        }
        if g.rows.is_empty() {
            return Some((
                format!("{at}/rows"),
                format!(
                    "“{}” grubunda bileşim yok; en az bir bileşim verin.",
                    g.name
                ),
            ));
        }
        if g.rows.len() > MOST_COMBINATIONS {
            return Some((
                format!("{at}/rows"),
                format!(
                    "Bir grupta en çok {MOST_COMBINATIONS} bileşim olur; {} var.",
                    g.rows.len()
                ),
            ));
        }
        for (r, row) in g.rows.iter().enumerate() {
            let at = format!("{at}/rows/{r}");
            if row.cells.len() != g.fields.len() {
                return Some((
                    format!("{at}/cells"),
                    format!(
                        "{}. bileşimde {} hücre var; grubun {} alanı var.",
                        r + 1,
                        row.cells.len(),
                        g.fields.len()
                    ),
                ));
            }
            for (c, (cell, k)) in row.cells.iter().zip(&g.fields).enumerate() {
                if let Cell::Code { code } = cell {
                    let f = fields.iter().find(|f| f.key() == k).expect("checked above");
                    if !f.values.iter().flatten().any(|v| v.code == *code) {
                        return Some((
                            format!("{at}/cells/{c}"),
                            format!("“{code}”, “{}” alanının listesinde yok.", f.label()),
                        ));
                    }
                }
            }
            if let Some(s) = &row.subtype
                && subtypes.is_none_or(|t| t.get(s).is_none())
            {
                return Some((
                    format!("{at}/subtype"),
                    format!(
                        "{}. bileşimin alt türü “{s}” katmanın alt türlerinden değil.",
                        r + 1
                    ),
                ));
            }
            if g.rows[..r].contains(row) {
                return Some((at, format!("{}. bileşim daha önce var.", r + 1)));
            }
        }
    }
    None
}

/// `groups` after the layer's fields became `fields` (docs/adr/0423 §1): a group losing a field, or
/// whose field is no longer listed, goes; a row naming a code gone from its list goes, and a group
/// left without rows; a row whose subtype is gone goes.
pub fn follow_contingents(
    groups: &[FieldGroup],
    fields: &[LayerField],
    subtypes: Option<&crate::subtypes::LayerSubtypes>,
) -> Vec<FieldGroup> {
    groups
        .iter()
        .filter(|g| {
            g.fields.iter().all(|k| {
                fields.iter().any(|f| {
                    f.key() == k
                        && f.values.as_ref().is_some_and(|v| !v.is_empty())
                        && !f.holds_many()
                })
            })
        })
        .map(|g| {
            let rows = g
                .rows
                .iter()
                .filter(|r| {
                    r.subtype
                        .as_ref()
                        .is_none_or(|s| subtypes.is_some_and(|t| t.get(s).is_some()))
                        && r.cells.iter().zip(&g.fields).all(|(c, k)| match c {
                            Cell::Code { code } => {
                                fields.iter().find(|f| f.key() == k).is_some_and(|f| {
                                    f.values.iter().flatten().any(|v| v.code == *code)
                                })
                            }
                            _ => true,
                        })
                })
                .cloned()
                .collect();
            FieldGroup { rows, ..g.clone() }
        })
        .filter(|g| !g.rows.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fields::{FieldChoice, LayerFieldKind};

    fn listed(name: &str, codes: &[&str]) -> LayerField {
        LayerField {
            values: Some(codes.iter().map(|c| FieldChoice::new(*c, *c)).collect()),
            ..LayerField::new(name, LayerFieldKind::Text)
        }
    }

    #[test]
    fn a_group_allows_its_rows_and_offers_their_codes() {
        let fields = vec![
            listed("Tür", &["konut", "sanayi"]),
            listed("Malzeme", &["ahsap", "beton", "celik"]),
        ];
        let g = FieldGroup {
            name: "Yapı".into(),
            fields: vec!["Tür".into(), "Malzeme".into()],
            restrictive: true,
            rows: vec![
                Combination {
                    cells: vec![
                        Cell::Code {
                            code: "konut".into(),
                        },
                        Cell::Any,
                    ],
                    subtype: None,
                },
                Combination {
                    cells: vec![
                        Cell::Code {
                            code: "sanayi".into(),
                        },
                        Cell::Code {
                            code: "beton".into(),
                        },
                    ],
                    subtype: None,
                },
                Combination {
                    cells: vec![
                        Cell::Code {
                            code: "sanayi".into(),
                        },
                        Cell::Code {
                            code: "celik".into(),
                        },
                    ],
                    subtype: None,
                },
            ],
        };
        assert_eq!(
            contingents_problem(&fields, None, std::slice::from_ref(&g)),
            None
        );
        let v = |a: &str, b: &str| vec![a.to_owned(), b.to_owned()];
        assert!(g.holds(&v("konut", "ahsap"), None));
        assert!(!g.holds(&v("sanayi", "ahsap"), None));
        assert!(g.holds(&v("", ""), None));
        assert_eq!(
            g.offered(1, &v("sanayi", ""), None),
            Some(vec!["beton".into(), "celik".into()])
        );
        assert_eq!(g.offered(1, &v("konut", ""), None), None, "any value there");
        assert_eq!(
            g.offered(0, &v("", "celik"), None),
            Some(vec!["konut".into(), "sanayi".into()])
        );
        let found = unmet(std::slice::from_ref(&g), &fields, None, |n| {
            Some(if n == "Tür" { "sanayi" } else { "ahsap" }.to_owned())
        });
        assert_eq!(
            combination_words(&g, &fields, &found[0].values),
            "Tür = sanayi, Malzeme = ahsap",
            "labels as codes here"
        );
        let mut fewer = fields.clone();
        fewer[1]
            .values
            .as_mut()
            .unwrap()
            .retain(|c| c.code != "celik");
        assert_eq!(
            follow_contingents(std::slice::from_ref(&g), &fewer, None)[0]
                .rows
                .len(),
            2
        );
    }
}
