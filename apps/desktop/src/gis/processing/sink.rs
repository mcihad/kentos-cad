//! Sonuç and Bütün tablo (docs/adr/0592 §3): where a tool run here writes its result, and an input on a bound layer
//! read from its source. “Çizime” is the drawing as ever; “GeoPackage dosyasına” and “PostGIS veritabanına” run
//! `cad.processing.run` with a sink on a copy of the drawing on another thread, the drawing unchanged, Durdur stopping
//! it between parts (nothing of the result is then written). An input on a layer bound to a table can be read whole
//! from its source (“Bütün tablo”) when the result does not go to the drawing: the part the drawing holds is said in
//! the switch's note. The window's line says what was written where.
//!
//! The target is chosen before the output's name (docs/adr/9354): to a file or a database the output layer is the
//! table's name there. A refusal is said in the window's words (the switch to turn on, the choice to make), the file
//! told from the table by the provider's code. A written result offers Katman olarak ekle (its tables as data layers)
//! and, for a file, Dosya konumunu aç. Every run here, written or refused, goes into İşlem geçmişi with its target.

use crate::gis;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use iced::futures::channel::oneshot;
use iced::widget::{Column, button, column, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{ProcessingRun, RunSink, SinkMode};
use kentos_domain::Document;
use kentos_native_application::processing_host::{BatchFlow, RunEnd, tools};
use kentos_processing::command::command_values;
use kentos_processing::{ParamKind, Tool};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::radio::RadioGroup;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Switch, Tip, tip};
use serde_json::{Value, json};

use super::Event;
use super::dialog::{RunStatus, ToolDialog};
use crate::app::{App, Message};

/// Where the result goes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Place {
    #[default]
    Drawing,
    File,
    Database,
}

/// A features input on a layer bound to a table, now.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Bound {
    pub param: String,
    pub layer: String,
    pub table: String,
    /// The drawing holds a part of the table (an area).
    pub part: bool,
}

/// The window's Sonuç.
#[derive(Clone, Debug)]
pub(crate) struct LocalSink {
    pub place: Place,
    pub path: String,
    pub mode: SinkMode,
    pub overwrite: bool,
    pub address: String,
    pub schema: String,
    /// The inputs read whole from their source.
    pub whole: BTreeSet<String>,
    pub bound: Vec<Bound>,
    /// The run going on another thread: its stop.
    pub stop: Option<Arc<AtomicBool>>,
    /// The run asked, kept until it ends for its history record.
    pub asked: Option<Asked>,
    /// What the last run wrote: its card's actions.
    pub written: Option<Written>,
}

/// Where a run wrote: the window's Sonucun yeri as it was, for Yeniden aç (docs/adr/9354).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Target {
    pub place: Place,
    pub path: String,
    pub mode: SinkMode,
    pub overwrite: bool,
    pub address: String,
    pub schema: String,
}

impl LocalSink {
    /// Its target as it is now.
    fn target(&self) -> Target {
        Target {
            place: self.place,
            path: self.path.clone(),
            mode: self.mode,
            overwrite: self.overwrite,
            address: self.address.clone(),
            schema: self.schema.clone(),
        }
    }

    /// A run's target back (Yeniden aç).
    pub(crate) fn aim(&mut self, t: &Target) {
        self.place = t.place;
        self.path = t.path.clone();
        self.mode = t.mode;
        self.overwrite = t.overwrite;
        self.address = t.address.clone();
        self.schema = t.schema.clone();
    }
}

/// A run to a file or a database as it was asked (docs/adr/9354): its record's tool, values, inputs and target.
#[derive(Clone, Debug)]
pub(crate) struct Asked {
    pub tool: String,
    pub label: String,
    pub values: kentos_processing::Values,
    /// When it started: ms since the Unix epoch, and the clock to time it.
    pub started: (u64, std::time::Instant),
    /// The run in words: its inputs as they resolved, then where it writes.
    pub words: Vec<String>,
    pub target: Target,
}

/// What a run to a file or a database wrote (docs/adr/9354).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Written {
    /// The GeoPackage's path or the database's address.
    pub address: String,
    pub file: bool,
    /// Its tables as a data layer names them (`schema.table` in a database), with their rows.
    pub tables: Vec<(String, u64)>,
    /// Katman olarak ekle was asked.
    pub added: bool,
}

/// A run to a file or a database that wrote its result: what the line says, where, and what flowed through it.
#[derive(Clone, Debug)]
pub struct Ran {
    text: String,
    written: Written,
    records_in: u64,
    records_out: u64,
}

impl Default for LocalSink {
    fn default() -> Self {
        Self {
            place: Place::Drawing,
            path: String::new(),
            mode: SinkMode::New,
            overwrite: false,
            address: String::new(),
            schema: "public".to_owned(),
            whole: BTreeSet::new(),
            bound: Vec::new(),
            stop: None,
            asked: None,
            written: None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum SinkEvent {
    Place(Place),
    Path(String),
    ChoosePath,
    Mode(SinkMode),
    Overwrite(bool),
    Address(String),
    Schema(String),
    Whole(String, bool),
    /// The run on the other thread ended: what it wrote, or why it did not.
    Ran(Result<Ran, String>),
    /// Katman olarak ekle: the written tables as data layers.
    AddLayer,
    /// Dosya konumunu aç: the written file's folder.
    Reveal,
}

fn ev(e: SinkEvent) -> Message {
    Message::Gis(gis::Event::Processing(Event::Sink(e)))
}

const MODES: [(SinkMode, &str); 3] = [
    (SinkMode::New, "Yeni"),
    (SinkMode::AddTables, "Tablo ekle"),
    (SinkMode::ReplaceTables, "Tabloyu değiştir"),
];

impl LocalSink {
    /// The bound inputs again from the window's values (the layer chosen may have changed); one no longer bound is
    /// not read whole any more. A database address is offered from the drawing's PostGIS layers.
    pub(crate) fn refresh(
        &mut self,
        doc: &Document,
        tool: &Tool,
        values: &kentos_processing::Values,
    ) {
        let params = command_values(doc, tool, values);
        self.bound = tool
            .parameters
            .iter()
            .filter(|p| matches!(p.kind, ParamKind::Features { .. }))
            .filter_map(|p| {
                let v = params.get(&p.name)?;
                if v.get("scope").and_then(Value::as_str) != Some("layer") {
                    return None;
                }
                let layer = v.get("layerId").and_then(Value::as_str)?;
                let link = doc.settings().sources.iter().find(|s| s.layer == layer)?;
                Some(Bound {
                    param: p.name.clone(),
                    layer: layer.to_owned(),
                    table: link.table.clone(),
                    part: link.extent.is_some(),
                })
            })
            .collect();
        let bound: BTreeSet<&str> = self.bound.iter().map(|b| b.param.as_str()).collect();
        self.whole.retain(|p| bound.contains(p.as_str()));
        if self.address.is_empty()
            && let Some(db) = doc
                .settings()
                .sources
                .iter()
                .find(|s| kentos_commands::is_database(&s.path))
        {
            self.address = db.path.clone();
        }
    }

    /// The sink the run is given; none for the drawing.
    pub(crate) fn sink(&self) -> Option<RunSink> {
        let path = match self.place {
            Place::Drawing => return None,
            Place::File => self.path.trim().to_owned(),
            Place::Database => self.address.trim().to_owned(),
        };
        Some(RunSink {
            path,
            schema: (self.place == Place::Database).then(|| self.schema.trim().to_owned()),
            overwrite: self.overwrite && self.mode == SinkMode::New,
            mode: self.mode,
            // A tool's table output goes to the file too (UR2-029: Nokta bulutu alan sorgusu's table was refused, the
            // window having no way to ask for it).
            tables: true,
        })
    }

    /// The window's values with each input read whole from its source as chosen (`scope: source` on its layer), when
    /// the result does not go to the drawing.
    pub(crate) fn read_whole(
        &self,
        mut parameters: std::collections::BTreeMap<String, Value>,
    ) -> std::collections::BTreeMap<String, Value> {
        if self.place == Place::Drawing {
            return parameters;
        }
        for b in &self.bound {
            if self.whole.contains(&b.param) {
                let mut v = json!({ "scope": "source", "layer": b.layer });
                if let Some(kinds) = parameters.get(&b.param).and_then(|p| p.get("kinds")) {
                    v["kinds"] = kinds.clone();
                }
                parameters.insert(b.param.clone(), v);
            }
        }
        parameters
    }

    /// What keeps Çalıştır from asking: an empty path or address.
    pub(crate) fn problem(&self) -> Option<&'static str> {
        match self.place {
            Place::Drawing => None,
            Place::File if self.path.trim().is_empty() => {
                Some("Sonucun GeoPackage dosyasını seçin.")
            }
            Place::Database if self.address.trim().is_empty() => Some(
                "Sonucun yazılacağı veritabanının adresini yazın (postgresql://kullanıcı@sunucu/veritabanı).",
            ),
            _ => None,
        }
    }
}

/// Sonucun yeri in the window's form: the drawing, a GeoPackage (its path, its tables' mode) or a database (its
/// address, the schema).
pub(crate) fn place_view(window: &ToolDialog) -> Element<'_, Message> {
    let s = &window.sink;
    let places = RadioGroup::new(s.place, |p| ev(SinkEvent::Place(p)))
        .option(Place::Drawing, "Çizime", "yeni katman, tek geri alma adımı")
        .option(Place::File, "GeoPackage dosyasına", "çizim değişmez")
        .option(
            Place::Database,
            "PostGIS veritabanına",
            "çizim değişmez; tablolar tek işlemde",
        );
    let mut out = Column::new().spacing(10).push(places);
    let field = |placeholder: &str, value: &str, on: fn(String) -> SinkEvent| {
        kentos_ui::widget::text_field(placeholder, value)
            .style(style::field::validated(false))
            .on_input(move |t| ev(on(t)))
            .width(Fill)
    };
    let modes = || {
        let at = MODES.iter().position(|(m, _)| *m == s.mode);
        Select::new(MODES.iter().map(|(_, l)| Choice::new(*l)), at, |i| {
            ev(SinkEvent::Mode(MODES.get(i).map_or(SinkMode::New, |m| m.0)))
        })
        .prefix("Tablolar")
    };
    match s.place {
        Place::Drawing => {}
        Place::File => {
            let choose = button(label::body("Seç…"))
                .padding(kentos_ui::theme::metrics::action())
                .style(style::button::secondary)
                .on_press(ev(SinkEvent::ChoosePath));
            out = out.push(
                row![
                    field("/klasör/sonuç.gpkg", &s.path, SinkEvent::Path),
                    choose
                ]
                .spacing(6)
                .align_y(Center),
            );
            let mut line = row![container(modes()).width(Fill)]
                .spacing(16)
                .align_y(Center);
            if s.mode == SinkMode::New {
                line = line.push(
                    row![
                        Switch::new(s.overwrite, |v| ev(SinkEvent::Overwrite(v))),
                        label::body("Varsa üzerine yaz")
                    ]
                    .spacing(8)
                    .align_y(Center),
                );
            }
            out = out.push(line);
        }
        Place::Database => {
            out = out.push(field(
                "postgresql://kullanıcı@sunucu/veritabanı",
                &s.address,
                SinkEvent::Address,
            ));
            out = out.push(
                row![
                    label::body("Şema").style(style::text::muted),
                    container(field("public", &s.schema, SinkEvent::Schema)).width(Fill),
                    container(modes()).width(Fill)
                ]
                .spacing(10)
                .align_y(Center),
            );
            out = out.push(
                label::caption(
                    "Parola adrese yazılmaz: PGPASSWORD ortam değişkeni ya da ~/.pgpass.",
                )
                .style(style::text::muted),
            );
        }
    }
    out.into()
}

/// Bütün tablo under a features input on a bound layer: read whole from its source, open when the result does not
/// go to the drawing.
pub(crate) fn whole_row<'a>(window: &'a ToolDialog, param: &str) -> Option<Element<'a, Message>> {
    let s = &window.sink;
    let b = s.bound.iter().find(|b| b.param == param)?;
    let on = s.whole.contains(&b.param);
    let open = s.place != Place::Drawing;
    let name = b.param.clone();
    let switch = Switch::new(on && open, move |v| ev(SinkEvent::Whole(name.clone(), v)));
    let switch: Element<'a, Message> = if open {
        switch.into()
    } else {
        tip(
            switch,
            Tip::new("Sonuç bir dosyaya ya da veritabanına yazılırken açılır"),
            iced::widget::tooltip::Position::Top,
        )
    };
    let note = if b.part {
        format!(
            "Çizimde {} tablosunun yalnız bir parçası var; açıkken bütün tablo okunur.",
            b.table
        )
    } else {
        format!("Çizimdeki nesneler yerine {} tablosu okunur.", b.table)
    };
    Some(
        row![
            switch,
            column![
                label::body("Bütün tablo (kaynaktan oku)"),
                label::caption(note).style(style::text::muted)
            ]
            .spacing(2)
            .width(Fill)
        ]
        .spacing(8)
        .align_y(Center)
        .into(),
    )
}

/// The run's flow on the other thread: its stop; how far it is is not shown (the line says it runs).
struct Flow(Arc<AtomicBool>);

impl BatchFlow for Flow {
    fn stopped(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    fn progress(&self, _done: u64, _total: u64) {}

    fn stop_flag(&self) -> Option<Arc<AtomicBool>> {
        Some(self.0.clone())
    }
}

const STOPPED: &str = "Durduruldu; sonuç yazılmadı.";

/// The run on a copy of the drawing: what it wrote, or why it did not, in the window's words.
pub(crate) fn run_to_sink(
    copy: &mut Document,
    input: &ProcessingRun,
    stop: Arc<AtomicBool>,
) -> Result<Ran, String> {
    if tools().is_none() {
        kentos_processing::command::register();
    }
    let Some(host) = tools() else {
        return Err("İşlem araçları bu ortamda yok.".to_owned());
    };
    match host.run(copy, input, true, &Flow(stop)) {
        Ok(RunEnd::Ran(ran, warnings)) => {
            let mut text = said(&ran);
            for w in warnings {
                text.push(' ');
                // A part of a table: the window's switch, not the channels' input (docs/adr/9354).
                if w.code == kentos_commands::codes::PARTIAL_SCOPE {
                    let head = w.message.split_once("; araç").map_or(w.message.as_str(), |(h, _)| h);
                    text.push_str(&format!(
                        "{head}; araç o parçayla çalıştı. Bütün tablo için girdinin altındaki Bütün tablo (kaynaktan oku) \
                         anahtarını açın."
                    ));
                } else {
                    text.push_str(&w.message);
                }
            }
            let written = ran.sink.as_ref().map_or_else(
                || Written { address: String::new(), file: false, tables: Vec::new(), added: false },
                |s| Written {
                    address: s.path.clone(),
                    file: !kentos_commands::is_database(&s.path),
                    tables: s.tables.iter().map(|t| (t.table.clone(), u64::from(t.rows))).collect(),
                    added: false,
                },
            );
            Ok(Ran {
                text,
                written,
                records_in: ran.record.total.records_in,
                records_out: ran.record.total.records_out,
            })
        }
        Ok(RunEnd::Cancelled) => Err(STOPPED.to_owned()),
        Err(e) => Err(refusal_words(&e, input)),
    }
}

/// A refusal in the window's words (docs/adr/9354): the channels' words name the input's fields (`overwrite`,
/// `sink.mode`); here the switch to turn on or the choice to make. The provider's code tells a file that is there from
/// a table that is.
fn refusal_words(e: &kentos_commands::CommandError, input: &ProcessingRun) -> String {
    use kentos_commands::codes;
    let path = input.sink.as_ref().map_or("", |s| s.path.as_str());
    let name = std::path::Path::new(path).file_name().map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned());
    // The table's name between the quotes of the channel's sentence.
    let table = e.message.split_once('“').and_then(|(_, r)| r.split_once('”')).map(|(t, _)| t.to_owned());
    match e.code.as_str() {
        codes::FILE_EXISTS => format!(
            "Bu GeoPackage zaten var: {name}. Başka dosya seçin ya da Varsa üzerine yaz'ı açın. Dosyaya dokunulmadı."
        ),
        codes::FILE_NOT_FOUND => format!(
            "{name} yok. Tablo eklenecek GeoPackage'ı seçin ya da Tablolar'da Yeni'yi seçin."
        ),
        codes::TABLE_EXISTS if e.message.contains("replaceTables") => {
            let table = table.unwrap_or_default();
            if kentos_commands::is_database(path) {
                format!(
                    "“{table}” tablosu veritabanında var. Tablolar'da Tabloyu değiştir'i seçin, başka bir tablo adı ya da \
                     şema yazın. Veritabanına dokunulmadı."
                )
            } else {
                format!(
                    "“{table}” tablosu {name} dosyasında var. Tablolar'da Tabloyu değiştir'i seçin ya da başka bir tablo \
                     adı yazın. Dosyaya dokunulmadı."
                )
            }
        }
        _ => e.message.clone(),
    }
}

/// What the line says of a run that wrote its result: the tool's summary, where (the file's name, or the database's
/// host and name), the tables with their rows, what was read from a source.
fn said(ran: &kentos_commands::ProcessingRan) -> String {
    let Some(sink) = &ran.sink else {
        return ran.summary.clone();
    };
    let tables: Vec<String> = sink
        .tables
        .iter()
        .map(|t| format!("{} ({} satır)", t.table, t.rows))
        .collect();
    let place = if kentos_commands::is_database(&sink.path) {
        let at = sink
            .path
            .split_once("://")
            .map_or(sink.path.as_str(), |(_, r)| r);
        let at = at.rsplit_once('@').map_or(at, |(_, h)| h);
        format!("{at} veritabanına")
    } else {
        let name = std::path::Path::new(&sink.path)
            .file_name()
            .map_or_else(|| sink.path.clone(), |n| n.to_string_lossy().into_owned());
        format!("{name} dosyasına")
    };
    let mut text = format!(
        "{} Sonuç {place} yazıldı: {}.",
        ran.summary,
        tables.join(", ")
    );
    let read: Vec<String> = ran
        .inputs
        .iter()
        .filter_map(|i| Some(format!("{} kayıt ({})", i.rows?, i.table.as_deref()?)))
        .collect();
    if !read.is_empty() {
        text.push_str(&format!(" Kaynaktan okunan: {}.", read.join(", ")));
    }
    text
}

/// The run as it is asked from the window (docs/adr/9354): its tool and values, its inputs as they resolve now, where it
/// writes.
pub(super) fn asked_of(window: &ToolDialog) -> Asked {
    let started = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    let mut words: Vec<String> = window
        .tool
        .parameters
        .iter()
        .filter(|p| matches!(p.kind, ParamKind::Features { .. }))
        .filter_map(|p| Some(format!("{}: {}", p.label, window.inputs.get(&p.name)?.description)))
        .collect();
    words.push(target_words(&window.sink));
    Asked {
        tool: window.tool.id.clone(),
        label: window.tool.label.clone(),
        values: kentos_processing::parameters::stored_values(&window.tool, &window.values),
        started: (started, std::time::Instant::now()),
        words,
        target: window.sink.target(),
    }
}

/// Where a run writes, in words: the file's name, or the database's host and the schema.
fn target_words(s: &LocalSink) -> String {
    match s.place {
        Place::Drawing => "Sonucun yeri: çizim".to_owned(),
        Place::File => {
            let path = s.path.trim();
            let name = std::path::Path::new(path).file_name().map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned());
            format!("Sonucun yeri: {name} (GeoPackage dosyası)")
        }
        Place::Database => {
            let at = s.address.trim();
            let at = at.split_once("://").map_or(at, |(_, r)| r);
            let at = at.rsplit_once('@').map_or(at, |(_, h)| h);
            format!("Sonucun yeri: {at}, şema {} (PostGIS veritabanı)", s.schema.trim())
        }
    }
}

/// The history record of a run to a file or a database: written (its tables) or refused (why), with its target.
fn sink_record(a: Asked, end: &Result<Ran, String>) -> kentos_processing::RunRecord {
    let mut words = a.words;
    let (status, summary, records_in, records_out) = match end {
        Ok(ran) => {
            let tables: Vec<String> = ran.written.tables.iter().map(|(t, n)| format!("{t} ({n} satır)")).collect();
            if !tables.is_empty() {
                words.push(format!("Yazılan tablolar: {}", tables.join(", ")));
            }
            (kentos_processing::Status::Ok, ran.text.clone(), ran.records_in, ran.records_out)
        }
        Err(why) if why == STOPPED => (kentos_processing::Status::Canceled, why.clone(), 0, 0),
        Err(why) => (kentos_processing::Status::Error, format!("Yazılmadı: {why}"), 0, 0),
    };
    kentos_processing::RunRecord {
        seq: 0,
        tool_id: a.tool,
        label: a.label,
        values: a.values,
        started: a.started.0,
        ms: a.started.1.elapsed().as_millis() as u64,
        status,
        summary,
        added: Vec::new(),
        touched: Vec::new(),
        target: Some(kentos_processing::Target::Client),
        records_in,
        records_out,
        steps: Vec::new(),
        words,
    }
}

/// The card heading the form after a run wrote its result to a file or a database (docs/adr/9354): where, its tables,
/// Katman olarak ekle and, for a file, Dosya konumunu aç.
pub(crate) fn written_card(w: &Written) -> Element<'_, Message> {
    use kentos_ui::icon::{Icon, Tone, icon};
    let place = if w.file {
        w.address.clone()
    } else {
        let at = w.address.split_once("://").map_or(w.address.as_str(), |(_, r)| r);
        at.rsplit_once('@').map_or(at, |(_, h)| h).to_owned()
    };
    let tables: Vec<String> = w.tables.iter().map(|(t, n)| format!("{t} ({n} satır)")).collect();
    let action = |text: &'static str, glyph: Icon, e: Option<SinkEvent>| {
        button(row![icon(glyph).size(14.0), label::caption(text)].spacing(5).align_y(Center))
            .padding(kentos_ui::theme::metrics::action_small())
            .style(style::button::secondary)
            .on_press_maybe(e.map(ev))
    };
    let mut actions = row![].spacing(6).align_y(Center);
    actions = actions.push(if w.added {
        action("Katman olarak eklendi", Icon::Success, None)
    } else {
        action("Katman olarak ekle", Icon::Layers, Some(SinkEvent::AddLayer))
    });
    if w.file {
        actions = actions.push(action("Dosya konumunu aç", Icon::Open, Some(SinkEvent::Reveal)));
    }
    container(
        column![
            row![
                icon(Icon::Success).size(14.0).tone(Tone::Success),
                label::caption(if w.file { "Sonuç dosyaya yazıldı" } else { "Sonuç veritabanına yazıldı" })
                    .font(kentos_ui::theme::typography::ui_strong()),
            ]
            .spacing(6)
            .align_y(Center),
            kentos_ui::widget::Pairs::new()
                .spacing_x(10.0)
                .spacing_y(2.0)
                .push(
                    label::caption(if w.file { "Dosya" } else { "Veritabanı" }).style(style::text::muted),
                    label::caption(place).wrapping(iced::widget::text::Wrapping::WordOrGlyph),
                )
                .push(label::caption("Tablolar").style(style::text::muted), label::caption(tables.join(", "))),
            actions,
        ]
        .spacing(8),
    )
    .padding(12)
    .width(Fill)
    .style(style::container::tile)
    .into()
}

impl App {
    pub(crate) fn processing_sink_event(&mut self, e: SinkEvent) -> Task<Message> {
        let Some(window) = &mut self.gis.processing.dialog else {
            return Task::none();
        };
        let s = &mut window.sink;
        // Another target: the card of what was written goes.
        if matches!(e, SinkEvent::Place(_) | SinkEvent::Path(_) | SinkEvent::Mode(_) | SinkEvent::Address(_) | SinkEvent::Schema(_)) {
            s.written = None;
        }
        match e {
            SinkEvent::Place(p) => s.place = p,
            SinkEvent::Path(t) => s.path = t,
            SinkEvent::Mode(m) => s.mode = m,
            SinkEvent::Overwrite(v) => s.overwrite = v,
            SinkEvent::Address(t) => s.address = t,
            SinkEvent::Schema(t) => s.schema = t,
            SinkEvent::Whole(p, on) => {
                if on {
                    s.whole.insert(p);
                } else {
                    s.whole.remove(&p);
                }
            }
            SinkEvent::ChoosePath => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Sonucun GeoPackage dosyası")
                            .add_filter("GeoPackage (.gpkg)", &["gpkg"])
                            .save_file()
                            .await
                            .map(|f| f.path().to_string_lossy().into_owned())
                    },
                    |path| match path {
                        Some(p) => ev(SinkEvent::Path(p)),
                        // Given up: nothing changes.
                        None => Message::Gis(gis::Event::Processing(Event::FileChosen(
                            String::new(),
                            None,
                        ))),
                    },
                );
            }
            SinkEvent::Ran(end) => {
                s.stop = None;
                let asked = s.asked.take();
                let record = asked.map(|a| (a.target.clone(), sink_record(a, &end)));
                window.status = match end {
                    Ok(ran) => {
                        s.written = Some(ran.written);
                        RunStatus::Ok {
                            text: ran.text,
                            pick: Vec::new(),
                            selected: false,
                            undo: false,
                            table: None,
                            warning: false,
                        }
                    }
                    Err(why) => {
                        s.written = None;
                        RunStatus::Error(why)
                    }
                };
                // İşlem geçmişi keeps it as it keeps a run on the drawing (docs/adr/9354).
                if let Some((target, r)) = record {
                    let seq = self.gis.processing.runner.add_record(r).seq;
                    self.gis.processing.sink_targets.insert(seq, target);
                }
            }
            SinkEvent::AddLayer => {
                let Some(w) = s.written.as_mut().filter(|w| !w.added) else {
                    return Task::none();
                };
                w.added = true;
                let (address, tables) = (w.address.clone(), w.tables.iter().map(|(t, _)| t.clone()).collect());
                return self.add_tables(address, tables);
            }
            SinkEvent::Reveal => {
                let Some(folder) = s
                    .written
                    .as_ref()
                    .filter(|w| w.file)
                    .and_then(|w| std::path::Path::new(&w.address).parent().map(std::path::Path::to_path_buf))
                else {
                    return Task::none();
                };
                if let Err(e) = crate::attachments::open_in_system(&folder) {
                    self.warn(format!("Dosyanın klasörü açılamadı: {e}. Klasör: {}", folder.display()));
                }
            }
        }
        Task::none()
    }

    /// The run Çalıştır asks for with the result to a file or a database: the window's values (the inputs read whole
    /// from their source as chosen), the objects chosen, the box on screen, the sink.
    pub(crate) fn processing_sink_input(&self) -> Option<ProcessingRun> {
        let (doc, window) = (
            self.document.as_ref()?,
            self.gis.processing.dialog.as_ref()?,
        );
        let parameters =
            window
                .sink
                .read_whole(command_values(&doc.model, &window.tool, &window.values));
        let selection: Vec<String> = self
            .selection
            .ids()
            .iter()
            .filter_map(|s| doc.model.uid(*s))
            .map(|u| u.to_string())
            .collect();
        let v = self.viewport.camera.visible_bounds();
        Some(ProcessingRun {
            environment: self.processing_environment().ok().flatten(),
            model: None,
            tool: window.tool.id.clone(),
            parameters,
            selection,
            view: Some([v.min_x, v.min_y, v.max_x, v.max_y]),
            sink: window.sink.sink(),
            // The drawing's path, so a file it links by a relative path (a point cloud's LAZ, a raster) is found from its
            // folder as the run on the drawing finds it (UR2-029).
            drawing: doc.path.as_ref().map(|p| p.display().to_string()),
        })
    }

    /// Çalıştır with the result to a file or a database: `cad.processing.run` on a copy of the drawing on another
    /// thread (`run_to_sink`).
    pub(crate) fn processing_sink_run(&mut self) -> Task<Message> {
        if let Err(why) = self.processing_environment() {
            if let Some(window) = &mut self.gis.processing.dialog {
                window.status = RunStatus::Invalid(why);
            }
            return Task::none();
        }
        let input = self.processing_sink_input();
        // A data layer among the inputs: the run reads the copy holding its rows (data_inputs.rs, docs/adr/9353), the
        // rows chosen in its table taken as chosen.
        let located = self.data_location();
        let (Some(doc), Some(window), Some(mut input)) =
            (&self.document, &mut self.gis.processing.dialog, input)
        else {
            return Task::none();
        };
        if let Some(why) = window.sink.problem() {
            window.status = RunStatus::Invalid(why.to_owned());
            return Task::none();
        }
        if let Some(l) = located.as_ref().filter(|l| !l.reading.is_empty()) {
            window.status =
                RunStatus::Invalid(format!("Kaynaktan okunuyor: {}. Satırlar okununca çalıştırın.", l.reading.join(", ")));
            return Task::none();
        }
        let mut copy = match &located {
            Some(l) => {
                input.selection.extend(l.chosen().iter().filter_map(|s| l.doc.uid(*s).map(|u| u.to_string())));
                l.doc.reading_copy()
            }
            None => doc.model.reading_copy(),
        };
        let stop = Arc::new(AtomicBool::new(false));
        window.sink.stop = Some(stop.clone());
        window.sink.written = None;
        window.reopen_note = None;
        window.sink.asked = Some(asked_of(window));
        window.status = RunStatus::Running {
            fraction: 0.0,
            label: if window.sink.whole.is_empty() {
                "Sonuç yazılıyor…".to_owned()
            } else {
                "Kaynaktan okunuyor ve sonuç yazılıyor…".to_owned()
            },
            background: true,
        };
        let (done, wait) = oneshot::channel();
        std::thread::spawn(move || {
            let _ = done.send(run_to_sink(&mut copy, &input, stop));
        });
        Task::perform(wait, |end| {
            ev(SinkEvent::Ran(
                end.unwrap_or_else(|_| Err(STOPPED.to_owned())),
            ))
        })
    }

    /// Durdur for a run to a file or a database: asked to stop; the line says it when the thread ends.
    pub(crate) fn processing_sink_stop(&mut self) -> bool {
        match self
            .gis
            .processing
            .dialog
            .as_ref()
            .and_then(|w| w.sink.stop.as_ref())
        {
            Some(stop) => {
                stop.store(true, Ordering::Relaxed);
                true
            }
            None => false,
        }
    }
}
