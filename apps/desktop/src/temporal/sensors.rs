//! Sensör gözlemleri (docs/adr/0646 §6): a sensor layer's assets, each with its latest value, quality, moment and age
//! by `cad.sensors.latest`; an asset older than Eskime süresi is shown as a stopped stream (“Akış kesik”), not as
//! current. The moment asked is the time slider's while it is open (and its Bilinen gün), else now. Gözlem al… takes
//! a SensorThings Observations file in through `cad.sensors.ingest` (one step “Sensör gözlemleri”), by the duplicate
//! and late policies chosen, and says what it wrote and why a row was not.

use std::path::PathBuf;

use iced::widget::{Column, container, row, scrollable, text, text_input};
use iced::{Element, Fill, Task};
use kentos_commands::{
    CommandResult, DuplicatePolicy, LatePolicy, LayerNodeType, SensorsIngest, SensorsIngested,
    SensorsLatest, SensorsLatestAnswer,
};
use kentos_geometry_core::time::{self, Window as TimeWindow};
use kentos_native_application::{ExecutionContext, sensors_ingest, sensors_latest};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog as Frame, focus_ring, overlay};

use super::{Event as TimeEvent, msg as time_msg};
use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Sensör gözlemleri";
const NONE: &str = "Yok";
const DUPLICATES: [&str; 3] = ["İlki kalır", "Sonuncusu geçer", "Hepsi yazılır"];
const LATE: [&str; 2] = ["Kabul et", "Reddet"];

/// The window: the layer and fields chosen, the policies, the answer.
#[derive(Debug, Clone)]
pub struct Window {
    layer: String,
    value: String,
    /// Empty: none.
    quality: String,
    /// Eskime süresi in minutes, as typed.
    stale: String,
    /// The moment asked, as typed; empty: the slider's while it is open, else now.
    at: String,
    /// The asset a file's observations are of, as typed.
    asset: String,
    duplicates: usize,
    late: usize,
    answer: Option<Result<SensorsLatestAnswer, String>>,
    status: Option<(bool, String)>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Layer(usize),
    Value(usize),
    Quality(usize),
    Stale(String),
    At(String),
    Asset(String),
    Duplicates(usize),
    Late(usize),
    Take,
    Taken(Option<PathBuf>),
    Close,
}

fn msg(e: Event) -> Message {
    time_msg(TimeEvent::Sensors(e))
}

/// An age in words: “45 sn”, “25 dk”, “3 sa 5 dk”, “2 gün”.
fn age_words(seconds: f64) -> String {
    let s = seconds.max(0.0).round() as i64;
    match s {
        0..60 => format!("{s} sn"),
        60..3600 => format!("{} dk", s / 60),
        3600..86_400 => {
            let m = (s % 3600) / 60;
            if m == 0 {
                format!("{} sa", s / 3600)
            } else {
                format!("{} sa {m} dk", s / 3600)
            }
        }
        _ => format!("{} gün", s / 86_400),
    }
}

/// A moment written for the eye: 01.06.2024 10:20.
fn moment_words(utc: &str) -> String {
    match time::read(utc) {
        time::Read::Moment(t) => time::show(t, time::Unit::Minute),
        _ => utc.to_owned(),
    }
}

impl App {
    /// The layers a sensor's observations go on: layers of records (no object with a geometry) whose time names the
    /// asset field.
    fn sensor_layers(&self) -> Vec<String> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        doc.model
            .layers()
            .leaves()
            .into_iter()
            .filter(|n| n.kind == LayerNodeType::Layer)
            .filter(|n| {
                n.time
                    .as_ref()
                    .is_some_and(|t| t.key.as_deref().is_some_and(|k| !k.trim().is_empty()))
            })
            .filter(|n| doc.model.by_layer(&n.id).all(|e| !e.has_geometry()))
            .map(|n| n.id.clone())
            .collect()
    }

    /// The fields a layer's value can be read from: its attributes but the time's own.
    fn sensor_fields(&self, layer: &str) -> Vec<String> {
        let own: Vec<String> = self
            .document
            .as_ref()
            .and_then(|d| d.model.layers().get(layer).and_then(|n| n.time.clone()))
            .map(|t| {
                [
                    Some(t.start),
                    t.end,
                    t.key,
                    t.recorded_start,
                    t.recorded_end,
                ]
                .into_iter()
                .flatten()
                .collect()
            })
            .unwrap_or_default();
        self.attribute_names(layer)
            .into_iter()
            .filter(|n| !own.contains(n))
            .collect()
    }

    fn sensor_window(&self, layer: String) -> Window {
        let fields = self.sensor_fields(&layer);
        let looks = |words: &[&str]| {
            fields
                .iter()
                .find(|f| words.iter().any(|w| f.to_lowercase().contains(w)))
                .cloned()
        };
        Window {
            value: looks(&["deger", "değer", "value", "olcum", "ölçüm"])
                .or_else(|| fields.first().cloned())
                .unwrap_or_default(),
            quality: looks(&["kalite", "quality"]).unwrap_or_default(),
            layer,
            stale: "60".into(),
            at: String::new(),
            asset: String::new(),
            duplicates: 0,
            late: 0,
            answer: None,
            status: None,
        }
    }

    /// Sensör gözlemleri (`time.sensors`): the window over the first sensor layer (the active one when it is one).
    pub(crate) fn open_sensors(&mut self) {
        let layers = self.sensor_layers();
        if layers.is_empty() {
            self.warn(
                "Çizimde sensör katmanı yok: zaman ayarında kimlik alanı olan bir kayıt katmanı gerekir (Zaman ayarları).",
            );
            return;
        }
        let active = self
            .document
            .as_ref()
            .map(|d| d.model.layers().active().to_owned())
            .unwrap_or_default();
        let layer = if layers.contains(&active) {
            active
        } else {
            layers[0].clone()
        };
        let mut w = self.sensor_window(layer);
        w.answer = Some(self.sensors_ask(&w));
        self.time.sensors = Some(w);
        self.dialog = Some(Dialog::Sensors);
    }

    /// The latest of each asset as the window asks it: the slider's moment and day known while it is open.
    fn sensors_ask(&self, w: &Window) -> Result<SensorsLatestAnswer, String> {
        let Some(doc) = &self.document else {
            return Err("Açık çizim yok.".into());
        };
        let s = &self.time.slider;
        let utc = |t: f64| {
            let w = time::write(t, false);
            if w.contains('T') { format!("{w}Z") } else { w }
        };
        let typed = w.at.trim();
        let at = if !typed.is_empty() {
            Some(typed.to_owned())
        } else {
            s.window().map(|win| match win {
                TimeWindow::Instant(a) => utc(a),
                // A period asks its last moment: the latest observation within it.
                TimeWindow::Range(_, b) => utc(b - 1.0),
            })
        };
        let recorded = doc
            .model
            .layers()
            .get(&w.layer)
            .and_then(|n| n.time.as_ref())
            .is_some_and(|t| t.recorded());
        let stale = w
            .stale
            .trim()
            .replace(',', ".")
            .parse::<f64>()
            .ok()
            .filter(|m| m.is_finite() && *m >= 0.0)
            .map(|m| m * 60.0);
        let input = SensorsLatest {
            layer: w.layer.clone(),
            value_field: w.value.clone(),
            quality_field: (!w.quality.is_empty()).then(|| w.quality.clone()),
            at,
            known_at: if recorded && s.open {
                s.known_at.map(utc)
            } else {
                None
            },
            stale_after: stale,
            expected_assets: Vec::new(),
        };
        let mut model = doc.model.clone();
        match sensors_latest::execute(&mut ExecutionContext::new(&mut model), input) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                Err(error.message)
            }
            _ => Err("Sorulamadı.".into()),
        }
    }

    /// The window asked again (the drawing or the slider changed).
    pub(crate) fn sensors_refresh(&mut self) {
        if let Some(mut w) = self.time.sensors.clone() {
            w.answer = Some(self.sensors_ask(&w));
            self.time.sensors = Some(w);
        }
    }

    pub(crate) fn sensors_event(&mut self, event: Event) -> Task<Message> {
        let Some(mut w) = self.time.sensors.clone() else {
            return Task::none();
        };
        match event {
            Event::Layer(i) => {
                if let Some(id) = self.sensor_layers().get(i).cloned() {
                    let (stale, at) = (w.stale.clone(), w.at.clone());
                    w = self.sensor_window(id);
                    w.stale = stale;
                    w.at = at;
                }
            }
            Event::Value(i) => {
                if let Some(f) = self.sensor_fields(&w.layer).get(i) {
                    w.value = f.clone();
                }
            }
            Event::Quality(i) => {
                w.quality = if i == 0 {
                    String::new()
                } else {
                    self.sensor_fields(&w.layer)
                        .get(i - 1)
                        .cloned()
                        .unwrap_or_default()
                };
            }
            Event::Stale(t) => w.stale = t,
            Event::At(t) => w.at = t,
            Event::Asset(t) => w.asset = t,
            Event::Duplicates(i) => w.duplicates = i,
            Event::Late(i) => w.late = i,
            Event::Take => {
                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("SensorThings gözlemleri")
                            .add_filter("SensorThings JSON", &["json"])
                            .pick_file()
                            .await?;
                        Some(file.path().to_path_buf())
                    },
                    |p| msg(Event::Taken(p)),
                );
            }
            Event::Taken(None) => return Task::none(),
            Event::Taken(Some(path)) => {
                w.status = Some(self.sensors_take(&w, path));
            }
            Event::Close => {
                self.time.sensors = None;
                self.dialog = None;
                return Task::none();
            }
        }
        w.answer = Some(self.sensors_ask(&w));
        self.time.sensors = Some(w);
        Task::none()
    }

    /// A SensorThings file taken in: what was written, or why nothing.
    fn sensors_take(&mut self, w: &Window, path: PathBuf) -> (bool, String) {
        let Some(doc) = &mut self.document else {
            return (true, "Açık çizim yok.".into());
        };
        let input = SensorsIngest {
            layer: w.layer.clone(),
            value_field: w.value.clone(),
            quality_field: (!w.quality.is_empty()).then(|| w.quality.clone()),
            observations: None,
            file: Some(path.display().to_string()),
            url: None,
            asset: Some(w.asset.trim().to_owned()),
            pages: None,
            received_at: None,
            duplicates: [
                DuplicatePolicy::First,
                DuplicatePolicy::Last,
                DuplicatePolicy::All,
            ][w.duplicates],
            late: [LatePolicy::Accept, LatePolicy::Reject][w.late],
            late_field: None,
            expected_revision: None,
        };
        let result = sensors_ingest::execute(&mut ExecutionContext::new(&mut doc.model), input);
        match result {
            CommandResult::Completed { output, .. } => (false, taken_words(&output)),
            CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                (true, error.message)
            }
            _ => (true, "Gözlemler alınamadı.".into()),
        }
    }

    pub(crate) fn sensors_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.time.sensors else {
            return text("").into();
        };
        let layers = self.sensor_layers();
        let fields = self.sensor_fields(&w.layer);
        let layer = Select::new(
            layers
                .iter()
                .map(|id| Choice::new(self.layer_path(id)))
                .collect::<Vec<_>>(),
            layers.iter().position(|id| *id == w.layer),
            |i| msg(Event::Layer(i)),
        );
        let value = Select::new(
            fields
                .iter()
                .map(|f| Choice::new(f.clone()))
                .collect::<Vec<_>>(),
            fields.iter().position(|f| *f == w.value),
            |i| msg(Event::Value(i)),
        )
        .placeholder("Seçin");
        let quality = Select::new(
            std::iter::once(Choice::new(NONE))
                .chain(fields.iter().map(|f| Choice::new(f.clone())))
                .collect::<Vec<_>>(),
            if w.quality.is_empty() {
                Some(0)
            } else {
                fields.iter().position(|f| *f == w.quality).map(|i| i + 1)
            },
            |i| msg(Event::Quality(i)),
        );
        let stale = focus_ring(
            text_input("60", &w.stale)
                .on_input(|t| msg(Event::Stale(t)))
                .padding([5, 8])
                .size(typography::body())
                .style(style::field::input),
        );
        let at = focus_ring(
            text_input(
                if self.time.slider.open {
                    "Zaman sürgüsünün anı"
                } else {
                    "Şimdi"
                },
                &w.at,
            )
            .on_input(|t| msg(Event::At(t)))
            .padding([5, 8])
            .size(typography::body())
            .style(style::field::input),
        );
        let mut body = Column::new()
            .spacing(12)
            .push(
                row![
                    container(words::field("Katman", layer, None)).width(Fill),
                    container(words::field("Değer alanı", value, None)).width(Fill),
                    container(words::field("Kalite alanı", quality, None)).width(Fill),
                ]
                .spacing(10),
            )
            .push(
                row![
                    container(words::field(
                        "Sorulan an",
                        at,
                        Some("Boşsa Zaman sürgüsü açıkken onun anı, değilse şimdi.".to_owned()),
                    ))
                    .width(Fill),
                    container(words::field(
                        "Eskime süresi (dk)",
                        stale,
                        Some("Bundan eski son değer “Akış kesik” gösterilir.".to_owned()),
                    ))
                    .width(Fill),
                ]
                .spacing(10),
            );
        let asked = "Sorulan an";
        match &w.answer {
            Some(Ok(a)) => {
                body = body.push(label::caption(format!(
                    "{asked}: {} · {} varlık, {} akış kesik{}",
                    moment_words(&a.at),
                    a.assets.len(),
                    a.stale,
                    if a.unreadable > 0 {
                        format!(", {} kayıt okunamadı (varlığı ya da anı yok)", a.unreadable)
                    } else {
                        String::new()
                    }
                )));
                body = body.push(sensor_table(a));
            }
            Some(Err(why)) => body = body.push(words::text_line(Kind::Warn, why.clone())),
            None => {}
        }
        let asset = focus_ring(
            text_input("S1", &w.asset)
                .on_input(|t| msg(Event::Asset(t)))
                .padding([5, 8])
                .size(typography::body())
                .style(style::field::input),
        );
        let duplicates = Select::new(
            DUPLICATES
                .iter()
                .map(|d| Choice::new(*d))
                .collect::<Vec<_>>(),
            Some(w.duplicates),
            |i| msg(Event::Duplicates(i)),
        );
        let late = Select::new(
            LATE.iter().map(|d| Choice::new(*d)).collect::<Vec<_>>(),
            Some(w.late),
            |i| msg(Event::Late(i)),
        );
        body = body.push(label::heading("Gözlem al"));
        body = body.push(
            row![
                container(words::field(
                    "Varlık",
                    asset,
                    Some("Dosyadaki gözlemlerin sensörü ya da sayacı.".to_owned())
                ))
                .width(Fill),
                container(words::field("Aynı andaki gözlem", duplicates, None)).width(Fill),
                container(words::field("Geç gelen gözlem", late, None)).width(Fill),
            ]
            .spacing(10),
        );
        if let Some((error, said)) = &w.status {
            body = body.push(words::text_line(
                if *error { Kind::Error } else { Kind::Ok },
                said.clone(),
            ));
        }
        overlay::modal(
            Frame::new(TITLE)
                .push(body)
                .action(words::secondary(
                    "Gözlem al…",
                    (!w.asset.trim().is_empty() && !w.value.is_empty()).then(|| msg(Event::Take)),
                ))
                .action(words::primary("Kapat", Some(msg(Event::Close))))
                .width(760.0),
            msg(Event::Close),
        )
    }
}

/// What an ingest wrote, in a sentence.
fn taken_words(o: &SensorsIngested) -> String {
    let mut parts = vec![format!("{} gözlem yazıldı", o.added.len())];
    if !o.replaced.is_empty() {
        parts.push(format!(
            "{} eski gözlemin yerine yenisi geçti",
            o.replaced.len()
        ));
    }
    if o.skipped > 0 {
        parts.push(format!("{} yinelenen atlandı", o.skipped));
    }
    if o.late > 0 {
        parts.push(format!("{} geç gelen", o.late));
    }
    let mut out = parts.join(", ") + ".";
    if let Some(first) = o.rejected.first() {
        out.push_str(&format!(
            " {} satır yazılmadı; ilki ({}. satır): {}",
            o.rejected.len(),
            first.index + 1,
            first.message
        ));
    }
    if o.more {
        out.push_str(" Dosya bir sonraki sayfayı adlandırıyor; o sayfa okunmadı.");
    }
    out
}

/// The assets' latest: Varlık, Değer, Kalite, An, Yaş, Durum; a stale one in the warning's colour.
fn sensor_table(a: &SensorsLatestAnswer) -> Element<'_, Message> {
    let cell = |s: String, w: f32| container(label::body(s)).width(typography::scaled(w));
    let head = row![
        container(label::caption("Varlık")).width(typography::scaled(110.0)),
        container(label::caption("Değer")).width(Fill),
        container(label::caption("Kalite")).width(typography::scaled(90.0)),
        container(label::caption("An")).width(typography::scaled(130.0)),
        container(label::caption("Yaş")).width(typography::scaled(80.0)),
        container(label::caption("Durum")).width(typography::scaled(140.0)),
    ]
    .spacing(8);
    let mut rows = Column::new().spacing(4);
    if a.assets.is_empty() {
        rows = rows.push(words::text_line(
            Kind::Info,
            "Bu anda hiçbir varlığın gözlemi yok.",
        ));
    }
    for s in &a.assets {
        let state = if s.stale {
            words::text_line(
                Kind::Warn,
                format!("Akış kesik: {}", age_words(s.age_seconds)),
            )
        } else {
            words::text_line(Kind::Ok, "Güncel")
        };
        rows = rows.push(
            row![
                cell(s.asset.clone(), 110.0),
                container(label::body(s.value.clone().unwrap_or_else(|| "—".into()))).width(Fill),
                cell(s.quality.clone().unwrap_or_else(|| "—".into()), 90.0),
                cell(moment_words(&s.observed_at), 130.0),
                cell(age_words(s.age_seconds), 80.0),
                container(state).width(typography::scaled(140.0)),
            ]
            .spacing(8),
        );
    }
    Column::new()
        .spacing(6)
        .push(head)
        .push(container(scrollable(rows)).max_height(typography::scaled(220.0)))
        .into()
}
