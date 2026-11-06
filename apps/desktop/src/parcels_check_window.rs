//! Parsel alan denetimi (docs/adr/1400; `parcels.check`): every parcel's computed area beside its Tapu alanı, the
//! difference and the tolerance of the 2022 regulation's Article 8(2), by `cad.parcels.check`, read only. The scope
//! is the selection's areas, else every area on the parcel layer. A choice above the table shows all, the ones
//! outside, the ones without a readable record, or the ones whose identity (İl/İlçe/Mahalle/Ada/Parsel) is held
//! twice; Dışındakileri seç selects the parcels outside the tolerance; CSV kaydet writes the table.

use std::path::PathBuf;

use iced::widget::Column;
use iced::{Element, Task};
use kentos_commands::{CommandResult, Entity, ParcelAreaCheck, ParcelsCheck, ParcelsChecked};
use kentos_domain::{Slot, Uuid};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, parcels_check};
use kentos_ui::widget::{Dialog as Frame, Segmented};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, num, sheet};
use crate::traces::Control;

pub const TITLE: &str = "Parsel alan denetimi";
const CLOSE: &str = "Kapat";
const SAVE: &str = "CSV kaydet…";
const SELECT: &str = "Dışındakileri seç";

/// Which parcels the table shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Show {
    #[default]
    All,
    Outside,
    Unrecorded,
    Duplicate,
}

impl std::fmt::Display for Show {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Show::All => "Tümü",
            Show::Outside => "Sınır dışında",
            Show::Unrecorded => "Kaydı yok / okunamadı",
            Show::Duplicate => "Yinelenen kimlik",
        })
    }
}

impl Show {
    fn shows(self, p: &ParcelAreaCheck) -> bool {
        match self {
            Show::All => true,
            Show::Outside => p.status == "outside",
            Show::Unrecorded => p.status == "noRecord" || p.status == "invalidRecord",
            Show::Duplicate => p.duplicate,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct State {
    pub uids: Option<Vec<String>>,
    pub show: Show,
    pub answer: Option<Result<ParcelsChecked, String>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Show(Show),
    Select,
    Save,
    Saved(Option<PathBuf>),
    Close,
}

fn msg(e: Event) -> Message {
    Message::ParcelsCheck(e)
}

/// A status in words.
pub fn status_word(status: &str) -> &'static str {
    match status {
        "within" => "İçinde",
        "outside" => "Dışında",
        "noRecord" => "Kaydı yok",
        "invalidRecord" => "Okunamadı",
        _ => "—",
    }
}

/// The table as CSV (`;`; areas in square metres as computed, the record as written).
pub fn csv(r: &ParcelsChecked) -> String {
    let mut out = String::from(
        "Nesne;İl;İlçe;Mahalle;Ada;Parsel;Hesap alanı (m²);Tapu alanı (yazıldığı gibi);Fark (m²);Yanılma sınırı (m²);Durum;Yinelenen\n",
    );
    for p in &r.parcels {
        out.push_str(&format!(
            "{};{};{};{};{};{};{};{};{};{};{};{}\n",
            p.uid,
            p.il,
            p.ilce,
            p.mahalle,
            p.ada,
            p.parsel,
            p.area,
            p.recorded_text.as_deref().unwrap_or(""),
            p.difference.map(|d| d.to_string()).unwrap_or_default(),
            p.tolerance,
            status_word(&p.status),
            if p.duplicate { "evet" } else { "" }
        ));
    }
    out
}

impl App {
    fn parcels_check_run(&mut self) {
        let input = ParcelsCheck { uids: self.parcels_check.uids.clone(), field: None };
        let Some(doc) = self.document.as_mut() else { return };
        self.parcels_check.answer = Some(match parcels_check::plan(&ExecutionContext::new(&mut doc.model), &input) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err(error.message)
            }
            _ => Err("Denetim yapılamadı.".to_owned()),
        });
    }

    /// Parsel alan denetimi (`parcels.check`): the selected areas, else the parcel layer's.
    pub(crate) fn open_parcels_check(&mut self) -> Task<Message> {
        let Some(doc) = self.document.as_ref() else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let areas: Vec<String> = self
            .selection
            .ids()
            .iter()
            .filter(|s| matches!(doc.model.get(**s), Some(Entity::Polygon(_))))
            .filter_map(|s| doc.model.uid(*s))
            .map(|u| u.to_string())
            .collect();
        self.parcels_check = State { uids: (!areas.is_empty()).then_some(areas), ..State::default() };
        self.parcels_check_run();
        self.dialog = Some(Dialog::ParcelsCheck);
        Task::none()
    }

    /// The table written to `path` as CSV (a test or a trace gives it here, the picker in the app).
    pub(crate) fn parcels_check_save(&mut self, path: PathBuf) {
        let Some(Ok(r)) = &self.parcels_check.answer else { return };
        match std::fs::write(&path, csv(r)) {
            Ok(()) => self.say(Level::Success, format!("{TITLE}: {} parsel {} dosyasına yazıldı.", r.parcels.len(), path.display())),
            Err(e) => self.warn(format!("{TITLE}: {} yazılamadı: {e}. Başka bir klasör seçin.", path.display())),
        }
    }

    pub(crate) fn parcels_check_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Show(show) => self.parcels_check.show = show,
            Event::Select => {
                let Some(Ok(r)) = &self.parcels_check.answer else { return Task::none() };
                let Some(doc) = self.document.as_ref() else { return Task::none() };
                let slots: Vec<Slot> = r
                    .parcels
                    .iter()
                    .filter(|p| p.status == "outside")
                    .filter_map(|p| Uuid::parse_str(&p.uid).ok().and_then(|u| doc.model.slot_of(u)))
                    .collect();
                let n = slots.len();
                self.selection.set(slots);
                self.parcels_check = State::default();
                self.dialog = None;
                self.say(Level::Info, format!("{TITLE}: yanılma sınırı dışındaki {n} parsel seçildi."));
            }
            Event::Save => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title(TITLE)
                            .set_file_name("parsel-alan-denetimi.csv")
                            .add_filter("CSV", &["csv"])
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::Saved(p)),
                );
            }
            Event::Saved(Some(path)) => self.parcels_check_save(path),
            Event::Saved(None) => {}
            Event::Close => {
                self.parcels_check = State::default();
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn parcels_check_view(&self) -> Element<'_, Message> {
        let s = &self.parcels_check;
        let f = self.format();
        let mut lines = Vec::new();
        let mut rows = Vec::new();
        let mut outside = 0;
        match &s.answer {
            Some(Ok(r)) if r.parcels.is_empty() => lines.push(words::text_line(
                Kind::Warn,
                "Denetlenecek parsel yok: parsel katmanında alan yok ya da seçimde alan yok.",
            )),
            Some(Ok(r)) => {
                outside = r.outside;
                let scope = if s.uids.is_some() { "Seçili" } else { "Parsel katmanındaki" };
                lines.push(words::text_line(
                    if r.outside > 0 { Kind::Warn } else { Kind::Ok },
                    format!(
                        "{scope} {} parsel: {} sınır içinde, {} sınır dışında, {} kaydı yok, {} kaydı okunamadı.",
                        r.parcels.len(),
                        r.within,
                        r.outside,
                        r.no_record,
                        r.invalid_record
                    ),
                ));
                if !r.duplicates.is_empty() {
                    lines.push(words::text_line(
                        Kind::Error,
                        format!("Aynı kimlikte birden çok parsel: {}.", r.duplicates.join(", ")),
                    ));
                }
                if r.no_identity > 0 {
                    lines.push(words::text_line(Kind::Warn, format!("{} parselin Ada ya da Parsel numarası yok.", r.no_identity)));
                }
                lines.push(words::text_line(
                    Kind::Info,
                    "Yanılma sınırı: 2022 Yönetmeliği Madde 8(2), arşiv onaylı geometri tablosu. Hesap alanı kaydın üstüne yazılmaz; sonuç resmî uygunluk iddiası değildir.",
                ));
                for p in r.parcels.iter().filter(|p| s.show.shows(p)) {
                    let id = match (p.ada.is_empty(), p.parsel.is_empty()) {
                        (false, false) => format!("{}/{}", p.ada, p.parsel),
                        (true, false) => format!("—/{}", p.parsel),
                        _ => "—".to_owned(),
                    };
                    rows.push(vec![
                        if p.duplicate { format!("{id} ⚠") } else { id },
                        p.mahalle.clone(),
                        f.area(p.area),
                        p.recorded_text.clone().unwrap_or_default(),
                        p.difference.map(|d| if d > 0.0 { format!("+{}", f.area(d)) } else { f.area(d) }).unwrap_or_default(),
                        format!("±{}", f.area(p.tolerance)),
                        status_word(&p.status).to_owned(),
                    ]);
                }
            }
            Some(Err(why)) => lines.push(words::text_line(Kind::Error, why.clone())),
            None => {}
        }
        let shows = [Show::All, Show::Outside, Show::Unrecorded, Show::Duplicate];
        let table = sheet(
            vec![
                col("Ada/Parsel", 80.0),
                col("Mahalle", 90.0),
                num("Hesap alanı", 110.0),
                num("Tapu alanı", 90.0),
                num("Fark", 90.0),
                num("Yanılma sınırı", 100.0),
                col("Durum", 80.0),
            ],
            rows,
            "Bu süzgeçte parsel yok.",
            300.0,
        );
        let body = Column::new()
            .spacing(12)
            .push(words::summary(lines))
            .push(Segmented::new(shows, s.show, |v| msg(Event::Show(v))))
            .push(table);
        let ready = matches!(&s.answer, Some(Ok(r)) if !r.parcels.is_empty());
        Frame::new(TITLE)
            .push(body)
            .aside(words::secondary(SAVE, ready.then(|| msg(Event::Save))))
            .aside(words::secondary(SELECT, (outside > 0).then(|| msg(Event::Select))))
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(780.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn parcels_check_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(SELECT) => Some(msg(Event::Select)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
