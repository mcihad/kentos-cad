//! Uygulama kontrolü (docs/adr/1700; `asbuilt.check`): the measured points of a layer set against the design's points,
//! surface and lines by `cad.asbuilt.check`, read only. The form names the layers, the surface, the names' attribute,
//! the search radius, the tolerances and the quality attribute and limit, every value the user's (empty: not checked;
//! KentOS gives none). The result follows the form: the summary counts the points by status, the root mean squares
//! and whether the design changed since the expected digest; the table lists every measured point with its
//! differences and flags (Dışındakiler: only those outside, unmatched or ambiguous). Dışındakileri seç selects them,
//! CSV kaydet writes the table.
//!
//! Saha paketi (docs/adr/1920; `cad.field.package`): the same design (points, lines, surface, the names' attribute) and
//! the same limits, with the quality notes typed below them (a line each), written as one `.zip` for the field; the
//! file's name is asked, the drawing is not changed. Kaydın saha paketi (docs/adr/2420) checks the record with the
//! package it was made with; paketin tasarımıyla (docs/adr/2820) compares it with that package's own design, and its
//! own surface when it carries one (docs/adr/3220: `yuzey.xml`, the drawing's later surface change said as a warning).
//!
//! İş türü (docs/adr/3620): the as-built and the stakeout check set the points against the design; İzleme sets this
//! epoch's marks against an earlier epoch's by name (their movement, never a design change), with the control points
//! that should not move. The form then shows only the epochs, the control points and the movement limits, and the
//! result speaks of movement.

use std::path::PathBuf;

use iced::widget::{container, row, text_editor};
use iced::{Center, Element, Task};
use kentos_commands::{AsbuiltCheck, AsbuiltChecked, AsbuiltKind, AsbuiltPoint, CommandResult, Entity, FieldPackage};
use kentos_domain::{Slot, Uuid};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, asbuilt_check, field_package};
use kentos_ui::label;
use kentos_ui::widget::grow::Grows;
use kentos_ui::widget::{Dialog as Frame, Form, Segmented};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, num, sheet};
use crate::style::fields;
use crate::traces::Control;

pub const TITLE: &str = "Uygulama kontrolü";
const CLOSE: &str = "Kapat";
const SAVE: &str = "CSV kaydet…";
const SELECT: &str = "Dışındakileri seç";
const PACKAGE: &str = "Saha paketi yaz…";
/// The returned record's field package (docs/adr/2420): chosen, and taken out.
const PICK_PACKAGE: &str = "Paketi seç…";
const DROP_PACKAGE: &str = "Paketi kaldır";
/// The record checked with its package's own design (docs/adr/2820).
const PACKAGE_DESIGN: &str = "paketin tasarımıyla";
/// The choice that leaves a layer or the surface out.
pub(crate) const NONE: &str = "";

/// Which points the table shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Show {
    #[default]
    All,
    Faults,
}

impl std::fmt::Display for Show {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Show::All => "Tümü",
            Show::Faults => "Dışındakiler",
        })
    }
}

#[derive(Debug, Default)]
pub struct State {
    /// The drawing's layers with points: id and name; the design's lines' layers.
    pub layers: Vec<(String, String)>,
    pub line_layers: Vec<(String, String)>,
    pub surfaces: Vec<(String, String)>,
    pub measured: String,
    pub design: String,
    pub lines: String,
    pub surface: String,
    pub field: String,
    pub search: String,
    pub horizontal: String,
    pub vertical: String,
    pub quality_field: String,
    pub quality: String,
    pub show: Show,
    pub answer: Option<Result<AsbuiltChecked, String>>,
    /// The package's quality notes, a line each.
    pub notes: text_editor::Content,
    /// The field package the measured record was made with (docs/adr/2420); empty: none.
    pub record_package: String,
    /// The record checked with the package's own design, not the drawing's (docs/adr/2820).
    pub record_design: bool,
    /// The job's kind (docs/adr/3620): 0 as-built, 1 stakeout check, 2 monitoring.
    pub kind: usize,
    /// A monitoring job's control points, by name, comma separated.
    pub control: String,
    /// The surface's limit held to the nearest distance (docs/adr/4020), not the height difference.
    pub nearest: bool,
    /// A monitoring job's earlier epochs (docs/adr/4420): their layers' names, oldest first, comma separated.
    pub epochs: String,
    /// The series table's epoch titles (docs/adr/4420): “1. dönem” … then “Bu dönem”, as the last answer has them.
    pub epoch_titles: Vec<String>,
}

/// The jobs' words, in the kind's order (docs/adr/3620).
pub(crate) const KINDS: [&str; 3] = ["Yapılanın kontrolü (as-built)", "Aplikasyon kontrolü", "İzleme (dönemsel hareket)"];

#[derive(Clone, Debug)]
pub enum Event {
    Measured(String),
    Design(String),
    Lines(String),
    Surface(String),
    Field(String),
    Search(String),
    Horizontal(String),
    Vertical(String),
    QualityField(String),
    Quality(String),
    Show(Show),
    Select,
    Save,
    Saved(Option<PathBuf>),
    Notes(text_editor::Action),
    Package,
    Packaged(Option<PathBuf>),
    /// The returned record's package (docs/adr/2420).
    PickPackage,
    PickedPackage(Option<PathBuf>),
    DropPackage,
    /// The package's own design compared (docs/adr/2820).
    PackageDesign(bool),
    /// The job's kind and a monitoring job's control points (docs/adr/3620).
    Kind(usize),
    Control(String),
    /// Yüzey farkı (docs/adr/4020): the height difference (false) or the nearest distance.
    Nearest(bool),
    /// Önceki dönemler (docs/adr/4420): the earlier epochs' layers' names.
    Epochs(String),
    Close,
}

fn msg(e: Event) -> Message {
    Message::Asbuilt(e)
}

/// A status in words.
pub fn status_word(s: &str) -> &'static str {
    match s {
        "within" => "İçinde",
        "outside" => "Dışında",
        "unmatched" => "Eşleşmedi",
        "ambiguous" => "Belirsiz",
        _ => "—",
    }
}

/// A monitoring job's status in words (docs/adr/3620): a mark beyond a limit has moved; one not at the earlier epoch
/// is new.
pub fn watch_status_word(s: &str) -> &'static str {
    match s {
        "within" => "Sınır içinde",
        "outside" => "Sınırı aştı",
        "unmatched" => "Yeni nokta",
        "ambiguous" => "Belirsiz",
        _ => "—",
    }
}

/// A monitoring job's flag in words (docs/adr/3620).
pub fn watch_flag_word(f: &str) -> &'static str {
    match f {
        "outsideHorizontal" => "yatay sınırı aştı",
        "outsideVertical" => "düşey sınırı aştı",
        other => flag_word(other),
    }
}

/// Whether the answer is a monitoring job's (docs/adr/3620).
fn watched(r: &AsbuiltChecked) -> bool {
    r.kind == Some(AsbuiltKind::Monitoring)
}

/// A flag in words.
pub fn flag_word(f: &str) -> &'static str {
    match f {
        "proposed" => "yerle eşlendi (öneri)",
        "outsideHorizontal" => "yatay dışında",
        "outsideVertical" => "düşey dışında",
        "outsideSurface" => "yüzeye göre dışında",
        "outsideLine" => "çizgiye göre dışında",
        "lowQuality" => "kalite düşük",
        "noHeight" => "kotu yok",
        "offSurface" => "yüzeyin dışında",
        _ => "—",
    }
}

/// A length to the millimetre with its sign, comma decimal (+0,012); below half a millimetre “0,000” (no sign of a
/// rounded zero).
fn mm(v: f64) -> String {
    if v.abs() < 0.0005 {
        return "0,000".into();
    }
    let t = format!("{v:+.3}");
    t.replace('.', ",")
}

fn limit(text: &str, words: &str) -> Result<Option<f64>, String> {
    let t = text.trim().replace(',', ".");
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>().ok().filter(|v| v.is_finite()).map(Some).ok_or_else(|| format!("{words} “{text}” bir sayı değil."))
}

fn opt(v: &str) -> Option<String> {
    (!v.trim().is_empty()).then(|| v.trim().to_owned())
}

/// The table as CSV (metres as computed).
pub fn csv(r: &AsbuiltChecked) -> String {
    let watch = watched(r);
    let (status, flag): (fn(&str) -> &'static str, fn(&str) -> &'static str) =
        if watch { (watch_status_word, watch_flag_word) } else { (status_word, flag_word) };
    let mut out = format!(
        "Nokta;Nesne;{};Eşleme;ΔDoğu (m);ΔKuzey (m);ΔKot (m);Yatay (m);Yüzeye göre (m);Yüzeye en yakın (m);Çizgiye göre (m);σ (m);Durum;İşaretler\n",
        if watch { "Önceki dönemin noktası" } else { "Tasarım noktası" }
    );
    let o = |v: Option<f64>| v.map(|v| v.to_string()).unwrap_or_default();
    for p in &r.points {
        out.push_str(&format!(
            "{};{};{};{};{};{};{};{};{};{};{};{};{};{}\n",
            p.name,
            p.uid,
            p.design_name.as_deref().unwrap_or(""),
            p.by.as_deref().unwrap_or(""),
            o(p.de),
            o(p.dn),
            o(p.dh),
            o(p.horizontal),
            o(p.surface),
            o(p.nearest),
            o(p.normal),
            o(p.sigma),
            status(&p.status),
            p.flags.iter().map(|f| flag(f)).collect::<Vec<_>>().join(", ")
        ));
    }
    out
}

/// The package's file name, or nothing.
fn package_name(path: &str) -> &str {
    std::path::Path::new(path).file_name().and_then(|n| n.to_str()).unwrap_or("")
}

/// What of the package's design was compared: its points and lines (docs/adr/2820) and its surface when it carries one
/// (docs/adr/3220), or only its digest expected.
pub(crate) fn package_design_words(p: &kentos_commands::AsbuiltPackage) -> &'static str {
    match (p.design, p.surface) {
        (true, true) => "tasarım paketin noktaları, çizgileri ve yüzeyi",
        (true, false) => "tasarım paketin noktaları ve çizgileri",
        _ => "tasarım özeti paketten",
    }
}

/// What changed since the record's package: the drawing's design (docs/adr/2820) and surface (docs/adr/3220). Checked
/// against the package's own, the drawing's change is said as a warning, not the record's error.
pub(crate) fn changes(r: &kentos_commands::AsbuiltChecked) -> Vec<(Kind, &'static str)> {
    let mut out = Vec::new();
    let own = r.package.as_ref().is_some_and(|p| p.design);
    if r.design_changed && own {
        out.push((Kind::Warn, "Çizimdeki tasarım paketten sonra değişmiş; kayıt sahaya verilen tasarımla, paketin tasarımıyla denetlendi."));
    } else if r.design_changed {
        out.push((Kind::Error, "Tasarım, saha kaydının yapıldığı tasarımdan değişmiş."));
    }
    let own_surface = r.package.as_ref().is_some_and(|p| p.surface);
    if r.surface_changed && own_surface {
        out.push((Kind::Warn, "Çizimdeki yüzey paketten sonra değişmiş; yüzey farkları sahaya verilen yüzeyle, paketin yüzeyiyle (yuzey.xml)."));
    } else if r.surface_changed {
        out.push((Kind::Error, "Yüzey, saha paketinin yazıldığı yüzeyden değişmiş."));
    }
    out
}

/// What the check took from the record's package (docs/adr/2420): its stamp and revision, the limits it gave, and the
/// system, epoch or heights it was written for when they are not the project's.
fn package_lines(p: &kentos_commands::AsbuiltPackage, f: &kentos_interaction::Format) -> Vec<Element<'static, Message>> {
    let mut out = Vec::new();
    let limit = |k: &str| match k {
        "horizontal" => p.horizontal.map(|v| format!("yatay {}", f.length(v))),
        "vertical" => p.vertical.map(|v| format!("düşey {}", f.length(v))),
        _ => p.quality.map(|v| format!("kalite {}", f.length(v))),
    };
    let taken: Vec<String> = p.taken.iter().filter_map(|k| limit(k)).collect();
    out.push(words::text_line(
        Kind::Info,
        format!(
            "Saha paketi {} (revizyon {}): {}; {}.",
            p.stamp.replace('T', " "),
            p.revision,
            if taken.is_empty() { "sınırlar girdinin".to_owned() } else { format!("paketten {}", taken.join(", ")) },
            package_design_words(p)
        ),
    ));
    if !p.same_system {
        out.push(words::text_line(
            Kind::Warn,
            format!("Paket {} için yazılmış; projenin sistemi başka.", p.epsg.as_deref().unwrap_or("sistemsiz bir proje")),
        ));
    }
    if !p.same_epoch {
        out.push(words::text_line(
            Kind::Warn,
            format!("Paketin epoku {}; projeninki başka.", p.epoch.map_or("yok".to_owned(), |e| format!("{e}"))),
        ));
    }
    if !p.same_heights {
        out.push(words::text_line(
            Kind::Warn,
            format!("Paketin kot sistemi “{}”; projeninki başka.", p.heights.as_deref().unwrap_or("yok")),
        ));
    }
    out
}

impl App {
    fn asbuilt_input(&self) -> Result<AsbuiltCheck, String> {
        let s = &self.asbuilt;
        // A monitoring job (docs/adr/3620) sets the earlier epoch's marks by name: the design's other parts stay out.
        let watch = s.kind == 2;
        let unless = |v: Option<String>| if watch { None } else { v };
        Ok(AsbuiltCheck {
            measured: s.measured.clone(),
            design: opt(&s.design),
            surface: unless(opt(&s.surface)),
            lines: unless(opt(&s.lines)),
            field: opt(&s.field),
            search: if watch { None } else { limit(&s.search, "Arama yarıçapı")? },
            horizontal: limit(&s.horizontal, "Yatay tolerans")?,
            vertical: limit(&s.vertical, "Düşey tolerans")?,
            quality_field: opt(&s.quality_field),
            quality: limit(&s.quality, "Kalite sınırı")?,
            expected_design: None,
            package: unless(opt(&s.record_package)),
            package_design: !watch && s.record_design && !s.record_package.is_empty(),
            kind: match s.kind {
                1 => Some(kentos_commands::AsbuiltKind::Stakeout),
                2 => Some(kentos_commands::AsbuiltKind::Monitoring),
                _ => None,
            },
            control: if watch {
                s.control.split([',', ';']).map(str::trim).filter(|c| !c.is_empty()).map(str::to_owned).collect()
            } else {
                Vec::new()
            },
            surface_by: (!watch && s.nearest).then_some(kentos_commands::AsbuiltSurfaceBy::Nearest),
            // Docs/adr/4420: the earlier epochs by their layers' names, each epoch's day its layer's Veri kümesi
            // bilgisi's (Güncelleme günü), this epoch's too.
            epochs: if watch { self.asbuilt_epochs()? } else { Vec::new() },
            date: if watch && !s.epochs.trim().is_empty() { self.layer_day(&s.measured) } else { None },
        })
    }

    /// A layer's day, its Veri kümesi bilgisi's Güncelleme günü (docs/adr/4420).
    fn layer_day(&self, id: &str) -> Option<String> {
        let doc = self.document.as_ref()?;
        doc.model.layers().get(id).and_then(|n| n.dataset.as_ref()).and_then(|d| d.updated.clone())
    }

    /// Önceki dönemler (docs/adr/4420): the layers named, oldest first, each with its day; a name no layer has is said.
    fn asbuilt_epochs(&self) -> Result<Vec<kentos_commands::AsbuiltEpoch>, String> {
        let s = &self.asbuilt;
        let mut out = Vec::new();
        for name in s.epochs.split([',', ';']).map(str::trim).filter(|n| !n.is_empty()) {
            let low = name.to_lowercase();
            let Some((id, _)) = s.layers.iter().find(|(id, n)| n.to_lowercase() == low || id == name) else {
                return Err(format!("“{name}” adlı katman yok; önceki dönemleri katman adlarıyla, eskiden yeniye yazın."));
            };
            out.push(kentos_commands::AsbuiltEpoch { layer: id.clone(), date: self.layer_day(id) });
        }
        Ok(out)
    }

    /// The package's input from the form: its design and limits, the notes a line each (empty lines left out).
    fn asbuilt_package_input(&self, path: &std::path::Path) -> Result<FieldPackage, String> {
        let s = &self.asbuilt;
        Ok(FieldPackage {
            design: opt(&s.design),
            lines: opt(&s.lines),
            surface: opt(&s.surface),
            field: opt(&s.field),
            horizontal: limit(&s.horizontal, "Yatay tolerans")?,
            vertical: limit(&s.vertical, "Düşey tolerans")?,
            quality: limit(&s.quality, "Kalite sınırı")?,
            notes: s.notes.text().lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_owned).collect(),
            path: path.display().to_string(),
            // The save dialog asked before a file was replaced.
            overwrite: true,
            stamp: None,
        })
    }

    /// Saha paketi yaz…'s file chosen: the package written by `cad.field.package`.
    pub(crate) fn asbuilt_package(&mut self, mut path: PathBuf) {
        if path.extension().is_none() {
            path.set_extension("zip");
        }
        let input = match self.asbuilt_package_input(&path) {
            Ok(input) => input,
            Err(why) => return self.warn(format!("Saha paketi: {why}")),
        };
        let Some(doc) = self.document.as_mut() else { return };
        let result = field_package::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            // docs/adr/3220: the surface goes in itself, its file and its triangles said.
            let surface = out.surface.as_ref().map_or_else(String::new, |s| match (&s.file, s.triangles) {
                (Some(file), Some(n)) => format!(", yüzey {} ({file}, {n} üçgen)", s.name),
                _ => format!(", yüzey {}", s.name),
            });
            self.say(
                Level::Success,
                format!(
                    "Saha paketi yazıldı: {} nokta, {} çizgi{surface}, {} not ({}).",
                    out.points.len(),
                    out.lines,
                    out.notes.len(),
                    path.display()
                ),
            );
        }
    }

    fn asbuilt_run(&mut self) {
        let input = self.asbuilt_input();
        let Some(doc) = self.document.as_mut() else { return };
        self.asbuilt.answer = Some(input.and_then(|input| {
            match asbuilt_check::plan(&ExecutionContext::new(&mut doc.model), &input) {
                CommandResult::Completed { output, .. } => Ok(output),
                CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                    Err(error.message)
                }
                _ => Err("Denetlenemedi.".to_owned()),
            }
        }));
        // Docs/adr/4420: the series' epochs as the table titles them.
        let epochs = match &self.asbuilt.answer {
            Some(Ok(r)) => r.tracks.first().map_or(0, |t| t.steps.len()),
            _ => 0,
        };
        self.asbuilt.epoch_titles =
            (1..epochs).map(|k| format!("{k}. dönem")).chain((epochs > 0).then(|| "Bu dönem".to_owned())).collect();
    }

    /// Uygulama kontrolü (`asbuilt.check`): the layers guessed by their names (a measured layer, a design layer, a
    /// kerb's lines), the first surface; no limit.
    pub(crate) fn open_asbuilt(&mut self) -> Task<Message> {
        let Some(doc) = self.document.as_ref() else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let leaves: Vec<(String, String)> =
            doc.model.layers().leaves().iter().map(|n| (n.id.clone(), n.name.clone())).collect();
        let holding = |what: fn(&Entity) -> bool| -> Vec<(String, String)> {
            leaves
                .iter()
                .filter(|(id, _)| doc.model.entities().any(|e| e.base().layer_id == *id && what(e)))
                .cloned()
                .collect()
        };
        // A terrain surface's own point layers are its sources, not a survey's: left out of the guesses.
        let sources: Vec<String> = doc
            .model
            .settings()
            .road
            .as_ref()
            .map(|r| r.surfaces.iter().flat_map(|s| s.points.layers.iter().cloned()).collect())
            .unwrap_or_default();
        let layers = holding(|e| matches!(e, Entity::Point(_)));
        let guessable: Vec<(String, String)> = layers.iter().filter(|(id, _)| !sources.contains(id)).cloned().collect();
        let line_layers = holding(|e| matches!(e, Entity::Line(_) | Entity::Polyline(_)));
        let find = |list: &[(String, String)], words: &[&str]| {
            list.iter().find(|(_, n)| words.iter().any(|w| n.to_lowercase().contains(w))).map(|(id, _)| id.clone())
        };
        let measured = find(&guessable, &["ölçül", "olcul", "ölçüm", "olcum", "saha", "gerçekleşen"])
            .or_else(|| guessable.first().map(|(id, _)| id.clone()))
            .unwrap_or_default();
        let design = find(&guessable, &["tasarım", "tasarim", "aplikasyon", "proje"]).filter(|d| *d != measured).unwrap_or_default();
        let lines = find(&line_layers, &["bordür", "bordur", "tasarım", "tasarim"]).unwrap_or_default();
        let surfaces: Vec<(String, String)> = doc
            .model
            .settings()
            .road
            .as_ref()
            .map(|r| r.surfaces.iter().map(|s| (s.id.clone(), s.name.clone())).collect())
            .unwrap_or_default();
        let surface = find(&surfaces, &["tasarım", "tasarim", "proje"]).unwrap_or_default();
        self.asbuilt = State {
            layers,
            line_layers,
            surfaces,
            measured,
            design,
            lines,
            surface,
            ..State::default()
        };
        self.asbuilt_run();
        self.dialog = Some(Dialog::Asbuilt);
        Task::none()
    }

    pub(crate) fn asbuilt_save(&mut self, path: PathBuf) {
        let Some(Ok(r)) = &self.asbuilt.answer else { return };
        match std::fs::write(&path, csv(r)) {
            Ok(()) => self.say(Level::Success, format!("{TITLE}: {} nokta {} dosyasına yazıldı.", r.points.len(), path.display())),
            Err(e) => self.warn(format!("{TITLE}: {} yazılamadı: {e}. Başka bir klasör seçin.", path.display())),
        }
    }

    pub(crate) fn asbuilt_event(&mut self, event: Event) -> Task<Message> {
        let s = &mut self.asbuilt;
        match event {
            Event::Measured(v) => s.measured = v,
            Event::Design(v) => s.design = v,
            Event::Kind(k) => s.kind = k.min(2),
            Event::Control(v) => s.control = v,
            Event::Epochs(v) => s.epochs = v,
            Event::Nearest(on) => s.nearest = on,
            Event::Lines(v) => s.lines = v,
            Event::Surface(v) => s.surface = v,
            Event::Field(v) => s.field = v,
            Event::Search(v) => s.search = v,
            Event::Horizontal(v) => s.horizontal = v,
            Event::Vertical(v) => s.vertical = v,
            Event::QualityField(v) => s.quality_field = v,
            Event::Quality(v) => s.quality = v,
            Event::Show(v) => {
                s.show = v;
                return Task::none();
            }
            Event::Select => {
                let Some(Ok(r)) = &self.asbuilt.answer else { return Task::none() };
                let Some(doc) = self.document.as_ref() else { return Task::none() };
                let slots: Vec<Slot> = r
                    .points
                    .iter()
                    .filter(|p| p.status != "within")
                    .filter_map(|p| Uuid::parse_str(&p.uid).ok().and_then(|u| doc.model.slot_of(u)))
                    .collect();
                let n = slots.len();
                self.selection.set(slots);
                self.asbuilt = State::default();
                self.dialog = None;
                self.say(Level::Info, format!("{TITLE}: içinde olmayan {n} nokta seçildi."));
                return Task::none();
            }
            Event::Save => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title(TITLE)
                            .set_file_name("uygulama-kontrolu.csv")
                            .add_filter("CSV", &["csv"])
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::Saved(p)),
                );
            }
            Event::Saved(Some(path)) => {
                self.asbuilt_save(path);
                return Task::none();
            }
            Event::Saved(None) => return Task::none(),
            Event::Notes(action) => {
                s.notes.perform(action);
                return Task::none();
            }
            Event::Package => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Saha paketi")
                            .set_file_name("saha-paketi.zip")
                            .add_filter("Saha paketi (.zip)", &["zip"])
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::Packaged(p)),
                );
            }
            Event::Packaged(Some(path)) => {
                self.asbuilt_package(path);
                return Task::none();
            }
            Event::Packaged(None) => return Task::none(),
            Event::PickPackage => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Kaydın saha paketi")
                            .add_filter("Saha paketi (.zip)", &["zip"])
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::PickedPackage(p)),
                );
            }
            Event::PickedPackage(Some(path)) => s.record_package = path.display().to_string(),
            Event::PickedPackage(None) => return Task::none(),
            Event::DropPackage => {
                s.record_package.clear();
                s.record_design = false;
            }
            Event::PackageDesign(on) => s.record_design = on,
            Event::Close => {
                self.asbuilt = State::default();
                self.dialog = None;
                return Task::none();
            }
        }
        self.asbuilt_run();
        Task::none()
    }

    pub(crate) fn asbuilt_view(&self) -> Element<'_, Message> {
        let s = &self.asbuilt;
        let f = self.format();
        let text = |value: &str, hint: &str, on: fn(String) -> Event, width: f32| {
            container(fields::input(None, hint, value, false, false).on_input(move |v| msg(on(v)))).width(width)
        };
        let with_none = |list: &[(String, String)]| {
            let mut v = vec![(NONE.to_owned(), "—".to_owned())];
            v.extend(list.iter().cloned());
            v
        };
        let package = !(s.design.is_empty() && s.lines.is_empty());
        // A monitoring job (docs/adr/3620): this epoch's and the earlier epoch's marks, the control points and the
        // movement limits; no design parts, no field package.
        let watch = s.kind == 2;
        let kinds: Vec<(String, String)> = KINDS.iter().enumerate().map(|(i, k)| (i.to_string(), (*k).to_owned())).collect();
        let mut form = Form::new()
            .label_width(170.0)
            .field(
                "İş türü",
                container(fields::select_owned(kinds, &s.kind.to_string(), |v| msg(Event::Kind(v.parse().unwrap_or(0))))).width(260),
            )
            .field(
                if watch { "Bu dönem" } else { "Ölçülen" },
                row![
                    container(fields::select_owned(s.layers.clone(), &s.measured, |v| msg(Event::Measured(v)))).width(200),
                    label::caption("Ad alanı"),
                    text(&s.field, "Ad", Event::Field, 100.0)
                ]
                .spacing(8)
                .align_y(Center),
            );
        form = if watch {
            form.field(
                "Önceki dönem",
                container(fields::select_owned(with_none(&s.layers), &s.design, |v| msg(Event::Design(v)))).width(200),
            )
            .help("Noktalar adla eşlenir; fark bu dönem − önceki dönem, yani noktanın hareketi.")
            .field("Kontrol noktaları", text(&s.control, "ör. K1, K2", Event::Control, 260.0))
            .help("Oynamaması gereken noktalar: biri sınırı aşarsa iki dönem aynı datumda değildir ya da nokta oynamıştır.")
            // Docs/adr/4420: the series through the earlier epochs.
            .field("Önceki dönemler", text(&s.epochs, "ör. Birinci dönem, İkinci dönem", Event::Epochs, 260.0))
            .help("Katman adları eskiden yeniye: her nokta ilk ölçüldüğü döneme göre izlenir; günler katmanların Veri kümesi bilgisinden (hız için).")
        } else {
            form.field(
                "Tasarım noktaları",
                row![
                    container(fields::select_owned(with_none(&s.layers), &s.design, |v| msg(Event::Design(v)))).width(200),
                    label::caption("Arama yarıçapı (m)"),
                    text(&s.search, "—", Event::Search, 80.0)
                ]
                .spacing(8)
                .align_y(Center),
            )
            .field(
                "Tasarım çizgileri",
                row![
                    container(fields::select_owned(with_none(&s.line_layers), &s.lines, |v| msg(Event::Lines(v)))).width(200),
                    label::caption("Yüzey"),
                    container(fields::select_owned(with_none(&s.surfaces), &s.surface, |v| msg(Event::Surface(v)))).width(160)
                ]
                .spacing(8)
                .align_y(Center),
            )
            // Docs/adr/4020: the surface's limit on the height difference or on the nearest distance.
            .field(
                "Yüzey farkı",
                Segmented::new([SurfaceBy(false), SurfaceBy(true)], SurfaceBy(s.nearest), |v| msg(Event::Nearest(v.0))),
            )
            .help("Düşey fark noktanın kotu eksi yüzeyinki; en yakın uzaklık noktadan yüzeye en kısa uzaklık (işareti düşey farkınki). Düşey tolerans seçilene uygulanır.")
            .field(
                "Kaydın saha paketi",
                row![
                    container(fields::input(None, "—", package_name(&s.record_package), false, false)).width(200),
                    words::secondary(PICK_PACKAGE, Some(msg(Event::PickPackage))),
                    words::secondary(DROP_PACKAGE, (!s.record_package.is_empty()).then(|| msg(Event::DropPackage))),
                    words::check(
                        s.record_design,
                        PACKAGE_DESIGN,
                        (!s.record_package.is_empty()).then(|| msg(Event::PackageDesign(!s.record_design)))
                    )
                ]
                .spacing(8)
                .align_y(Center),
            )
        };
        form = form
            .field(
                if watch { "Hareket sınırı yatay / düşey (m)" } else { "Tolerans yatay / düşey (m)" },
                row![text(&s.horizontal, "—", Event::Horizontal, 80.0), text(&s.vertical, "—", Event::Vertical, 80.0)]
                    .spacing(8)
                    .align_y(Center),
            )
            .field(
                "Kalite",
                row![
                    text(&s.quality_field, "σ özniteliği", Event::QualityField, 140.0),
                    label::caption("en çok σ (m)"),
                    text(&s.quality, "—", Event::Quality, 80.0)
                ]
                .spacing(8)
                .align_y(Center),
            );
        if !watch {
            form = form
                .section("Saha paketi")
                .field(
                    "Kalite notları",
                    text_editor(&s.notes)
                        .placeholder("Her satır bir not: başlanacak kontrol noktaları, kot sistemi, alet…")
                        .on_action(|a| msg(Event::Notes(a)))
                        .height(56)
                        .padding(6)
                        .size(kentos_ui::theme::typography::body()),
                )
                .help("Tasarım noktaları, çizgileri, yüzey, toleranslar ve kalite sınırı notlarla tek .zip olur; çizim değişmez.")
                .row(words::secondary(PACKAGE, package.then(|| msg(Event::Package))));
        }
        let mut lines = Vec::new();
        let mut rows = Vec::new();
        let mut faults = 0;
        match &s.answer {
            Some(Ok(r)) => {
                faults = r.points.iter().filter(|p| p.status != "within").count();
                let watch = watched(r);
                lines.push(words::text_line(
                    if r.outside + r.unmatched + r.ambiguous == 0 { Kind::Ok } else { Kind::Warn },
                    if watch {
                        format!(
                            "Bu dönemde {} nokta: {} sınır içinde, {} sınırı aştı, {} önceki dönemde yok, {} belirsiz; önceki dönemin {} noktası bu dönemde ölçülmedi.",
                            r.points.len(),
                            r.within,
                            r.outside,
                            r.unmatched,
                            r.ambiguous,
                            r.missing.len()
                        )
                    } else {
                        format!(
                            "{} ölçülen nokta: {} içinde, {} dışında, {} eşleşmedi, {} belirsiz; {} tasarım noktası ölçülmedi.",
                            r.points.len(),
                            r.within,
                            r.outside,
                            r.unmatched,
                            r.ambiguous,
                            r.missing.len()
                        )
                    },
                ));
                let o = |v: Option<f64>| v.map_or("—".to_owned(), |v| f.length(v));
                lines.push(words::text_line(
                    Kind::Info,
                    format!(
                        "{}: yatay KOH {}, en büyük {}; düşey KOH {}, en büyük {}.",
                        if watch { "Hareketler" } else { "Eşlenenler" },
                        o(r.rms_horizontal),
                        o(r.max_horizontal),
                        o(r.rms_vertical),
                        o(r.max_vertical)
                    ),
                ));
                if r.low_quality > 0 {
                    lines.push(words::text_line(
                        Kind::Warn,
                        format!(
                            "{} noktanın σ'sı kalite sınırının üstünde: ölçüm çözümü zayıf, {} sayılmadı.",
                            r.low_quality,
                            if watch { "hareket" } else { "yapım hatası" }
                        ),
                    ));
                }
                // With the package's own design (docs/adr/2820) and surface (docs/adr/3220) the record is checked against
                // what the field had; the drawing's change is said, not an error of the record.
                for (kind, said) in changes(r) {
                    lines.push(words::text_line(kind, said));
                }
                if let Some(p) = &r.package {
                    lines.extend(package_lines(p, &f));
                }
                // A monitoring job's control points (docs/adr/3620).
                if let Some(c) = &r.control {
                    if !c.moved.is_empty() {
                        lines.push(words::text_line(
                            Kind::Warn,
                            format!(
                                "Kontrol noktası oynadı: {}. İki dönem aynı datumda değil ya da nokta sabit değil; hareketleri buna göre okuyun.",
                                c.moved.join(", ")
                            ),
                        ));
                    } else if !c.checked.is_empty() {
                        lines.push(words::text_line(Kind::Ok, format!("Kontrol noktaları sabit: {}.", c.checked.join(", "))));
                    }
                    if !c.missing.is_empty() {
                        lines.push(words::text_line(
                            Kind::Warn,
                            format!("İki dönemde de ölçülmeyen kontrol noktası: {}.", c.missing.join(", ")),
                        ));
                    }
                }
                if !r.missing.is_empty() {
                    lines.push(words::text_line(
                        Kind::Info,
                        format!("Ölçülmeyen: {}.", r.missing.iter().map(|m| m.name.as_str()).collect::<Vec<_>>().join(", ")),
                    ));
                }
                lines.push(words::text_line(
                    Kind::Info,
                    if watch {
                        "Farklar bu dönem − önceki dönem, yani noktanın hareketi. Sınırlar sizin (boş: denetlenmez); çizime bir şey yazılmaz."
                    } else {
                        "Farklar ölçülen − tasarım; çizgiye göre uzaklık çizgi yönünün solunda artı. Sınırlar sizin (boş: denetlenmez); çizime bir şey yazılmaz."
                    },
                ));
                for p in r.points.iter().filter(|p| s.show == Show::All || p.status != "within") {
                    rows.push(if watch { watch_row(p) } else { point_row(p) });
                }
            }
            Some(Err(why)) => lines.push(words::text_line(Kind::Error, why.clone())),
            None => {}
        }
        // A monitoring job's table (docs/adr/3620): the movement and its horizontal length; no design columns.
        let columns = if matches!(&s.answer, Some(Ok(r)) if watched(r)) {
            vec![
                col("Nokta", 70.0),
                num("ΔDoğu", 70.0),
                num("ΔKuzey", 70.0),
                num("ΔKot", 70.0),
                num("Yatay", 70.0),
                col("Durum", 110.0),
                col("İşaretler", 230.0),
            ]
        } else {
            vec![
                col("Nokta", 70.0),
                col("Tasarım", 70.0),
                num("ΔDoğu", 70.0),
                num("ΔKuzey", 70.0),
                num("ΔKot", 70.0),
                num("Yüzey", 70.0),
                // Docs/adr/4020: the nearest distance to the surface.
                num("En yakın", 70.0),
                num("Çizgi", 70.0),
                col("Durum", 80.0),
                col("İşaretler", 160.0),
            ]
        };
        let table = sheet(columns, rows, "Bu süzgeçte nokta yok.", 260.0);
        // Docs/adr/4420: a monitoring job's series: each point's horizontal movement from its first epoch at every
        // epoch (— where it was not measured or before it), its last height movement and its mean rates.
        let series = match &s.answer {
            Some(Ok(r)) if watched(r) && !r.tracks.is_empty() => {
                let mut columns = vec![col("Nokta", 70.0)];
                for title in &s.epoch_titles {
                    columns.push(num(title, 80.0));
                }
                columns.extend([num("ΔKot", 70.0), num("Hız m/yıl", 80.0), num("Düşey m/yıl", 80.0)]);
                let comma = |v: f64| format!("{v:.3}").replace('.', ",");
                let rows: Vec<Vec<String>> = r
                    .tracks
                    .iter()
                    .map(|t| {
                        let mut row = vec![t.name.clone()];
                        row.extend(t.steps.iter().map(|st| match st {
                            Some(st) if st.outside => format!("{} !", comma(st.horizontal)),
                            Some(st) => comma(st.horizontal),
                            None => "—".to_owned(),
                        }));
                        row.push(t.steps.last().cloned().flatten().and_then(|st| st.dz).map(mm).unwrap_or_default());
                        row.push(t.rate_horizontal.map(comma).unwrap_or_default());
                        row.push(t.rate_vertical.map(comma).unwrap_or_default());
                        row
                    })
                    .collect();
                let lead = format!(
                    "Dönem dizisi: {} nokta ilk ölçüldüğü döneme göre izlendi (yatay hareket, m; ! sınırı aştı).",
                    r.tracks.len()
                );
                Some((lead, sheet(columns, rows, "İzlenen nokta yok.", 200.0)))
            }
            _ => None,
        };
        let mut body = Grows::new()
            .spacing(12.0)
            .width(iced::Fill)
            .push(form)
            .push(words::summary(lines))
            .push(Segmented::new([Show::All, Show::Faults], s.show, |v| msg(Event::Show(v))))
            .push_fit(table, kentos_ui::theme::typography::scaled(120.0));
        if let Some((lead, series)) = series {
            body = body.push(label::caption(lead)).push_fit(series, kentos_ui::theme::typography::scaled(100.0));
        }
        let ready = matches!(&s.answer, Some(Ok(r)) if !r.points.is_empty());
        Frame::new(TITLE)
            .scroll(body)
            .aside(words::secondary(SAVE, ready.then(|| msg(Event::Save))))
            .aside(words::secondary(SELECT, (faults > 0).then(|| msg(Event::Select))))
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(1000.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn asbuilt_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(SELECT) => Some(msg(Event::Select)),
            Control::Press(PACKAGE) => Some(msg(Event::Package)),
            Control::Press(PICK_PACKAGE) => Some(msg(Event::PickPackage)),
            Control::Press(DROP_PACKAGE) => Some(msg(Event::DropPackage)),
            Control::Check(PACKAGE_DESIGN, on) => Some(msg(Event::PackageDesign(on))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// A monitoring job's row (docs/adr/3620): the mark, its movement and the movement's horizontal length.
fn watch_row(p: &AsbuiltPoint) -> Vec<String> {
    let o = |v: Option<f64>| v.map(mm).unwrap_or_default();
    vec![
        p.name.clone(),
        o(p.de),
        o(p.dn),
        o(p.dh),
        // A length, never negative: no sign.
        p.horizontal.map(|v| format!("{v:.3}").replace('.', ",")).unwrap_or_default(),
        watch_status_word(&p.status).to_owned(),
        p.flags.iter().map(|f| watch_flag_word(f)).collect::<Vec<_>>().join(", "),
    ]
}

fn point_row(p: &AsbuiltPoint) -> Vec<String> {
    let o = |v: Option<f64>| v.map(mm).unwrap_or_default();
    vec![
        p.name.clone(),
        p.design_name.clone().unwrap_or_default(),
        o(p.de),
        o(p.dn),
        o(p.dh),
        o(p.surface),
        o(p.nearest),
        o(p.normal),
        status_word(&p.status).to_owned(),
        p.flags.iter().map(|f| flag_word(f)).collect::<Vec<_>>().join(", "),
    ]
}

/// Yüzey farkı's two choices (docs/adr/4020): the height difference or the nearest distance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SurfaceBy(bool);

impl std::fmt::Display for SurfaceBy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.0 { "En yakın uzaklık" } else { "Düşey fark" })
    }
}
