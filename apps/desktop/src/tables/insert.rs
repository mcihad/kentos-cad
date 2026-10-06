//! Tablo ekle (docs/adr/0184 §3, §4; the web's `ui/table/TableInsertDialog.ts`):
//! the source (Boş tablo, Dosyadan, a schedule of objects picked with
//! Sahneden seç or selected before), the heading row, Yazı stili (a CAD
//! project's) and the text height on paper, Çizgiler and Kalın çerçeve; a
//! preview of the first rows and the table's size. Yerleştir hands the
//! table to the placement tool (`kentos_interaction::table_place`). The
//! window keeps its choices for as long as the app runs; objects selected
//! when it opens are the schedule's.

use iced::widget::{Column, button, column, container, row, text_input};
use iced::{Center, Element, Fill, Length, Task, Theme};
use kentos_contracts::{TableFileRead, TableGrid, TableSource, TextLook};
use kentos_domain::{Slot, Uuid};
use kentos_geometry_core::ops::table::{Cells, ScheduleKind};
use kentos_interaction::Level;
use kentos_interaction::table::{self as table, Look};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::table::{self as grid, Table};
use kentos_ui::widget::{Dialog as Frame, Segmented, Tip, focus_ring, overlay, tip};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const INSERT_TITLE: &str = "Tablo ekle";
const PLACE: &str = "Yerleştir";
const CANCEL: &str = "Vazgeç";
const PICK: &str = "Sahneden seç";
const HEADER: &str = "İlk satır başlık";
const FRAME: &str = "Çerçeve";
/// The rows and columns the preview shows at most.
const PREVIEW_ROWS: usize = 8;
const PREVIEW_COLUMNS: usize = 6;
/// The most rows and columns Boş tablo asks for.
const BLANK_MOST: usize = 100;

/// Where a new table's rows come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Blank,
    File,
    Coordinates,
    Areas,
    Attributes,
}

impl Source {
    const ALL: [Source; 5] = [
        Source::Blank,
        Source::File,
        Source::Coordinates,
        Source::Areas,
        Source::Attributes,
    ];

    fn words(self) -> &'static str {
        match self {
            Source::Blank => "Boş tablo",
            Source::File => "Dosyadan",
            Source::Coordinates => "Koordinat",
            Source::Areas => "Alan",
            Source::Attributes => "Öznitelik",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Source::Blank => "table",
            Source::File => "tableFile",
            Source::Coordinates => "tableCoordinates",
            Source::Areas => "tableAreas",
            Source::Attributes => "tableAttributes",
        }
    }

    fn hint(self) -> &'static str {
        match self {
            Source::Blank => {
                "Satır ve sütun sayısı verilen boş tablo: hücreler Tabloyu düzenle ile yazılır."
            }
            Source::File => {
                "Excel (.xlsx) çalışma kitabının bir sayfası ya da CSV/TXT dosyası; ayırıcı ve kodlama dosyadan anlaşılır."
            }
            Source::Coordinates => {
                "Koordinat çizelgesi: noktaların ve çizgi, çoklu çizgi ve alan köşelerinin adları, koordinatları ve kotları; ortak köşe bir kez."
            }
            Source::Areas => {
                "Alan çizelgesi: alanların ve dairelerin adları, alanları ve çevreleri; birden çoksa toplamı."
            }
            Source::Attributes => {
                "Öznitelik tablosu: nesnelerin adları ve öznitelikleri, her öznitelik bir sütun."
            }
        }
    }

    fn schedule(self) -> Option<ScheduleKind> {
        match self {
            Source::Coordinates => Some(ScheduleKind::Coordinates),
            Source::Areas => Some(ScheduleKind::Areas),
            Source::Attributes => Some(ScheduleKind::Attributes),
            _ => None,
        }
    }

    /// The kinds Sahneden seç takes for a schedule (the web's `SCHEDULE_KINDS`).
    fn kinds(self) -> Option<Vec<String>> {
        let kinds: &[&str] = match self {
            Source::Coordinates => &["point", "line", "polyline", "polygon"],
            Source::Areas => &["polygon", "circle", "ellipse"],
            _ => return None,
        };
        Some(kinds.iter().map(|k| (*k).to_owned()).collect())
    }
}

/// Çizgiler's choices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lines {
    All,
    Outer,
    Rows,
    None,
}

impl std::fmt::Display for Lines {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Lines::All => "Tümü",
            Lines::Outer => "Dış",
            Lines::Rows => "Satırlar",
            Lines::None => "Yok",
        })
    }
}

impl Lines {
    const ALL: [Lines; 4] = [Lines::All, Lines::Outer, Lines::Rows, Lines::None];

    fn grid(self) -> Option<TableGrid> {
        match self {
            Lines::All => None,
            Lines::Outer => Some(TableGrid::Outer),
            Lines::Rows => Some(TableGrid::Rows),
            Lines::None => Some(TableGrid::None),
        }
    }
}

/// What Tablo ekle keeps for as long as the app runs (the web's `insertState`).
#[derive(Clone, Debug)]
pub struct Insert {
    pub source: Source,
    pub rows: String,
    pub columns: String,
    pub header: bool,
    /// The cells' height on paper, mm, as typed.
    pub height_mm: String,
    pub lines: Lines,
    pub frame: bool,
    /// Kalın çerçeve's width on paper, mm, as typed.
    pub frame_mm: String,
    /// The file read: its name and sheets.
    pub file: Option<(String, TableFileRead)>,
    pub sheet: usize,
    /// The schedule's objects, by persistent id, in the drawing's order.
    pub objects: Vec<Uuid>,
    /// Yazı stili: a project text style's id; none, Standart.
    pub style: Option<String>,
    /// Picking objects: the selection to put back.
    pub(crate) aside: Option<Vec<Slot>>,
}

impl Default for Insert {
    fn default() -> Self {
        Self {
            source: Source::Blank,
            rows: "4".to_owned(),
            columns: "3".to_owned(),
            header: true,
            height_mm: "2.5".to_owned(),
            lines: Lines::All,
            frame: false,
            frame_mm: "0.7".to_owned(),
            file: None,
            sheet: 0,
            objects: Vec::new(),
            style: None,
            aside: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Source(Source),
    Rows(String),
    Columns(String),
    Header(bool),
    Height(String),
    Lines(Lines),
    Frame(bool),
    FrameWidth(String),
    Style(Option<String>),
    Sheet(usize),
    ChooseFile,
    FileRead(Option<(String, Vec<u8>)>),
    Pick,
    Place,
    Close,
}

fn msg(event: Event) -> Message {
    Message::TableInsert(event)
}

/// A typed number: a decimal point or comma.
fn number(t: &str) -> Option<f64> {
    t.trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}

/// A whole number from 1 to `most`.
fn count(t: &str, most: usize) -> Option<usize> {
    t.trim()
        .parse::<usize>()
        .ok()
        .filter(|n| (1..=most).contains(n))
}

/// The cells the source gives now, with their source; or why none.
enum Now {
    Cells(Cells, Option<TableSource>),
    Why(Kind, String),
}

impl App {
    /// Tablo ekle (`table.insert`): the window; objects selected now are
    /// the schedule's (tables left out).
    pub(crate) fn open_table_insert(&mut self, picked: bool) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if !picked {
            let model = &doc.model;
            let slots = table::in_order(model, self.selection.ids());
            self.table_insert.objects = slots
                .into_iter()
                .filter(|s| !matches!(model.get(*s), Some(kentos_contracts::Entity::Table(_))))
                .filter_map(|s| model.uid(s))
                .collect();
            self.table_insert.style = self.memory.text_style.map(|id| id.to_string());
        }
        self.dialog = Some(Dialog::TableInsert);
    }

    /// The schedule's objects still in the drawing, in the order kept.
    fn table_objects(&self) -> Vec<Slot> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        self.table_insert
            .objects
            .iter()
            .filter_map(|u| doc.model.slot_of(*u))
            .collect()
    }

    fn table_cells_now(&self) -> Now {
        let w = &self.table_insert;
        let Some(doc) = &self.document else {
            return Now::Why(Kind::Info, "Açık çizim yok.".to_owned());
        };
        match w.source {
            Source::Blank => match (count(&w.rows, BLANK_MOST), count(&w.columns, BLANK_MOST)) {
                (Some(n), Some(m)) => Now::Cells(table::blank_cells(n, m), None),
                _ => Now::Why(
                    Kind::Error,
                    format!(
                        "Satır ve sütun sayısı 1 ile {BLANK_MOST} arasında bir tam sayı olmalı."
                    ),
                ),
            },
            Source::File => {
                let Some((name, read)) = &w.file else {
                    return Now::Why(
                        Kind::Info,
                        "Bir Excel (.xlsx), CSV ya da TXT dosyası seçin.".to_owned(),
                    );
                };
                if let Some(problem) = &read.problem {
                    return Now::Why(Kind::Error, problem.clone());
                }
                let Some(sheet) = read.sheets.get(w.sheet).or_else(|| read.sheets.first()) else {
                    return Now::Why(Kind::Error, "Dosyada okunacak sayfa yok.".to_owned());
                };
                let cells = kentos_interaction::table::sheet_cells(sheet, w.header);
                match &cells.problem {
                    Some(p) => Now::Why(Kind::Error, p.clone()),
                    None => Now::Cells(
                        cells,
                        Some(TableSource::File {
                            name: name.clone(),
                            sheet: sheet.name.clone(),
                        }),
                    ),
                }
            }
            source => {
                let Some(kind) = source.schedule() else {
                    return Now::Why(Kind::Info, String::new());
                };
                let slots = self.table_objects();
                if slots.is_empty() {
                    return Now::Why(
                        Kind::Info,
                        "Nesne seçilmedi: Sahneden seç ile çizimden seçin.".to_owned(),
                    );
                }
                let cells = table::schedule(&doc.model, kind, &slots, &self.format());
                match &cells.problem {
                    Some(p) => Now::Why(Kind::Error, p.clone()),
                    None => Now::Cells(cells, Some(table::source_of(&doc.model, kind, &slots))),
                }
            }
        }
    }

    /// Tablo ekle's look: the chosen style's face and fixed height, else the
    /// paper height; lines and frame at the plot scale (the web's `lookOf`).
    fn table_look(&self) -> Option<Look> {
        let w = &self.table_insert;
        let doc = self.document.as_ref()?;
        let settings = doc.model.settings();
        let scale = settings.plot_scale;
        let style = w
            .style
            .as_ref()
            .and_then(|id| settings.text_styles.iter().find(|s| &s.id == id));
        let mm = number(&w.height_mm).filter(|v| *v > 0.0)?;
        let look = kentos_contracts::apply_text_style(
            style,
            &TextLook {
                face: Default::default(),
                width_factor: None,
                height: mm / 1000.0 * scale,
            },
            scale,
        );
        let frame = if w.frame && w.lines != Lines::None {
            Some(number(&w.frame_mm).filter(|v| *v > 0.0)? / 1000.0 * scale)
        } else {
            None
        };
        Some(Look {
            header: w.source.schedule().is_some() || w.header,
            height: look.height,
            face: look.face,
            grid: w.lines.grid(),
            frame,
        })
    }

    pub(crate) fn table_insert_event(&mut self, event: Event) -> Task<Message> {
        let w = &mut self.table_insert;
        match event {
            Event::Source(s) => w.source = s,
            Event::Rows(t) => w.rows = t,
            Event::Columns(t) => w.columns = t,
            Event::Header(on) => w.header = on,
            Event::Height(t) => w.height_mm = t,
            Event::Lines(l) => w.lines = l,
            Event::Frame(on) => w.frame = on,
            Event::FrameWidth(t) => w.frame_mm = t,
            Event::Style(id) => w.style = id,
            Event::Sheet(i) => w.sheet = i,
            Event::ChooseFile => {
                return Task::perform(
                    async {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("Tablo dosyası")
                            .add_filter("Tablo (.xlsx, .csv, .txt)", &["xlsx", "csv", "txt", "tsv"])
                            .pick_file()
                            .await?;
                        let bytes = file.read().await;
                        Some((file.file_name(), bytes))
                    },
                    |read| msg(Event::FileRead(read)),
                );
            }
            Event::FileRead(None) => {}
            Event::FileRead(Some((name, bytes))) => {
                let read = kentos_formats::table_file::read(&bytes);
                w.file = Some((name, read));
                w.sheet = 0;
            }
            Event::Pick => self.table_insert_pick(),
            Event::Place => self.table_insert_place(),
            Event::Close => {
                self.dialog = None;
            }
        }
        Task::none()
    }

    /// Sahneden seç: the window steps aside, the objects are picked, the
    /// window comes back with them (the web's `pickObjects`).
    fn table_insert_pick(&mut self) {
        let Some(doc) = &self.document else {
            return;
        };
        let kinds = self.table_insert.source.kinds();
        let now: Vec<Slot> = self.table_objects();
        let _ = doc;
        self.table_insert.aside = Some(self.selection.ids().to_vec());
        self.selection.set(now);
        self.dialog = None;
        self.field = None;
        self.snap = None;
        self.say(Level::Command, format!("{INSERT_TITLE}: nesneler"));
        self.session.run(Box::new(
            kentos_interaction::pick_objects::PickObjects::new("Nesneler", kinds),
        ));
        self.with_tool(|s, cx| s.activate(cx));
    }

    /// The pick over: the objects kept (Enter) become the schedule's; the
    /// selection and the window come back. False when Tablo ekle was not picking.
    pub(crate) fn table_insert_picked(&mut self, keep: bool) -> bool {
        let Some(before) = self.table_insert.aside.take() else {
            return false;
        };
        if keep && let Some(doc) = &self.document {
            let slots = table::in_order(&doc.model, self.selection.ids());
            self.table_insert.objects =
                slots.into_iter().filter_map(|s| doc.model.uid(s)).collect();
        }
        self.selection.set(before);
        self.open_table_insert(true);
        true
    }

    /// Yerleştir: the window closes and the table hangs from the cursor.
    fn table_insert_place(&mut self) {
        let (Now::Cells(cells, source), Some(look)) = (self.table_cells_now(), self.table_look())
        else {
            return;
        };
        let Some(doc) = &self.document else {
            return;
        };
        let geometry = table::new_table(
            &doc.model,
            &cells,
            &look,
            kentos_interaction::Vec2::new(0.0, 0.0),
            source,
        );
        self.memory.text_style = self
            .table_insert
            .style
            .as_deref()
            .and_then(|id| Uuid::parse_str(id).ok());
        self.dialog = None;
        self.field = None;
        self.session
            .run(Box::new(kentos_interaction::table_place::TablePlace::new(
                geometry,
            )));
        self.say(Level::Command, INSERT_TITLE);
        self.with_tool(|s, cx| s.activate(cx));
    }

    pub(crate) fn table_insert_view(&self) -> Element<'_, Message> {
        let w = &self.table_insert;
        let Some(doc) = &self.document else {
            return label::body("").into();
        };
        let settings = doc.model.settings();
        let format = self.format();
        // Kaynak: five cards, an icon over each name.
        let cards = row(Source::ALL.into_iter().map(|s| {
            let chosen = s == w.source;
            let face = column![
                icon(crate::icons::from_web(Some(s.icon()))).size(22.0),
                label::body(s.words()).font(if chosen {
                    typography::ui_strong()
                } else {
                    typography::ui()
                }),
            ]
            .spacing(6)
            .align_x(Center);
            tip(
                button(container(face).center_x(Fill).center_y(Fill))
                    .width(Fill)
                    .height(Length::Fixed(typography::scaled(62.0)))
                    .padding([8, 6])
                    .style(move |theme: &Theme, status| card(theme, status, chosen))
                    .on_press(msg(Event::Source(s))),
                Tip::new(s.hint()),
                iced::widget::tooltip::Position::Bottom,
            )
        }))
        .spacing(8);
        let hint = label::caption(w.source.hint());
        let detail: Element<'_, Message> = match w.source {
            Source::Blank => column![
                row![
                    words::field(
                        "Satır",
                        focus_ring(
                            text_input("4", &w.rows)
                                .on_input(|t| msg(Event::Rows(t)))
                                .padding([5, 8])
                                .width(90)
                                .style(style::field::input)
                        ),
                        None
                    ),
                    words::field(
                        "Sütun",
                        focus_ring(
                            text_input("3", &w.columns)
                                .on_input(|t| msg(Event::Columns(t)))
                                .padding([5, 8])
                                .width(90)
                                .style(style::field::input)
                        ),
                        None
                    ),
                    words::field(
                        "Başlık",
                        words::check(w.header, HEADER, Some(msg(Event::Header(!w.header)))),
                        None
                    ),
                ]
                .spacing(18),
                hint,
            ]
            .spacing(8)
            .into(),
            Source::File => {
                let (name, meta) = match &w.file {
                    Some((name, read)) if read.problem.is_none() => {
                        let n = read.sheets.len();
                        let mut said = Vec::new();
                        if n > 1 {
                            said.push(format!("{n} sayfa"));
                        }
                        if let Some(e) = &read.encoding {
                            said.push(e.clone());
                        }
                        (name.clone(), said.join(" · "))
                    }
                    Some((name, _)) => (name.clone(), String::new()),
                    None => ("Dosya seçilmedi".to_owned(), String::new()),
                };
                let choose = button(
                    row![
                        icon(crate::icons::from_web(Some("tableFile"))).size(16.0),
                        label::body(if w.file.is_some() {
                            "Başka dosya…"
                        } else {
                            "Dosya seç…"
                        })
                    ]
                    .spacing(6)
                    .align_y(Center),
                )
                .padding([5, 12])
                .style(style::button::secondary)
                .on_press(msg(Event::ChooseFile));
                let file = container(
                    row![
                        column![
                            label::body(name).font(typography::ui_strong()),
                            label::caption(meta)
                        ]
                        .spacing(2)
                        .width(Fill),
                        choose
                    ]
                    .spacing(10)
                    .align_y(Center),
                )
                .padding([8, 10])
                .width(Fill)
                .style(style::container::bordered);
                let sheets = w.file.as_ref().map_or(0, |(_, r)| r.sheets.len());
                let mut options = row![].spacing(18);
                if sheets > 1
                    && let Some((_, read)) = &w.file
                {
                    let names: Vec<Choice> = read
                        .sheets
                        .iter()
                        .enumerate()
                        .map(|(i, s)| {
                            Choice::new(
                                s.name.clone().unwrap_or_else(|| format!("Sayfa {}", i + 1)),
                            )
                        })
                        .collect();
                    options = options.push(words::field(
                        "Sayfa",
                        Select::new(names, Some(w.sheet), |i| msg(Event::Sheet(i))),
                        None,
                    ));
                }
                options = options.push(words::field(
                    "Başlık",
                    words::check(w.header, HEADER, Some(msg(Event::Header(!w.header)))),
                    None,
                ));
                column![file, options, hint].spacing(8).into()
            }
            _ => {
                let n = self.table_objects().len();
                let pick = button(
                    row![icon(Icon::Target).size(16.0), label::body(PICK)]
                        .spacing(6)
                        .align_y(Center),
                )
                .padding([5, 12])
                .style(style::button::secondary)
                .on_press(msg(Event::Pick));
                let objects = container(
                    row![
                        label::body(if n > 0 {
                            format!("{n} nesne")
                        } else {
                            "Nesne seçilmedi".to_owned()
                        })
                        .font(typography::ui_strong())
                        .width(Fill),
                        tip(
                            pick,
                            Tip::new("Pencere çizimin yanına çekilir; nesneleri tıklayın ya da pencereyle seçin, Enter bitirir."),
                            iced::widget::tooltip::Position::Top,
                        )
                    ]
                    .spacing(10)
                    .align_y(Center),
                )
                .padding([8, 10])
                .width(Fill)
                .style(style::container::bordered);
                column![objects, hint].spacing(8).into()
            }
        };
        // Görünüş: Yazı stili (a CAD project's) and the height; Çizgiler and Kalın çerçeve.
        let scale = settings.plot_scale;
        let style_now = w
            .style
            .as_ref()
            .and_then(|id| settings.text_styles.iter().find(|s| &s.id == id));
        let fixed = style_now.and_then(|s| s.height);
        let mut looks = row![].spacing(18);
        if kentos_interaction::styles::shown(settings) {
            let ids: Vec<Option<String>> = std::iter::once(None)
                .chain(settings.text_styles.iter().map(|s| Some(s.id.clone())))
                .collect();
            let names: Vec<Choice> = std::iter::once(Choice::new(kentos_contracts::STANDARD_STYLE))
                .chain(
                    settings
                        .text_styles
                        .iter()
                        .map(|s| Choice::new(s.name.clone())),
                )
                .collect();
            let at = ids.iter().position(|id| *id == w.style);
            looks = looks.push(words::field(
                "Yazı stili",
                container(Select::new(names, at, move |i| {
                    msg(Event::Style(ids.get(i).cloned().flatten()))
                }))
                .width(Length::Fixed(typography::scaled(220.0))),
                None,
            ));
        }
        let shown_mm = fixed.or_else(|| number(&w.height_mm)).unwrap_or(0.0);
        let height_hint = format!(
            "Kâğıtta; çizimde {} (1:{}).",
            format.length(shown_mm / 1000.0 * scale),
            kentos_geometry_core::display::fixed(scale, 0)
        );
        let height: Element<'_, Message> = match fixed {
            Some(mm) => label::body(format!(
                "{} mm (stilin)",
                kentos_geometry_core::display::fixed(mm, 2)
            ))
            .into(),
            None => row![
                focus_ring(
                    text_input("2.5", &w.height_mm)
                        .on_input(|t| msg(Event::Height(t)))
                        .padding([5, 8])
                        .width(80)
                        .style(style::field::input)
                ),
                label::muted("mm"),
            ]
            .spacing(6)
            .align_y(Center)
            .into(),
        };
        looks = looks.push(words::field("Yazı yüksekliği", height, Some(height_hint)));
        let lines = Segmented::new(Lines::ALL, w.lines, |l| msg(Event::Lines(l))).icons(
            [
                "tableGridAll",
                "tableGridOuter",
                "tableGridRows",
                "tableGridNone",
            ]
            .into_iter()
            .map(|n| crate::icons::from_web(Some(n))),
        );
        let framed = w.frame && w.lines != Lines::None;
        let mut frame_row = row![tip(
            words::check(
                framed,
                FRAME,
                (w.lines != Lines::None).then(|| msg(Event::Frame(!w.frame)))
            ),
            Tip::new(if w.lines == Lines::None {
                "Çizgisiz tabloda çerçeve yok."
            } else {
                "Dış çizgi, kâğıtta bu kalınlıkta, tablonun içine doğru dolu bir bant olur."
            }),
            iced::widget::tooltip::Position::Top,
        )]
        .spacing(10)
        .align_y(Center);
        if framed {
            frame_row = frame_row.push(
                row![
                    focus_ring(
                        text_input("0.7", &w.frame_mm)
                            .on_input(|t| msg(Event::FrameWidth(t)))
                            .padding([5, 8])
                            .width(70)
                            .style(style::field::input)
                    ),
                    label::muted("mm"),
                ]
                .spacing(6)
                .align_y(Center),
            );
        }
        let lines_row = row![
            words::field("Çizgiler", lines, None),
            words::field("Kalın çerçeve", frame_row, None),
        ]
        .spacing(18);
        let appearance = column![
            label::body("Görünüş").font(typography::ui_strong()),
            looks,
            lines_row
        ]
        .spacing(10);
        // The preview and the summary.
        let now = self.table_cells_now();
        let look = self.table_look();
        let (preview, summary, ready): (Element<'_, Message>, Element<'_, Message>, bool) =
            match &now {
                Now::Why(kind, why) => (
                    Column::new().into(),
                    words::summary(vec![words::text_line(*kind, why.clone())]),
                    false,
                ),
                Now::Cells(cells, source) => {
                    let header = w.source.schedule().is_some() || w.header;
                    let m = cells.cells.first().map_or(0, Vec::len).min(PREVIEW_COLUMNS);
                    let columns = (0..m).map(|j| {
                        let c = grid::Column::new(if header {
                            cells.cells[0][j].clone()
                        } else {
                            String::new()
                        })
                        .width(Fill);
                        if cells.aligns.get(j).is_some_and(|a| a == "right") {
                            c.align_right()
                        } else {
                            c
                        }
                    });
                    let body_rows = cells
                        .cells
                        .iter()
                        .skip(usize::from(header))
                        .take(PREVIEW_ROWS)
                        .map(|r| {
                            grid::Row::new(
                                r.iter()
                                    .take(m)
                                    .map(|w| label::caption(w.clone()).into())
                                    .collect::<Vec<Element<'_, Message>>>(),
                            )
                        });
                    let mut more = Vec::new();
                    let shown_rows = cells.cells.len().saturating_sub(usize::from(header));
                    if shown_rows > PREVIEW_ROWS {
                        more.push(format!("{} satır daha", shown_rows - PREVIEW_ROWS));
                    }
                    let all_columns = cells.cells.first().map_or(0, Vec::len);
                    if all_columns > PREVIEW_COLUMNS {
                        more.push(format!("{} sütun daha", all_columns - PREVIEW_COLUMNS));
                    }
                    let mut preview = column![
                        label::caption("Önizleme"),
                        container(Table::new(columns).extend(body_rows))
                            .width(Fill)
                            .style(style::container::field_box),
                    ]
                    .spacing(4);
                    if !more.is_empty() {
                        preview = preview.push(label::caption(format!("… {}.", more.join(", "))));
                    }
                    let mut lines = Vec::new();
                    match &look {
                    Some(look) => {
                        let t = table::new_table(
                            &doc.model,
                            cells,
                            look,
                            kentos_interaction::Vec2::new(0.0, 0.0),
                            source.clone(),
                        );
                        if let kentos_contracts::EntityGeometry::Table { rows, columns, .. } = &t {
                            lines.push(words::text_line(
                                Kind::Ok,
                                format!(
                                    "{} satır × {} sütun; tablo {} × {}.",
                                    cells.cells.len(),
                                    all_columns,
                                    format.length(columns.iter().sum()),
                                    format.length(rows.iter().sum())
                                ),
                            ));
                        }
                    }
                    None => lines.push(words::text_line(
                        Kind::Error,
                        "Yazı yüksekliği ve çerçeve kalınlığı sıfırdan büyük bir sayı olmalı (mm).",
                    )),
                }
                    if cells.fitted > 0 {
                        lines.push(words::text_line(
                            Kind::Info,
                            format!(
                                "{} hücrenin satır sonları boşluk oldu: hücre tek satırdır.",
                                cells.fitted
                            ),
                        ));
                    }
                    (preview.into(), words::summary(lines), look.is_some())
                }
            };
        let content = column![
            words::field("Kaynak", cards, None),
            detail,
            iced::widget::rule::horizontal(1),
            appearance,
            preview,
            summary,
        ]
        .spacing(12);
        overlay::modal(
            Frame::new(INSERT_TITLE)
                .scroll(content)
                .action(words::secondary(CANCEL, Some(msg(Event::Close))))
                .action(words::primary(PLACE, ready.then(|| msg(Event::Place))))
                .width(700.0),
            msg(Event::Close),
        )
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn table_insert_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let w = &self.table_insert;
        Ok(match control {
            Control::Press(PLACE) => Some(msg(Event::Place)),
            Control::Press(CANCEL) => Some(msg(Event::Close)),
            Control::Press(PICK) => Some(msg(Event::Pick)),
            Control::Fill("Satır", t) => Some(msg(Event::Rows(t.to_owned()))),
            Control::Fill("Sütun", t) => Some(msg(Event::Columns(t.to_owned()))),
            Control::Fill("Yazı yüksekliği", t) => Some(msg(Event::Height(t.to_owned()))),
            Control::Fill("Çerçeve kalınlığı", t) => {
                Some(msg(Event::FrameWidth(t.to_owned())))
            }
            Control::Check(HEADER, on) => (w.header != on).then(|| msg(Event::Header(on))),
            Control::Check(FRAME, on) => (w.frame != on).then(|| msg(Event::Frame(on))),
            Control::Press(words) => {
                if let Some(s) = Source::ALL.into_iter().find(|s| s.words() == words) {
                    Some(msg(Event::Source(s)))
                } else if let Some(l) = Lines::ALL.into_iter().find(|l| l.to_string() == words) {
                    Some(msg(Event::Lines(l)))
                } else {
                    return Err(format!("“{INSERT_TITLE}” penceresinde “{words}” yok"));
                }
            }
            other => return Err(format!("“{INSERT_TITLE}” penceresinde {other} yok")),
        })
    }
}

/// A source card: bordered, the chosen one in the accent (the web's `.table-source`).
fn card(
    theme: &Theme,
    status: iced::widget::button::Status,
    chosen: bool,
) -> iced::widget::button::Style {
    let t = Tokens::of(theme);
    let hovered = matches!(status, iced::widget::button::Status::Hovered);
    let (background, edge, text) = match (chosen, hovered) {
        (true, _) => (t.selection(), t.accent_line(), t.accent),
        (false, true) => (t.layer(0.06), t.border_strong(), t.text),
        (false, false) => (t.field, t.border, t.muted),
    };
    iced::widget::button::Style {
        background: Some(iced::Background::Color(background)),
        text_color: text,
        border: iced::Border {
            color: edge,
            width: 1.0,
            radius: kentos_ui::theme::shape::md().into(),
        },
        ..iced::widget::button::Style::default()
    }
}
