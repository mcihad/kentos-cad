//! Yol projesi (plan/road.md Y13.3, docs/adr/0911 §4; ROAD-MOD-04): the road project's tree in the dock, as
//! `road.project.query` reads it (Civil 3D's Prospector, OpenRoads' Explorer › OpenRoads Model), drawn by KentOS UI's
//! status tree: a row a source or a drawing under its parent, its state a pill (up to date is quiet), its reasons the
//! row's tip. A source's row opens its editor; a drawing's row selects its objects and shows them, its ↻ draws it again
//! (`road.project.rebuild`). Above: Kurarak denetle (the deep reading: surfaces and corridors made, approved volumes
//! computed) and Kendiliğinden yeniden üret (`road.project.edit`); the tree's head: the counts that filter, Hepsini
//! yeniden üret. The tree is read again only when the drawing's revision or the depth changes.

use std::cell::RefCell;
use std::collections::HashSet;

use iced::widget::{column, container};
use iced::{Element, Task};
use kentos_commands::{RoadNode, RoadNodeKind as N, RoadNodeState as S, RoadProjectEdit, RoadProjectRebuild, RoadProjectTree};
use kentos_domain::{Slot, Uuid};
use kentos_native_application::{ExecutionContext, road_project_edit, road_project_query, road_project_rebuild};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::widget::tree_view::Check;
use kentos_ui::widget::{Status, StatusItem, StatusTree};

use kentos_interaction::Level;

use crate::app::{App, Message};

#[derive(Clone, Debug)]
pub enum Event {
    /// A row, by its place in the tree.
    Open(usize),
    /// Draw one drawing again (by id; `edited` also when changed by hand), or every stale and missing one.
    Rebuild(Option<String>, bool),
    Auto(bool),
    /// Read the tree deep (surfaces and corridors made), or back to the quick reading.
    Deep(bool),
    /// A branch folded or opened against its start, by its key.
    Fold(String),
    /// Only the rows in a state (and their parents), or all.
    Filter(Option<Status>),
}

fn msg(e: Event) -> Message {
    Message::RoadProject(e)
}

/// The last tree read: the revision and the depth it was read at, and what it answered.
type Read = (String, bool, Result<RoadProjectTree, String>);

/// The panel's state: deep or quick, the user's folds, the filter, and the last tree.
#[derive(Default)]
pub struct PanelState {
    pub deep: bool,
    /// The branches the user folded or opened against their start (a branch starts open when something in it wants
    /// attention, folded when all of it is quiet): by kind and id.
    toggled: HashSet<String>,
    filter: Option<Status>,
    read: RefCell<Option<Read>>,
}

impl std::fmt::Debug for PanelState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PanelState").field("deep", &self.deep).finish()
    }
}

/// A state in words.
fn state_name(s: S) -> &'static str {
    match s {
        S::Current => "Güncel",
        S::New => "Kabul edilmedi",
        S::Stale => "Güncel değil",
        S::Unbuildable => "Kurulamıyor",
        S::Unsolved => "Çözülmeyen kesit",
        S::Approved => "Onaylı",
        S::Changed => "Onaydan sonra değişti",
        S::Missing => "Nesnesi eksik",
        S::Edited => "Elle değişti",
        S::Gone => "Kaynağı yok",
    }
}

/// A state's tone: up to date quiet, stale amber, broken red (Civil 3D's yellow badge on an out of date item).
fn tone(s: S) -> Status {
    match s {
        S::Current => Status::Ok,
        S::New => Status::Neutral,
        S::Approved => Status::Info,
        S::Stale | S::Unsolved | S::Missing | S::Edited => Status::Warning,
        S::Unbuildable | S::Changed | S::Gone => Status::Danger,
    }
}

/// What a row of `kind` is called.
fn kind_words(kind: N) -> &'static str {
    match kind {
        N::Surface => "Yüzey",
        N::Alignment => "Aks",
        N::Profile => "Boykesit",
        N::Sections => "Kesit takımı",
        N::Superelevation => "Dever",
        N::Junction => "Kavşak",
        N::Template => "Tip kesit",
        N::Corridor => "Koridor",
        N::Volume => "Kübaj",
        N::Haul => "Brükner",
        N::Guardrail => "Otokorkuluk",
        N::Culvert => "Menfez",
        N::Wall => "İstinat duvarı",
        N::SheetSet => "Pafta takımı",
        N::Pay => "Poz metrajı",
        N::Drawing => "Çizim",
    }
}

/// The command that opens a row's editor and its icon (the editor's own).
fn editor_of(kind: N) -> Option<(&'static str, &'static str)> {
    Some(match kind {
        N::Surface => ("terrain.surfaces", "terrainSurface"),
        N::Alignment => ("alignment.editor", "alignmentEditor"),
        N::Profile => ("profile.editor", "profileEditor"),
        N::Sections | N::Template => ("section.editor", "sectionEditor"),
        N::Superelevation => ("superelevation.editor", "superelevationEditor"),
        N::Junction => ("junction.editor", "junctionEditor"),
        N::Corridor => ("corridor.editor", "corridorEditor"),
        N::Volume => ("volume.editor", "volumeEditor"),
        N::Haul => ("haul.editor", "haulEditor"),
        N::Guardrail => ("guardrail.editor", "guardrailEditor"),
        N::Culvert => ("culvert.editor", "culvertEditor"),
        N::Wall => ("retainingWall.editor", "retainingWallEditor"),
        N::SheetSet => ("road.sheets", "roadSheets"),
        N::Pay => ("payItems.editor", "payItemsEditor"),
        N::Drawing => return None,
    })
}

/// A row's key in the folds: its kind and id.
fn key_of(n: &RoadNode) -> String {
    format!("{:?}:{}", n.kind, n.id)
}

impl App {
    /// The tree as the drawing is now, read again when its revision or the depth changed.
    pub(crate) fn road_project_read(&self) -> Option<Result<RoadProjectTree, String>> {
        let doc = self.document.as_ref()?;
        let revision = doc.model.revision().to_string();
        let deep = self.road_project.deep;
        let mut kept = self.road_project.read.borrow_mut();
        if let Some((r, d, answer)) = kept.as_ref()
            && *r == revision
            && *d == deep
        {
            return Some(answer.clone());
        }
        let answer = road_project_query::preview(&doc.model, deep);
        *kept = Some((revision, deep, answer.clone()));
        Some(answer)
    }

    pub(crate) fn road_project_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Open(k) => {
                let Some(Ok(tree)) = self.road_project_read() else { return Task::none() };
                let Some(n) = tree.nodes.get(k).cloned() else { return Task::none() };
                if n.kind == N::Drawing {
                    self.select_drawing(&n.id);
                } else if let Some((command, _)) = editor_of(n.kind) {
                    return self.update(Message::Run(command));
                }
            }
            Event::Rebuild(id, edited) => {
                let Some(doc) = self.document.as_mut() else { return Task::none() };
                let input = RoadProjectRebuild { drawings: id.map(|i| vec![i]), edited, expected_revision: None };
                let result = road_project_rebuild::execute(&mut ExecutionContext::new(&mut doc.model), input);
                if let Some(out) = self.answered(result) {
                    let objects: usize = out.rebuilt.iter().map(|r| r.uids.len()).sum();
                    let words = match out.rebuilt.len() {
                        0 => "Güncel olmayan çizim yok.".to_owned(),
                        n => format!("{n} çizim yeniden üretildi: {objects} nesne."),
                    };
                    self.say(Level::Success, words);
                }
            }
            Event::Auto(on) => {
                let Some(doc) = self.document.as_mut() else { return Task::none() };
                let input = RoadProjectEdit { auto_rebuild: Some(on), expected_revision: None };
                let result = road_project_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
                if self.answered(result).is_some() {
                    let words = if on {
                        "Kendiliğinden yeniden üret açık: eskiyen çizimler değişikliğin adımında yeniden üretilir."
                    } else {
                        "Kendiliğinden yeniden üret kapalı."
                    };
                    self.say(Level::Info, words.to_owned());
                }
            }
            Event::Deep(on) => self.road_project.deep = on,
            Event::Fold(key) => {
                if !self.road_project.toggled.remove(&key) {
                    self.road_project.toggled.insert(key);
                }
            }
            Event::Filter(f) => self.road_project.filter = f,
        }
        Task::none()
    }

    /// A drawing's objects selected and shown.
    fn select_drawing(&mut self, id: &str) {
        let Some(doc) = self.document.as_ref() else { return };
        let Some(d) = doc.model.road_drawings().iter().find(|d| d.id == id) else { return };
        let slots: Vec<Slot> =
            d.uids.iter().filter_map(|u| Uuid::parse_str(u).ok()).filter_map(|u| doc.model.slot_of(u)).collect();
        let ids: Vec<f64> = slots.iter().map(|s| f64::from(s.0)).collect();
        self.selection.set(slots);
        if !ids.is_empty() {
            self.navigating(|app| app.zoom_to_objects(&ids));
        }
    }

    pub(crate) fn road_project_view(&self) -> Element<'_, Message> {
        let Some(read) = self.road_project_read() else {
            return container(label::caption("Açık çizim yok.")).padding([8, 12]).into();
        };
        let tree = match read {
            Ok(t) => t,
            Err(why) => return container(label::caption(why)).padding([8, 12]).into(),
        };
        // The tree's items from its flat rows, children before their parents taken.
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); tree.nodes.len()];
        let mut roots = Vec::new();
        for (k, n) in tree.nodes.iter().enumerate() {
            match n.parent {
                Some(p) if (p as usize) < k => children[p as usize].push(k),
                _ => roots.push(k),
            }
        }
        let mut made: Vec<Option<StatusItem<'_, Message>>> = (0..tree.nodes.len()).map(|_| None).collect();
        for k in (0..tree.nodes.len()).rev() {
            let mut item = road_item(k, &tree.nodes[k]);
            for &c in &children[k] {
                if let Some(child) = made[c].take() {
                    item = item.push(child);
                }
            }
            made[k] = Some(item);
        }
        let items: Vec<_> = roots.into_iter().filter_map(|k| made[k].take()).collect();
        let on = |b: bool| if b { Check::Checked } else { Check::Unchecked };
        let options = column![
            crate::style::fields::check(
                on(self.road_project.deep),
                "Kurarak denetle".to_owned(),
                Some(msg(Event::Deep(!self.road_project.deep))),
            ),
            crate::style::fields::check(
                on(tree.auto_rebuild),
                "Kendiliğinden yeniden üret".to_owned(),
                Some(msg(Event::Auto(!tree.auto_rebuild))),
            ),
        ]
        .spacing(4)
        .padding([2, 12]);
        let rebuild = (tree.rebuildable > 0).then(|| msg(Event::Rebuild(None, false)));
        let body = StatusTree::new(items)
            .toggled(&self.road_project.toggled, |key| msg(Event::Fold(key)))
            .filter(self.road_project.filter, |f| msg(Event::Filter(f)))
            .bulk(icon(Icon::Redo).size(13.0), "Hepsini yeniden üret", tree.rebuildable as usize, rebuild)
            .counted("Güncel değil", "Sorun")
            .empty("Projede yol kaynağı yok: Arazi sekmesinden yüzey, Güzergâh'tan aks oluşturun.");
        column![options, kentos_ui::widget::horizontal_divider(), body.view()].spacing(4).padding([6, 0]).into()
    }
}

/// A row of the tree: its kind's icon, its name, its count or kind faint, its state's pill and why, and for a drawing
/// the button that draws it again.
fn road_item<'a>(k: usize, n: &RoadNode) -> StatusItem<'a, Message> {
    let meta = match (n.kind, n.objects) {
        (N::Drawing, Some(c)) => format!("{c} nesne"),
        (kind, _) => kind_words(kind).to_owned(),
    };
    let glyph = match editor_of(n.kind) {
        Some((_, name)) => crate::icons::from_web(Some(name)),
        None => Icon::Layers,
    };
    let mut item = StatusItem::new(key_of(n), n.name.clone())
        .icon(icon(glyph).size(14.0))
        .meta(meta)
        .status(tone(n.state), state_name(n.state))
        .detail(n.why.join(" "))
        .on_press(msg(Event::Open(k)));
    match (n.kind, n.state) {
        (N::Drawing, S::Stale | S::Missing | S::Current) => {
            item = item.action(icon(Icon::Redo).size(13.0), "Yeniden üret", msg(Event::Rebuild(Some(n.id.clone()), false)));
        }
        (N::Drawing, S::Edited) => {
            item = item.action(
                icon(Icon::Redo).size(13.0),
                "Üstüne yaz: elle değişenler gider",
                msg(Event::Rebuild(Some(n.id.clone()), true)),
            );
        }
        _ => {}
    }
    item
}
