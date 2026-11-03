//! Aplikasyon cetveli (plan/road.md Y10, docs/adr/0908, `stakeout.table`): a corridor's coded section points as Civil
//! 3D's corridor points report and OpenRoads' cross section grade book list them: km, code, side, offset, Y, X, Z, on
//! the grid or with the offsets also as ground lengths. The table is read as it is typed (`road.stakeout.query`), its
//! points drawn in plan, and written as a workbook; nothing in the drawing changes.
//!
//! ```text
//! ┌ Aplikasyon cetveli ────────────────────────────────────────────────────────────────────────────┐
//! │ Koridor [▾]  Başlangıç km [ ]  Bitiş km [ ]  Aralık (m) [ ]  Kodlar [axis, hinge, …]  Öteleme [▾] │
//! │ method · n km · m nokta · warnings                                                               │
//! │ ┌ points in plan ┐  Km  Bölge  Kod  Yan  Öteleme  Zemin  Y  X  Z                                  │
//! │ └────────────────┘  …                                                                            │
//! │ Biçim [▾] [Cetveli yaz]                                                                [Kapat]   │
//! └──────────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::canvas::{self, Path, Stroke};
use iced::widget::{Column, column, container, row, scrollable, space};
use iced::{Element, Fill, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{CommandWarning, StakeSide, StakeoutDistances, StakeoutQueried, StakeoutQuery, WorkbookFormat};
use kentos_native_application::road_stakeout;
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
}

#[derive(Debug, Clone)]
pub enum Event {
    Corridor(String),
    /// 0 the start, 1 the end, 2 the step, 3 the codes.
    Field(usize, String),
    Distances(StakeoutDistances),
    Format(WorkbookFormat),
    Write,
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
                return crate::table_write::save(vec![road_stakeout::sheet(table)], w.format, name);
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
        let mut body = Column::new().spacing(10);
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
                for wn in &w.preview.warnings {
                    body = body.push(label::caption(wn.message.clone()).style(warn));
                }
                let plan = container(iced::widget::canvas(Plan { table: t }).width(300).height(typography::scaled(300.0)))
                    .style(style::container::field_box);
                body = body.push(row![plan, container(rows_table(t)).width(Fill)].spacing(12));
            }
            _ => {}
        }
        let formats: Vec<WorkbookFormat> = crate::table_write::FORMATS.iter().map(|c| c.format).collect();
        let format = Select::new(
            crate::table_write::FORMATS.iter().map(|c| Choice::new(c.label.to_owned())),
            formats.iter().position(|f| *f == w.format),
            move |i| msg(Event::Format(formats[i])),
        );
        body = body.push(
            row![
                words::field("Biçim", container(format).width(240), None),
                words::secondary(WRITE, w.preview.table.as_ref().map(|_| msg(Event::Write))),
                space::horizontal().width(Fill),
                label::caption("Cetvel çizimi değiştirmez.").style(muted),
                words::secondary(CLOSE, Some(msg(Event::Close))),
            ]
            .spacing(8)
            .align_y(iced::alignment::Vertical::Bottom),
        );
        Frame::new(TITLE).push(body).width(1180.0).modal(msg(Event::Close))
    }

    pub(crate) fn stakeout_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.stakeout_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(WRITE) => Some(msg(Event::Write)),
            Control::Fill("Başlangıç km (m)", t) => Some(msg(Event::Field(0, t.to_owned()))),
            Control::Fill("Bitiş km (m)", t) => Some(msg(Event::Field(1, t.to_owned()))),
            Control::Fill("Aralık (m)", t) => Some(msg(Event::Field(2, t.to_owned()))),
            Control::Fill("Kodlar", t) => Some(msg(Event::Field(3, t.to_owned()))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The rows as the window lists them.
fn rows_table(t: &StakeoutQueried) -> Element<'_, Message> {
    let muted = |th: &Theme| iced::widget::text::Style { color: Some(Tokens::of(th).muted) };
    let ground = t.distances == StakeoutDistances::Ground;
    let mut head: Vec<(&str, f32)> = vec![("Km", 78.0), ("Bölge", 40.0), ("Kod", 110.0), ("Yan", 44.0), ("Öteleme", 66.0)];
    if ground {
        head.push(("Zemin", 66.0));
    }
    head.extend([("Y", 96.0), ("X", 96.0), ("Z", 66.0)]);
    let widths: Vec<f32> = head.iter().map(|h| h.1).collect();
    let header = head.iter().fold(row![].spacing(6), |r, (h, wd)| r.push(label::caption(*h).style(muted).width(*wd)));
    let mut rows = Column::new().spacing(2);
    for r in t.rows.iter().take(SHOWN) {
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
        rows = rows.push(
            cells.into_iter().zip(widths.iter().copied()).fold(row![].spacing(6), |row, (c, wd)| row.push(label::caption(c).width(wd))),
        );
    }
    column![header, scrollable(rows).height(typography::scaled(278.0))].spacing(4).padding([6, 8]).into()
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
