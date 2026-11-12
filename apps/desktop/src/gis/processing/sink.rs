//! Sonuç and Bütün tablo (docs/adr/0592 §3): where a tool run here writes its result, and an input on a bound layer
//! read from its source. “Çizime” is the drawing as ever; “GeoPackage dosyasına” and “PostGIS veritabanına” run
//! `cad.processing.run` with a sink on a copy of the drawing on another thread, the drawing unchanged, Durdur stopping
//! it between parts (nothing of the result is then written). An input on a layer bound to a table can be read whole
//! from its source (“Bütün tablo”) when the result does not go to the drawing: the part the drawing holds is said in
//! the switch's note. The window's line says what was written where.

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
pub(crate) enum Place {
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
    /// The run on the other thread ended: what the line says, or why it did not.
    Ran(Result<String, String>),
}

fn ev(e: SinkEvent) -> Message {
    Message::Processing(Event::Sink(e))
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
            tables: false,
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

/// The run on a copy of the drawing: what the window's line says, or why it did not write.
pub(crate) fn run_to_sink(
    copy: &mut Document,
    input: &ProcessingRun,
    stop: Arc<AtomicBool>,
) -> Result<String, String> {
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
                text.push_str(&w.message);
            }
            Ok(text)
        }
        Ok(RunEnd::Cancelled) => Err(STOPPED.to_owned()),
        Err(e) => Err(e.message),
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

impl App {
    pub(crate) fn processing_sink_event(&mut self, e: SinkEvent) -> Task<Message> {
        let Some(window) = &mut self.processing.dialog else {
            return Task::none();
        };
        let s = &mut window.sink;
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
                        None => Message::Processing(Event::FileChosen(String::new(), None)),
                    },
                );
            }
            SinkEvent::Ran(end) => {
                s.stop = None;
                window.status = match end {
                    Ok(text) => RunStatus::Ok {
                        text,
                        pick: Vec::new(),
                        selected: false,
                        undo: false,
                        table: None,
                    },
                    Err(why) => RunStatus::Error(why),
                };
            }
        }
        Task::none()
    }

    /// The run Çalıştır asks for with the result to a file or a database: the window's values (the inputs read whole
    /// from their source as chosen), the objects chosen, the box on screen, the sink.
    pub(crate) fn processing_sink_input(&self) -> Option<ProcessingRun> {
        let (doc, window) = (self.document.as_ref()?, self.processing.dialog.as_ref()?);
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
            tool: window.tool.id.clone(),
            parameters,
            selection,
            view: Some([v.min_x, v.min_y, v.max_x, v.max_y]),
            sink: window.sink.sink(),
            drawing: None,
        })
    }

    /// Çalıştır with the result to a file or a database: `cad.processing.run` on a copy of the drawing on another
    /// thread (`run_to_sink`).
    pub(crate) fn processing_sink_run(&mut self) -> Task<Message> {
        if let Err(why) = self.processing_environment() {
            if let Some(window) = &mut self.processing.dialog {
                window.status = RunStatus::Invalid(why);
            }
            return Task::none();
        }
        let input = self.processing_sink_input();
        let (Some(doc), Some(window), Some(input)) =
            (&self.document, &mut self.processing.dialog, input)
        else {
            return Task::none();
        };
        if let Some(why) = window.sink.problem() {
            window.status = RunStatus::Invalid(why.to_owned());
            return Task::none();
        }
        let mut copy = doc.model.reading_copy();
        let stop = Arc::new(AtomicBool::new(false));
        window.sink.stop = Some(stop.clone());
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
