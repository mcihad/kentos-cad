//! Çok kollu kılavuz on the desktop (docs/adr/0326 §5), each by `cad.leaders.edit`: Kol ekle (`leader.addArm`: the
//! selected leader's new arm's tip picked on the drawing, bound to the place a snap gives there), Kol sil
//! (`leader.removeArm`: the arm whose tip is nearest the point picked), Hizala (`leader.align`: a window, the selected
//! leaders in the order they were selected, the first staying), Topla (`leader.collect`), Blok içerik
//! (`leader.blocks`: the selected leader takes the selected inserts as its content) and Kılavuz stilleri
//! (`leader.styles`: the project's leader styles by `cad.styles.edit`). Öznitelikler's leader rows send their style and
//! Bağı çöz here.

use std::fmt;

use iced::widget::{Column, button, container, row, text_input};
use iced::{Element, Fill, Task};
use kentos_commands::{
    AnnotationStyleKind, CommandResult, Entity, LeaderArrow, LeaderEdit, LeaderStyleDef,
    LeadersEdit, SnapFind, StyleEditOperation, StylesEdit, SubRef, Vec2,
};
use kentos_domain::{Slot, Uuid};
use kentos_interaction::Level;
use kentos_interaction::pick::PickPoint;
use kentos_native_application::{ExecutionContext, leaders_edit, snap_find, styles_edit};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog as Frame, Segmented, focus_ring, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};

/// The windows' titles, which a trace names them by.
pub const ALIGN_TITLE: &str = "Kılavuzları hizala";
pub const STYLES_TITLE: &str = "Kılavuz stilleri";

/// The arrowheads a leader style offers, the filled one first.
pub const ARROWS: [(Option<LeaderArrow>, &str); 6] = [
    (None, "Dolu ok"),
    (Some(LeaderArrow::Open), "Açık ok"),
    (Some(LeaderArrow::Closed), "Boş üçgen"),
    (Some(LeaderArrow::Dot), "Dolu nokta"),
    (Some(LeaderArrow::DotBlank), "Boş nokta"),
    (Some(LeaderArrow::None), "Yok"),
];

/// Hizala's direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Way {
    Across,
    Down,
    Angle,
}

impl fmt::Display for Way {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Way::Across => "Yatay",
            Way::Down => "Dikey",
            Way::Angle => "Açıyla",
        })
    }
}

/// What a point picked is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Picking {
    AddArm(Slot),
    RemoveArm(Slot),
}

/// Hizala's window.
#[derive(Debug, Clone)]
pub struct AlignWindow {
    leaders: Vec<Slot>,
    way: Way,
    angle: String,
    spacing: String,
}

/// Kılavuz stilleri's window: the table as a draft, the chosen style.
#[derive(Debug, Clone)]
pub struct StylesWindow {
    styles: Vec<LeaderStyleDef>,
    chosen: Option<usize>,
    height: String,
    size: String,
}

#[derive(Debug, Clone, Default)]
pub struct LeaderTools {
    picking: Option<Picking>,
    align: Option<AlignWindow>,
    styles: Option<StylesWindow>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Way(Way),
    Angle(String),
    Spacing(String),
    Align,
    /// Öznitelikler: the leaders follow a style (none: no style).
    Style(Vec<Slot>, Option<String>),
    /// Öznitelikler: arm `arm`'s tip let go of its place.
    Unbind(Slot, u32),
    Choose(usize),
    New,
    Remove,
    Name(String),
    Height(String),
    Arrow(usize),
    Size(String),
    Mask(bool),
    Save,
    Close,
}

fn msg(event: Event) -> Message {
    Message::LeaderTools(event)
}

/// What a command answered, as the app says it; whether it was done.
fn answered<T>(app: &mut App, result: &CommandResult<T>) -> bool {
    match result {
        CommandResult::Completed { warnings, .. } => {
            for w in warnings {
                app.warn(w.message.clone());
            }
            true
        }
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => {
            app.warn(error.message.clone());
            false
        }
        CommandResult::Queued { .. } | CommandResult::Cancelled => false,
    }
}

impl App {
    fn selected_of(&self, kind: &str) -> Vec<Slot> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        self.selection
            .ids()
            .iter()
            .copied()
            .filter(|s| doc.model.get(*s).is_some_and(|e| e.kind() == kind))
            .collect()
    }

    fn leaders_edit(&mut self, edit: LeaderEdit, done: &str) -> bool {
        let Some(doc) = self.document.as_mut() else {
            return false;
        };
        let result = leaders_edit::execute(
            &mut ExecutionContext::new(&mut doc.model),
            LeadersEdit {
                edit,
                expected_revision: None,
            },
        );
        let ok = answered(self, &result);
        if ok {
            self.say(Level::Success, done.to_owned());
        }
        ok
    }

    /// The leader commands by their ids; false for any other.
    pub(crate) fn leader_tools_run(&mut self, id: &str) -> bool {
        match id {
            "leader.addArm" | "leader.removeArm" => {
                let [leader] = self.selected_of("leader")[..] else {
                    self.warn("Önce bir kılavuz seçin.");
                    return true;
                };
                let (picking, field) = if id == "leader.addArm" {
                    (Picking::AddArm(leader), "Yeni kolun ucu")
                } else {
                    (Picking::RemoveArm(leader), "Silinecek kolun ucu")
                };
                self.leader_tools.picking = Some(picking);
                self.say(Level::Command, format!("Kılavuz: {}", field.to_lowercase()));
                self.field = None;
                self.snap = None;
                self.session.run(Box::new(PickPoint::new("", field)));
                self.with_tool(|s, cx| s.activate(cx));
            }
            "leader.align" => {
                let leaders = self.selected_of("leader");
                if leaders.len() < 2 {
                    self.warn("Hizalamak için en az iki kılavuz seçin; ilk seçilen yerinde kalır.");
                    return true;
                }
                self.leader_tools.align = Some(AlignWindow {
                    leaders,
                    way: Way::Across,
                    angle: "0".into(),
                    spacing: String::new(),
                });
                self.dialog = Some(Dialog::LeaderAlign);
            }
            "leader.collect" => {
                let uids = self.uids_of(&self.selected_of("leader"));
                if uids.len() < 2 {
                    self.warn("Toplamak için en az iki blok içerikli kılavuz seçin; ilk seçilende toplanır.");
                    return true;
                }
                let _ = self.leaders_edit(LeaderEdit::Collect { uids }, "Kılavuzlar toplandı.");
            }
            "leader.blocks" => {
                let [leader] = self.selected_of("leader")[..] else {
                    self.warn("Bir kılavuz ve içeriği olacak blok yerleştirmelerini seçin.");
                    return true;
                };
                let uid = self.uids_of(&[leader]).pop().unwrap_or_default();
                let blocks = self.uids_of(&self.selected_of("insert"));
                let _ = self.leaders_edit(
                    LeaderEdit::Blocks { uid, blocks },
                    "Kılavuzun içeriği yazıldı.",
                );
            }
            "leader.styles" => {
                let styles = self
                    .document
                    .as_ref()
                    .map_or_else(Vec::new, |d| d.model.settings().leader_styles.clone());
                let mut w = StylesWindow {
                    styles,
                    chosen: None,
                    height: String::new(),
                    size: String::new(),
                };
                if !w.styles.is_empty() {
                    w.choose(0);
                }
                self.leader_tools.styles = Some(w);
                self.dialog = Some(Dialog::LeaderStyles);
            }
            _ => return false,
        }
        true
    }

    /// The point Kol ekle or Kol sil asked for (input.rs); false when none asked.
    pub(crate) fn leader_tools_picked(&mut self, p: Option<kentos_geometry_core::Vec2>) -> bool {
        let Some(picking) = self.leader_tools.picking.take() else {
            return false;
        };
        let Some(p) = p.map(|p| Vec2 { x: p.x, y: p.y }) else {
            return true;
        };
        match picking {
            Picking::AddArm(leader) => {
                let uid = self.uids_of(&[leader]).pop().unwrap_or_default();
                // The place a snap gives at the point, on another object: the tip follows it.
                let aperture = 10.0 / self.viewport.camera.scale.max(1e-9);
                let tip = self.document.as_ref().and_then(|d| {
                    let input = SnapFind {
                        at: p,
                        aperture,
                        kinds: None,
                        from: None,
                        all: false,
                    };
                    match snap_find::answer(&d.model, Some(self.spatial.store()), &input) {
                        CommandResult::Completed { output, .. } => {
                            output.snaps.into_iter().find_map(|s| {
                                let u = Uuid::parse_str(s.uid.as_deref()?).ok()?;
                                (u.to_string() != uid).then_some(SubRef {
                                    uid: kentos_commands::EntityId(*u.as_bytes()),
                                    place: s.place?,
                                    path: Vec::new(),
                                    broken: None,
                                })
                            })
                        }
                        _ => None,
                    }
                });
                let bound = tip.is_some();
                let _ = self.leaders_edit(
                    LeaderEdit::AddArm {
                        uid,
                        pts: vec![p],
                        tip,
                    },
                    if bound {
                        "Kol eklendi; ucu nesneye bağlı."
                    } else {
                        "Kol eklendi."
                    },
                );
            }
            Picking::RemoveArm(leader) => {
                let Some(Entity::Leader(l)) = self
                    .document
                    .as_ref()
                    .and_then(|d| d.model.get(leader))
                    .cloned()
                else {
                    return true;
                };
                let tips = std::iter::once(l.pts[0]).chain(l.arms.iter().map(|a| a.pts[0]));
                let nearest = tips
                    .enumerate()
                    .map(|(i, q)| (i, (q.x - p.x).hypot(q.y - p.y)))
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .map_or(0, |(i, _)| i);
                let uid = self.uids_of(&[leader]).pop().unwrap_or_default();
                let _ = self.leaders_edit(
                    LeaderEdit::RemoveArm {
                        uid,
                        arm: nearest as u32,
                    },
                    "Kol silindi.",
                );
            }
        }
        true
    }

    fn align_input(&self) -> Result<LeaderEdit, String> {
        let w = self.leader_tools.align.as_ref().ok_or_else(String::new)?;
        let angle = match w.way {
            Way::Across => 0.0,
            Way::Down => 90.0,
            Way::Angle => w
                .angle
                .trim()
                .replace(',', ".")
                .parse::<f64>()
                .map_err(|_| "Açı bir sayı olmalı.".to_owned())?,
        };
        let spacing = match w.spacing.trim() {
            "" => None,
            s => Some(
                s.replace(',', ".")
                    .parse::<f64>()
                    .map_err(|_| "Aralık bir sayı olmalı (boş: izdüşümüyle).".to_owned())?,
            ),
        };
        Ok(LeaderEdit::Align {
            uids: self.uids_of(&w.leaders),
            angle,
            spacing,
        })
    }

    pub(crate) fn leader_tools_event(&mut self, event: Event) -> Task<Message> {
        if let Some(w) = self.leader_tools.align.as_mut() {
            match &event {
                Event::Way(v) => w.way = *v,
                Event::Angle(t) => w.angle.clone_from(t),
                Event::Spacing(t) => w.spacing.clone_from(t),
                _ => {}
            }
        }
        if let Some(w) = self.leader_tools.styles.as_mut() {
            match &event {
                Event::Choose(i) => w.choose(*i),
                Event::New => w.add(),
                Event::Remove => w.remove(),
                Event::Name(t) => {
                    if let Some(s) = w.current() {
                        s.name.clone_from(t);
                    }
                }
                Event::Height(t) => {
                    w.height.clone_from(t);
                    let h = t.trim().replace(',', ".").parse::<f64>().ok();
                    if let (Some(s), Some(h)) = (w.current(), h) {
                        s.height = h;
                    }
                }
                Event::Arrow(i) => {
                    if let Some(s) = w.current() {
                        s.arrow = ARROWS.get(*i).and_then(|(a, _)| *a);
                    }
                }
                Event::Size(t) => {
                    w.size.clone_from(t);
                    let k = t.trim().replace(',', ".").parse::<f64>().ok();
                    if let Some(s) = w.current() {
                        s.arrow_size = k.filter(|k| *k != 1.0);
                    }
                }
                Event::Mask(on) => {
                    if let Some(s) = w.current() {
                        s.mask = *on;
                    }
                }
                _ => {}
            }
        }
        match event {
            Event::Align => match self.align_input() {
                Ok(edit) => {
                    if self.leaders_edit(edit, "Kılavuzlar hizalandı.") {
                        self.leader_tools.align = None;
                        self.dialog = None;
                    }
                }
                Err(why) => self.warn(why),
            },
            Event::Style(slots, style) => {
                let uids = self.uids_of(&slots);
                let _ =
                    self.leaders_edit(LeaderEdit::Style { uids, style }, "Kılavuz stili yazıldı.");
            }
            Event::Unbind(slot, arm) => {
                let uid = self.uids_of(&[slot]).pop().unwrap_or_default();
                let _ = self.leaders_edit(
                    LeaderEdit::Bind {
                        uid,
                        arm,
                        tip: None,
                    },
                    "Kolun ucu serbest.",
                );
            }
            Event::Save => self.leader_styles_save(),
            Event::Close => {
                self.leader_tools = LeaderTools::default();
                self.dialog = None;
            }
            _ => {}
        }
        Task::none()
    }

    /// Kaydet: the table's styles by `cad.styles.edit` (created, updated, removed), the leaders that follow them in
    /// the same steps.
    fn leader_styles_save(&mut self) {
        let Some(w) = self.leader_tools.styles.clone() else {
            return;
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let before = doc.model.settings().leader_styles.clone();
        let mut inputs: Vec<StylesEdit> = Vec::new();
        let edit = |operation, leader: Option<LeaderStyleDef>, style: Option<String>| StylesEdit {
            wall: None,
            kind: AnnotationStyleKind::Leader,
            operation,
            style,
            name: None,
            text: None,
            dimension: None,
            leader,
            styles: None,
            library: None,
            replace: None,
            expected_revision: None,
        };
        for old in &before {
            if !w.styles.iter().any(|s| s.id == old.id) {
                inputs.push(edit(StyleEditOperation::Remove, None, Some(old.id.clone())));
            }
        }
        for s in &w.styles {
            match before.iter().find(|o| o.id == s.id) {
                Some(o) if o == s => {}
                Some(_) => inputs.push(edit(StyleEditOperation::Update, Some(s.clone()), None)),
                None => inputs.push(edit(
                    StyleEditOperation::Create,
                    Some(LeaderStyleDef {
                        id: String::new(),
                        ..s.clone()
                    }),
                    None,
                )),
            }
        }
        let mut said = Vec::new();
        for input in inputs {
            if let CommandResult::Failed { error } =
                styles_edit::execute(&mut ExecutionContext::new(&mut doc.model), input)
            {
                said.push(error.message);
                break;
            }
        }
        if said.is_empty() {
            self.say(Level::Success, "Kılavuz stilleri kaydedildi.");
            self.leader_tools.styles = None;
            self.dialog = None;
        }
        for s in said {
            self.warn(s);
        }
    }

    pub(crate) fn leader_align_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.leader_tools.align else {
            return label::caption("").into();
        };
        let field = |placeholder: &'static str, value: &str, on: fn(String) -> Event| {
            focus_ring(
                text_input(placeholder, value)
                    .on_input(move |t| msg(on(t)))
                    .padding([5, 8])
                    .width(120)
                    .style(style::field::input),
            )
        };
        let mut options = row![words::field(
            "Doğrultu",
            Segmented::new([Way::Across, Way::Down, Way::Angle], w.way, |v| msg(
                Event::Way(v)
            )),
            None
        )]
        .spacing(18)
        .align_y(iced::Bottom);
        if w.way == Way::Angle {
            options = options.push(words::field(
                "Açı (°)",
                field("0", &w.angle, Event::Angle),
                None,
            ));
        }
        options = options.push(words::field(
            "Aralık (m)",
            field("İzdüşümüyle", &w.spacing, Event::Spacing),
            None,
        ));
        let mut lines = vec![words::text_line(
            Kind::Info,
            format!(
                "Seçili {} kılavuzun ilki yerinde kalır; diğerlerinin notları onun hizasına iner. Uçlar yerinde kalır.",
                w.leaders.len()
            ),
        )];
        if let Err(why) = self.align_input() {
            lines.push(words::text_line(Kind::Warn, why));
        }
        overlay::modal(
            Frame::new(ALIGN_TITLE)
                .push(
                    Column::new()
                        .spacing(12)
                        .push(words::summary(lines))
                        .push(options),
                )
                .action(words::secondary("Vazgeç", Some(msg(Event::Close))))
                .action(words::primary(
                    "Hizala",
                    self.align_input().is_ok().then(|| msg(Event::Align)),
                ))
                .width(560.0),
            msg(Event::Close),
        )
    }

    pub(crate) fn leader_styles_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.leader_tools.styles else {
            return label::caption("").into();
        };
        let mut list = Column::new().spacing(2);
        for (i, s) in w.styles.iter().enumerate() {
            let chosen = w.chosen == Some(i);
            list = list.push(
                button(label::body(s.name.clone()))
                    .on_press(msg(Event::Choose(i)))
                    .width(Fill)
                    .padding([5, 8])
                    .style(style::button::row(chosen)),
            );
        }
        if w.styles.is_empty() {
            list = list.push(label::caption("Kılavuz stili yok. Yeni ile ekleyin."));
        }
        let left = Column::new()
            .spacing(8)
            .width(200)
            .push(
                container(list)
                    .style(style::container::field_box)
                    .height(240)
                    .width(Fill),
            )
            .push(
                row![
                    words::secondary("Yeni", Some(msg(Event::New))),
                    words::secondary("Sil", w.chosen.map(|_| msg(Event::Remove)))
                ]
                .spacing(8),
            );
        let form: Element<'_, Message> = match w.chosen.and_then(|i| w.styles.get(i)) {
            Some(s) => {
                let input = |placeholder: &'static str, value: String, on: fn(String) -> Event| {
                    focus_ring(
                        text_input(placeholder, &value)
                            .on_input(move |t| msg(on(t)))
                            .padding([5, 8])
                            .width(200)
                            .style(style::field::input),
                    )
                };
                let arrows: Vec<Choice> = ARROWS.iter().map(|(_, n)| Choice::new(*n)).collect();
                let at = ARROWS.iter().position(|(a, _)| *a == s.arrow);
                Column::new()
                    .spacing(10)
                    .push(words::field(
                        "Ad",
                        input("Kılavuz", s.name.clone(), Event::Name),
                        None,
                    ))
                    .push(words::field(
                        "Notun yüksekliği (mm)",
                        input("2,5", w.height.clone(), Event::Height),
                        None,
                    ))
                    .push(words::field(
                        "Ok başı",
                        Select::new(arrows, at, |i| msg(Event::Arrow(i))),
                        None,
                    ))
                    .push(words::field(
                        "Ok boyu (× yükseklik)",
                        input("1", w.size.clone(), Event::Size),
                        None,
                    ))
                    .push(words::check(
                        s.mask,
                        "Notun zemini",
                        Some(msg(Event::Mask(!s.mask))),
                    ))
                    .into()
            }
            None => label::caption("Bir stil seçin.").into(),
        };
        let problem = kentos_commands::leader_styles_problem(&w.styles);
        let mut body = Column::new()
            .spacing(12)
            .push(row![left, form].spacing(18).align_y(iced::Top));
        if let Some(p) = &problem {
            body = body.push(words::summary(vec![words::text_line(
                Kind::Warn,
                p.clone(),
            )]));
        }
        overlay::modal(
            Frame::new(STYLES_TITLE)
                .push(body)
                .action(words::secondary("Vazgeç", Some(msg(Event::Close))))
                .action(words::primary(
                    "Kaydet",
                    problem.is_none().then(|| msg(Event::Save)),
                ))
                .width(520.0),
            msg(Event::Close),
        )
    }
}

impl StylesWindow {
    fn current(&mut self) -> Option<&mut LeaderStyleDef> {
        self.chosen.and_then(|i| self.styles.get_mut(i))
    }

    fn choose(&mut self, i: usize) {
        self.chosen = Some(i).filter(|i| *i < self.styles.len());
        if let Some(s) = self.chosen.and_then(|i| self.styles.get(i)) {
            self.height = crate::crs::js_number(s.height);
            self.size = s
                .arrow_size
                .map_or_else(String::new, crate::crs::js_number);
        }
    }

    fn add(&mut self) {
        let n = self.styles.len() + 1;
        let id = Uuid::now_v7().to_string();
        self.styles.push(LeaderStyleDef {
            id,
            name: format!("Kılavuz {n}"),
            height: 2.5,
            arrow: None,
            arrow_size: None,
            mask: false,
        });
        self.choose(self.styles.len() - 1);
    }

    fn remove(&mut self) {
        if let Some(i) = self.chosen {
            self.styles.remove(i);
            self.chosen = None;
            if !self.styles.is_empty() {
                self.choose(i.min(self.styles.len() - 1));
            }
        }
    }
}
