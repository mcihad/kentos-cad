//! A panel's relation components on the desktop (docs/adr/8057 §67): a relation picker's records and a relation
//! list's rows, read from the open drawing's model as a form's related box reads them (gis/forms/entry/related.rs):
//! no Python, no command, drawn from the drawing as it is, so a record the panel's code adds shows at once. A record's
//! words are its `display` field, else its label, else its first field with a value, as the form's box names one.

use iced::Element;
use kentos_extensions::ui_panel::Component;
use kentos_ui::widget::{Choice, Select};
use serde_json::{Map, Value, json};

use super::panels::{OpenPanel, PanelEvent};
use crate::app::{App, Message};

/// The most records a relation picker offers and a relation list shows.
const MOST: usize = 500;
/// The columns a relation list shows when it names none.
const FIRST_COLUMNS: usize = 4;

/// A project relation as a panel reads it: its parent and child layers, the child field holding the parent's
/// persistent id, and whether it holds several.
struct Tie {
    parent: String,
    child: String,
    field: String,
    many: bool,
}

impl App {
    /// Project relation `id` of the open drawing, or why the panel cannot read it.
    fn tie(&self, id: &str) -> Result<Tie, String> {
        let doc = self.document.as_ref().ok_or("Açık çizim yok.")?;
        let r = doc
            .model
            .settings()
            .relations
            .iter()
            .find(|r| r.id == id)
            .ok_or_else(|| format!("“{id}” ilişkisi bu projede yok."))?;
        let field = doc
            .model
            .layers()
            .get(&r.child)
            .and_then(|n| n.fields.iter().find(|f| f.key() == r.field))
            .ok_or_else(|| format!("“{id}” ilişkisinin alt katmanında bağ alanı yok."))?;
        Ok(Tie {
            parent: r.parent.clone(),
            child: r.child.clone(),
            field: field.name.clone(),
            many: field.multiple,
        })
    }

    /// A relation picker's choices: the relation's parent layer's records in the layer's order, each its persistent
    /// id and words; at most [`MOST`].
    pub(crate) fn relation_choices(
        &self,
        relation: &str,
        display: Option<&str>,
    ) -> Result<Vec<(String, String)>, String> {
        let tie = self.tie(relation)?;
        let Some(doc) = self.document.as_ref() else {
            return Ok(Vec::new());
        };
        let node = doc.model.layers().get(&tie.parent);
        Ok(doc
            .model
            .by_layer_with_uids(&tie.parent)
            .take(MOST)
            .map(|(uid, x)| {
                let attrs = &x.base().attrs;
                let named = display
                    .and_then(|d| attrs.get(d))
                    .filter(|v| !v.trim().is_empty())
                    .cloned();
                let first = || {
                    node.and_then(|n| {
                        n.fields
                            .iter()
                            .find_map(|f| attrs.get(&f.name).filter(|v| !v.trim().is_empty()))
                            .cloned()
                    })
                };
                let words = named
                    .or_else(|| x.base().label.clone())
                    .or_else(first)
                    .unwrap_or_else(|| format!("Kayıt {}", short(&uid.to_string())));
                (uid.to_string(), words)
            })
            .collect())
    }

    /// A relation list's rows: the records of the relation's child layer naming `parent`, as a table's columns (the
    /// fields' aliases, else names) and rows, each with its `uid`; at most [`MOST`].
    pub(crate) fn relation_rows(
        &self,
        relation: &str,
        parent: &str,
        columns: &[String],
    ) -> Result<(Vec<String>, Vec<Value>), String> {
        let tie = self.tie(relation)?;
        let Some(doc) = self.document.as_ref() else {
            return Ok((Vec::new(), Vec::new()));
        };
        let node = doc.model.layers().get(&tie.child);
        // Each column's name, title and field (its kind writes its values as the layer's table does: a date
        // 16.11.2026); a name the layer lacks is a column of none.
        let fields: Vec<(String, String, Option<kentos_commands::LayerField>)> =
            node.map_or_else(Vec::new, |n| {
                let named = |name: &str| {
                    let field = n.fields.iter().find(|f| f.name == name).cloned();
                    let title = field
                        .as_ref()
                        .map_or_else(|| name.to_owned(), |f| f.label().to_owned());
                    (name.to_owned(), title, field)
                };
                if columns.is_empty() {
                    // Its first fields but the one holding the parent: an id is no reading.
                    n.fields
                        .iter()
                        .filter(|f| f.name != tie.field)
                        .take(FIRST_COLUMNS)
                        .map(|f| named(&f.name))
                        .collect()
                } else {
                    columns.iter().map(|c| named(c)).collect()
                }
            });
        let names = |value: &str| -> Vec<String> {
            if tie.many {
                value.split(';').map(str::trim).map(str::to_owned).collect()
            } else {
                vec![value.trim().to_owned()]
            }
        };
        let rows: Vec<Value> = doc
            .model
            .by_layer_with_uids(&tie.child)
            .filter(|(_, x)| {
                x.base()
                    .attrs
                    .get(&tie.field)
                    .is_some_and(|v| names(v).iter().any(|n| n == parent))
            })
            .take(MOST)
            .map(|(uid, x)| {
                let mut row = Map::new();
                row.insert("uid".to_owned(), json!(uid.to_string()));
                for (name, title, field) in &fields {
                    let value = x.base().attrs.get(name).filter(|v| !v.is_empty());
                    let shown = value.map(|v| {
                        field
                            .as_ref()
                            .map_or_else(|| v.clone(), |f| kentos_commands::display_value(f, v))
                    });
                    row.insert(title.clone(), shown.map_or(Value::Null, |v| json!(v)));
                }
                Value::Object(row)
            })
            .collect();
        let mut titles = vec!["uid".to_owned()];
        titles.extend(fields.into_iter().map(|(_, title, _)| title));
        Ok((titles, rows))
    }
}

impl OpenPanel {
    /// The record component `of` holds: a feature picker's first object, a relation picker's record.
    pub(crate) fn parent_of(&self, of: &str) -> Option<&str> {
        match self.definition.body.iter().find(|c| c.id() == Some(of))? {
            Component::FeaturePicker { .. } => self.picks.get(of)?.first().map(String::as_str),
            Component::RelationPicker { .. } => self
                .choices
                .get(of)
                .map(String::as_str)
                .filter(|c| !c.is_empty()),
            _ => None,
        }
    }
}

/// A persistent id's first eight characters: a record without words, or two with the same, told apart.
fn short(uid: &str) -> String {
    uid.chars().take(8).collect()
}

/// A relation picker: its relation's parent records by their words (two with the same words told apart by their ids'
/// starts), the chosen one shown; why not, when its relation is not in the project.
pub(super) fn relation_select<'a>(
    app: &App,
    p: &OpenPanel,
    id: &str,
    relation: &str,
    display: Option<&str>,
    event: fn(PanelEvent) -> Message,
) -> Element<'a, Message> {
    let choices = match app.relation_choices(relation, display) {
        Ok(choices) => choices,
        Err(why) => return kentos_ui::label::muted(why).into(),
    };
    let chosen = p.choices.get(id);
    let at = choices.iter().position(|(uid, _)| Some(uid) == chosen);
    let shown: Vec<Choice> = choices
        .iter()
        .map(|(uid, words)| {
            let twins = choices.iter().filter(|(_, w)| w == words).count();
            let choice = Choice::new(words.clone());
            if twins > 1 {
                choice.detail(short(uid))
            } else {
                choice
            }
        })
        .collect();
    let empty = choices.is_empty();
    let (id, uids) = (
        id.to_owned(),
        choices.into_iter().map(|(uid, _)| uid).collect::<Vec<_>>(),
    );
    Select::new(shown, at, move |i| {
        event(PanelEvent::Choose(id.clone(), uids[i].clone()))
    })
    .placeholder(if empty { "Kayıt yok" } else { "Seçin" })
    .into()
}

/// A relation list: the records its relation ties to the record `of` holds, as a table; asks for a record first, and
/// says why not when its relation is not in the project.
pub(super) fn relation_table<'a>(
    app: &App,
    p: &OpenPanel,
    id: &str,
    words: Option<&str>,
    relation: &str,
    of: &str,
    columns: &[String],
) -> Element<'a, Message> {
    let title = words.unwrap_or("İlişkili kayıtlar");
    let said = |why: String| -> Element<'a, Message> {
        iced::widget::column![
            kentos_ui::label::caption(title.to_owned()),
            kentos_ui::label::muted(why)
        ]
        .spacing(4)
        .into()
    };
    let Some(parent) = p.parent_of(of) else {
        return said("Önce bir kayıt seçin.".to_owned());
    };
    match app.relation_rows(relation, parent, columns) {
        Ok((titles, rows)) => super::panel_view::rows_table(
            p,
            id,
            Some(title),
            &titles,
            &rows,
            "Bu kayda bağlı kayıt yok.",
        ),
        Err(why) => said(why),
    }
}

#[cfg(test)]
mod tests {
    use kentos_extensions::ui_panel;
    use serde_json::json;

    use super::super::panels::{OpenPanel, PanelEvent};
    use crate::app::{App, Message};

    const RECORDS: &str = include_str!("../../../../../fixtures/kcad/v2/records.json");
    const BINA: &str = "0192f5a1-25a0-7000-8000-000000000301";

    /// The records drawing with its owners' relation: a building (named “A Blok”) and two owners, records without a
    /// geometry whose `Bina` field holds its persistent id.
    fn app() -> App {
        let (mut app, _) = App::boot(None);
        let mut snapshot: serde_json::Value = serde_json::from_str(RECORDS).expect("reads");
        snapshot["settings"]["relations"] = json!([{ "id": "malikler", "name": "Malikler", "parent": "bina",
            "child": "malik", "field": "Bina" }]);
        snapshot["layers"][0]["fields"] = json!([{ "name": "Ad", "kind": "text" }]);
        snapshot["layers"][1]["fields"]
            .as_array_mut()
            .expect("the owners' fields")
            .push(json!({ "name": "Tarih", "kind": "date" }));
        snapshot["entities"][1]["attrs"]["Tarih"] = json!("2026-11-16");
        snapshot["entities"][0]["attrs"] = json!({ "Ad": "A Blok" });
        let snapshot: kentos_commands::DocumentSnapshotV2 =
            serde_json::from_value(snapshot).expect("a drawing");
        let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        app
    }

    fn panel() -> OpenPanel {
        let value = json!({ "format": ui_panel::FORMAT, "schemaVersion": ui_panel::VERSION, "title": "Malikler",
            "body": [
                { "type": "relationPicker", "id": "bina", "label": "Bina", "relation": "malikler", "display": "Ad",
                  "required": true },
                { "type": "relationList", "id": "malikler", "label": "Malikler", "relation": "malikler", "of": "bina",
                  "columns": ["Ad", "Pay"] }
            ] });
        OpenPanel::new(
            "tr.ornek.deneme",
            "malik",
            ui_panel::read(&value).expect("a panel"),
        )
    }

    /// A relation picker offers the parent layer's records by their words and gives the chosen one's persistent id; a
    /// relation list's rows are the child records naming it, with their uids (its columns, else the child's fields but
    /// the link); a relation the project lacks is said; the code sets a record by its id.
    #[test]
    fn a_relation_picker_offers_the_parents_and_its_list_shows_their_children() {
        let mut app = app();
        assert_eq!(
            app.relation_choices("malikler", Some("Ad"))
                .expect("its records"),
            [(BINA.to_owned(), "A Blok".to_owned())]
        );
        let (titles, rows) = app
            .relation_rows("malikler", BINA, &["Ad".into(), "Pay".into()])
            .expect("its rows");
        assert_eq!(titles, ["uid", "Ad", "Pay"]);
        let owners: Vec<&str> = rows.iter().filter_map(|r| r["Ad"].as_str()).collect();
        assert_eq!(owners, ["Ayşe Demir", "Mehmet Kaya"]);
        assert!(rows.iter().all(|r| r["uid"].as_str().is_some()));
        // A value as the layer's table writes it: a date in the day's words.
        let (_, rows) = app
            .relation_rows("malikler", BINA, &["Tarih".into()])
            .expect("its rows");
        assert_eq!(rows[0]["Tarih"], "16.11.2026");
        assert_eq!(
            app.relation_rows("malikler", BINA, &[])
                .expect("its rows")
                .0,
            ["uid", "Ad", "Pay", "Tarih"]
        );
        assert!(
            app.relation_rows("malikler", "baska", &[])
                .expect("none")
                .1
                .is_empty()
        );
        let why = app.relation_choices("yok", None).unwrap_err();
        assert!(why.contains("“yok” ilişkisi bu projede yok"), "{why}");
        app.extensions.panels.push(panel());
        let p = &app.extensions.panels[0];
        assert_eq!(p.parent_of("bina"), None);
        assert!(p.check_params().unwrap_err().1.contains("kayıt seçilmedi"));
        let _ = app.ext_panel_event(PanelEvent::Choose("bina".into(), BINA.into()));
        let p = &mut app.extensions.panels[0];
        assert_eq!(p.parent_of("bina"), Some(BINA));
        assert_eq!(p.check_params().expect("its values")["bina"], json!(BINA));
        assert_eq!(p.rule_value("bina").as_deref(), Some(BINA));
        let not_an_id =
            super::super::panel_session::set(p, &json!({ "id": "bina", "value": "A Blok" }), None);
        assert!(not_an_id.unwrap_err().message.contains("kalıcı kimliği"));
        super::super::panel_session::set(p, &json!({ "id": "bina", "value": null }), None)
            .expect("let go");
        assert_eq!(p.parent_of("bina"), None);
    }
}
