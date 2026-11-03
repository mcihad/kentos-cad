//! Read-only CAD quality panel. Findings and markers never become document entities.
use crate::{
    app::{App, Message},
    bottom::BottomTab,
};
use iced::widget::{button, column, container, row};
use iced::{Element, Fill, Length, Task};
use kentos_commands::{CommandResult, DrawingInspect, DrawingInspected, InspectionGroup};
use kentos_domain::{Slot, Uuid};
use kentos_interaction::{Marker, MarkerShape, Preview, Stroke, Tag, Tone, Vec2};
use kentos_ui::widget::table::{Column as TableColumn, Row as TableRow, Table};
use kentos_ui::{
    label, style,
    theme::typography,
    widget::{Elided, NumberInput, number::units, select::{Choice, Select}},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub(crate) const TAB: &str = "CAD denetimi";
const SCOPES: [&str; 3] = ["Görünür çizim", "Seçili nesneler", "Etkin katman"];

#[derive(Debug)]
pub(crate) struct Panel {
    scope: usize,
    gap: f64,
    boundaries: bool,
    hatches: bool,
    annotations: bool,
    hidden: bool,
    busy: bool,
    token: u64,
    session: u64,
    generation: u64,
    cancel: Arc<AtomicBool>,
    output: Option<DrawingInspected>,
    selected: Option<usize>,
    message: Option<String>,
}
impl Default for Panel {
    fn default() -> Self {
        Self {
            scope: 0,
            gap: 0.01,
            boundaries: true,
            hatches: true,
            annotations: true,
            hidden: false,
            busy: false,
            token: 0,
            session: 0,
            generation: 0,
            cancel: Arc::new(AtomicBool::new(false)),
            output: None,
            selected: None,
            message: None,
        }
    }
}
impl Panel {
    fn invalidate(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.token = self.token.wrapping_add(1);
        self.busy = false;
        self.output = None;
        self.selected = None;
        self.message = Some("Seçenekler değişti; yeniden denetleyin.".into());
    }
}
impl Drop for Panel {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
#[derive(Clone, Debug)]
pub enum Event {
    Run,
    Stop,
    Scope(usize),
    Gap(f64),
    Boundaries(bool),
    Hatches(bool),
    Annotations(bool),
    Sources,
    Hidden(bool),
    Ready(u64, u64, u64, Box<CommandResult<DrawingInspected>>),
    Show(usize),
    Near,
    Repair,
}
fn msg(e: Event) -> Message {
    Message::CadInspection(e)
}
fn cell(text: String) -> Element<'static, Message> {
    Elided::new(text)
        .size(typography::body())
        .width(Fill)
        .into()
}

impl App {
    pub(crate) fn open_cad_inspection(&mut self) -> Task<Message> {
        if self.document.is_none() || !self.cad_axes() {
            self.output("Önce bir CAD çizimi açın.");
            return Task::none();
        }
        self.show_bottom(BottomTab::CadInspection);
        let bar = f64::from(kentos_ui::widget::tabs::height());
        if f64::from(self.bottom_log()) + bar < crate::search::PANEL_HEIGHT {
            self.bottom_dragged(
                Some((crate::search::PANEL_HEIGHT - bar) as f32),
                std::time::Instant::now(),
            );
        }
        self.cad_inspection_event(Event::Run)
    }
    fn cad_inspection_current(&self) -> bool {
        self.document.as_ref().is_some_and(|d| {
            d.session == self.cad_inspection.session
                && d.model.generation() == self.cad_inspection.generation
        })
    }
    fn cad_inspection_input(&self) -> Option<DrawingInspect> {
        let doc = self.document.as_ref()?;
        let p = &self.cad_inspection;
        let uids = match p.scope {
            1 => Some(
                self.selection
                    .ids()
                    .iter()
                    .filter_map(|s| doc.model.uid(*s))
                    .map(|u| u.to_string())
                    .collect(),
            ),
            2 => Some(
                doc.model
                    .entities_with_uids()
                    .filter(|(_, e)| e.base().layer_id == doc.model.layers().active())
                    .map(|(u, _)| u.to_string())
                    .collect(),
            ),
            _ => None,
        };
        let mut checks = Vec::new();
        if p.boundaries {
            checks.push(InspectionGroup::Boundaries);
        }
        if p.hatches {
            checks.push(InspectionGroup::Hatches);
        }
        if p.annotations {
            checks.push(InspectionGroup::Annotations);
        }
        Some(DrawingInspect {
            uids,
            checks,
            max_gap: Some(p.gap),
            limit: Some(200),
            include_hidden: p.hidden,
            expected_revision: Some(doc.model.revision().to_string()),
        })
    }
    pub(crate) fn cad_inspection_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Run => {
                if !self.cad_inspection.boundaries && !self.cad_inspection.hatches && !self.cad_inspection.annotations {
                    return Task::none();
                }
                let Some(input) = self.cad_inspection_input() else {
                    return Task::none();
                };
                let echo = self.echo_query(
                    kentos_commands::CAD_DRAWING_INSPECT,
                    &serde_json::to_value(&input).unwrap_or_default(),
                );
                let doc = self.document.as_ref().unwrap();
                let model = doc.model.clone();
                let (session, generation) = (doc.session, doc.model.generation());
                let p = &mut self.cad_inspection;
                p.cancel.store(true, Ordering::Relaxed);
                p.cancel = Arc::new(AtomicBool::new(false));
                p.token = p.token.wrapping_add(1);
                p.session = session;
                p.generation = generation;
                p.busy = true;
                p.message = None;
                p.output = None;
                p.selected = None;
                let token = p.token;
                let cancel = Arc::clone(&p.cancel);
                return Task::batch([
                    echo,
                    crate::db_connections::off_thread(
                        move || {
                            kentos_native_application::drawing_inspect::inspect(
                                &model,
                                &input,
                                || cancel.load(Ordering::Relaxed),
                            )
                        },
                        move |answer| {
                            msg(Event::Ready(token, session, generation, Box::new(answer)))
                        },
                    ),
                ]);
            }
            Event::Ready(token, session, generation, answer) => {
                if token != self.cad_inspection.token
                    || self.document.as_ref().is_none_or(|d| d.session != session)
                {
                    return Task::none();
                }
                self.cad_inspection.busy = false;
                if self
                    .document
                    .as_ref()
                    .is_none_or(|d| d.model.generation() != generation)
                {
                    self.cad_inspection.message =
                        Some("Çizim denetim sırasında değişti; yeniden denetleyin.".into());
                    return Task::none();
                }
                match *answer {
                    CommandResult::Completed { output, .. } => {
                        self.cad_inspection.output = Some(output)
                    }
                    CommandResult::Cancelled => {
                        self.cad_inspection.message =
                            Some("Denetim durduruldu; çizim değiştirilmedi.".into())
                    }
                    CommandResult::Failed { error }
                    | CommandResult::Conflict { error }
                    | CommandResult::NeedsInput { error } => {
                        self.cad_inspection.message = Some(error.message)
                    }
                    CommandResult::Queued { .. } => {
                        self.cad_inspection.message = Some(
                            "Denetim sonucu henüz hazır değil; yeniden deneyin.".into(),
                        )
                    }
                }
            }
            Event::Stop => {
                self.cad_inspection.cancel.store(true, Ordering::Relaxed);
                self.cad_inspection.token = self.cad_inspection.token.wrapping_add(1);
                self.cad_inspection.busy = false;
                self.cad_inspection.output = None;
                self.cad_inspection.selected = None;
                self.cad_inspection.message =
                    Some("Denetim durduruldu; çizim değiştirilmedi.".into());
            }
            Event::Scope(i) => {
                self.cad_inspection.scope = i.min(2);
                self.cad_inspection.invalidate();
            }
            Event::Gap(n) => {
                self.cad_inspection.gap = n;
                self.cad_inspection.invalidate();
            }
            Event::Boundaries(on) => {
                self.cad_inspection.boundaries = on;
                self.cad_inspection.invalidate();
            }
            Event::Hatches(on) => {
                self.cad_inspection.hatches = on;
                self.cad_inspection.invalidate();
            }
            Event::Annotations(on) => {
                self.cad_inspection.annotations = on;
                self.cad_inspection.invalidate();
            }
            Event::Hidden(on) => {
                self.cad_inspection.hidden = on;
                self.cad_inspection.invalidate();
            }
            Event::Show(i) => {
                if !self.cad_inspection_current() {
                    return Task::none();
                }
                let Some(f) = self
                    .cad_inspection
                    .output
                    .as_ref()
                    .and_then(|r| r.findings.get(i))
                    .cloned()
                else {
                    return Task::none();
                };
                let doc = self.document.as_ref().unwrap();
                let slots: Vec<Slot> = f
                    .uids
                    .iter()
                    .take(if f.group == InspectionGroup::Annotations { 1 } else { usize::MAX })
                    .filter_map(|u| Uuid::parse_str(u).ok())
                    .filter_map(|u| doc.model.slot_of(u))
                    .collect();
                let ids: Vec<_> = slots.iter().map(|s| f64::from(s.0)).collect();
                self.selection.set(slots);
                self.cad_inspection.selected = Some(i);
                if !ids.is_empty() {
                    self.navigating(|app| app.zoom_to_objects(&ids));
                }
            }
            Event::Near => {
                if !self.cad_inspection_current() {
                    return Task::none();
                }
                let Some(points) = self
                    .cad_inspection
                    .selected
                    .and_then(|i| self.cad_inspection.output.as_ref()?.findings.get(i))
                    .map(|f| f.points.clone())
                else {
                    return Task::none();
                };
                if let Some(first) = points.first() {
                    let pad = self.cad_inspection.gap.max(0.02) * 4.;
                    let mut b = kentos_render_wgpu::Bounds {
                        min_x: first.x - pad,
                        min_y: first.y - pad,
                        max_x: first.x + pad,
                        max_y: first.y + pad,
                    };
                    for p in points {
                        b.min_x = b.min_x.min(p.x - pad);
                        b.min_y = b.min_y.min(p.y - pad);
                        b.max_x = b.max_x.max(p.x + pad);
                        b.max_y = b.max_y.max(p.y + pad);
                    }
                    self.navigating(|app| app.viewport.camera.fit(&b, 24.));
                }
            }
            Event::Sources => {
                if !self.cad_inspection_current() { return Task::none(); }
                let Some(f) = self.cad_inspection.selected.and_then(|i| self.cad_inspection.output.as_ref()?.findings.get(i)) else { return Task::none(); };
                let doc = &self.document.as_ref().unwrap().model;
                let slots: Vec<_> = f.uids.iter().skip(1).filter_map(|u| Uuid::parse_str(u).ok()).filter_map(|u| doc.slot_of(u)).collect();
                if slots.is_empty() { self.warn("Bu bulgunun çizimde kalan kaynak nesnesi yok."); }
                else {
                    let ids: Vec<_> = slots.iter().map(|s| f64::from(s.0)).collect();
                    self.selection.set(slots);
                    self.navigating(|app| app.zoom_to_objects(&ids));
                }
            }
            Event::Repair => {
                if !self.cad_inspection_current() { return Task::none(); }
                let Some(f) = self.cad_inspection.selected.and_then(|i| self.cad_inspection.output.as_ref()?.findings.get(i)).cloned() else { return Task::none(); };
                if f.group == InspectionGroup::Annotations {
                    if let Some(slot) = f.uids.first().and_then(|u| Uuid::parse_str(u).ok()).and_then(|u| self.document.as_ref()?.model.slot_of(u)) {
                        self.selection.set([slot]);
                    }
                    return self.run(if f.code.starts_with("leader_") { "leader.bindTip" } else { "tool.dimensionReassociate" });
                }
                if f.group == InspectionGroup::Hatches {
                    self.memory.hatch_edit_mode = 2;
                    self.memory.hatch_assoc = true;
                    return self.run("tool.hatchEdit");
                }
            }
        }
        Task::none()
    }
    pub(crate) fn cad_inspection_tab(&self) -> Element<'_, Message> {
        let p = &self.cad_inspection;
        let current = self.cad_inspection_current();
        let count = p.output.as_ref().map_or_else(
            || {
                if p.busy {
                    "Denetleniyor…".into()
                } else {
                    "Denetle ile başlayın".into()
                }
            },
            |r| {
                format!(
                    "{} nesne · {} bulgu · {} tarama{}",
                    r.examined,
                    r.findings.len(),
                    r.hatches.len(),
                    if r.truncated { " · Kısmi sonuç" } else { "" }
                )
            },
        );
        let bar = row![
            button(label::body("Denetle"))
                .padding(kentos_ui::theme::metrics::action())
                .style(style::button::primary)
                .on_press_maybe((!p.busy && (p.boundaries || p.hatches || p.annotations)).then(|| msg(Event::Run))),
            button(label::body("Durdur")).padding(kentos_ui::theme::metrics::action())
                .style(style::button::secondary).on_press_maybe(p.busy.then(|| msg(Event::Stop))),
            container(Select::new(SCOPES.map(Choice::new), Some(p.scope), |i| msg(Event::Scope(i)))).width(220),
            label::muted(count)
        ]
        .spacing(8)
        .align_y(iced::Center);
        let options = row![
            crate::exchange::words::check(p.boundaries, "Sınırlar", Some(msg(Event::Boundaries(!p.boundaries)))),
            crate::exchange::words::check(p.hatches, "Taramalar", Some(msg(Event::Hatches(!p.hatches)))),
            crate::exchange::words::check(p.annotations, "Ölçü ve kılavuzlar", Some(msg(Event::Annotations(!p.annotations)))),
            label::muted("Boşluk eşiği"),
            NumberInput::new(p.gap, |n| msg(Event::Gap(n)))
                .units(units::MILLIMETRE)
                .range(0.0..=1000.0)
                .decimals(2)
                .width(120),
            crate::exchange::words::check(p.hidden, "Gizli katmanlar", Some(msg(Event::Hidden(!p.hidden))))
        ]
        .spacing(12)
        .align_y(iced::Center);
        let findings = p
            .output
            .as_ref()
            .map(|o| o.findings.clone())
            .unwrap_or_default();
        let selected = p.selected;
        let columns = [
            TableColumn::new("#").width(Length::Fixed(40.)),
            TableColumn::new("Düzey").width(Length::Fixed(80.)),
            TableColumn::new("Bulgu").width(Length::FillPortion(6)),
            TableColumn::new("Konum / eşik").width(Length::FillPortion(2)),
        ];
        let f = self.format();
        let table = Table::new(columns)
            .virtualized(findings.len(), move |i| {
                let item = &findings[i];
                let severity = match item.severity {
                    kentos_commands::InspectionSeverity::Info => "Bilgi",
                    kentos_commands::InspectionSeverity::Warning => "Uyarı",
                    kentos_commands::InspectionSeverity::Error => "Hata",
                };
                TableRow::new([
                    cell((i + 1).to_string()),
                    cell(severity.into()),
                    cell(item.message.clone()),
                    cell(
                        item.distance
                            .map_or_else(|| format!("{} nesne", item.uids.len()), |n| f.length(n)),
                    ),
                ])
                .selected(selected == Some(i))
                .on_press(msg(Event::Show(i)))
            })
            .empty("Bu kapsamda bulgu yok. Destek kapsamı ve kısmi sonuç notunu da inceleyin.")
            .height(Fill);
        let mut body = column![bar, options].spacing(8);
        if let Some(message) = &p.message {
            body = body.push(label::body(message.clone()));
        }
        if p.output.is_some() && !current {
            body = body.push(label::body(
                "Çizim değişti. Eski bulgularla işlem yapılmaz; yeniden denetleyin.",
            ));
        }
        if let Some(out) = &p.output {
            if out.truncated {
                body = body.push(label::body(
                    "Çalışma veya sonuç sınırına ulaşıldı; denetim kısmi. Kapsamı daraltın.",
                ));
            }
            if !out.unsupported.is_empty() {
                body = body.push(label::muted(format!(
                    "{} blok, eğri veya büyük kaynak sınır denetimi dışında; temiz sayılmadı.",
                    out.unsupported.len()
                )));
            }
            if !out.hatches.is_empty() {
                let bound = out
                    .hatches
                    .iter()
                    .filter(|h| h.state == kentos_commands::HatchLinkState::Associated)
                    .count();
                let broken = out
                    .hatches
                    .iter()
                    .filter(|h| h.state == kentos_commands::HatchLinkState::Broken)
                    .count();
                body=body.push(label::muted(format!("Tarama bağı: {bound} ilişkili · {broken} kaynağı sorunlu · {} ilişkisiz. İlişkisiz tarama sınır değişimini izlemez.",out.hatches.len()-bound-broken)));
            }
        }
        body = body.push(table);
        if let Some(finding) = p.selected.and_then(|i| p.output.as_ref()?.findings.get(i)) {
            body = body.push(label::body(finding.repair.clone())).push(
                row![
                    button(label::body("Konuma yakınlaş")).padding(kentos_ui::theme::metrics::action()).style(style::button::secondary).on_press_maybe(current.then(|| msg(Event::Near))),
                    button(label::body("Kaynağa git")).padding(kentos_ui::theme::metrics::action()).style(style::button::secondary)
                        .on_press_maybe((current && finding.group == InspectionGroup::Annotations && finding.uids.len() > 1).then(|| msg(Event::Sources))),
                    button(label::body(if finding.group == InspectionGroup::Hatches { "Taramayı düzenle…" }
                        else if finding.code.starts_with("leader_") { "Kılavuz ucunu bağla…" } else { "Ölçüyü yeniden bağla…" })).on_press_maybe(
                        (current && finding.group != InspectionGroup::Boundaries && finding.code != "dimension_override")
                            .then(|| msg(Event::Repair))
                    ).padding(kentos_ui::theme::metrics::action()).style(style::button::secondary)
                ]
                .spacing(8),
            );
        }
        container(body).padding(10).height(Fill).into()
    }
    pub(crate) fn cad_inspection_preview(&self) -> Option<Preview> {
        if !self.command_expanded
            || self.bottom_tab != BottomTab::CadInspection
            || !self.cad_inspection_current()
        {
            return None;
        }
        let f = self
            .cad_inspection
            .selected
            .and_then(|i| self.cad_inspection.output.as_ref()?.findings.get(i))?;
        let points: Vec<_> = f.points.iter().map(|p| Vec2::new(p.x, p.y)).collect();
        let markers = points
            .iter()
            .map(|p| Marker {
                at: *p,
                shape: MarkerShape::Ring(7.),
                tone: Tone::Danger,
            })
            .collect();
        let strokes = if points.len() == 2 {
            vec![Stroke::dashed(points.clone(), false, [5., 4.]).tone(Tone::Danger)]
        } else {
            Vec::new()
        };
        Some(Preview {
            markers,
            strokes,
            tag: points.first().map(|p| Tag {
                at: *p,
                lines: vec![finding_title(&f.code).to_owned()],
            }),
            tag_tone: Tone::Danger,
            ..Preview::default()
        })
    }
}

/// The scene keeps a short label; the panel contains the full explanation and repair advice.
fn finding_title(code: &str) -> &'static str {
    match code {
        "boundary_open_end" => "Açık uç",
        "boundary_gap" => "Sınırda açıklık",
        "boundary_duplicate" => "Yinelenen kenar",
        "boundary_self_intersection" => "Kendini kesen sınır",
        "boundary_crossing" => "Kesişen kenarlar",
        "boundary_tangent" => "Teğet temas",
        "hatch_unassociated" => "İlişkisiz tarama",
        "hatch_missing_source" => "Eksik sınır kaynağı",
        "dimension_broken" => "Kopuk ölçü bağı",
        "dimension_partial" => "Kısmen bağlı ölçü",
        "dimension_free" => "Serbest ölçü",
        "dimension_override" => "Ölçü yazısı değişmiş",
        "dimension_invalid" => "Geçersiz ölçü",
        "leader_broken" => "Kopuk kılavuz bağı",
        "leader_free" => "Serbest kılavuz ucu",
        _ => "Sınırı inceleyin",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inspected() -> App {
        let mut app = crate::files_testing::app_with_drawing();
        crate::files_testing::make_cad(&mut app);
        let doc = app.document.as_ref().unwrap();
        app.cad_inspection.session = doc.session;
        app.cad_inspection.generation = doc.model.generation();
        app.cad_inspection.output = Some(serde_json::from_value(serde_json::json!({
            "findings":[{"group":"boundaries","severity":"warning","code":"boundary_open_end",
                "uids":[],"points":[{"x":2.,"y":3.}],"message":"Long explanation","repair":"Inspect source"}],
            "hatches":[],"unsupported":[],"examined":1,"comparisons":1,"truncated":false,
            "revision":doc.model.revision().to_string()
        })).unwrap());
        app.cad_inspection.selected = Some(0);
        app.command_expanded = true;
        app.bottom_tab = BottomTab::CadInspection;
        app
    }

    #[test]
    fn diagnosis_reaches_the_main_scene_but_never_replaces_an_active_tool() {
        let mut app = inspected();
        let preview = app.drawing_preview(&app.format()).expect("main scene diagnosis");
        assert_eq!(preview.markers.len(), 1);
        assert_eq!(preview.tag.unwrap().lines, vec!["Açık uç"]);
        assert!(app.session.start("line"));
        assert!(app.drawing_preview(&app.format()).is_none_or(|p| p.tag.is_none()));
    }

    #[test]
    fn stale_diagnosis_is_hidden_and_cannot_change_selection() {
        let mut app = inspected();
        app.cad_inspection.generation = app.cad_inspection.generation.wrapping_add(1);
        let selected = app.selection.ids().to_vec();
        let _ = app.cad_inspection_event(Event::Show(0));
        assert_eq!(app.selection.ids(), selected);
        assert!(app.cad_inspection_preview().is_none());
        app.cad_inspection.generation = app.document.as_ref().unwrap().model.generation();
        app.command_expanded = false;
        assert!(app.cad_inspection_preview().is_none());
    }
}
