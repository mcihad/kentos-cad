//! Alt ağlar (docs/adr/0557 §2; plan/gis.md CBS-27): a window with a utility network's subnetworks (`cad.subnetworks.find`,
//! read again after every write) and what Güncelle would write (`cad.subnetworks.update`'s plan). A subnetwork is what a
//! source (a controller) reaches by the network's rules; the table says each one's name, controllers, objects and length;
//! under it what no controller reaches, what two reach, and whether the attribute the network names for them is up to
//! date. A row selects its objects; Güncelle writes the names in one undo step and the window reads again.

use iced::widget::{Column, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    CommandResult, NetworkDef, SubnetworkObjects, SubnetworksFind, SubnetworksFound,
    SubnetworksUpdate, SubnetworksUpdated,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, subnetworks, subnetworks_update};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog as Frame, Elided, VirtualList};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const SUBNETWORKS_TITLE: &str = "Alt ağlar";
const UPDATE: &str = "Güncelle";
const NETWORKS: &str = "Ağlar…";
const CLOSE: &str = "Kapat";
const ROW: f32 = 26.0;

/// What a row of the window's table or its lines under it selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    Subnetwork(usize),
    Orphans,
    Shared,
}

#[derive(Debug, Clone)]
pub struct Window {
    /// The network's id.
    network: String,
    report: Result<SubnetworksFound, String>,
    preview: Result<SubnetworksUpdated, String>,
    chosen: Option<Pick>,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// The network chosen from the list (an index among those with subnetworks).
    Network(usize),
    Pick(Pick),
    Update,
    /// Ağlar…: the networks' window, where the subnetworks are defined.
    Networks,
    Close,
}

fn msg(event: Event) -> Message {
    Message::Subnetworks(event)
}

/// The project's networks that define subnetworks.
fn defined(app: &App) -> Vec<NetworkDef> {
    app.document.as_ref().map_or_else(Vec::new, |d| {
        d.model
            .settings()
            .networks
            .iter()
            .filter(|n| n.subnetworks.is_some())
            .cloned()
            .collect()
    })
}

fn find_input(network: &str) -> SubnetworksFind {
    SubnetworksFind {
        network: network.to_owned(),
        members: true,
    }
}

fn update_input(network: &str) -> SubnetworksUpdate {
    SubnetworksUpdate {
        network: network.to_owned(),
        expected_revision: None,
    }
}

fn failure<T>(result: CommandResult<T>, fallback: &str) -> Result<T, String> {
    match result {
        CommandResult::Completed { output, .. } => Ok(output),
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => Err(error.message),
        _ => Err(fallback.to_owned()),
    }
}

impl App {
    /// Alt ağlar (`network.subnetworks`): the window on the first network that defines subnetworks.
    pub(crate) fn open_subnetworks(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        let network = defined(self)
            .first()
            .map(|n| n.id.clone())
            .unwrap_or_default();
        self.subnetworks_window = Some(Window {
            network,
            report: Err(String::new()),
            preview: Err(String::new()),
            chosen: None,
        });
        self.dialog = Some(Dialog::Subnetworks);
        self.refresh_subnetworks();
    }

    /// The subnetworks and the preview read again.
    fn refresh_subnetworks(&mut self) {
        let (Some(w), Some(doc)) = (&self.subnetworks_window, &self.document) else {
            return;
        };
        let (report, preview) = if w.network.is_empty() {
            (Err(String::new()), Err(String::new()))
        } else {
            let report = failure(
                subnetworks::run(&doc.model, &find_input(&w.network)),
                "Alt ağlar bulunamadı.",
            );
            let preview = if let Some(locked) = self.tree_locked() {
                Err(locked.to_owned())
            } else {
                failure(
                    subnetworks_update::plan_in(&doc.model, &update_input(&w.network)),
                    "Güncelleme planlanamadı.",
                )
            };
            (report, preview)
        };
        if let Some(w) = self.subnetworks_window.as_mut() {
            let count = report.as_ref().map_or(0, |r| r.subnetworks.len());
            w.chosen = w.chosen.filter(|p| match p {
                Pick::Subnetwork(i) => *i < count,
                _ => true,
            });
            w.report = report;
            w.preview = preview;
        }
    }

    pub(crate) fn subnetworks_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.subnetworks_window.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Network(k) => {
                if let Some(n) = defined(self).get(k) {
                    let id = n.id.clone();
                    if let Some(w) = self.subnetworks_window.as_mut() {
                        w.network = id;
                        w.chosen = None;
                    }
                }
            }
            Event::Pick(p) => {
                w.chosen = Some(p);
                self.show_subnetwork(p);
                return Task::none();
            }
            Event::Update => {
                self.update_subnetworks();
            }
            Event::Networks => {
                self.subnetworks_window = None;
                self.dialog = None;
                return self.open_networks();
            }
            Event::Close => {
                self.subnetworks_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        self.refresh_subnetworks();
        Task::none()
    }

    /// A row's objects selected and brought into view.
    fn show_subnetwork(&mut self, pick: Pick) {
        let Some(Ok(r)) = self.subnetworks_window.as_ref().map(|w| w.report.clone()) else {
            return;
        };
        let uids: Vec<String> = match pick {
            Pick::Subnetwork(i) => r
                .subnetworks
                .get(i)
                .map_or_else(Vec::new, |s| s.members.clone()),
            Pick::Orphans => r.orphans.uids.clone(),
            Pick::Shared => r.shared.uids.clone(),
        };
        let slots: Vec<_> = self.document.as_ref().map_or_else(Vec::new, |d| {
            uids.iter()
                .filter_map(|u| kentos_domain::Uuid::parse_str(u).ok())
                .filter_map(|u| d.model.slot_of(u))
                .collect()
        });
        if !slots.is_empty() {
            let ids: Vec<f64> = slots.iter().map(|s| f64::from(s.0)).collect();
            self.selection.set(slots);
            self.zoom_to_objects(&ids);
        }
    }

    /// Writes the subnetworks' names; whether it wrote (what it did said).
    pub(crate) fn update_subnetworks(&mut self) -> bool {
        let Some(input) = self
            .subnetworks_window
            .as_ref()
            .map(|w| update_input(&w.network))
        else {
            return false;
        };
        if let Some(locked) = self.tree_locked() {
            self.warn(locked);
            return false;
        }
        let Some(doc) = self.document.as_mut() else {
            return false;
        };
        let result = subnetworks_update::execute(&mut ExecutionContext::new(&mut doc.model), input);
        let Some(done) = self.answered(result) else {
            return false;
        };
        if done.written + done.cleared == 0 {
            self.say(
                Level::Info,
                "Alt ağ adları zaten güncel; değişen olmadı.".to_owned(),
            );
        } else {
            self.say(Level::Success, updated_words(&done));
        }
        true
    }

    pub(crate) fn subnetworks_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.subnetworks_window else {
            return iced::widget::text("").into();
        };
        let format = self.format();
        let listed = defined(self);
        let at = listed.iter().position(|n| n.id == w.network);
        let network: Element<'_, Message> = if listed.is_empty() {
            row![
                container(label::muted(
                    "Alt ağı tanımlı ağ yok. Ağlar penceresinde ağın Alt ağlar bölümünü (ulaşım, öznitelik) yazın."
                ))
                .width(Fill),
                words::secondary(NETWORKS, Some(msg(Event::Networks))),
            ]
            .spacing(8)
            .align_y(Center)
            .into()
        } else {
            Select::new(
                listed.iter().map(|n| Choice::new(n.name.clone())),
                at,
                |k| msg(Event::Network(k)),
            )
            .into()
        };
        let (lines, report): (Vec<Element<'_, Message>>, Option<&SubnetworksFound>) =
            match &w.report {
                Err(why) if why.is_empty() => (Vec::new(), None),
                Err(why) => (vec![words::text_line(Kind::Warn, why.clone())], None),
                Ok(r) => (vec![words::text_line(Kind::Info, report_words(r))], Some(r)),
            };
        let chosen = w.chosen;
        let rows: Vec<(String, String, String, String, String, bool)> = report
            .map(|r| {
                r.subnetworks
                    .iter()
                    .map(|s| {
                        (
                            s.name.clone(),
                            s.controllers.len().to_string(),
                            s.edges.to_string(),
                            s.junctions.to_string(),
                            format.length(s.length),
                            s.unverified > 0,
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        let head = kentos_ui::widget::table::band(
            row![
                container(kentos_ui::widget::table::title("Alt ağ")).width(Fill),
                container(kentos_ui::widget::table::title("Denetleyici"))
                    .width(typography::scaled(90.0)),
                container(kentos_ui::widget::table::title("Kenar")).width(typography::scaled(60.0)),
                container(kentos_ui::widget::table::title("Düğüm")).width(typography::scaled(60.0)),
                container(kentos_ui::widget::table::title("Uzunluk"))
                    .width(typography::scaled(110.0)),
                container(kentos_ui::widget::table::title("")).width(typography::scaled(24.0)),
            ]
            .spacing(8)
            .padding([0, 8]),
        );
        let count = rows.len();
        let list = VirtualList::new(count, typography::scaled(ROW), move |i| {
            let (name, controllers, edges, junctions, length, unverified) = &rows[i];
            let name_cell = Elided::new(name.clone())
                .size(typography::caption())
                .font(typography::ui())
                .style(|theme: &iced::Theme| iced::widget::text::Style {
                    color: Some(Tokens::of(theme).text),
                })
                .width(Fill);
            let mark: Element<'_, Message> = if *unverified {
                icon(Icon::Warning).size(12.0).tone(Tone::Danger).into()
            } else {
                icon(Icon::Check).size(12.0).tone(Tone::Success).into()
            };
            let face = row![
                name_cell,
                container(label::caption(controllers.clone())).width(typography::scaled(90.0)),
                container(label::caption(edges.clone())).width(typography::scaled(60.0)),
                container(label::caption(junctions.clone())).width(typography::scaled(60.0)),
                container(label::caption(length.clone())).width(typography::scaled(110.0)),
                container(mark)
                    .width(typography::scaled(24.0))
                    .align_x(Center),
            ]
            .spacing(8)
            .padding([0, 8])
            .height(typography::scaled(ROW))
            .align_y(Center);
            iced::widget::button(face)
                .on_press(msg(Event::Pick(Pick::Subnetwork(i))))
                .padding(0)
                .width(Fill)
                .style(style::button::row(chosen == Some(Pick::Subnetwork(i))))
                .into()
        })
        .height(typography::scaled(156.0));
        let table = container(Column::new().push(head).push(list))
            .style(style::container::field_box)
            .width(Fill);
        let mut under = Column::new().spacing(4);
        if let Some(r) = report {
            under = under.push(self.objects_row(
                "Hiçbir denetleyicinin ulaşmadığı",
                &r.orphans,
                Pick::Orphans,
                chosen,
            ));
            under = under.push(self.objects_row(
                "Birden çok alt ağın ulaştığı",
                &r.shared,
                Pick::Shared,
                chosen,
            ));
        }
        let (preview, ready) = match &w.preview {
            Err(why) if why.is_empty() => (Vec::new(), false),
            Err(why) => (vec![words::text_line(Kind::Warn, why.clone())], false),
            Ok(p) => {
                let mut lines = vec![words::text_line(
                    if p.written + p.cleared > 0 {
                        Kind::Info
                    } else {
                        Kind::Ok
                    },
                    planned_words(p),
                )];
                if p.skipped > 0 {
                    lines.push(words::text_line(
                        Kind::Warn,
                        format!(
                            "{} nesneye yazılmaz (kilitli katman, öznitelik alanı olmayan katman ya da alanın kuralına uymayan ad).",
                            p.skipped
                        ),
                    ));
                }
                (lines, p.written + p.cleared > 0)
            }
        };
        let hint = report.map_or_else(String::new, |r| {
            format!(
                "Satıra basınca nesneleri seçilip gösterilir. Öznitelik: “{}”; ulaşım: {}.",
                r.attribute,
                match r.reach {
                    kentos_commands::SubnetworkReach::Downstream => "akış aşağı",
                    kentos_commands::SubnetworkReach::Connected => "bağlı (her yön)",
                }
            )
        });
        let body = Column::new()
            .spacing(12)
            .push(words::field("Ağ", network, None))
            .push(words::summary(lines))
            .push(words::field("Alt ağlar", table, Some(hint)))
            .push(under)
            .push(words::summary(preview));
        Frame::new(SUBNETWORKS_TITLE)
            .push(body)
            .action(words::secondary(CLOSE, Some(msg(Event::Close))))
            .action(words::primary(UPDATE, ready.then(|| msg(Event::Update))))
            .width(720.0)
            .modal(msg(Event::Close))
    }

    /// A line under the table: how many objects, pressed to select them.
    fn objects_row<'a>(
        &self,
        words_: &'a str,
        objects: &SubnetworkObjects,
        pick: Pick,
        chosen: Option<Pick>,
    ) -> Element<'a, Message> {
        let face = row![
            container(label::caption(words_)).width(Fill),
            label::caption(objects.count.to_string()),
        ]
        .spacing(8)
        .padding([2, 8])
        .align_y(Center);
        let press = (objects.count > 0).then(|| msg(Event::Pick(pick)));
        iced::widget::button(face)
            .on_press_maybe(press)
            .padding(0)
            .width(Fill)
            .style(style::button::row(chosen == Some(pick)))
            .into()
    }

    /// The window's report (tests read it).
    #[cfg(test)]
    pub(crate) fn subnetworks_report(&self) -> Option<&SubnetworksFound> {
        self.subnetworks_window.as_ref()?.report.as_ref().ok()
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn subnetworks_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        if self.subnetworks_window.is_none() {
            return Err(format!("{SUBNETWORKS_TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(UPDATE) => Some(msg(Event::Update)),
            Control::Press(NETWORKS) => Some(msg(Event::Networks)),
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            other => return Err(format!("“{SUBNETWORKS_TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The report in a sentence.
fn report_words(r: &SubnetworksFound) -> String {
    let stale = r.stale.changed.count + r.stale.missing.count + r.stale.left_over.count;
    let verdict = if stale == 0 {
        format!("“{}” güncel", r.attribute)
    } else {
        format!(
            "“{}” eskimiş: {} farklı, {} eksik, {} artakalan değerli nesne",
            r.attribute, r.stale.changed.count, r.stale.missing.count, r.stale.left_over.count
        )
    };
    let unverified: u32 = r.subnetworks.iter().map(|s| s.unverified).sum();
    let tail = if unverified > 0 {
        format!("; {unverified} bağlantı sorunundan geçiyor (Ağlar › Denetle), güvenilir değil")
    } else {
        String::new()
    };
    format!(
        "{} denetleyici, {} alt ağ; {verdict}{tail}.",
        r.controllers,
        r.subnetworks.len()
    )
}

/// What the plan would do, in a sentence.
fn planned_words(p: &SubnetworksUpdated) -> String {
    if p.written + p.cleared == 0 {
        return "Güncelle'nin yazacağı bir şey yok: alt ağ adları güncel.".to_owned();
    }
    let mut parts = Vec::new();
    if p.written > 0 {
        parts.push(format!("{} nesneye alt ağ adı yazılır", p.written));
    }
    if p.cleared > 0 {
        parts.push(format!("{} nesnenin değeri alınır", p.cleared));
    }
    format!("Güncelle: {}. Tek adımda geri alınır.", parts.join(", "))
}

/// What the write did, in a sentence.
fn updated_words(p: &SubnetworksUpdated) -> String {
    let mut parts = Vec::new();
    if p.written > 0 {
        parts.push(format!("{} nesneye alt ağ adı yazıldı", p.written));
    }
    if p.cleared > 0 {
        parts.push(format!("{} nesnenin değeri alındı", p.cleared));
    }
    if p.skipped > 0 {
        parts.push(format!("{} nesneye yazılmadı", p.skipped));
    }
    format!("Alt ağlar güncellendi: {}.", parts.join(", "))
}
