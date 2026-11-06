//! Düzenleme dağıtım taslağı (docs/adr/1460; `readjustment.distribute`): a land readjustment's first numbers by
//! `cad.readjustment.distribute`, read only and always a draft. The rule package is typed here (who issues it, the
//! document, when it holds, its release, the deduction ratio, the numeric policy); the records come from a CSV
//! (`parsel;hak sahibi;tapu alanı;pay`, holders by id, not by name). The answer shows the holders' totals or the
//! parcels (deduction, net, the remainder and where it went); Sonucu kaydet writes it as CSV. Nothing is written to
//! the drawing; no official decision is made.

use std::path::PathBuf;

use iced::widget::{Column, container, row, text_input};
use iced::{Center, Element, Task};
use kentos_commands::{CommandResult, ReadjustmentDistribute, ReadjustmentDistributed, ReadjustmentEntry, ReadjustmentRules};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, readjustment};
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
}

impl std::fmt::Display for View {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            View::Holders => "Hak sahipleri",
            View::Parcels => "Parseller",
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

impl App {
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
        self.readjustment = State::default();
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
            Event::Scale(v) => r.scale = v.max(0.0) as u32,
            Event::Rounding(v) => r.rounding = v,
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
            );
        let mut lines = vec![words::text_line(
            Kind::Info,
            "Taslak: sonuç tekrarlanabilir bir öneridir; incelenip yetkili onayı olmadan kaydın yerine geçmez, resmî uygunluk kararı verilmez.",
        )];
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
        let body = Column::new()
            .spacing(12)
            .push(form)
            .push(words::summary(lines))
            .push(Segmented::new([View::Holders, View::Parcels], s.view, |v| msg(Event::View(v))))
            .push(table);
        let ready = matches!(s.answer, Some(Ok(_)));
        Frame::new(TITLE)
            .push(body)
            .aside(words::secondary(SAVE, ready.then(|| msg(Event::Save))))
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(860.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn readjustment_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(OPEN) => Some(msg(Event::Pick)),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
