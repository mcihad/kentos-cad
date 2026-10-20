//! The table of a batch (docs/adr/0529 §4): rows of parameter values as the İşlemler window's Toplu çalıştır shows
//! them (a column per parameter, a row per run, a cell's text as typed) and the `cad.processing.batch` input they
//! make. The reading of a cell by the parameter's kind is here, not in the window: the same words a script gives
//! as JSON, and the same refusals, with the row and the column they are about.

use std::collections::BTreeMap;

use kentos_commands::{BatchRow, ProcessingBatch};
use kentos_domain::Document;
use serde_json::{Value, json};

use crate::text::fold_turkish;
use crate::types::{ParamDef, ParamKind, Tool};

/// The most rows a batch takes (the command's).
pub use crate::command::MAX_BATCH_ROWS;

/// Whether a parameter's value can be typed in a cell: a number, a text, a switch, a choice, a layer, the objects of
/// a layer (or the selection, all, the visible), an expression, a field. A point, a file, a network and the lists
/// are chosen in the window's form.
fn takes_cell(kind: &ParamKind) -> bool {
    matches!(
        kind,
        ParamKind::Number { .. }
            | ParamKind::Text { .. }
            | ParamKind::Boolean
            | ParamKind::Choice { .. }
            | ParamKind::Layer { .. }
            | ParamKind::Features { .. }
            | ParamKind::Expression { .. }
            | ParamKind::Field { .. }
    )
}

/// The parameters a column can be made of, in the tool's order.
pub fn offered(tool: &Tool) -> Vec<&ParamDef> {
    tool.parameters
        .iter()
        .filter(|p| takes_cell(&p.kind))
        .collect()
}

/// One row: its name and a cell for each column (an empty cell takes the window's value).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Row {
    pub label: String,
    pub cells: Vec<String>,
}

/// The rows and the parameters their columns are.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Row>,
}

impl Table {
    /// A table of `rows` empty rows and no column yet.
    pub fn with_rows(rows: usize) -> Self {
        let mut t = Table::default();
        for _ in 0..rows {
            t.add_row();
        }
        t
    }

    /// A row at the end with empty cells; none past [`MAX_BATCH_ROWS`].
    pub fn add_row(&mut self) -> bool {
        if self.rows.len() >= MAX_BATCH_ROWS {
            return false;
        }
        self.rows.push(Row {
            label: String::new(),
            cells: vec![String::new(); self.columns.len()],
        });
        true
    }

    /// Takes row `i` out; the last row stays (a batch needs one).
    pub fn remove_row(&mut self, i: usize) {
        if self.rows.len() > 1 && i < self.rows.len() {
            self.rows.remove(i);
        }
    }

    /// A column of parameter `name` at the end, unless it is there.
    pub fn add_column(&mut self, name: &str) {
        if self.columns.iter().any(|c| c == name) {
            return;
        }
        self.columns.push(name.to_owned());
        for r in &mut self.rows {
            r.cells.push(String::new());
        }
    }

    pub fn remove_column(&mut self, i: usize) {
        if i < self.columns.len() {
            self.columns.remove(i);
            for r in &mut self.rows {
                if i < r.cells.len() {
                    r.cells.remove(i);
                }
            }
        }
    }

    /// A cell's text, set; a cell not there is ignored.
    pub fn set_cell(&mut self, row: usize, column: usize, text: String) {
        if let Some(cell) = self.rows.get_mut(row).and_then(|r| r.cells.get_mut(column)) {
            *cell = text;
        }
    }
}

/// Why a cell cannot be read: its row (from 0), the parameter of its column and the reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub row: usize,
    pub column: String,
    pub message: String,
}

/// The layer a typed id or name is, by its id first, else by its name (Turkish letters folded).
fn layer_of(doc: &Document, text: &str) -> Option<String> {
    let layers = doc.layers();
    let t = text.trim();
    if let Some(node) = layers.get(t) {
        return Some(node.id.clone());
    }
    let folded = fold_turkish(t);
    layers
        .leaves()
        .into_iter()
        .find(|l| fold_turkish(&l.name) == folded)
        .map(|l| l.id.clone())
}

/// A cell's text as the value of parameter `def`; the reason when it does not fit. A cell is not empty here (an
/// empty cell gives no value: the row takes the window's).
pub fn cell_value(doc: &Document, def: &ParamDef, text: &str) -> Result<Value, String> {
    let t = text.trim();
    match &def.kind {
        ParamKind::Number { .. } => t
            .replace(',', ".")
            .parse::<f64>()
            .ok()
            .filter(|n| n.is_finite())
            .map(|n| json!(n))
            .ok_or_else(|| format!("sayı bekleniyor; “{t}” yazılmış")),
        ParamKind::Boolean => {
            let said = |words: &[&str]| words.iter().any(|w| fold_turkish(w) == fold_turkish(t));
            if said(&["evet", "e", "1", "true", "doğru", "var"]) {
                Ok(json!(true))
            } else if said(&["hayır", "h", "0", "false", "yanlış", "yok"]) {
                Ok(json!(false))
            } else {
                Err(format!("evet ya da hayır bekleniyor; “{t}” yazılmış"))
            }
        }
        ParamKind::Choice { options } => {
            let folded = fold_turkish(t);
            options
                .iter()
                .find(|o| {
                    o.value == t
                        || fold_turkish(&o.value) == folded
                        || fold_turkish(&o.label) == folded
                })
                .map(|o| json!(o.value))
                .ok_or_else(|| {
                    let names: Vec<&str> = options.iter().map(|o| o.value.as_str()).collect();
                    format!("seçenek bekleniyor ({}); “{t}” yazılmış", names.join(", "))
                })
        }
        ParamKind::Layer { .. } => Ok(match layer_of(doc, t) {
            Some(id) => json!({ "layerId": id }),
            None => json!({ "newName": t }),
        }),
        ParamKind::Features { .. } => {
            let said = |words: &[&str]| words.iter().any(|w| fold_turkish(w) == fold_turkish(t));
            if said(&["seçim"]) {
                Ok(json!({ "scope": "selection" }))
            } else if said(&["hepsi", "tümü"]) {
                Ok(json!({ "scope": "all" }))
            } else if said(&["görünen"]) {
                Ok(json!({ "scope": "visible" }))
            } else {
                layer_of(doc, t)
                    .map(|id| json!({ "scope": "layer", "layerId": id }))
                    .ok_or_else(|| {
                        format!(
                            "“{t}” adlı katman yok; katman adı, “seçim”, “hepsi” ya da “görünen” yazın"
                        )
                    })
            }
        }
        ParamKind::Text { .. } | ParamKind::Expression { .. } | ParamKind::Field { .. } => {
            Ok(json!(text))
        }
        _ => Err("bu parametre tabloda yazılamaz; pencerenin formundan seçilir".to_owned()),
    }
}

/// The `cad.processing.batch` input of a table: the window's values (`common`, as the command takes them), the
/// selection and the box, and each row's own values from its filled cells.
pub fn input(
    doc: &Document,
    tool: &Tool,
    common: BTreeMap<String, Value>,
    selection: Vec<String>,
    view: Option<[f64; 4]>,
    table: &Table,
) -> Result<ProcessingBatch, Refusal> {
    let mut rows = Vec::with_capacity(table.rows.len());
    for (i, r) in table.rows.iter().enumerate() {
        let mut parameters = BTreeMap::new();
        for (c, name) in table.columns.iter().enumerate() {
            let text = r.cells.get(c).map_or("", String::as_str);
            if text.trim().is_empty() {
                continue;
            }
            let Some(def) = tool.parameters.iter().find(|p| &p.name == name) else {
                continue;
            };
            let value = cell_value(doc, def, text).map_err(|message| Refusal {
                row: i,
                column: name.clone(),
                message: format!("“{}”: {message}.", def.label),
            })?;
            parameters.insert(name.clone(), value);
        }
        let label = r.label.trim();
        rows.push(BatchRow {
            label: (!label.is_empty()).then(|| label.to_owned()),
            parameters,
        });
    }
    Ok(ProcessingBatch {
        tool: tool.id.clone(),
        parameters: common,
        rows,
        selection,
        view,
    })
}

#[cfg(test)]
mod tests {
    use kentos_commands::DocumentSnapshotV1;

    use super::*;
    use crate::registry::Registry;

    fn doc() -> Document {
        let text = std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../../fixtures/processing/v1/parcels.kcad"),
        )
        .expect("the drawing");
        Document::from_snapshot(DocumentSnapshotV1::from_json(&text).expect("reads"))
            .expect("opens")
    }

    fn param<'a>(tool: &'a Tool, name: &str) -> &'a ParamDef {
        tool.parameters
            .iter()
            .find(|p| p.name == name)
            .expect("the parameter")
    }

    #[test]
    fn a_cell_is_read_by_its_parameters_kind() {
        let doc = doc();
        let tool = Registry::builtin().tool("geometry.buffer").expect("tool");
        // A number with a comma; an empty or word cell is refused with the typed text.
        assert_eq!(
            cell_value(&doc, param(&tool, "distance"), " 12,5 "),
            Ok(json!(12.5))
        );
        assert!(cell_value(&doc, param(&tool, "distance"), "çok").is_err());
        assert_eq!(
            cell_value(&doc, param(&tool, "dissolve"), "Evet"),
            Ok(json!(true))
        );
        assert_eq!(
            cell_value(&doc, param(&tool, "dissolve"), "hayır"),
            Ok(json!(false))
        );
        // A features cell names a layer by its name or id, or the selection, all, the visible.
        assert_eq!(
            cell_value(&doc, param(&tool, "input"), "Parsel"),
            Ok(json!({ "scope": "layer", "layerId": "parsel" }))
        );
        assert_eq!(
            cell_value(&doc, param(&tool, "input"), "seçim"),
            Ok(json!({ "scope": "selection" }))
        );
        assert!(cell_value(&doc, param(&tool, "input"), "yok böyle").is_err());
        // A target layer: an existing one by name, else a new one by that name.
        assert_eq!(
            cell_value(&doc, param(&tool, "layer"), "parsel"),
            Ok(json!({ "layerId": "parsel" }))
        );
        assert_eq!(
            cell_value(&doc, param(&tool, "layer"), "Tamponlar"),
            Ok(json!({ "newName": "Tamponlar" }))
        );
        // A choice by its value or its words.
        let side = param(&tool, "side");
        let ParamKind::Choice { options } = &side.kind else {
            panic!("a choice");
        };
        let first = &options[0];
        assert_eq!(cell_value(&doc, side, &first.value), Ok(json!(first.value)));
        assert_eq!(cell_value(&doc, side, &first.label), Ok(json!(first.value)));
    }

    #[test]
    fn a_table_makes_the_batch_the_command_takes() {
        let doc = doc();
        let tool = Registry::builtin().tool("geometry.buffer").expect("tool");
        let mut t = Table::with_rows(3);
        t.add_column("distance");
        t.add_column("dissolve");
        t.rows[0].cells[0] = "5".into();
        t.rows[1].cells[0] = "10".into();
        t.rows[1].cells[1] = "evet".into();
        t.rows[1].label = " on metre ".into();
        // The third row's cells are empty: it takes the window's values.
        let b = input(&doc, &tool, BTreeMap::new(), Vec::new(), None, &t).expect("fits");
        assert_eq!(b.rows.len(), 3);
        assert_eq!(b.rows[0].parameters["distance"], json!(5.0));
        assert!(!b.rows[0].parameters.contains_key("dissolve"));
        assert_eq!(b.rows[1].label.as_deref(), Some("on metre"));
        assert_eq!(b.rows[1].parameters["dissolve"], json!(true));
        assert!(b.rows[2].parameters.is_empty() && b.rows[2].label.is_none());
        // A cell that does not fit names its row and column.
        t.rows[2].cells[0] = "x".into();
        let e = input(&doc, &tool, BTreeMap::new(), Vec::new(), None, &t).expect_err("refused");
        assert_eq!((e.row, e.column.as_str()), (2, "distance"));
        assert!(
            e.message.starts_with("“Uzaklık”: sayı bekleniyor"),
            "{}",
            e.message
        );
    }

    #[test]
    fn rows_and_columns_come_and_go() {
        let mut t = Table::with_rows(2);
        t.add_column("a");
        t.add_column("a");
        t.add_column("b");
        assert_eq!(t.columns, ["a", "b"]);
        t.set_cell(1, 1, "x".into());
        t.remove_column(0);
        assert_eq!(t.rows[1].cells, ["x"]);
        t.remove_row(0);
        t.remove_row(0);
        assert_eq!(t.rows.len(), 1, "the last row stays");
        while t.add_row() {}
        assert_eq!(t.rows.len(), MAX_BATCH_ROWS);
    }
}
