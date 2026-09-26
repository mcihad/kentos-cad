//! DXF içe aktar (the web's `ui/io/DxfImportDialog.ts`). The shared reader
//! reads the whole file once, off the UI thread (blocks exploded, object
//! coordinate systems applied, a report of what was converted or left out).
//! The window shows the source's layers with their object counts and where
//! each goes (a project layer with the same name, or a new layer in a group
//! named after the file), the report, and the coordinate system question.
//! Everything chosen goes in as one undo step.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use iced::widget::{Column, column, row};
use iced::{Center, Element, Fill, Length, Task};
use kentos_contracts::{DxfReadOptions, ImportLayer, ImportResult, LayerStyle};
use kentos_interaction::{Format, Level};
use kentos_ui::label;
use kentos_ui::widget::table::{Column as TableColumn, Row as TableRow, Table};
use kentos_ui::widget::tree_view::{Check, check_box};
use kentos_ui::widget::{Dialog, overlay, swatch};

use super::apply::{self, ImportPlan, LayerTarget, layer_named};
use super::crs::CrsQuestion;
use super::words::{self, Kind as Line};
use super::{Event as Exchange, Kind, Picked, Window, message, off_thread};
use crate::app::{App, Message};
use crate::view::hex_color;

/// Reads told apart: a late answer to an older read is dropped.
static READS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct State {
    file: Picked,
    read: u64,
    reading: bool,
    result: Option<Arc<ImportResult>>,
    failed: Option<String>,
    /// The first object with a number that is not finite: nothing is imported.
    unusable: Option<String>,
    /// Source layer names left out by the user.
    excluded: BTreeSet<String>,
    crs: CrsQuestion,
    /// What the footer says, and whether it is an error.
    status: Option<(bool, String)>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Read {
        read: u64,
        result: Result<Arc<ImportResult>, String>,
        unusable: Option<String>,
    },
    Toggle(String),
    ToggleAll,
    Crs(u32),
    Another,
    Run,
}

fn event(e: Event) -> Message {
    message(Exchange::DxfImport(e))
}

/// Where a source layer's objects go: the project layer of the same name
/// (and whether it is locked), or a new one.
struct Target {
    existing: Option<String>,
    locked: bool,
}

impl App {
    fn target(&self, layer: &ImportLayer) -> Target {
        let Some(doc) = &self.document else {
            return Target {
                existing: None,
                locked: false,
            };
        };
        match layer_named(&doc.model, &layer.name) {
            Some(node) => Target {
                locked: doc.model.layers().is_locked(&node.id),
                existing: Some(node.id.clone()),
            },
            None => Target {
                existing: None,
                locked: false,
            },
        }
    }

    /// The layers that will be imported: chosen, and not onto a locked layer.
    fn dxf_included<'a>(&self, state: &'a State) -> Vec<&'a ImportLayer> {
        state
            .result
            .as_ref()
            .map(|r| {
                r.layers
                    .iter()
                    .filter(|l| !state.excluded.contains(&l.name) && !self.target(l).locked)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(super) fn dxf_import_picked(&mut self, file: Picked) -> Task<Message> {
        let srid = self.project_srid();
        // “Başka dosya…” keeps the chosen coordinate system.
        let crs = match &self.exchange {
            Some(Window::DxfImport(s)) => s.crs,
            _ => CrsQuestion::new(srid),
        };
        let read = READS.fetch_add(1, Ordering::Relaxed) + 1;
        let bytes = file.bytes.clone();
        self.open_window(Window::DxfImport(State {
            file,
            read,
            reading: true,
            result: None,
            failed: None,
            unusable: None,
            excluded: BTreeSet::new(),
            crs,
            status: None,
        }));
        off_thread(
            move || {
                let result = kentos_formats::dxf::read(&bytes, &DxfReadOptions { max_entities: 0 });
                let unusable = result
                    .as_ref()
                    .ok()
                    .and_then(|r| apply::unusable(&r.entities))
                    .map(|(place, kind)| apply::unusable_text(place, kind));
                (result, unusable)
            },
            move |(result, unusable)| {
                Exchange::DxfImport(Event::Read {
                    read,
                    result: result.map(Arc::new),
                    unusable,
                })
            },
        )
    }

    pub(super) fn dxf_import_event(&mut self, e: Event) -> Task<Message> {
        if let Event::Another = e {
            return self.pick(Kind::Dxf);
        }
        if let Event::Run = e {
            self.run_dxf_import();
            return Task::none();
        }
        let Some(Window::DxfImport(s)) = &mut self.exchange else {
            return Task::none();
        };
        match e {
            Event::Read {
                read,
                result,
                unusable,
            } => {
                if read != s.read {
                    return Task::none();
                }
                s.reading = false;
                match result {
                    Ok(r) => s.result = Some(r),
                    Err(e) => s.failed = Some(e),
                }
                s.unusable = unusable;
            }
            Event::Toggle(name) => {
                if !s.excluded.remove(&name) {
                    s.excluded.insert(name);
                }
            }
            Event::ToggleAll => {
                if s.excluded.is_empty() {
                    if let Some(r) = &s.result {
                        s.excluded = r.layers.iter().map(|l| l.name.clone()).collect();
                    }
                } else {
                    s.excluded.clear();
                }
            }
            Event::Crs(srid) => s.crs.srid = srid,
            Event::Another | Event::Run => {}
        }
        Task::none()
    }

    fn dxf_can_import(&self, s: &State) -> bool {
        !s.reading
            && s.result.is_some()
            && s.unusable.is_none()
            && !self.dxf_included(s).is_empty()
            && s.crs.matches(self.project_srid())
    }

    fn run_dxf_import(&mut self) {
        let Some(Window::DxfImport(s)) = &self.exchange else {
            return;
        };
        if !self.dxf_can_import(s) {
            return;
        }
        let Some(result) = s.result.clone() else {
            return;
        };
        let name = s.file.name.clone();
        let layers: Vec<(String, LayerTarget)> = self
            .dxf_included(s)
            .into_iter()
            .map(|l| {
                let target = match self.target(l).existing {
                    Some(id) => LayerTarget::Existing(id),
                    None => LayerTarget::New {
                        name: l.name.clone(),
                        style: Box::new(LayerStyle {
                            color: l.color.clone(),
                            line_type: l.line_type,
                            // The web's `...(l.lineWeight ? { lineWeight } : {})`: none or 0 is the default.
                            line_weight: l
                                .line_weight
                                .filter(|w| *w != 0.0)
                                .unwrap_or(kentos_domain::default_style().line_weight),
                            ..kentos_domain::default_style()
                        }),
                        visible: l.visible,
                        locked: l.locked,
                    },
                };
                (l.name.clone(), target)
            })
            .collect();
        let chosen = layers.len();
        let plan = ImportPlan {
            label: format!("DXF: {name}"),
            layers,
            group: Some(name.clone()),
        };
        let Some(doc) = &mut self.document else {
            return;
        };
        match apply::apply_import(&mut doc.model, result.entities.clone(), &plan) {
            Err(error) => {
                if let Some(Window::DxfImport(s)) = &mut self.exchange {
                    s.status = Some((true, error));
                }
            }
            Ok(applied) => {
                self.zoom_to(&applied.slots);
                let into = if applied.created.is_empty() {
                    String::new()
                } else {
                    format!("; {} yeni katman “{name}” grubunda", applied.created.len())
                };
                self.say(
                    Level::Success,
                    format!(
                        "“{name}”: {} nesne {chosen} katmana alındı{into}. Tek adımda geri alınabilir.",
                        applied.slots.len()
                    ),
                );
                if !result.report.skipped.is_empty() {
                    let skipped: Vec<String> = result
                        .report
                        .skipped
                        .iter()
                        .map(words::report_text)
                        .collect();
                    self.warn(format!(
                        "“{name}” içinde alınmayanlar: {}",
                        skipped.join(" ")
                    ));
                }
                self.close_exchange();
            }
        }
    }

    pub(super) fn dxf_import_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let srid = self.project_srid();
        let r = s.result.as_deref();
        let meta = match (r, &s.failed) {
            (Some(r), _) => r
                .report
                .source
                .iter()
                .map(|f| format!("{}: {}", f.label, f.value))
                .collect::<Vec<_>>()
                .join(", "),
            (None, Some(_)) => "okunamadı".to_owned(),
            (None, None) => "okunuyor…".to_owned(),
        };
        let mut body = Column::new()
            .spacing(12)
            .push(words::file_line(&s.file.name, meta))
            .push(self.dxf_layers(s))
            .push(self.dxf_summary(s))
            .push(s.crs.view(srid, |srid| event(Event::Crs(srid))));
        if let Some((error, words)) = &s.status {
            body = body.push(words::text_line(
                if *error { Line::Error } else { Line::Info },
                words.clone(),
            ));
        }
        let can = self.dxf_can_import(s);
        overlay::blocking(
            Dialog::new("DXF içe aktar")
                .push(body)
                .action(words::ghost("Başka dosya…", Some(event(Event::Another))))
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary("İçe aktar", can.then(|| event(Event::Run))))
                .width(900.0),
        )
    }

    fn dxf_layers<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let Some(r) = &s.result else {
            return words::empty(if s.failed.is_some() {
                "Katman yok."
            } else {
                "Dosya okunuyor…"
            });
        };
        if r.layers.is_empty() {
            return words::empty("Dosyada alınacak nesne yok.");
        }
        let all = match s.excluded.len() {
            0 => Check::Checked,
            n if n == r.layers.len() => Check::Unchecked,
            _ => Check::Mixed,
        };
        let head = row![
            check_box(all, Some(event(Event::ToggleAll))),
            label::caption(format!("Bütün katmanlar ({})", r.layers.len())),
        ]
        .spacing(8)
        .align_y(Center);
        let rows = r.layers.iter().map(|l| {
            let t = self.target(l);
            let checked = !s.excluded.contains(&l.name) && !t.locked;
            let toggle = (!t.locked).then(|| event(Event::Toggle(l.name.clone())));
            let where_to = match (&t.existing, t.locked) {
                (Some(id), true) => format!(
                    "“{}” katmanı kilitli; alınmaz. Kilidini Katmanlar panelinden açın.",
                    self.layer_name(id)
                ),
                (Some(id), false) => format!("“{}” katmanına eklenir", self.layer_path(id)),
                (None, _) => format!(
                    "yeni katman{}{}",
                    if l.visible { "" } else { ", gizli" },
                    if l.locked { ", kilitli" } else { "" }
                ),
            };
            TableRow::new([
                check_box(
                    if checked {
                        Check::Checked
                    } else {
                        Check::Unchecked
                    },
                    toggle,
                ),
                row![swatch(hex_color(&l.color)), label::body(l.name.clone())]
                    .spacing(6)
                    .align_y(Center)
                    .into(),
                label::body(l.count.to_string()).into(),
                if t.locked {
                    label::caption(where_to)
                        .style(kentos_ui::style::text::danger)
                        .into()
                } else {
                    label::caption(where_to).into()
                },
            ])
        });
        let table = Table::new([
            TableColumn::new("").width(22),
            TableColumn::new("DXF katmanı").width(Length::FillPortion(2)),
            TableColumn::new("Nesne").width(60).align_right(),
            TableColumn::new("Nereye").width(Length::FillPortion(3)),
        ])
        .extend(rows)
        .height(Length::Fixed(200.0));
        column![head, table].spacing(6).width(Fill).into()
    }

    fn dxf_summary<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let Some(r) = &s.result else {
            return words::summary(vec![match &s.failed {
                Some(e) => words::text_line(Line::Error, e.clone()),
                None => words::text_line(
                    Line::Info,
                    "Dosya okunuyor; büyük dosyalar biraz sürebilir.",
                ),
            }]);
        };
        let included = self.dxf_included(s);
        let chosen: BTreeSet<&str> = included.iter().map(|l| l.name.as_str()).collect();
        let mut counts = Vec::new();
        for e in &r.entities {
            if chosen.contains(e.base().layer_id.as_str()) {
                words::count(&mut counts, e.kind());
            }
        }
        let total: u32 = counts.iter().map(|(_, n)| n).sum();
        let created = included
            .iter()
            .filter(|l| self.target(l).existing.is_none())
            .count();
        let mut lines = vec![if total > 0 {
            let group = if created > 0 {
                format!(
                    " {created} yeni katman “{}” grubunda kurulacak.",
                    s.file.name
                )
            } else {
                String::new()
            };
            words::text_line(
                Line::Ok,
                format!(
                    "{total} nesne alınacak: {}.{group}",
                    words::kind_counts(&counts)
                ),
            )
        } else {
            words::text_line(Line::Warn, "Alınacak nesne yok; en az bir katman seçin.")
        }];
        if let Some(unusable) = &s.unusable {
            lines.push(words::text_line(Line::Error, unusable.clone()));
        }
        lines.extend(words::report_lines(&r.report.notes, Line::Info, 12));
        lines.extend(words::report_lines(&r.report.skipped, Line::Warn, 12));
        if let (Some(b), Some(doc)) = (&r.bounds, &self.document) {
            lines.push(words::text_line(
                Line::Info,
                words::extent_text(&Format::of(doc.settings()), b),
            ));
        }
        words::summary(lines)
    }

    /// The open drawing's coordinate system.
    pub(super) fn project_srid(&self) -> u32 {
        self.document.as_ref().map_or(5256, |d| d.settings().srid)
    }

    pub(super) fn layer_name(&self, id: &str) -> String {
        self.document
            .as_ref()
            .and_then(|d| d.model.layers().get(id))
            .map_or_else(|| id.to_owned(), |n| n.name.clone())
    }

    /// “Grup / Katman” (web `layers.path`).
    pub(super) fn layer_path(&self, id: &str) -> String {
        let Some(doc) = &self.document else {
            return id.to_owned();
        };
        let layers = doc.model.layers();
        let mut names = Vec::new();
        let mut at = layers.get(id);
        let mut current = id.to_owned();
        while let Some(node) = at {
            names.push(node.name.clone());
            let parent = layers.parent(&current).map(|p| p.id.clone());
            at = parent.as_deref().and_then(|p| layers.get(p));
            current = parent.unwrap_or_default();
        }
        names.reverse();
        names.join(" / ")
    }
}
