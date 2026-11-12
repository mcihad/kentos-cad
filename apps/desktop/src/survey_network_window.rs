//! GNSS ağ dengelemesi (docs/adr/1740; `map.survey.adjustment`; named “Arşivden ağ dengelemesi” until 8 November 2026): the project's GNSS network adjusted from its
//! archive by `map.survey.network`, reading only until Kabul et. Baz içe aktar brings a GVX 1.0 file's or KentOS's
//! baseline text's vectors into the last arazi ölçümü (`map.survey.edit baselines`); the known point form adds a mark's
//! given coordinates (`known`); the class (BÖHHBÜY 2018's C1, C2, C3, or none) and the variance factor are the user's.
//! The summary says both stages' m0 and model test, the blunders, the class's result and whether the network can be
//! accepted; the tables list the marks (held, weighted or new, σ north/east/up, how far an accepted point moves), the
//! baselines (length, residual, w, status; Dışla leaves one out with its test as the reason, Geri al takes it back)
//! and the known points. Kabul et writes it (`map.survey.edit network`): one undo step. Harita only.
//!
//! The measurement epoch (docs/adr/2120): typed, or the mean of the baselines' observation times; the known points at
//! another epoch are moved to it (BÖHHBÜY Madde 15(1)(a)) by their own velocity when the form gave one (Earth-centred
//! VX, VY, VZ, m/yr, docs/adr/2520), else along the campaign's velocity grid, said in the summary and in Bilinen
//! noktalar. With three known points or more the summary says BÖHHBÜY Madde 15(1)(c)'s similarity (the
//! scale, its test and the class's limit) and Bilinen noktalar its residuals (docs/adr/2160).
//!
//! Docs/adr/4920: Oturum korelasyonları adjusts each baseline session's baselines by their full covariance (`correlations:
//! session`), Yeni noktaların hızı reads each new mark's velocity from the campaign's velocity grid; the summary says
//! which correlations went in and from which grid, Noktalar the velocities, Bazlar each baseline's session.

use std::path::PathBuf;

use iced::widget::{container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    CommandResult, KnownMovedBy, MapSurveyNetwork, MapSurveyNetworked, MapSurveyOperation as Op, NetworkCorrelations,
    NetworkEpochFrom, NetworkStage,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, map_survey_network};
use kentos_ui::label;
use kentos_ui::widget::grow::Grows;
use kentos_ui::widget::tree_view::Check;
use kentos_ui::widget::{Dialog as Frame, Form, NumberInput, Segmented};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{self, col, num, sheet};
use crate::style::fields;
use crate::traces::Control;

pub const TITLE: &str = "GNSS ağ dengelemesi";
const CLOSE: &str = "Kapat";
const ACCEPT: &str = "Kabul et";
const IMPORT: &str = "Baz içe aktar…";
const ADD: &str = "Bilinen nokta ekle";
const CORRELATED: &str = "Oturum korelasyonları";
const VELOCITIES: &str = "Yeni noktaların hızı";

/// What the table shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum View {
    #[default]
    Marks,
    Baselines,
    Known,
    /// docs/adr/5320: the baseline sessions, each block's correlations.
    Sessions,
}

impl std::fmt::Display for View {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            View::Marks => "Noktalar",
            View::Baselines => "Bazlar",
            View::Known => "Bilinen noktalar",
            View::Sessions => "Oturumlar",
        })
    }
}

/// The rule class checked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Class {
    #[default]
    None,
    C1,
    C2,
    C3,
}

impl Class {
    fn id(self) -> Option<String> {
        match self {
            Class::None => None,
            Class::C1 => Some("C1".into()),
            Class::C2 => Some("C2".into()),
            Class::C3 => Some("C3".into()),
        }
    }
}

impl std::fmt::Display for Class {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Class::None => "Sınıf yok",
            Class::C1 => "C1",
            Class::C2 => "C2",
            Class::C3 => "C3",
        })
    }
}

/// The known point form's boxes, as typed.
#[derive(Clone, Debug, PartialEq)]
pub struct KnownForm {
    pub mark: String,
    pub lat: String,
    pub lon: String,
    pub height: String,
    pub srid: String,
    pub epoch: String,
    pub sigma: [String; 3],
    /// Earth-centred VX, VY, VZ (m/yr), as typed (docs/adr/2520).
    pub velocity: [String; 3],
    pub source: String,
}

impl Default for KnownForm {
    fn default() -> Self {
        KnownForm {
            mark: String::new(),
            lat: String::new(),
            lon: String::new(),
            height: String::new(),
            srid: "5252".into(),
            epoch: String::new(),
            sigma: Default::default(),
            velocity: Default::default(),
            source: String::new(),
        }
    }
}

/// A box of the known point form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Mark,
    Lat,
    Lon,
    Height,
    Srid,
    Epoch,
    Sigma(usize),
    Velocity(usize),
    Source,
}

#[derive(Clone, Debug, Default)]
pub struct State {
    pub view: View,
    pub class: Class,
    pub scale: f64,
    /// The measurement epoch as typed; empty: the baselines' observation times.
    pub epoch: String,
    pub known: KnownForm,
    pub answer: Option<Result<MapSurveyNetworked, String>>,
    /// Kabul et draws each point's error ellipse, magnified this many times (docs/adr/2960); and its height bar
    /// (docs/adr/3360).
    pub ellipses: bool,
    pub magnify: f64,
    pub bars: bool,
    /// Each ellipse's semi-axes written beside it (docs/adr/3760).
    pub labels: bool,
    /// Docs/adr/4920: the sessions' full covariance used, and the new marks' velocities read from the grid.
    pub correlated: bool,
    pub velocities: bool,
}

#[derive(Clone, Debug)]
pub enum Event {
    View(View),
    Class(Class),
    Scale(f64),
    Epoch(String),
    Import,
    Imported(Option<PathBuf>),
    Exclude(String),
    Include(String),
    /// docs/adr/5320: every session's independent baselines into the adjustment, the dependent ones out.
    Independent,
    Known(Field, String),
    AddKnown,
    /// The error ellipses drawn by Kabul et, and their magnification (docs/adr/2960).
    Ellipses(bool),
    Magnify(f64),
    /// The height bars beside the ellipses (docs/adr/3360).
    Bars(bool),
    /// The ellipses' labels (docs/adr/3760).
    Labels(bool),
    /// Docs/adr/4920: the sessions' correlations, the new marks' velocities.
    Correlated(bool),
    Velocities(bool),
    Accept,
    Close,
}

fn msg(e: Event) -> Message {
    Message::SurveyNetwork(e)
}

/// Metres as millimetres to a tenth; a value that rounds to nothing is 0.0, not −0.0.
fn mm(v: f64) -> String {
    let t = format!("{:.1}", v * 1000.0);
    if t == "-0.0" { "0.0".to_owned() } else { t }
}

/// A box's number (a point before the decimals), none when empty.
fn number(text: &str, words: &str) -> Result<Option<f64>, String> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>().ok().filter(|v| v.is_finite()).map(Some).ok_or_else(|| format!("{words} “{t}” bir sayı değil (ondalıktan önce nokta)."))
}

/// The summary's confidence region line (docs/adr/2560): the largest error ellipse of the marks the stage moved, its
/// mark, and the confidence; none without one.
pub(crate) fn region_line(a: &MapSurveyNetworked) -> Option<String> {
    let (mark, e) = a
        .stations
        .iter()
        .filter_map(|st| st.ellipse.as_ref().map(|e| (st.mark.as_str(), e)))
        .max_by(|x, y| x.1.major.total_cmp(&y.1.major))?;
    Some(format!(
        "Güven bölgesi (%{:.0}): en büyük hata elipsi {mark} noktasında, yarı eksenleri {} ve {} mm; kotun aralığı ±{} mm.",
        e.confidence * 100.0,
        mm(e.major),
        mm(e.minor),
        mm(e.up)
    ))
}

/// The summary's epoch line (docs/adr/2120): where the network's epoch comes from and how far the known points moved.
pub(crate) fn epoch_line(a: &MapSurveyNetworked) -> String {
    // Points before the decimals, as the window's other numbers (m0, vᵀPv, mm).
    let e = |v: f64| format!("{v:.2}");
    let mut line = match (a.epoch_from, a.epoch) {
        (NetworkEpochFrom::Given, Some(t)) => format!("Ölçme epoku {} (yazılan).", e(t)),
        (NetworkEpochFrom::Baselines, Some(t)) => format!("Ölçme epoku {} (bazların gözlem zamanlarının ortası).", e(t)),
        (_, Some(t)) => format!("Bazların gözlem zamanı yok: ağın epoku bilinen noktalarınki, {}.", e(t)),
        _ => "Bazların gözlem zamanı ve bilinen noktaların epoku yok: ağın epoku belirsiz.".to_owned(),
    };
    if !a.moved.is_empty() {
        let horizontal = a.moved.iter().map(|m| m.shift[0].hypot(m.shift[1])).fold(0.0, f64::max);
        let up = a.moved.iter().map(|m| m.shift[2].abs()).fold(0.0, f64::max);
        // By their own velocity, the campaign's grid, or both (docs/adr/2520).
        let own = a.moved.iter().filter(|m| m.by == KnownMovedBy::Velocity).count();
        let how = match (own, a.moved.len() - own) {
            (_, 0) => " kendi hızıyla oraya taşındı".to_owned(),
            (0, _) => " arazi ölçümünün hız ızgarasıyla oraya taşındı".to_owned(),
            (o, g) => format!(" oraya taşındı: {o} nokta kendi hızıyla, {g} nokta hız ızgarasıyla"),
        };
        line.push_str(&format!(
            " {} bilinen nokta{how} (en çok {} mm yatay, {} mm düşey; Madde 15(1)(a)).",
            a.moved.len(),
            mm(horizontal),
            mm(up)
        ));
    }
    line
}

/// The summary's similarity line (Madde 15(1)(c), docs/adr/2160): the scale, its test, the class's limit.
fn similarity_line(a: &MapSurveyNetworked) -> (Kind, String) {
    let Some(s) = &a.similarity else {
        return (Kind::Info, "Benzerlik dönüşümü (Madde 15(1)(c)) için en az üç bilinen nokta gerekir.".to_owned());
    };
    let test = if s.agrees {
        format!("t = {:.2} ≤ {:.2}, ölçek uyumlu", s.t, s.quantile)
    } else {
        format!("t = {:.2} > {:.2}, ölçek uyumsuz", s.t, s.quantile)
    };
    let limit = match a.rules.as_ref().and_then(|r| r.similarity.map(|h| (r.class.clone(), h))) {
        Some((class, true)) => format!("; {class} sınırı tutar"),
        Some((class, false)) => format!("; {class} sınırını aşar"),
        None => String::new(),
    };
    let kind = if s.agrees && !limit.contains("aşar") { Kind::Ok } else { Kind::Warn };
    (
        kind,
        format!(
            "Benzerlik dönüşümü (Madde 15(1)(c), {} bilinen nokta): ölçek {:.2} ppm (σ {:.2}); {test}{limit}.",
            s.known.len(),
            s.scale,
            s.sigma
        ),
    )
}

/// The summary's line of what docs/adr/4920 adds: which correlations went in, and the velocity grid; none when neither
/// was asked for nor the baselines have sessions.
pub(crate) fn sessions_line(a: &MapSurveyNetworked) -> Option<(Kind, String)> {
    let mut parts = Vec::new();
    let mut kind = Kind::Info;
    if let Some(c) = &a.correlations {
        parts.push(match c.used {
            NetworkCorrelations::Session if c.sessions.is_empty() => {
                "Oturum korelasyonları istendi; dengelemeye giren bazların ortak oturumu yok".to_owned()
            }
            NetworkCorrelations::Session => format!(
                "{} baz oturumu ({}) tam kovaryansıyla girdi, bazlar korelasyonlu ölçü gibi bileşen bileşen sınandı",
                c.sessions.len(),
                c.sessions.join(", ")
            ),
            NetworkCorrelations::None => {
                kind = Kind::Warn;
                format!(
                    "{} baz oturumunun ({}) korelasyonları kullanılmadı; aynı oturumun bazları bağımsız sayıldı",
                    c.unused.len(),
                    c.unused.join(", ")
                )
            }
        });
    }
    if let Some(g) = &a.velocity_grid {
        parts.push(format!("yeni noktaların hızı “{}” hız ızgarasından", g.name));
    }
    let text = parts.join("; ");
    let mut chars = text.chars();
    let first = chars.next()?;
    Some((kind, format!("{}{}.", first.to_uppercase(), chars.as_str())))
}

/// A row of Bazlar: an archive baseline and, when it went into the network, its free stage's test.
struct BaselineRow {
    id: String,
    /// docs/adr/5320: Bağımsız, Bağımlı or nothing (no session, excluded).
    independent: String,
    session: String,
    from: String,
    to: String,
    length: f64,
    up: Option<f64>,
    w: Option<f64>,
    state: &'static str,
    rules: Option<bool>,
    excluded: bool,
}

/// A stage's line: m0 and the model test, the blunders.
fn stage_line(name: &str, st: &NetworkStage) -> (Kind, String) {
    let held = st.held.join(", ");
    let test = match (st.m0, st.passed) {
        (Some(m0), Some(true)) => format!("m0 = {m0:.2}, χ² geçti (vᵀPv {:.2} ≤ {:.2}, serbestlik {})", st.omega, st.chi2.unwrap_or(0.0), st.dof),
        (Some(m0), Some(false)) => format!("m0 = {m0:.2}, χ² geçmedi (vᵀPv {:.2} > {:.2})", st.omega, st.chi2.unwrap_or(0.0)),
        _ => "fazla ölçü yok, sınanamaz".to_owned(),
    };
    let blunders = if st.blunders == 0 {
        format!("uyuşumsuz ölçü yok (w ≤ {:.2})", st.limit)
    } else {
        format!("{} uyuşumsuz ölçü (w > {:.2}), en kötüsü {}", st.blunders, st.limit, st.worst.clone().unwrap_or_default())
    };
    let kind = if st.blunders > 0 || st.passed == Some(false) { Kind::Error } else { Kind::Ok };
    (kind, format!("{name} ({held} sabit): {test}; {blunders}."))
}

impl App {
    pub(crate) fn survey_network_run(&mut self) {
        let epoch = match number(&self.survey_network.epoch, "Ölçme epoku") {
            Ok(e) => e,
            Err(why) => {
                self.survey_network.answer = Some(Err(why));
                return;
            }
        };
        let input = MapSurveyNetwork {
            class: self.survey_network.class.id(),
            scale: (self.survey_network.scale != 1.0).then_some(self.survey_network.scale),
            epoch,
            correlations: self.survey_network.correlated.then_some(NetworkCorrelations::Session),
            velocities: self.survey_network.velocities,
            ..MapSurveyNetwork::default()
        };
        let Some(doc) = self.document.as_mut() else { return };
        self.survey_network.answer = Some(match map_survey_network::plan(&ExecutionContext::new(&mut doc.model), &input) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err(error.message)
            }
            _ => Err("Ağ dengelenemedi.".to_owned()),
        });
    }

    /// GNSS ağ dengelemesi (`map.survey.adjustment`): the window over the project's network.
    pub(crate) fn open_survey_network(&mut self) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return Task::none();
        }
        self.survey_network = State { scale: 1.0, magnify: 1000.0, ..State::default() };
        self.survey_network_run();
        self.dialog = Some(Dialog::SurveyNetwork);
        Task::none()
    }

    /// A baseline file brought into the last arazi ölçümü (a test or a trace gives it here, the picker in the app).
    pub(crate) fn survey_network_file(&mut self, path: PathBuf) {
        let Some(campaign) = self.survey_campaign() else { return };
        let operation = Op::Baselines { campaign, file: path.to_string_lossy().into_owned() };
        if let Some(out) = self.map_survey_edit(operation) {
            self.say(Level::Success, format!("{} baz arşive alındı.", out.records.len()));
        }
        self.survey_network_run();
    }

    fn survey_network_known(&mut self) {
        let k = self.survey_network.known.clone();
        let read = || -> Result<Op, String> {
            let need = |v: Option<f64>, words: &str| v.ok_or_else(|| format!("{words} boş; değerini yazın."));
            let lat = need(number(&k.lat, "Enlem")?, "Enlem")?;
            let lon = need(number(&k.lon, "Boylam")?, "Boylam")?;
            let height = need(number(&k.height, "Elipsoit yüksekliği")?, "Elipsoit yüksekliği")?;
            let srid = k.srid.trim().parse::<u32>().map_err(|_| format!("Çerçeve “{}” bir EPSG kodu değil.", k.srid.trim()))?;
            let epoch = number(&k.epoch, "Epok")?;
            let s = [number(&k.sigma[0], "σ kuzey")?, number(&k.sigma[1], "σ doğu")?, number(&k.sigma[2], "σ yukarı")?];
            let sigma = match s {
                [None, None, None] => None,
                [Some(n), Some(e), Some(u)] => Some([n, e, u]),
                _ => return Err("σ'nın üçünü birden yazın (ağırlıklı) ya da üçünü de boş bırakın (sabit).".into()),
            };
            let v = [number(&k.velocity[0], "VX")?, number(&k.velocity[1], "VY")?, number(&k.velocity[2], "VZ")?];
            let velocity = match v {
                [None, None, None] => None,
                [Some(x), Some(y), Some(z)] => Some([x, y, z]),
                _ => return Err("Hızın üç bileşenini birden yazın (VX, VY, VZ, m/yıl) ya da üçünü de boş bırakın.".into()),
            };
            let source = Some(k.source.trim().to_owned()).filter(|t| !t.is_empty());
            Ok(Op::Known { mark: k.mark.trim().to_owned(), lat, lon, height, srid, epoch, sigma, velocity, source })
        };
        match read() {
            Ok(op) => {
                if let Some(out) = self.map_survey_edit(op) {
                    self.say(Level::Success, format!("“{}” bilinen nokta olarak eklendi ({}).", k.mark.trim(), out.id.unwrap_or_default()));
                    self.survey_network.known = KnownForm::default();
                }
            }
            Err(why) => self.warn(why),
        }
        self.survey_network_run();
    }

    fn survey_network_accept(&mut self) {
        let op = Op::Network {
            campaign: None,
            scale: (self.survey_network.scale != 1.0).then_some(self.survey_network.scale),
            class: self.survey_network.class.id(),
            rules: None,
            hold: None,
            epoch: number(&self.survey_network.epoch, "Ölçme epoku").ok().flatten(),
            correlations: self.survey_network.correlated.then_some(NetworkCorrelations::Session),
            velocities: self.survey_network.velocities,
            layer: None,
            // On the layer chosen in Katmanlar, else the active one (docs/adr/2960).
            ellipses: self.survey_network.ellipses.then(|| kentos_commands::NetworkEllipses {
                layer: self.selected_layer.clone().unwrap_or_else(|| {
                    self.document.as_ref().map(|d| d.model.layers().active().to_owned()).unwrap_or_default()
                }),
                scale: self.survey_network.magnify,
                heights: self.survey_network.bars,
                labels: self.survey_network.labels,
                label_height: None,
            }),
        };
        if let Some(out) = self.map_survey_edit(op) {
            let drawn = match (out.ellipses, out.bars) {
                (0, _) => String::new(),
                (n, 0) => format!(", {n} hata elipsi {} kat büyütülerek çizildi", self.survey_network.magnify),
                (n, b) => format!(", {n} hata elipsi ve {b} kot çubuğu {} kat büyütülerek çizildi", self.survey_network.magnify),
            };
            // Docs/adr/3760: and their labels; docs/adr/4160: those moved off east, those with no free place.
            let drawn = match out.labels {
                0 => drawn,
                l => format!("{drawn}; {l} etiket yazıldı"),
            };
            let drawn = match (out.labels_moved, out.labels_crowded) {
                (0, 0) => drawn,
                (m, 0) => format!("{drawn} ({m}'i elipse ya da etikete düşmesin diye başka yana kondu)"),
                (0, c) => format!("{drawn} ({c}'i için boş yer yok, doğuda çakışıyor)"),
                (m, c) => format!("{drawn} ({m}'i başka yana kondu; {c}'i için boş yer yok, doğuda çakışıyor)"),
            };
            self.say(
                Level::Success,
                format!("Ağ dengelemesi kabul edildi ({}): {} nokta yazıldı{drawn}.", out.id.unwrap_or_default(), out.records.len()),
            );
        }
        self.survey_network_run();
    }

    pub(crate) fn survey_network_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::View(v) => self.survey_network.view = v,
            Event::Class(c) => {
                self.survey_network.class = c;
                self.survey_network_run();
            }
            Event::Ellipses(on) => self.survey_network.ellipses = on,
            Event::Bars(on) => self.survey_network.bars = on,
            Event::Labels(on) => self.survey_network.labels = on,
            Event::Correlated(on) => {
                self.survey_network.correlated = on;
                self.survey_network_run();
            }
            Event::Velocities(on) => {
                self.survey_network.velocities = on;
                self.survey_network_run();
            }
            Event::Magnify(v) => {
                if v > 0.0 {
                    self.survey_network.magnify = v;
                }
            }
            Event::Scale(v) => {
                if v > 0.0 {
                    self.survey_network.scale = v;
                    self.survey_network_run();
                }
            }
            Event::Epoch(text) => {
                self.survey_network.epoch = text;
                self.survey_network_run();
            }
            Event::Import => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("GNSS bazları")
                            .add_filter("GVX ya da baz metni", &["gvx", "xml", "txt", "csv"])
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::Imported(p)),
                );
            }
            Event::Imported(Some(path)) => self.survey_network_file(path),
            Event::Imported(None) => {}
            Event::Exclude(id) => {
                let reason = self
                    .survey_network
                    .answer
                    .as_ref()
                    .and_then(|a| a.as_ref().ok())
                    .and_then(|a| a.baselines.iter().find(|b| b.id == id))
                    .and_then(|b| b.free.w.map(|w| format!("Serbest dengelemede uyuşumsuz (w = {w:.2}); GNSS ağ dengelemesi penceresinden.")))
                    .unwrap_or_else(|| "GNSS ağ dengelemesi penceresinden dışlandı.".to_owned());
                self.map_survey_edit(Op::Exclude { solution: id, reason });
                self.survey_network_run();
            }
            Event::Independent => {
                self.map_survey_edit(Op::Independent { session: None });
                self.survey_network_run();
            }
            Event::Include(id) => {
                self.map_survey_edit(Op::Include { solution: id });
                self.survey_network_run();
            }
            Event::Known(field, text) => {
                let k = &mut self.survey_network.known;
                match field {
                    Field::Mark => k.mark = text,
                    Field::Lat => k.lat = text,
                    Field::Lon => k.lon = text,
                    Field::Height => k.height = text,
                    Field::Srid => k.srid = text,
                    Field::Epoch => k.epoch = text,
                    Field::Sigma(i) => k.sigma[i] = text,
                    Field::Velocity(i) => k.velocity[i] = text,
                    Field::Source => k.source = text,
                }
            }
            Event::AddKnown => self.survey_network_known(),
            Event::Accept => self.survey_network_accept(),
            Event::Close => {
                self.survey_network = State::default();
                self.dialog = None;
            }
        }
        Task::none()
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn survey_network_view(&self) -> Element<'_, Message> {
        let s = &self.survey_network;
        let survey = self.document.as_ref().and_then(|d| d.model.map_survey());
        let (count, used) = survey.map_or((0, 0), |sv| {
            (sv.baselines.len(), sv.baselines.iter().filter(|b| b.adjust && b.excluded.is_none()).count())
        });
        let head = Form::new()
            .label_width(130.0)
            .field(
                "Bazlar",
                row![
                    words::secondary(IMPORT, Some(msg(Event::Import))),
                    label::caption(format!("Arşivde {count} baz, {used} tanesi dengelemede (GVX 1.0 ya da B; satırlı baz metni)."))
                ]
                .spacing(10)
                .align_y(Center),
            )
            .field(
                "Sınıf",
                row![
                    Segmented::new([Class::None, Class::C1, Class::C2, Class::C3], s.class, |c| msg(Event::Class(c))),
                    label::caption("BÖHHBÜY 2018, Madde 15, 20, 22")
                ]
                .spacing(10)
                .align_y(Center),
            )
            .field("Varyans çarpanı", NumberInput::new(s.scale, |v| msg(Event::Scale(v))).step(0.5).decimals(2).point().width(140))
            .field(
                "Korelasyon ve hız",
                row![
                    fields::check(
                        if s.correlated { Check::Checked } else { Check::Unchecked },
                        CORRELATED,
                        Some(msg(Event::Correlated(!s.correlated)))
                    ),
                    fields::check(
                        if s.velocities { Check::Checked } else { Check::Unchecked },
                        VELOCITIES,
                        Some(msg(Event::Velocities(!s.velocities)))
                    ),
                    label::caption("Oturumun bazları tam kovaryansıyla girer; hız arazi ölçümünün hız ızgarasından okunur.")
                ]
                .spacing(10)
                .align_y(Center),
            )
            .field(
                "Ölçme epoku",
                row![
                    container(
                        fields::input(None, "Bazlardan", &s.epoch, false, false).on_input(|v| msg(Event::Epoch(v)))
                    )
                    .width(140),
                    label::caption("Boşsa bazların gözlem zamanlarının ortası; bilinen noktalar oraya kendi hızıyla ya da hız ızgarasıyla taşınır.")
                ]
                .spacing(10)
                .align_y(Center),
            )
            .field(
                "Hata elipsleri",
                row![
                    fields::check(
                        if s.ellipses { Check::Checked } else { Check::Unchecked },
                        "Kabul et çizsin",
                        Some(msg(Event::Ellipses(!s.ellipses)))
                    ),
                    NumberInput::new(s.magnify, |v| msg(Event::Magnify(v))).step(100.0).decimals(0).width(110),
                    // Docs/adr/3360: the height's half-interval as a bar through the point.
                    fields::check(
                        if s.bars { Check::Checked } else { Check::Unchecked },
                        "Kot çubuğu",
                        s.ellipses.then(|| msg(Event::Bars(!s.bars)))
                    ),
                    // Docs/adr/3760: the semi-axes in millimetres beside each ellipse.
                    fields::check(
                        if s.labels { Check::Checked } else { Check::Unchecked },
                        "Etiket",
                        s.ellipses.then(|| msg(Event::Labels(!s.labels)))
                    ),
                    label::caption("kat büyütülmüş; Katmanlar'da seçili katmana, yoksa etkin katmana. 1000 kat: 1\u{a0}cm, 10\u{a0}m çizilir.")
                ]
                .spacing(10)
                .align_y(Center),
            );
        let mut lines = Vec::new();
        let mut table: Element<'_, Message> = label::caption("").into();
        let mut acceptable = false;
        match &s.answer {
            Some(Ok(a)) => {
                acceptable = a.acceptable;
                lines.push(words::text_line(Kind::Info, epoch_line(a)));
                for (name, st) in [("Serbest dengeleme", &a.free), ("Bilinen noktalara dayalı", &a.constrained)] {
                    let (kind, text) = stage_line(name, st);
                    lines.push(words::text_line(kind, text));
                }
                let (kind, text) = similarity_line(a);
                lines.push(words::text_line(kind, text));
                if let Some(text) = region_line(a) {
                    lines.push(words::text_line(Kind::Info, text));
                }
                if let Some((kind, text)) = sessions_line(a) {
                    lines.push(words::text_line(kind, text));
                }
                if let Some(r) = &a.rules {
                    let confidence = r.confidence.map_or("Baarda α₀ = 0,001".to_owned(), |c| format!("güven {c}"));
                    lines.push(words::text_line(
                        if r.passed { Kind::Ok } else { Kind::Warn },
                        if r.passed {
                            format!("{} ({}; {confidence}): bazların ve noktaların standart sapmaları sınırların içinde.", r.class, r.articles)
                        } else {
                            // What does not hold: the baselines, the stations, the similarity's scale (docs/adr/2160).
                            let mut parts = Vec::new();
                            if r.baselines + r.stations > 0 {
                                parts.push(format!("{} baz ve {} nokta sınırı aşıyor", r.baselines, r.stations));
                            }
                            if r.similarity == Some(false) {
                                parts.push("benzerlik dönüşümünün ölçeği sınırı aşıyor".to_owned());
                            }
                            format!("{} ({}; {confidence}): {}.", r.class, r.articles, parts.join("; "))
                        },
                    ));
                }
                if a.acceptable {
                    lines.push(words::text_line(Kind::Info, "Kabul edilebilir: Kabul et yeni noktaları çizer, kabul edilmiş noktaları eski sonucunu koruyarak taşır."));
                }
                let accepted = a.current.as_ref().map(|id| format!("Bu ağ “{id}” dengelemesiyle kabul edildi; noktaları güncel."));
                for why in &a.why {
                    let kind = if accepted.as_ref() == Some(why) { Kind::Ok } else { Kind::Error };
                    lines.push(words::text_line(kind, why.clone()));
                }
                lines.push(words::text_line(
                    Kind::Info,
                    "Teknik denetimdir, kurum onayı değildir. Sınıfın sayıları KentOS'un kural paketindendir (catalog/v1/rules/gnss-ag-bohhbuy-2018.json).",
                ));
                let f = |v: Option<f64>| v.map_or(String::new(), mm);
                table = match s.view {
                    View::Marks => sheet(
                        vec![
                            col("İşaret", 90.0),
                            col("Rol", 90.0),
                            num("σ kuzey (mm)", 90.0),
                            num("σ doğu (mm)", 90.0),
                            num("σ yukarı (mm)", 90.0),
                            col("Hız D/K/Y (mm/yıl)", 130.0),
                            col("Güven bölgesi a/b, yön, kot (mm)", 190.0),
                            num("ΔY (mm)", 80.0),
                            num("ΔX (mm)", 80.0),
                            num("ΔZ (mm)", 80.0),
                            col("Sınıf", 70.0),
                            col("Kabul edilen", 90.0),
                        ],
                        a.stations
                            .iter()
                            .map(|st| {
                                let role = match (&st.known, st.held) {
                                    (Some(_), true) => "Sabit",
                                    (Some(_), false) => "Ağırlıklı",
                                    _ => "Yeni",
                                };
                                vec![
                                    st.mark.clone(),
                                    role.to_owned(),
                                    mm(st.sigma[0]),
                                    mm(st.sigma[1]),
                                    mm(st.sigma[2]),
                                    // Docs/adr/4920: from the campaign's grid, east/north/up.
                                    st.velocity.map_or(String::new(), |v| format!("{}/{}/{}", mm(v[0]), mm(v[1]), mm(v[2]))),
                                    // The error ellipse's semi-axes, its direction (a semt), the height's interval.
                                    st.ellipse.as_ref().map_or(String::new(), |e| {
                                        let way = e.azimuth.map_or("yönsüz".to_owned(), |d| self.format().bearing(d * 10.0 / 9.0));
                                        format!("{}/{} · {way} · ±{}", mm(e.major), mm(e.minor), mm(e.up))
                                    }),
                                    f(st.dx),
                                    f(st.dy),
                                    f(st.dz),
                                    st.rules.map_or(String::new(), |ok| if ok { "Tutar".into() } else { "Aşar".into() }),
                                    st.point.clone().unwrap_or_default(),
                                ]
                            })
                            .collect(),
                        "Ağda işaret yok.",
                        260.0,
                    ),
                    View::Baselines => {
                        // Every baseline of the archive: those in the network with their test, the others with why not.
                        let rows: Vec<BaselineRow> = survey
                            .map(|sv| {
                                sv.baselines
                                    .iter()
                                    .map(|b| {
                                        let tested = a.baselines.iter().find(|x| x.id == b.id);
                                        let state = match (&b.excluded, b.adjust, tested.map(|t| t.free.flag.as_str())) {
                                            (Some(_), _, _) => "Dışlandı",
                                            (None, false, _) => "Dengede değil",
                                            (None, true, Some("blunder")) => "Uyuşumsuz",
                                            (None, true, Some("uncontrolled")) => "Denetlenemez",
                                            (None, true, Some(_)) => "Geçti",
                                            (None, true, None) => "Başka ölçüm",
                                        };
                                        // docs/adr/5320: one of its session's n − 1 independent baselines, or dependent.
                                        let independent = sv
                                            .session_of(&b.id)
                                            .filter(|_| b.excluded.is_none())
                                            .and_then(|x| kentos_native_application::map_survey::independent_baselines(sv, &x.id))
                                            .map_or(String::new(), |(free, _)| {
                                                if free.contains(&b.id) { "Bağımsız".into() } else { "Bağımlı".into() }
                                            });
                                        BaselineRow {
                                            id: b.id.clone(),
                                            independent,
                                            session: sv.session_of(&b.id).map(|x| x.id.clone()).unwrap_or_default(),
                                            from: b.from.clone(),
                                            to: b.to.clone(),
                                            length: (b.d[0] * b.d[0] + b.d[1] * b.d[1] + b.d[2] * b.d[2]).sqrt(),
                                            up: tested.map(|t| t.free.v[2]),
                                            w: tested.and_then(|t| t.free.w),
                                            state,
                                            rules: tested.and_then(|t| t.rules),
                                            excluded: b.excluded.is_some(),
                                        }
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        road_dialog::grid_table(
                            vec![
                                col("Baz", 56.0),
                                col("Oturum", 56.0),
                                col("Bağımsız", 76.0),
                                col("Başlangıç", 80.0),
                                col("Bitiş", 80.0),
                                num("Uzunluk (m)", 96.0),
                                num("Artık yukarı (mm)", 110.0),
                                num("w", 60.0),
                                col("Durum", 110.0),
                                col("Sınıf", 60.0),
                                col("", 70.0),
                            ],
                            rows.len(),
                            move |i, k| {
                                let b = &rows[i];
                                let text = |t: String| -> Element<'_, Message> { kentos_ui::widget::table::text(t).into() };
                                let number = |t: String| -> Element<'_, Message> { kentos_ui::widget::table::number(t).into() };
                                match k {
                                    0 => text(b.id.clone()),
                                    1 => text(b.session.clone()),
                                    2 => text(b.independent.clone()),
                                    3 => text(b.from.clone()),
                                    4 => text(b.to.clone()),
                                    5 => number(format!("{:.3}", b.length)),
                                    6 => number(b.up.map_or(String::new(), mm)),
                                    7 => number(b.w.map_or("—".to_owned(), |w| format!("{w:.2}"))),
                                    8 => text(b.state.to_owned()),
                                    9 => text(b.rules.map_or(String::new(), |ok| if ok { "Tutar".into() } else { "Aşar".into() })),
                                    _ => {
                                        let (words, event) = if b.excluded {
                                            ("Geri al", Event::Include(b.id.clone()))
                                        } else {
                                            ("Dışla", Event::Exclude(b.id.clone()))
                                        };
                                        let x = iced::widget::button(label::caption(words))
                                            .style(kentos_ui::style::button::ghost)
                                            .padding([0, 6])
                                            .on_press(msg(event));
                                        kentos_ui::widget::Cell::padded(container(x).center_x(Fill)).into()
                                    }
                                }
                            },
                            "Arşivde baz yok.",
                        )
                    }
                    View::Known => sheet(
                        vec![
                            col("Kimlik", 50.0),
                            col("İşaret", 70.0),
                            num("Enlem (°)", 100.0),
                            num("Boylam (°)", 100.0),
                            num("h (m)", 75.0),
                            num("EPSG", 50.0),
                            num("Epok", 55.0),
                            col("Taşınma D/K/Y (mm)", 150.0),
                            num("Artık D/K/Y (mm)", 120.0),
                            col("Ağırlık", 90.0),
                            road_dialog::Col { title: "Kaynak", width: None, number: false },
                        ],
                        survey
                            .map(|sv| {
                                sv.known
                                    .iter()
                                    .map(|k| {
                                        vec![
                                            k.id.clone(),
                                            k.mark.clone(),
                                            format!("{:.9}", k.lat),
                                            format!("{:.9}", k.lon),
                                            format!("{:.4}", k.height),
                                            k.srid.to_string(),
                                            k.epoch.map_or(String::new(), |e| format!("{e}")),
                                            // Moved to the measurement epoch by its own velocity or along the
                                            // velocity grid (docs/adr/2120, 2520).
                                            a.moved.iter().find(|m| m.id == k.id).map_or(String::new(), |m| {
                                                format!(
                                                    "{} {}/{}/{}",
                                                    if m.by == KnownMovedBy::Velocity { "hız" } else { "ızgara" },
                                                    mm(m.shift[0]),
                                                    mm(m.shift[1]),
                                                    mm(m.shift[2])
                                                )
                                            }),
                                            // The similarity's residual (Madde 15(1)(c), docs/adr/2160).
                                            a.similarity
                                                .as_ref()
                                                .and_then(|s| s.residuals.iter().find(|r| r.id == k.id))
                                                .map_or(String::new(), |r| format!("{}/{}/{}", mm(r.enu[0]), mm(r.enu[1]), mm(r.enu[2]))),
                                            match (&k.excluded, k.sigma) {
                                                (Some(_), _) => "Dışlandı".to_owned(),
                                                (None, Some(g)) => format!("σ {}/{}/{} mm", mm(g[0]), mm(g[1]), mm(g[2])),
                                                (None, None) => "Sabit".to_owned(),
                                            },
                                            k.source.clone().unwrap_or_default(),
                                        ]
                                    })
                                    .collect()
                            })
                            .unwrap_or_default(),
                        "Bilinen nokta yok.",
                        200.0,
                    ),
                    // docs/adr/5320: each session's blocks, the nine correlations row by row.
                    View::Sessions => sheet(
                        vec![
                            col("Oturum", 56.0),
                            col("Ad", 70.0),
                            col("Satır", 56.0),
                            col("Sütun", 56.0),
                            num("ρ X·(X Y Z)", 150.0),
                            num("ρ Y·(X Y Z)", 150.0),
                            num("ρ Z·(X Y Z)", 150.0),
                            num("En büyük |ρ|", 90.0),
                        ],
                        survey
                            .map(|sv| {
                                sv.baseline_sessions
                                    .iter()
                                    .flat_map(|o| {
                                        o.blocks.iter().map(move |b| {
                                            let three = |k: usize| {
                                                b.r[k * 3..k * 3 + 3]
                                                    .iter()
                                                    .map(|v| format!("{v:.3}").replace('.', ","))
                                                    .collect::<Vec<_>>()
                                                    .join("  ")
                                            };
                                            let most = b.r.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
                                            vec![
                                                o.id.clone(),
                                                o.name.clone(),
                                                b.row.clone(),
                                                b.col.clone(),
                                                three(0),
                                                three(1),
                                                three(2),
                                                format!("{most:.3}").replace('.', ","),
                                            ]
                                        })
                                    })
                                    .collect()
                            })
                            .unwrap_or_default(),
                        "Arşivde baz oturumu yok: GVX dosyasıyla okunur.",
                        200.0,
                    ),
                };
            }
            Some(Err(why)) => lines.push(words::text_line(Kind::Error, why.clone())),
            None => {}
        }
        let k = &s.known;
        let input = |value: &str, hint: &str, field: Field, width: f32| {
            container(fields::input(None, hint, value, false, false).on_input(move |v| msg(Event::Known(field, v)))).width(width)
        };
        // One label column, three equal boxes a row (the owner: a form's fields line up), their hints the words.
        let box_ = 150.0;
        let known = Form::new()
            .label_width(130.0)
            .field(
                "İşaret, çerçeve, epok",
                row![input(&k.mark, "İşaret", Field::Mark, box_), input(&k.srid, "EPSG (5252)", Field::Srid, box_), input(&k.epoch, "Epok (2026.84)", Field::Epoch, box_)]
                    .spacing(8)
                    .align_y(Center),
            )
            .field(
                "Enlem, boylam, h",
                row![input(&k.lat, "Enlem (°)", Field::Lat, box_), input(&k.lon, "Boylam (°)", Field::Lon, box_), input(&k.height, "h (m)", Field::Height, box_)]
                    .spacing(8)
                    .align_y(Center),
            )
            .field(
                "σ (m)",
                row![
                    input(&k.sigma[0], "Kuzey (boş: sabit)", Field::Sigma(0), box_),
                    input(&k.sigma[1], "Doğu", Field::Sigma(1), box_),
                    input(&k.sigma[2], "Yukarı", Field::Sigma(2), box_)
                ]
                .spacing(8)
                .align_y(Center),
            )
            .field(
                "Hız (m/yıl)",
                row![
                    input(&k.velocity[0], "VX (boş: hızsız)", Field::Velocity(0), box_),
                    input(&k.velocity[1], "VY", Field::Velocity(1), box_),
                    input(&k.velocity[2], "VZ", Field::Velocity(2), box_)
                ]
                .spacing(8)
                .align_y(Center),
            )
            .field(
                "Kaynak",
                row![input(&k.source, "TUSAGA-Aktif, C1 …", Field::Source, 3.0 * box_ + 16.0), words::secondary(ADD, Some(msg(Event::AddKnown)))]
                    .spacing(8)
                    .align_y(Center),
            );
        let mut body = Grows::new()
            .spacing(12.0)
            .width(Fill)
            .push(head)
            .push(words::summary(lines))
            .push(Segmented::new([View::Marks, View::Baselines, View::Known, View::Sessions], s.view, |v| msg(Event::View(v))));
        body = body.push_fit(table, kentos_ui::theme::typography::scaled(120.0));
        if s.view == View::Known {
            body = body.push(known);
        }
        // docs/adr/5320: n receivers of a session give n − 1 independent baselines; the rest go out of the adjustment.
        if s.view == View::Sessions {
            body = body.push(
                row![
                    words::secondary("Bağımsız bazları seç", Some(msg(Event::Independent))),
                    label::caption(
                        "n alıcılı oturumda n − 1 bağımsız baz: kısadan uzuna, işaretleri henüz bağlanmamış ikisini \
                         bağlayan baz bağımsızdır; bağımlılar dengelemeden çıkar (geri alınabilir)."
                    )
                    .width(Fill),
                ]
                .spacing(12)
                .align_y(Center),
            );
        }
        Frame::new(TITLE)
            .scroll(body)
            .aside(words::secondary(ACCEPT, acceptable.then(|| msg(Event::Accept))))
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(1000.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn survey_network_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(ACCEPT) => Some(msg(Event::Accept)),
            Control::Press(IMPORT) => Some(msg(Event::Import)),
            Control::Press(ADD) => Some(msg(Event::AddKnown)),
            Control::Press(CORRELATED) => Some(msg(Event::Correlated(!self.survey_network.correlated))),
            Control::Press(VELOCITIES) => Some(msg(Event::Velocities(!self.survey_network.velocities))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
