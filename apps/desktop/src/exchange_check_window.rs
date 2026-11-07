//! Devir denetimi (docs/adr/1660; `file.exchangeCheck`): the drawing taken to an exchange format and back by KentOS's
//! own writer and reader (`file.exchange.check`), read only. The form names the format and the tolerance; Denetle runs
//! the trip (a temporary file, removed). The answer keeps the two kinds of change apart: Sayısal (the vertices' offset
//! from the original, heights, measures, lengths, areas) and Anlamsal (what was not written, kinds changed, curves
//! become chords, heights, measures, attributes, style and ids lost), each loss a row with its count. Kayıplıları seç
//! selects the objects the losses name.

use iced::widget::{Column, container, row};
use iced::{Center, Element, Task};
use kentos_commands::{CallOp, CommandResult, ExchangeCheck, ExchangeChecked, ExchangeFormat};
use kentos_domain::{Slot, Uuid};
use kentos_interaction::Level;
use kentos_ui::label;
use kentos_ui::widget::{Dialog as Frame, Form};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, num, sheet};
use crate::style::fields;
use crate::traces::Control;

pub const TITLE: &str = "Devir denetimi";
const CLOSE: &str = "Kapat";
const RUN: &str = "Denetle";
const SELECT: &str = "Kayıplıları seç";

#[derive(Clone, Debug)]
pub struct State {
    pub format: ExchangeFormat,
    pub tolerance: String,
    pub answer: Option<Result<ExchangeChecked, String>>,
}

impl Default for State {
    fn default() -> Self {
        Self { format: ExchangeFormat::Gpkg, tolerance: "0,001".into(), answer: None }
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Format(String),
    Tolerance(String),
    Run,
    Select,
    Close,
}

fn msg(e: Event) -> Message {
    Message::ExchangeCheck(e)
}

fn key(f: ExchangeFormat) -> &'static str {
    match f {
        ExchangeFormat::Gpkg => "gpkg",
        ExchangeFormat::Parquet => "parquet",
        ExchangeFormat::Fgb => "fgb",
        ExchangeFormat::Gml => "gml",
        ExchangeFormat::Shp => "shp",
        ExchangeFormat::Dxf => "dxf",
    }
}

/// A difference as metres to the micrometre, comma decimal, no trailing zeros; below a micrometre said so.
fn small(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    if v.abs() < 1e-6 {
        return "< 0,000001".into();
    }
    let t = format!("{v:.6}");
    t.trim_end_matches('0').trim_end_matches('.').replace('.', ",")
}

/// An object kind in words.
fn kind_word(kind: &str) -> String {
    match kind {
        "point" => "Nokta",
        "line" => "Çizgi",
        "polyline" => "Çoklu çizgi",
        "polygon" => "Alan",
        "circle" => "Daire",
        "arc" => "Yay",
        "ellipse" => "Elips",
        "spline" => "Eğri",
        "nurbs" => "NURBS",
        "text" => "Yazı",
        "dimension" => "Ölçü",
        "hatch" => "Tarama",
        "insert" => "Blok",
        "leader" => "Kılavuz",
        "table" => "Tablo",
        "image" => "Resim",
        "raster" => "Raster",
        other => return other.to_owned(),
    }
    .to_owned()
}

/// A loss's detail in words: kinds by their Turkish names, a style by its own, an attribute as it is named.
fn detail_words(loss: &str, detail: &str) -> String {
    match loss {
        "notWritten" | "chorded" | "extra" => kind_word(detail),
        "kindChanged" => detail.split(" → ").map(kind_word).collect::<Vec<_>>().join(" → "),
        "styleLost" => match detail {
            "color" => "Renk".into(),
            "lineWeight" => "Çizgi kalınlığı".into(),
            "symbol" => "Sembol".into(),
            other => other.into(),
        },
        _ => detail.to_owned(),
    }
}

/// A loss's kind in words.
pub fn loss_word(kind: &str) -> &'static str {
    match kind {
        "notWritten" => "Yazılmadı",
        "kindChanged" => "Türü değişti",
        "chorded" => "Eğri kirişe döndü",
        "zLost" => "Kot kayboldu",
        "zAdded" => "Kot eklendi",
        "mLost" => "Ölçü (M) kayboldu",
        "attrLost" => "Öznitelik kayboldu",
        "attrChanged" => "Öznitelik değişti",
        "attrAdded" => "Öznitelik eklendi",
        "styleLost" => "Nesne stili kayboldu",
        "identityLost" => "Kalıcı kimlik korunmadı",
        "extra" => "Fazla geri okunan",
        _ => "—",
    }
}

impl App {
    pub(crate) fn open_exchange_check(&mut self) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return Task::none();
        }
        self.exchange_check = State::default();
        self.dialog = Some(Dialog::ExchangeCheck);
        Task::none()
    }

    /// Denetle: the trip on the open drawing (a test calls it as the button does).
    pub(crate) fn exchange_check_run(&mut self) {
        let t = self.exchange_check.tolerance.trim().replace(',', ".");
        let tolerance = if t.is_empty() { None } else { t.parse::<f64>().ok() };
        if !t.is_empty() && tolerance.is_none() {
            self.exchange_check.answer = Some(Err(format!("Tolerans “{}” bir sayı değil.", self.exchange_check.tolerance)));
            return;
        }
        let Some(doc) = self.document.as_ref() else { return };
        let input = ExchangeCheck { format: self.exchange_check.format, layers: Vec::new(), tolerance };
        self.exchange_check.answer = Some(match kentos_headless::exchange_check::run(&doc.model, &input, CallOp::Execute) {
            CommandResult::Completed { output, .. } => serde_json::from_value(output).map_err(|e| e.to_string()),
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err(error.message)
            }
            _ => Err("Denetlenemedi.".into()),
        });
    }

    pub(crate) fn exchange_check_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Format(v) => {
                if let Some(f) = ExchangeFormat::ALL.into_iter().find(|f| key(*f) == v) {
                    self.exchange_check.format = f;
                    self.exchange_check.answer = None;
                }
            }
            Event::Tolerance(v) => self.exchange_check.tolerance = v,
            Event::Run => self.exchange_check_run(),
            Event::Select => {
                let Some(Ok(r)) = &self.exchange_check.answer else { return Task::none() };
                let Some(doc) = self.document.as_ref() else { return Task::none() };
                let mut slots: Vec<Slot> = r
                    .losses
                    .iter()
                    .filter(|l| l.kind != "identityLost")
                    .flat_map(|l| l.uids.iter())
                    .filter_map(|u| Uuid::parse_str(u).ok().and_then(|u| doc.model.slot_of(u)))
                    .collect();
                slots.sort_by_key(|s| s.0);
                slots.dedup();
                let n = slots.len();
                self.selection.set(slots);
                self.exchange_check = State::default();
                self.dialog = None;
                self.say(Level::Info, format!("{TITLE}: kaybı olan {n} nesne seçildi."));
            }
            Event::Close => {
                self.exchange_check = State::default();
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn exchange_check_view(&self) -> Element<'_, Message> {
        let s = &self.exchange_check;
        let f = self.format();
        let formats: Vec<(String, String)> =
            ExchangeFormat::ALL.iter().map(|x| (key(*x).to_owned(), x.title().to_owned())).collect();
        let form = Form::new()
            .label_width(110.0)
            .field(
                "Biçim",
                row![
                    container(fields::select_owned(formats, key(s.format), |v| msg(Event::Format(v)))).width(200),
                    label::caption("Tolerans (m)"),
                    container(fields::input(None, "0,001", &s.tolerance, false, false).on_input(|v| msg(Event::Tolerance(v))))
                        .width(100)
                ]
                .spacing(10)
                .align_y(Center),
            );
        let mut lines = Vec::new();
        let mut rows = Vec::new();
        let mut lost = 0;
        match &s.answer {
            Some(Ok(r)) => {
                let not: u32 = r.not_written.iter().map(|k| k.count).sum();
                lines.push(words::text_line(
                    if not == 0 { Kind::Ok } else { Kind::Warn },
                    format!(
                        "{} nesneden {} geri okundu ve eşlendi; {} yazılmadı{}.",
                        r.objects,
                        r.matched,
                        not,
                        if r.not_written.is_empty() {
                            String::new()
                        } else {
                            format!(
                                " ({})",
                                r.not_written.iter().map(|k| format!("{} {}", k.count, kind_word(&k.kind).to_lowercase())).collect::<Vec<_>>().join(", ")
                            )
                        }
                    ),
                ));
                let n = &r.numeric;
                let opt = |v: Option<f64>| v.map_or("—".to_owned(), small);
                lines.push(words::text_line(
                    if n.above == 0 { Kind::Ok } else { Kind::Error },
                    format!(
                        "Sayısal: en büyük köşe kayması {} m (tolerans {}; aşan {}), kot {} m, ölçü {}, uzunluk {} m, alan {} m².",
                        opt(n.offset),
                        f.length(r.tolerance),
                        n.above,
                        opt(n.height),
                        opt(n.measure),
                        opt(n.length),
                        opt(n.area)
                    ),
                ));
                for w in r.writer.iter().chain(&r.reader) {
                    lines.push(words::text_line(Kind::Info, w.clone()));
                }
                lines.push(words::text_line(
                    Kind::Info,
                    "Sonuç KentOS'un yazıcısıyla okuyucusunun birlikte yaptığıdır; başka bir programın okuduğu kendisinindir. Dosya geçicidir, silindi.",
                ));
                for l in &r.losses {
                    if l.kind != "identityLost" && l.kind != "extra" {
                        lost += l.uids.len();
                    }
                    rows.push(vec![
                        loss_word(&l.kind).to_owned(),
                        l.detail.as_deref().map(|d| detail_words(&l.kind, d)).unwrap_or_default(),
                        l.count.to_string(),
                    ]);
                }
            }
            Some(Err(why)) => lines.push(words::text_line(Kind::Error, why.clone())),
            None => lines.push(words::text_line(
                Kind::Info,
                "Denetle çizimi seçilen biçime yazar ve KentOS'un okuyucusuyla geri okur; sayısal sapmayı ve anlamsal kayıpları ayrı söyler.",
            )),
        }
        let table = sheet(
            vec![col("Anlamsal kayıp", 220.0), col("Ayrıntı", 220.0), num("Nesne", 80.0)],
            rows,
            "Anlamsal kayıp yok.",
            260.0,
        );
        let body = Column::new().spacing(12).push(form).push(words::summary(lines)).push(table);
        Frame::new(TITLE)
            .push(body)
            .aside(words::secondary(SELECT, (lost > 0).then(|| msg(Event::Select))))
            .action(words::secondary(CLOSE, Some(msg(Event::Close))))
            .action(words::primary(RUN, Some(msg(Event::Run))))
            .width(760.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn exchange_check_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(RUN) => Some(msg(Event::Run)),
            Control::Press(SELECT) => Some(msg(Event::Select)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
