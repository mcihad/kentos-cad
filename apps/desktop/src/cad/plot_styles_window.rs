//! Baskı stili tablosu (`sheet.plotStyles`, docs/adr/1130; plan/cad-advanced.md CAD-ADV-16): a `.ctb`/`.stb` plot style table
//! read by `cad.plotstyles.import`'s plan as it is chosen — its pens or styles as KentOS will plot them and every field
//! it plots otherwise, by name — its pens given to the drawing's layers by hand (never guessed from a layer's colour)
//! and the sheets it is applied to; İçe aktar writes the profile and the sheets' plot settings as one change of the
//! book (one undo), Esc or Kapat writes nothing.
//!
//! ```text
//! ┌ Baskı stili tablosu ───────────────────────────────────────────────────────────────── × ┐
//! │ Tablo ─ Dosya [….ctb] [Dosya seç…], Profilin adı      │ [CTB] [255 stil] [239 aynen] [5 kayıp] │
//! │ Kalemler ─ Duvar [Kalem 1 ▾], Aks [— ▾] …             │ ⚠ Çizgi tipi: 5 …                      │
//! │ Paftalar ─ A-101 [■], A-102 [ ]                       │ Stil · Renk · Tarama · Kalınlık · Uç … │
//! │                                                [Kapat] [■ İçe aktar]                     │
//! └──────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use std::path::PathBuf;

use iced::widget::{button, row};
use iced::{Center, Element, Task};
use kentos_commands::{CommandResult, PlotStylesImport, PlotStylesImported};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, plot_styles_import};
use kentos_ui::widget::{Banner, Form, Switch};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::road_dialog::{self, RoadDialog, col, num};

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Baskı stili tablosu";
pub(crate) const IMPORT: &str = "İçe aktar";
pub(crate) const PICK: &str = "Dosya seç…";

/// The window's state, kept while the program runs.
#[derive(Debug, Clone, Default)]
pub struct PlotStyles {
    pub(crate) path: String,
    pub(crate) name: String,
    /// The drawing's layers by id and path, and the pen or style each is given (an index of [`PlotStyles::keys`]).
    pub(crate) pens: Vec<(String, String, Option<usize>)>,
    /// The book's sheets by id and name, and whether the profile is applied to each.
    pub(crate) sheets: Vec<(String, String, bool)>,
    pub(crate) result: Option<Result<PlotStylesImported, String>>,
    pub(crate) warnings: Vec<String>,
    /// Which table the window shows: the report's lines or the styles.
    pub(crate) view: View,
}

/// The window's two tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Report,
    Styles,
}

impl std::fmt::Display for View {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            View::Report => "Rapor",
            View::Styles => "Stiller",
        })
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Path(String),
    Pick,
    Picked(Option<PathBuf>),
    Name(String),
    /// A layer (by its row) given a pen (by the pick's row; 0 none).
    Pen(usize, usize),
    Sheet(usize, bool),
    View(View),
    Import,
    Close,
}

fn msg(e: Event) -> Message {
    Message::PlotStyles(e)
}

/// A style key as the window says it: a CTB's pen by its number, a STB's style by its name.
fn key_words(key: &serde_json::Value) -> String {
    match key["type"].as_str() {
        Some("aci") => format!("Kalem {}", key["index"].as_u64().unwrap_or(0)),
        _ => key["name"].as_str().unwrap_or_default().to_owned(),
    }
}

impl PlotStyles {
    /// The table's keys, in its order, from the last plan.
    pub(crate) fn keys(&self) -> Vec<serde_json::Value> {
        match &self.result {
            Some(Ok(out)) => out.profile["entries"]
                .as_array()
                .map(|e| e.iter().map(|x| x["key"].clone()).collect())
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    pub(crate) fn input(&self, keys: &[serde_json::Value]) -> PlotStylesImport {
        PlotStylesImport {
            path: self.path.trim().to_owned(),
            base: None,
            name: (!self.name.trim().is_empty()).then(|| self.name.trim().to_owned()),
            assignments: self
                .pens
                .iter()
                .filter_map(|(id, _, k)| {
                    let key = keys.get((*k)?)?;
                    Some(serde_json::json!({ "target": { "type": "modelLayer", "id": id }, "key": key }))
                })
                .collect(),
            sheets: self.sheets.iter().filter(|s| s.2).map(|s| s.0.clone()).collect(),
            expected_revision: None,
        }
    }
}

/// A style's cells: its colour, screening, width, ends and joins as the profile plots them.
fn style_cells(style: &serde_json::Value) -> [String; 4] {
    let color = match style["color"]["type"].as_str() {
        Some("color") => style["color"]["value"].as_str().unwrap_or_default().to_owned(),
        Some("grayscale") => "Gri".to_owned(),
        Some("black") => "Siyah".to_owned(),
        _ => "Nesnenin".to_owned(),
    };
    let screening = format!("%{}", style["screening"].as_u64().unwrap_or(100));
    let width = style["lineWeight"]
        .as_i64()
        .map_or_else(|| "Nesnenin".to_owned(), |um| road_dialog::grouped(um as f64 / 1000.0, 2));
    let ends = |k: &str, names: [(&'static str, &'static str); 3]| -> &'static str {
        style[k]
            .as_str()
            .and_then(|v| names.iter().find(|n| n.0 == v).map(|n| n.1))
            .unwrap_or("Nesnenin")
    };
    let cap = ends("cap", [("butt", "Düz"), ("square", "Kare"), ("round", "Yuvarlak")]);
    let join = ends("join", [("miter", "Köşeli"), ("bevel", "Pahlı"), ("round", "Yuvarlak")]);
    [color, screening, width, format!("{cap} / {join}")]
}

/// A report line's title, by its kind.
fn kind_words(kind: &str) -> &'static str {
    match kind {
        "colorKind" => "Renk türü",
        "modeColor" => "mode_color",
        "grayscale" => "Gri tonlama",
        "grayscaleColor" => "Gri tonlama ve renk",
        "dither" => "Titreşim",
        "screenWithoutDither" => "Titreşimsiz tarama",
        "policyBits" => "Renk kuralı bitleri",
        "physicalPen" => "Fiziksel kalem",
        "virtualPen" => "Sanal kalem",
        "linetype" => "Çizgi tipi",
        "lineweightOutside" => "Tablo dışı kalınlık",
        "lineweightRounded" => "Yuvarlanan kalınlık",
        "endStyle" => "Uç",
        "joinStyle" => "Birleşim",
        "fillStyle" => "Dolgu",
        "description" => "Açıklama",
        "normalChanged" => "Normal stili",
        "unknownField" => "Bilinmeyen alan",
        "scaleFactor" => "Ölçek çarpanı",
        "checksum" => "Sağlama",
        "length" => "Uzunluk",
        _ => "Rapor",
    }
}

impl App {
    /// Baskı stili tablosu (`sheet.plotStyles`).
    pub(crate) fn open_plot_styles(&mut self) {
        let Some(doc) = self.document.as_ref() else {
            self.output("Açık çizim yok.");
            return;
        };
        let mut w = self.plot_styles.take().unwrap_or_default();
        fn walk(nodes: &[kentos_commands::LayerNode], here: &str, out: &mut Vec<(String, String)>) {
            for n in nodes {
                let path = if here.is_empty() { n.name.clone() } else { format!("{here} / {}", n.name) };
                if n.kind == kentos_commands::LayerNodeType::Layer {
                    out.push((n.id.clone(), path.clone()));
                }
                walk(&n.children, &path, out);
            }
        }
        let mut layers = Vec::new();
        walk(doc.model.layers().nodes(), "", &mut layers);
        let given: Vec<(String, String, Option<usize>)> = w.pens.clone();
        w.pens = layers
            .into_iter()
            .map(|(id, path)| {
                let k = given.iter().find(|g| g.0 == id).and_then(|g| g.2);
                (id, path, k)
            })
            .collect();
        let on: Vec<(String, String, bool)> = w.sheets.clone();
        w.sheets = kentos_native_application::sheets_edit::sheets_in(&doc.model)
            .unwrap_or_default()
            .into_iter()
            .map(|s| {
                let chosen = on.iter().any(|o| o.0 == s.id && o.2);
                (s.id, s.name, chosen)
            })
            .collect();
        self.plot_styles = Some(w);
        self.plot_styles_preview();
        self.dialog = Some(Dialog::PlotStyles);
    }

    /// What `cad.plotstyles.import` makes of the chosen table (nothing written).
    pub(crate) fn plot_styles_preview(&mut self) {
        let (Some(doc), Some(w)) = (self.document.as_mut(), self.plot_styles.as_mut()) else {
            return;
        };
        w.warnings.clear();
        if w.path.trim().is_empty() {
            w.result = None;
            return;
        }
        let keys = w.keys();
        let input = w.input(&keys);
        w.result = Some(match plot_styles_import::plan(&ExecutionContext::new(&mut doc.model), &input) {
            CommandResult::Completed { output, warnings } => {
                w.warnings = warnings.into_iter().map(|x| x.message).collect();
                Ok(output)
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err(error.message)
            }
            _ => Err("Yapılamadı.".into()),
        });
        // A pen the new table lacks is no longer given.
        let n = w.keys().len();
        for p in &mut w.pens {
            if p.2.is_some_and(|k| k >= n) {
                p.2 = None;
            }
        }
    }

    pub(crate) fn plot_styles_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.plot_styles.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Path(t) => w.path = t,
            Event::Pick => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Baskı stili tablosunu seçin")
                            .add_filter("Baskı stili tablosu (.ctb, .stb)", &["ctb", "stb"])
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::Picked(p)),
                );
            }
            Event::Picked(None) => return Task::none(),
            Event::Picked(Some(p)) => {
                w.path = p.to_string_lossy().into_owned();
                // A new table: its pens are given afresh.
                for x in &mut w.pens {
                    x.2 = None;
                }
            }
            Event::Name(t) => w.name = t,
            Event::Pen(i, k) => {
                if let Some(p) = w.pens.get_mut(i) {
                    p.2 = k.checked_sub(1);
                }
            }
            Event::Sheet(i, on) => {
                if let Some(s) = w.sheets.get_mut(i) {
                    s.2 = on;
                }
            }
            Event::View(v) => {
                w.view = v;
                return Task::none();
            }
            Event::Import => return self.plot_styles_import(),
            Event::Close => {
                self.dialog = None;
                return Task::none();
            }
        }
        self.plot_styles_preview();
        Task::none()
    }

    fn plot_styles_import(&mut self) -> Task<Message> {
        let (Some(doc), Some(w)) = (self.document.as_mut(), self.plot_styles.as_ref()) else {
            return Task::none();
        };
        let keys = w.keys();
        let input = w.input(&keys);
        // A book edit is no undo step of the drawing by itself (docs/adr/0251 §3); in a group it is one (ADR 0590).
        let group = doc.model.begin_group(TITLE);
        let answer = plot_styles_import::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if matches!(answer, CommandResult::Completed { .. }) {
            doc.model.end_group(group);
        } else {
            doc.model.cancel_group(group);
        }
        match answer {
            CommandResult::Completed { output, warnings } => {
                let name = output.profile["name"].as_str().unwrap_or_default().to_owned();
                self.dialog = None;
                self.plot_styles = Some(PlotStyles::default());
                self.say_warnings(&warnings);
                let applied = if output.applied.is_empty() {
                    "hiçbir paftaya uygulanmadı".to_owned()
                } else {
                    format!("{} paftaya uygulandı", output.applied.len())
                };
                self.say(
                    Level::Success,
                    format!(
                        "“{name}” baskı profili alındı: {} stilin {}'i aynen; {applied}. Tek adımda geri alınır.",
                        output.styles, output.exact
                    ),
                );
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                self.warn(format!("{TITLE}: {}", error.message));
            }
            _ => {}
        }
        Task::none()
    }

    pub(crate) fn plot_styles_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.plot_styles else {
            return label::body("").into();
        };
        let keys = w.keys();
        let options: Vec<String> = std::iter::once("—".to_owned()).chain(keys.iter().map(key_words)).collect();
        let pick_file = button(
            row![
                kentos_ui::icon::icon(crate::icons::from_web(Some("fileOpen"))).size(14.0),
                label::body(PICK)
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding(kentos_ui::theme::metrics::action())
        .style(style::button::secondary)
        .on_press(msg(Event::Pick));
        let mut form = Form::new()
            .label_width(118.0)
            .section("Tablo")
            .field(
                "Dosya",
                kentos_ui::widget::text_field(".ctb ya da .stb", &w.path).on_input(|t| msg(Event::Path(t))),
            )
            .row(pick_file)
            .field(
                "Profilin adı",
                kentos_ui::widget::text_field("dosyanın adı", &w.name).on_input(|t| msg(Event::Name(t))),
            )
            .help("Dosya yalnız okunur; profil dosyanın SHA-256'sını ve adını taşır.")
            .section("Kalemler");
        if keys.is_empty() {
            form = form.help("Tablo seçilince her katmana bir kalem ya da stil verilebilir.");
        } else {
            for (i, (_, path, k)) in w.pens.iter().enumerate() {
                let o = options.clone();
                form = form.field(path.clone(), road_dialog::pick(o, Some(k.map_or(0, |k| k + 1)), move |c| msg(Event::Pen(i, c))));
            }
            form = form.help("Kalem katmanın renginden tahmin edilmez; verilmeyen katman nesnenin görünüşüyle basılır.");
        }
        form = form.section("Paftalar");
        if w.sheets.is_empty() {
            form = form.help("Kitapta pafta yok; profil kitaba yine alınır.");
        }
        for (i, (_, name, on)) in w.sheets.iter().enumerate() {
            form = form.field(name.clone(), Switch::new(*on, move |v| msg(Event::Sheet(i, v))).label("Uygula"));
        }
        let mut d = RoadDialog::new(TITLE, form, msg(Event::Close))
            .form_width(400.0)
            .width(1180.0)
            .empty("Bir .ctb ya da .stb seçilince kalemleri KentOS'un basacağı gibi ve aynı basılmayan her alanı burada görürsünüz; çizim değişmez.");
        let mut can = false;
        match &w.result {
            Some(Err(why)) => d = d.note(Banner::error(why.clone())),
            Some(Ok(out)) => {
                can = true;
                let lost = |s: &str| {
                    out.losses.iter().filter(|l| l["severity"] == s).count().to_string()
                };
                d = d
                    .figure("Biçim", out.format.to_uppercase())
                    .figure("Stil", out.styles.to_string())
                    .figure("Aynen", out.exact.to_string())
                    .figure("Kayıp", lost("lost"))
                    .figure("Yaklaşık", lost("approximate"));
                if !out.description.is_empty() {
                    d = d.status(out.description.clone());
                }
                // One line says how much differs; the report's lines are a table beside the styles' (Rapor / Stiller).
                let (lost_n, near_n) = (lost("lost"), lost("approximate"));
                d = d.note(if lost_n == "0" && near_n == "0" {
                    Banner::info("Bütün stiller tablonun kalemlerinin bastığı gibi alınır.".to_owned())
                } else {
                    Banner::warning(format!(
                        "{lost_n} alan türü KentOS'ta basılmıyor (nesnenin görünüşü kalır), {near_n} alan türü yaklaşık basılıyor; her biri Rapor'da adıyla."
                    ))
                });
                let list = |l: &serde_json::Value, k: &str, most: usize| {
                    let all: Vec<String> = l[k]
                        .as_array()
                        .map(|v| v.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect())
                        .unwrap_or_default();
                    let mut s = all.iter().take(most).cloned().collect::<Vec<_>>().join(", ");
                    if all.len() > most {
                        s.push_str(&format!(" … (+{})", all.len() - most));
                    }
                    s
                };
                let report: Vec<Vec<String>> = out
                    .losses
                    .iter()
                    .map(|l| {
                        let severity = match l["severity"].as_str() {
                            Some("lost") => "Kayıp",
                            Some("approximate") => "Yaklaşık",
                            _ => "Not",
                        };
                        vec![
                            severity.to_owned(),
                            kind_words(l["kind"].as_str().unwrap_or_default()).to_owned(),
                            list(l, "styles", 8),
                            list(l, "values", 4),
                            l["words"].as_str().unwrap_or_default().to_owned(),
                        ]
                    })
                    .collect();
                let rows: Vec<Vec<String>> = out.profile["entries"]
                    .as_array()
                    .map(|entries| {
                        entries
                            .iter()
                            .map(|e| {
                                let words = key_words(&e["key"]);
                                let given: Vec<String> = w
                                    .pens
                                    .iter()
                                    .filter(|p| p.2.and_then(|k| keys.get(k)) == Some(&e["key"]))
                                    .map(|p| p.1.clone())
                                    .collect();
                                let [c, s, wd, ends] = style_cells(&e["style"]);
                                vec![words, c, s, wd, ends, given.join(", ")]
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let styles = road_dialog::table(
                    vec![
                        col("Stil", 96.0),
                        col("Renk", 84.0),
                        num("Tarama", 64.0),
                        num("Kalınlık (mm)", 96.0),
                        col("Uç / birleşim", 130.0),
                        col("Katman", 140.0),
                    ],
                    rows,
                    "Tabloda stil yok",
                );
                let shown = match w.view {
                    View::Report => road_dialog::table(
                        vec![col("Ağırlık", 72.0), col("Konu", 130.0), col("Stiller", 90.0), col("Değerler", 90.0), col("Açıklama", 520.0)],
                        report,
                        "Rapor boş: her alan aynen alınır",
                    ),
                    View::Styles => styles,
                };
                d = d.detail(road_dialog::detail_choice([View::Report, View::Styles], w.view, |v| msg(Event::View(v)), shown));
            }
            None => {}
        }
        d.primary(IMPORT, can.then(|| msg(Event::Import))).view()
    }
}

#[cfg(test)]
#[path = "../tests/plot_styles_window_tests.rs"]
mod tests;
