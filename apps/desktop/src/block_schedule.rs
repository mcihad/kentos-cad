//! Blok çizelgesi (docs/adr/0317 §3): a block's inserts (or every block's) counted, found by an attribute and
//! scheduled, from `cad.blocks.query`. The counts stand at the top (in the drawing, by layer, through other blocks),
//! a condition below them (a tag, how it is compared, a value), then the table of the inserts that match: block,
//! layer, X, Y, turn and the attributes' values. Seç selects them, Panoya kopyala copies the table as tab-separated
//! text, Dışa aktar ▾ writes it as a workbook (table_write.rs).

use iced::widget::{Column, Row, column, container, row, scrollable, text, text_input};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    AttributeFilter, AttributeOp, BlockId, BlockReport, BlocksQueried, BlocksQuery, CellKind,
    CommandResult, ScheduleCell, WorkbookColumn, WorkbookFormat, WorkbookSheet,
};
use kentos_domain::Slot;
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, blocks_query};
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::{Dialog as Frame, Menu, focus_ring, overlay};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};

/// The window's title, which a trace names it by.
pub const SCHEDULE_TITLE: &str = "Blok çizelgesi";
/// The most rows the window shows (the file and the clipboard take them all).
const SHOWN: usize = 500;

/// How a condition compares, as the window names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Op(AttributeOp);

impl std::fmt::Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.0 {
            AttributeOp::Eq => "=",
            AttributeOp::Ne => "≠",
            AttributeOp::Contains => "İçerir",
            AttributeOp::Empty => "Boş",
            AttributeOp::NotEmpty => "Dolu",
            AttributeOp::Lt => "<",
            AttributeOp::Gt => ">",
        })
    }
}

const OPS: [Op; 7] = [
    Op(AttributeOp::Contains),
    Op(AttributeOp::Eq),
    Op(AttributeOp::Ne),
    Op(AttributeOp::Empty),
    Op(AttributeOp::NotEmpty),
    Op(AttributeOp::Lt),
    Op(AttributeOp::Gt),
];

#[derive(Debug, Clone)]
pub struct Window {
    block: Option<BlockId>,
    name: String,
    tag: String,
    op: Op,
    value: String,
    counts: Result<BlocksQueried, String>,
    found: Result<BlocksQueried, String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Tag(String),
    Op(Op),
    Value(String),
    Find,
    Select,
    Copy,
    Export(WorkbookFormat),
    Close,
}

fn msg(event: Event) -> Message {
    Message::BlockSchedule(event)
}

/// A cell as the window shows it: a multiline value on one line, its lines by “ / ” (copied and saved as it is).
fn cell_text(c: &ScheduleCell) -> String {
    match c {
        ScheduleCell::Text(t) => t.lines().collect::<Vec<_>>().join(" / "),
        ScheduleCell::Number(n) => kentos_geometry_core::display::fixed(*n, 3),
    }
}

impl App {
    /// Say ve çizelgele… on a Bloklar row (a block) or Ekle › Blok's Blok çizelgesi (every block).
    pub(crate) fn open_block_schedule(&mut self, block: Option<BlockId>) {
        let Some(doc) = &self.document else { return };
        let name = block
            .and_then(|id| doc.model.block(id))
            .map_or_else(|| "Bütün bloklar".to_owned(), |b| format!("“{}”", b.name));
        self.block_schedule = Some(Window {
            block,
            name,
            tag: String::new(),
            op: OPS[0],
            value: String::new(),
            counts: Err(String::new()),
            found: Err(String::new()),
        });
        self.dialog = Some(Dialog::BlockSchedule);
        self.block_schedule_run();
    }

    /// The counts and the table asked again of the command.
    fn block_schedule_run(&mut self) {
        let (Some(w), Some(doc)) = (&self.block_schedule, &self.document) else {
            return;
        };
        let blocks = w.block.map(|b| vec![b]);
        let filters = (!w.tag.trim().is_empty()).then(|| {
            vec![AttributeFilter {
                tag: w.tag.trim().to_owned(),
                op: w.op.0,
                value: (!matches!(w.op.0, AttributeOp::Empty | AttributeOp::NotEmpty))
                    .then(|| w.value.trim().to_owned()),
            }]
        });
        let ask = |report, filters| {
            let input = BlocksQuery {
                report,
                blocks: blocks.clone(),
                layers: None,
                filters,
                columns: None,
            };
            match blocks_query::plan(
                &ExecutionContext::new(&mut doc.model.reading_copy()),
                &input,
            ) {
                CommandResult::Completed { output, .. } => Ok(output),
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => Err(error.message),
                _ => Err(String::new()),
            }
        };
        let counts = ask(BlockReport::Count, None);
        let found = ask(BlockReport::Schedule, filters);
        if let Some(w) = self.block_schedule.as_mut() {
            w.counts = counts;
            w.found = found;
        }
    }

    pub(crate) fn block_schedule_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.block_schedule.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Tag(t) => w.tag = t,
            Event::Op(op) => {
                w.op = op;
                self.block_schedule_run();
            }
            Event::Value(t) => w.value = t,
            Event::Find => self.block_schedule_run(),
            Event::Select => {
                let Ok(found) = &w.found else {
                    return Task::none();
                };
                let Some(doc) = &self.document else {
                    return Task::none();
                };
                let slots: Vec<Slot> = found
                    .uids
                    .iter()
                    .filter_map(|u| kentos_domain::Uuid::parse_str(u).ok())
                    .filter_map(|u| doc.model.slot_of(u))
                    .collect();
                let n = slots.len();
                self.selection.set(slots);
                self.block_schedule = None;
                self.dialog = None;
                self.say(Level::Success, format!("{n} yerleştirme seçildi."));
            }
            Event::Copy => {
                let Ok(found) = &w.found else {
                    return Task::none();
                };
                let mut lines = vec![found.columns.join("\t")];
                lines.extend(
                    found
                        .rows
                        .iter()
                        .map(|r| r.iter().map(cell_text).collect::<Vec<_>>().join("\t")),
                );
                let n = found.rows.len();
                self.say(
                    Level::Info,
                    format!("Çizelgenin {n} satırı panoya kopyalandı."),
                );
                return iced::clipboard::write(lines.join("\n"));
            }
            Event::Export(format) => {
                let Ok(found) = &w.found else {
                    return Task::none();
                };
                let columns = found
                    .columns
                    .iter()
                    .enumerate()
                    .map(|(i, c)| WorkbookColumn {
                        name: c.clone(),
                        kind: if (2..5).contains(&i) {
                            CellKind::Number
                        } else {
                            CellKind::Text
                        },
                        decimals: None,
                    })
                    .collect();
                let rows = found
                    .rows
                    .iter()
                    .map(|r| {
                        r.iter()
                            .map(|c| match c {
                                ScheduleCell::Number(n) => format!("{n}"),
                                ScheduleCell::Text(t) => t.clone(),
                            })
                            .collect()
                    })
                    .collect();
                let sheet = WorkbookSheet {
                    name: "Blok çizelgesi".to_owned(),
                    columns,
                    rows,
                };
                return crate::table_write::save(vec![sheet], format, "blok-cizelgesi".to_owned());
            }
            Event::Close => {
                self.block_schedule = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn block_schedule_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.block_schedule else {
            return text("").into();
        };
        // The counts: each block's in the drawing, by layer, and through other blocks.
        let mut counts = Vec::new();
        match &w.counts {
            Ok(q) => {
                for c in &q.counts {
                    let layers = c
                        .layers
                        .iter()
                        .map(|(l, n)| {
                            let name = self
                                .document
                                .as_ref()
                                .and_then(|d| d.model.layers().get(l).map(|n| n.name.clone()))
                                .unwrap_or_else(|| l.clone());
                            format!("{name} {n}")
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    let mut line = format!("{}: çizimde {}", c.name, c.count);
                    if !layers.is_empty() {
                        line.push_str(&format!(" ({layers})"));
                    }
                    if c.nested > 0 {
                        line.push_str(&format!(", başka blokların içinden {}", c.nested));
                    }
                    counts.push(words::text_line(Kind::Info, line));
                }
            }
            Err(why) if !why.is_empty() => counts.push(words::text_line(Kind::Error, why.clone())),
            Err(_) => {}
        }
        let condition = row![
            words::field(
                "Etiket",
                focus_ring(
                    text_input("NUMARA", &w.tag)
                        .on_input(|t| msg(Event::Tag(t)))
                        .on_submit(msg(Event::Find))
                        .padding([5, 8])
                        .width(130)
                        .style(style::field::input),
                ),
                None
            ),
            words::field(
                "Karşılaştır",
                Segmented::new(OPS, w.op, |o| msg(Event::Op(o))),
                None
            ),
            words::field(
                "Değer",
                focus_ring(
                    text_input("K-", &w.value)
                        .on_input(|t| msg(Event::Value(t)))
                        .on_submit(msg(Event::Find))
                        .padding([5, 8])
                        .width(120)
                        .style(style::field::input),
                ),
                None
            ),
            container(words::secondary("Bul", Some(msg(Event::Find)))).padding(iced::Padding {
                top: 18.0,
                right: 0.0,
                bottom: 0.0,
                left: 0.0
            }),
        ]
        .spacing(10)
        .align_y(iced::Bottom);
        let table: Element<'_, Message> = match &w.found {
            Ok(q) if !q.rows.is_empty() => {
                let width = |i: usize| if i < 2 { 120.0 } else { 90.0 };
                let head = q
                    .columns
                    .iter()
                    .enumerate()
                    .fold(Row::new().spacing(6), |r, (i, c)| {
                        r.push(
                            text(c.clone())
                                .font(typography::ui_strong())
                                .size(typography::caption())
                                .width(width(i)),
                        )
                    });
                let mut body = Column::new().spacing(2);
                for r in q.rows.iter().take(SHOWN) {
                    body = body.push(r.iter().enumerate().fold(
                        Row::new().spacing(6),
                        |row, (i, c)| {
                            let t = text(cell_text(c))
                                .size(typography::caption())
                                .wrapping(text::Wrapping::None)
                                .width(width(i));
                            row.push(if matches!(c, ScheduleCell::Number(_)) {
                                t.align_x(iced::alignment::Horizontal::Right)
                            } else {
                                t
                            })
                        },
                    ));
                }
                column![head, scrollable(scrollable(body).horizontal()).height(260)]
                    .spacing(6)
                    .into()
            }
            Ok(_) => words::text_line(Kind::Info, "Koşula uyan yerleştirme yok.".to_owned()),
            Err(why) if !why.is_empty() => words::text_line(Kind::Error, why.clone()),
            Err(_) => text("").into(),
        };
        let found = w.found.as_ref().map_or(0, |q| q.rows.len());
        let mut lines = vec![words::text_line(
            Kind::Info,
            format!(
                "{}: sayılar, koşula uyan yerleştirmeler ve öznitelik değerleri.",
                w.name
            ),
        )];
        lines.extend(counts);
        let body = Column::new()
            .spacing(12)
            .push(words::summary(lines))
            .push(condition)
            .push(table)
            .push(words::text_line(
                Kind::Ok,
                if found > SHOWN {
                    format!("{found} yerleştirme; ilk {SHOWN} satır gösteriliyor, dosya ve pano hepsini alır.")
                } else {
                    format!("{found} yerleştirme.")
                },
            ));
        // Dışa aktar ▾ (table_write.rs), as Öznitelik tablosu's.
        let face = container(
            row![
                kentos_ui::icon::icon(crate::icons::from_web(Some("export"))).size(14.0),
                kentos_ui::label::body("Dışa aktar"),
                kentos_ui::icon::icon(kentos_ui::icon::Icon::ChevronDown)
                    .size(12.0)
                    .tone(kentos_ui::icon::Tone::Muted),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([4, 10])
        .style(style::container::field_box);
        let export: Element<'_, Message> = if found > 0 {
            kentos_ui::widget::MenuButton::new(face, || {
                crate::table_write::items(Menu::new(), |f| msg(Event::Export(f)))
            })
            .into()
        } else {
            face.into()
        };
        let body = body.push(row![export].width(Fill));
        overlay::modal(
            Frame::new(SCHEDULE_TITLE)
                .push(body)
                .action(words::secondary("Kapat", Some(msg(Event::Close))))
                .action(words::secondary(
                    "Panoya kopyala",
                    (found > 0).then(|| msg(Event::Copy)),
                ))
                .action(words::primary(
                    "Seç",
                    (found > 0).then(|| msg(Event::Select)),
                ))
                .width(860.0),
            msg(Event::Close),
        )
    }
}
