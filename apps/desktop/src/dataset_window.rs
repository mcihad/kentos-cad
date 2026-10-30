//! Veri kümesi bilgisi (docs/adr/0633, `layer.dataset`): a layer as a dataset, from the Katmanlar menu.
//!
//! ```text
//! ┌ Veri kümesi bilgisi ─────────────────────────────────────────────────────────────┐
//! │ Katman [Kadastro / Parsel                                               ▾]       │
//! │ [Tanım] [Sözlük] [Soy] [Kayıt]                                                   │
//! │ Başlık [Kadastro parselleri ]   Lisans [CC-BY-4.0           ]                    │
//! │ Açıklama [ …                                                  ]                  │
//! │ Sorumlu  [Ad ] [Kurum ] [E-posta ]                                               │
//! │ …                                                                                │
//! │ 3 nesne · alan 3 · TUREF / TM36 (EPSG:5256)                  [Vazgeç] [Kaydet]   │
//! └──────────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! Tanım edits the description and Kaydet writes it through `cad.datasets.edit` (one step “Veri kümesi bilgisi”);
//! the other tabs read `cad.datasets.describe`'s answer, worked out once when the layer is chosen: Sözlük the fields,
//! Soy what a processing tool made the layer from, whether its inputs changed since, the layers it came from and the
//! layers made from it, Kayıt the layer as an OGC API Records record (to the clipboard or a file).

use iced::widget::{Column, container, row, scrollable, text_editor};
use iced::{Element, Fill, Length, Task};
use kentos_commands::{
    CommandResult, DatasetDescription, DatasetInfo, DatasetParty, DatasetsDescribe, DatasetsEdit,
    InputState, dataset_problem,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, datasets_describe, datasets_edit};
use kentos_ui::theme::typography;
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::table::{self, Table};
use kentos_ui::widget::tabs::{Tab, Tabs};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::icons::from_web;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Veri kümesi bilgisi";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const COPY: &str = "Panoya kopyala";
const NO_LAYER: &str = "Çizimde nesne tutan katman yok.";
const TABS: [&str; 4] = ["Tanım", "Sözlük", "Soy", "Kayıt"];

/// The description as typed: every part a text, numbers too, read on Kaydet.
#[derive(Debug, Default)]
pub struct Form {
    title: String,
    description: text_editor::Content,
    name: String,
    organisation: String,
    email: String,
    license: String,
    constraints: String,
    keywords: String,
    updated: String,
    scale: String,
    resolution: String,
    accuracy: String,
    completeness: String,
    statement: String,
}

/// The window: the layer, its description as typed and as described.
#[derive(Debug)]
pub struct Window {
    layer: String,
    tab: usize,
    form: Form,
    described: Option<DatasetDescription>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Layer(usize),
    Tab(usize),
    Field(Part, String),
    Description(text_editor::Action),
    Copy,
    Save,
    Cancel,
}

/// A part of the form a line edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Title,
    Name,
    Organisation,
    Email,
    License,
    Constraints,
    Keywords,
    Updated,
    Scale,
    Resolution,
    Accuracy,
    Completeness,
    Statement,
}

fn msg(e: Event) -> Message {
    Message::DatasetInfo(e)
}

fn some(t: &str) -> Option<String> {
    let t = t.trim();
    (!t.is_empty()).then(|| t.to_owned())
}

/// A number typed with a dot or a comma.
fn number(t: &str) -> Result<Option<f64>, ()> {
    match some(t) {
        None => Ok(None),
        Some(t) => t.replace(',', ".").parse::<f64>().map(Some).map_err(|_| ()),
    }
}

impl Form {
    fn of(d: Option<&DatasetInfo>) -> Self {
        let d = d.cloned().unwrap_or_default();
        let p = d.responsible.clone().unwrap_or_default();
        let num = |v: Option<f64>| v.map(|v| v.to_string()).unwrap_or_default();
        Form {
            title: d.title.unwrap_or_default(),
            description: text_editor::Content::with_text(d.description.as_deref().unwrap_or("")),
            name: p.name.unwrap_or_default(),
            organisation: p.organisation.unwrap_or_default(),
            email: p.email.unwrap_or_default(),
            license: d.license.unwrap_or_default(),
            constraints: d.constraints.unwrap_or_default(),
            keywords: d.keywords.join(", "),
            updated: d.updated.unwrap_or_default(),
            scale: d.scale.map(|s| s.to_string()).unwrap_or_default(),
            resolution: num(d.resolution),
            accuracy: num(d.accuracy),
            completeness: d.completeness.unwrap_or_default(),
            statement: d.statement.unwrap_or_default(),
        }
    }

    fn set(&mut self, part: Part, text: String) {
        let at = match part {
            Part::Title => &mut self.title,
            Part::Name => &mut self.name,
            Part::Organisation => &mut self.organisation,
            Part::Email => &mut self.email,
            Part::License => &mut self.license,
            Part::Constraints => &mut self.constraints,
            Part::Keywords => &mut self.keywords,
            Part::Updated => &mut self.updated,
            Part::Scale => &mut self.scale,
            Part::Resolution => &mut self.resolution,
            Part::Accuracy => &mut self.accuracy,
            Part::Completeness => &mut self.completeness,
            Part::Statement => &mut self.statement,
        };
        *at = text;
    }

    /// The description as typed, or what does not read as a number.
    fn read(&self) -> Result<DatasetInfo, String> {
        let description = self.description.text();
        let party = DatasetParty {
            name: some(&self.name),
            organisation: some(&self.organisation),
            email: some(&self.email),
        };
        let scale = match some(&self.scale) {
            None => None,
            Some(t) => Some(
                t.trim_start_matches("1:")
                    .replace(['.', ' '], "")
                    .parse::<u32>()
                    .map_err(|_| format!("“{t}” bir ölçek paydası değil (1000 ya da 1:1000)."))?,
            ),
        };
        let resolution = number(&self.resolution)
            .map_err(|()| format!("“{}” bir sayı değil (çözünürlük, metre).", self.resolution))?;
        let accuracy = number(&self.accuracy).map_err(|()| {
            format!(
                "“{}” bir sayı değil (konum doğruluğu, metre).",
                self.accuracy
            )
        })?;
        let mut keywords: Vec<String> = Vec::new();
        for k in self.keywords.split(',').filter_map(some) {
            if !keywords.contains(&k) {
                keywords.push(k);
            }
        }
        Ok(DatasetInfo {
            title: some(&self.title),
            description: some(description.trim_end_matches('\n')),
            responsible: (party != DatasetParty::default()).then_some(party),
            license: some(&self.license),
            constraints: some(&self.constraints),
            keywords,
            updated: some(&self.updated),
            scale,
            resolution,
            accuracy,
            completeness: some(&self.completeness),
            statement: some(&self.statement),
        })
    }

    /// Why Kaydet may not write, as the command would say it; none when it may.
    fn problem(&self) -> Option<String> {
        match self.read() {
            Err(why) => Some(why),
            Ok(d) if d.is_empty() => None,
            Ok(d) => dataset_problem(&d),
        }
    }
}

impl App {
    /// The layers a description may go on: those that hold objects, by id.
    fn dataset_layers(&self) -> Vec<String> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        doc.model
            .layers()
            .leaves()
            .into_iter()
            .filter(|l| l.service.is_none())
            .map(|l| l.id.clone())
            .collect()
    }

    /// The layer as `cad.datasets.describe` describes it, its record with it.
    fn describe_layer(&mut self, layer: &str) -> Option<DatasetDescription> {
        let doc = self.document.as_mut()?;
        match datasets_describe::execute(
            &mut ExecutionContext::new(&mut doc.model),
            DatasetsDescribe {
                layers: vec![layer.to_owned()],
                records: true,
            },
        ) {
            CommandResult::Completed { output, .. } => output.datasets.into_iter().next(),
            _ => None,
        }
    }

    fn dataset_window(&mut self, layer: String, tab: usize) -> Window {
        let described = self.describe_layer(&layer);
        Window {
            form: Form::of(described.as_ref().and_then(|d| d.dataset.as_ref())),
            described,
            layer,
            tab,
        }
    }

    /// Veri kümesi bilgisi (`layer.dataset`): the window over `layer`, else the active layer, else the first one.
    pub(crate) fn open_dataset_info(&mut self, layer: Option<String>) -> Task<Message> {
        let layers = self.dataset_layers();
        if layers.is_empty() {
            self.warn(NO_LAYER);
            return Task::none();
        }
        let active = self
            .document
            .as_ref()
            .map(|d| d.model.layers().active().to_owned())
            .unwrap_or_default();
        let wanted = layer.unwrap_or(active);
        let id = layers
            .iter()
            .find(|l| **l == wanted)
            .cloned()
            .unwrap_or_else(|| layers[0].clone());
        let w = self.dataset_window(id, 0);
        self.dataset_info = Some(w);
        self.dialog = Some(Dialog::DatasetInfo);
        Task::none()
    }

    pub(crate) fn dataset_info_event(&mut self, e: Event) -> Task<Message> {
        let Some(mut w) = self.dataset_info.take() else {
            return Task::none();
        };
        match e {
            Event::Layer(i) => {
                if let Some(id) = self.dataset_layers().get(i).cloned() {
                    w = self.dataset_window(id, w.tab);
                }
            }
            Event::Tab(i) => w.tab = i.min(TABS.len() - 1),
            Event::Field(part, text) => w.form.set(part, text),
            Event::Description(action) => w.form.description.perform(action),
            Event::Copy => {
                let text = w
                    .described
                    .as_ref()
                    .and_then(|d| d.record.as_ref())
                    .and_then(|r| serde_json::to_string_pretty(r).ok())
                    .unwrap_or_default();
                self.dataset_info = Some(w);
                self.say(Level::Success, "Kayıt panoya kopyalandı.");
                return iced::clipboard::write(text);
            }
            Event::Save => {
                if w.form.problem().is_none() && self.write_dataset(&w) {
                    self.dialog = None;
                    return Task::none();
                }
            }
            Event::Cancel => {
                self.dialog = None;
                return Task::none();
            }
        }
        self.dataset_info = Some(w);
        Task::none()
    }

    /// Writes the description through `cad.datasets.edit` and says what it did; whether it wrote.
    fn write_dataset(&mut self, w: &Window) -> bool {
        let Ok(dataset) = w.form.read() else {
            return false;
        };
        let name = self.layer_name(&w.layer);
        let Some(doc) = self.document.as_mut() else {
            return false;
        };
        let input = DatasetsEdit {
            layer: w.layer.clone(),
            dataset: (!dataset.is_empty()).then_some(dataset),
            expected_revision: None,
        };
        match datasets_edit::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, .. } => {
                if output.changed {
                    self.announce(
                        Level::Success,
                        format!("“{name}” katmanının veri kümesi bilgisi yazıldı."),
                    );
                } else {
                    self.announce(
                        Level::Info,
                        format!("“{name}” katmanının veri kümesi bilgisi zaten böyle."),
                    );
                }
                true
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.warn(error.message);
                false
            }
            _ => false,
        }
    }

    pub(crate) fn dataset_info_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.dataset_info else {
            return label::caption("").into();
        };
        let layers = self.dataset_layers();
        let layer = Select::new(
            layers
                .iter()
                .map(|id| Choice::new(self.layer_path(id)))
                .collect::<Vec<_>>(),
            layers.iter().position(|id| *id == w.layer),
            |i| msg(Event::Layer(i)),
        );
        let stale = w.described.as_ref().is_some_and(|d| d.stale);
        let tabs = Tabs::new(
            TABS.iter().enumerate().map(|(i, t)| {
                let tab = Tab::new(*t).closable(false);
                if i == 2 && stale {
                    tab.icon(from_web(Some("warning")))
                } else {
                    tab
                }
            }),
            w.tab,
            |i| msg(Event::Tab(i)),
        );
        let page: Element<'_, Message> = match w.tab {
            0 => self.dataset_form(w),
            1 => dictionary(w.described.as_ref()),
            2 => self.lineage(w.described.as_ref()),
            _ => record(w.described.as_ref()),
        };
        let summary = w.described.as_ref().map_or_else(String::new, summary_line);
        let problem = (w.tab == 0).then(|| w.form.problem()).flatten();
        let mut body = Column::new()
            .spacing(12)
            .push(words::field("Katman", layer, None))
            .push(tabs)
            .push(container(page).width(Fill));
        body = body.push(match &problem {
            Some(p) => words::text_line(Kind::Warn, p.clone()),
            None => words::text_line(Kind::Info, summary),
        });
        let mut frame = Frame::new(TITLE)
            .scroll(body)
            .action(words::secondary(CANCEL, Some(msg(Event::Cancel))));
        if w.tab == 3 {
            frame = frame.action(words::secondary(COPY, Some(msg(Event::Copy))));
        }
        frame
            .action(words::primary(
                SAVE,
                problem.is_none().then(|| msg(Event::Save)),
            ))
            .width(820.0)
            .max_height(820.0)
            .modal(msg(Event::Cancel))
    }

    /// Tanım: the description's parts, two to a row.
    fn dataset_form<'a>(&'a self, w: &'a Window) -> Element<'a, Message> {
        let f = &w.form;
        let line = |placeholder: &'static str, value: &'a str, part: Part| {
            kentos_ui::widget::text_field(placeholder, value)
                .on_input(move |t| msg(Event::Field(part, t)))
        };
        let pair = |a: Element<'a, Message>, b: Element<'a, Message>| -> Element<'a, Message> {
            row![container(a).width(Fill), container(b).width(Fill)]
                .spacing(12)
                .into()
        };
        let description = text_editor(&f.description)
            .placeholder("Veri kümesi neyi içerir, kimin için")
            .on_action(|a| msg(Event::Description(a)))
            .height(64)
            .padding(6)
            .size(typography::body());
        Column::new()
            .spacing(10)
            .push(pair(
                words::field("Başlık", line("Katmanın adı", &f.title, Part::Title), None),
                words::field("Lisans", line("CC-BY-4.0", &f.license, Part::License), None),
            ))
            .push(words::field("Açıklama", description, None))
            .push(words::field(
                "Sorumlu",
                row![
                    container(line("Ad", &f.name, Part::Name)).width(Fill),
                    container(line("Kurum", &f.organisation, Part::Organisation)).width(Fill),
                    container(line("E-posta", &f.email, Part::Email)).width(Fill),
                ]
                .spacing(8),
                None,
            ))
            .push(pair(
                words::field(
                    "Kullanım kısıtı",
                    line("Kurum içi kullanım", &f.constraints, Part::Constraints),
                    None,
                ),
                words::field(
                    "Anahtar sözcükler",
                    line("kadastro, parsel", &f.keywords, Part::Keywords),
                    Some("Virgülle ayırın.".to_owned()),
                ),
            ))
            .push(
                row![
                    container(words::field(
                        "Güncelleme",
                        line("YYYY-AA-GG", &f.updated, Part::Updated),
                        None
                    ))
                    .width(Fill),
                    container(words::field(
                        "Ölçek",
                        line("1:1000", &f.scale, Part::Scale),
                        None
                    ))
                    .width(Fill),
                    container(words::field(
                        "Çözünürlük (m)",
                        line("0,5", &f.resolution, Part::Resolution),
                        None
                    ))
                    .width(Fill),
                    container(words::field(
                        "Konum doğruluğu (m)",
                        line("0,05", &f.accuracy, Part::Accuracy),
                        None
                    ))
                    .width(Fill),
                ]
                .spacing(12),
            )
            .push(pair(
                words::field(
                    "Tamlık",
                    line("Bütün ilçe", &f.completeness, Part::Completeness),
                    None,
                ),
                words::field(
                    "Üretim açıklaması",
                    line("Saha ölçmesi, 2026", &f.statement, Part::Statement),
                    None,
                ),
            ))
            .into()
    }

    /// Soy: what the layer was made from and whether that changed, the layers it came from and those made from it.
    fn lineage<'a>(&'a self, d: Option<&'a DatasetDescription>) -> Element<'a, Message> {
        let Some(d) = d else {
            return label::muted("Katman tanımlanamadı.").into();
        };
        let name = |id: &str| {
            self.document
                .as_ref()
                .and_then(|doc| doc.model.layers().get(id))
                .map_or_else(|| format!("{id} (çizimde yok)"), |n| self.layer_path(&n.id))
        };
        let mut col = Column::new().spacing(10);
        match &d.derivation {
            None => {
                col = col.push(label::muted(
                    "Bu katman bir işlem aracıyla üretilmedi; türetim kaydı yok.",
                ));
            }
            Some(dv) => {
                col = col.push(if d.stale {
                    words::text_line(
                        Kind::Warn,
                        "Eski türev: girdilerinden biri ya da geldiği bir katman, katman üretildikten sonra değişti. \
                         Aracı yeniden çalıştırın.",
                    )
                } else {
                    words::text_line(Kind::Ok, "Güncel: girdileri üretildiği andaki gibi.")
                });
                col = col.push(words::field(
                    "Araç",
                    label::body(format!(
                        "{} ({}, sürüm {})",
                        dv.label, dv.tool, dv.tool_version
                    )),
                    None,
                ));
                col = col.push(words::field(
                    "Üretildi",
                    label::body(format!(
                        "{} · {} · çizim sürümü {}",
                        dv.at, dv.engine, dv.document_revision
                    )),
                    None,
                ));
                if !dv.parameters.is_empty() {
                    let values: Vec<String> = dv
                        .parameters
                        .iter()
                        .map(|(k, v)| format!("{k}: {v}"))
                        .collect();
                    col = col.push(words::field(
                        "Değerler",
                        label::mono(values.join("\n")),
                        None,
                    ));
                }
                let mut inputs = Column::new().spacing(4);
                for i in &d.inputs {
                    let (kind, said) = match i.state {
                        InputState::Current => (Kind::Ok, "aynı"),
                        InputState::Changed => (Kind::Warn, "değişti"),
                        InputState::Gone => (Kind::Error, "çizimde yok"),
                    };
                    inputs = inputs.push(words::text_line(kind, format!("{}: {said}", i.name)));
                }
                col = col.push(words::field("Girdiler", inputs, None));
            }
        }
        let list = |ids: &[String]| -> Element<'a, Message> {
            if ids.is_empty() {
                label::muted("Yok.").into()
            } else {
                label::body(
                    ids.iter()
                        .map(|id| name(id))
                        .collect::<Vec<_>>()
                        .join(" ← "),
                )
                .into()
            }
        };
        col.push(words::field("Geldiği katmanlar", list(&d.upstream), None))
            .push(words::field("Ondan üretilenler", list(&d.downstream), None))
            .into()
    }
}

/// The description's one line: objects, kinds, extent's system.
fn summary_line(d: &DatasetDescription) -> String {
    let kinds: Vec<String> = d
        .kinds
        .iter()
        .map(|(k, n)| format!("{} {n}", kentos_commands::kind_words(k)))
        .collect();
    let mut out = format!("{} nesne", d.objects);
    if !kinds.is_empty() {
        out.push_str(&format!(" ({})", kinds.join(", ")));
    }
    out.push_str(&format!(" · {}", d.crs));
    if let Some(g) = d.geographic {
        out.push_str(&format!(
            " · WGS 84 kutusu {:.5}°–{:.5}° D, {:.5}°–{:.5}° K",
            g[0], g[2], g[1], g[3]
        ));
    }
    out
}

/// Sözlük: the fields, one a row.
fn dictionary(d: Option<&DatasetDescription>) -> Element<'_, Message> {
    let Some(d) = d else {
        return label::muted("Katman tanımlanamadı.").into();
    };
    if d.dictionary.is_empty() {
        return label::muted(
            "Katmanın alanı yok: nesnelerin öznitelikleri tanımsız. Alanlar… ile tanımlayın.",
        )
        .into();
    }
    let columns = [
        ("Alan", 170.0),
        ("Takma ad", 150.0),
        ("Tür", 110.0),
        ("Zorunlu", 70.0),
        ("Birim", 70.0),
        ("Değerler", 80.0),
        ("Dolu", 80.0),
    ]
    .map(|(title, width)| table::Column::new(title).width(Length::Fixed(width)));
    Table::new(columns)
        .extend(d.dictionary.iter().map(|f| {
            table::Row::new(
                [
                    f.name.clone(),
                    f.alias.clone().unwrap_or_default(),
                    f.kind.name().to_owned(),
                    if f.required { "evet" } else { "" }.to_owned(),
                    f.unit.clone().unwrap_or_default(),
                    f.choices.map(|n| n.to_string()).unwrap_or_default(),
                    format!("{}/{}", f.filled, d.objects),
                ]
                .map(|value| table::text(value).into()),
            )
        }))
        .into()
}

/// Kayıt: the record as JSON.
fn record(d: Option<&DatasetDescription>) -> Element<'_, Message> {
    let text = d
        .and_then(|d| d.record.as_ref())
        .and_then(|r| serde_json::to_string_pretty(r).ok())
        .unwrap_or_default();
    Column::new()
        .spacing(6)
        .push(label::muted(
            "Katmanın OGC API Records kaydı (GeoJSON): bir kataloğa verilebilir.",
        ))
        .push(
            container(
                scrollable(container(label::mono(text)).width(Fill))
                    .width(Fill)
                    .height(Length::Fixed(360.0)),
            )
            .padding(8)
            .style(style::container::field_box)
            .width(Fill),
        )
        .into()
}

impl App {
    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn dataset_info_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(w) = &self.dataset_info else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Fill("Başlık", t) => Some(msg(Event::Field(Part::Title, t.to_owned()))),
            Control::Fill("Lisans", t) => Some(msg(Event::Field(Part::License, t.to_owned()))),
            Control::Press(SAVE) => w.form.problem().is_none().then(|| msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(t) => match TABS.iter().position(|x| *x == t) {
                Some(i) => Some(msg(Event::Tab(i))),
                None => return Err(format!("“{TITLE}” penceresinde “{t}” düğmesi yok")),
            },
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Event, Part};
    use crate::app::{App, Dialog, Message};

    /// The shared cases' drawing with three derived layers (`fixtures/commands/v1/cad.datasets.describe.json`).
    fn app() -> App {
        let (mut app, _) = App::boot(None);
        let cases: serde_json::Value = serde_json::from_str(include_str!(
            "../../../fixtures/commands/v1/cad.datasets.describe.json"
        ))
        .expect("the cases read");
        let setup = cases["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .find(|c| {
                c["name"]
                    .as_str()
                    .is_some_and(|n| n.starts_with("türetim kaydı"))
            })
            .expect("the derived drawing")["setup"]
            .to_string();
        let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&setup).expect("reads");
        let doc = crate::document::Document::new(snapshot, None).expect("opens");
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        app
    }

    fn ev(app: &mut App, e: Event) {
        let _ = app.update(Message::DatasetInfo(e));
    }

    #[test]
    fn the_window_writes_a_description_through_the_command() {
        let mut app = app();
        let _ = app.update(Message::Layer(crate::layering::Event::Dataset(
            "parsel".into(),
        )));
        assert_eq!(app.dialog, Some(Dialog::DatasetInfo));
        ev(
            &mut app,
            Event::Field(Part::Title, "Kadastro parselleri".into()),
        );
        ev(&mut app, Event::Field(Part::Updated, "2026-02-30".into()));
        ev(&mut app, Event::Save);
        assert_eq!(
            app.dialog,
            Some(Dialog::DatasetInfo),
            "a day that is no day: not written"
        );
        ev(&mut app, Event::Field(Part::Updated, "2026-10-20".into()));
        ev(&mut app, Event::Field(Part::Scale, "1:1000".into()));
        ev(
            &mut app,
            Event::Field(Part::Keywords, "kadastro, parsel, kadastro".into()),
        );
        ev(&mut app, Event::Save);
        assert!(app.dialog.is_none(), "Kaydet closes the window");
        let doc = app.document.as_ref().expect("open");
        let d = doc
            .model
            .layers()
            .get("parsel")
            .and_then(|n| n.dataset.clone())
            .expect("written");
        assert_eq!(d.title.as_deref(), Some("Kadastro parselleri"));
        assert_eq!(d.scale, Some(1000));
        assert_eq!(d.keywords, vec!["kadastro".to_owned(), "parsel".to_owned()]);
        let _ = app.update(Message::Run("edit.undo"));
        let doc = app.document.as_ref().expect("open");
        assert!(
            doc.model
                .layers()
                .get("parsel")
                .is_some_and(|n| n.dataset.is_none()),
            "one undo step"
        );
    }

    #[test]
    fn a_derived_layer_says_it_is_stale() {
        let mut app = app();
        let _ = app.update(Message::Layer(crate::layering::Event::Dataset(
            "ikinci".into(),
        )));
        let w = app.dataset_info.as_ref().expect("open");
        let d = w.described.as_ref().expect("described");
        assert!(d.stale);
        assert_eq!(d.upstream, vec!["tampon".to_owned(), "parsel".to_owned()]);
    }

    /// The four tabs, dark and light (`.run/shots/veri-kumesi-*`):
    ///
    /// ```text
    /// cargo test -p kentos-desktop --lib dataset_window::tests::screens -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn screens() {
        use iced::Size;
        use kentos_ui::snapshot::Snapshot;

        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
            for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
                for (tab, name, layer) in [
                    (0, "tanim", "parsel"),
                    (1, "sozluk", "parsel"),
                    (2, "soy", "ikinci"),
                    (3, "kayit", "parsel"),
                ] {
                    let mut app = app();
                    let _ = app
                        .settings
                        .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                    app.apply_settings();
                    let _ = app.update(Message::WindowResized(Size::new(width, height)));
                    if let Some(doc) = app.document.as_mut() {
                        let _ = kentos_native_application::datasets_edit::execute(
                            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
                            serde_json::from_value(serde_json::json!({
                                "layer": "parsel",
                                "dataset": {
                                    "title": "Kadastro parselleri",
                                    "description": "İlçenin tescilli parselleri, tapu ve kadastro verisiyle.",
                                    "responsible": { "name": "Harita Müdürlüğü", "organisation": "Örnek Belediyesi",
                                                     "email": "harita@ornek.bel.tr" },
                                    "license": "CC-BY-4.0", "constraints": "Kurum içi kullanım.",
                                    "keywords": ["kadastro", "parsel"], "updated": "2026-10-20", "scale": 1000,
                                    "accuracy": 0.05, "completeness": "Bütün ilçe.",
                                    "statement": "TKGM'nin 2026 aktarımı."
                                }
                            }))
                            .expect("an input"),
                        );
                    }
                    let _ = app.update(Message::Layer(crate::layering::Event::Dataset(
                        layer.into(),
                    )));
                    ev(&mut app, Event::Tab(tab));
                    let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                    let mut update = |app: &mut App, message| {
                        let _ = app.update(message);
                    };
                    snapshot.settle(&mut app, App::view, &mut update);
                    let file = out.join(format!("veri-kumesi-{name}-{width}x{height}{suffix}.png"));
                    snapshot
                        .render(app.view(), &app.theme())
                        .save(&file)
                        .expect("writes the picture");
                    println!("{}", file.display());
                }
            }
        }
    }
}
