//! Başvurular (docs/adr/0345 §3, §5; `arch.viewRefs`): the drawing's view marks and the targets they may point at, as
//! `cad.viewmarks.references` lists them. A mark is chosen in the list; Bağla points it at a target, Bağı kopar lets it
//! go, Görünüm oluştur keeps a detail's circle as a named view and points the detail at it, all through
//! `cad.viewmarks.edit` and `cad.views.edit`. Göster brings a mark into view, Git shows a named view.

use iced::widget::{Column, button, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    Entity, ViewMarkForm, ViewMarksOperation, ViewMarksReferenced, ViewTarget, ViewsOp,
};
use kentos_ui::icon::icon;
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;
use kentos_interaction::Level;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Başvurular";
const CLOSE: &str = "Kapat";
const SHOW: &str = "Göster";
const BIND: &str = "Bağla";
const DETACH: &str = "Bağı kopar";
const MAKE_VIEW: &str = "Görünüm oluştur";
const GO: &str = "Git";

#[derive(Clone, Debug)]
pub struct Window {
    pub refs: ViewMarksReferenced,
    /// The mark chosen in the list, by its place.
    pub chosen: Option<usize>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Choose(usize),
    Show(usize),
    Bind(usize),
    Detach,
    MakeView,
    Go(usize),
    Close,
}

fn msg(e: Event) -> Message {
    Message::ViewRefs(e)
}

/// A mark's icon by its form.
fn icon_of(form: ViewMarkForm) -> &'static str {
    match form {
        ViewMarkForm::Section => "viewSection",
        ViewMarkForm::Elevation => "viewElevation",
        ViewMarkForm::Detail => "viewDetail",
    }
}

impl App {
    fn view_refs_read(&self) -> Option<ViewMarksReferenced> {
        let doc = self.document.as_ref()?;
        Some(kentos_native_application::viewmarks::references_of(
            &doc.model,
        ))
    }

    /// Başvurular (`arch.viewRefs`): the window, the selected mark chosen.
    pub(crate) fn open_view_refs(&mut self) {
        let Some(refs) = self.view_refs_read() else {
            self.output("Açık çizim yok.");
            return;
        };
        let selected: Vec<String> = self
            .document
            .as_ref()
            .map(|d| {
                self.selection
                    .ids()
                    .iter()
                    .filter_map(|s| d.model.uid(*s))
                    .map(|u| u.to_string())
                    .collect()
            })
            .unwrap_or_default();
        let chosen = refs.marks.iter().position(|m| selected.contains(&m.uid));
        self.view_refs = Some(Window { refs, chosen });
        self.dialog = Some(Dialog::ViewRefs);
    }

    fn view_refs_slot(&self, uid: &str) -> Option<kentos_domain::Slot> {
        let doc = self.document.as_ref()?;
        doc.model.slot_of(kentos_domain::Uuid::parse_str(uid).ok()?)
    }

    /// `cad.viewmarks.edit` on the chosen mark, and the list again.
    fn view_refs_edit(&mut self, op: ViewMarksOperation) {
        let Some(uid) = self
            .view_refs
            .as_ref()
            .and_then(|w| w.chosen.and_then(|k| w.refs.marks.get(k)))
            .map(|m| m.uid.clone())
        else {
            self.warn("Önce listeden bir işaret seçin.");
            return;
        };
        let Some(slot) = self.view_refs_slot(&uid) else {
            return;
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        for said in kentos_interaction::properties::edit_viewmark(&mut doc.model, slot, op) {
            self.warn(said);
        }
        self.view_refs_again();
    }

    fn view_refs_again(&mut self) {
        let refs = self.view_refs_read();
        if let (Some(w), Some(refs)) = (self.view_refs.as_mut(), refs) {
            w.refs = refs;
        }
    }

    pub(crate) fn view_refs_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Choose(k) => {
                if let Some(w) = self.view_refs.as_mut() {
                    w.chosen = Some(k);
                }
            }
            Event::Show(k) => {
                let uid = self
                    .view_refs
                    .as_ref()
                    .and_then(|w| w.refs.marks.get(k))
                    .map(|m| m.uid.clone());
                if let Some(slot) = uid.and_then(|u| self.view_refs_slot(&u)) {
                    self.zoom_to_objects(&[f64::from(slot.0)]);
                }
                if let Some(w) = self.view_refs.as_mut() {
                    w.chosen = Some(k);
                }
            }
            Event::Bind(t) => {
                let target = self
                    .view_refs
                    .as_ref()
                    .and_then(|w| w.refs.targets.get(t))
                    .map(|t| t.target.clone());
                if let Some(target) = target {
                    self.view_refs_edit(ViewMarksOperation::Retarget { target });
                }
            }
            Event::Detach => self.view_refs_edit(ViewMarksOperation::Detach),
            Event::MakeView => self.view_refs_make_view(),
            Event::Go(t) => {
                let name = self
                    .view_refs
                    .as_ref()
                    .and_then(|w| w.refs.targets.get(t))
                    .filter(|t| t.target.view.is_some())
                    .map(|t| t.name.clone());
                if let Some(name) = name {
                    self.show_named_view(&name);
                }
            }
            Event::Close => {
                self.view_refs = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    /// Görünüm oluştur: the chosen detail's circle kept as a named view (“Detay 1”, its number), the detail pointed at it.
    fn view_refs_make_view(&mut self) {
        let Some(mark) = self
            .view_refs
            .as_ref()
            .and_then(|w| w.chosen.and_then(|k| w.refs.marks.get(k)))
            .cloned()
        else {
            self.warn("Önce listeden bir detay işareti seçin.");
            return;
        };
        let radius = self
            .view_refs_slot(&mark.uid)
            .and_then(|s| self.document.as_ref()?.model.get(s).cloned())
            .and_then(|e| match e {
                Entity::ViewMark(m) if m.mark.form == ViewMarkForm::Detail => m.mark.radius,
                _ => None,
            });
        let Some(r) = radius else {
            self.warn("Görünüm yalnız detay işaretinin sınırından oluşturulur.");
            return;
        };
        let base = format!(
            "Detay {}",
            if mark.mark.is_empty() {
                "?"
            } else {
                &mark.mark
            }
        );
        let mut name = base.clone();
        let mut k = 2;
        while self.named_views().iter().any(|v| v.name == name) {
            name = format!("{base} ({k})");
            k += 1;
        }
        let Some(done) = self.edit_views(ViewsOp::Save {
            name: name.clone(),
            center: mark.at,
            width: 2.4 * r,
            height: 2.4 * r,
            twist: None,
        }) else {
            return;
        };
        let Some(view) = done.view else { return };
        self.view_refs_edit(ViewMarksOperation::Retarget {
            target: ViewTarget::view(&kentos_commands::view_key(&view)),
        });
        self.say(
            Level::Success,
            format!("“{name}” görünümü oluşturuldu ve detay ona bağlandı."),
        );
    }

    pub(crate) fn view_refs_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.view_refs else {
            return iced::widget::text("").into();
        };
        let broken = w.refs.marks.iter().filter(|m| m.broken).count();
        let summary = words::summary(vec![words::text_line(
            if broken > 0 { Kind::Warn } else { Kind::Ok },
            if broken > 0 {
                format!(
                    "{} işaret, {} hedef; {broken} işaretin hedefi yok.",
                    w.refs.marks.len(),
                    w.refs.targets.len()
                )
            } else {
                format!(
                    "{} işaret, {} hedef.",
                    w.refs.marks.len(),
                    w.refs.targets.len()
                )
            },
        )]);
        let small = |text: &'static str, on: Option<Message>| {
            button(label::body(text))
                .on_press_maybe(on)
                .padding(kentos_ui::theme::metrics::action())
                .style(style::button::secondary)
        };
        let mut marks = Column::new().spacing(6).push(label::strong("İşaretler"));
        if w.refs.marks.is_empty() {
            marks = marks.push(label::caption("Çizimde görünüm işareti yok: Mimari › Görünüm'den Kesit, Görünüş ya da Detay çizin."));
        }
        for (k, m) in w.refs.marks.iter().enumerate() {
            let target = w
                .refs
                .targets
                .iter()
                .find(|t| Some(&t.target) == m.target.as_ref())
                .map_or_else(
                    || {
                        if m.target.is_some() {
                            "hedefi yok".to_owned()
                        } else {
                            "hedefsiz".to_owned()
                        }
                    },
                    |t| t.name.clone(),
                );
            let title = format!(
                "{} {} → {}",
                m.form.label(),
                if m.mark.is_empty() {
                    "(sıra)"
                } else {
                    &m.mark
                },
                target
            );
            let text = Column::new()
                .spacing(2)
                .push(label::body(title))
                .push(label::caption(format!(
                    "Balon: {} / {}",
                    m.words.top, m.words.bottom
                )));
            let chosen = w.chosen == Some(k);
            let mut line = row![
                icon(crate::icons::from_web(Some(if m.broken {
                    "warning"
                } else {
                    icon_of(m.form)
                })))
                .size(18.0),
                container(text).width(Fill),
                small(SHOW, Some(msg(Event::Show(k)))),
            ]
            .spacing(10)
            .align_y(Center);
            if chosen {
                line = line.push(small(DETACH, m.target.as_ref().map(|_| msg(Event::Detach))));
                if m.form == ViewMarkForm::Detail {
                    line = line.push(small(MAKE_VIEW, Some(msg(Event::MakeView))));
                }
            }
            marks = marks.push(
                button(line)
                    .on_press(msg(Event::Choose(k)))
                    .padding([6, 10])
                    .width(Fill)
                    .style(style::button::row(chosen)),
            );
        }
        let mut targets = Column::new().spacing(6).push(label::strong("Hedefler"));
        if w.refs.targets.is_empty() {
            targets = targets.push(label::caption(
                "Adlı görünüm ya da paftada görünüm penceresi yok: Görünüm › Görünümler'den kaydedin ya da pafta kipinde harita çerçevesi ekleyin.",
            ));
        }
        let chosen = w.chosen.is_some();
        for (k, t) in w.refs.targets.iter().enumerate() {
            let kind = if t.target.view.is_some() {
                "Adlı görünüm"
            } else {
                "Pafta penceresi"
            };
            let text = Column::new()
                .spacing(2)
                .push(label::body(t.name.clone()))
                .push(label::caption(format!(
                    "{kind}; numarası {}{}; {} işaret",
                    t.number,
                    t.order
                        .map_or_else(String::new, |o| format!(", sırası {o}")),
                    t.marks.len()
                )));
            let mut line = row![
                icon(crate::icons::from_web(Some(if t.target.view.is_some() {
                    "namedViews"
                } else {
                    "sheet"
                })))
                .size(18.0),
                container(text).width(Fill),
            ]
            .spacing(10)
            .align_y(Center);
            if t.target.view.is_some() {
                line = line.push(small(GO, Some(msg(Event::Go(k)))));
            }
            line = line.push(small(BIND, chosen.then(|| msg(Event::Bind(k)))));
            targets = targets.push(
                container(line)
                    .padding([6, 10])
                    .style(style::container::field_box)
                    .width(Fill),
            );
        }
        let body = Column::new().spacing(12).push(summary).push(
            iced::widget::scrollable(Column::new().spacing(14).push(marks).push(targets))
                .height(iced::Length::Shrink)
                .height(400.0),
        );
        Frame::new(TITLE)
            .push(body)
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(760.0)
            .modal(msg(Event::Close))
    }

    /// The window's buttons by their words (a trace's `dialog` step): the first mark's and target's.
    pub(crate) fn view_refs_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        if self.view_refs.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SHOW) => Some(msg(Event::Show(0))),
            Control::Press(BIND) => Some(msg(Event::Bind(0))),
            Control::Press(DETACH) => Some(msg(Event::Detach)),
            Control::Press(MAKE_VIEW) => Some(msg(Event::MakeView)),
            Control::Press(GO) => Some(msg(Event::Go(0))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
