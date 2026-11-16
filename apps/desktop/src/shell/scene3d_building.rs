//! 3B görünüm of a building (docs/adr/8067; plan/bim-plan.md BIM-06): the bodies the IFC export writes
//! (`arch_ifc::model` → `kentos_ifc::mesh`), so what the architect sees is what the file holds, through the same
//! scene core and renderer as the road's (scene3d_window.rs). This module gives that window its building: the meshes,
//! their parts (the class's colour, a room a pale volume; the objects selected in the plan and those the last check
//! found something about over them), the storeys shown and the rooms on or off on the tool strip, and a click's object
//! selected in the plan. While its window is open beside the plan the building follows the drawing: an edit, an undo,
//! the plan's selection and the findings chosen in Denetim show in 3D as they change. It is a view: it never changes
//! the drawing.
//!
//! ```text
//! ┌ 3B görünüm — KentOS CAD ─────────────────────────────────────────────────────────────────────┐
//! │ ◈ Kuzeydoğu ▾  ⤢ Sığdır │ ▤ Katlar ▾  ▢ Mahaller                                          ⧉ │
//! ├───────────────────────────────────────────────────────────────────────────────────────────────┤
//! │                     the bodies: a click selects in the plan                              (K)  │
//! │ ┌ Lejant ┐                                                                                    │
//! │ │■ Duvar │                                                                                    │
//! ├───────────────────────────────────────────────────────────────────────────────────────────────┤
//! │ ⌖ Duvar D-12, Zemin kat   Y …  X …  Z …                   68 yapı elemanı, 9 mahal │ ⚠ 2 │ ? │
//! └───────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use std::collections::BTreeSet;
use std::sync::Arc;

use iced::Element;
use kentos_domain::{Document, Slot, Uuid};
use kentos_ifc::building::Class;
use kentos_ifc::mesh::{self, Kind, Mesh};
use kentos_ui::widget::Menu;

use crate::app::{App, Message};
use crate::scene3d_window::{Event, Scene, Source, Window};
use crate::style::strip;
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
    // The blocks that are building elements (docs/adr/8070 §5): a lift, furniture, a sanitary fixture; a colour of
    // their own, the floor's grey would hide them.
    ("Donatı", [0.27, 0.56, 0.52, 1.0]),
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
const ALL_STOREYS: &str = "Bütün katları göster";

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
            // A curtain wall's own body is its panels: glass (docs/adr/8070 §7).
            Class::Window | Class::CurtainWall => 8,
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
    /// The plan's selection version and the findings' key `selected` and `found` were taken at
    /// (`App::follow_scene3d`): neither is taken again until one of them changes.
    pub(crate) taken: (u64, u64),
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
            taken: (0, 0),
        })
    }

    /// How many element bodies it has (a room is no body): what the IFC file holds.
    pub(crate) fn bodies(&self) -> usize {
        self.meshes.iter().filter(|m| matches!(m.kind, Kind::Element(_))).count()
    }

    /// How many rooms.
    #[cfg(test)]
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

    /// The meshes shown as the scene's sources, each named by its class, its own name and its storey (“Duvar D-12,
    /// Zemin kat”; a name that starts with its class's word is not said twice); a click goes through a room to what is
    /// behind it.
    pub(crate) fn sources(&self) -> Vec<Source> {
        self.shown()
            .map(|m| {
                let class = PARTS[class_part(m.kind) as usize].0;
                let storey = self.storeys.get(m.storey).map_or("", |s| s.0.as_str());
                let own = m.name.trim();
                let mut name = if own.is_empty() {
                    class.to_owned()
                } else if own.to_lowercase().starts_with(&class.to_lowercase()) {
                    own.to_owned()
                } else {
                    format!("{class} {own}")
                };
                if !storey.is_empty() {
                    name.push_str(", ");
                    name.push_str(storey);
                }
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

    /// The status bar's count: bodies and rooms, those shown when a storey or the rooms are hidden.
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
        out
    }

    /// How many of the objects shown the last check found something about (they are red).
    pub(crate) fn found_shown(&self) -> usize {
        self.shown().filter(|m| m.id.is_some_and(|id| self.found.contains(&Uuid::from_u128(id).to_string()))).count()
    }

    /// Every storey shown again.
    pub(crate) fn show_all_storeys(&mut self) {
        for s in &mut self.storeys {
            s.1 = true;
        }
    }

    /// The strip's storeys: a menu of ticks, each storey's while it shows (the menu stays open while ticking), all
    /// of them back at once when some are hidden; its words say how many show when not all do.
    pub(crate) fn storeys_menu(&self) -> Element<'_, Message> {
        let shown = self.storeys.iter().filter(|s| s.1).count();
        let words =
            if shown == self.storeys.len() { STOREYS.to_owned() } else { format!("{STOREYS} {shown}/{}", self.storeys.len()) };
        let storeys = self.storeys.clone();
        strip::menu(
            crate::icons::from_web(Some("levelsAll")),
            &words,
            "Katlar: gösterilen katlar. Menü açıkken katlar art arda işaretlenir; gizlenen katın gövdeleri ve mahalleri görünmez.",
            move || {
                let mut m = Menu::new();
                for (i, (name, on)) in storeys.iter().enumerate() {
                    m = m.check(name.clone(), *on, Message::Scene3d(Event::Storey(i))).stay();
                }
                if storeys.iter().any(|s| !s.1) {
                    m = m.separator().item(ALL_STOREYS, Message::Scene3d(Event::AllStoreys));
                }
                m
            },
        )
    }

    /// The strip's rooms switch.
    pub(crate) fn rooms_toggle(&self) -> Element<'_, Message> {
        strip::toggle(
            Some(crate::icons::from_web(Some("room"))),
            ROOMS,
            "Mahaller: mahallerin soluk hacimlerini gösterir ya da gizler. Tıklama hacmin ardındaki nesneyi seçer; ardında bir şey yoksa mahali.",
            self.rooms,
            Message::Scene3d(Event::Rooms),
        )
    }

    /// A trace's press on a storey's or the rooms' tick.
    pub(crate) fn control(&self, control: &Control<'_>) -> Option<Message> {
        match control {
            Control::Press(ROOMS) => Some(Message::Scene3d(Event::Rooms)),
            Control::Press(ALL_STOREYS) => Some(Message::Scene3d(Event::AllStoreys)),
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
        let made = b.made;
        self.scene3d_window = Some(Box::new(Window::new(scene, Vec::new(), Some(b), made)));
        let taken = (self.selection.version(), self.building3d_found_key());
        if let Some(b) = self.scene3d_window.as_mut().and_then(|w| w.building.as_mut()) {
            b.taken = taken;
        }
        self.show_surface(crate::windows::Surface::Scene3d);
    }

    /// A click's object selected in the plan (a click on nothing, or on what the drawing has no object for, empties
    /// the selection, as a click on nothing in the plan does), and the scene's selected parts made again.
    pub(crate) fn building3d_select(&mut self, owner: Option<String>) {
        let Some(doc) = self.document.as_ref() else { return };
        let slots: Vec<Slot> =
            owner.iter().filter_map(|u| Uuid::parse_str(u).ok()).filter_map(|u| doc.model.slot_of(u)).collect();
        self.selection.set(slots);
        let selected = self.building3d_selected();
        let version = self.selection.version();
        if let Some(w) = self.scene3d_window.as_mut()
            && let Some(b) = w.building.as_mut()
        {
            b.selected = selected;
            // Taken now: following the plan does not paint it again.
            b.taken.0 = version;
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
            self.scene3d_gone();
            self.warn("Binada gösterilecek gövde kalmadı; 3B görünüm kapandı.".to_owned());
            return;
        };
        b.rooms = rooms;
        b.taken = (self.selection.version(), self.building3d_found_key());
        for s in &mut b.storeys {
            s.1 = !hidden.contains(&s.0);
        }
        if let Some(w) = self.scene3d_window.as_mut() {
            w.scene = Arc::new(Scene::of(b.sources(), b.count()));
            w.building = Some(b);
        }
    }

    /// The building follows the plan while its window is open (`App::follow_scene3d`): the drawing changed, made
    /// again; the plan's selection or the findings chosen in Denetim changed, painted again. Nothing is taken while
    /// neither changed: the selection's version and the findings' key say so without a list made.
    pub(crate) fn building3d_follow(&mut self) {
        self.building3d_refresh();
        let now = (self.selection.version(), self.building3d_found_key());
        if self.scene3d_window.as_ref().and_then(|w| w.building.as_ref()).is_none_or(|b| b.taken == now) {
            return;
        }
        let (selected, found) = (self.building3d_selected(), self.building3d_found());
        if let Some(w) = self.scene3d_window.as_mut()
            && let Some(b) = w.building.as_mut()
        {
            b.taken = now;
            if b.selected != selected || b.found != found {
                b.selected = selected;
                b.found = found;
                w.scene = Arc::new(Scene::of(b.sources(), b.count()));
            }
        }
    }

    /// What the findings shown in 3D depend on, as one number: the check (its drawing's state and how many it
    /// found), the rows chosen, the group shown and whether the drawing changed since.
    pub(crate) fn building3d_found_key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let a = &self.arch.arch_audit;
        let mut h = std::collections::hash_map::DefaultHasher::new();
        a.audited.as_ref().map(|x| (x.generation, x.revision, x.findings.len())).hash(&mut h);
        a.selected.hash(&mut h);
        a.chosen.hash(&mut h);
        a.group.hash(&mut h);
        self.arch_audit_stale().hash(&mut h);
        h.finish()
    }

    /// The persistent ids of the objects selected in the plan.
    fn building3d_selected(&self) -> BTreeSet<String> {
        let Some(doc) = self.document.as_ref() else { return BTreeSet::new() };
        self.selection.ids().iter().filter_map(|s| doc.model.uid(*s)).map(|u| u.to_string()).collect()
    }

    /// The objects of the findings chosen in Denetim, else of all it lists (its last check, while the drawing is as it
    /// saw it): a drawing's findings (a layer's name, a room's number) can name nearly every object, so the rows chosen
    /// say what to look at.
    pub(crate) fn building3d_found(&self) -> BTreeSet<String> {
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
