//! Boru hattı denetimi (docs/adr/1640; `pipes.check`): the pipes of a utility network checked as a gravity line by
//! `cad.pipes.check`, read only. The form names the network, where the inverts are (two attributes or the vertices'
//! heights), the diameter's attribute, unit and catalogue, the ground (a terrain surface, the structures' cover level,
//! or none) and the limits, every one of them the user's (empty: not checked; KentOS gives none). The result follows
//! the form. The summary counts the findings by kind, the table lists every pipe with its slope, diameter, cover and
//! findings (Bulgulu: only those with one); Bulgulu boruları seç selects them, CSV kaydet writes the table. Hat profili
//! draws the run from a pipe along the flow as a longitudinal profile under the drawing (`cad.pipes.profile`, docs/adr/1860).
//! Diğer ağ (docs/adr/2360) measures the clearances to another utility network's pipes: where they cross, the vertical
//! clearance and which is above; where they pass, the horizontal one. The least clearances and the side that must be
//! above are the user's; Kesişim and Yakın geçiş list them. With it, Hat profili draws its pipes crossing the run too
//! (kesişen borular, docs/adr/2760). Kesişimleri göster (docs/adr/3160) leaves the crossings and near passes marked on
//! the drawing, coloured by the check's verdict, until İşareti kaldır; the window closes.

use std::path::PathBuf;

use iced::widget::{container, row};
use kentos_ui::widget::grow::Grows;
use iced::{Center, Element, Task};
use kentos_commands::{
    CommandResult, NetworkKind, PipeAbove, PipeCheck, PipeClearanceAsk, PipeCrossing, PipeDiameter, PipeGround,
    PipeInverts, PipeNear, PipeProfileOther, PipeUnit, PipesCheck, PipesChecked, PipesProfile,
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
const SHOW: &str = "Kesişimleri göster";
/// The run from the pipe chosen drawn as a longitudinal profile (`cad.pipes.profile`, docs/adr/1860).
const DRAW_PROFILE: &str = "Hat profilini çiz";
/// Diğer ağ's pipes crossing the run drawn in the profile too (docs/adr/2760).
const PROFILE_CROSSINGS: &str = "kesişen borular";

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

/// What the table shows: every pipe, those with a finding, the crossings with the other network, its near passes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Show {
    #[default]
    All,
    Found,
    Crossings,
    Near,
}

impl std::fmt::Display for Show {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Show::All => "Tümü",
            Show::Found => "Bulgulu",
            Show::Crossings => "Kesişim",
            Show::Near => "Yakın geçiş",
        })
    }
}

/// Which network's pipe must be above where the two cross (docs/adr/2360).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Above {
    #[default]
    Either,
    This,
    Other,
}

impl std::fmt::Display for Above {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Above::Either => "Fark etmez",
            Above::This => "Bu ağ",
            Above::Other => "Diğer ağ",
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
    /// Diğer ağ's pipes crossing the run drawn in the profile (docs/adr/2760); on when the window opens.
    pub profile_crossings: bool,
    pub layers: Vec<(String, String)>,
    /// Diğer ağ (docs/adr/2360): the other network's id (empty: no clearance), where its inverts and diameters are,
    /// the least vertical and horizontal clearances (metres) and the side that must be above.
    pub other: String,
    pub other_from: From,
    pub other_start: String,
    pub other_end: String,
    pub other_diameter: String,
    pub other_unit: String,
    pub vertical: String,
    pub horizontal: String,
    pub above: Above,
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
    ProfileCrossings(bool),
    DrawProfile,
    Other(String),
    OtherFrom(From),
    OtherStart(String),
    OtherEnd(String),
    OtherDiameter(String),
    OtherUnit(String),
    Vertical(String),
    Horizontal(String),
    Above(Above),
    Select,
    /// docs/adr/3160: the crossings and near passes marked on the drawing.
    Mark,
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
        "clash" => "Çakışma",
        "closeCrossing" => "Düşey açıklık az",
        "wrongSide" => "Üstte olması gereken altta",
        "closeAlongside" => "Yatay açıklık az",
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

fn unit_of(text: &str) -> PipeUnit {
    match text {
        "cm" => PipeUnit::Cm,
        "m" => PipeUnit::M,
        _ => PipeUnit::Mm,
    }
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
        let unit = unit_of(&s.unit);
        let slope = |t: &str, w: &str| limit(t, w).map(|v| v.map(|v| v / 1000.0));
        let clearance = if s.other.is_empty() {
            None
        } else {
            Some(PipeClearanceAsk {
                network: s.other.clone(),
                inverts: match s.other_from {
                    From::Fields => PipeInverts::Fields { start: s.other_start.clone(), end: s.other_end.clone() },
                    From::Heights => PipeInverts::Heights,
                },
                diameter: (!s.other_diameter.trim().is_empty()).then(|| PipeDiameter {
                    field: s.other_diameter.clone(),
                    unit: unit_of(&s.other_unit),
                    catalog: Vec::new(),
                }),
                vertical: limit(&s.vertical, "En az düşey açıklık")?,
                horizontal: limit(&s.horizontal, "En az yatay açıklık")?,
                above: match s.above {
                    Above::Either => None,
                    Above::This => Some(PipeAbove::Pipe),
                    Above::Other => Some(PipeAbove::Other),
                },
            })
        };
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
            clearance,
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
        // The selection's own network when every selected object is its part (docs/adr/2480), else the first.
        let picked = self.selection_roles().network.and_then(|id| networks.iter().find(|n| n.0 == id).cloned());
        let Some(first) = picked.or_else(|| networks.first().cloned()) else {
            self.warn("Boru hattı denetimi: projede altyapı ağı yok. Önce Ağlar'da boruların katmanıyla bir altyapı ağı tanımlayın.");
            return Task::none();
        };
        let first = &first;
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
            other_start: "Akar baş".into(),
            other_end: "Akar son".into(),
            other_unit: "mm".into(),
            profile_crossings: true,
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
            // The other network of the clearances: its pipes crossing the run drawn in the profile (docs/adr/2760).
            crossing: check.clearance.filter(|_| s.profile_crossings).map(|c| PipeProfileOther {
                network: c.network,
                inverts: c.inverts,
                diameter: c.diameter.map(|d| PipeDiameter { catalog: Vec::new(), ..d }),
            }),
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
            // The crossing pipes (docs/adr/2760): those without an invert there are listed, not drawn.
            let crossing = match (out.crossings.len(), out.crossings.iter().filter(|c| c.invert.is_none()).count()) {
                (0, _) => String::new(),
                (n, 0) => format!(", {n} kesişen boru"),
                (n, k) => format!(", {n} kesişen boru ({k} borunun orada kotu yok, çizilmedi)"),
            };
            self.say(
                Level::Success,
                format!(
                    "Hat profili çizildi: {} boru, {:.2} m, kıyas kotu {:.2}{crossing}; {} nesne.",
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

    /// Diğer ağ chosen: where its inverts and diameters are, guessed from its pipes' own attributes (Akar baş, Çap;
    /// else the vertices' heights), as the checked network's are when the window opens.
    fn pipes_check_other(&mut self, id: String) {
        let guess = self.document.as_ref().and_then(|doc| {
            let net = doc.model.settings().networks.iter().find(|n| n.id == id)?;
            let layers: Vec<&str> = net.edges.iter().map(|l| l.layer.as_str()).collect();
            let has = |field: &str| {
                doc.model.entities().any(|e| layers.contains(&e.base().layer_id.as_str()) && e.base().attrs.contains_key(field))
            };
            Some((if has("Akar baş") { From::Fields } else { From::Heights }, has("Çap")))
        });
        let s = &mut self.pipes_check;
        if let Some((from, diameter)) = guess {
            s.other_from = from;
            s.other_diameter = if diameter { "Çap".into() } else { String::new() };
        }
        if id.is_empty() && matches!(s.show, Show::Crossings | Show::Near) {
            s.show = Show::All;
        }
        s.other = id;
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
            Event::ProfileCrossings(v) => {
                s.profile_crossings = v;
                return Task::none();
            }
            Event::DrawProfile => {
                self.pipes_profile_draw();
                return Task::none();
            }
            Event::Other(v) => self.pipes_check_other(v),
            Event::OtherFrom(v) => s.other_from = v,
            Event::OtherStart(v) => s.other_start = v,
            Event::OtherEnd(v) => s.other_end = v,
            Event::OtherDiameter(v) => s.other_diameter = v,
            Event::OtherUnit(v) => s.other_unit = v,
            Event::Vertical(v) => s.vertical = v,
            Event::Horizontal(v) => s.horizontal = v,
            Event::Above(v) => s.above = v,
            Event::Mark => {
                let (Ok(input), Some(Ok(r))) = (self.pipes_check_input(), &self.pipes_check.answer) else { return Task::none() };
                let Some(doc) = self.document.as_ref() else { return Task::none() };
                let Some(m) = crate::pipes_marks::of(r) else { return Task::none() };
                let fails = m.crossings.iter().filter(|c| c.verdict == crate::pipes_marks::Verdict::Fails).count();
                let unknown = m.crossings.iter().filter(|c| c.verdict == crate::pipes_marks::Verdict::Unknown).count();
                let text = format!(
                    "{TITLE}: {} kesişim ({fails} sorunlu, {unknown} açıklığı bilinmeyen) ve {} yakın geçiş çizimde işaretlendi; İşareti kaldır'a dek kalır.",
                    m.crossings.len(),
                    m.near.len()
                );
                self.pipes_shown_marks = Some(crate::pipes_marks::Shown::new(doc.session, input));
                self.pipes_check = State::default();
                self.dialog = None;
                self.say(Level::Info, text);
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
        // Diğer ağ (docs/adr/2360): the other utility networks; none chosen, no clearance.
        let others: Vec<(String, String)> = std::iter::once((String::new(), "Yok".to_owned()))
            .chain(s.networks.iter().filter(|(id, _)| *id != s.network).cloned())
            .collect();
        let other_from = Segmented::new([From::Fields, From::Heights], s.other_from, |v| msg(Event::OtherFrom(v)));
        let mut other = row![container(fields::select_owned(others, &s.other, |v| msg(Event::Other(v)))).width(170)]
            .spacing(8)
            .align_y(Center);
        if !s.other.is_empty() {
            other = other.push(label::caption("akar")).push(other_from);
            if s.other_from == From::Fields {
                other = other
                    .push(text(&s.other_start, "Baş", Event::OtherStart, 100.0))
                    .push(text(&s.other_end, "Son", Event::OtherEnd, 100.0));
            }
        }
        let form = form.field("Diğer ağ", other);
        let form = if s.other.is_empty() {
            form
        } else {
            let units: Vec<(String, String)> = ["mm", "cm", "m"].iter().map(|u| (u.to_string(), u.to_string())).collect();
            form.field(
                "Diğer ağın çapı",
                row![
                    text(&s.other_diameter, "Öznitelik", Event::OtherDiameter, 110.0),
                    container(fields::select_owned(units, &s.other_unit, |v| msg(Event::OtherUnit(v)))).width(70)
                ]
                .spacing(8)
                .align_y(Center),
            )
            .field(
                "Açıklık (m)",
                row![
                    label::caption("düşey en az"),
                    text(&s.vertical, "—", Event::Vertical, 80.0),
                    label::caption("yatay en az"),
                    text(&s.horizontal, "—", Event::Horizontal, 80.0),
                    label::caption("üstte"),
                    Segmented::new([Above::Either, Above::This, Above::Other], s.above, |v| msg(Event::Above(v)))
                ]
                .spacing(8)
                .align_y(Center),
            )
        };
        let mut lines = Vec::new();
        let mut rows = Vec::new();
        let mut found = 0;
        // docs/adr/3160: Kesişimleri göster while there is a crossing or a near pass.
        let crossed = matches!(&s.answer, Some(Ok(r)) if !r.crossings.is_empty() || !r.near.is_empty());
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
                if !s.other.is_empty() {
                    lines.push(words::text_line(Kind::Info, clearance_line(r)));
                }
                let names = self.pipes_check_names();
                match s.show {
                    Show::Crossings => rows.extend(r.crossings.iter().map(|c| crossing_row(self, c, &f))),
                    Show::Near => rows.extend(r.near.iter().map(|n| near_row(self, n, &f))),
                    Show::All | Show::Found => {
                        for (p, name) in r.pipes.iter().zip(names) {
                            if s.show == Show::Found && p.findings.is_empty() {
                                continue;
                            }
                            rows.push(pipe_row(p, name, &f));
                        }
                    }
                }
            }
            Some(Err(why)) => lines.push(words::text_line(Kind::Error, why.clone())),
            None => {}
        }
        let (east, north) = (f.east_label(), f.north_label());
        let (columns, empty) = match s.show {
            Show::Crossings => (
                vec![
                    col("Boru", 80.0),
                    col("Diğer boru", 90.0),
                    num(east, 110.0),
                    num(north, 110.0),
                    num("Akar (m)", 80.0),
                    num("Diğer akar (m)", 100.0),
                    num("Açıklık (m)", 90.0),
                    col("Üstte", 90.0),
                ],
                "Diğer ağın borularıyla kesişim yok.",
            ),
            Show::Near => (
                vec![
                    col("Boru", 80.0),
                    col("Diğer boru", 90.0),
                    num(east, 110.0),
                    num(north, 110.0),
                    num("Eksenler (m)", 100.0),
                    num("Açıklık (m)", 90.0),
                ],
                if s.horizontal.trim().is_empty() {
                    "Yakın geçişler için en az yatay açıklığı yazın."
                } else {
                    "En az yatay açıklıktan yakın geçen boru yok."
                },
            ),
            Show::All | Show::Found => (
                vec![
                    col("Boru", 80.0),
                    num("Uzunluk", 80.0),
                    num("Eğim ‰", 70.0),
                    num("Çap (mm)", 70.0),
                    num("En az örtü", 80.0),
                    col("Bulgular", 420.0),
                ],
                "Bu süzgeçte boru yok.",
            ),
        };
        let table = sheet(columns, rows, empty, 260.0);
        // The table takes the height the rest leaves, at least a head and three rows; its rows scroll inside it.
        let body = Grows::new()
            .spacing(12.0)
            .width(iced::Fill)
            .push(form)
            .push(words::summary(lines))
            .push(if s.other.is_empty() {
                Segmented::new(vec![Show::All, Show::Found], s.show, |v| msg(Event::Show(v)))
            } else {
                Segmented::new(vec![Show::All, Show::Found, Show::Crossings, Show::Near], s.show, |v| msg(Event::Show(v)))
            })
            .push_fit(table, kentos_ui::theme::typography::scaled(120.0));
        let ready = matches!(&s.answer, Some(Ok(r)) if !r.pipes.is_empty());
        // Hat profili (docs/adr/1860): from a pipe along the flow, to a pipe or where the flow ends or splits.
        let pipes: Vec<(String, String)> = match &s.answer {
            Some(Ok(r)) => r.pipes.iter().zip(self.pipes_check_names()).map(|(p, n)| (p.uid.clone(), n)).collect(),
            _ => Vec::new(),
        };
        let ends: Vec<(String, String)> =
            std::iter::once((String::new(), "Akışın sonu".to_owned())).chain(pipes.iter().cloned()).collect();
        let mut line = row![
            container(fields::select_owned(pipes, &s.profile_start, |v| msg(Event::ProfileStart(v)))).width(120),
            label::caption("bitiş"),
            container(fields::select_owned(ends, &s.profile_end, |v| msg(Event::ProfileEnd(v)))).width(140),
            label::caption("katman"),
            container(fields::select_owned(s.layers.clone(), &s.profile_layer, |v| msg(Event::ProfileLayer(v)))).width(150),
        ]
        .spacing(8)
        .align_y(Center);
        // With Diğer ağ: its pipes crossing the run drawn in the profile too (docs/adr/2760).
        if !s.other.is_empty() {
            line = line.push(words::check(
                s.profile_crossings,
                PROFILE_CROSSINGS,
                Some(msg(Event::ProfileCrossings(!s.profile_crossings))),
            ));
        }
        let line = line.push(words::secondary(DRAW_PROFILE, (ready && !s.profile_start.is_empty()).then(|| msg(Event::DrawProfile))));
        let profile = Form::new().label_width(110.0).field("Hat profili", line);
        let body = body.push(profile);
        Frame::new(TITLE)
            .scroll(body)
            .aside(words::secondary(SAVE, ready.then(|| msg(Event::Save))))
            .aside(words::secondary(SELECT, (found > 0).then(|| msg(Event::Select))))
            .aside(words::secondary(SHOW, crossed.then(|| msg(Event::Mark))))
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(1000.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn pipes_check_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(SELECT) => Some(msg(Event::Select)),
            Control::Press(SHOW) => Some(msg(Event::Mark)),
            Control::Press(DRAW_PROFILE) => Some(msg(Event::DrawProfile)),
            Control::Check(PROFILE_CROSSINGS, on) => Some(msg(Event::ProfileCrossings(on))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The clearances' line: the crossings, the clashes and those whose clearance is not known, the near passes.
fn clearance_line(r: &PipesChecked) -> String {
    let clash = r.crossings.iter().filter(|c| c.clearance.is_some_and(|v| v < 0.0)).count();
    let unknown = r.crossings.iter().filter(|c| c.clearance.is_none()).count();
    let mut out = format!("Diğer ağ: {} kesişim, {clash} çakışma", r.crossings.len());
    if unknown > 0 {
        out.push_str(&format!(", {unknown} kesişimde açıklık bilinmiyor (akar ya da çap yok)"));
    }
    out.push_str(&format!(
        "; {} yakın geçiş. Açıklık iç çaplarla ölçülür, et kalınlığı katılmaz; sınırlar sizin.",
        r.near.len()
    ));
    out
}

fn above_word(a: Option<PipeAbove>) -> &'static str {
    match a {
        Some(PipeAbove::Pipe) => "Bu ağ",
        Some(PipeAbove::Other) => "Diğer ağ",
        None => "",
    }
}

fn crossing_row(app: &App, c: &PipeCrossing, f: &kentos_interaction::Format) -> Vec<String> {
    let bare = |v: Option<f64>| v.map(|v| f.length_bare(v)).unwrap_or_default();
    vec![
        name_of(app, &c.pipe),
        name_of(app, &c.other),
        f.coord(c.at[0]),
        f.coord(c.at[1]),
        bare(c.invert),
        bare(c.other_invert),
        bare(c.clearance),
        above_word(c.above).to_owned(),
    ]
}

fn near_row(app: &App, n: &PipeNear, f: &kentos_interaction::Format) -> Vec<String> {
    vec![
        name_of(app, &n.pipe),
        name_of(app, &n.other),
        f.coord(n.at[0]),
        f.coord(n.at[1]),
        f.length_bare(n.distance),
        n.clearance.map(|v| f.length_bare(v)).unwrap_or_default(),
    ]
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
