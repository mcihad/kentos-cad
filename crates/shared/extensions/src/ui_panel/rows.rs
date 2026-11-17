//! A panel's rows (plan/eklenti.md ADIM 22, the component inventory's layout; docs/adr/8057 §55): a component's `row`
//! key puts it beside the components next to it with the same row, as cells of one line (Ada | Parsel, Ara | Temizle).
//! KentOS lays the cells out: side by side where each has room, one under another as the form's other fields where it
//! has not (a narrow dock, large text). A view, not a rule: what a cell holds is asked, checked and given to the
//! command as any field's.
//!
//! - **Next to each other:** a row's components stand one after another in the body; a hidden one leaves the others
//!   their line.
//! - **Small things only:** fields, pickers, check boxes, buttons and words; a table, a chart, a card, a section or an
//!   image takes the panel's width and stands on its own line.
//! - **One tab:** the cells of a row are on the same tab's page.

use serde_json::Value as Json;

use super::tabs::tab_of;

/// The key a component's row is written under.
pub const ROW_KEY: &str = "row";

/// The most cells a row has.
pub const MOST_IN_ROW: usize = 4;

/// The longest a row's name is.
const MOST_CHARS: usize = 40;

/// The kinds a row holds: small enough to share a line.
pub const ROW_KINDS: [&str; 16] = [
    "text",
    "textField",
    "numberField",
    "dateField",
    "timeField",
    "dateTimeField",
    "select",
    "multiSelect",
    "checkbox",
    "layerPicker",
    "fieldPicker",
    "crsPicker",
    "pointPicker",
    "relationPicker",
    "filePicker",
    "button",
];

/// Whether components of `kind` take a row.
pub fn takes_row(kind: &str) -> bool {
    ROW_KINDS.contains(&kind)
}

/// A component's row as written, trimmed; none without one, or when it is not a row's name (Kapı 1 says so).
pub fn row_of(component: &Json) -> Option<String> {
    component
        .get(ROW_KEY)
        .and_then(Json::as_str)
        .map(str::trim)
        .filter(|t| fits(t))
        .map(str::to_owned)
}

fn fits(words: &str) -> bool {
    !words.is_empty() && words.chars().count() <= MOST_CHARS && !words.contains(['\n', '\r'])
}

/// Kapı 1's problems with a panel's rows, each with its JSON pointer: a row that is not a row's name, on a kind that
/// takes none, apart from the rest of its row, on another tab than its row, more than [`MOST_IN_ROW`] cells.
pub fn check(body: &[Json]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    // The row being read: its name, its tab and how many cells it has; the rows already ended.
    let mut open: Option<(String, Option<String>, usize)> = None;
    let mut ended: Vec<String> = Vec::new();
    for (i, component) in body.iter().enumerate() {
        let at = format!("/body/{i}/{ROW_KEY}");
        let row = match component.get(ROW_KEY) {
            None => None,
            Some(Json::String(words)) if fits(words.trim()) => Some(words.trim().to_owned()),
            Some(_) => {
                out.push((
                    at.clone(),
                    format!(
                        "row, bileşenin satırının adı olmalı: boş olmayan, tek satır, en çok {MOST_CHARS} karakterlik \
                         bir söz."
                    ),
                ));
                None
            }
        };
        let kind = component.get("type").and_then(Json::as_str).unwrap_or("");
        if row.is_some() && !takes_row(kind) {
            out.push((
                at.clone(),
                format!(
                    "{kind} bir satıra girmez: panelin genişliğini alır. Satıra girenler: {}.",
                    ROW_KINDS.join(", ")
                ),
            ));
        }
        let tab = tab_of(component);
        let continues = matches!((&open, &row), (Some((name, _, _)), Some(this)) if name == this);
        if continues {
            if let Some((name, row_tab, cells)) = open.as_mut() {
                *cells += 1;
                if *cells == MOST_IN_ROW + 1 {
                    out.push((
                        at,
                        format!("“{name}” satırında en çok {MOST_IN_ROW} bileşen olur."),
                    ));
                } else if *row_tab != tab {
                    out.push((
                        at,
                        format!("“{name}” satırının bileşenleri aynı sekmede durur."),
                    ));
                }
            }
        } else {
            if let Some((name, _, _)) = open.take() {
                ended.push(name);
            }
            if let Some(this) = row {
                if ended.contains(&this) {
                    out.push((
                        at,
                        format!(
                            "“{this}” satırının bileşenleri gövdede art arda durur; araya başka bileşen girmiş."
                        ),
                    ));
                }
                open = Some((this, tab, 1));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// A row is read trimmed; a row's cells stand next to each other, on one tab, small kinds, at most four; each
    /// mistake is said where it is written.
    #[test]
    fn a_rows_cells_stand_together_and_a_wrong_one_is_said_where_it_is() {
        assert_eq!(
            row_of(&json!({ "type": "textField", "row": " Konum " })),
            Some("Konum".to_owned())
        );
        let body = vec![
            json!({ "type": "textField", "id": "ada", "label": "Ada", "row": "Konum" }),
            json!({ "type": "textField", "id": "parsel", "label": "Parsel", "row": "Konum" }),
            json!({ "type": "button", "id": "ara", "label": "Ara", "command": "x", "row": "Düğmeler" }),
            json!({ "type": "button", "id": "sil", "label": "Temizle", "command": "y", "row": "Düğmeler" }),
        ];
        assert!(check(&body).is_empty(), "{:?}", check(&body));
        let wrong = vec![
            json!({ "type": "textField", "id": "ada", "label": "Ada", "row": "Konum" }),
            json!({ "type": "checkbox", "id": "x", "label": "X" }),
            json!({ "type": "textField", "id": "parsel", "label": "Parsel", "row": "Konum" }),
            json!({ "type": "table", "id": "t", "columns": ["a"], "row": "Tablo" }),
            json!({ "type": "numberField", "id": "n", "label": "N", "row": 3 }),
            json!({ "type": "textField", "id": "a", "label": "A", "row": "Sekmeli" }),
            json!({ "type": "textField", "id": "b", "label": "B", "row": "Sekmeli", "tab": "İkinci" }),
        ];
        let said = check(&wrong);
        let places: Vec<&str> = said.iter().map(|(at, _)| at.as_str()).collect();
        assert_eq!(
            places,
            ["/body/2/row", "/body/3/row", "/body/4/row", "/body/6/row"],
            "{said:?}"
        );
        assert!(said[0].1.contains("art arda"));
        assert!(said[1].1.contains("satıra girmez"));
        assert!(said[3].1.contains("aynı sekmede"));
        let five: Vec<Json> = (0..5)
            .map(|i| json!({ "type": "textField", "id": format!("f{i}"), "label": "F", "row": "Uzun" }))
            .collect();
        assert_eq!(
            check(&five),
            [(
                "/body/4/row".to_owned(),
                "“Uzun” satırında en çok 4 bileşen olur.".to_owned()
            )]
        );
    }
}
