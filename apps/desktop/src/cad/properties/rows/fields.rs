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
    use kentos_commands::{LayerFieldKind, MULTIPLE_JOIN, check_value, display_value};
    let mut rows = Vec::new();
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
        rows.push(Row {
            label: Cow::Owned(if f.required {
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
        });
        if let Err(refused) = checked {
            rows.push(Row {
                warn: true,
                ..Row::note(&refused.message)
            });
        }
    }
    rows
}
