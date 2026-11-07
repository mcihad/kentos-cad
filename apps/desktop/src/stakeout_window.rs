//! Aplikasyon cetveli (plan/road.md Y10, docs/adr/0908, `stakeout.table`): a corridor's coded section points as Civil
//! 3D's corridor points report and OpenRoads' cross section grade book list them: km, code, side, offset, Y, X, Z, on
//! the grid or with the offsets also as ground lengths. The table is read as it is typed (`road.stakeout.query`), its
//! points drawn in plan, and written as a workbook, or sent to an instrument (`road.stakeout.send`, docs/adr/0908 §9):
//! as a LandXML string road or an instrument's coordinate file; nothing in the drawing changes.
//!
//! ```text
//! ┌ Aplikasyon cetveli ────────────────────────────────────────────────────────────────────────────┐
//! │ Koridor [▾]  Başlangıç km [ ]  Bitiş km [ ]  Aralık (m) [ ]  Kodlar [axis, hinge, …]  Öteleme [▾] │
//! │ method · n km · m nokta · warnings                                                               │
//! │ ┌ points in plan ┐  Km  Bölge  Kod  Yan  Öteleme  Zemin  Y  X  Z                                  │
//! │ └────────────────┘  …                                                                            │
//! │ Biçim [▾] [Cetveli yaz] │ Alete gönder [▾]  İlk no [ ]  İş adı [ ]  [Alete gönder…]  what goes  [Kapat] │
//! └──────────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::canvas::{self, Path, Stroke};
use iced::widget::{container, row};
use iced::{Element, Fill, Point, Rectangle, Renderer, Task, Theme, mouse};
use std::path::PathBuf;

use kentos_commands::{
    CommandWarning, StakeSide, StakeoutDistances, StakeoutQueried, StakeoutQuery, StakeoutSend, StakeoutSendFormat,
    WorkbookFormat,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, road_stakeout, road_stakeout_send};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Aplikasyon cetveli";
const CLOSE: &str = "Kapat";
const WRITE: &str = "Cetveli yaz";
const SEND: &str = "Alete gönder…";
/// The formats a stakeout is sent as, with their labels, in the order the list shows them.
const SENDS: [(StakeoutSendFormat, &str); 7] = [
    (StakeoutSendFormat::Landxml, "LandXML string yol"),
    (StakeoutSendFormat::Gsi16, "Leica GSI-16"),
    (StakeoutSendFormat::Gsi8, "Leica GSI-8"),
    (StakeoutSendFormat::Gts7, "Topcon GTS-7"),
    (StakeoutSendFormat::Jobxml, "Trimble JobXML"),
    (StakeoutSendFormat::Nikon, "Nikon RAW"),
    (StakeoutSendFormat::Csv, "CSV (Ad, Y, X, Z, Kod)"),
];
/// The most rows the window lists; the file has them all.
const SHOWN: usize = 400;

/// The table as last read.
#[derive(Debug, Clone, Default)]
pub(crate) struct Preview {
    pub(crate) table: Option<StakeoutQueried>,
    pub(crate) warnings: Vec<CommandWarning>,
    pub(crate) why: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub(crate) corridor: Option<String>,
    /// The range's start, its end, the step and the codes, as typed.
    pub(crate) fields: [String; 4],
    pub(crate) distances: StakeoutDistances,
    format: WorkbookFormat,
    pub(crate) preview: Preview,
    pub(crate) send: StakeoutSendFormat,
    /// The first point number and the job's name, as typed.
    pub(crate) send_fields: [String; 2],
}

#[derive(Debug, Clone)]
pub enum Event {
    Corridor(String),
    /// 0 the start, 1 the end, 2 the step, 3 the codes.
    Field(usize, String),
    Distances(StakeoutDistances),
    Format(WorkbookFormat),
    Write,
    SendFormat(StakeoutSendFormat),
    /// 0 the first number, 1 the job.
    SendField(usize, String),
    Send,
    /// Where the file goes, from the save dialog.
    Sent(Option<PathBuf>),
    Close,
}

fn msg(event: Event) -> Message {
    Message::Stakeout(event)
}

fn number(text: &str) -> Result<Option<f64>, String> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>().map(Some).map_err(|_| format!("“{t}” bir sayı değil (ondalık ayırıcı nokta)."))
}

fn grouped(v: f64, digits: usize) -> String {
    let text = format!("{:.digits$}", v.abs());
    let (whole, fraction) = text.split_once('.').unwrap_or((&text, ""));
    let mut out = String::new();
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    let sign = if v < 0.0 && text.chars().any(|c| c.is_ascii_digit() && c != '0') { "−" } else { "" };
    if fraction.is_empty() { format!("{sign}{out}") } else { format!("{sign}{out},{fraction}") }
}

impl Window {
    /// The query as typed, or why it is not one.
    fn query(&self) -> Result<StakeoutQuery, String> {
        let corridor = self.corridor.clone().ok_or("Bir koridor seçin.")?;
        let codes: Vec<String> =
            self.fields[3].split([',', ' ', ';']).map(str::trim).filter(|c| !c.is_empty()).map(str::to_owned).collect();
        Ok(StakeoutQuery {
            corridor,
            from: number(&self.fields[0])?,
            to: number(&self.fields[1])?,
            step: number(&self.fields[2])?,
            codes: (!codes.is_empty()).then_some(codes),
            distances: self.distances,
        })
    }

    /// The send to `path` as typed, or why it is not one.
    pub(crate) fn send_input(&self, path: String) -> Result<StakeoutSend, String> {
        let q = self.query()?;
        let first = match self.send_fields[0].trim() {
            "" => None,
            t => Some(t.parse::<u32>().map_err(|_| format!("“{t}” bir nokta numarası değil."))?),
        };
        let job = self.send_fields[1].trim();
        Ok(StakeoutSend {
            corridor: q.corridor,
            from: q.from,
            to: q.to,
            step: q.step,
            codes: q.codes,
            format: self.send,
            path,
            overwrite: true,
            first,
            job: (!job.is_empty()).then(|| job.to_owned()),
            stamp: None,
        })
    }
}

impl App {
    /// Aplikasyon cetveli: the window on the road project's first corridor.
    pub(crate) fn open_stakeout(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Aplikasyon cetveli yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let Some(first) = doc.model.road_corridors().first() else {
            self.warn("Aplikasyon cetveli bir koridorun noktalarıdır: önce Koridor'da bir koridor oluşturun.".to_owned());
            return;
        };
        self.stakeout_window = Some(Box::new(Window {
            corridor: Some(first.id.clone()),
            fields: Default::default(),
            distances: StakeoutDistances::Grid,
            format: WorkbookFormat::Xlsx,
            preview: Preview::default(),
            send: StakeoutSendFormat::Landxml,
            send_fields: Default::default(),
        }));
        self.dialog = Some(Dialog::Stakeout);
        self.refresh_stakeout();
    }

    fn refresh_stakeout(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.stakeout_window.as_mut() else { return };
        w.preview = match w.query().and_then(|q| road_stakeout::preview(&doc.model, &q)) {
            Err(why) => Preview { why: Some(why), ..Preview::default() },
            Ok((table, warnings)) => Preview { table: Some(table), warnings, why: None },
        };
    }

    pub(crate) fn stakeout_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.stakeout_window.as_mut() else { return Task::none() };
        match event {
            Event::Corridor(id) => w.corridor = Some(id),
            Event::Field(k, t) => w.fields[k.min(3)] = t,
            Event::Distances(d) => w.distances = d,
            Event::Format(f) => {
                w.format = f;
                return Task::none();
            }
            Event::Write => {
                let Some(table) = &w.preview.table else {
                    let why = w.preview.why.clone().unwrap_or_else(|| "Bir koridor seçin.".into());
                    self.warn(format!("Cetvel yazılamaz: {why}"));
                    return Task::none();
                };
                let name = format!("{} aplikasyon cetveli", table.name);
                // Docs/adr/1038: the sources it was computed from and when it was written, as a sheet beside it.
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
                let written = crate::cloud::local_time::iso_local(i64::try_from(now).unwrap_or(0), crate::cloud::local_time::Zone::system())
                    .replace('T', " ");
                return crate::table_write::save(
                    vec![road_stakeout::sheet(table), road_stakeout::sources_sheet(table, &written)],
                    w.format,
                    name,
                );
            }
            Event::SendFormat(f) => {
                w.send = f;
                return Task::none();
            }
            Event::SendField(k, t) => {
                w.send_fields[k.min(1)] = t;
                return Task::none();
            }
            Event::Send => {
                if let Err(why) = w.send_input(String::new()) {
                    self.warn(format!("Gönderilemez: {why}"));
                    return Task::none();
                }
                let (ext, name) = road_stakeout_send::extension(w.send);
                let file = format!("{} aplikasyon.{ext}", w.preview.table.as_ref().map_or("Koridor", |t| t.name.as_str()));
                let filter = format!("{name} (.{ext})");
                return Task::perform(
                    async move {
                        let f = rfd::AsyncFileDialog::new()
                            .set_title("Aplikasyonu alete gönder")
                            .set_file_name(file)
                            .add_filter(filter, &[ext])
                            .save_file()
                            .await?;
                        Some(f.path().to_path_buf())
                    },
                    |p| msg(Event::Sent(p)),
                );
            }
            Event::Sent(path) => {
                self.stakeout_sent(path);
                return Task::none();
            }
            Event::Close => {
                self.stakeout_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        self.refresh_stakeout();
        Task::none()
    }

    /// The file chosen: the points written by `road.stakeout.send` (the dialog asked before writing over).
    pub(crate) fn stakeout_sent(&mut self, path: Option<PathBuf>) {
        let Some(mut path) = path else { return };
        let Some(w) = self.stakeout_window.as_ref() else { return };
        let (ext, name) = road_stakeout_send::extension(w.send);
        if path.extension().is_none() {
            path.set_extension(ext);
        }
        let input = match w.send_input(path.display().to_string()) {
            Ok(i) => i,
            Err(why) => {
                self.warn(format!("Gönderilemez: {why}"));
                return;
            }
        };
        let Some(doc) = self.document.as_mut() else { return };
        let result = road_stakeout_send::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            let strings = if out.strings > 0 { format!(", {} dizi", out.strings) } else { String::new() };
            let left = if out.left.is_empty() { String::new() } else { format!(", {} nokta taşınamadı", out.left.len()) };
            self.say(Level::Success, format!("{name} yazıldı: {} nokta{strings}{left} ({}).", out.points, path.display()));
        }
    }

    pub(crate) fn road_stakeout_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.stakeout_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let corridors: Vec<(String, String)> = self
            .document
            .as_ref()
            .map_or_else(Vec::new, |d| d.model.road_corridors().iter().map(|c| (c.id.clone(), c.name.clone())).collect());
        let ids: Vec<String> = corridors.iter().map(|c| c.0.clone()).collect();
        let on_corridor = Select::new(
            corridors.iter().map(|c| Choice::new(c.1.clone())),
            w.corridor.as_ref().and_then(|c| ids.iter().position(|i| i == c)),
            move |i| msg(Event::Corridor(ids[i].clone())),
        )
        .placeholder("Koridor");
        let kinds = [(StakeoutDistances::Grid, "Plan boyu"), (StakeoutDistances::Ground, "Plan ve zemin boyu")];
        let on_kind = Select::new(
            kinds.iter().map(|k| Choice::new(k.1.to_owned())),
            kinds.iter().position(|k| k.0 == w.distances),
            move |i| msg(Event::Distances(kinds[i].0)),
        );
        let field = |hint: &'static str, k: usize, width: f32| {
            kentos_ui::widget::text_field(hint, &w.fields[k]).on_input(move |t| msg(Event::Field(k, t))).width(width)
        };
        let mut body = kentos_ui::widget::grow::Grows::new().spacing(10.0);
        body = body.push(
            row![
                words::field("Koridor", container(on_corridor).width(200), None),
                words::field("Başlangıç km (m)", field("baştan", 0, 110.0), None),
                words::field("Bitiş km (m)", field("sona", 1, 110.0), None),
                words::field("Aralık (m)", field("koridorun km'leri", 2, 140.0), None),
                words::field("Kodlar", field("hepsi (axis, hinge, daylight …)", 3, 260.0), None),
                words::field("Öteleme", container(on_kind).width(170), None),
            ]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Bottom),
        );
        match (&w.preview.why, &w.preview.table) {
            (Some(e), _) => body = body.push(label::caption(e.clone()).style(danger)),
            (None, Some(t)) => {
                let shown = if t.rows.len() > SHOWN {
                    format!(" · ilk {SHOWN} nokta gösterilir, cetvel hepsini yazar")
                } else {
                    String::new()
                };
                body = body.push(label::caption(format!(
                    "{} · {} km · {} nokta{shown}",
                    t.method,
                    t.stations,
                    grouped(t.rows.len() as f64, 0)
                )));
                // Docs/adr/1038: what the table is computed from, each as it is now.
                let src = &t.sources;
                let mut said = crate::road_dialog::chain_words(
                    &src.corridor,
                    &src.alignment,
                    &src.profile,
                    &src.templates,
                    src.superelevation.as_ref(),
                    src.surface.as_ref(),
                );
                for v in &src.volumes {
                    said.push_str(&format!(", kübaj {} ({}{})", v.id, v.revision, if v.approved { ", onaylı" } else { "" }));
                }
                body = body.push(label::caption(said).style(muted));
                for wn in &w.preview.warnings {
                    body = body.push(label::caption(wn.message.clone()).style(warn));
                }
                let plan = crate::picture::picture(iced::widget::canvas(Plan { table: t }).width(Fill).height(Fill), TITLE);
                body = body.push_grow(
                    row![container(plan).width(iced::Length::FillPortion(1)), container(rows_table(t)).width(iced::Length::FillPortion(2))].spacing(12).height(Fill),
                    typography::scaled(300.0),
                );
            }
            _ => {}
        }
        let formats: Vec<WorkbookFormat> = crate::table_write::FORMATS.iter().map(|c| c.format).collect();
        let format = Select::new(
            crate::table_write::FORMATS.iter().map(|c| Choice::new(c.label.to_owned())),
            formats.iter().position(|f| *f == w.format),
            move |i| msg(Event::Format(formats[i])),
        );
        let on_send = Select::new(
            SENDS.iter().map(|s| Choice::new(s.1.to_owned())),
            SENDS.iter().position(|s| s.0 == w.send),
            move |i| msg(Event::SendFormat(SENDS[i].0)),
        );
        let landxml = w.send == StakeoutSendFormat::Landxml;
        let what = if landxml {
            "Aks, boykesit ve her kodun dizisi (Trimble, Leica, Topcon)."
        } else {
            "Noktalar sırayla numaralı; taşınamayan kod düşer."
        };
        let send_field = |hint: &'static str, k: usize, width: f32| {
            kentos_ui::widget::text_field(hint, &w.send_fields[k]).on_input(move |t| msg(Event::SendField(k, t))).width(width)
        };
        let mut line = row![
            words::field("Biçim", container(format).width(210), None),
            words::secondary(WRITE, w.preview.table.as_ref().map(|_| msg(Event::Write))),
            container(iced::widget::rule::vertical(1).style(style::field::hairline)).height(32),
            words::field("Alete gönder", container(on_send).width(200), None),
        ]
        .spacing(8);
        if !landxml {
            line = line.push(words::field("İlk no", send_field("1", 0, 80.0), None));
            line = line.push(words::field("İş adı", send_field("koridorun adı", 1, 140.0), None));
        }
        // Docs/adr/1031: what the heights sent are; a project that does not say it sends none.
        let heights = self
            .document
            .as_ref()
            .and_then(|d| d.model.settings().heights.as_ref())
            .filter(|h| match h.kind {
                kentos_commands::HeightKind::Unknown => false,
                kentos_commands::HeightKind::Orthometric => h.vertical().is_some(),
                kentos_commands::HeightKind::Ellipsoidal => true,
            })
            .map(|h| h.title());
        let said = match heights {
            Some(t) => label::caption(format!("Kotlar: {t}.")).style(muted),
            None => label::caption("Projenin kotlarının düşey sistemi söylenmemiş: gönderilmez (Proje ayarları › Kotlar).").style(warn),
        };
        // The buttons are the dialog's footer: resized or full screen they sit on its bottom edge (ADR 0746 eki).
        let footer =
            line.push(words::primary(SEND, w.preview.table.as_ref().map(|_| msg(Event::Send))))
                .push(container(iced::widget::column![label::caption(what).style(muted), said].spacing(2)).width(Fill))
                .push(words::secondary(CLOSE, Some(msg(Event::Close))))
                .align_y(iced::alignment::Vertical::Bottom);
        Frame::new(TITLE).scroll(body).footer(footer).width(1180.0).modal(msg(Event::Close))
    }

    pub(crate) fn stakeout_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.stakeout_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(WRITE) => Some(msg(Event::Write)),
            Control::Press(SEND) => Some(msg(Event::Send)),
            Control::Fill("Başlangıç km (m)", t) => Some(msg(Event::Field(0, t.to_owned()))),
            Control::Fill("Bitiş km (m)", t) => Some(msg(Event::Field(1, t.to_owned()))),
            Control::Fill("Aralık (m)", t) => Some(msg(Event::Field(2, t.to_owned()))),
            Control::Fill("Kodlar", t) => Some(msg(Event::Field(3, t.to_owned()))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The rows as the window lists them: the kit's sheet beside the plan, its rows filling the height.
fn rows_table(t: &StakeoutQueried) -> Element<'_, Message> {
    use crate::road_dialog::{col, num};
    let ground = t.distances == StakeoutDistances::Ground;
    let mut cols = vec![col("Km", 88.0), num("Bölge", 56.0), col("Kod", 110.0), col("Yan", 56.0), num("Öteleme (m)", 92.0)];
    if ground {
        cols.push(num("Zemin (m)", 84.0));
    }
    cols.extend([num("Y (m)", 108.0), num("X (m)", 108.0), num("Z (m)", 80.0)]);
    let rows = t
        .rows
        .iter()
        .take(SHOWN)
        .map(|r| {
            let side = match r.side {
                StakeSide::Left => "sol",
                StakeSide::Axis => "eksen",
                StakeSide::Right => "sağ",
            };
            let mut cells = vec![r.km_text.clone(), (r.region + 1).to_string(), r.code.clone(), side.to_owned(), grouped(r.offset, 3)];
            if ground {
                cells.push(r.ground_offset.map_or_else(|| "—".into(), |g| grouped(g, 3)));
            }
            cells.extend([grouped(r.x, 3), grouped(r.y, 3), grouped(r.z, 3)]);
            cells
        })
        .collect();
    crate::road_dialog::sheet_room(cols, rows, "Nokta yok")
}

/// The table's points in plan, north up, coloured by side; each station's points joined across.
struct Plan<'a> {
    table: &'a StakeoutQueried,
}

impl canvas::Program<Message> for Plan<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let rows = &self.table.rows;
        if rows.is_empty() {
            return vec![frame.into_geometry()];
        }
        let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for r in rows {
            x0 = x0.min(r.x);
            x1 = x1.max(r.x);
            y0 = y0.min(r.y);
            y1 = y1.max(r.y);
        }
        let pad = 12.0;
        let span = (x1 - x0).max(y1 - y0).max(1e-6);
        let scale = ((bounds.width - 2.0 * pad).min(bounds.height - 2.0 * pad) as f64) / span;
        let (cx, cy) = (0.5 * (x0 + x1), 0.5 * (y0 + y1));
        let at = |x: f64, y: f64| {
            Point::new(
                (bounds.width as f64 * 0.5 + (x - cx) * scale) as f32,
                (bounds.height as f64 * 0.5 - (y - cy) * scale) as f32,
            )
        };
        // Each station's points, left to right, as one line across.
        let mut start = 0;
        while start < rows.len() {
            let key = (rows[start].station, rows[start].region);
            let end = rows[start..].iter().position(|r| (r.station, r.region) != key).map_or(rows.len(), |n| start + n);
            if end - start > 1 {
                let line = Path::new(|b| {
                    b.move_to(at(rows[start].x, rows[start].y));
                    for r in &rows[start + 1..end] {
                        b.line_to(at(r.x, r.y));
                    }
                });
                frame.stroke(&line, Stroke::default().with_width(0.8).with_color(tk.border_strong()));
            }
            start = end;
        }
        for r in rows {
            let color = match r.side {
                StakeSide::Left => tk.accent,
                StakeSide::Axis => tk.text,
                StakeSide::Right => tk.warning,
            };
            frame.fill(&Path::circle(at(r.x, r.y), if r.side == StakeSide::Axis { 2.6 } else { 2.0 }), color);
        }
        vec![frame.into_geometry()]
    }
}
