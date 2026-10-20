//! Kaynakla uzlaştır (`source.reconcile`, docs/adr/0538, plan/gis.md CBS-37): a bound layer and its table, both changed
//! since they last matched (a QGIS user's edit beside the drawing's), put together object by object.
//!
//! The window shows `cad.sources.reconcile`'s plan: what comes into the layer from the table, what goes to the table
//! from the layer, what both changed alike, and each conflict with the attributes that differ (both sides' values) and
//! whether the geometry does. Each conflict takes a side, Benimki (the layer's) or Tablodaki (the table's), one by one
//! or all at once; a row shows its object in the drawing. Uzlaştır runs the same command with the sides taken: the
//! table's side into the layer and the layer's to the table, one undo step and one transaction. Kaynağa yaz refused
//! because the file changed opens it.

use std::collections::BTreeMap;

use iced::widget::{Column, button, container, row, scrollable, text};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    CommandResult, ConflictKind, ReconcileConflict, ReconcileSide, ReconcileTake,
    SourcesReconcile, SourcesReconciled,
};
use kentos_interaction::Level;
use kentos_native_application::ExecutionContext;
use kentos_native_application::sources::reconcile;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Dialog as Frame, Segmented, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};

/// The window's title, which a trace names it by.
pub const RECONCILE_TITLE: &str = "Kaynakla uzlaştır";
const APPLY: &str = "Uzlaştır";
const CANCEL: &str = "Vazgeç";
const ALL_MINE: &str = "Hepsi benimki";
const ALL_THEIRS: &str = "Hepsi tablodaki";

/// A side as the window writes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Side(pub ReconcileSide);

impl std::fmt::Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.0 {
            ReconcileSide::Layer => "Benimki",
            ReconcileSide::Table => "Tablodaki",
        })
    }
}

/// The window: the layer, the plan as it opened, the sides taken so far.
#[derive(Debug, Clone)]
pub struct Window {
    layer: String,
    plan: SourcesReconciled,
    sides: BTreeMap<String, ReconcileSide>,
    /// Opened because Kaynağa yaz was refused: the file changed.
    refused: bool,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// A conflict's side, by its object.
    Side(String, ReconcileSide),
    /// Every conflict to one side.
    All(ReconcileSide),
    /// A conflict's object shown in the drawing.
    Show(String),
    Apply,
    Cancel,
}

impl Window {
    /// The first conflict's object (a picture's choice).
    #[cfg(test)]
    pub(crate) fn first_conflict(&self) -> Option<String> {
        self.plan.conflicts.first().map(|c| c.object.clone())
    }
}

fn msg(event: Event) -> Message {
    Message::Reconcile(event)
}

/// “2 eklenecek, 1 değişecek”: the counts that are not naught; “yok” when all are.
fn counts(c: &kentos_commands::ReconcileCounts) -> String {
    let parts: Vec<String> = [
        (c.added, "eklenecek"),
        (c.modified, "değişecek"),
        (c.removed, "silinecek"),
    ]
    .iter()
    .filter(|(n, _)| *n > 0)
    .map(|(n, w)| format!("{n} {w}"))
    .collect();
    if parts.is_empty() {
        "yok".to_owned()
    } else {
        parts.join(", ")
    }
}

fn kind_words(k: ConflictKind) -> &'static str {
    match k {
        ConflictKind::Changed => "İki yanda da değişti",
        ConflictKind::RemovedInLayer => "Katmanda silindi, tabloda değişti",
        ConflictKind::RemovedInTable => "Tabloda silindi, katmanda değişti",
    }
}

/// A conflict's differences in a line: “malik: Veli ↔ Zehra · geometri farklı”.
fn differences(c: &ReconcileConflict) -> String {
    let mut parts: Vec<String> = c
        .fields
        .iter()
        .map(|f| {
            format!(
                "{}: {} ↔ {}",
                f.name,
                f.layer.as_deref().unwrap_or("—"),
                f.table.as_deref().unwrap_or("—")
            )
        })
        .collect();
    if c.geometry && c.kind == ConflictKind::Changed {
        parts.push("geometri farklı".to_owned());
    }
    parts.join(" · ")
}

impl App {
    /// The plan of the layer's reconcile with the sides taken.
    fn reconcile_plan(
        &self,
        layer: &str,
        sides: &BTreeMap<String, ReconcileSide>,
    ) -> Option<CommandResult<SourcesReconciled>> {
        let doc = self.document.as_ref()?;
        let mut model = doc.model.clone();
        Some(reconcile::plan(&ExecutionContext::new(&mut model), &input(layer, sides)))
    }

    /// Kaynakla uzlaştır on a bound layer: the window with the plan, or what refused it.
    pub(crate) fn open_reconcile(&mut self, layer: &str, refused: bool) {
        let sides = BTreeMap::new();
        let Some(planned) = self.reconcile_plan(layer, &sides) else {
            self.output("Açık çizim yok.");
            return;
        };
        match planned {
            CommandResult::Completed { output, .. } => {
                self.reconcile = Some(Window {
                    layer: layer.to_owned(),
                    plan: output,
                    sides,
                    refused,
                });
                self.dialog = Some(Dialog::SourceReconcile);
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.announce(Level::Error, error.message),
            _ => {}
        }
    }

    pub(crate) fn reconcile_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Side(object, side) => {
                if let Some(w) = self.reconcile.as_mut() {
                    w.sides.insert(object, side);
                }
                self.reconcile_replan();
            }
            Event::All(side) => {
                if let Some(w) = self.reconcile.as_mut() {
                    for c in &w.plan.conflicts {
                        w.sides.insert(c.object.clone(), side);
                    }
                }
                self.reconcile_replan();
            }
            Event::Show(object) => {
                let slot = self.document.as_ref().and_then(|d| {
                    let uid = kentos_domain::Uuid::parse_str(&object).ok()?;
                    d.model.slot_of(uid)
                });
                if let Some(slot) = slot {
                    self.selection.set([slot]);
                    self.zoom_to_objects(&[f64::from(slot.0)]);
                }
            }
            Event::Apply => self.reconcile_apply(),
            Event::Cancel => {
                self.reconcile = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    /// The plan again with the sides taken, so the counts say what Uzlaştır will do.
    fn reconcile_replan(&mut self) {
        let Some(w) = self.reconcile.clone() else {
            return;
        };
        match self.reconcile_plan(&w.layer, &w.sides) {
            Some(CommandResult::Completed { output, .. }) => {
                if let Some(w) = self.reconcile.as_mut() {
                    w.plan = output;
                }
            }
            Some(
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error },
            ) => self.announce(Level::Error, error.message),
            _ => {}
        }
    }

    /// How a conflict's object is named in its row: its first attributes in the layer's field order (the table's for
    /// one the layer removed), else the start of its lasting id.
    fn reconcile_name(&self, layer: &str, c: &ReconcileConflict) -> String {
        let short: String = c.object.chars().take(8).collect();
        let Some(doc) = self.document.as_ref() else {
            return short;
        };
        let attrs = kentos_domain::Uuid::parse_str(&c.object)
            .ok()
            .and_then(|u| doc.model.slot_of(u))
            .and_then(|s| doc.model.get(s))
            .map(|e| e.base().attrs.clone());
        let fields: Vec<String> = doc
            .model
            .layers()
            .get(layer)
            .map(|n| n.fields.iter().map(|f| f.name.clone()).collect())
            .unwrap_or_default();
        let named: Vec<String> = match attrs {
            Some(a) => fields
                .iter()
                .filter_map(|f| a.get(f).filter(|v| !v.is_empty()).map(|v| format!("{f} {v}")))
                .take(2)
                .collect(),
            // The layer removed it: the table's values, as the differences say them.
            None => c
                .fields
                .iter()
                .filter_map(|f| f.table.as_ref().map(|v| format!("{} {v}", f.name)))
                .take(2)
                .collect(),
        };
        if named.is_empty() { short } else { named.join(" · ") }
    }

    fn reconcile_apply(&mut self) {
        let Some(w) = self.reconcile.clone() else {
            return;
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let answer = reconcile::execute(
            &mut ExecutionContext::new(&mut doc.model),
            input(&w.layer, &w.sides),
        );
        match answer {
            CommandResult::Completed { output, warnings } => {
                self.reconcile = None;
                self.dialog = None;
                let name = self.bound_name(&w.layer);
                self.announce(
                    if warnings.is_empty() {
                        Level::Success
                    } else {
                        Level::Warn
                    },
                    format!(
                        "“{name}” tablosuyla uzlaştırıldı: tablodan katmana {}; katmandan tabloya {}. Ctrl+Z katmanı geri alır, dosya yazılmış kalır.",
                        counts(&output.from_table).replace("eklenecek", "eklendi").replace("değişecek", "değişti").replace("silinecek", "silindi"),
                        counts(&output.to_table).replace("eklenecek", "eklendi").replace("değişecek", "değişti").replace("silinecek", "silindi"),
                    ),
                );
                for warning in warnings {
                    self.say(Level::Warn, warning.message);
                }
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.announce(Level::Error, error.message),
            _ => {}
        }
    }

    pub(crate) fn reconcile_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.reconcile else {
            return text("").into();
        };
        let p = &w.plan;
        let unresolved = p
            .conflicts
            .iter()
            .filter(|c| !w.sides.contains_key(&c.object))
            .count();
        let mut lines = Vec::new();
        if w.refused {
            lines.push(words::text_line(
                Kind::Warn,
                "Dosya katman onunla son eşleştiğinden beri başka bir programda değişti; yazmak o değişiklikleri ezerdi. Son eşleşmeye göre iki yanın değişiklikleri aşağıda.",
            ));
        }
        lines.push(words::text_line(
            Kind::Info,
            format!("Tablodan katmana: {}.", counts(&p.from_table)),
        ));
        lines.push(words::text_line(
            Kind::Info,
            format!("Katmandan tabloya: {}.", counts(&p.to_table)),
        ));
        if p.agreed > 0 {
            lines.push(words::text_line(
                Kind::Ok,
                format!("İki yanın aynı biçimde değiştirdiği {} nesne uyuşuyor.", p.agreed),
            ));
        }
        lines.push(if p.conflicts.is_empty() {
            words::text_line(Kind::Ok, "Çatışma yok: Uzlaştır iki yanı birleştirir.")
        } else if unresolved > 0 {
            words::text_line(
                Kind::Warn,
                format!(
                    "{} çatışmadan {unresolved} tanesinin yanı seçilmedi; son yazan kazanmaz, her biri için seçin.",
                    p.conflicts.len()
                ),
            )
        } else {
            words::text_line(
                Kind::Ok,
                format!("{} çatışmanın hepsinin yanı seçildi.", p.conflicts.len()),
            )
        });
        let mut body = Column::new().spacing(12).push(words::summary(lines));
        if !p.conflicts.is_empty() {
            let mut list = Column::new().spacing(0).padding([4, 0]);
            for c in &p.conflicts {
                let picked = w.sides.get(&c.object).copied().map(Side);
                let object = c.object.clone();
                let choice = Segmented::optional(
                    [Side(ReconcileSide::Layer), Side(ReconcileSide::Table)],
                    picked,
                    move |s| msg(Event::Side(object.clone(), s.0)),
                );
                let detail = differences(c);
                let mut words_col = Column::new().spacing(2).push(label::body(format!(
                    "{} — {}",
                    self.reconcile_name(&w.layer, c),
                    kind_words(c.kind)
                )));
                if !detail.is_empty() {
                    words_col = words_col.push(label::caption(detail).style(
                        |theme: &iced::Theme| iced::widget::text::Style {
                            color: Some(Tokens::of(theme).muted),
                        },
                    ));
                }
                let shown = self
                    .document
                    .as_ref()
                    .and_then(|d| kentos_domain::Uuid::parse_str(&c.object).ok().and_then(|u| d.model.slot_of(u)))
                    .is_some();
                let face = button(words_col.width(Fill))
                    .on_press_maybe(shown.then(|| msg(Event::Show(c.object.clone()))))
                    .padding([4, 6])
                    .width(Fill)
                    .style(style::button::ghost);
                list = list.push(
                    container(row![face, choice].spacing(10).align_y(Center))
                        // Room on the right for the list's scroll bar.
                        .padding(iced::Padding {
                            top: 3.0,
                            right: 18.0,
                            bottom: 3.0,
                            left: 4.0,
                        }),
                );
            }
            body = body
                .push(
                    row![
                        label::caption(format!("Çatışmalar ({})", p.conflicts.len())),
                        iced::widget::Space::new().width(Fill),
                        words::secondary(ALL_MINE, Some(msg(Event::All(ReconcileSide::Layer)))),
                        words::secondary(ALL_THEIRS, Some(msg(Event::All(ReconcileSide::Table)))),
                    ]
                    .spacing(8)
                    .align_y(Center),
                )
                .push(
                    container(scrollable(list).height(typography::scaled(260.0)))
                        .style(style::container::field_box)
                        .width(Fill),
                );
        }
        overlay::modal(
            Frame::new(RECONCILE_TITLE)
                .push(body)
                .action(words::secondary(CANCEL, Some(msg(Event::Cancel))))
                .action(words::primary(
                    APPLY,
                    (unresolved == 0).then(|| msg(Event::Apply)),
                ))
                .width(640.0),
            msg(Event::Cancel),
        )
    }
}

/// The command's input: the sides taken, one by one.
fn input(layer: &str, sides: &BTreeMap<String, ReconcileSide>) -> SourcesReconcile {
    SourcesReconcile {
        layer: layer.to_owned(),
        take: sides
            .iter()
            .map(|(object, side)| ReconcileTake {
                object: object.clone(),
                side: *side,
            })
            .collect(),
        rest: None,
    }
}
