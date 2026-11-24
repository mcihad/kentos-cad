//! Parsel alan çizelgesi (docs/adr/9306; `map.parcelReport`): the parcels' corners with their point numbers and Y, X, the
//! edges' lengths and the area computed from the coordinates, the record's area apart, by `cad.parcels.report`, which
//! writes nothing. The scope is the selection's areas, else every area on the parcel layer. The choices (how the
//! corners are named, a new number's prefix and first number, the edges) read the report again as they change.
//! Tabloyu yerleştir closes the window and hangs the table from the cursor; a click writes it by `cad.parcels.table`
//! (kentos_interaction::parcel_table_place). CSV kaydet writes the table's words.

use crate::map;
use crate::map::MapState;
use std::path::PathBuf;

use iced::{Element, Task};
use kentos_commands::{
    CommandResult, CornerNames, Entity, ParcelsReport, ParcelsReported, ParcelsTable,
};
use kentos_interaction::Level;
use kentos_native_application::parcels_report::report as parcels_report;
use kentos_ui::widget::grow::Grows;
use kentos_ui::widget::switch::Switch;
use kentos_ui::widget::{Dialog as Frame, Form, NumberInput, Segmented};

use crate::app::{App, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, num, sheet};
use crate::traces::Control;

pub const TITLE: &str = "Parsel alan çizelgesi";
const CLOSE: &str = "Kapat";
const SAVE: &str = "CSV kaydet…";
pub const PLACE: &str = "Tabloyu yerleştir";
const NAMES: &str = "Köşe adları";
const PREFIX: &str = "Numara öneki";
const START: &str = "İlk numara";
const LENGTHS: &str = "Kenar uzunlukları";

/// How the corners are named, in the window's words.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Names {
    #[default]
    Points,
    Numbers,
}

impl std::fmt::Display for Names {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Names::Points => "Noktalardan",
            Names::Numbers => "Hepsine yeni numara",
        })
    }
}

impl Names {
    fn corner_names(self) -> CornerNames {
        match self {
            Names::Points => CornerNames::Points,
            Names::Numbers => CornerNames::Numbers,
        }
    }
}

#[derive(Clone, Debug)]
pub struct State {
    pub uids: Option<Vec<String>>,
    pub names: Names,
    pub prefix: String,
    pub start: u32,
    pub lengths: bool,
    pub answer: Option<Result<ParcelsReported, String>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            uids: None,
            names: Names::Points,
            prefix: String::new(),
            start: 1,
            lengths: true,
            answer: None,
        }
    }
}

impl State {
    /// The report the window asks for.
    pub fn input(&self) -> ParcelsReport {
        ParcelsReport {
            uids: self.uids.clone(),
            names: Some(self.names.corner_names()),
            prefix: (!self.prefix.is_empty()).then(|| self.prefix.clone()),
            start: Some(self.start),
            lengths: Some(self.lengths),
            field: None,
        }
    }

    /// The table the placement writes, at the origin until the click.
    pub fn table(&self) -> ParcelsTable {
        let r = self.input();
        ParcelsTable {
            uids: r.uids,
            names: r.names,
            prefix: r.prefix,
            start: r.start,
            lengths: r.lengths,
            field: r.field,
            at: kentos_commands::Vec2 { x: 0.0, y: 0.0 },
            layer_id: None,
            height: None,
            expected_revision: None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Names(Names),
    Prefix(String),
    Start(f64),
    Lengths(bool),
    Save,
    Saved(Option<PathBuf>),
    Place,
    Close,
}

fn msg(e: Event) -> Message {
    Message::from(map::Event::ParcelReport(e))
}

/// The table's words as CSV (`;` between cells; a cell holding `;` or `"` quoted).
pub fn csv(r: &ParcelsReported) -> String {
    let cell = |w: &str| {
        if w.contains([';', '"']) {
            format!("\"{}\"", w.replace('"', "\"\""))
        } else {
            w.to_owned()
        }
    };
    r.cells
        .iter()
        .map(|row| row.iter().map(|w| cell(w)).collect::<Vec<_>>().join(";") + "\n")
        .collect()
}

/// The module's `App` methods of this file, as an extension trait (docs/adr/9199 §3.5).
pub(crate) trait ParcelReportWindowApp {
    fn parcel_report_run(&mut self);
    fn open_parcel_report(&mut self) -> Task<Message>;
    fn parcel_report_save(&mut self, path: PathBuf);
    fn parcel_report_place(&mut self);
    fn parcel_report_event(&mut self, event: Event) -> Task<Message>;
    fn parcel_report_view(&self) -> Element<'_, Message>;
    fn parcel_report_control(&self, control: Control<'_>) -> Result<Option<Message>, String>;
}

impl ParcelReportWindowApp for App {
    fn parcel_report_run(&mut self) {
        let input = self.modules.map().parcel_report.input();
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        self.modules.map_mut().parcel_report.answer =
            Some(match parcels_report::plan_in(&doc.model, &input) {
                CommandResult::Completed { output, .. } => Ok(output),
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => Err(error.message),
                _ => Err("Çizelge hesaplanamadı.".to_owned()),
            });
    }

    /// Parsel alan çizelgesi (`map.parcelReport`): the selected areas, else the parcel layer's.
    fn open_parcel_report(&mut self) -> Task<Message> {
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
        let kept = std::mem::take(&mut self.modules.map_mut().parcel_report);
        // The choices stay while the program runs; the scope is read again.
        self.modules.map_mut().parcel_report = State {
            uids: (!areas.is_empty()).then_some(areas),
            answer: None,
            ..kept
        };
        self.parcel_report_run();
        self.dialog = Some(crate::map::dialog(map::MapDialogKind::ParcelReport));
        Task::none()
    }

    /// The table's words written to `path` as CSV (a test or a trace gives it here, the picker in the app).
    fn parcel_report_save(&mut self, path: PathBuf) {
        let Some(Ok(r)) = &self.modules.map().parcel_report.answer else {
            return;
        };
        match std::fs::write(&path, csv(r)) {
            Ok(()) => self.say(
                Level::Success,
                format!(
                    "{TITLE}: {} parsel {} dosyasına yazıldı.",
                    r.parcels.len(),
                    path.display()
                ),
            ),
            Err(e) => self.warn(format!(
                "{TITLE}: {} yazılamadı: {e}. Başka bir klasör seçin.",
                path.display()
            )),
        }
    }

    /// Tabloyu yerleştir: the window closes and the table hangs from the cursor.
    fn parcel_report_place(&mut self) {
        let input = self.modules.map().parcel_report.table();
        self.dialog = None;
        self.session.run(Box::new(
            kentos_interaction::parcel_table_place::ParcelTablePlace::new(input),
        ));
        self.say(
            Level::Command,
            kentos_interaction::parcel_table_place::LABEL,
        );
        self.with_tool(|s, cx| s.activate(cx));
    }

    fn parcel_report_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Names(names) => {
                self.modules.map_mut().parcel_report.names = names;
                self.parcel_report_run();
            }
            Event::Prefix(prefix) => {
                self.modules.map_mut().parcel_report.prefix = prefix;
                self.parcel_report_run();
            }
            Event::Start(start) => {
                self.modules.map_mut().parcel_report.start = if start.is_finite() && start >= 0.0 {
                    start.min(f64::from(u32::MAX)) as u32
                } else {
                    1
                };
                self.parcel_report_run();
            }
            Event::Lengths(on) => {
                self.modules.map_mut().parcel_report.lengths = on;
                self.parcel_report_run();
            }
            Event::Save => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title(TITLE)
                            .set_file_name("parsel-alan-cizelgesi.csv")
                            .add_filter("CSV", &["csv"])
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::Saved(p)),
                );
            }
            Event::Saved(Some(path)) => self.parcel_report_save(path),
            Event::Saved(None) => {}
            Event::Place => self.parcel_report_place(),
            Event::Close => {
                self.modules.map_mut().parcel_report.answer = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    fn parcel_report_view(&self) -> Element<'_, Message> {
        let s = &self.modules.map().parcel_report;
        let mut lines = vec![words::text_line(
            Kind::Info,
            "Köşeler saat yönünde, en kuzeydekinden; alan köşe koordinatlarından, projeksiyon düzleminde. Tapu alanı ayrıca yazılır, hesap alanı onun üstüne yazılmaz.",
        )];
        // The column heads are the report's own words (its second row), borrowed for the table's head.
        let heads: &[String] = match &s.answer {
            Some(Ok(r)) => r.cells.get(1).map_or(&[], Vec::as_slice),
            _ => &[],
        };
        let mut rows: Vec<Vec<String>> = Vec::new();
        let ready = match &s.answer {
            Some(Ok(r)) if r.parcels.is_empty() => {
                lines.push(words::text_line(
                    Kind::Warn,
                    "Çizelgeye yazılacak parsel yok: parsel katmanında alan yok ya da seçimde alan yok.",
                ));
                false
            }
            Some(Ok(r)) => {
                let scope = if s.uids.is_some() {
                    "Seçili"
                } else {
                    "Parsel katmanındaki"
                };
                lines.push(words::text_line(
                    Kind::Ok,
                    format!(
                        "{scope} {} parsel, {} köşe: {} köşenin adı noktadan, {} köşeye yeni numara. Toplam hesap alanı {} m².",
                        r.parcels.len(),
                        r.places,
                        r.named,
                        r.numbered,
                        kentos_geometry_core::display::fixed(r.area, self.format().area_decimals)
                    ),
                ));
                let outside = r.parcels.iter().filter(|p| p.status == "outside").count();
                if outside > 0 {
                    lines.push(words::text_line(
                        Kind::Warn,
                        format!(
                            "{outside} parselin hesap alanı tapu alanından yanılma sınırı (2022 Yönetmeliği Madde 8(2)) kadar uzak; sonuç resmî uygunluk iddiası değildir."
                        ),
                    ));
                }
                rows = r.cells.iter().skip(2).cloned().collect();
                true
            }
            Some(Err(why)) => {
                lines.push(words::text_line(Kind::Error, why.clone()));
                false
            }
            None => false,
        };
        let names = Segmented::new([Names::Points, Names::Numbers], s.names, |v| {
            msg(Event::Names(v))
        });
        let form = Form::new()
            .label_width(150.0)
            .field(NAMES, names)
            .help("Noktalardan: köşede duran noktanın adı, nokta olmayan köşeye yeni numara.")
            .field(
                PREFIX,
                kentos_ui::widget::text_field("Önek yok", &s.prefix)
                    .on_input(|v| msg(Event::Prefix(v)))
                    .width(140.0),
            )
            .field(
                START,
                NumberInput::new(f64::from(s.start), |v| msg(Event::Start(v)))
                    .step(1.0)
                    .decimals(0)
                    .width(140.0),
            )
            .field(
                LENGTHS,
                Switch::new(s.lengths, |on| msg(Event::Lengths(on)))
                    .label("Kenarı ve yayın yarıçapını yaz"),
            );
        let cols = heads
            .iter()
            .enumerate()
            .map(|(i, h)| if i == 0 { col(h, 220.0) } else { num(h, 120.0) })
            .collect::<Vec<_>>();
        let table = sheet(cols, rows, "Çizelgede satır yok.", 320.0);
        let body = Grows::new()
            .spacing(12.0)
            .width(iced::Fill)
            .push(words::summary(lines))
            .push(form)
            .push_fit(table, kentos_ui::theme::typography::scaled(120.0));
        Frame::new(TITLE)
            .scroll(body)
            .aside(words::secondary(SAVE, ready.then(|| msg(Event::Save))))
            .action(words::secondary(CLOSE, Some(msg(Event::Close))))
            .action(words::primary(PLACE, ready.then(|| msg(Event::Place))))
            .width(880.0)
            .modal(msg(Event::Close))
    }

    fn parcel_report_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(PLACE) => Some(msg(Event::Place)),
            Control::Pick(NAMES, "Noktalardan") => Some(msg(Event::Names(Names::Points))),
            Control::Pick(NAMES, "Hepsine yeni numara") => Some(msg(Event::Names(Names::Numbers))),
            Control::Fill(PREFIX, text) => Some(msg(Event::Prefix(text.to_owned()))),
            Control::Fill(START, text) => {
                Some(msg(Event::Start(text.trim().parse().unwrap_or(1.0))))
            }
            Control::Check(LENGTHS, on) => Some(msg(Event::Lengths(on))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
