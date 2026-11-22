//! Satırlar başkasınca değiştirilmiş (docs/adr/9148): a save of a data layer that met rows someone else changed since
//! they were taken. The shared write path merges what the two sides changed in different fields and writes it; what
//! clashes comes here, a row at a time: each field both sides changed to different values with its first value, the
//! user's and the table's, and a choice of which stays; the geometry moved on both sides likewise; a row deleted in the
//! table (written again as a new row, or let go) and a row deleted here that the table changed (deleted still, or kept).
//! Nothing is chosen for the user: Seçilenleri kaydet waits until every clash has a choice, then settles the rows (the
//! object made of the choice as one undo step, the row compared from now on with the table's as read) and saves again,
//! which checks the table once more. Vazgeç keeps the draft as it is; Hepsinde benimkini yaz writes over every clash.

use iced::widget::{Column, container, row};
use iced::{Center, Element, Fill, Length};
use kentos_native_application::data_clash::{RowClash, merged, value_words};
use kentos_ui::label;
use kentos_ui::theme::typography;
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::segmented::Segmented;

use crate::app::{App, Message};
use crate::data_edits::{Event, event};
use crate::exchange::words;

/// The window's title.
pub const TITLE: &str = "Satırlar başkasınca değiştirilmiş";
const SETTLE: &str = "Seçilenleri kaydet";
const ALL_MINE: &str = "Hepsinde benimkini yaz";
const KEEP: &str = "Vazgeç";

/// A save that met clashes: its layers, each row's clash and what is chosen for each of its choices (its fields, then
/// its geometry; a deleted row's one): true the user's, false the table's, none not yet. The words go to the status bar
/// and the log as a failed save's do.
#[derive(Debug, Clone, Default)]
pub(crate) struct Conflict {
    pub layers: Vec<String>,
    pub rows: Vec<RowClash>,
    pub chosen: Vec<Vec<Option<bool>>>,
}

impl Conflict {
    /// The clashes of a save, nothing chosen; a row deleted on both sides has its one way, let go, chosen.
    pub(crate) fn of(layers: Vec<String>, rows: Vec<RowClash>) -> Self {
        let chosen = rows
            .iter()
            .map(|r| {
                if r.gone() && r.mine.is_none() {
                    vec![Some(false)]
                } else if r.gone() || r.deleted_here() {
                    vec![None]
                } else {
                    vec![None; r.fields.len() + usize::from(r.geometry)]
                }
            })
            .collect();
        Conflict { layers, rows, chosen }
    }

    /// Whether every clash has a choice.
    pub(crate) fn ready(&self) -> bool {
        self.chosen.iter().flatten().all(Option::is_some)
    }
}

/// A choice's two words: the user's, the table's.
fn ways(r: &RowClash) -> (&'static str, &'static str) {
    if r.gone() {
        ("Yeni satır olarak ekle", "Değişikliğimi bırak")
    } else if r.deleted_here() {
        ("Yine sil", "Silmekten vazgeç")
    } else {
        ("Benimki", "Tablodaki")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Way(bool, &'static str);

impl std::fmt::Display for Way {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.1)
    }
}

impl App {
    /// The window: a block a row, a line a clash, the buttons.
    pub(crate) fn data_conflict_view(&self) -> Element<'_, Message> {
        let Some(c) = &self.gis.data_edits.conflict else {
            return Column::new().into();
        };
        let width = |w: f32| Length::Fixed(typography::scaled(w));
        let mut body = Column::new().spacing(12).push(label::muted(
            "Aynı alanı iki yanda farklı değiştirilmiş satırlar var. Her biri için hangisinin kalacağını seçin; seçilmeyen yazılmaz. Öbür alanlardaki değişiklikler birleşip yazılır.",
        ));
        for (r_at, (r, picks)) in c.rows.iter().zip(&c.chosen).enumerate() {
            let layer = self.data_layer_name(&r.layer);
            let mut block = Column::new().spacing(6).push(label::strong(format!("Satır {} · {layer}", r.key)));
            let (mine, theirs) = ways(r);
            let choice = |i: usize, pick: Option<bool>| -> Element<'_, Message> {
                let options = [Way(true, mine), Way(false, theirs)];
                // None chosen yet: neither is shown chosen.
                let chosen = match pick {
                    Some(true) => options[0],
                    Some(false) => options[1],
                    None => Way(true, ""),
                };
                Segmented::new(options, chosen, move |w: Way| event(Event::Choose(r_at, i, w.0))).into()
            };
            if r.gone() || r.deleted_here() {
                let what = if r.gone() {
                    match r.mine {
                        Some(_) => "Tabloda silinmiş; sizde değişmiş.",
                        None => "Tabloda da silinmiş.",
                    }
                } else {
                    "Sizde silinmiş; tabloda değişmiş."
                };
                let line = row![container(label::body(what)).width(Fill)].spacing(12).align_y(Center);
                let line = if r.gone() && r.mine.is_none() {
                    line.push(label::muted("Bırakılır."))
                } else {
                    line.push(choice(0, picks.first().copied().flatten()))
                };
                block = block.push(line);
            } else {
                block = block.push(
                    row![
                        container(label::caption("Alan")).width(width(120.0)),
                        container(label::caption("İlk değer")).width(width(140.0)),
                        container(label::caption("Benim değerim")).width(width(140.0)),
                        container(label::caption("Tablodaki değer")).width(width(140.0)),
                        label::caption("Kalan"),
                    ]
                    .spacing(12),
                );
                for (i, f) in r.fields.iter().enumerate() {
                    block = block.push(
                        row![
                            container(label::body(f.field.clone())).width(width(120.0)),
                            container(label::body(value_words(&f.first))).width(width(140.0)),
                            container(label::body(value_words(&f.mine))).width(width(140.0)),
                            container(label::body(value_words(&f.theirs))).width(width(140.0)),
                            choice(i, picks.get(i).copied().flatten()),
                        ]
                        .spacing(12)
                        .align_y(Center),
                    );
                }
                if r.geometry {
                    let i = r.fields.len();
                    block = block.push(
                        row![
                            container(label::body("Şekil")).width(width(120.0)),
                            container(label::body("iki yanda da değişmiş")).width(width(432.0)),
                            choice(i, picks.get(i).copied().flatten()),
                        ]
                        .spacing(12)
                        .align_y(Center),
                    );
                }
            }
            body = body.push(block);
        }
        // How many rows the choice writes in one go, the sound ones with the settled (docs/adr/9152).
        body = body.push(label::muted(self.data_conflict_count(c)));
        Frame::new(TITLE)
            .push(body)
            .action(words::secondary(KEEP, Some(event(Event::KeepConflict))))
            .action(words::secondary(ALL_MINE, Some(event(Event::SaveOver(c.layers.clone())))))
            .action(words::primary(SETTLE, c.ready().then(|| event(Event::Settle))))
            .width(860.0)
            .modal(event(Event::KeepConflict))
    }

    /// What Seçilenleri kaydet writes: the layers' rows waiting, less those whose draft the choices let go.
    pub(crate) fn data_conflict_count(&self, c: &Conflict) -> String {
        let waiting: usize = c
            .layers
            .iter()
            .map(|l| {
                let ch = self.document.as_ref().map(|d| d.model.data_changes(l)).unwrap_or_default();
                ch.added.len() + ch.changed.len() + ch.deleted.len()
            })
            .sum();
        let dropped = c
            .rows
            .iter()
            .zip(&c.chosen)
            .filter(|(r, p)| (r.gone() || r.deleted_here()) && p.first().copied().flatten() == Some(false))
            .count();
        let written = waiting.saturating_sub(dropped);
        if dropped == 0 {
            format!("Seçilenleri kaydet {written} satırı tek işlemde yazar.")
        } else {
            format!("Seçilenleri kaydet {written} satırı tek işlemde yazar; {dropped} satırın taslağı bırakılır.")
        }
    }

    /// A choice made.
    pub(crate) fn data_conflict_choose(&mut self, row: usize, at: usize, mine: bool) {
        if let Some(pick) = self
            .gis
            .data_edits
            .conflict
            .as_mut()
            .and_then(|c| c.chosen.get_mut(row))
            .and_then(|r| r.get_mut(at))
        {
            *pick = Some(mine);
        }
    }

    /// Seçilenleri kaydet: each row settled as chosen, then the layers saved again (the table checked once more).
    /// Whether it settled; the layers to save.
    pub(crate) fn data_conflict_settle(&mut self) -> Option<Vec<String>> {
        // Not every clash chosen: the window stays as it is.
        if !self.gis.data_edits.conflict.as_ref().is_some_and(Conflict::ready) {
            return None;
        }
        let c = self.gis.data_edits.conflict.take()?;
        let doc = self.document.as_mut()?;
        let label = "Çakışmayı çöz";
        let mut updates: Vec<(kentos_domain::Slot, kentos_commands::Entity)> = Vec::new();
        let mut rebase: Vec<(String, kentos_domain::Uuid, kentos_commands::Entity)> = Vec::new();
        let mut untake: Vec<(String, kentos_domain::Uuid)> = Vec::new();
        let mut let_go: Vec<(String, kentos_domain::Uuid)> = Vec::new();
        for (r, picks) in c.rows.iter().zip(&c.chosen) {
            let first = picks.first().copied().flatten().unwrap_or(false);
            match (&r.mine, &r.theirs) {
                (Some(_), Some(theirs)) => {
                    // The object as it is now (it may have changed since the save), merged with the table's row.
                    let Some(slot) = doc.model.slot_of(r.uid) else {
                        continue;
                    };
                    let Some(now) = doc.model.get(slot).cloned() else {
                        continue;
                    };
                    let take: std::collections::HashSet<&str> = r
                        .fields
                        .iter()
                        .zip(picks)
                        .filter(|(_, p)| **p == Some(false))
                        .map(|(f, _)| f.field.as_str())
                        .collect();
                    let geometry_theirs = r.geometry && picks.get(r.fields.len()).copied().flatten() == Some(false);
                    let made = merged(&r.first, &now, theirs, &|n| take.contains(n), geometry_theirs);
                    if made != now {
                        updates.push((slot, made));
                    }
                    rebase.push((r.layer.clone(), r.uid, theirs.clone()));
                }
                // Deleted here, changed in the table: deleted still against the table's row, or forgotten.
                (None, Some(theirs)) if first => rebase.push((r.layer.clone(), r.uid, theirs.clone())),
                (None, Some(_)) => untake.push((r.layer.clone(), r.uid)),
                // Deleted in the table: written again as a new row, or let go.
                (Some(_), None) if first => untake.push((r.layer.clone(), r.uid)),
                (_, None) => let_go.push((r.layer.clone(), r.uid)),
            }
        }
        if !updates.is_empty() {
            let done = doc.model.transact(label, |d| {
                d.update_many(updates.clone(), label);
                Ok::<(), kentos_domain::Refusal>(())
            });
            if let Err(why) = done {
                let why = why.0;
                self.warn(why);
                return None;
            }
        }
        let mut problems = Vec::new();
        for (layer, uid, row) in rebase {
            if let Err(why) = doc.model.data_rebase(&layer, uid, row) {
                problems.push(why);
            }
        }
        for (layer, uid) in untake {
            doc.model.data_untake(&layer, uid);
        }
        let mut dropped_keys: Vec<(String, i64)> = Vec::new();
        for (layer, uid) in let_go {
            if let Some(key) = doc.model.taken_key(&layer, uid) {
                dropped_keys.push((layer.clone(), key));
            }
            if let Err(why) = doc.model.data_let_go(&layer, uid) {
                problems.push(why);
            }
        }
        // A row let go or written again as a new one is no longer the source's gone row.
        for r in c.rows.iter().filter(|r| r.gone()) {
            dropped_keys.push((r.layer.clone(), r.key));
        }
        for (layer, key) in dropped_keys {
            if let Some(keys) = self.gis.data_edits.gone.get_mut(&layer) {
                keys.remove(&key);
            }
        }
        for why in problems {
            self.warn(why);
        }
        Some(c.layers)
    }
}

#[cfg(test)]
#[path = "data_conflict_tests.rs"]
mod data_conflict_tests;
