//! Adres kaydıyla karşılaştır (`address.compare`, docs/adr/0736): the project's address data compared with an address
//! register (`cad.addresses.compare`), the institution's or the national address database's table of address codes:
//! a table file (CSV or text, XLSX, ODS) or a layer of the drawing. Its columns are chosen from its first row (guessed
//! from their names, as Alanları tanı guesses the doors'); Karşılaştır runs the command. Its lists are rows: a door's
//! row selects the door and brings it into view, a layer's register row its object. Farkları kayıttan yaz writes the
//! register's street, door number, neighbourhood and district to the doors that write them otherwise: one change set
//! (`cad.changeset.apply` of `cad.entities.set` calls, “Adres kaydından yaz”), one undo step, compared again after it.

use std::path::PathBuf;

use iced::futures::SinkExt as _;
use iced::futures::channel::mpsc;
use iced::widget::{Column, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    ADDRESS_COMPARE_MAX_LIMIT, AddressPart, AddressRegister, AddressesCompare, AddressesCompared,
    CallOp, CommandResult,
};
use kentos_formats::table_file::{Limits, read_with};
use kentos_geometry_core::ops::geocode::fields;
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, addresses_compare, registry};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog as Frame, Elided, Segmented, VirtualList};
use kentos_ui::{label, style};

use super::Event as Address;
use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Adres kaydıyla karşılaştır";
const COMPARE: &str = "Karşılaştır";
const WRITE: &str = "Farkları kayıttan yaz";
const CHOOSE: &str = "Dosya seç…";
const CLOSE: &str = "Kapat";
const NONE: &str = "—";
const ROW: f32 = 26.0;
/// The register's columns the window asks for, in its order: code, street, door number, neighbourhood, district, x, y.
const COLUMNS: [&str; 7] = [
    "Adres kodu",
    "Yol",
    "Kapı no",
    "Mahalle",
    "İlçe",
    "X (doğu)",
    "Y (kuzey)",
];

/// Where the register is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    File,
    Layer,
}

impl Source {
    const ALL: [Source; 2] = [Source::File, Source::Layer];
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Source::File => "Tablo dosyası",
            Source::Layer => "Çizimdeki katman",
        })
    }
}

/// A list of the answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum List {
    Unregistered,
    Differences,
    Distances,
    Undrawn,
    Repeats,
}

impl List {
    const ALL: [List; 5] = [
        List::Unregistered,
        List::Differences,
        List::Distances,
        List::Undrawn,
        List::Repeats,
    ];

    fn words(self) -> &'static str {
        match self {
            List::Unregistered => "Kayıtta yok",
            List::Differences => "Farklı yazılmış",
            List::Distances => "Uzak",
            List::Undrawn => "Çizimde yok",
            List::Repeats => "Kayıtta yinelenen",
        }
    }

    fn count(self, a: &AddressesCompared) -> u32 {
        match self {
            List::Unregistered => a.unregistered,
            List::Differences => a.differing,
            List::Distances => a.distant,
            List::Undrawn => a.undrawn,
            List::Repeats => a.repeated,
        }
    }
}

/// A list's choice in the segmented row: its words and its count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Tab(List, u32);

impl std::fmt::Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.0.words(), self.1)
    }
}

#[derive(Debug, Clone)]
pub struct Window {
    /// The address data, by its index among the project's.
    locator: usize,
    source: Source,
    path: Option<PathBuf>,
    /// A workbook's sheets; empty for a text table.
    sheets: Vec<String>,
    sheet: usize,
    /// The register layer's id.
    layer: Option<String>,
    /// The register's columns, from its first row (a layer's fields).
    columns: Vec<String>,
    /// For each of [`COLUMNS`], the column chosen.
    picks: [Option<usize>; 7],
    tolerance: String,
    answer: Option<Result<AddressesCompared, String>>,
    list: List,
    chosen: Option<usize>,
    /// The comparison running on its thread: the ticket its answer comes with.
    asking: Option<u64>,
    tickets: u64,
}

#[derive(Debug, Clone)]
pub enum Event {
    Locator(usize),
    Source(Source),
    Choose,
    Chosen(Option<PathBuf>),
    Sheet(usize),
    Layer(usize),
    /// One of [`COLUMNS`], and its choice (0: none).
    Column(usize, usize),
    Tolerance(String),
    Compare,
    /// The comparison's answer from its thread, with its ticket.
    Answered(u64, Box<CommandResult<AddressesCompared>>),
    List(List),
    Pick(usize),
    Write,
    Close,
}

fn msg(e: Event) -> Message {
    Message::Address(Address::Compare(e))
}

/// The register's first row, and a workbook's sheets: what a file's columns are chosen from.
fn header(
    path: &std::path::Path,
    sheet: Option<&str>,
) -> Result<(Vec<String>, Vec<String>), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{} okunamadı: {e}", path.display()))?;
    let read = read_with(
        &bytes,
        Limits {
            rows: 1,
            columns: Limits::PROCESSING.columns,
        },
    );
    if let Some(problem) = read.problem {
        return Err(problem);
    }
    let sheets: Vec<String> = read.sheets.iter().filter_map(|s| s.name.clone()).collect();
    let first = match sheet {
        Some(name) => read.sheets.iter().find(|s| s.name.as_deref() == Some(name)),
        None => read.sheets.first(),
    };
    let columns = first
        .and_then(|s| s.rows.first())
        .map(|r| r.iter().map(|c| c.trim().to_owned()).collect())
        .unwrap_or_default();
    Ok((columns, sheets))
}

/// The columns guessed from their names: the doors' guess for the address parts (a register's “ADRES_NO” is its
/// address code too), `X`/`Y` (or east and north) for the position.
fn guess(columns: &[String]) -> [Option<usize>; 7] {
    let g = fields::doors(columns);
    let at = |name: &Option<String>| {
        name.as_ref()
            .and_then(|n| columns.iter().position(|c| c == n))
    };
    let folded = |c: &str| {
        kentos_geometry_core::ops::geocode::fold(c)
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .collect::<String>()
    };
    let axis = |names: &[&str]| {
        columns
            .iter()
            .position(|c| names.contains(&folded(c).as_str()))
    };
    let code = at(&g.code).or_else(|| axis(&["adresno", "adresnumarasi", "uavtno", "uavtkodu"]));
    [
        code,
        at(&g.street),
        at(&g.door),
        at(&g.neighbourhood),
        at(&g.district),
        axis(&["x", "saga", "dogu", "easting"]),
        axis(&["y", "yukari", "kuzey", "northing"]),
    ]
}

impl App {
    /// The project's address data whose doors name a code: what can be compared.
    fn compare_locators(&self) -> Vec<(String, String)> {
        self.document.as_ref().map_or_else(Vec::new, |d| {
            d.model
                .settings()
                .address_locators
                .iter()
                .filter(|l| l.doors.as_ref().is_some_and(|d| d.code.is_some()))
                .map(|l| (l.id.clone(), l.name.clone()))
                .collect()
        })
    }

    /// `address.compare`: the window.
    pub(crate) fn open_address_compare(&mut self) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return Task::none();
        }
        self.address.compare = Some(Window {
            locator: 0,
            source: Source::File,
            path: None,
            sheets: Vec::new(),
            sheet: 0,
            layer: None,
            columns: Vec::new(),
            picks: [None; 7],
            tolerance: "10".into(),
            answer: None,
            list: List::Unregistered,
            chosen: None,
            asking: None,
            tickets: 0,
        });
        self.dialog = Some(Dialog::AddressCompare);
        Task::none()
    }

    /// The command's input as the window says it; why not.
    fn compare_input(&self, w: &Window) -> Result<AddressesCompare, String> {
        let locators = self.compare_locators();
        let Some((locator, _)) = locators.get(w.locator) else {
            return Err("Kapılarında adres kodu alanı olan adres verisi yok; Adres verisi penceresinde kapıların Adres kodu alanını seçin.".into());
        };
        let name = |k: usize| w.picks[k].and_then(|i| w.columns.get(i).cloned());
        let Some(code) = name(0) else {
            return Err("Adres kodunun sütununu seçin.".into());
        };
        let tolerance = w
            .tolerance
            .trim()
            .replace(',', ".")
            .parse::<f64>()
            .map_err(|_| "Toleransı metre olarak yazın (10 gibi).".to_owned())?;
        let (path, layer) = match w.source {
            Source::File => match &w.path {
                Some(p) => (Some(p.display().to_string()), None),
                None => return Err("Adres kaydının dosyasını seçin.".into()),
            },
            Source::Layer => match &w.layer {
                Some(l) => (None, Some(l.clone())),
                None => return Err("Adres kaydının katmanını seçin.".into()),
            },
        };
        Ok(AddressesCompare {
            locator: Some(locator.clone()),
            register: AddressRegister {
                path,
                sheet: (w.source == Source::File && !w.sheets.is_empty())
                    .then(|| w.sheets.get(w.sheet).cloned())
                    .flatten(),
                layer,
                code,
                street: name(1),
                door: name(2),
                neighbourhood: name(3),
                district: name(4),
                x: name(5),
                y: name(6),
            },
            tolerance: Some(tolerance),
            limit: Some(ADDRESS_COMPARE_MAX_LIMIT),
            drawing: None,
        })
    }

    /// Karşılaştır: the command run on a copy of the drawing as it is (its objects shared), on a thread of its own: a
    /// register of a quarter of a million rows takes most of a second to read.
    fn run_address_compare(&mut self) -> Task<Message> {
        let Some(w) = self.address.compare.as_ref() else {
            return Task::none();
        };
        let input = match self.compare_input(w) {
            Ok(input) => input,
            Err(why) => {
                if let Some(w) = self.address.compare.as_mut() {
                    w.answer = Some(Err(why));
                }
                return Task::none();
            }
        };
        let Some(model) = self.document.as_ref().map(|d| d.model.clone()) else {
            return Task::none();
        };
        let Some(w) = self.address.compare.as_mut() else {
            return Task::none();
        };
        w.tickets += 1;
        let ticket = w.tickets;
        w.asking = Some(ticket);
        iced_runtime::task::blocking(move |mut out: mpsc::Sender<Message>| {
            let answer = addresses_compare::plan_in(&model, &input);
            drop(model);
            let _ = iced::futures::executor::block_on(
                out.send(msg(Event::Answered(ticket, Box::new(answer)))),
            );
        })
    }

    /// The comparison's answer, when it is the one asked last.
    fn address_compare_answered(&mut self, ticket: u64, result: CommandResult<AddressesCompared>) {
        if self.address.compare.as_ref().and_then(|w| w.asking) != Some(ticket) {
            return;
        }
        let answer = match result {
            CommandResult::Completed { output, warnings } => {
                for w in warnings {
                    self.warn(w.message);
                }
                Ok(output)
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => Err(error.message),
            _ => Err("Karşılaştırılamadı.".into()),
        };
        if let Some(w) = self.address.compare.as_mut() {
            w.asking = None;
            // The first list with findings comes forward.
            if let Ok(a) = &answer {
                w.list = List::ALL
                    .into_iter()
                    .find(|l| l.count(a) > 0)
                    .unwrap_or(List::Unregistered);
            }
            w.answer = Some(answer);
            w.chosen = None;
        }
    }

    pub(crate) fn address_compare_event(&mut self, e: Event) -> Task<Message> {
        let layers = self.address_layers();
        let fields_of = |app: &App, layer: &str| app.address_fields(layer);
        match e {
            Event::Close => {
                self.address.compare = None;
                self.dialog = None;
                return Task::none();
            }
            Event::Choose => {
                return Task::perform(
                    async {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("Adres kaydı")
                            .add_filter(
                                "Tablo (CSV, metin, XLSX, ODS)",
                                &["csv", "txt", "tsv", "xlsx", "ods"],
                            )
                            .pick_file()
                            .await?;
                        Some(file.path().to_path_buf())
                    },
                    |path| msg(Event::Chosen(path)),
                );
            }
            Event::Chosen(None) => return Task::none(),
            Event::Chosen(Some(path)) => {
                let read = header(&path, None);
                let Some(w) = self.address.compare.as_mut() else {
                    return Task::none();
                };
                w.path = Some(path);
                w.sheet = 0;
                w.answer = None;
                match read {
                    Ok((columns, sheets)) => {
                        w.picks = guess(&columns);
                        w.columns = columns;
                        w.sheets = sheets;
                    }
                    Err(why) => {
                        w.columns.clear();
                        w.sheets.clear();
                        w.picks = [None; 7];
                        w.answer = Some(Err(why));
                    }
                }
                return Task::none();
            }
            Event::Sheet(k) => {
                let Some(w) = self.address.compare.as_mut() else {
                    return Task::none();
                };
                w.sheet = k;
                if let (Some(path), Some(name)) = (w.path.clone(), w.sheets.get(k).cloned())
                    && let Ok((columns, _)) = header(&path, Some(&name))
                {
                    w.picks = guess(&columns);
                    w.columns = columns;
                }
                w.answer = None;
                return Task::none();
            }
            Event::Layer(k) => {
                let id = layers.get(k).map(|(id, _)| id.clone());
                let columns = id
                    .as_deref()
                    .map(|l| fields_of(self, l))
                    .unwrap_or_default();
                let Some(w) = self.address.compare.as_mut() else {
                    return Task::none();
                };
                w.layer = id;
                w.picks = guess(&columns);
                w.picks[5] = None;
                w.picks[6] = None;
                w.columns = columns;
                w.answer = None;
                return Task::none();
            }
            Event::Compare => return self.run_address_compare(),
            Event::Answered(ticket, result) => {
                self.address_compare_answered(ticket, *result);
                return Task::none();
            }
            Event::Write => return self.write_address_register(),
            Event::Pick(i) => {
                if let Some(w) = self.address.compare.as_mut() {
                    w.chosen = Some(i);
                }
                self.show_compared(i);
                return Task::none();
            }
            _ => {}
        }
        let Some(w) = self.address.compare.as_mut() else {
            return Task::none();
        };
        match e {
            Event::Locator(k) => {
                w.locator = k;
                w.answer = None;
            }
            Event::Source(s) => {
                w.source = s;
                w.columns.clear();
                w.picks = [None; 7];
                w.answer = None;
            }
            Event::Column(k, choice) => {
                if let Some(p) = w.picks.get_mut(k) {
                    *p = choice.checked_sub(1);
                }
                w.answer = None;
            }
            Event::Tolerance(t) => {
                w.tolerance = t;
            }
            Event::List(l) => {
                w.list = l;
                w.chosen = None;
            }
            _ => {}
        }
        Task::none()
    }

    /// A row of the chosen list: its door, or its register objects, selected and brought into view.
    fn show_compared(&mut self, i: usize) {
        let Some(w) = self.address.compare.as_ref() else {
            return;
        };
        let Some(Ok(a)) = &w.answer else {
            return;
        };
        let uids: Vec<String> = match w.list {
            List::Unregistered => a.unregistered_doors.get(i).map(|d| vec![d.uid.clone()]),
            List::Differences => a.differences.get(i).map(|d| vec![d.uid.clone()]),
            List::Distances => a.distances.get(i).map(|d| vec![d.uid.clone()]),
            List::Undrawn => a.undrawn_addresses.get(i).map(|u| u.uids.clone()),
            List::Repeats => a.register_repeats.get(i).map(|r| r.uids.clone()),
        }
        .unwrap_or_default();
        let slots: Vec<_> = self.document.as_ref().map_or_else(Vec::new, |d| {
            uids.iter()
                .filter_map(|u| kentos_domain::Uuid::parse_str(u).ok())
                .filter_map(|u| d.model.slot_of(u))
                .collect()
        });
        if !slots.is_empty() {
            let ids: Vec<f64> = slots.iter().map(|s| f64::from(s.0)).collect();
            self.selection.set(slots);
            self.zoom_to_objects(&ids);
        }
    }

    /// Farkları kayıttan yaz: each differing door given the register's texts of the parts it writes otherwise, as one
    /// change set; the comparison run again.
    pub(crate) fn write_address_register(&mut self) -> Task<Message> {
        let Some(w) = self.address.compare.as_ref() else {
            return Task::none();
        };
        let Some(Ok(a)) = &w.answer else {
            return Task::none();
        };
        let locators = self.compare_locators();
        let Some(doors) = locators.get(w.locator).and_then(|(id, _)| {
            self.document.as_ref().and_then(|d| {
                d.model
                    .settings()
                    .address_locators
                    .iter()
                    .find(|l| &l.id == id)
                    .and_then(|l| l.doors.clone())
            })
        }) else {
            return Task::none();
        };
        let steps: Vec<serde_json::Value> = a
            .differences
            .iter()
            .filter_map(|d| {
                let mut attrs = serde_json::Map::new();
                for p in &d.parts {
                    let field = match p.part {
                        AddressPart::Street => Some(doors.street.clone()),
                        AddressPart::Door => Some(doors.door.clone()),
                        AddressPart::Neighbourhood => doors.neighbourhood.clone(),
                        AddressPart::District => doors.district.clone(),
                    };
                    if let Some(f) = field {
                        attrs.insert(f, serde_json::Value::String(p.register.clone()));
                    }
                }
                (!attrs.is_empty()).then(|| {
                    serde_json::json!({
                        "command": kentos_commands::CAD_ENTITIES_SET,
                        "input": {"uids": [d.uid], "operation": "attributes", "attrs": attrs},
                    })
                })
            })
            .collect();
        if steps.is_empty() {
            return Task::none();
        }
        if let Some(locked) = self.tree_locked() {
            self.warn(locked);
            return Task::none();
        }
        let count = steps.len();
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let answer = registry::run(
            &mut ExecutionContext::new(&mut doc.model),
            kentos_commands::CAD_CHANGESET_APPLY,
            CallOp::Execute,
            serde_json::json!({"steps": steps, "label": "Adres kaydından yaz"}),
        );
        match answer {
            Ok(v) if v["status"] == "completed" => {
                self.say(
                    Level::Success,
                    format!("Kayıttan yazılan kapı: {count}; Geri al ile dönülür."),
                );
                return self.run_address_compare();
            }
            Ok(v) => {
                let why = v["error"]["message"]
                    .as_str()
                    .unwrap_or("Kayıttan yazılamadı.");
                self.warn(why.to_owned());
            }
            Err(e) => self.warn(format!("Kayıttan yazılamadı: {e:?}")),
        }
        Task::none()
    }

    pub(crate) fn address_compare_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.address.compare else {
            return iced::widget::text("").into();
        };
        let locators = self.compare_locators();
        let locator: Element<'_, Message> = if locators.is_empty() {
            label::muted(
                "Kapılarında adres kodu alanı olan adres verisi yok; Adres verisi penceresinde kapıların Adres kodu alanını seçin.",
            )
            .into()
        } else {
            Select::new(
                locators.iter().map(|(_, n)| Choice::new(n.clone())),
                Some(w.locator.min(locators.len() - 1)),
                |k| msg(Event::Locator(k)),
            )
            .into()
        };
        let source = Segmented::new(Source::ALL, w.source, |s| msg(Event::Source(s)));
        let place: Element<'_, Message> = match w.source {
            Source::File => {
                let name = w
                    .path
                    .as_ref()
                    .map_or_else(|| "Dosya seçilmedi".to_owned(), |p| p.display().to_string());
                let mut line = row![
                    container(
                        Elided::new(name)
                            .size(typography::body())
                            .font(typography::ui())
                            .width(Fill)
                    )
                    .width(Fill),
                    words::secondary(CHOOSE, Some(msg(Event::Choose))),
                ]
                .spacing(8)
                .align_y(Center);
                if w.sheets.len() > 1 {
                    line = line.push(
                        container(Select::new(
                            w.sheets.iter().map(|s| Choice::new(s.clone())),
                            Some(w.sheet),
                            |k| msg(Event::Sheet(k)),
                        ))
                        .width(typography::scaled(160.0)),
                    );
                }
                line.into()
            }
            Source::Layer => {
                let layers = self.address_layers();
                Select::new(
                    layers.iter().map(|(_, p)| Choice::new(p.clone())),
                    w.layer
                        .as_ref()
                        .and_then(|l| layers.iter().position(|(id, _)| id == l)),
                    |k| msg(Event::Layer(k)),
                )
                .placeholder("Katman seçin")
                .into()
            }
        };
        // The columns, two to a row.
        let column_select = |k: usize| -> Element<'_, Message> {
            let choices = std::iter::once(Choice::new(NONE))
                .chain(w.columns.iter().map(|c| Choice::new(c.clone())));
            words::field(
                COLUMNS[k],
                Select::new(choices, Some(w.picks[k].map_or(0, |i| i + 1)), move |c| {
                    msg(Event::Column(k, c))
                }),
                None,
            )
        };
        let mut columns = Column::new().spacing(8);
        for pair in [[0, 1], [2, 3], [4, 5]] {
            columns = columns.push(
                row![
                    container(column_select(pair[0])).width(Fill),
                    container(column_select(pair[1])).width(Fill),
                ]
                .spacing(12),
            );
        }
        columns = columns.push(
            row![
                container(column_select(6)).width(Fill),
                container(words::field(
                    "Tolerans (m)",
                    kentos_ui::widget::text_field("10", &w.tolerance)
                        .on_input(|t| msg(Event::Tolerance(t))),
                    None,
                ))
                .width(Fill),
            ]
            .spacing(12),
        );
        let mut body = Column::new()
            .spacing(12)
            .push(words::field("Adres verisi", locator, None))
            .push(words::field(
                "Adres kaydı",
                Column::new().spacing(6).push(source).push(place),
                None,
            ));
        if !w.columns.is_empty() {
            body = body.push(words::field(
                "Sütunlar",
                columns,
                Some("Mahalle ve ilçe yalnız kapılarda o alan varsa karşılaştırılır; X ve Y projenin koordinatlarındadır, katmanda verilmezse noktanın yeri alınır.".into()),
            ));
        }
        let mut can_write = false;
        if w.asking.is_some() {
            body = body.push(words::summary(vec![words::text_line(
                Kind::Info,
                "Karşılaştırılıyor…",
            )]));
        }
        match &w.answer {
            None => {}
            Some(Err(why)) => {
                body = body.push(words::summary(vec![words::text_line(
                    Kind::Warn,
                    why.clone(),
                )]))
            }
            Some(Ok(a)) => {
                can_write = a.differing > 0;
                body = body.push(words::summary(vec![words::text_line(
                    if List::ALL.iter().all(|l| l.count(a) == 0) {
                        Kind::Ok
                    } else {
                        Kind::Info
                    },
                    answer_words(a),
                )]));
                let tabs: Vec<Tab> = List::ALL.iter().map(|l| Tab(*l, l.count(a))).collect();
                let current = Tab(w.list, w.list.count(a));
                body = body.push(Segmented::new(tabs, current, |t: Tab| {
                    msg(Event::List(t.0))
                }));
                body = body.push(self.compared_rows(w, a));
            }
        }
        Frame::new(TITLE)
            .scroll(body)
            .action(words::secondary(CLOSE, Some(msg(Event::Close))))
            .action(words::secondary(
                WRITE,
                (can_write && w.asking.is_none()).then(|| msg(Event::Write)),
            ))
            .action(words::primary(
                COMPARE,
                w.asking.is_none().then(|| msg(Event::Compare)),
            ))
            .width(760.0)
            .max_height(760.0)
            .modal(msg(Event::Close))
    }

    /// The chosen list's rows: its words left, its detail right; pressed, its door or objects shown.
    fn compared_rows<'a>(&self, w: &'a Window, a: &'a AddressesCompared) -> Element<'a, Message> {
        let format = self.format();
        let part = |p: AddressPart| match p {
            AddressPart::Street => "Yol",
            AddressPart::Door => "Kapı no",
            AddressPart::Neighbourhood => "Mahalle",
            AddressPart::District => "İlçe",
        };
        let rows: Vec<(String, String)> = match w.list {
            List::Unregistered => a
                .unregistered_doors
                .iter()
                .map(|d| (format!("{} No: {}", d.street, d.door), d.code.clone()))
                .collect(),
            List::Differences => a
                .differences
                .iter()
                .map(|d| {
                    let parts: Vec<String> = d
                        .parts
                        .iter()
                        .map(|p| format!("{}: {} → {}", part(p.part), p.drawing, p.register))
                        .collect();
                    (parts.join("; "), format!("{} · satır {}", d.code, d.row))
                })
                .collect(),
            List::Distances => a
                .distances
                .iter()
                .map(|d| {
                    (
                        format!("{} m uzakta", format.length(d.distance)),
                        format!("{} · satır {}", d.code, d.row),
                    )
                })
                .collect(),
            List::Undrawn => a
                .undrawn_addresses
                .iter()
                .map(|u| {
                    let hood = if u.neighbourhood.is_empty() {
                        String::new()
                    } else {
                        format!(", {}", u.neighbourhood)
                    };
                    (
                        format!("{} No: {}{hood}", u.street, u.door),
                        format!("{} kod · satır {}", u.codes.len(), rows_words(&u.rows)),
                    )
                })
                .collect(),
            List::Repeats => a
                .register_repeats
                .iter()
                .map(|r| (r.code.clone(), format!("satır {}", rows_words(&r.rows))))
                .collect(),
        };
        if rows.is_empty() {
            return label::muted("Bu listede bulgu yok.").into();
        }
        let chosen = w.chosen;
        let count = rows.len();
        let list = VirtualList::new(count, typography::scaled(ROW), move |i| {
            let (left, right) = &rows[i];
            let face = row![
                Elided::new(left.clone())
                    .size(typography::caption())
                    .font(typography::ui())
                    .style(|theme: &iced::Theme| iced::widget::text::Style {
                        color: Some(Tokens::of(theme).text),
                    })
                    .width(Fill),
                container(label::caption(right.clone())).width(typography::scaled(220.0)),
            ]
            .spacing(8)
            .padding([0, 8])
            .height(typography::scaled(ROW))
            .align_y(Center);
            iced::widget::button(face)
                .on_press(msg(Event::Pick(i)))
                .padding(0)
                .width(Fill)
                .style(style::button::row(chosen == Some(i)))
                .into()
        })
        .height(typography::scaled(208.0));
        container(list)
            .style(style::container::field_box)
            .width(Fill)
            .into()
    }

    /// The window's answer (tests read it).
    #[cfg(test)]
    pub(crate) fn address_compared(&self) -> Option<&AddressesCompared> {
        self.address
            .compare
            .as_ref()?
            .answer
            .as_ref()?
            .as_ref()
            .ok()
    }

    /// Chooses the register file without its dialog (tests and scenes).
    #[cfg(test)]
    pub(crate) fn address_compare_file(&mut self, path: PathBuf) {
        let _ = self.address_compare_event(Event::Chosen(Some(path)));
    }

    /// The comparison run here, not on its thread (tests and scenes): as Karşılaştır's answer comes.
    #[cfg(test)]
    pub(crate) fn address_compare_now(&mut self) {
        let Some(w) = self.address.compare.as_ref() else {
            return;
        };
        let (Ok(input), Some(doc)) = (self.compare_input(w), self.document.as_ref()) else {
            return;
        };
        let result = addresses_compare::plan_in(&doc.model, &input);
        if let Some(w) = self.address.compare.as_mut() {
            w.tickets += 1;
            w.asking = Some(w.tickets);
        }
        let ticket = self.address.compare.as_ref().map_or(0, |w| w.tickets);
        self.address_compare_answered(ticket, result);
    }
}

/// Rows' numbers, the first three and how many more.
fn rows_words(rows: &[u32]) -> String {
    let mut words: Vec<String> = rows.iter().take(3).map(u32::to_string).collect();
    if rows.len() > 3 {
        words.push(format!("+{}", rows.len() - 3));
    }
    words.join(", ")
}

/// The answer in a sentence: the numbers stand alone, no suffix bent onto them.
fn answer_words(a: &AddressesCompared) -> String {
    let mut out = format!(
        "Kapılar: {}; kodlu: {}; kodu kayıtta: {}. Kayıt: {} kodlu satır",
        a.doors, a.coded, a.matched, a.rows
    );
    if a.uncoded_rows > 0 {
        out.push_str(&format!(", kodsuz satır (alınmadı): {}", a.uncoded_rows));
    }
    out.push('.');
    if List::ALL.iter().all(|l| l.count(a) == 0) {
        out.push_str(" Kayıtla uyuşmayan bir şey yok.");
    }
    out
}
