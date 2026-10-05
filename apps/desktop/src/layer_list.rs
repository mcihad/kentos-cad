//! Katman listesi (docs/adr/0177 §6; the web's `app/layerList.ts` and
//! `ui/layers/LayerListDialog.ts`, both run the shared cases
//! fixtures/layers/v1/list.json written by
//! scripts/fixtures/layer_list_cases.py).
//!
//! Every node of the tree in its order is a row: its path, name, kind, its
//! own visibility and lock, whether it is the active layer, a layer's colour
//! (a drawing colour's name, else the value in capitals), line type and
//! weight in mm (two decimals, a decimal comma), and its objects (a group's on
//! all its layers). The CSV file is UTF-8 with a byte order mark, semicolons
//! and CR LF, as Excel's Turkish settings open it; the clipboard takes the
//! rows between tabs. The window shows the rows; Panoya kopyala and CSV
//! olarak kaydet… hand them over.

use std::path::PathBuf;

use iced::widget::{Column, container};
use iced::{Element, Fill, Task};
use kentos_contracts::{LayerNode, LayerNodeType};
use kentos_geometry_core::display::fixed;
use kentos_interaction::Level;
use kentos_ui::widget::table::{self, Table};
use kentos_ui::widget::{Dialog as Frame, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::ribbon_panels::{DRAW_COLORS, LINE_TYPES};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const LIST_TITLE: &str = "Katman listesi";
const COPY: &str = "Panoya kopyala";
const SAVE: &str = "CSV olarak kaydet…";
const CLOSE: &str = "Kapat";

/// The rows' header, the file's first line.
pub const HEADER: [&str; 10] = [
    "Yol",
    "Ad",
    "Tür",
    "Görünür",
    "Kilitli",
    "Etkin",
    "Renk",
    "Çizgi tipi",
    "Kalınlık (mm)",
    "Nesne sayısı",
];

fn yes(b: bool) -> String {
    if b { "Evet" } else { "Hayır" }.to_owned()
}

/// The header and a row per node, in the tree's order; `count` the objects on a layer.
pub fn rows(tree: &[LayerNode], active: &str, count: &dyn Fn(&str) -> usize) -> Vec<Vec<String>> {
    fn objects(n: &LayerNode, count: &dyn Fn(&str) -> usize) -> usize {
        match n.kind {
            LayerNodeType::Layer => count(&n.id),
            LayerNodeType::Group => n.children.iter().map(|c| objects(c, count)).sum(),
        }
    }
    fn go(
        nodes: &[LayerNode],
        above: &[String],
        active: &str,
        count: &dyn Fn(&str) -> usize,
        out: &mut Vec<Vec<String>>,
    ) {
        for n in nodes {
            let mut path = above.to_vec();
            path.push(n.name.clone());
            let layer = n.kind == LayerNodeType::Layer;
            let color = DRAW_COLORS
                .iter()
                .find(|(_, v)| *v == n.style.color)
                .map_or_else(
                    || n.style.color.to_uppercase(),
                    |(name, _)| (*name).to_owned(),
                );
            let line_type = LINE_TYPES
                .iter()
                .find(|(t, _)| *t == n.style.line_type)
                .map_or("", |(_, name)| name);
            out.push(vec![
                path.join(" / "),
                n.name.clone(),
                if layer { "Katman" } else { "Grup" }.to_owned(),
                yes(n.visible),
                yes(n.locked),
                yes(n.id == active),
                if layer { color } else { String::new() },
                if layer {
                    line_type.to_owned()
                } else {
                    String::new()
                },
                if layer {
                    fixed(n.style.line_weight, 2).replace('.', ",")
                } else {
                    String::new()
                },
                objects(n, count).to_string(),
            ]);
            go(&n.children, &path, active, count, out);
        }
    }
    let mut out = vec![HEADER.iter().map(|h| (*h).to_owned()).collect()];
    go(tree, &[], active, count, &mut out);
    out
}

/// A field between `sep`s: quoted, its quotes doubled, when it holds the
/// separator, a quote or a line break.
fn field(text: &str, sep: char) -> String {
    if text.contains(sep) || text.contains(['"', '\r', '\n']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_owned()
    }
}

/// The CSV file's text: a byte order mark, fields between semicolons, lines
/// ending in CR LF.
pub fn csv(rows: &[Vec<String>]) -> String {
    let mut out = String::from('\u{feff}');
    for r in rows {
        let line: Vec<String> = r.iter().map(|f| field(f, ';')).collect();
        out.push_str(&line.join(";"));
        out.push_str("\r\n");
    }
    out
}

/// The clipboard's text: fields between tabs, lines ending in LF.
pub fn tsv(rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    for r in rows {
        let line: Vec<String> = r.iter().map(|f| field(f, '\t')).collect();
        out.push_str(&line.join("\t"));
        out.push('\n');
    }
    out
}

/// The open window: nothing of its own, the rows are the drawing's.
#[derive(Debug, Clone, Default)]
pub struct Window;

#[derive(Debug, Clone)]
pub enum Event {
    Copy,
    Save,
    Saved(Option<PathBuf>),
    Close,
}

fn msg(event: Event) -> Message {
    Message::LayerList(event)
}

impl App {
    /// The open drawing's rows.
    fn layer_list_rows(&self) -> Vec<Vec<String>> {
        let Some(doc) = &self.document else {
            return vec![HEADER.iter().map(|h| (*h).to_owned()).collect()];
        };
        let model = &doc.model;
        let layers = model.layers();
        rows(layers.nodes(), layers.active(), &|id| model.count(id))
    }

    /// Katman listesi (`layer.list`): the window.
    pub(crate) fn open_layer_list(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        self.layer_list = Some(Window);
        self.dialog = Some(Dialog::LayerList);
    }

    pub(crate) fn layer_list_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Copy => {
                let rows = self.layer_list_rows();
                self.say(
                    Level::Success,
                    format!(
                        "Katman listesi panoya kopyalandı ({} satır; elektronik tabloya yapıştırılabilir).",
                        rows.len() - 1
                    ),
                );
                return iced::clipboard::write(tsv(&rows));
            }
            Event::Save => {
                let name = self
                    .document
                    .as_ref()
                    .map_or_else(|| "cizim".to_owned(), |d| d.model.name().to_owned());
                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("Katman listesini kaydet")
                            .add_filter("CSV (.csv)", &["csv"])
                            .set_file_name(format!("{name}-katmanlar.csv"))
                            .save_file()
                            .await?;
                        Some(file.path().to_path_buf())
                    },
                    |path| msg(Event::Saved(path)),
                );
            }
            Event::Saved(None) => {}
            Event::Saved(Some(path)) => {
                let rows = self.layer_list_rows();
                match std::fs::write(&path, csv(&rows)) {
                    Ok(()) => {
                        let file = path
                            .file_name()
                            .map_or_else(String::new, |f| f.to_string_lossy().into_owned());
                        self.say(
                            Level::Success,
                            format!(
                                "Katman listesi CSV olarak kaydedildi: {file} ({} satır).",
                                rows.len() - 1
                            ),
                        );
                    }
                    Err(e) => self.warn(format!(
                        "Katman listesi kaydedilemedi ({e}); başka bir klasör seçip yeniden deneyin."
                    )),
                }
            }
            Event::Close => {
                self.layer_list = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn layer_list_view(&self) -> Element<'_, Message> {
        let rows = self.layer_list_rows();
        let layers = rows[1..].iter().filter(|r| r[2] == "Katman").count();
        let groups = rows.len() - 1 - layers;
        let widths: [f32; 10] = [160.0, 96.0, 52.0, 58.0, 46.0, 42.0, 72.0, 84.0, 84.0, 80.0];
        let columns = HEADER.iter().zip(widths).enumerate().map(|(i, (h, w))| {
            let c = table::Column::new(*h).width(w);
            if i >= 8 { c.align_right() } else { c }
        });
        let body: Vec<table::Row<'_, Message>> = rows[1..]
            .iter()
            .map(|r| table::Row::new(r.iter().map(|c| label::caption(c.clone()).into())))
            .collect();
        let min: f32 = widths.iter().sum::<f32>() + 8.0 * 10.0 + 20.0;
        let list = container(
            Table::new(columns)
                .extend(body)
                .horizontal()
                .min_width(min)
                .height(320.0),
        )
        .style(style::container::field_box)
        .width(Fill);
        let summary = words::summary(vec![words::text_line(
            Kind::Info,
            format!("{layers} katman ve {groups} grup, ağacın sırasıyla."),
        )]);
        overlay::modal(
            Frame::new(LIST_TITLE)
                .push(Column::new().spacing(12).push(summary).push(list))
                .action(words::secondary(COPY, Some(msg(Event::Copy))))
                .action(words::secondary(SAVE, Some(msg(Event::Save))))
                .action(words::primary(CLOSE, Some(msg(Event::Close))))
                .width(920.0),
            msg(Event::Close),
        )
    }

    /// The window's buttons by their words (a trace's `dialog` step).
    pub(crate) fn layer_list_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        if self.layer_list.is_none() {
            return Err(format!("{LIST_TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(COPY) => Some(msg(Event::Copy)),
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            other => return Err(format!("“{LIST_TITLE}” penceresinde {other} yok")),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use kentos_contracts::LayerNode;
    use serde_json::Value;

    /// The shared cases (fixtures/layers/v1/list.json, written by
    /// scripts/fixtures/layer_list_cases.py from the ADR, not KentOS code);
    /// the web runs them in app/layerList.test.ts.
    #[test]
    fn the_rows_csv_and_clipboard_text_are_as_the_shared_cases_say() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/layers/v1/list.json");
        let file: Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON");
        assert_eq!(file["format"], "kentos.layer-list-cases");
        for c in file["cases"].as_array().expect("cases") {
            let tree: Vec<LayerNode> = serde_json::from_value(c["tree"].clone()).expect("a tree");
            let counts = &c["counts"];
            let rows = super::rows(&tree, c["active"].as_str().expect("active"), &|id| {
                counts[id].as_u64().unwrap_or(0) as usize
            });
            let want: Vec<Vec<String>> = serde_json::from_value(c["rows"].clone()).expect("rows");
            assert_eq!(rows, want, "{}", c["name"]);
            assert_eq!(
                super::csv(&rows),
                c["csv"].as_str().expect("csv"),
                "{}",
                c["name"]
            );
            assert_eq!(
                super::tsv(&rows),
                c["tsv"].as_str().expect("tsv"),
                "{}",
                c["name"]
            );
        }
    }
}
