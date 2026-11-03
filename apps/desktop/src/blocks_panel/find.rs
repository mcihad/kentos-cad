//! Recognition settings in a compact dialog; results in the dock so the drawing remains interactive.
use crate::{
    app::{App, Dialog, Message},
    exchange::words::{self, Kind},
    traces::Control,
};
use iced::{
    Element, Fill, Task,
    widget::{Space, button, column, container, responsive, row, scrollable, text, tooltip},
};
use kentos_commands::{BlocksFind, BlocksFound, CommandResult};
use kentos_ui::{
    label, style,
    theme::typography,
    widget::{Dialog as Frame, NumberInput, Tip, VirtualList, number::units, tip},
};
use std::collections::HashSet;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

mod convert;

pub(crate) const TITLE: &str = "Tekrarlanan bileşenleri bul";
static NEXT: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub(super) struct Window {
    source: Vec<String>,
    session: u64,
    generation: u64,
    revision: String,
    rotations: bool,
    mirrors: bool,
    scales: bool,
    strict: bool,
    hidden: bool,
    tolerance: f64,
    /// Editing settings must not silently discard the previous accepted results when cancelled.
    previous_options: Option<(bool, bool, bool, bool, bool, f64)>,
    token: u64,
    cancel: Arc<AtomicBool>,
    busy: bool,
    output: Option<BlocksFound>,
    chosen: Vec<bool>,
    warnings: Vec<String>,
    refused: Option<String>,
    conversion: Option<convert::Window>,
}
impl Drop for Window {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Settings,
    Convert,
    Conversion(convert::Event),
    NewSource,
    Run,
    Stop,
    CloseOptions,
    Back,
    Rotations,
    Mirrors,
    Scales,
    Strict,
    Hidden,
    Tolerance(f64),
    Ready(u64, Box<CommandResult<BlocksFound>>),
    Toggle(usize),
    All,
    Show(usize),
    Select,
}
fn msg(e: Event) -> Message {
    super::msg(super::Event::Find(e))
}
fn tolerance(text: &str) -> Option<f64> {
    kentos_ui::widget::number::evaluate(text, units::MILLIMETRE)
        .ok()
        .filter(|n| *n > 0.0 && *n <= 0.001)
}
impl Window {
    fn current(&self, app: &App) -> bool {
        app.document
            .as_ref()
            .is_some_and(|d| d.session == self.session && d.model.generation() == self.generation)
    }
    fn changed(&mut self) {
        self.output = None;
        self.chosen.clear();
        self.warnings.clear();
        self.refused = None;
    }
    fn input(&self, doc: &kentos_domain::Document) -> Option<BlocksFind> {
        if !self.tolerance.is_finite() || self.tolerance <= 0.0 || self.tolerance > 0.001 {
            return None;
        }
        Some(BlocksFind {
            source: self.source.clone(),
            uids: (!self.hidden).then(|| {
                doc.entities_with_uids()
                    .filter(|(_, e)| {
                        doc.layers().is_visible(&e.base().layer_id)
                            && !doc.layers().is_frozen(&e.base().layer_id)
                    })
                    .map(|(u, _)| u.to_string())
                    .collect()
            }),
            rotations: Some(self.rotations),
            mirrors: Some(self.mirrors),
            scales: Some(self.scales),
            geometry_only: Some(!self.strict),
            tolerance: Some(self.tolerance),
            limit: Some(1000),
            expected_revision: Some(self.revision.clone()),
        })
    }
}

impl App {
    pub(crate) fn close_block_find(&mut self) {
        self.blocks_panel.find = None;
        if self.dialog == Some(Dialog::BlockFind) {
            self.dialog = None;
        }
    }
    pub(crate) fn open_block_find(&mut self) {
        self.show_blocks_panel();
        if self
            .blocks_panel
            .find
            .as_ref()
            .is_some_and(|w| w.current(self))
        {
            let w = self.blocks_panel.find.as_mut().unwrap();
            if !w.busy {
                if w.output.is_some() && w.previous_options.is_none() {
                    w.previous_options = Some((
                        w.rotations,
                        w.mirrors,
                        w.scales,
                        w.strict,
                        w.hidden,
                        w.tolerance,
                    ));
                }
                self.dialog = Some(Dialog::BlockFind);
            }
            return;
        }
        let Some(doc) = &self.document else {
            return;
        };
        self.blocks_panel.find = Some(Window {
            source: self.uids_of(self.selection.ids()),
            session: doc.session,
            generation: doc.model.generation(),
            revision: doc.model.revision().to_string(),
            rotations: true,
            mirrors: false,
            scales: false,
            strict: true,
            hidden: false,
            tolerance: 0.000_001,
            previous_options: None,
            token: 0,
            cancel: Arc::new(AtomicBool::new(false)),
            busy: false,
            output: None,
            chosen: Vec::new(),
            warnings: Vec::new(),
            refused: None,
            conversion: None,
        });
        self.dialog = Some(Dialog::BlockFind);
    }
    pub(crate) fn block_find_close_options(&mut self) {
        if let Some(w) = self.blocks_panel.find.as_mut() { w.conversion = None; }
        self.dialog = None;
        if let Some(w) = self.blocks_panel.find.as_mut()
            && let Some((rotations, mirrors, scales, strict, hidden, tolerance)) =
                w.previous_options.take()
        {
            w.rotations = rotations;
            w.mirrors = mirrors;
            w.scales = scales;
            w.strict = strict;
            w.hidden = hidden;
            w.tolerance = tolerance;
        }
        if self
            .blocks_panel
            .find
            .as_ref()
            .is_some_and(|w| !w.busy && w.output.is_none())
        {
            self.blocks_panel.find = None;
        }
    }
    fn block_find_start(&mut self) -> Task<Message> {
        let Some(w) = self.blocks_panel.find.as_ref() else {
            return Task::none();
        };
        if !w.current(self) {
            if let Some(w) = self.blocks_panel.find.as_mut() {
                w.refused =
                    Some("Çizim değişti. Yeni kaynak seçip aramayı yeniden başlatın.".into());
            }
            return Task::none();
        }
        let Some(doc) = self.document.as_ref() else {
            return Task::none();
        };
        let Some(input) = w.input(&doc.model) else {
            return Task::none();
        };
        if w.source.is_empty() || w.source.len() > kentos_commands::RECOGNITION_SOURCE_MAX || w.busy
        {
            return Task::none();
        }
        let model = doc.model.clone();
        let w = self.blocks_panel.find.as_mut().unwrap();
        w.cancel.store(true, Ordering::Relaxed);
        w.cancel = Arc::new(AtomicBool::new(false));
        w.token = NEXT.fetch_add(1, Ordering::Relaxed);
        w.changed();
        w.busy = true;
        let token = w.token;
        w.previous_options = None;
        let cancel = Arc::clone(&w.cancel);
        self.dialog = None;
        let echo = self.echo_query(
            kentos_commands::CAD_BLOCKS_FIND,
            &serde_json::to_value(&input).expect("find input"),
        );
        Task::batch([
            echo,
            crate::db_connections::off_thread(
                move || {
                    kentos_native_application::blocks_find::inspect(&model, &input, || {
                        cancel.load(Ordering::Relaxed)
                    })
                },
                move |answer| msg(Event::Ready(token, Box::new(answer))),
            ),
        ])
    }
    pub(super) fn block_find_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Convert => return self.block_convert_open(),
            Event::Conversion(e) => return self.block_convert_event(e),
            Event::Back => self.close_block_find(),
            Event::CloseOptions => self.block_find_close_options(),
            Event::NewSource => {
                self.blocks_panel.find = None;
                self.open_block_find();
            }
            Event::Settings => self.open_block_find(),
            Event::Run => return self.block_find_start(),
            Event::Stop => {
                if let Some(w) = self.blocks_panel.find.as_mut() {
                    w.cancel.store(true, Ordering::Relaxed);
                    w.token = NEXT.fetch_add(1, Ordering::Relaxed);
                    w.busy = false;
                    w.refused = Some("Arama durduruldu; çizim değiştirilmedi.".into());
                }
            }
            Event::Ready(token, answer) => {
                let current = self
                    .blocks_panel
                    .find
                    .as_ref()
                    .is_some_and(|w| w.current(self));
                if let Some(w) = self
                    .blocks_panel
                    .find
                    .as_mut()
                    .filter(|w| w.token == token && !w.cancel.load(Ordering::Relaxed))
                {
                    w.busy = false;
                    if !current {
                        w.refused=Some("Arama sırasında çizim değişti; eski sonuç kullanılmadı. Yeni kaynak seçin.".into());
                        return Task::none();
                    }
                    match *answer {
                        CommandResult::Completed { output, warnings } => {
                            w.chosen = vec![true; output.matches.len()];
                            w.output = Some(output);
                            w.warnings = warnings.into_iter().map(|w| w.message).collect();
                        }
                        CommandResult::Failed { error }
                        | CommandResult::Conflict { error }
                        | CommandResult::NeedsInput { error } => w.refused = Some(error.message),
                        CommandResult::Cancelled => {
                            w.refused = Some("Arama durduruldu; çizim değiştirilmedi.".into())
                        }
                        _ => w.refused = Some("Arama tamamlanmadı; yeniden deneyin.".into()),
                    }
                }
            }
            Event::Toggle(i) => {
                if let Some(w) = self.blocks_panel.find.as_mut()
                    && let Some(v) = w.chosen.get_mut(i)
                {
                    *v = !*v;
                }
            }
            Event::All => {
                if let Some(w) = self.blocks_panel.find.as_mut() {
                    let on = !w.chosen.iter().all(|v| *v);
                    w.chosen.fill(on);
                }
            }
            Event::Show(i) => {
                let Some(w) = self.blocks_panel.find.as_ref().filter(|w| w.current(self)) else {
                    return Task::none();
                };
                let Some(group) = w.output.as_ref().and_then(|o| o.matches.get(i)) else {
                    return Task::none();
                };
                let Some(doc) = &self.document else {
                    return Task::none();
                };
                let slots: Vec<_> = group
                    .members
                    .iter()
                    .filter_map(|u| kentos_domain::Uuid::parse_str(u).ok())
                    .filter_map(|u| doc.model.slot_of(u))
                    .collect();
                self.selection.set(slots);
                self.zoom_selection();
            }
            Event::Select => {
                let Some(w) = self.blocks_panel.find.as_ref().filter(|w| w.current(self)) else {
                    return Task::none();
                };
                let (Some(output), Some(doc)) = (&w.output, &self.document) else {
                    return Task::none();
                };
                let mut seen = HashSet::new();
                let slots: Vec<_> = output
                    .matches
                    .iter()
                    .zip(&w.chosen)
                    .filter(|(_, on)| **on)
                    .flat_map(|(g, _)| &g.members)
                    .filter_map(|u| kentos_domain::Uuid::parse_str(u).ok())
                    .filter(|u| seen.insert(*u))
                    .filter_map(|u| doc.model.slot_of(u))
                    .collect();
                self.selection.set(slots);
                self.zoom_selection();
            }
            option => {
                if let Some(w) = self.blocks_panel.find.as_mut().filter(|w| !w.busy) {
                    match option {
                        Event::Rotations => w.rotations = !w.rotations,
                        Event::Mirrors => w.mirrors = !w.mirrors,
                        Event::Scales => w.scales = !w.scales,
                        Event::Strict => w.strict = !w.strict,
                        Event::Hidden => w.hidden = !w.hidden,
                        Event::Tolerance(t) => w.tolerance = t,
                        _ => return Task::none(),
                    }
                    w.refused = None;
                }
            }
        }
        Task::none()
    }
    pub(crate) fn block_find_view(&self) -> Element<'_, Message> {
        if self.blocks_panel.find.as_ref().is_some_and(|w| w.conversion.is_some()) { return self.block_convert_view(); }
        let Some(w) = &self.blocks_panel.find else {
            return text("").into();
        };
        let mut body=column![
            words::text_line(Kind::Info,format!("Kaynak: {} nesne. Tek bir bileşenin çizgi, 2B çoklu çizgi, çember ve yaylarını kullanın. Arama çizimi değiştirmez.",w.source.len())),
            words::check(w.rotations,"Döndürülmüş eşleri de bul",Some(msg(Event::Rotations))),
            words::check(w.mirrors,"Aynalanmış eşleri de bul",Some(msg(Event::Mirrors))),
            words::check(w.scales,"Farklı boyuttaki ölçekli eşleri de bul",Some(msg(Event::Scales))),
            words::check(w.strict,"Katman, görünüş ve öznitelikler de aynı olsun",Some(msg(Event::Strict))),
            words::check(w.hidden,"Gizli ve donmuş katmanları da ara",Some(msg(Event::Hidden))),
            words::field("Geometri toleransı (mm)",NumberInput::new(w.tolerance,|t| msg(Event::Tolerance(t))).units(units::MILLIMETRE).range(0.0..=0.001).step(0.001).decimals(3),Some("Sıfırdan büyük, en çok 1 mm. Varsayılan 0,001 mm; çizim boyutlarını değiştirmez.".into())),
            label::caption("Sonuçlar Bloklar panelinde açılır; çizimde adayları ayrı ayrı inceleyip seçebilirsiniz.")
        ].spacing(12);
        if let Some(why) = &w.refused {
            body = body.push(words::text_line(Kind::Error, why.clone()));
        }
        if w.source.len() > kentos_commands::RECOGNITION_SOURCE_MAX {
            body = body.push(words::text_line(
                Kind::Warn,
                "Kaynak en çok 128 nesne olabilir. Tek bir bileşen seçip yeniden açın.",
            ));
        }
        if !w.current(self) {
            body = body.push(words::text_line(
                Kind::Warn,
                "Çizim değişti. Pencereyi kapatıp yeni kaynak seçin.",
            ));
        }
        Frame::new(TITLE)
            .scroll(body)
            .action(words::secondary("Vazgeç", Some(msg(Event::CloseOptions))))
            .action(words::primary(
                "Bul",
                (!w.source.is_empty()
                    && w.source.len() <= kentos_commands::RECOGNITION_SOURCE_MAX
                    && !w.busy
                    && w.current(self)
                    && w.tolerance.is_finite()
                    && w.tolerance > 0.0
                    && w.tolerance <= 0.001)
                    .then(|| msg(Event::Run)),
            ))
            .width(620.0)
            .max_height(650.0)
            .modal(msg(Event::CloseOptions))
    }
    pub(crate) fn block_find_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(w) = &self.blocks_panel.find else {
            return Ok(None);
        };
        if w.conversion.is_some() { return self.block_convert_control(control); }
        let e = match control {
            Control::Press("Vazgeç") => Event::CloseOptions,
            Control::Press("Bul")
                if !w.busy
                    && w.current(self)
                    && !w.source.is_empty()
                    && w.source.len() <= kentos_commands::RECOGNITION_SOURCE_MAX
                    && w.tolerance.is_finite()
                    && w.tolerance > 0.0
                    && w.tolerance <= 0.001 =>
            {
                Event::Run
            }
            Control::Press("Bul") => return Ok(None),
            Control::Fill("Geometri toleransı (mm)", value) => Event::Tolerance(
                tolerance(value).ok_or("Tolerans sıfırdan büyük ve en çok 1 mm olmalı.")?,
            ),
            Control::Check(title, on) => {
                let (now, event) = match title {
                    "Döndürülmüş eşleri de bul" => (w.rotations, Event::Rotations),
                    "Aynalanmış eşleri de bul" => (w.mirrors, Event::Mirrors),
                    "Farklı boyuttaki ölçekli eşleri de bul" => (w.scales, Event::Scales),
                    "Katman, görünüş ve öznitelikler de aynı olsun" => {
                        (w.strict, Event::Strict)
                    }
                    "Gizli ve donmuş katmanları da ara" => (w.hidden, Event::Hidden),
                    _ => return Err(format!("{TITLE}: {control} yok")),
                };
                if now == on {
                    return Ok(None);
                }
                event
            }
            _ => return Err(format!("{TITLE}: {control} yok")),
        };
        Ok(Some(msg(e)))
    }
}

pub(super) fn panel(app: &App) -> Element<'_, Message> {
    responsive(move |size| panel_at_height(app, size.height)).into()
}

fn scroll_direction() -> scrollable::Direction {
    scrollable::Direction::Vertical(
        scrollable::Scrollbar::new()
            .width(4)
            .scroller_width(4)
            .spacing(4),
    )
}

fn panel_at_height(app: &App, height: f32) -> Element<'_, Message> {
    let Some(w) = &app.blocks_panel.find else {
        return text("").into();
    };
    let current = w.current(app);
    let chosen = w.chosen.iter().filter(|v| **v).count();
    let header = row![
        label::body(w.output.as_ref().map_or_else(
            || "Tekrarlanan bileşenler".to_owned(),
            |o| {
                if !current {
                    "Sonuç güncel değil".to_owned()
                } else if o.truncated {
                    format!("Kısmi: {} eşleşme", o.matches.len())
                } else {
                    format!("{} eşleşme · Kaynak: {}", o.matches.len(), w.source.len())
                }
            }
        ))
        .width(Fill),
        words::secondary("Bloklara dön", Some(msg(Event::Back)))
    ]
    .spacing(8)
    .align_y(iced::Center);
    let controls = if let Some(output) = &w.output {
        row![
            tip(
                words::check(
                    w.chosen.iter().all(|v| *v),
                    "Tümü",
                    (!output.matches.is_empty()).then(|| msg(Event::All))
                ),
                Tip::new(format!(
                    "{} adayın {chosen} tanesi seçili",
                    output.matches.len()
                )),
                tooltip::Position::Top
            ),
            Space::new().width(Fill),
            words::secondary("Yeni kaynak", Some(msg(Event::NewSource))),
            words::secondary("Ayarlar", Some(msg(Event::Settings)))
        ]
        .spacing(6)
        .align_y(iced::Center)
    } else {
        row![
            words::secondary("Yeni kaynak", (!w.busy).then(|| msg(Event::NewSource))),
            words::secondary("Ayarlar", (!w.busy).then(|| msg(Event::Settings))),
            label::caption(format!("Kaynak: {} nesne", w.source.len()))
        ]
        .spacing(6)
        .align_y(iced::Center)
    };
    let mut top = column![header, controls].spacing(6);
    if !current {
        top = top.push(words::text_line(
            Kind::Warn,
            "Çizim değişti; eski sonuçlar seçilemez. Yeni kaynak seçin.",
        ));
    }
    if let Some(why) = &w.refused {
        top = top.push(words::text_line(Kind::Warn, why.clone()));
    }
    if w.busy {
        return container(
            column![
                scrollable(top.push(label::body(
                    "Eşleşmeler aranıyor… Çizimi kullanmaya devam edebilirsiniz."
                )))
                .height(Fill)
                .direction(scroll_direction()),
                words::secondary("Durdur", Some(msg(Event::Stop)))
            ]
            .spacing(8)
            .height(Fill),
        )
        .padding(8)
        .width(Fill)
        .height(Fill)
        .into();
    }
    let Some(output) = &w.output else {
        return container(
            scrollable(top.push(label::caption(
                "Bir bileşen seçip Yeni kaynak ile aramaya başlayın.",
            )))
            .height(Fill)
            .direction(scroll_direction()),
        )
        .padding(8)
        .width(Fill)
        .height(Fill)
        .into();
    };
    for warning in &w.warnings {
        top = top.push(words::text_line(Kind::Warn, warning.clone()));
    }
    if !output.related.is_empty() {
        top = top.push(label::caption(format!(
            "Bağlı nesneler: {}. Bağlar değiştirilmedi.",
            output
                .related
                .iter()
                .map(|(k, n)| format!("{n} {}", crate::view::kind_name(k)))
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    if output.matches.is_empty() {
        top = top.push(label::body("Eşleşme bulunamadı."));
    }
    let list = VirtualList::new(output.matches.len(), typography::scaled(48.0), move |i| {
        let g = &output.matches[i];
        let detail = format!(
            "{} nesne · {:.1}° · ×{:.3}{}{}",
            g.members.len(),
            g.rotation.to_degrees(),
            g.scale,
            if g.mirror { " · Ayna" } else { "" },
            if g.overlaps { " · Ortak nesne" } else { "" }
        );
        let choose = column![
            words::check(
                w.chosen[i],
                format!("Eşleşme {}", i + 1),
                Some(msg(Event::Toggle(i)))
            ),
            label::caption(detail)
        ]
        .spacing(3)
        .width(Fill);
        container(
            row![
                choose,
                button(label::caption("Göster"))
                    .on_press_maybe(current.then(|| msg(Event::Show(i))))
                    .style(style::button::ghost)
            ]
            .spacing(4)
            .align_y(iced::Center),
        )
        .padding([4, 0])
        .into()
    })
    .height(Fill);
    // The dock can be much shorter than the window: keep a complete row and the action
    // visible. Long warnings scroll above them instead of clipping a candidate's controls.
    let head = container(
        scrollable(top)
            .height(iced::Shrink)
            .direction(scroll_direction()),
    )
    .max_height((height - typography::scaled(48.0 + 28.0) - 32.0).max(0.0));
    container(
        column![
            head,
            list,
            row![
                button(label::body(format!("Göster ({chosen})")))
                    .on_press_maybe((current && chosen > 0).then(|| msg(Event::Select)))
                    .padding(kentos_ui::theme::metrics::action())
                    .style(style::button::secondary),
                words::primary("Bloğa dönüştür…", (current && chosen > 0).then(|| msg(Event::Convert)))
            ].spacing(6).align_y(iced::Center)
        ]
        .spacing(8)
        .height(Fill),
    )
    .padding(8)
    .width(Fill)
    .height(Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_domain::Slot;

    pub(super) fn test_app() -> App {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../fixtures/commands/v1/cad.blocks.find.json"
        ))
        .unwrap();
        let model = kentos_domain::Document::from_snapshot(
            kentos_commands::DocumentSnapshotV1::from_json(&fixture["setup"].to_string()).unwrap(),
        )
        .unwrap();
        let mut app = crate::files_testing::app_with_drawing();
        app.document =
            Some(crate::document::Document::from_v2(model.to_snapshot_v2(), None).unwrap());
        crate::files_testing::make_cad(&mut app);
        app.selection.set([Slot(1)]);
        app
    }
    pub(super) fn complete(app: &mut App) {
        let _ = app.block_find_event(Event::Run);
        assert_eq!(app.dialog, None, "results leave the canvas interactive");
        let w = app.blocks_panel.find.as_ref().unwrap();
        let token = w.token;
        let input = w.input(&app.document.as_ref().unwrap().model).unwrap();
        let answer = kentos_native_application::blocks_find::inspect(
            &app.document.as_ref().unwrap().model,
            &input,
            || false,
        );
        let _ = app.block_find_event(Event::Ready(token, Box::new(answer)));
    }

    #[test]
    fn query_options_results_selection_and_cancel_never_change_the_drawing() {
        let mut app = test_app();
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        app.open_block_find();
        assert_eq!(app.dialog, Some(Dialog::BlockFind));
        let _ = app.block_find_event(Event::Tolerance(tolerance("0,001").unwrap()));
        complete(&mut app);
        assert_eq!(
            app.blocks_panel
                .find
                .as_ref()
                .unwrap()
                .output
                .as_ref()
                .unwrap()
                .matches
                .len(),
            2
        );
        assert_eq!(app.selection.ids(), [Slot(1)]);
        let _ = app.block_find_event(Event::Show(1));
        assert_eq!(app.selection.ids(), [Slot(2)]);
        let _ = app.block_find_event(Event::Toggle(0));
        let _ = app.block_find_event(Event::Select);
        assert_eq!(app.selection.ids(), [Slot(2)]);
        let _ = app.block_find_event(Event::Settings);
        assert_eq!(app.dialog, Some(Dialog::BlockFind));
        let _ = app.block_find_event(Event::Mirrors);
        assert!(app.blocks_panel.find.as_ref().unwrap().mirrors);
        app.close_dialog();
        assert!(!app.blocks_panel.find.as_ref().unwrap().mirrors);
        assert_eq!(
            app.blocks_panel
                .find
                .as_ref()
                .unwrap()
                .output
                .as_ref()
                .unwrap()
                .matches
                .len(),
            2
        );
        assert!(
            app.blocks_panel.find.is_some(),
            "closing options keeps accepted results"
        );
        let _ = app.block_find_event(Event::Back);
        assert!(app.blocks_panel.find.is_none());
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
    }

    #[test]
    fn cancelled_or_stale_background_results_cannot_replace_results_or_select_new_document_objects()
    {
        let mut app = test_app();
        app.open_block_find();
        let _ = app.block_find_event(Event::Run);
        let w = app.blocks_panel.find.as_ref().unwrap();
        let old = w.token;
        let cancel = Arc::clone(&w.cancel);
        let _ = app.block_find_event(Event::Stop);
        assert!(cancel.load(Ordering::Relaxed));
        let _ = app.block_find_event(Event::Ready(
            old,
            Box::new(CommandResult::Completed {
                output: BlocksFound::default(),
                warnings: Vec::new(),
            }),
        ));
        assert!(app.blocks_panel.find.as_ref().unwrap().output.is_none());
        complete(&mut app);
        let replacement = test_app();
        app.document = replacement.document;
        app.selection.set([Slot(4)]);
        let _ = app.block_find_event(Event::Select);
        assert_eq!(app.selection.ids(), [Slot(4)]);
        let _ = app.block_find_event(Event::Show(0));
        assert_eq!(app.selection.ids(), [Slot(4)]);
    }

    #[test]
    fn tolerance_uses_the_shared_unit_expression_parser_and_empty_source_cannot_start() {
        assert_eq!(tolerance("0,001"), Some(1e-6));
        assert_eq!(tolerance("0.001 mm"), Some(1e-6));
        assert_eq!(tolerance("0.000001 m"), Some(1e-6));
        assert_eq!(tolerance("0.05 mm + 0.05 mm"), Some(0.0001));
        assert_eq!(tolerance("0"), None);
        assert_eq!(tolerance("2 mm"), None);
        assert_eq!(tolerance("bad"), None);
        let mut app = test_app();
        app.open_block_find();
        let _ = app.block_find_event(Event::Tolerance(0.0));
        assert!(
            app.block_find_control(Control::Press("Bul"))
                .unwrap()
                .is_none()
        );
        app.close_dialog();
        app.selection.clear();
        app.open_block_find();
        assert!(
            app.block_find_control(Control::Press("Bul"))
                .unwrap()
                .is_none()
        );
        app.close_dialog();
        assert!(app.blocks_panel.find.is_none());
    }

    #[test]
    fn queries_echo_only_when_enabled_and_closing_the_dock_cancels_pending_work() {
        use kentos_ui::widget::python::EntryKind;
        let mut app = test_app();
        app.open_block_find();
        complete(&mut app);
        assert!(!app.python.repl.entries().any(|e| e.kind == EntryKind::Echo));
        assert!(
            app.settings
                .choose(&[(crate::python::echo::SETTING, serde_json::json!(true))])
                .is_empty()
        );
        let _ = app.block_find_event(Event::Run);
        let echoed: Vec<_> = app
            .python
            .repl
            .entries()
            .filter(|e| e.kind == EntryKind::Echo)
            .collect();
        assert_eq!(echoed.len(), 1);
        assert!(echoed[0].text.contains("cad.blocks.find"));
        let cancel = Arc::clone(&app.blocks_panel.find.as_ref().unwrap().cancel);
        let _ = app.update(Message::Dock(kentos_ui::widget::docking::Event::Closed(
            crate::app::Panel::Blocks,
        )));
        assert!(cancel.load(Ordering::Relaxed));
        assert!(app.blocks_panel.find.is_none());
    }
}
