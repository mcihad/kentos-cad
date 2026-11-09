//! Düzenleme dağıtım taslağı (docs/adr/1460; `readjustment.distribute`): a land readjustment's first numbers by
//! `cad.readjustment.distribute`, read only and always a draft. The rule package is typed here (who issues it, the
//! document, when it holds, its release, the deduction ratio, the numeric policy); the records come from a CSV
//! (`parsel;hak sahibi;tapu alanı;pay`, holders by id, not by name). The answer shows the holders' totals or the
//! parcels (deduction, net, the remainder and where it went); Sonucu kaydet writes it as CSV. Nothing is written to
//! the drawing; no official decision is made.
//!
//! The scope (docs/adr/2060; `cad.readjustment.scope`): the area selected when the window opens is the boundary
//! (düzenleme sınırı). Kapsam shows the separation sheet's numbers of the parcels it takes in and the basis of each
//! (Yönetmelik m. 31), with the rule package's places and rounding; Esas alanları kayıtlara yaz puts each basis into the
//! records of its parcel and says what changed, what kept its file area and what did not match.

use std::path::PathBuf;

use iced::widget::{container, row, text_input};
use iced::{Center, Element, Task};
use kentos_commands::{
    CommandResult, Entity, ReadjustmentDistribute, ReadjustmentDistributed, ReadjustmentEntry, ReadjustmentRules,
    ReadjustmentScope, ReadjustmentScopeParcel, ReadjustmentScoped,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, readjustment, readjustment_scope};
use kentos_ui::label;
use kentos_ui::widget::{Dialog as Frame, Form, NumberInput, Segmented};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, num, sheet};
use crate::style::fields;
use crate::traces::Control;

pub const TITLE: &str = "Düzenleme dağıtım taslağı";
const CLOSE: &str = "Kapat";
const OPEN: &str = "Kayıtları seç…";
const SAVE: &str = "Sonucu kaydet…";
const APPLY: &str = "Esas alanları kayıtlara yaz";
const MODES: [(&str, &str); 4] = [
    ("half_even", "Yarımda çifte"),
    ("half_away_from_zero", "Yarımda sıfırdan uzağa"),
    ("toward_zero", "Sıfıra doğru"),
    ("away_from_zero", "Sıfırdan uzağa"),
];
const RULES: [(&str, &str); 2] = [("largest_remainder", "En büyük artık"), ("reject", "Artığı reddet")];

/// What the table shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum View {
    #[default]
    Holders,
    Parcels,
    Scope,
    /// Ayırma çapı (docs/adr/2280): the partial parcels' parts, corner by corner.
    Sheet,
}

impl std::fmt::Display for View {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            View::Holders => "Hak sahipleri",
            View::Parcels => "Parseller",
            View::Scope => "Kapsam",
            View::Sheet => "Ayırma çapı",
        })
    }
}

#[derive(Clone, Debug)]
pub struct State {
    pub rules: ReadjustmentRules,
    pub entries: Vec<ReadjustmentEntry>,
    pub file: Option<String>,
    pub view: View,
    pub answer: Option<Result<ReadjustmentDistributed, String>>,
    /// The boundary's uid: the area selected when the window opened (docs/adr/2060).
    pub boundary: Option<String>,
    /// The narrowest piece counted (metres).
    pub tolerance: f64,
    pub scope: Option<Result<ReadjustmentScoped, String>>,
    /// What Esas alanları kayıtlara yaz did, a line each (true: to look at).
    pub applied: Vec<(bool, String)>,
    /// The parcel numbers of the parts outside, as typed: `Ada/Parsel: n, n; …` (docs/adr/3080).
    pub numbers: String,
    /// The numbers' height on paper, millimetres, as typed (docs/adr/4280); empty: labelled points.
    pub number_height: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            rules: ReadjustmentRules {
                id: String::new(),
                release: 1,
                jurisdiction: String::new(),
                source: String::new(),
                effective: String::new(),
                approved: false,
                deduction: String::new(),
                scale: 2,
                rounding: "half_even".into(),
                remainder: "largest_remainder".into(),
            },
            entries: Vec::new(),
            file: None,
            view: View::default(),
            answer: None,
            boundary: None,
            tolerance: readjustment_scope::FIRST_TOLERANCE,
            scope: None,
            applied: Vec::new(),
            numbers: String::new(),
            number_height: String::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Id(String),
    Release(f64),
    Jurisdiction(String),
    Source(String),
    Effective(String),
    Deduction(String),
    Scale(f64),
    Rounding(String),
    Remainder(String),
    Pick,
    Picked(Option<PathBuf>),
    View(View),
    Tolerance(f64),
    Apply,
    /// Çapı çizime yaz (docs/adr/2280).
    WriteSheet,
    /// Çapın paftaları (docs/adr/2680).
    MakeSheets,
    /// The parcel numbers of the parts outside, typed (docs/adr/3080).
    Numbers(String),
    /// Their height on paper, millimetres, typed (docs/adr/4280).
    NumberHeight(String),
    Save,
    Saved(Option<PathBuf>),
    Close,
}

fn msg(e: Event) -> Message {
    Message::Readjustment(e)
}

/// The records of a CSV: `parsel;hak sahibi;tapu alanı;pay` a line (a head line whose third column is not a number is
/// skipped); a line of another shape is said by its number.
pub fn records(text: &str) -> Result<Vec<ReadjustmentEntry>, String> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let cells: Vec<&str> = line.split(';').map(str::trim).collect();
        if cells.len() != 4 {
            return Err(format!("{}. satır dört sütun değil (parsel;hak sahibi;tapu alanı;pay).", n + 1));
        }
        if n == 0 && !cells[2].chars().next().is_some_and(|c| c.is_ascii_digit()) {
            continue;
        }
        out.push(ReadjustmentEntry {
            parcel: cells[0].to_owned(),
            holder: cells[1].to_owned(),
            area: cells[2].to_owned(),
            share: cells[3].to_owned(),
        });
    }
    Ok(out)
}

/// The answer as CSV: the package, then a line a holder's part, then the holders' totals.
pub fn csv(r: &ReadjustmentDistributed) -> String {
    let mut out = format!(
        "Kural paketi;{} (sürüm {});Taslak;{}\nTapu alanı;{};Kesinti;{};Net;{};Hak sahiplerinin toplamı;{};Toplam korunuyor;{}\n\nParsel;Hak sahibi;Pay;Kesin değer (m²);Değer (m²)\n",
        r.rules,
        r.release,
        if r.draft { "evet" } else { "hayır" },
        r.registered,
        r.deduction,
        r.net,
        r.held,
        if r.conserved { "evet" } else { "hayır" }
    );
    for p in &r.parcels {
        for x in &p.parts {
            out.push_str(&format!("{};{};{};{};{}\n", p.parcel, x.holder, x.share, x.exact, x.value));
        }
    }
    out.push_str("\nHak sahibi;Toplam (m²)\n");
    for h in &r.holders {
        out.push_str(&format!("{};{}\n", h.holder, h.value));
    }
    out
}

/// A scope parcel's status in words (with what the part outside keeps when it is said).
pub fn status_word(p: &ReadjustmentScopeParcel) -> String {
    let rest = p.outside_recorded.as_deref().unwrap_or("");
    let mut out = match p.status.as_str() {
        "registered" => "Tescilli esas".to_owned(),
        "computed" if p.pieces > 1 => format!("Giren kısım esas; {} parçada {rest} kalır", p.pieces),
        "computed" => format!("Giren kısım esas; dışarıda {rest} kalır"),
        "needsCorrection" => "Yanılma sınırı dışında: düzeltme gerekir".to_owned(),
        "noRecord" => "Tescilli yüzölçümü yok".to_owned(),
        "invalidRecord" => "Kayıt okunmuyor".to_owned(),
        "finerThanPolicy" => "Tescilli, basamaktan ince".to_owned(),
        "outsideTooSmall" => format!("Girmeyen hatayı taşımıyor ({rest})"),
        "sliver" => "Sınır kenara çok yakın: şerit".to_owned(),
        "outside" => "Sınırın dışında".to_owned(),
        other => other.to_owned(),
    };
    if p.duplicate {
        out.push_str(" · aynı Ada/Parsel");
    }
    out
}

/// Each record whose parcel has a basis in the scope (and is the only one of its name) takes the basis as its area;
/// the lines: what changed, the parcels whose records kept the file's area (no basis), the scope's parcels without a
/// record, the records' parcels outside the scope.
pub fn apply(entries: &mut [ReadjustmentEntry], scope: &ReadjustmentScoped) -> Vec<(bool, String)> {
    let mut changed = Vec::new();
    let mut kept = Vec::new();
    let mut unrecorded = Vec::new();
    for p in &scope.parcels {
        let mine: Vec<usize> = (0..entries.len()).filter(|&i| entries[i].parcel.trim() == p.parcel).collect();
        if mine.is_empty() {
            if p.kind != "outside" {
                unrecorded.push(p.parcel.clone());
            }
            continue;
        }
        match (&p.basis, p.duplicate) {
            (Some(b), false) => {
                let before = entries[mine[0]].area.clone();
                for &i in &mine {
                    entries[i].area = b.clone();
                }
                if before.trim() != b {
                    changed.push(format!("{} {before} → {b}", p.parcel));
                }
            }
            _ => kept.push(format!("{} ({})", p.parcel, if p.duplicate { "aynı Ada/Parsel" } else { status_short(&p.status) })),
        }
    }
    let mut outside: Vec<String> = Vec::new();
    for e in entries.iter() {
        let name = e.parcel.trim();
        if !scope.parcels.iter().any(|p| p.parcel == name) && !outside.iter().any(|o| o == name) {
            outside.push(name.to_owned());
        }
    }
    let mut lines = vec![(
        false,
        if changed.is_empty() {
            "Esas alanlar kayıtlara yazıldı; hiçbir kaydın alanı değişmedi.".to_owned()
        } else {
            format!("{} parselin kayıtlarında alan esas alanla değişti: {}.", changed.len(), changed.join("; "))
        },
    )];
    if !kept.is_empty() {
        lines.push((true, format!("Esası olmayan parsellerin kayıtları dosyadaki alanla kaldı, inceleyin: {}.", kept.join(", "))));
    }
    if !unrecorded.is_empty() {
        lines.push((true, format!("Kapsamda olup kaydı olmayan parsel: {}.", unrecorded.join(", "))));
    }
    if !outside.is_empty() {
        lines.push((true, format!("Kapsamda olmayan kayıt parseli: {}.", outside.join(", "))));
    }
    lines
}

/// A status in a few words, for a list.
fn status_short(status: &str) -> &'static str {
    match status {
        "needsCorrection" => "yanılma sınırı dışında",
        "noRecord" => "tescilli yüzölçümü yok",
        "invalidRecord" => "kayıt okunmuyor",
        "finerThanPolicy" => "politikadan ince",
        "outsideTooSmall" => "girmeyen kısım hatayı taşımıyor",
        "sliver" => "şerit",
        "outside" => "dışarıda",
        _ => "esas yok",
    }
}

/// The one selected object's uid when it is an area: the boundary.
fn chosen_boundary(app: &App) -> Option<String> {
    let doc = app.document.as_ref()?;
    let [slot] = app.selection.ids() else { return None };
    matches!(doc.model.get(*slot)?, Entity::Polygon(_)).then(|| doc.model.uid(*slot).map(|u| u.to_string())).flatten()
}

impl App {
    /// The scope's input: the boundary with the rule package's places and rounding and the piece tolerance.
    pub(crate) fn readjustment_scope_input(&self) -> Option<ReadjustmentScope> {
        let s = &self.readjustment;
        Some(ReadjustmentScope {
            boundary: s.boundary.clone()?,
            uids: None,
            field: None,
            scale: s.rules.scale,
            rounding: s.rules.rounding.clone(),
            tolerance: Some(s.tolerance),
        })
    }

    /// The scope of the boundary with the rule package's places and rounding.
    pub(crate) fn readjustment_scope_run(&mut self) {
        let Some(input) = self.readjustment_scope_input() else {
            self.readjustment.scope = None;
            return;
        };
        let Some(doc) = self.document.as_ref() else { return };
        self.readjustment.scope = Some(readjustment_scope::preview(&doc.model, &input));
    }

    pub(crate) fn readjustment_run(&mut self) {
        let s = &self.readjustment;
        if s.entries.is_empty() {
            self.readjustment.answer = None;
            return;
        }
        let input = ReadjustmentDistribute { rules: s.rules.clone(), entries: s.entries.clone() };
        let Some(doc) = self.document.as_mut() else { return };
        self.readjustment.answer = Some(match readjustment::plan(&ExecutionContext::new(&mut doc.model), &input) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err(match error.path {
                    Some(p) => format!("{} ({p})", error.message),
                    None => error.message,
                })
            }
            _ => Err("Hesaplanamadı.".to_owned()),
        });
    }

    /// Düzenleme dağıtım taslağı (`readjustment.distribute`).
    pub(crate) fn open_readjustment(&mut self) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return Task::none();
        }
        self.readjustment = State { boundary: chosen_boundary(self), ..State::default() };
        self.readjustment_scope_run();
        self.dialog = Some(Dialog::Readjustment);
        Task::none()
    }

    /// The records read from `path` (a test or a trace gives it here, the picker in the app).
    pub(crate) fn readjustment_file(&mut self, path: PathBuf) {
        match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|t| records(&t)) {
            Ok(entries) => {
                self.readjustment.entries = entries;
                self.readjustment.file = path.file_name().map(|n| n.to_string_lossy().into_owned());
                self.readjustment_run();
            }
            Err(why) => self.readjustment.answer = Some(Err(format!("{} okunamadı: {why}", path.display()))),
        }
    }

    pub(crate) fn readjustment_event(&mut self, event: Event) -> Task<Message> {
        let r = &mut self.readjustment.rules;
        match event {
            Event::Id(v) => r.id = v,
            Event::Release(v) => r.release = v.max(1.0) as u32,
            Event::Jurisdiction(v) => r.jurisdiction = v,
            Event::Source(v) => r.source = v,
            Event::Effective(v) => r.effective = v,
            Event::Deduction(v) => r.deduction = v,
            Event::Scale(v) => {
                r.scale = v.max(0.0) as u32;
                self.readjustment_scope_run();
            }
            Event::Rounding(v) => {
                r.rounding = v;
                self.readjustment_scope_run();
            }
            Event::Tolerance(v) => {
                self.readjustment.tolerance = v;
                self.readjustment_scope_run();
                return Task::none();
            }
            Event::WriteSheet => self.readjustment_sheet_write(),
            Event::MakeSheets => return self.readjustment_sheets_make(),
            Event::Numbers(t) => self.readjustment.numbers = t,
            Event::NumberHeight(t) => self.readjustment.number_height = t,
            Event::Apply => {
                if let Some(Ok(scope)) = &self.readjustment.scope {
                    let scope = scope.clone();
                    self.readjustment.applied = apply(&mut self.readjustment.entries, &scope);
                }
            }
            Event::Remainder(v) => r.remainder = v,
            Event::View(v) => {
                self.readjustment.view = v;
                return Task::none();
            }
            Event::Pick => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Düzenleme kayıtları")
                            .add_filter("CSV", &["csv", "txt"])
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::Picked(p)),
                );
            }
            Event::Picked(Some(path)) => {
                self.readjustment_file(path);
                return Task::none();
            }
            Event::Picked(None) | Event::Saved(None) => return Task::none(),
            Event::Save => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title(TITLE)
                            .set_file_name("duzenleme-taslagi.csv")
                            .add_filter("CSV", &["csv"])
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::Saved(p)),
                );
            }
            Event::Saved(Some(path)) => {
                if let Some(Ok(r)) = &self.readjustment.answer {
                    match std::fs::write(&path, csv(r)) {
                        Ok(()) => self.say(Level::Success, format!("{TITLE}: sonuç {} dosyasına yazıldı.", path.display())),
                        Err(e) => self.warn(format!("{TITLE}: {} yazılamadı: {e}.", path.display())),
                    }
                }
                return Task::none();
            }
            Event::Close => {
                self.readjustment = State::default();
                self.dialog = None;
                return Task::none();
            }
        }
        self.readjustment_run();
        Task::none()
    }

    pub(crate) fn readjustment_view(&self) -> Element<'_, Message> {
        let s = &self.readjustment;
        let r = &s.rules;
        let text = |placeholder: &str, value: &str, on: fn(String) -> Event| -> text_input::TextInput<'_, Message> {
            fields::input(None, placeholder, value, false, false).on_input(move |v| msg(on(v)))
        };
        let modes: Vec<(String, String)> = MODES.iter().map(|(a, b)| ((*a).into(), (*b).into())).collect();
        let rules: Vec<(String, String)> = RULES.iter().map(|(a, b)| ((*a).into(), (*b).into())).collect();
        // Short fields that belong together share a line; every line starts on the box column.
        let form = Form::new()
            .label_width(120.0)
            .field(
                "Kural paketi",
                row![
                    container(text("Kimlik", &r.id, Event::Id)).width(220),
                    label::caption("Sürüm"),
                    NumberInput::new(r.release as f64, |v| msg(Event::Release(v))).step(1.0).decimals(0).width(80)
                ]
                .spacing(10)
                .align_y(Center),
            )
            .field("Kurum", container(text("Kuralı veren kurum", &r.jurisdiction, Event::Jurisdiction)).width(400))
            .field("Kaynak belge", container(text("Kuralın dayandığı belge", &r.source, Event::Source)).width(400))
            .field(
                "Yürürlük",
                row![
                    container(text("YYYY-AA-GG", &r.effective, Event::Effective)).width(140),
                    label::caption("Kesinti oranı"),
                    container(text("0.40", &r.deduction, Event::Deduction)).width(100)
                ]
                .spacing(10)
                .align_y(Center),
            )
            .field(
                "Sayı politikası",
                row![
                    NumberInput::new(r.scale as f64, |v| msg(Event::Scale(v))).step(1.0).decimals(0).width(60),
                    label::caption("basamak"),
                    container(fields::select_owned(modes, &r.rounding, |v| msg(Event::Rounding(v)))).width(190),
                    container(fields::select_owned(rules, &r.remainder, |v| msg(Event::Remainder(v)))).width(150)
                ]
                .spacing(10)
                .align_y(Center),
            )
            .field(
                "Kayıtlar",
                row![
                    words::secondary(OPEN, Some(msg(Event::Pick))),
                    label::caption(match &s.file {
                        Some(f) => format!("{f}: {} kayıt (parsel;hak sahibi;tapu alanı;pay)", s.entries.len()),
                        None => "CSV: parsel;hak sahibi;tapu alanı;pay (hak sahibi kimlikle, adla değil)".to_owned(),
                    })
                ]
                .spacing(10)
                .align_y(Center),
            )
            .field(
                "Kapsam",
                row![
                    words::secondary(APPLY, (matches!(s.scope, Some(Ok(_))) && !s.entries.is_empty()).then(|| msg(Event::Apply))),
                    label::caption("Şerit toleransı"),
                    NumberInput::new(s.tolerance, |v| msg(Event::Tolerance(v))).step(0.001).decimals(3).width(90),
                    label::caption("m")
                ]
                .spacing(10)
                .align_y(Center),
            );
        // The parts outside's parcel numbers, in the separation sheet's view (docs/adr/3080).
        let form = if s.view == View::Sheet {
            form.field(
                "Dış parça no",
                crate::style::fields::input(None, "101/10: 101/24, 101/25; …", &s.numbers, false, false)
                    .on_input(|t| msg(Event::Numbers(t))),
            )
            // docs/adr/4280: the numbers written as texts fitted inside their parts at this height on paper.
            .field(
                "Numara yazısı (mm)",
                row![
                    crate::style::fields::input(None, "boş: noktanın etiketi", &s.number_height, false, false)
                        .on_input(|t| msg(Event::NumberHeight(t)))
                        .width(160),
                    label::caption("parçaya sığana dek onda bir adımlarla küçülür; yatay ya da en uzun kenarı boyunca")
                ]
                .spacing(10)
                .align_y(Center),
            )
        } else {
            form
        };
        let mut lines = vec![words::text_line(
            Kind::Info,
            "Taslak: sonuç tekrarlanabilir bir öneridir; incelenip yetkili onayı olmadan kaydın yerine geçmez, resmî uygunluk kararı verilmez.",
        )];
        let f = self.format();
        match &s.scope {
            Some(Ok(k)) => lines.push(words::text_line(
                if k.review > 0 { Kind::Warn } else { Kind::Ok },
                format!(
                    "Düzenleme sınırı {} m²: {} parsel ({} tamamı, {} kısmen giriyor); {} parselin esas alanı var, toplamı {} m²; {} parsel incelenmeli; parsellerin kaplamadığı alan {} m².",
                    f.area_bare(k.boundary_area),
                    k.parcels.len(),
                    k.whole,
                    k.partial,
                    k.ready,
                    k.basis,
                    k.review,
                    f.area_bare(k.uncovered)
                ),
            )),
            Some(Err(why)) => lines.push(words::text_line(Kind::Error, format!("Kapsam: {why}"))),
            None => lines.push(words::text_line(
                Kind::Info,
                "Düzenleme sınırı seçili değil: çizimde sınırı (kapalı bir alan) seçip pencereyi açın; kapsam ve ayırma çapının sayıları onunla hesaplanır.",
            )),
        }
        for (look, line) in &s.applied {
            lines.push(words::text_line(if *look { Kind::Warn } else { Kind::Ok }, line.clone()));
        }
        let mut table: Element<'_, Message> = words::empty("Kayıt yok: kayıtları seçin.");
        match &s.answer {
            Some(Ok(a)) => {
                lines.push(words::text_line(
                    if a.conserved { Kind::Ok } else { Kind::Error },
                    format!(
                        "Tapu alanı {} m² = kesinti {} m² + net {} m²; hak sahiplerinin toplamı {} m²: {}.",
                        a.registered,
                        a.deduction,
                        a.net,
                        a.held,
                        if a.conserved { "toplam kesin korunuyor" } else { "toplam korunmuyor" }
                    ),
                ));
                let to = |v: &[String]| if v.is_empty() { String::new() } else { format!(" → {}", v.join(", ")) };
                lines.push(words::text_line(
                    Kind::Info,
                    format!(
                        "Kesintinin kesin değeri {} m², yuvarlanmış {} m²; parsellere dağıtımın artığı {}{}.",
                        a.deduction_exact,
                        a.deduction,
                        a.deduction_remainder,
                        to(&a.deduction_remainder_to)
                    ),
                ));
                table = match s.view {
                    // Kapsam's table is the scope's, set below.
                    View::Scope | View::Sheet => words::empty("Düzenleme sınırı seçili değil."),
                    View::Holders => sheet(
                        vec![col("Hak sahibi", 160.0), num("Toplam hak (m²)", 160.0)],
                        a.holders.iter().map(|h| vec![h.holder.clone(), h.value.clone()]).collect(),
                        "Hak sahibi yok.",
                        240.0,
                    ),
                    View::Parcels => sheet(
                        vec![
                            col("Parsel", 80.0),
                            num("Tapu alanı", 95.0),
                            num("Kesinti", 85.0),
                            num("Net", 85.0),
                            col("Hak sahipleri (pay: değer)", 260.0),
                            col("Artık", 110.0),
                        ],
                        a.parcels
                            .iter()
                            .map(|p| {
                                let parts: Vec<String> = p.parts.iter().map(|x| format!("{} ({}): {}", x.holder, x.share, x.value)).collect();
                                vec![
                                    p.parcel.clone(),
                                    p.area.clone(),
                                    p.deduction.clone(),
                                    p.net.clone(),
                                    parts.join("; "),
                                    if p.remainder == "0" { String::new() } else { format!("{}{}", p.remainder, to(&p.remainder_to)) },
                                ]
                            })
                            .collect(),
                        "Parsel yok.",
                        240.0,
                    ),
                };
            }
            Some(Err(why)) => lines.push(words::text_line(Kind::Error, why.clone())),
            None => {}
        }
        if s.view == View::Sheet {
            // Article 13(3): each part outside is registered with a parcel number; the user writes the numbers given.
            lines.push(words::text_line(
                Kind::Info,
                "Dış parçaların parsel numaraları (Yönetmelik m. 13/3): düzenleme dışında kalan her parça parsel numarasıyla tescil edilir. Numaraları verildiği gibi Dış parça no kutusuna “101/10: 101/24, 101/25; …” biçiminde yazın; Çapın paftaları Kısımlar tablosunda gösterir. KentOS numara vermez.",
            ));
            table = match &s.scope {
                Some(Ok(k)) => sheet(
                    vec![col("Parsel", 70.0), col("Kısım", 150.0), num("Nokta", 55.0), num("Y", 130.0), num("X", 130.0), num("Alan (m²)", 100.0)],
                    crate::readjustment_sheet::rows(k, &f),
                    "Kısmen giren parsel yok: ayırma çapı gerekmez.",
                    240.0,
                ),
                Some(Err(_)) => words::empty("Kapsam hesaplanamadı."),
                None => words::empty("Düzenleme sınırı seçili değil."),
            };
        }
        if s.view == View::Scope {
            table = match &s.scope {
                Some(Ok(k)) => sheet(
                    vec![
                        col("Parsel", 55.0),
                        col("Giriş", 60.0),
                        num("Tescilli", 80.0),
                        num("Hesaplanan", 75.0),
                        num("Fark / sınır", 105.0),
                        num("Giren", 70.0),
                        num("Girmeyen", 70.0),
                        num("Esas alan", 70.0),
                        col("Durum", 270.0),
                    ],
                    k.parcels
                        .iter()
                        .map(|p| {
                            let kind = match p.kind.as_str() {
                                "whole" => "Tamamı",
                                "partial" => "Kısmen",
                                _ => "Dışarıda",
                            };
                            let diff = match p.difference {
                                Some(d) => format!("{}{} / ±{}", if d > 0.0 { "+" } else { "" }, f.area_bare(d), f.area_bare(p.limit)),
                                None => format!("— / ±{}", f.area_bare(p.limit)),
                            };
                            vec![
                                p.parcel.clone(),
                                kind.to_owned(),
                                p.recorded_text.clone().unwrap_or_else(|| "—".into()),
                                f.area_bare(p.calculated),
                                diff,
                                f.area_bare(p.inside),
                                f.area_bare(p.outside),
                                p.basis.clone().unwrap_or_else(|| "—".into()),
                                status_word(p),
                            ]
                        })
                        .collect(),
                    "Sınırın içinde parsel yok.",
                    240.0,
                ),
                Some(Err(_)) => words::empty("Kapsam hesaplanamadı."),
                None => words::empty("Düzenleme sınırı seçili değil."),
            };
        }
        // The body scrolls and the table takes what is left, so the footer stays on a short window.
        let body = kentos_ui::widget::grow::Grows::new()
            .spacing(12.0)
            .width(iced::Fill)
            .push(form)
            .push(words::summary(lines))
            .push(Segmented::new([View::Holders, View::Parcels, View::Scope, View::Sheet], s.view, |v| msg(Event::View(v))))
            .push_fit(table, kentos_ui::theme::typography::scaled(200.0));
        let ready = matches!(s.answer, Some(Ok(_)));
        let parts = matches!(&s.scope, Some(Ok(k)) if crate::readjustment_sheet::has_parts(k));
        let frame = Frame::new(TITLE).scroll(body).aside(words::secondary(SAVE, ready.then(|| msg(Event::Save))));
        // The separation sheet's view: its parts written on the drawing, or a sheet for each parcel (docs/adr/2680).
        let frame = if s.view == View::Sheet {
            frame
                .aside(words::secondary(crate::readjustment_sheet::WRITE, parts.then(|| msg(Event::WriteSheet))))
                .aside(words::secondary(crate::readjustment_sheet::MAKE, parts.then(|| msg(Event::MakeSheets))))
        } else {
            frame
        };
        frame
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(1000.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn readjustment_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(OPEN) => Some(msg(Event::Pick)),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(APPLY) => Some(msg(Event::Apply)),
            Control::Press(crate::readjustment_sheet::WRITE) => Some(msg(Event::WriteSheet)),
            Control::Press(crate::readjustment_sheet::MAKE) => Some(msg(Event::MakeSheets)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
