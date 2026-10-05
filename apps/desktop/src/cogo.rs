//! Kayıtlı ölçüler (docs/adr/0180; the web's `app/cogo.ts` and
//! `ui/cogo/CogoCheckDialog.ts`): the recorded measurements of the drawing's
//! lines and arcs checked against the drawing by the core's rules
//! (`kentos_geometry_core::ops::cogo`, the web's through WASM): Kayıtlı
//! ölçüleri denetle's window, a row per recorded value, and Kayıtlı ölçüleri
//! çizimden yaz (`cogo.update`), which writes the selected objects' measured
//! values in one undo step “Kayıtlı ölçüleri yaz”.

use std::collections::BTreeMap;
use std::path::PathBuf;

use iced::widget::{Column, container, row, text, text_input};
use iced::{Element, Fill, Task};
use kentos_contracts::{EntitiesSetProperties, Entity, PropertiesOperation};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::display::fixed;
use kentos_geometry_core::ops::cogo::{self, Field, Finding, Item, Status};
use kentos_geometry_core::ops::compare::Attrs;
use kentos_interaction::Level;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, set};
use kentos_ui::theme::Tokens;
use kentos_ui::widget::table::{self, Table};
use kentos_ui::widget::{Dialog as Frame, Segmented, focus_ring, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::points::edit::{in_step, refusal};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const COGO_TITLE: &str = "Kayıtlı ölçüleri denetle";
const LENGTH_TOLERANCE: &str = "Uzunluk toleransı (m)";
const SEMT_TOLERANCE: &str = "Semt toleransı (cc)";
const CHECK: &str = "Denetle";
const ONLY: &str = "Yalnız farklar";
const COPY: &str = "Panoya kopyala";
const SAVE: &str = "CSV olarak kaydet…";
const UPDATE: &str = "Seçilenlere çizimden yaz";
const CLOSE: &str = "Kapat";
/// Kayıtlı ölçüleri çizimden yaz's undo step.
const STEP: &str = "Kayıtlı ölçüleri yaz";

/// A recorded value's finding in words.
pub fn status_words(s: Status) -> &'static str {
    match s {
        Status::Ok => "Uyuyor",
        Status::Differs => "Farklı",
        Status::Unreadable => "Okunamadı",
    }
}

/// A measurement's name in the list.
pub fn field_words(f: Field) -> &'static str {
    match f {
        Field::Semt => "Semt",
        Field::Length => "Uzunluk",
        Field::Radius => "Yarıçap",
        Field::Arc => "Yay uzunluğu",
    }
}

/// An item's own finding: unreadable, over the tolerance, or matching.
fn item_status(item: &Item) -> Status {
    match (item.difference, item.over) {
        (None, _) => Status::Unreadable,
        (Some(_), true) => Status::Differs,
        (Some(_), false) => Status::Ok,
    }
}

/// Which objects the window checks, as the segmented control says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    All,
    Selection,
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Scope::All => "Bütün çizim",
            Scope::Selection => "Seçim",
        })
    }
}

/// An object with recorded values, and what checking them found.
#[derive(Debug, Clone)]
pub struct CogoRow {
    slot: Slot,
    layer: String,
    kind: &'static str,
    finding: Finding,
}

/// The lines and arcs (every one, or those in `only`) that have recorded
/// values, checked in the drawing's order.
pub fn rows_of(doc: &Document, only: Option<&[Slot]>, length: f64, cc: f64) -> Vec<CogoRow> {
    doc.entities()
        .filter(|e| matches!(e, Entity::Line(_) | Entity::Arc(_)))
        .filter(|e| only.is_none_or(|ids| ids.contains(&Slot(e.base().id))))
        .filter_map(|e| {
            let attrs = Attrs(
                e.base()
                    .attrs
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            );
            let finding = cogo::check(&shape(e), &attrs, length, cc)?;
            Some(CogoRow {
                slot: Slot(e.base().id),
                layer: doc.layers().path(&e.base().layer_id),
                kind: crate::selecting::kind_title(e.kind()),
                finding,
            })
        })
        .collect()
}

/// “12 nesne denetlendi: 10 uyuyor, 1 farklı, 1 okunamadı.”
pub fn summary(rows: &[CogoRow]) -> String {
    if rows.is_empty() {
        return "Kayıtlı ölçüsü olan çizgi ya da yay yok.".to_owned();
    }
    let count = |s: Status| rows.iter().filter(|r| r.finding.status == s).count();
    let said: Vec<String> = [
        (count(Status::Ok), "uyuyor"),
        (count(Status::Differs), "farklı"),
        (count(Status::Unreadable), "okunamadı"),
    ]
    .into_iter()
    .filter(|(n, _)| *n > 0)
    .map(|(n, w)| format!("{n} {w}"))
    .collect();
    format!("{} nesne denetlendi: {}.", rows.len(), said.join(", "))
}

/// The report's header (the window's columns too).
pub const REPORT_HEADER: [&str; 7] = [
    "Durum",
    "Katman",
    "Tür",
    "Ölçü",
    "Kayıtlı",
    "Çizimden",
    "Fark",
];

/// A recorded value's words: Durum, Katman, Tür, Ölçü, Kayıtlı, Çizimden,
/// Fark (cc for the semt, mm for lengths); the decimal comma for the CSV file.
fn item_words(row: &CogoRow, i: usize, comma: bool) -> Vec<String> {
    let item = &row.finding.items[i];
    let n = |v: f64, d: usize| {
        let t = fixed(v, d);
        if comma { t.replace('.', ",") } else { t }
    };
    let semt = item.field == Field::Semt;
    vec![
        status_words(item_status(item)).to_owned(),
        row.layer.clone(),
        row.kind.to_owned(),
        field_words(item.field).to_owned(),
        item.recorded.clone(),
        n(item.measured, if semt { 4 } else { 3 }),
        match item.difference {
            None => String::new(),
            Some(d) if semt => format!("{} cc", n(d, 0)),
            Some(d) => format!("{} mm", n(d * 1000.0, 1)),
        },
    ]
}

/// The rows' items the list shows: every one, or (Yalnız farklar) those that
/// differ or cannot be read.
fn shown(rows: &[CogoRow], only_diffs: bool) -> Vec<(usize, usize)> {
    rows.iter()
        .enumerate()
        .flat_map(|(r, row)| (0..row.finding.items.len()).map(move |i| (r, i)))
        .filter(|&(r, i)| !only_diffs || item_status(&rows[r].finding.items[i]) != Status::Ok)
        .collect()
}

/// The report's rows: the header, then every recorded value.
fn report_rows(rows: &[CogoRow]) -> Vec<Vec<String>> {
    let mut out = vec![REPORT_HEADER.iter().map(|h| (*h).to_owned()).collect()];
    for row in rows {
        for i in 0..row.finding.items.len() {
            out.push(item_words(row, i, true));
        }
    }
    out
}

/// The window.
#[derive(Debug, Clone)]
pub struct Window {
    scope: Scope,
    length: String,
    cc: String,
    only_diffs: bool,
    rows: Vec<CogoRow>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Scope(Scope),
    Length(String),
    Cc(String),
    Check,
    OnlyDiffs(bool),
    /// A row of the list (its index among those shown): its object selected and shown.
    Show(usize),
    Copy,
    Save,
    Saved(Option<PathBuf>),
    Update,
    Close,
}

fn msg(event: Event) -> Message {
    Message::Cogo(event)
}

impl App {
    /// Kayıtlı ölçüleri denetle (`cogo.check`): the window, the selection
    /// checked when there is one, else the whole drawing; checked at once.
    pub(crate) fn open_cogo_check(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        self.cogo = Some(Window {
            scope: if self.selection.is_empty() {
                Scope::All
            } else {
                Scope::Selection
            },
            length: "0.01".to_owned(),
            cc: "50".to_owned(),
            only_diffs: true,
            rows: Vec::new(),
        });
        self.dialog = Some(Dialog::Cogo);
        self.run_cogo_check();
    }

    /// Checks as the window sets it and says the summary.
    fn run_cogo_check(&mut self) {
        let (Some(w), Some(doc)) = (&self.cogo, &self.document) else {
            return;
        };
        let number = |t: &str| t.trim().replace(',', ".").parse::<f64>().ok();
        let (Some(length), Some(cc)) = (number(&w.length), number(&w.cc)) else {
            self.warn("Toleranslar sıfır ya da artı birer sayı olmalı: uzunluk metre, semt cc.");
            return;
        };
        if !(length >= 0.0 && cc >= 0.0) {
            self.warn("Toleranslar sıfır ya da artı birer sayı olmalı: uzunluk metre, semt cc.");
            return;
        }
        let only = (w.scope == Scope::Selection).then(|| self.selection.ids());
        let rows = rows_of(&doc.model, only, length, cc);
        let said = summary(&rows);
        if let Some(w) = self.cogo.as_mut() {
            w.rows = rows;
        }
        self.say(Level::Info, said);
    }

    /// Kayıtlı ölçüleri çizimden yaz (`cogo.update`): the selected lines and
    /// arcs take their measured values as their recorded ones (semt with 4
    /// decimals, lengths with 3; an arc's radius and arc length too), one undo
    /// step “Kayıtlı ölçüleri yaz”; one on a locked layer is left out and
    /// said. Whether anything was written.
    pub(crate) fn cogo_update(&mut self) -> bool {
        let slots = self.selection.ids().to_vec();
        let Some(doc) = self.document.as_mut() else {
            self.output("Açık çizim yok.");
            return false;
        };
        let model = &mut doc.model;
        let edges: Vec<Entity> = slots
            .iter()
            .filter_map(|s| model.get(*s).cloned())
            .filter(|e| matches!(e, Entity::Line(_) | Entity::Arc(_)))
            .collect();
        if edges.is_empty() {
            self.warn("Kayıtlı ölçüsü çizimden yazılacak çizgi ya da yay seçin.");
            return false;
        }
        let (locked, free): (Vec<Entity>, Vec<Entity>) = edges
            .into_iter()
            .partition(|e| model.layers().is_locked(&e.base().layer_id));
        let mut written = 0;
        if !free.is_empty() {
            let refused = in_step(model, STEP, |doc| {
                for e in &free {
                    let (Some(m), Some(uid)) =
                        (cogo::measure(&shape(e)), doc.uid(Slot(e.base().id)))
                    else {
                        continue;
                    };
                    let mut attrs = BTreeMap::from([
                        (cogo::SEMT.to_owned(), Some(fixed(m.semt, 4))),
                        (cogo::LENGTH.to_owned(), Some(fixed(m.length, 3))),
                    ]);
                    if let Some(r) = m.radius {
                        attrs.insert(cogo::RADIUS.to_owned(), Some(fixed(r, 3)));
                    }
                    if let Some(a) = m.arc {
                        attrs.insert(cogo::ARC.to_owned(), Some(fixed(a, 3)));
                    }
                    let input = EntitiesSetProperties {
                        uids: vec![uid.to_string()],
                        layer_id: None,
                        color: None,
                        line_weight: None,
                        symbol: None,
                        attrs: Some(attrs),
                        label: None,
                        operation: PropertiesOperation::Attributes,
                        expected_revision: None,
                        unlink: false,
                    };
                    if let Some(error) =
                        refusal(set::execute(&mut ExecutionContext::new(doc), input))
                    {
                        return Some(error.message);
                    }
                    written += 1;
                }
                None
            });
            if let Some(why) = refused {
                self.warn(why);
                return false;
            }
        }
        if written > 0 {
            self.say(
                Level::Success,
                format!("{written} nesnenin kayıtlı ölçüleri çizimden yazıldı."),
            );
        }
        if !locked.is_empty() {
            self.warn(format!(
                "{} nesne kilitli katmanda: kayıtlı ölçüleri yazılmadı. Kilidini Katmanlar panelinden açın.",
                locked.len()
            ));
        }
        written > 0
    }

    pub(crate) fn cogo_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Scope(s) => {
                if let Some(w) = self.cogo.as_mut() {
                    w.scope = s;
                }
            }
            Event::Length(t) => {
                if let Some(w) = self.cogo.as_mut() {
                    w.length = t;
                }
            }
            Event::Cc(t) => {
                if let Some(w) = self.cogo.as_mut() {
                    w.cc = t;
                }
            }
            Event::Check => self.run_cogo_check(),
            Event::OnlyDiffs(on) => {
                if let Some(w) = self.cogo.as_mut() {
                    w.only_diffs = on;
                }
            }
            Event::Show(index) => {
                let slot = self.cogo.as_ref().and_then(|w| {
                    shown(&w.rows, w.only_diffs)
                        .get(index)
                        .map(|&(r, _)| w.rows[r].slot)
                });
                if let Some(slot) = slot {
                    self.selection.set([slot]);
                    return self.run("view.zoomSelection");
                }
            }
            Event::Copy => {
                if let Some(w) = &self.cogo
                    && !w.rows.is_empty()
                {
                    let lines = report_rows(&w.rows);
                    let n = lines.len() - 1;
                    let text = crate::layer_list::tsv(&lines);
                    self.say(
                        Level::Success,
                        format!("Kayıtlı ölçüler raporu panoya kopyalandı ({n} satır; elektronik tabloya yapıştırılabilir)."),
                    );
                    return iced::clipboard::write(text);
                }
            }
            Event::Save => {
                let name = self
                    .document
                    .as_ref()
                    .map_or_else(|| "cizim".to_owned(), |d| d.model.name().to_owned());
                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("Kayıtlı ölçüler raporunu kaydet")
                            .add_filter("CSV (.csv)", &["csv"])
                            .set_file_name(format!("{name}-kayitli-olculer.csv"))
                            .save_file()
                            .await?;
                        Some(file.path().to_path_buf())
                    },
                    |path| msg(Event::Saved(path)),
                );
            }
            Event::Saved(None) => {}
            Event::Saved(Some(path)) => {
                if let Some(w) = &self.cogo {
                    let lines = report_rows(&w.rows);
                    match std::fs::write(&path, crate::layer_list::csv(&lines)) {
                        Ok(()) => {
                            let file = path
                                .file_name()
                                .map_or_else(String::new, |f| f.to_string_lossy().into_owned());
                            self.say(
                                Level::Success,
                                format!(
                                    "Kayıtlı ölçüler raporu CSV olarak kaydedildi: {file} ({} satır).",
                                    lines.len() - 1
                                ),
                            );
                        }
                        Err(e) => self.warn(format!(
                            "Rapor kaydedilemedi ({e}); başka bir klasör seçip yeniden deneyin."
                        )),
                    }
                }
            }
            Event::Update => {
                if self.cogo_update() {
                    self.run_cogo_check();
                }
            }
            Event::Close => {
                self.cogo = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn cogo_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.cogo else {
            return text("").into();
        };
        let number = |value: &str, on: fn(String) -> Event| {
            focus_ring(
                text_input("", value)
                    .on_input(move |t| msg(on(t)))
                    .padding([5, 8])
                    .width(110)
                    .style(style::field::input),
            )
        };
        let controls = row![
            words::field(
                "Kapsam",
                Segmented::new([Scope::All, Scope::Selection], w.scope, |s| msg(
                    Event::Scope(s)
                )),
                None
            ),
            words::field(LENGTH_TOLERANCE, number(&w.length, Event::Length), None),
            words::field(SEMT_TOLERANCE, number(&w.cc, Event::Cc), None),
            Column::new().push(words::primary(CHECK, Some(msg(Event::Check)))),
            words::field(
                "Liste",
                words::check(
                    w.only_diffs,
                    ONLY,
                    Some(msg(Event::OnlyDiffs(!w.only_diffs)))
                ),
                None
            ),
        ]
        .spacing(18)
        .align_y(iced::Alignment::End);
        let kind = if w.rows.is_empty() {
            Kind::Info
        } else if w.rows.iter().any(|r| r.finding.status != Status::Ok) {
            Kind::Warn
        } else {
            Kind::Ok
        };
        let said = words::summary(vec![words::text_line(kind, summary(&w.rows))]);
        let list = shown(&w.rows, w.only_diffs);
        let cells: Vec<(Status, Vec<String>)> = list
            .iter()
            .map(|&(r, i)| {
                let row = &w.rows[r];
                (
                    item_status(&row.finding.items[i]),
                    item_words(row, i, false),
                )
            })
            .collect();
        let widths: [f32; 7] = [90.0, 150.0, 70.0, 100.0, 110.0, 110.0, 90.0];
        let columns = REPORT_HEADER
            .iter()
            .zip(widths)
            .map(|(h, wd)| table::Column::new(*h).width(wd));
        let body = Table::new(columns)
            .virtualized(cells.len(), move |i| {
                let (status, words) = &cells[i];
                let status = *status;
                let row_cells = words.iter().enumerate().map(move |(c, t)| {
                    let cell = label::caption(t.clone());
                    if c == 0 {
                        cell.style(move |theme: &iced::Theme| {
                            let tk = Tokens::of(theme);
                            iced::widget::text::Style {
                                color: Some(match status {
                                    Status::Ok => tk.success,
                                    Status::Differs => tk.danger,
                                    Status::Unreadable => tk.warning,
                                }),
                            }
                        })
                        .into()
                    } else {
                        cell.into()
                    }
                });
                table::Row::new(row_cells).on_press(msg(Event::Show(i)))
            })
            .horizontal()
            .height(240.0);
        let table = container(body)
            .style(style::container::field_box)
            .width(Fill);
        let any = !w.rows.is_empty();
        let content = Column::new()
            .spacing(12)
            .push(controls)
            .push(said)
            .push(table);
        overlay::modal(
            Frame::new(COGO_TITLE)
                .push(content)
                .action(words::secondary(COPY, any.then(|| msg(Event::Copy))))
                .action(words::secondary(SAVE, any.then(|| msg(Event::Save))))
                .action(words::secondary(UPDATE, Some(msg(Event::Update))))
                .action(words::primary(CLOSE, Some(msg(Event::Close))))
                .width(860.0),
            msg(Event::Close),
        )
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn cogo_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let Some(w) = &self.cogo else {
            return Err(format!("{COGO_TITLE} penceresi açık değil"));
        };
        let any = !w.rows.is_empty();
        Ok(match control {
            Control::Fill(LENGTH_TOLERANCE, t) => Some(msg(Event::Length(t.to_owned()))),
            Control::Fill(SEMT_TOLERANCE, t) => Some(msg(Event::Cc(t.to_owned()))),
            Control::Check(ONLY, on) => (w.only_diffs != on).then(|| msg(Event::OnlyDiffs(on))),
            Control::Press(CHECK) => Some(msg(Event::Check)),
            Control::Press(COPY) => any.then(|| msg(Event::Copy)),
            Control::Press(UPDATE) => Some(msg(Event::Update)),
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(words) if words == Scope::All.to_string() => {
                Some(msg(Event::Scope(Scope::All)))
            }
            Control::Press(words) if words == Scope::Selection.to_string() => {
                Some(msg(Event::Scope(Scope::Selection)))
            }
            other => return Err(format!("“{COGO_TITLE}” penceresinde {other} yok")),
        })
    }
}
