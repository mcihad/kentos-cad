//! Alan hesapla and Yapıştır of the attribute table (docs/adr/0507, plan/gis.md CBS-20): one window over
//! `cad.fields.write`. Alan hesapla takes a field, an expression (ε opens the İfade oluşturucu) and the objects it
//! goes to, each count said (Seçililer, Gösterilenler, Tümü); Yapıştır reads the clipboard's rows (tab between cells,
//! a first row of field names taken as the header) onto the rows shown from the first selected one. Önizle runs the
//! command's plan: how many values change, stay, are left empty and do not fit, the first changes and problems by the
//! row's place. Yaz writes all or nothing; Uyanları yaz writes the fitting ones.

use std::collections::BTreeMap;

use iced::widget::{Column, row};
use iced::{Element, Length, Task};
use kentos_commands::{
    CommandResult, EmptyResult, FieldRow, FieldsWrite, FieldsWriteOp, FieldsWritten, WritePolicy,
};
use kentos_domain::Slot;
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, fields_write};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::{Dialog, overlay};

use super::{Event, Rows};
use crate::app::{App, Dialog as Asking, Message};

pub const TITLE_CALC: &str = "Alan hesapla";
pub const TITLE_PASTE: &str = "Yapıştır";

/// The objects an Alan hesapla goes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Selected,
    Shown,
    All,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    Calculate,
    /// The rows read from the clipboard, and the lines that fell past the table's last row.
    Paste {
        rows: Vec<FieldRow>,
        beyond: usize,
    },
}

/// The window while it is open.
#[derive(Clone, Debug)]
pub(crate) struct WriteWindow {
    pub layer: String,
    pub mode: Mode,
    pub field: String,
    pub expression: String,
    pub scope: Scope,
    pub clear: bool,
    /// The plan's answer, or why there is none.
    pub plan: Option<Result<FieldsWritten, String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WriteEvent {
    /// Alan hesapla… from the bar.
    Open,
    /// Yapıştır from the bar: the clipboard read.
    Paste,
    Pasted(Option<String>),
    Field(String),
    Expression(String),
    Scope(Scope),
    Clear(bool),
    Preview,
    /// ε: the İfade oluşturucu on the expression.
    Builder,
    Run(WritePolicy),
    Close,
}

fn msg(e: WriteEvent) -> Message {
    Message::Features(Event::Write(e))
}

/// The clipboard's text as rows of cells: tab between them, a line each (an empty last line dropped).
pub(crate) fn cells(text: &str) -> Vec<Vec<String>> {
    let mut lines: Vec<&str> = text
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .collect();
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines
        .iter()
        .map(|l| l.split('\t').map(str::to_owned).collect())
        .collect()
}

/// The pasted rows onto the table: the field columns in order (or by a header naming them), the rows shown from
/// `start`; how many lines fell past the last row.
pub(crate) fn paste_rows(
    grid: &[Vec<String>],
    fields: &[String],
    objects: &[String],
    start: usize,
) -> (Vec<FieldRow>, usize) {
    let fold = |s: &str| s.trim().to_lowercase();
    let header = grid.first().is_some_and(|first| {
        !first.is_empty()
            && first
                .iter()
                .all(|c| fields.iter().any(|f| fold(f) == fold(c)))
    });
    let names: Vec<Option<String>> = if header {
        grid[0]
            .iter()
            .map(|c| fields.iter().find(|f| fold(f) == fold(c)).cloned())
            .collect()
    } else {
        fields.iter().cloned().map(Some).collect()
    };
    let data = if header { &grid[1..] } else { grid };
    let mut rows = Vec::new();
    let mut beyond = 0;
    for (i, line) in data.iter().enumerate() {
        let Some(uid) = objects.get(start + i) else {
            beyond += 1;
            continue;
        };
        let values: BTreeMap<String, Option<String>> = line
            .iter()
            .zip(&names)
            .filter_map(|(cell, name)| {
                let name = name.clone()?;
                let v = cell.trim();
                Some((name, (!v.is_empty()).then(|| v.to_owned())))
            })
            .collect();
        if !values.is_empty() {
            rows.push(FieldRow {
                uid: uid.clone(),
                values,
            });
        }
    }
    (rows, beyond)
}

impl App {
    /// The table's field columns and the rows shown (persistent ids), from the first selected row.
    fn paste_target(&self, rows: &Rows) -> (Vec<String>, Vec<String>, usize) {
        let Some(doc) = &self.document else {
            return (Vec::new(), Vec::new(), 0);
        };
        // The columns the table shows, as a copy of it writes them (docs/adr/0546 §1).
        let fields: Vec<String> = rows
            .visible
            .iter()
            .filter_map(|&j| rows.table.columns[j].key.clone())
            .collect();
        let objects: Vec<String> = rows
            .shown
            .iter()
            .filter_map(|&i| doc.model.uid(rows.slots[i]).map(|u| u.to_string()))
            .collect();
        let start = rows
            .shown
            .iter()
            .position(|&i| self.selection.ids().contains(&rows.slots[i]))
            .unwrap_or(0);
        (fields, objects, start)
    }

    fn write_input(&self, w: &WriteWindow, policy: WritePolicy) -> FieldsWrite {
        let write = match &w.mode {
            Mode::Calculate => {
                let uids = match w.scope {
                    Scope::All => Vec::new(),
                    Scope::Selected | Scope::Shown => self.scope_objects(w),
                };
                FieldsWriteOp::Calculate {
                    field: w.field.trim().to_owned(),
                    expression: w.expression.clone(),
                    uids,
                    empty: if w.clear {
                        EmptyResult::Clear
                    } else {
                        EmptyResult::Keep
                    },
                }
            }
            Mode::Paste { rows, .. } => FieldsWriteOp::Values { rows: rows.clone() },
        };
        FieldsWrite {
            layer: w.layer.clone(),
            write,
            policy,
        }
    }

    /// The persistent ids Seçililer or Gösterilenler stand for (a selection off the layer counts none).
    fn scope_objects(&self, w: &WriteWindow) -> Vec<String> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let slots: Vec<Slot> = match w.scope {
            Scope::Selected => self
                .selection
                .ids()
                .iter()
                .copied()
                .filter(|s| {
                    doc.model
                        .get(*s)
                        .is_some_and(|e| e.base().layer_id == w.layer)
                })
                .collect(),
            _ => self.shown_features(),
        };
        slots
            .iter()
            .filter_map(|s| doc.model.uid(*s).map(|u| u.to_string()))
            .collect()
    }

    /// How many objects each scope stands for.
    fn scope_counts(&self, w: &WriteWindow) -> [usize; 3] {
        let all = self
            .document
            .as_ref()
            .map_or(0, |d| d.model.by_layer(&w.layer).count());
        let mut sel = w.clone();
        sel.scope = Scope::Selected;
        let mut shown = w.clone();
        shown.scope = Scope::Shown;
        [
            self.scope_objects(&sel).len(),
            self.scope_objects(&shown).len(),
            all,
        ]
    }

    fn write_preview(&mut self) {
        let Some(w) = self.features.write.clone() else {
            return;
        };
        let input = self.write_input(&w, WritePolicy::All);
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let plan = match fields_write::plan(&ExecutionContext::new(&mut doc.model), &input) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => Err(error.message),
            _ => Err("Önizlenemedi.".into()),
        };
        if let Some(w) = self.features.write.as_mut() {
            w.plan = Some(plan);
        }
    }

    pub(crate) fn features_write_event(&mut self, e: WriteEvent) -> Task<Message> {
        match e {
            WriteEvent::Open => {
                let Some(doc) = &self.document else {
                    return Task::none();
                };
                let rows = self.feature_rows(doc);
                let Some(layer) = rows.layer.clone() else {
                    return Task::none();
                };
                let field = rows
                    .table
                    .columns
                    .iter()
                    .find_map(|c| c.key.clone())
                    .unwrap_or_default();
                let scope = if self.selection.is_empty() {
                    Scope::Shown
                } else {
                    Scope::Selected
                };
                self.features.write = Some(WriteWindow {
                    layer,
                    mode: Mode::Calculate,
                    field,
                    expression: String::new(),
                    scope,
                    clear: false,
                    plan: None,
                });
                self.dialog = Some(Asking::FieldsWrite);
            }
            WriteEvent::Paste => {
                return iced::clipboard::read().map(|t| msg(WriteEvent::Pasted(t)));
            }
            WriteEvent::Pasted(None) => self.warn(
                "Panoda metin yok: elektronik tablodan hücreleri kopyalayıp yeniden deneyin.",
            ),
            WriteEvent::Pasted(Some(text)) => {
                let Some(doc) = &self.document else {
                    return Task::none();
                };
                let rows = self.feature_rows(doc);
                let Some(layer) = rows.layer.clone() else {
                    return Task::none();
                };
                let (fields, objects, start) = self.paste_target(&rows);
                let (pasted, beyond) = paste_rows(&cells(&text), &fields, &objects, start);
                if pasted.is_empty() {
                    self.warn("Panodaki metinden tabloya yazılacak hücre çıkmadı: hücreler sekmeyle ayrılmış olmalı.");
                    return Task::none();
                }
                self.features.write = Some(WriteWindow {
                    layer,
                    mode: Mode::Paste {
                        rows: pasted,
                        beyond,
                    },
                    field: String::new(),
                    expression: String::new(),
                    scope: Scope::Shown,
                    clear: false,
                    plan: None,
                });
                self.dialog = Some(Asking::FieldsWrite);
                self.write_preview();
            }
            WriteEvent::Field(f) => {
                if let Some(w) = self.features.write.as_mut() {
                    w.field = f;
                    w.plan = None;
                }
            }
            WriteEvent::Expression(t) => {
                if let Some(w) = self.features.write.as_mut() {
                    w.expression = t;
                    w.plan = None;
                }
            }
            WriteEvent::Scope(s) => {
                if let Some(w) = self.features.write.as_mut() {
                    w.scope = s;
                    w.plan = None;
                }
            }
            WriteEvent::Clear(c) => {
                if let Some(w) = self.features.write.as_mut() {
                    w.clear = c;
                    w.plan = None;
                }
            }
            WriteEvent::Preview => self.write_preview(),
            WriteEvent::Builder => {
                let Some(w) = &self.features.write else {
                    return Task::none();
                };
                let (layer, value) = (w.layer.clone(), w.expression.clone());
                return self.open_builder_for_fields_write(&layer, &value);
            }
            WriteEvent::Run(policy) => {
                let Some(w) = self.features.write.clone() else {
                    return Task::none();
                };
                let input = self.write_input(&w, policy);
                let step = fields_write::step_name(&input);
                let Some(doc) = self.document.as_mut() else {
                    return Task::none();
                };
                match fields_write::execute(&mut ExecutionContext::new(&mut doc.model), input) {
                    CommandResult::Completed { output, warnings } => {
                        self.features.write = None;
                        self.dialog = None;
                        self.features.changed();
                        let skipped = if output.invalid > 0 {
                            format!(
                                "; {} nesnenin değeri kurallara uymadığı için yazılmadı",
                                output.invalid
                            )
                        } else {
                            String::new()
                        };
                        self.announce(
                            if warnings.is_empty() { Level::Success } else { Level::Warn },
                            format!("{step}: {} nesnede değer yazıldı{skipped}. Geri almak için Ctrl+Z.", output.changed),
                        );
                    }
                    CommandResult::Failed { error }
                    | CommandResult::Conflict { error }
                    | CommandResult::NeedsInput { error } => {
                        if let Some(w) = self.features.write.as_mut() {
                            w.plan = Some(Err(error.message));
                        }
                    }
                    _ => {}
                }
            }
            WriteEvent::Close => {
                self.features.write = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    /// The row's place in the table (1-based, the drawing's order on the layer) of an object by its id.
    fn row_place(&self, layer: &str, uid: &str) -> String {
        let Some(doc) = &self.document else {
            return uid.to_owned();
        };
        doc.model
            .by_layer_with_uids(layer)
            .position(|(u, _)| u.to_string() == uid)
            .map_or_else(|| uid.to_owned(), |i| format!("{}. satır", i + 1))
    }

    pub(crate) fn features_write_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.features.write else {
            return iced::widget::text("").into();
        };
        let mut body = Column::new().spacing(12);
        let field = |placeholder: &'static str, value: &str, on: fn(String) -> WriteEvent| {
            kentos_ui::widget::text_field(placeholder, value).on_input(move |t| msg(on(t)))
        };
        let title = match &w.mode {
            Mode::Calculate => {
                let names: Vec<String> = self
                    .document
                    .as_ref()
                    .and_then(|d| d.model.layers().get(&w.layer))
                    .map(|n| n.fields.iter().map(|f| f.name.clone()).collect())
                    .unwrap_or_default();
                let hint = if names.is_empty() {
                    "Katmanın alanı yok: öznitelik adı yazın; yoksa eklenir.".to_owned()
                } else {
                    format!("Katmanın alanları: {}.", names.join(", "))
                };
                body = body.push(crate::exchange::words::field(
                    "Alan",
                    field("Alanın adı", &w.field, WriteEvent::Field),
                    Some(hint),
                ));
                let expression = row![
                    container_fill(field(
                        "yuvarla($alan, 2)",
                        &w.expression,
                        WriteEvent::Expression
                    )),
                    kentos_ui::widget::tip(
                        iced::widget::button(
                            kentos_ui::icon::icon(crate::icons::from_web(Some("expression")))
                                .size(16.0)
                        )
                        .style(style::button::ghost)
                        .padding(kentos_ui::theme::metrics::icon_small(16.0))
                        .on_press(msg(WriteEvent::Builder)),
                        kentos_ui::widget::Tip::new("İfade oluşturucu…"),
                        iced::widget::tooltip::Position::Top,
                    ),
                ]
                .spacing(4)
                .align_y(iced::Alignment::Center);
                body = body.push(crate::exchange::words::field(
                    "İfade",
                    expression,
                    Some("Her nesne için hesaplanır; alanlar adlarıyla, geometri $alan, $uzunluk ile; $sıra nesnenin sırası.".into()),
                ));
                let [sel, shown, all] = self.scope_counts(w);
                let mut scopes = Column::new().spacing(6);
                for (s, words, n) in [
                    (Scope::Selected, "Seçililer", sel),
                    (Scope::Shown, "Tabloda gösterilenler", shown),
                    (Scope::All, "Katmanın bütün nesneleri", all),
                ] {
                    scopes = scopes.push(crate::exchange::words::check(
                        w.scope == s,
                        format!("{words} ({n})"),
                        (n > 0 || s == Scope::All).then(|| msg(WriteEvent::Scope(s))),
                    ));
                }
                body = body.push(crate::exchange::words::field("Nesneler", scopes, None));
                body = body.push(crate::exchange::words::check(
                    w.clear,
                    "Sonuç boşsa alanın değerini kaldır (yoksa eski değer kalır)",
                    Some(msg(WriteEvent::Clear(!w.clear))),
                ));
                TITLE_CALC
            }
            Mode::Paste { rows, beyond } => {
                let fields: std::collections::BTreeSet<&String> =
                    rows.iter().flat_map(|r| r.values.keys()).collect();
                let mut said = format!(
                    "Panodan {} satır, {} alan: {}.",
                    rows.len(),
                    fields.len(),
                    fields
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                if *beyond > 0 {
                    said.push_str(&format!(
                        " {beyond} satır tablonun sonunu aştığı için yapıştırılmayacak."
                    ));
                }
                body = body.push(label::body(said));
                TITLE_PASTE
            }
        };
        match &w.plan {
            None => {
                body = body.push(label::muted(
                    "Önizle'ye basın: kaç değerin değişeceği ve uymayanlar burada görünür.",
                ))
            }
            Some(Err(why)) => {
                body = body.push(crate::exchange::words::text_line(
                    crate::exchange::words::Kind::Error,
                    why.clone(),
                ))
            }
            Some(Ok(p)) => {
                body = body.push(label::body(format!(
                    "{} nesnede değer değişecek, {} nesnede aynı{}{}.",
                    p.changed,
                    p.unchanged,
                    if p.kept > 0 {
                        format!(", {} nesnede sonuç boş (eski değer kalır)", p.kept)
                    } else {
                        String::new()
                    },
                    if p.invalid > 0 {
                        format!(", {} nesnenin değeri uymuyor", p.invalid)
                    } else {
                        String::new()
                    },
                )));
                let mut list = Column::new().spacing(2);
                for c in p.changes.iter().take(8) {
                    list = list.push(label::muted(format!(
                        "{} · {}: {} → {}",
                        self.row_place(&w.layer, &c.uid),
                        c.field,
                        c.before.as_deref().unwrap_or("—"),
                        c.after.as_deref().unwrap_or("—")
                    )));
                }
                for pr in p.problems.iter().take(8) {
                    list = list.push(crate::exchange::words::text_line(
                        crate::exchange::words::Kind::Error,
                        format!("{} · {}", self.row_place(&w.layer, &pr.uid), pr.message),
                    ));
                }
                body = body.push(list);
            }
        }
        let planned = w.plan.as_ref().and_then(|p| p.as_ref().ok());
        let whole = planned.is_some_and(|p| p.invalid == 0 && p.changed > 0);
        let some = planned.is_some_and(|p| p.invalid > 0 && p.changed > 0);
        let ready = match &w.mode {
            Mode::Calculate => !w.field.trim().is_empty() && !w.expression.trim().is_empty(),
            Mode::Paste { .. } => true,
        };
        let mut dialog = Dialog::new(title)
            .scroll(body)
            .action(crate::exchange::words::secondary(
                "Vazgeç",
                Some(msg(WriteEvent::Close)),
            ))
            .action(crate::exchange::words::secondary(
                "Önizle",
                ready.then(|| msg(WriteEvent::Preview)),
            ));
        if some {
            dialog = dialog.action(crate::exchange::words::secondary(
                "Uyanları yaz",
                Some(msg(WriteEvent::Run(WritePolicy::Valid))),
            ));
        }
        overlay::blocking(
            dialog
                .action(crate::exchange::words::primary(
                    "Yaz",
                    whole.then(|| msg(WriteEvent::Run(WritePolicy::All))),
                ))
                .width(620.0)
                .max_height(720.0),
        )
    }
}

fn container_fill<'a>(e: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    iced::widget::container(e).width(Length::Fill).into()
}
