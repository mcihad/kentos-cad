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
//! file's name is asked, the drawing is not changed.

use std::path::PathBuf;

use iced::widget::{container, row, text_editor};
use iced::{Center, Element, Task};
use kentos_commands::{AsbuiltCheck, AsbuiltChecked, AsbuiltPoint, CommandResult, Entity, FieldPackage};
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
/// The choice that leaves a layer or the surface out.
const NONE: &str = "";

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
}

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
    let mut out = String::from("Nokta;Nesne;Tasarım noktası;Eşleme;ΔDoğu (m);ΔKuzey (m);ΔKot (m);Yatay (m);Yüzeye göre (m);Çizgiye göre (m);σ (m);Durum;İşaretler\n");
    let o = |v: Option<f64>| v.map(|v| v.to_string()).unwrap_or_default();
    for p in &r.points {
        out.push_str(&format!(
            "{};{};{};{};{};{};{};{};{};{};{};{};{}\n",
            p.name,
            p.uid,
            p.design_name.as_deref().unwrap_or(""),
            p.by.as_deref().unwrap_or(""),
            o(p.de),
            o(p.dn),
            o(p.dh),
            o(p.horizontal),
            o(p.surface),
            o(p.normal),
            o(p.sigma),
            status_word(&p.status),
            p.flags.iter().map(|f| flag_word(f)).collect::<Vec<_>>().join(", ")
        ));
    }
    out
}

impl App {
    fn asbuilt_input(&self) -> Result<AsbuiltCheck, String> {
        let s = &self.asbuilt;
        Ok(AsbuiltCheck {
            measured: s.measured.clone(),
            design: opt(&s.design),
            surface: opt(&s.surface),
            lines: opt(&s.lines),
            field: opt(&s.field),
            search: limit(&s.search, "Arama yarıçapı")?,
            horizontal: limit(&s.horizontal, "Yatay tolerans")?,
            vertical: limit(&s.vertical, "Düşey tolerans")?,
            quality_field: opt(&s.quality_field),
            quality: limit(&s.quality, "Kalite sınırı")?,
            expected_design: None,
        })
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
            let surface = out.surface.as_ref().map_or_else(String::new, |s| format!(", yüzey {}", s.name));
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
        let form = Form::new()
            .label_width(170.0)
            .field(
                "Ölçülen",
                row![
                    container(fields::select_owned(s.layers.clone(), &s.measured, |v| msg(Event::Measured(v)))).width(200),
                    label::caption("Ad alanı"),
                    text(&s.field, "Ad", Event::Field, 100.0)
                ]
                .spacing(8)
                .align_y(Center),
            )
            .field(
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
            .field(
                "Tolerans yatay / düşey (m)",
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
            )
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
        let mut lines = Vec::new();
        let mut rows = Vec::new();
        let mut faults = 0;
        match &s.answer {
            Some(Ok(r)) => {
                faults = r.points.iter().filter(|p| p.status != "within").count();
                lines.push(words::text_line(
                    if r.outside + r.unmatched + r.ambiguous == 0 { Kind::Ok } else { Kind::Warn },
                    format!(
                        "{} ölçülen nokta: {} içinde, {} dışında, {} eşleşmedi, {} belirsiz; {} tasarım noktası ölçülmedi.",
                        r.points.len(),
                        r.within,
                        r.outside,
                        r.unmatched,
                        r.ambiguous,
                        r.missing.len()
                    ),
                ));
                let o = |v: Option<f64>| v.map_or("—".to_owned(), |v| f.length(v));
                lines.push(words::text_line(
                    Kind::Info,
                    format!(
                        "Eşlenenler: yatay KOH {}, en büyük {}; düşey KOH {}, en büyük {}.",
                        o(r.rms_horizontal),
                        o(r.max_horizontal),
                        o(r.rms_vertical),
                        o(r.max_vertical)
                    ),
                ));
                if r.low_quality > 0 {
                    lines.push(words::text_line(
                        Kind::Warn,
                        format!("{} noktanın σ'sı kalite sınırının üstünde: ölçüm çözümü zayıf, yapım hatası sayılmadı.", r.low_quality),
                    ));
                }
                if r.design_changed {
                    lines.push(words::text_line(Kind::Error, "Tasarım, saha kaydının yapıldığı tasarımdan değişmiş."));
                }
                if !r.missing.is_empty() {
                    lines.push(words::text_line(
                        Kind::Info,
                        format!("Ölçülmeyen: {}.", r.missing.iter().map(|m| m.name.as_str()).collect::<Vec<_>>().join(", ")),
                    ));
                }
                lines.push(words::text_line(
                    Kind::Info,
                    "Farklar ölçülen − tasarım; çizgiye göre uzaklık çizgi yönünün solunda artı. Sınırlar sizin (boş: denetlenmez); çizime bir şey yazılmaz.",
                ));
                for p in r.points.iter().filter(|p| s.show == Show::All || p.status != "within") {
                    rows.push(point_row(p));
                }
            }
            Some(Err(why)) => lines.push(words::text_line(Kind::Error, why.clone())),
            None => {}
        }
        let table = sheet(
            vec![
                col("Nokta", 70.0),
                col("Tasarım", 70.0),
                num("ΔDoğu", 70.0),
                num("ΔKuzey", 70.0),
                num("ΔKot", 70.0),
                num("Yüzey", 70.0),
                num("Çizgi", 70.0),
                col("Durum", 80.0),
                col("İşaretler", 230.0),
            ],
            rows,
            "Bu süzgeçte nokta yok.",
            260.0,
        );
        let body = Grows::new()
            .spacing(12.0)
            .width(iced::Fill)
            .push(form)
            .push(words::summary(lines))
            .push(Segmented::new([Show::All, Show::Faults], s.show, |v| msg(Event::Show(v))))
            .push_fit(table, kentos_ui::theme::typography::scaled(120.0));
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
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
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
        o(p.normal),
        status_word(&p.status).to_owned(),
        p.flags.iter().map(|f| flag_word(f)).collect::<Vec<_>>().join(", "),
    ]
}
