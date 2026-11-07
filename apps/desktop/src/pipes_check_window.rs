//! Boru hattı denetimi (docs/adr/1640; `pipes.check`): the pipes of a utility network checked as a gravity line by
//! `cad.pipes.check`, read only. The form names the network, where the inverts are (two attributes or the vertices'
//! heights), the diameter's attribute, unit and catalogue, the ground (a terrain surface, the structures' cover level,
//! or none) and the limits, every one of them the user's (empty: not checked; KentOS gives none). The result follows
//! the form. The summary counts the findings by kind, the table lists every pipe with its slope, diameter, cover and
//! findings (Bulgulu: only those with one); Bulgulu boruları seç selects them, CSV kaydet writes the table. Hat profili
//! draws the run from a pipe along the flow as a longitudinal profile under the drawing (`cad.pipes.profile`, docs/adr/1860).

use std::path::PathBuf;

use iced::widget::{container, row};
use kentos_ui::widget::grow::Grows;
use iced::{Center, Element, Task};
use kentos_commands::{
    CommandResult, NetworkKind, PipeCheck, PipeDiameter, PipeGround, PipeInverts, PipeUnit, PipesCheck, PipesChecked,
    PipesProfile,
};
use kentos_domain::{Slot, Uuid};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, pipes_check, pipes_profile};
use kentos_ui::label;
use kentos_ui::widget::{Dialog as Frame, Form, Segmented};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, num, sheet};
use crate::style::fields;
use crate::traces::Control;

pub const TITLE: &str = "Boru hattı denetimi";
const CLOSE: &str = "Kapat";
const SAVE: &str = "CSV kaydet…";
const SELECT: &str = "Bulgulu boruları seç";
/// The run from the pipe chosen drawn as a longitudinal profile (`cad.pipes.profile`, docs/adr/1860).
const DRAW_PROFILE: &str = "Hat profilini çiz";

/// Where the inverts are read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum From {
    #[default]
    Fields,
    Heights,
}

impl std::fmt::Display for From {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            From::Fields => "Öznitelik",
            From::Heights => "Köşe kotları",
        })
    }
}

/// Where the ground is read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Ground {
    #[default]
    Surface,
    Structures,
    None,
}

impl std::fmt::Display for Ground {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Ground::Surface => "Yüzey",
            Ground::Structures => "Kapak kotu",
            Ground::None => "Yok",
        })
    }
}

/// Which pipes the table shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Show {
    #[default]
    All,
    Found,
}

impl std::fmt::Display for Show {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Show::All => "Tümü",
            Show::Found => "Bulgulu",
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct State {
    /// The utility networks: id and name.
    pub networks: Vec<(String, String)>,
    pub network: String,
    pub from: From,
    pub start: String,
    pub end: String,
    pub diameter: String,
    pub unit: String,
    pub catalog: String,
    pub ground: Ground,
    /// The project's surfaces: id and name.
    pub surfaces: Vec<(String, String)>,
    pub surface: String,
    pub cover_field: String,
    pub min_cover: String,
    pub max_cover: String,
    pub min_slope: String,
    pub max_slope: String,
    pub show: Show,
    pub answer: Option<Result<PipesChecked, String>>,
    /// Hat profili (docs/adr/1860): the run's first pipe (uid), its last (empty: where the flow ends or splits), the
    /// layer it is drawn on (the drawing's layers: id and name).
    pub profile_start: String,
    pub profile_end: String,
    pub profile_layer: String,
    pub layers: Vec<(String, String)>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Network(String),
    From(From),
    Start(String),
    End(String),
    Diameter(String),
    Unit(String),
    Catalog(String),
    Ground(Ground),
    Surface(String),
    CoverField(String),
    MinCover(String),
    MaxCover(String),
    MinSlope(String),
    MaxSlope(String),
    Show(Show),
    ProfileStart(String),
    ProfileEnd(String),
    ProfileLayer(String),
    DrawProfile,
    Select,
    Save,
    Saved(Option<PathBuf>),
    Close,
}

fn msg(e: Event) -> Message {
    Message::PipesCheck(e)
}

/// A finding's kind in words.
pub fn kind_word(kind: &str) -> &'static str {
    match kind {
        "detached" => "Bağlantısız",
        "noStructure" => "Bacasız uç",
        "noInvert" => "Akar kotu yok",
        "invertUnit" => "Kot birimi yanlış olabilir",
        "invertIsDepth" => "Kot yerine derinlik olabilir",
        "reverseSlope" => "Ters eğim",
        "flat" => "Eğimsiz",
        "lowSlope" => "Eğim az",
        "steepSlope" => "Eğim çok",
        "outletAbove" => "Çıkış girişten yüksek",
        "noDiameter" => "Çap yok",
        "diameterUnit" => "Çap birimi yanlış olabilir",
        "notInCatalog" => "Çap katalogda yok",
        "noGround" => "Zemin kotu yok",
        "offSurface" => "Yüzeyin dışında",
        "aboveGround" => "Zeminin üstünde",
        "shallow" => "Örtü az",
        "deep" => "Örtü çok",
        "curved" => "Yay: örtü köşelerde",
        _ => "—",
    }
}

/// A limit's text as a number: empty is none, a comma is a point; other text is the field's error.
fn limit(text: &str, words: &str) -> Result<Option<f64>, String> {
    let t = text.trim().replace(',', ".");
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>().ok().filter(|v| v.is_finite()).map(Some).ok_or_else(|| format!("{words} “{text}” bir sayı değil."))
}

/// The catalogue's text (sizes apart by spaces, semicolons or slashes; a comma is a decimal point).
fn catalog(text: &str) -> Result<Vec<f64>, String> {
    text.split([' ', ';', '/'])
        .filter(|t| !t.trim().is_empty())
        .map(|t| {
            t.trim().replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite()).ok_or_else(|| format!("Katalogdaki “{t}” bir sayı değil."))
        })
        .collect()
}

fn per_mille(v: f64) -> String {
    let s = format!("{:.2}", v * 1000.0);
    s.replace('.', ",")
}

/// A diameter in millimetres, as written: no trailing zeros (0.0002 m is “0,2”).
fn millimetres(metres: f64) -> String {
    let s = format!("{:.4}", metres * 1000.0);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.replace('.', ",")
}

/// The pipe's name: its Ad, else the uid's first part.
fn name_of(app: &App, uid: &str) -> String {
    let doc = app.document.as_ref();
    Uuid::parse_str(uid)
        .ok()
        .and_then(|u| doc.and_then(|d| d.model.slot_of(u)).and_then(|s| doc.and_then(|d| d.model.get(s))))
        .and_then(|e| e.base().attrs.get("Ad").cloned())
        .filter(|a| !a.trim().is_empty())
        .unwrap_or_else(|| uid.split('-').next().unwrap_or(uid).to_owned())
}

/// The table as CSV (metres and slopes as computed).
pub fn csv(r: &PipesChecked, names: &[String]) -> String {
    let mut out = String::from("Boru;Nesne;Uzunluk (m);Akış;Akar yukarı (m);Akar aşağı (m);Eğim (m/m);Çap (m);En az örtü (m);En çok örtü (m);Bulgular\n");
    let opt = |v: Option<f64>| v.map(|v| v.to_string()).unwrap_or_default();
    for (p, name) in r.pipes.iter().zip(names) {
        out.push_str(&format!(
            "{};{};{};{};{};{};{};{};{};{};{}\n",
            name,
            p.uid,
            p.length,
            if p.reversed { "çizime ters" } else { "çizim yönünde" },
            opt(p.upstream),
            opt(p.downstream),
            opt(p.slope),
            opt(p.diameter),
            opt(p.min_cover),
            opt(p.max_cover),
            p.findings.iter().map(|k| kind_word(k)).collect::<Vec<_>>().join(", ")
        ));
    }
    out
}

impl App {
    fn pipes_check_input(&self) -> Result<PipesCheck, String> {
        let s = &self.pipes_check;
        let unit = match s.unit.as_str() {
            "cm" => PipeUnit::Cm,
            "m" => PipeUnit::M,
            _ => PipeUnit::Mm,
        };
        let slope = |t: &str, w: &str| limit(t, w).map(|v| v.map(|v| v / 1000.0));
        Ok(PipesCheck {
            network: s.network.clone(),
            inverts: match s.from {
                From::Fields => PipeInverts::Fields { start: s.start.clone(), end: s.end.clone() },
                From::Heights => PipeInverts::Heights,
            },
            diameter: (!s.diameter.trim().is_empty())
                .then(|| catalog(&s.catalog).map(|c| PipeDiameter { field: s.diameter.clone(), unit, catalog: c }))
                .transpose()?,
            ground: match s.ground {
                Ground::Surface => Some(PipeGround::Surface { surface: s.surface.clone() }),
                Ground::Structures => Some(PipeGround::Structures { field: s.cover_field.clone() }),
                Ground::None => None,
            },
            min_cover: limit(&s.min_cover, "En az örtü")?,
            max_cover: limit(&s.max_cover, "En çok örtü")?,
            min_slope: slope(&s.min_slope, "En az eğim")?,
            max_slope: slope(&s.max_slope, "En çok eğim")?,
        })
    }

    fn pipes_check_run(&mut self) {
        let input = self.pipes_check_input();
        let Some(doc) = self.document.as_mut() else { return };
        self.pipes_check.answer = Some(input.and_then(|input| {
            match pipes_check::plan(&ExecutionContext::new(&mut doc.model), &input) {
                CommandResult::Completed { output, .. } => Ok(output),
                CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                    Err(error.message)
                }
                _ => Err("Denetlenemedi.".to_owned()),
            }
        }));
    }

    /// Boru hattı denetimi (`pipes.check`): the first utility network, the attributes guessed from the pipes' own
    /// (Akar baş/son, Çap; else the vertices' heights), the first surface (else Kapak kotu, else no ground), no limit.
    pub(crate) fn open_pipes_check(&mut self) -> Task<Message> {
        let Some(doc) = self.document.as_ref() else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let settings = doc.model.settings();
        let networks: Vec<(String, String)> = settings
            .networks
            .iter()
            .filter(|n| n.kind == NetworkKind::Utility)
            .map(|n| (n.id.clone(), n.name.clone()))
            .collect();
        let Some(first) = networks.first() else {
            self.warn("Boru hattı denetimi: projede altyapı ağı yok. Önce Ağlar'da boruların katmanıyla bir altyapı ağı tanımlayın.");
            return Task::none();
        };
        let net = settings.networks.iter().find(|n| n.id == first.0);
        let edge_layers: Vec<String> = net.map(|n| n.edges.iter().map(|l| l.layer.clone()).collect()).unwrap_or_default();
        let junction_layers: Vec<String> = net.map(|n| n.junctions.iter().map(|l| l.layer.clone()).collect()).unwrap_or_default();
        let has = |layers: &[String], field: &str| {
            doc.model.entities().any(|e| layers.contains(&e.base().layer_id) && e.base().attrs.contains_key(field))
        };
        let surfaces: Vec<(String, String)> = settings
            .road
            .as_ref()
            .map(|r| r.surfaces.iter().map(|s| (s.id.clone(), s.name.clone())).collect())
            .unwrap_or_default();
        let from = if has(&edge_layers, "Akar baş") { From::Fields } else { From::Heights };
        let ground = if !surfaces.is_empty() {
            Ground::Surface
        } else if has(&junction_layers, "Kapak kotu") {
            Ground::Structures
        } else {
            Ground::None
        };
        let layers: Vec<(String, String)> =
            doc.model.layers().leaves().iter().map(|n| (n.id.clone(), n.name.clone())).collect();
        let profile_layer = layers
            .iter()
            .find(|(_, n)| ["profil", "boykesit"].iter().any(|w| n.to_lowercase().contains(w)))
            .map_or_else(|| doc.model.layers().active().to_owned(), |l| l.0.clone());
        self.pipes_check = State {
            network: first.0.clone(),
            profile_layer,
            layers,
            networks,
            from,
            start: "Akar baş".into(),
            end: "Akar son".into(),
            diameter: if has(&edge_layers, "Çap") { "Çap".into() } else { String::new() },
            unit: "mm".into(),
            ground,
            surface: surfaces.first().map(|s| s.0.clone()).unwrap_or_default(),
            surfaces,
            cover_field: "Kapak kotu".into(),
            ..State::default()
        };
        self.pipes_check_run();
        // The profile's first pipe: the first one checked.
        if let Some(Ok(r)) = &self.pipes_check.answer {
            self.pipes_check.profile_start = r.pipes.first().map(|p| p.uid.clone()).unwrap_or_default();
        }
        self.dialog = Some(Dialog::PipesCheck);
        Task::none()
    }

    /// Hat profilini çiz: the run from the pipe chosen as the check reads it, drawn under the drawing in one undo step.
    fn pipes_profile_draw(&mut self) {
        let check = match self.pipes_check_input() {
            Ok(c) => c,
            Err(why) => return self.warn(why),
        };
        let s = &self.pipes_check;
        let mut input = PipesProfile {
            network: check.network,
            start: s.profile_start.clone(),
            end: (!s.profile_end.is_empty()).then(|| s.profile_end.clone()),
            inverts: check.inverts,
            // The catalogue is the check's question, not the profile's.
            diameter: check.diameter.map(|d| PipeDiameter { catalog: Vec::new(), ..d }),
            ground: check.ground,
            layer_id: s.profile_layer.clone(),
            at: [0.0, 0.0],
            datum: None,
            exaggeration: None,
            height: None,
            expected_revision: None,
        };
        // Planned at the origin (nothing written), then drawn under the drawing by how far its frame reaches up.
        let up = match self.document.as_mut().map(|doc| pipes_profile::plan(&ExecutionContext::new(&mut doc.model), &input)) {
            Some(CommandResult::Completed { output, .. }) => (output.top - output.datum) * output.exaggeration,
            _ => 0.0,
        };
        input.at = self.under_drawing(up);
        let Some(doc) = self.document.as_mut() else { return };
        let result = pipes_profile::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            self.say(
                Level::Success,
                format!(
                    "Hat profili çizildi: {} boru, {:.2} m, kıyas kotu {:.2}; {} nesne.",
                    out.pipes.len(),
                    out.length,
                    out.datum,
                    out.uids.len()
                ),
            );
            self.pipes_check = State::default();
            self.dialog = None;
        }
    }

    fn pipes_check_names(&self) -> Vec<String> {
        match &self.pipes_check.answer {
            Some(Ok(r)) => r.pipes.iter().map(|p| name_of(self, &p.uid)).collect(),
            _ => Vec::new(),
        }
    }

    /// The table written to `path` as CSV (a test or a trace gives it here, the picker in the app).
    pub(crate) fn pipes_check_save(&mut self, path: PathBuf) {
        let names = self.pipes_check_names();
        let Some(Ok(r)) = &self.pipes_check.answer else { return };
        match std::fs::write(&path, csv(r, &names)) {
            Ok(()) => self.say(Level::Success, format!("{TITLE}: {} boru {} dosyasına yazıldı.", r.pipes.len(), path.display())),
            Err(e) => self.warn(format!("{TITLE}: {} yazılamadı: {e}. Başka bir klasör seçin.", path.display())),
        }
    }

    pub(crate) fn pipes_check_event(&mut self, event: Event) -> Task<Message> {
        let s = &mut self.pipes_check;
        match event {
            Event::Network(v) => s.network = v,
            Event::From(v) => s.from = v,
            Event::Start(v) => s.start = v,
            Event::End(v) => s.end = v,
            Event::Diameter(v) => s.diameter = v,
            Event::Unit(v) => s.unit = v,
            Event::Catalog(v) => s.catalog = v,
            Event::Ground(v) => s.ground = v,
            Event::Surface(v) => s.surface = v,
            Event::CoverField(v) => s.cover_field = v,
            Event::MinCover(v) => s.min_cover = v,
            Event::MaxCover(v) => s.max_cover = v,
            Event::MinSlope(v) => s.min_slope = v,
            Event::MaxSlope(v) => s.max_slope = v,
            Event::Show(v) => {
                s.show = v;
                return Task::none();
            }
            Event::ProfileStart(v) => {
                s.profile_start = v;
                return Task::none();
            }
            Event::ProfileEnd(v) => {
                s.profile_end = v;
                return Task::none();
            }
            Event::ProfileLayer(v) => {
                s.profile_layer = v;
                return Task::none();
            }
            Event::DrawProfile => {
                self.pipes_profile_draw();
                return Task::none();
            }
            Event::Select => {
                let Some(Ok(r)) = &self.pipes_check.answer else { return Task::none() };
                let Some(doc) = self.document.as_ref() else { return Task::none() };
                let slots: Vec<Slot> = r
                    .pipes
                    .iter()
                    .filter(|p| !p.findings.is_empty())
                    .filter_map(|p| Uuid::parse_str(&p.uid).ok().and_then(|u| doc.model.slot_of(u)))
                    .collect();
                let n = slots.len();
                self.selection.set(slots);
                self.pipes_check = State::default();
                self.dialog = None;
                self.say(Level::Info, format!("{TITLE}: bulgusu olan {n} boru seçildi."));
                return Task::none();
            }
            Event::Save => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title(TITLE)
                            .set_file_name("boru-hatti-denetimi.csv")
                            .add_filter("CSV", &["csv"])
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::Saved(p)),
                );
            }
            Event::Saved(Some(path)) => {
                self.pipes_check_save(path);
                return Task::none();
            }
            Event::Saved(None) => return Task::none(),
            Event::Close => {
                self.pipes_check = State::default();
                self.dialog = None;
                return Task::none();
            }
        }
        self.pipes_check_run();
        Task::none()
    }

    pub(crate) fn pipes_check_view(&self) -> Element<'_, Message> {
        let s = &self.pipes_check;
        let f = self.format();
        let text = |value: &str, hint: &str, on: fn(String) -> Event, width: f32| {
            container(fields::input(None, hint, value, false, false).on_input(move |v| msg(on(v)))).width(width)
        };
        let units: Vec<(String, String)> = ["mm", "cm", "m"].iter().map(|u| (u.to_string(), u.to_string())).collect();
        let inverts: Element<'_, Message> = match s.from {
            From::Fields => row![
                Segmented::new([From::Fields, From::Heights], s.from, |v| msg(Event::From(v))),
                text(&s.start, "Baş", Event::Start, 110.0),
                text(&s.end, "Son", Event::End, 110.0)
            ]
            .spacing(8)
            .align_y(Center)
            .into(),
            From::Heights => Segmented::new([From::Fields, From::Heights], s.from, |v| msg(Event::From(v))).into(),
        };
        let ground: Element<'_, Message> = match s.ground {
            Ground::Surface => row![
                Segmented::new([Ground::Surface, Ground::Structures, Ground::None], s.ground, |v| msg(Event::Ground(v))),
                container(fields::select_owned(s.surfaces.clone(), &s.surface, |v| msg(Event::Surface(v)))).width(170)
            ]
            .spacing(8)
            .align_y(Center)
            .into(),
            Ground::Structures => row![
                Segmented::new([Ground::Surface, Ground::Structures, Ground::None], s.ground, |v| msg(Event::Ground(v))),
                text(&s.cover_field, "Kapak kotu", Event::CoverField, 170.0)
            ]
            .spacing(8)
            .align_y(Center)
            .into(),
            Ground::None => Segmented::new([Ground::Surface, Ground::Structures, Ground::None], s.ground, |v| msg(Event::Ground(v))).into(),
        };
        let form = Form::new()
            .label_width(110.0)
            .field("Ağ", container(fields::select_owned(s.networks.clone(), &s.network, |v| msg(Event::Network(v)))).width(240))
            .field("Akar kotu", inverts)
            .field(
                "Çap",
                row![
                    text(&s.diameter, "Öznitelik", Event::Diameter, 110.0),
                    container(fields::select_owned(units, &s.unit, |v| msg(Event::Unit(v)))).width(70),
                    label::caption("Katalog"),
                    text(&s.catalog, "200 300 400", Event::Catalog, 170.0)
                ]
                .spacing(8)
                .align_y(Center),
            )
            .field("Zemin", ground)
            .field(
                "Örtü (m)",
                row![
                    label::caption("en az"),
                    text(&s.min_cover, "—", Event::MinCover, 80.0),
                    label::caption("en çok"),
                    text(&s.max_cover, "—", Event::MaxCover, 80.0)
                ]
                .spacing(8)
                .align_y(Center),
            )
            .field(
                "Eğim (‰)",
                row![
                    label::caption("en az"),
                    text(&s.min_slope, "—", Event::MinSlope, 80.0),
                    label::caption("en çok"),
                    text(&s.max_slope, "—", Event::MaxSlope, 80.0)
                ]
                .spacing(8)
                .align_y(Center),
            );
        let mut lines = Vec::new();
        let mut rows = Vec::new();
        let mut found = 0;
        match &s.answer {
            Some(Ok(r)) => {
                found = r.pipes.iter().filter(|p| !p.findings.is_empty()).count();
                let total: u32 = r.counts.iter().map(|c| c.count).sum();
                lines.push(words::text_line(
                    if total == 0 { Kind::Ok } else { Kind::Warn },
                    format!(
                        "{} boru, {}, {} parça: {} boruda {} bulgu.",
                        r.pipes.len(),
                        f.length(r.length),
                        r.parts,
                        found,
                        total
                    ),
                ));
                if !r.counts.is_empty() {
                    lines.push(words::text_line(
                        Kind::Info,
                        r.counts.iter().map(|c| format!("{} {}", kind_word(&c.kind), c.count)).collect::<Vec<_>>().join(" · "),
                    ));
                }
                lines.push(words::text_line(
                    Kind::Info,
                    format!(
                        "Örtü boru {} üstünden; sınırlar sizin (boş: denetlenmez). Birim hatası önerilir, düzeltilmez; çizime bir şey yazılmaz.",
                        if r.cover_to == "crown" { "tepesinin (akar + iç çap)" } else { "akarının" }
                    ),
                ));
                let names = self.pipes_check_names();
                for (p, name) in r.pipes.iter().zip(names) {
                    if s.show == Show::Found && p.findings.is_empty() {
                        continue;
                    }
                    rows.push(pipe_row(p, name, &f));
                }
            }
            Some(Err(why)) => lines.push(words::text_line(Kind::Error, why.clone())),
            None => {}
        }
        let table = sheet(
            vec![
                col("Boru", 80.0),
                num("Uzunluk", 80.0),
                num("Eğim ‰", 70.0),
                num("Çap (mm)", 70.0),
                num("En az örtü", 80.0),
                col("Bulgular", 420.0),
            ],
            rows,
            "Bu süzgeçte boru yok.",
            260.0,
        );
        // The table takes the height the rest leaves, at least a head and three rows; its rows scroll inside it.
        let body = Grows::new()
            .spacing(12.0)
            .width(iced::Fill)
            .push(form)
            .push(words::summary(lines))
            .push(Segmented::new([Show::All, Show::Found], s.show, |v| msg(Event::Show(v))))
            .push_fit(table, kentos_ui::theme::typography::scaled(120.0));
        let ready = matches!(&s.answer, Some(Ok(r)) if !r.pipes.is_empty());
        // Hat profili (docs/adr/1860): from a pipe along the flow, to a pipe or where the flow ends or splits.
        let pipes: Vec<(String, String)> = match &s.answer {
            Some(Ok(r)) => r.pipes.iter().zip(self.pipes_check_names()).map(|(p, n)| (p.uid.clone(), n)).collect(),
            _ => Vec::new(),
        };
        let ends: Vec<(String, String)> =
            std::iter::once((String::new(), "Akışın sonu".to_owned())).chain(pipes.iter().cloned()).collect();
        let profile = Form::new().label_width(110.0).field(
            "Hat profili",
            row![
                container(fields::select_owned(pipes, &s.profile_start, |v| msg(Event::ProfileStart(v)))).width(120),
                label::caption("bitiş"),
                container(fields::select_owned(ends, &s.profile_end, |v| msg(Event::ProfileEnd(v)))).width(140),
                label::caption("katman"),
                container(fields::select_owned(s.layers.clone(), &s.profile_layer, |v| msg(Event::ProfileLayer(v)))).width(150),
                words::secondary(DRAW_PROFILE, (ready && !s.profile_start.is_empty()).then(|| msg(Event::DrawProfile)))
            ]
            .spacing(8)
            .align_y(Center),
        );
        let body = body.push(profile);
        Frame::new(TITLE)
            .scroll(body)
            .aside(words::secondary(SAVE, ready.then(|| msg(Event::Save))))
            .aside(words::secondary(SELECT, (found > 0).then(|| msg(Event::Select))))
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(1000.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn pipes_check_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(SELECT) => Some(msg(Event::Select)),
            Control::Press(DRAW_PROFILE) => Some(msg(Event::DrawProfile)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

fn pipe_row(p: &PipeCheck, name: String, f: &kentos_interaction::Format) -> Vec<String> {
    vec![
        if p.reversed { format!("{name} ⇠") } else { name },
        f.length(p.length),
        p.slope.map(per_mille).unwrap_or_default(),
        p.diameter.map(millimetres).unwrap_or_default(),
        p.min_cover.map(|c| f.length(c)).unwrap_or_default(),
        p.findings.iter().map(|k| kind_word(k)).collect::<Vec<_>>().join(", "),
    ]
}
