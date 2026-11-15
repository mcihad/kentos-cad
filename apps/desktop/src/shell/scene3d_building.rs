//! 3B görünüm of a building (docs/adr/8067; plan/bim-plan.md BIM-06): the bodies the IFC export writes
//! (`arch_ifc::model` → `kentos_ifc::mesh`), so what the architect sees is what the file holds, through the same
//! scene core and renderer as the road's (scene3d_window.rs). This module gives that window its building: the meshes,
//! their parts (the class's colour, a room a pale volume; the objects selected in the plan and those the last check
//! found something about over them), the storeys shown and the rooms on or off, and a click's object selected in the
//! plan. It is a view: it never changes the drawing.
//!
//! ```text
//! ┌ 3B görünüm ─────────────────────────────────────────────────────────────────────────────────────┐
//! │ [Kuzeydoğu] [Güneybatı] [Kuzeybatı] [Üst] [Sığdır]        Katlar [x] Zemin kat [x] 1. kat  [x] Mahaller │
//! │ ┌ the bodies: drag to orbit, right drag (or Shift) to pan, the wheel to dolly; a click selects in the plan ┐ │
//! │ └──────────────────────────────────────────────────────────────────────────────────────────┘ │
//! │ ■ Duvar ■ Döşeme ■ Kapı ■ Pencere ■ Mahal ■ Seçili ■ Bulgu        Seçilen nokta: … · Duvar · …  │
//! │ 68 yapı elemanı, 9 mahal · 3 140 üçgen, 1 karo · bakış 45°, eğim 30° …                  [Kapat] │
//! └─────────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use std::collections::BTreeSet;
use std::sync::Arc;

use iced::Element;
use iced::widget::row;
use kentos_domain::{Document, Slot, Uuid};
use kentos_ifc::building::Class;
use kentos_ifc::mesh::{self, Kind, Mesh};
use kentos_ui::label;
use kentos_ui::theme::Tokens;
use kentos_ui::widget::tree_view::Check;

use crate::app::{App, Dialog, Message};
use crate::scene3d_window::{Event, Scene, Source, Window};
use crate::traces::Control;

/// The parts a building's tiles carry, their names and colours (linear RGB, alpha below 1: see-through): the classes,
/// a room, then what is selected and what a check found, an element's and a room's.
pub(crate) const PARTS: [(&str, [f32; 4]); 15] = [
    ("Duvar", [0.80, 0.78, 0.74, 1.0]),
    ("Döşeme", [0.60, 0.61, 0.64, 1.0]),
    ("Kolon", [0.52, 0.55, 0.60, 1.0]),
    ("Kiriş", [0.47, 0.50, 0.56, 1.0]),
    ("Çatı", [0.62, 0.32, 0.24, 1.0]),
    ("Merdiven", [0.72, 0.66, 0.55, 1.0]),
    ("Korkuluk", [0.33, 0.35, 0.38, 1.0]),
    ("Kapı", [0.52, 0.35, 0.20, 1.0]),
    ("Pencere", [0.55, 0.74, 0.88, 0.55]),
    ("Mahal", [0.30, 0.56, 0.86, 0.16]),
    ("Diğer", [0.62, 0.62, 0.62, 1.0]),
    ("Seçili", [1.0, 0.62, 0.05, 1.0]),
    ("Bulgu", [0.86, 0.10, 0.08, 1.0]),
    ("Seçili mahal", [1.0, 0.62, 0.05, 0.38]),
    ("Bulgulu mahal", [0.86, 0.10, 0.08, 0.42]),
];
const ROOM: u32 = 9;
const SELECTED: u32 = 11;
const FOUND: u32 = 12;
const SELECTED_ROOM: u32 = 13;
const FOUND_ROOM: u32 = 14;
const ROOMS: &str = "Mahaller";
const STOREYS: &str = "Katlar";

/// A class's part.
fn class_part(kind: Kind) -> u32 {
    match kind {
        Kind::Space => ROOM,
        Kind::Element(c) => match c {
            Class::Wall => 0,
            Class::Slab | Class::Covering => 1,
            Class::Column => 2,
            Class::Beam | Class::Member => 3,
            Class::Roof | Class::Chimney => 4,
            Class::Stair | Class::Ramp => 5,
            Class::Railing => 6,
            Class::Door => 7,
            Class::Window => 8,
            Class::TransportElement | Class::Furniture | Class::SanitaryTerminal => 10,
        },
    }
}

/// A building in the window.
#[derive(Debug, Clone)]
pub(crate) struct Building3d {
    pub(crate) meshes: Arc<Vec<Mesh>>,
    /// Each storey's name and whether it shows.
    pub(crate) storeys: Vec<(String, bool)>,
    /// Whether the rooms show.
    pub(crate) rooms: bool,
    /// The objects selected in the plan and those the last check found something about, by persistent id.
    pub(crate) selected: BTreeSet<String>,
    pub(crate) found: BTreeSet<String>,
    /// The drawing's generation and revision the meshes were made from.
    pub(crate) made: (u64, u64),
}

impl Building3d {
    /// The drawing's building: none when it has no body.
    pub(crate) fn of(doc: &Document, selected: BTreeSet<String>, found: BTreeSet<String>) -> Option<Building3d> {
        let (building, _, _) = kentos_native_application::arch_ifc::model(doc);
        let meshes = mesh::meshes(&building);
        if !meshes.iter().any(|m| matches!(m.kind, Kind::Element(_))) {
            return None;
        }
        Some(Building3d {
            meshes: Arc::new(meshes),
            // The drawing's own name of the storey (Zemin kat), not the file's coded one.
            storeys: building.storeys.iter().map(|s| (s.long_name.clone().unwrap_or_else(|| s.name.clone()), true)).collect(),
            rooms: true,
            selected,
            found,
            made: (doc.generation(), doc.revision()),
        })
    }

    /// How many element bodies it has (a room is no body): what the IFC file holds.
    pub(crate) fn bodies(&self) -> usize {
        self.meshes.iter().filter(|m| matches!(m.kind, Kind::Element(_))).count()
    }

    /// How many rooms.
    pub(crate) fn rooms_count(&self) -> usize {
        self.meshes.iter().filter(|m| m.kind == Kind::Space).count()
    }

    /// How many things the scene shows: bodies and rooms.
    pub(crate) fn count(&self) -> usize {
        self.shown().count()
    }

    /// The meshes shown: the storeys ticked, the rooms if they show.
    fn shown(&self) -> impl Iterator<Item = &Mesh> {
        self.meshes
            .iter()
            .filter(|m| self.storeys.get(m.storey).is_none_or(|s| s.1))
            .filter(|m| self.rooms || m.kind != Kind::Space)
    }

    /// A mesh's part: an object a check found something about red, else one selected in the plan amber, else its
    /// class's; a room stays see-through in each.
    fn part_of(&self, m: &Mesh) -> u32 {
        let owner = m.id.map(|id| Uuid::from_u128(id).to_string());
        let room = m.kind == Kind::Space;
        match owner {
            Some(u) if self.found.contains(&u) => {
                if room {
                    FOUND_ROOM
                } else {
                    FOUND
                }
            }
            Some(u) if self.selected.contains(&u) => {
                if room {
                    SELECTED_ROOM
                } else {
                    SELECTED
                }
            }
            _ => class_part(m.kind),
        }
    }

    /// The meshes shown as the scene's sources, each named by its class, its own name and its storey; a click goes
    /// through a room to what is behind it.
    pub(crate) fn sources(&self) -> Vec<Source> {
        self.shown()
            .map(|m| {
                let class = PARTS[class_part(m.kind) as usize].0;
                let storey = self.storeys.get(m.storey).map_or("", |s| s.0.as_str());
                let name = [class, m.name.as_str(), storey].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ");
                Source {
                    part: self.part_of(m),
                    name,
                    owner: m.id.map(|id| Uuid::from_u128(id).to_string()),
                    vertices: Arc::new(m.vertices.clone()),
                    triangles: m.triangles.clone(),
                    smooth: false,
                    through: m.kind == Kind::Space,
                }
            })
            .collect()
    }

    /// The legend: the parts the scene shows, in the parts' order.
    pub(crate) fn legend(&self) -> Vec<(&'static str, [f32; 4])> {
        let used: BTreeSet<u32> = self.shown().map(|m| self.part_of(m)).collect();
        used.into_iter().map(|p| PARTS[p as usize]).collect()
    }

    /// The line's count: bodies and rooms, those shown when a storey or the rooms are hidden.
    pub(crate) fn counted(&self) -> String {
        let bodies = self.shown().filter(|m| m.kind != Kind::Space).count();
        let rooms = self.shown().filter(|m| m.kind == Kind::Space).count();
        let mut out = format!("{bodies} yapı elemanı");
        if bodies != self.bodies() {
            out.push_str(&format!(" ({} içinden)", self.bodies()));
        }
        if self.rooms {
            out.push_str(&format!(", {rooms} mahal"));
        }
        let found = self.shown().filter(|m| m.id.is_some_and(|id| self.found.contains(&Uuid::from_u128(id).to_string()))).count();
        if found > 0 {
            out.push_str(&format!(" · {found} nesnede Denetim bulgusu"));
        }
        out
    }

    /// The storeys' and the rooms' ticks.
    pub(crate) fn filters(&self) -> Element<'_, Message> {
        let tick = |on: bool| if on { Check::Checked } else { Check::Unchecked };
        let mut r = row![label::caption(STOREYS).style(|t: &iced::Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) })]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Center);
        for (i, (name, on)) in self.storeys.iter().enumerate() {
            r = r.push(crate::style::fields::check(tick(*on), name.clone(), Some(Message::Scene3d(Event::Storey(i)))));
        }
        r.push(crate::style::fields::check(tick(self.rooms), ROOMS.to_owned(), Some(Message::Scene3d(Event::Rooms)))).into()
    }

    /// A trace's press on a storey's or the rooms' tick.
    pub(crate) fn control(&self, control: &Control<'_>) -> Option<Message> {
        match control {
            Control::Press(ROOMS) => Some(Message::Scene3d(Event::Rooms)),
            Control::Press(name) => {
                self.storeys.iter().position(|s| s.0 == *name).map(|i| Message::Scene3d(Event::Storey(i)))
            }
            _ => None,
        }
    }
}

impl App {
    /// 3B görünüm of the drawing's building: its bodies as the IFC export makes them, the plan's selection and the
    /// last check's findings over their classes' colours.
    pub(crate) fn open_building3d(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(b) = Building3d::of(&doc.model, self.building3d_selected(), self.building3d_found()) else {
            self.warn(
                "3B görünümde gösterilecek bir bina gövdesi yok: önce duvar, döşeme, kolon ya da çatı çizin (yol projesinde 3B görünüm araziyi ve koridorları gösterir)."
                    .to_owned(),
            );
            return;
        };
        let scene = Scene::of(b.sources(), b.count());
        self.scene3d_window = Some(Box::new(Window::new(scene, Vec::new(), Some(b))));
        self.dialog = Some(Dialog::Scene3d);
    }

    /// A click's object selected in the plan (a click on nothing, or on what the drawing has no object for, empties
    /// the selection, as a click on nothing in the plan does), and the scene's selected parts made again.
    pub(crate) fn building3d_select(&mut self, owner: Option<String>) {
        let Some(doc) = self.document.as_ref() else { return };
        let slots: Vec<Slot> =
            owner.iter().filter_map(|u| Uuid::parse_str(u).ok()).filter_map(|u| doc.model.slot_of(u)).collect();
        self.selection.set(slots);
        let selected = self.building3d_selected();
        if let Some(w) = self.scene3d_window.as_mut()
            && let Some(b) = w.building.as_mut()
        {
            b.selected = selected;
            w.scene = Arc::new(Scene::of(b.sources(), b.count()));
        }
    }

    /// The building made again when the drawing changed since it was made (an undo while the window is open): the
    /// storeys and the rooms ticked as they were, by their names.
    pub(crate) fn building3d_refresh(&mut self) {
        let Some(doc) = self.document.as_ref() else { return };
        let now = (doc.model.generation(), doc.model.revision());
        let Some(old) = self.scene3d_window.as_ref().and_then(|w| w.building.as_ref()) else { return };
        if old.made == now {
            return;
        }
        let (rooms, hidden): (bool, Vec<String>) =
            (old.rooms, old.storeys.iter().filter(|s| !s.1).map(|s| s.0.clone()).collect());
        let Some(mut b) = Building3d::of(&doc.model, self.building3d_selected(), self.building3d_found()) else {
            // Nothing left to show: the window closes and says why.
            self.scene3d_window = None;
            self.dialog = None;
            self.warn("Binada gösterilecek gövde kalmadı; 3B görünüm kapandı.".to_owned());
            return;
        };
        b.rooms = rooms;
        for s in &mut b.storeys {
            s.1 = !hidden.contains(&s.0);
        }
        if let Some(w) = self.scene3d_window.as_mut() {
            w.scene = Arc::new(Scene::of(b.sources(), b.count()));
            w.building = Some(b);
        }
    }

    /// The persistent ids of the objects selected in the plan.
    fn building3d_selected(&self) -> BTreeSet<String> {
        let Some(doc) = self.document.as_ref() else { return BTreeSet::new() };
        self.selection.ids().iter().filter_map(|s| doc.model.uid(*s)).map(|u| u.to_string()).collect()
    }

    /// The objects of the findings chosen in Denetim, else of all it lists (its last check, while the drawing is as it
    /// saw it): a drawing's findings (a layer's name, a room's number) can name nearly every object, so the rows chosen
    /// say what to look at.
    fn building3d_found(&self) -> BTreeSet<String> {
        if self.arch_audit_stale() {
            return BTreeSet::new();
        }
        let chosen = self.arch_audit_chosen();
        if !chosen.is_empty() {
            return chosen.into_iter().flat_map(|f| f.uids.iter().cloned()).collect();
        }
        let Some(a) = self.arch.arch_audit.audited.as_ref() else { return BTreeSet::new() };
        self.arch_audit_shown().into_iter().filter_map(|i| a.findings.get(i)).flat_map(|f| f.uids.iter().cloned()).collect()
    }
}
