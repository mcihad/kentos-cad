//! A layer's subtypes (docs/adr/0422, Alt türler): one field with a value list, the subtype field, sorts
//! the records; each subtype is one of its codes and narrows the other fields for the records of that
//! kind: the values its list offers, the value a new record starts with, whether a value is required.
//! A field as a record of a subtype sees it is [`LayerSubtypes::field_for`]: every rule that reads a
//! field (the value's check, the choices offered, the forms) reads that one.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::fields::{LayerField, check_value};

/// A layer holds at most this many subtypes.
pub const MOST_SUBTYPES: usize = 256;

/// A layer's subtypes: the field that sorts the records (its key) and the subtypes, in their order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerSubtypes {
    /// The subtype field's key (its id, else its name): a field with a value list, one value each.
    pub field: String,
    pub types: Vec<Subtype>,
}

/// One subtype: a code of the subtype field's list, its label, and what it narrows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Subtype {
    pub code: String,
    /// What it is called; empty: the subtype field's label of its code.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<String>", optional))]
    pub label: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SubtypeField>>", optional))]
    pub fields: Vec<SubtypeField>,
}

/// What a subtype changes on one field; a field it does not name keeps the layer's rules.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SubtypeField {
    /// The field's key.
    pub field: String,
    /// The codes of its value list this subtype offers (a field with a value list); none: all of them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub values: Option<Vec<String>>,
    /// The value a new record of this subtype starts with, in its field's canonical text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub default: Option<String>,
    /// A record of this subtype has a value (in addition to the field's own rule).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub required: bool,
}

impl LayerSubtypes {
    /// The subtype field among `fields`.
    pub fn sorter<'a>(&self, fields: &'a [LayerField]) -> Option<&'a LayerField> {
        fields.iter().find(|f| f.key() == self.field)
    }

    /// The subtype a record with `attrs` (by the fields' names) is of; none when its subtype field
    /// holds no subtype's code.
    pub fn of<'a, V: AsRef<str>>(
        &'a self,
        fields: &[LayerField],
        value_of: impl Fn(&str) -> Option<V>,
    ) -> Option<&'a Subtype> {
        let sorter = self.sorter(fields)?;
        let value = value_of(&sorter.name)?;
        let value = crate::fields::js_trim(value.as_ref());
        self.types.iter().find(|t| t.code == value)
    }

    /// The subtype whose code is `code`.
    pub fn get(&self, code: &str) -> Option<&Subtype> {
        self.types.iter().find(|t| t.code == code)
    }

    /// `field` as a record of `subtype` sees it: its list narrowed to the subtype's values (a retired
    /// value stays, retired, so that a record holding it keeps it), its default the subtype's, required
    /// when either says so.
    pub fn field_for(field: &LayerField, subtype: Option<&Subtype>) -> LayerField {
        let Some(rule) = subtype.and_then(|t| t.fields.iter().find(|r| r.field == field.key()))
        else {
            return field.clone();
        };
        let mut out = field.clone();
        if let (Some(keep), Some(values)) = (&rule.values, out.values.as_mut()) {
            for c in values.iter_mut() {
                if !keep.contains(&c.code) {
                    c.retired = true;
                }
            }
        }
        if rule.default.is_some() {
            out.default.clone_from(&rule.default);
        }
        out.required |= rule.required;
        out
    }

    /// The subtype's label: its own, else the subtype field's label of its code.
    pub fn label_of<'a>(&'a self, fields: &'a [LayerField], subtype: &'a Subtype) -> &'a str {
        if !subtype.label.is_empty() {
            return &subtype.label;
        }
        self.sorter(fields)
            .and_then(|f| f.values.as_ref())
            .and_then(|v| v.iter().find(|c| c.code == subtype.code))
            .map_or(subtype.code.as_str(), |c| c.label.as_str())
    }
}

/// The values a choice offers `field` on a record whose values `value_of` gives (docs/adr/0422 §2): the
/// field's offered values (docs/adr/0418) narrowed to its subtype's; a value the record holds stays,
/// labelled when its subtype does not offer it.
pub fn offered_on(
    subtypes: Option<&LayerSubtypes>,
    groups: &[crate::contingents::FieldGroup],
    fields: &[LayerField],
    field: &LayerField,
    value_of: impl Fn(&str) -> Option<String>,
) -> Vec<crate::fields::FieldChoice> {
    let value = value_of(&field.name).unwrap_or_default();
    let subtype = subtypes.and_then(|s| s.of(fields, &value_of));
    // A restrictive value group narrows it to what its rows allow with the other values (docs/adr/0423).
    let mut grouped: Option<Vec<String>> = None;
    for g in groups.iter().filter(|g| g.restrictive) {
        let Some(at) = g.fields.iter().position(|k| k == field.key()) else {
            continue;
        };
        let values = g.values_of(fields, &value_of);
        if let Some(codes) = g.offered(at, &values, subtype.map(|t| t.code.as_str())) {
            grouped = Some(match grouped {
                None => codes,
                Some(now) => now.into_iter().filter(|c| codes.contains(c)).collect(),
            });
        }
    }
    let held: Vec<&str> = if field.holds_many() {
        field.codes_of(&value)
    } else {
        vec![crate::fields::js_trim(&value)]
    };
    field
        .offered_values(&value)
        .into_iter()
        .filter_map(|mut c| {
            let in_group = grouped.as_ref().is_none_or(|g| g.contains(&c.code));
            if subtype_offers(field, subtype, &c.code) && in_group {
                return Some(c);
            }
            if !held.contains(&c.code.as_str()) {
                return None;
            }
            if !c.retired {
                let why = if in_group {
                    "alt türde yok"
                } else {
                    "bileşimde yok"
                };
                c.label = format!("{} ({why})", c.label);
            }
            Some(c)
        })
        .collect()
}

/// Whether `code` of `field` is offered to a record of `subtype`: a code of the subtype's values, or
/// any when the subtype does not narrow the field.
pub fn subtype_offers(field: &LayerField, subtype: Option<&Subtype>, code: &str) -> bool {
    let Some(rule) = subtype.and_then(|t| t.fields.iter().find(|r| r.field == field.key())) else {
        return true;
    };
    rule.values
        .as_ref()
        .is_none_or(|v| v.iter().any(|c| c == code))
}

/// A new record's empty fields filled (docs/adr/0199 §2, 0422 §2, 0752): the subtype field first by its own
/// default, then every other field by its subtype's default, else the field's own; a default the record's subtype
/// does not offer is left out. A field `skip` names is left to whoever gives it its default first (a form's own).
/// Every way a record is made fills the same: a drawing command, a form's new record, a record from a form.
pub fn new_record_defaults(
    fields: &[LayerField],
    subtypes: Option<&LayerSubtypes>,
    out: &mut std::collections::BTreeMap<String, String>,
    skip: impl Fn(&LayerField) -> bool,
) {
    let blank = |out: &std::collections::BTreeMap<String, String>, name: &str| {
        out.get(name)
            .is_none_or(|v| crate::fields::js_trim(v).is_empty())
    };
    if let Some(sorter) = subtypes.and_then(|s| s.sorter(fields))
        && !skip(sorter)
        && let Some(d) = sorter.default.as_ref().filter(|d| !d.is_empty())
        && blank(out, &sorter.name)
    {
        out.insert(sorter.name.clone(), d.clone());
    }
    let subtype = subtypes.and_then(|s| s.of(fields, |n| out.get(n).cloned()));
    for f in fields {
        if skip(f) || !blank(out, &f.name) {
            continue;
        }
        let seen = LayerSubtypes::field_for(f, subtype);
        let offered = |d: &str| {
            seen.values.is_none()
                || (if f.holds_many() {
                    f.codes_of(d)
                } else {
                    vec![d]
                })
                .iter()
                .all(|c| subtype_offers(f, subtype, c))
        };
        if let Some(d) = seen.default.filter(|d| !d.is_empty() && offered(d)) {
            out.insert(f.name.clone(), d);
        }
    }
}

/// The values of a record of `subtype` its subtype does not allow (docs/adr/0422 §2): each field
/// (in the fields' order) whose value holds codes the subtype does not offer, with those codes.
pub fn subtype_misfits<'a, V: AsRef<str>>(
    fields: &'a [LayerField],
    subtype: Option<&Subtype>,
    value_of: impl Fn(&str) -> Option<V>,
) -> Vec<(&'a LayerField, Vec<String>)> {
    let Some(t) = subtype else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for f in fields {
        let Some(rule) = t.fields.iter().find(|r| r.field == f.key()) else {
            continue;
        };
        let Some(keep) = &rule.values else { continue };
        let Some(value) = value_of(&f.name) else {
            continue;
        };
        let value = value.as_ref();
        let codes: Vec<&str> = if f.holds_many() {
            f.codes_of(value)
        } else {
            let v = crate::fields::js_trim(value);
            if v.is_empty() { vec![] } else { vec![v] }
        };
        let bad: Vec<String> = codes
            .into_iter()
            .filter(|c| !keep.iter().any(|k| k == c))
            .map(str::to_owned)
            .collect();
        if !bad.is_empty() {
            out.push((f, bad));
        }
    }
    out
}

/// `subtypes` after the layer's fields became `fields` (docs/adr/0422 §1): a field gone takes its
/// subtype rules with it, a code gone from a list leaves the subtypes and their values, a default that
/// no longer fits is dropped; none when the subtype field is gone or no subtype is left.
pub fn follow_subtypes(subtypes: &LayerSubtypes, fields: &[LayerField]) -> Option<LayerSubtypes> {
    let sorter = subtypes.sorter(fields)?;
    let codes = sorter.values.as_ref().filter(|_| !sorter.holds_many())?;
    let mut out = LayerSubtypes {
        field: subtypes.field.clone(),
        types: Vec::new(),
    };
    for t in &subtypes.types {
        if !codes.iter().any(|c| c.code == t.code) || out.types.iter().any(|o| o.code == t.code) {
            continue;
        }
        let mut kept = Subtype {
            code: t.code.clone(),
            label: t.label.clone(),
            fields: Vec::new(),
        };
        for r in &t.fields {
            let Some(f) = fields.iter().find(|f| f.key() == r.field) else {
                continue;
            };
            if f.key() == sorter.key() || kept.fields.iter().any(|o| o.field == r.field) {
                continue;
            }
            let values = match (&r.values, &f.values) {
                (Some(keep), Some(list)) => {
                    let v: Vec<String> = keep
                        .iter()
                        .filter(|c| list.iter().any(|x| x.code == **c))
                        .cloned()
                        .collect();
                    (!v.is_empty()).then_some(v)
                }
                _ => None,
            };
            let mut rule = SubtypeField {
                field: r.field.clone(),
                values,
                default: r.default.clone(),
                required: r.required,
            };
            let probe = Subtype {
                fields: vec![rule.clone()],
                ..kept.clone()
            };
            let fits = |d: &str| {
                let seen = LayerSubtypes::field_for(f, Some(&probe));
                check_value(&seen, d).is_ok_and(|c| c == d)
                    && (f.values.is_none()
                        || (if f.holds_many() {
                            f.codes_of(d)
                        } else {
                            vec![d]
                        })
                        .iter()
                        .all(|c| subtype_offers(f, Some(&probe), c)))
            };
            if rule
                .default
                .as_deref()
                .is_some_and(|d| d.is_empty() || !fits(d))
            {
                rule.default = None;
            }
            if rule.values.is_some() || rule.default.is_some() || rule.required {
                kept.fields.push(rule);
            }
        }
        out.types.push(kept);
    }
    (!out.types.is_empty()).then_some(out)
}

fn bad_label(label: &str) -> bool {
    label.chars().count() > 64 || label.trim() != label || label.chars().any(char::is_control)
}

/// Why `subtypes` cannot be `fields`' (docs/adr/0422 §1), with the path of the first fault; none when
/// they can.
pub fn subtypes_problem(
    fields: &[LayerField],
    subtypes: &LayerSubtypes,
) -> Option<(String, String)> {
    let Some(sorter) = subtypes.sorter(fields) else {
        return Some((
            "field".into(),
            format!(
                "Katmanda “{}” alanı yok; alt tür alanı katmanın bir alanıdır.",
                subtypes.field
            ),
        ));
    };
    let Some(codes) = sorter.values.as_ref().filter(|_| !sorter.holds_many()) else {
        return Some((
            "field".into(),
            format!(
                "“{}” alanının değer listesi yok ya da çok değerli; alt tür alanı tek değerli bir liste alanıdır.",
                sorter.label()
            ),
        ));
    };
    if subtypes.types.is_empty() {
        return Some((
            "types".into(),
            "Alt türü olmayan alt tür tanımı olmaz; en az bir alt tür verin.".into(),
        ));
    }
    if subtypes.types.len() > MOST_SUBTYPES {
        return Some((
            "types".into(),
            format!(
                "Bir katmanın en çok {MOST_SUBTYPES} alt türü olur; {} var.",
                subtypes.types.len()
            ),
        ));
    }
    for (i, t) in subtypes.types.iter().enumerate() {
        let at = format!("types/{i}");
        if !codes.iter().any(|c| c.code == t.code) {
            return Some((
                format!("{at}/code"),
                format!(
                    "“{}”, “{}” alanının listesinde yok; alt tür listenin bir değeridir.",
                    t.code,
                    sorter.label()
                ),
            ));
        }
        if subtypes.types[..i].iter().any(|o| o.code == t.code) {
            return Some((
                format!("{at}/code"),
                format!("“{}” alt türü iki kez var.", t.code),
            ));
        }
        if bad_label(&t.label) {
            return Some((
                format!("{at}/label"),
                format!(
                    "“{}” bir alt tür adı olamaz: en çok 64 karakter, başında ve sonunda boşluk olmadan.",
                    t.label
                ),
            ));
        }
        for (j, r) in t.fields.iter().enumerate() {
            let at = format!("{at}/fields/{j}");
            let Some(f) = fields.iter().find(|f| f.key() == r.field) else {
                return Some((
                    format!("{at}/field"),
                    format!("Katmanda “{}” alanı yok.", r.field),
                ));
            };
            if f.key() == sorter.key() {
                return Some((
                    format!("{at}/field"),
                    format!(
                        "Alt tür, alt tür alanının (“{}”) kendisini değiştiremez.",
                        f.label()
                    ),
                ));
            }
            if t.fields[..j].iter().any(|o| o.field == r.field) {
                return Some((
                    format!("{at}/field"),
                    format!(
                        "“{}” alanı “{}” alt türünde iki kez var.",
                        f.label(),
                        t.code
                    ),
                ));
            }
            if let Some(keep) = &r.values {
                let Some(list) = &f.values else {
                    return Some((
                        format!("{at}/values"),
                        format!(
                            "“{}” alanının değer listesi yok; alt tür yalnız listeli alanın değerlerini daraltır.",
                            f.label()
                        ),
                    ));
                };
                if keep.is_empty() {
                    return Some((
                        format!("{at}/values"),
                        format!(
                            "“{}” alanına “{}” alt türünde hiçbir değer bırakılmamış; en az bir değer seçin.",
                            f.label(),
                            t.code
                        ),
                    ));
                }
                for (k, c) in keep.iter().enumerate() {
                    if !list.iter().any(|v| v.code == *c) {
                        return Some((
                            format!("{at}/values/{k}"),
                            format!("“{c}”, “{}” alanının listesinde yok.", f.label()),
                        ));
                    }
                    if keep[..k].contains(c) {
                        return Some((
                            format!("{at}/values/{k}"),
                            format!("“{c}” değeri iki kez var."),
                        ));
                    }
                }
            }
            if let Some(d) = &r.default {
                let seen = LayerSubtypes::field_for(f, Some(t));
                match check_value(&seen, d) {
                    Ok(c) if c == *d && !d.is_empty() => {}
                    Ok(_) => {
                        return Some((
                            format!("{at}/default"),
                            format!(
                                "“{}” alanının “{}” alt türündeki varsayılanı alanın yazımıyla verilmeli.",
                                f.label(),
                                t.code
                            ),
                        ));
                    }
                    Err(r) => {
                        return Some((
                            format!("{at}/default"),
                            format!(
                                "“{}” alanının “{}” alt türündeki varsayılanı: {}",
                                f.label(),
                                t.code,
                                r.message
                            ),
                        ));
                    }
                }
                let codes: Vec<&str> = if f.holds_many() {
                    f.codes_of(d)
                } else {
                    vec![d.as_str()]
                };
                if f.values.is_some() && codes.iter().any(|c| !subtype_offers(f, Some(t), c)) {
                    return Some((
                        format!("{at}/default"),
                        format!(
                            "“{}” alanının varsayılanı “{}” alt türünün değerlerinden değil.",
                            f.label(),
                            t.code
                        ),
                    ));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fields::{FieldChoice, LayerFieldKind};

    fn listed(name: &str, codes: &[&str]) -> LayerField {
        LayerField {
            values: Some(
                codes
                    .iter()
                    .map(|c| FieldChoice::new(*c, c.to_uppercase()))
                    .collect(),
            ),
            ..LayerField::new(name, LayerFieldKind::Text)
        }
    }

    #[test]
    fn a_subtype_narrows_a_field_and_its_rules_are_checked() {
        let fields = vec![
            listed("Tür", &["konut", "sanayi"]),
            listed("Malzeme", &["ahsap", "beton", "celik"]),
        ];
        let subtypes = LayerSubtypes {
            field: "Tür".into(),
            types: vec![Subtype {
                code: "sanayi".into(),
                label: String::new(),
                fields: vec![SubtypeField {
                    field: "Malzeme".into(),
                    values: Some(vec!["beton".into(), "celik".into()]),
                    default: Some("celik".into()),
                    required: true,
                }],
            }],
        };
        assert_eq!(subtypes_problem(&fields, &subtypes), None);
        let t = subtypes.of(&fields, |n| (n == "Tür").then_some("sanayi"));
        let seen = LayerSubtypes::field_for(&fields[1], t);
        assert!(seen.required && seen.default.as_deref() == Some("celik"));
        assert_eq!(seen.offered_values("").len(), 2);
        assert_eq!(
            seen.offered_values("ahsap").len(),
            3,
            "a held value stays offered"
        );
        let bad = subtype_misfits(&fields, t, |n| (n == "Malzeme").then_some("ahsap"));
        assert_eq!(bad[0].1, ["ahsap"]);
        assert_eq!(subtypes.label_of(&fields, &subtypes.types[0]), "SANAYI");
        let mut wrong = subtypes.clone();
        wrong.types[0].fields[0].default = Some("ahsap".into());
        assert_eq!(wrong.types[0].fields[0].field, "Malzeme");
        assert!(subtypes_problem(&fields, &wrong).is_some());
    }
}
