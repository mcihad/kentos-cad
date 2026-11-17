//! A layer's form and fields in the panel (docs/adr/0199 §5, 0401 §9).

use super::*;

/// The Form row of a layer that has a form (docs/adr/0401 §9): the form's name and whether it is
/// published, with Formla düzenle and Formlar….
pub(super) fn form_rows(settings: &kentos_commands::ProjectSettings, layer: &str) -> Vec<Row> {
    let forms: Vec<&kentos_commands::ProjectForm> =
        settings.forms.iter().filter(|f| f.layer == layer).collect();
    let Some(form) = forms
        .iter()
        .find(|f| !f.releases.is_empty())
        .or(forms.first())
    else {
        return Vec::new();
    };
    let value = if form.releases.is_empty() {
        format!("{} (taslak)", form.name)
    } else {
        form.name.clone()
    };
    let items = vec![
        Choice::Command {
            label: "Formla düzenle",
            icon: Icon::Properties,
            id: "data.formEdit",
        },
        Choice::Command {
            label: "Formlar…",
            icon: Icon::Layout,
            id: "data.forms",
        },
    ];
    vec![
        Row::text("Form", value.clone()).editor(Some(Editor::Select {
            text: value,
            swatch: None,
            icon: None,
            items,
        })),
    ]
}

/// Öznitelikler's form of a layer's fields (docs/adr/0199 §5; the web's
/// `ui/properties/fieldRows.ts`): a row per field, in order, its alias and a
/// mark when it is required; a value list and yes or no in a list, anything
/// else in a cell showing the value as the field displays it; a value the
/// field refuses as written, why under it in the warning colour. Written
/// through `cad.entities.set` (“Değiştir”), which checks the value by the
/// same rules.
pub(super) fn field_rows(
    slot: Slot,
    base: &kentos_commands::EntityBase,
    fields: &[kentos_commands::LayerField],
    subtypes: Option<&kentos_commands::LayerSubtypes>,
    groups: &[kentos_commands::FieldGroup],
    locked: bool,
    labels: &[(String, std::collections::BTreeMap<String, String>)],
) -> Vec<Row> {
    use kentos_commands::{
        LayerFieldKind, LayerSubtypes, MULTIPLE_JOIN, check_value, display_value,
    };
    let mut rows = Vec::new();
    // The record's subtype (docs/adr/0422): a field it makes required is marked and, empty, said as one. The panel
    // read the layer's rule only: Vitrin's Okul made Kapasite required, the row showed neither the mark nor the
    // words while a write refused emptying it (real window, 15 November 2026).
    let subtype = subtypes.and_then(|s| s.of(fields, |n| base.attrs.get(n).cloned()));
    let subtype_words = subtypes.zip(subtype).map(|(s, t)| s.label_of(fields, t));
    for f in fields {
        let raw = base.attrs.get(&f.name).cloned().unwrap_or_default();
        let checked = check_value(f, &raw);
        // A form's list from another layer (docs/adr/0404): shown by its labels, written through the form.
        let lookup = labels
            .iter()
            .find(|(name, _)| *name == f.name)
            .map(|(_, l)| l);
        let (value, shown) = match (&checked, lookup) {
            (Ok(v), Some(codes)) => (
                v.clone(),
                v.split(MULTIPLE_JOIN)
                    .filter(|c| !c.is_empty())
                    .map(|c| codes.get(c).map_or(c, String::as_str))
                    .collect::<Vec<_>>()
                    .join(MULTIPLE_JOIN),
            ),
            (Ok(v), None) => (v.clone(), display_value(f, v)),
            (Err(_), _) => (raw.clone(), raw.clone()),
        };
        let field = Field::Attribute(slot, f.name.clone());
        let listed =
            f.kind == LayerFieldKind::Boolean || f.values.as_ref().is_some_and(|v| !v.is_empty());
        // A value the bound table always makes itself is shown, not written (docs/adr/0754 §3).
        let editor = if locked || lookup.is_some() || f.made_always() {
            None
        } else if listed {
            let pairs: Vec<(String, String)> = match &f.values {
                // A retired value only for the record holding it (docs/adr/0418), narrowed to the
                // record's subtype (docs/adr/0422).
                Some(list) if !list.is_empty() => {
                    kentos_commands::offered_on(subtypes, groups, fields, f, |n| {
                        base.attrs.get(n).cloned()
                    })
                    .into_iter()
                    .map(|c| (c.code, c.label))
                    .collect()
                }
                _ => vec![
                    ("true".to_owned(), "Evet".to_owned()),
                    ("false".to_owned(), "Hayır".to_owned()),
                ],
            };
            let pick = |label: String, code: String, chosen: bool| Choice::Pick {
                label,
                swatch: None,
                icon: None,
                wide: false,
                chosen,
                enabled: true,
                message: Message::Properties(Event::Commit(field.clone(), code)),
            };
            let mut items = vec![pick("—".to_owned(), String::new(), value.is_empty())];
            items.extend(
                pairs
                    .into_iter()
                    .map(|(code, label)| pick(label, code.clone(), code == value)),
            );
            Some(Editor::Select {
                text: if shown.is_empty() {
                    "—".to_owned()
                } else {
                    shown.clone()
                },
                swatch: None,
                icon: None,
                items,
            })
        } else {
            Some(Editor::Text(field))
        };
        let required =
            f.required || subtype.is_some_and(|t| LayerSubtypes::field_for(f, Some(t)).required);
        rows.push(Row {
            label: Cow::Owned(if required {
                format!("{} *", f.label())
            } else {
                f.label().to_owned()
            }),
            value: shown,
            numeric: f.kind.is_number() && !listed,
            unit: None,
            editor,
            note: false,
            warn: false,
            thumb: None,
            source: None,
        });
        if let Err(refused) = checked {
            rows.push(Row {
                warn: true,
                ..Row::note(&refused.message)
            });
        } else if required && !f.required && raw.trim().is_empty() {
            // The words a write refusing it says (core/checks.rs).
            rows.push(Row {
                warn: true,
                ..Row::note(&format!(
                    "“{}” alanı “{}” alt türünde zorunlu; boş bırakılamaz.",
                    f.label(),
                    subtype_words.unwrap_or_default()
                ))
            });
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_commands::{EntityBase, LayerField, LayerFieldKind, LayerSubtypes};
    use serde_json::json;

    /// docs/adr/0422: a field the record's subtype makes required is marked and, empty, said as one, in the words a
    /// write refusing it says; another subtype's record keeps the layer's rule (real window, Vitrin 1.0.16's Okul and
    /// Kapasite).
    #[test]
    fn a_field_its_records_subtype_makes_required_is_marked_and_said_when_empty() {
        let mut tur = LayerField::new("tur", LayerFieldKind::Text);
        tur.values = serde_json::from_value(
            json!([{ "code": "OKUL", "label": "Okul" }, { "code": "PARK", "label": "Park" }]),
        )
        .unwrap();
        let mut kapasite = LayerField::new("kapasite", LayerFieldKind::Integer);
        kapasite.alias = Some("Kapasite".into());
        let fields = vec![tur, kapasite];
        let subtypes: LayerSubtypes = serde_json::from_value(json!({ "field": "tur", "types": [
            { "code": "OKUL", "fields": [{ "field": "kapasite", "required": true }] },
            { "code": "PARK" }
        ]}))
        .unwrap();
        let rows = |attrs: serde_json::Value| -> Vec<(String, bool)> {
            let base: EntityBase =
                serde_json::from_value(json!({ "id": 1, "layerId": "tesis", "attrs": attrs }))
                    .unwrap();
            field_rows(Slot(0), &base, &fields, Some(&subtypes), &[], false, &[])
                .into_iter()
                .map(|r| {
                    (
                        if r.warn {
                            r.value
                        } else {
                            r.label.into_owned()
                        },
                        r.warn,
                    )
                })
                .collect()
        };
        assert_eq!(
            rows(json!({ "tur": "OKUL" })),
            [
                ("tur".to_owned(), false),
                ("Kapasite *".to_owned(), false),
                (
                    "“Kapasite” alanı “Okul” alt türünde zorunlu; boş bırakılamaz.".to_owned(),
                    true
                ),
            ]
        );
        assert_eq!(
            rows(json!({ "tur": "OKUL", "kapasite": "300" }))[1],
            ("Kapasite *".to_owned(), false),
            "given: marked, not said"
        );
        assert_eq!(
            rows(json!({ "tur": "PARK" })),
            [("tur".to_owned(), false), ("Kapasite".to_owned(), false)],
            "another subtype: the layer's rule"
        );
    }
}
