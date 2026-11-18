//! The model viewer (docs/adr/8061 §3–§5, docs/adr/8073 §2): a model's objects in a tree by structure or by class,
//! its bodies in the 3D scene, the chosen object's details; hide, isolate, show all; the spaces and the openings on and
//! off, everything see-through; a section plane or a box about the chosen objects; the views of every 3D view.
//!
//! It knows its host only through its messages (docs/adr/8061 §4): the host gives it the model ([`Viewer::load`]),
//! the reading's progress, the chosen object's details and the objects to choose or paint as found; it tells the host
//! what was chosen and when to stop reading or close ([`Out`]). No file, thread, command or application: the
//! application's adapter and the single-window example are its hosts. Its look is viewer_view.rs.

use std::collections::BTreeSet;
use std::fmt;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use kentos_model_view::section::{self, Cut, Section};
use kentos_model_view::{Details, Input, OrbitCamera, PARTS, Row, TreeMode, View, parts};
use kentos_render_scene3d::Style;
use kentos_scene3d::{Camera, Exaggeration, Viewport};

use crate::scene::{Gesture, Picked, Scene, Seen, Source};

/// What the panel at the left shows: the tree by structure, by class, or the objects that could not be shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Structure,
    Class,
    NotShown,
}

impl fmt::Display for Panel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Panel::Structure => "Yapı",
            Panel::Class => "Sınıf",
            Panel::NotShown => "Gösterilemeyen",
        })
    }
}

/// Where the viewer is: reading (the phase, how far of how much; 0 of 0 when not known), the model shown, or the file
/// refused (why, with what to do).
#[derive(Debug, Clone, PartialEq)]
pub enum Stage {
    Reading { phase: String, done: usize, total: usize },
    Shown,
    Failed(String),
}

#[derive(Debug, Clone)]
pub enum Message {
    Gesture(Gesture),
    View(usize),
    Fit,
    /// The camera to the chosen object, all of it in view.
    Focus,
    North,
    Legend,
    Panel(Panel),
    /// A tree node opened or closed, by its id.
    Open(String),
    Choose(Option<usize>),
    /// A tree row's eye: the object hidden, or shown again.
    Eye(usize),
    Hide,
    Isolate,
    ShowAll,
    Spaces,
    Openings,
    SeeThrough,
    Cut(Cut),
    /// A section box about the chosen object.
    Boxed,
    Uncut,
    /// The section plane's place along its axis.
    At(f64),
    /// The section plane keeps its other side.
    Flip,
    /// A details group folded or opened, by its title.
    Fold(String),
    /// The tree's search: the objects whose name, class or GlobalId hold these words (empty: the tree again).
    Search(String),
    /// A side panel dragged to a width (the tree's, the details'); none: its default width again (a double click).
    TreeWidth(Option<f32>),
    DetailsWidth(Option<f32>),
    Cancel,
    Close,
}

/// What the viewer tells its host.
#[derive(Debug, Clone, PartialEq)]
pub enum Out {
    /// The chosen object changed (its index and key; none: nothing is chosen): the host gives its details
    /// ([`Viewer::set_details`]) and may choose it elsewhere (the drawing it came from).
    Chosen(Option<(usize, String)>),
    /// The reading is to stop.
    Cancel,
    /// The viewer is to close.
    Close,
}

/// The viewer.
pub struct Viewer {
    pub title: String,
    pub stage: Stage,
    view: Option<View>,
    scene: Arc<Scene>,
    pub orbit: OrbitCamera,
    pub panel: Panel,
    open: BTreeSet<String>,
    rows: Vec<Row>,
    details: Option<(usize, Details)>,
    folded: BTreeSet<String>,
    pub legend: bool,
    pub see_through: bool,
    pub section: Section,
    pub picked: Option<Picked>,
    /// What the host says of the model for the status bar (its schema), as (words, tip).
    pub facts: Vec<(String, String)>,
    /// The tree's search words; empty: the tree.
    pub query: String,
    /// The side panels' widths as dragged; none: their default.
    pub tree_width: Option<f32>,
    pub details_width: Option<f32>,
    /// The scene's width to its height as it last drew (0: not drawn yet), and whether the model loaded before it was
    /// is to be fitted again to it (docs/adr/8119): a narrow scene sees less across than up and down.
    aspect: f64,
    refit: bool,
    pub seen: Arc<Mutex<Seen>>,
    pub(crate) pending: Arc<AtomicBool>,
}

impl fmt::Debug for Viewer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Viewer").field("title", &self.title).field("stage", &self.stage).finish_non_exhaustive()
    }
}

impl Viewer {
    /// A viewer for a file named `title`, reading it.
    pub fn new(title: impl Into<String>) -> Viewer {
        Viewer {
            title: title.into(),
            stage: Stage::Reading { phase: "Dosya okunuyor".to_owned(), done: 0, total: 0 },
            view: None,
            scene: Arc::new(Scene::of(Vec::new(), 0)),
            orbit: OrbitCamera { target: [0.0; 3], azimuth: 45.0, elevation: 30.0, distance: 10.0 },
            panel: Panel::Structure,
            open: BTreeSet::new(),
            rows: Vec::new(),
            details: None,
            folded: BTreeSet::new(),
            legend: true,
            see_through: false,
            section: Section::None,
            picked: None,
            facts: Vec::new(),
            query: String::new(),
            tree_width: None,
            details_width: None,
            aspect: 0.0,
            refit: false,
            seen: Arc::default(),
            pending: Arc::default(),
        }
    }

    /// How far the reading is.
    pub fn progress(&mut self, phase: impl Into<String>, done: usize, total: usize) {
        if matches!(self.stage, Stage::Reading { .. }) {
            self.stage = Stage::Reading { phase: phase.into(), done, total };
        }
    }

    /// The file refused: why, and what to do.
    pub fn failed(&mut self, why: impl Into<String>) {
        self.stage = Stage::Failed(why.into());
    }

    /// The model read: shown from the north-east, all of it in view, the tree open down to the storeys.
    pub fn load(&mut self, input: Input, facts: Vec<(String, String)>) {
        let view = View::new(input);
        self.open = view
            .input()
            .objects
            .iter()
            .filter(|o| o.category.is_place() && o.category != kentos_model_view::Category::Storey)
            .map(|o| o.key.clone())
            .collect();
        self.view = Some(view);
        self.facts = facts;
        self.stage = Stage::Shown;
        self.details = None;
        self.picked = None;
        self.section = Section::None;
        self.rebuild();
        self.look(0);
        // Fitted again by the scene's shape once it draws, unless the user moves first.
        self.refit = self.aspect <= 0.0;
    }

    /// The same source read again (the drawing edited, docs/adr/8119): the camera, the tree's open branches, the panel,
    /// the search, the section, the toggles and the side panes stay; the hidden, isolated, chosen and found objects stay
    /// by their keys ([`View::carried`]). The chosen object after it, whose details the host reads again; none before
    /// the first model, which is loaded as [`Viewer::load`] does.
    pub fn reload(&mut self, input: Input, facts: Vec<(String, String)>) -> Option<usize> {
        let Some(old) = self.view.as_ref() else {
            self.load(input, facts);
            return None;
        };
        let view = old.carried(input);
        let chosen = view.selected();
        self.view = Some(view);
        self.facts = facts;
        self.stage = Stage::Shown;
        self.details = None;
        self.picked = None;
        // A plane kept inside the model's span as it is now.
        if let Section::Plane { cut, at, .. } = self.section
            && let Some((lo, hi)) = self.span(cut)
            && let Section::Plane { at: now, .. } = &mut self.section
        {
            *now = at.clamp(lo, hi);
        }
        self.rebuild();
        chosen
    }

    pub fn view(&self) -> Option<&View> {
        self.view.as_ref()
    }

    pub fn scene(&self) -> &Arc<Scene> {
        &self.scene
    }

    /// The tree's rows as the panel shows them now.
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// The chosen object's details, when the host has given them.
    pub fn details(&self) -> Option<&Details> {
        let chosen = self.view.as_ref()?.selected()?;
        self.details.as_ref().filter(|(o, _)| *o == chosen).map(|(_, d)| d)
    }

    /// The chosen object's details, read by the host.
    pub fn set_details(&mut self, object: usize, details: Details) {
        self.details = Some((object, details));
    }

    pub(crate) fn is_folded(&self, title: &str) -> bool {
        self.folded.contains(title)
    }

    /// The object whose key is `key` chosen by the host (the drawing's selection); false when the model has none.
    pub fn choose_key(&mut self, key: &str) -> bool {
        let Some(o) = self.view.as_ref().and_then(|v| v.find(key)) else { return false };
        self.choose(Some(o));
        true
    }

    /// The objects whose keys are `keys` painted as found.
    pub fn set_found<'k>(&mut self, keys: impl IntoIterator<Item = &'k str>) {
        let Some(v) = self.view.as_mut() else { return };
        let found: Vec<usize> = keys.into_iter().filter_map(|k| v.find(k)).collect();
        v.set_found(found);
        self.rebuild();
    }

    /// The camera in the shown space (nothing exaggerated: a building is seen as it is).
    pub fn camera(&self) -> Camera {
        self.orbit.camera(&Exaggeration::NONE)
    }

    /// The parts' styles: everything see-through when asked, the chosen and the found standing out.
    pub fn styles(&self) -> Arc<Vec<Style>> {
        let keep = [parts::SELECTED, parts::FOUND];
        Arc::new(
            PARTS
                .iter()
                .enumerate()
                .map(|(k, (_, c))| {
                    let a = if self.see_through && !keep.contains(&(k as u32)) { c[3].min(0.3) } else { c[3] };
                    Style { color: [c[0], c[1], c[2], a] }
                })
                .collect(),
        )
    }

    /// The section's planes in the shown space.
    pub fn planes(&self) -> Vec<[f64; 4]> {
        self.section.planes().into_iter().map(|p| section::shown(p, &Exaggeration::NONE)).collect()
    }

    /// The span along `cut`'s axis a section plane moves in: the box of what shows.
    pub fn span(&self, cut: Cut) -> Option<(f64, f64)> {
        let b = self.view.as_ref()?.bounds();
        let k = cut.axis();
        (!b.is_empty()).then(|| (b.min[k], b.max[k]))
    }

    pub fn update(&mut self, message: Message) -> Option<Out> {
        match message {
            Message::Gesture(g) => return self.gesture(g),
            Message::View(i) => self.look(i),
            Message::Fit => self.fit(),
            Message::Focus => {
                let b = self.view.as_ref().and_then(|v| v.selected().map(|o| v.bounds_under(o)));
                if let Some(b) = b.filter(|b| !b.is_empty()) {
                    self.orbit.fit_in(&b, &Exaggeration::NONE, 1.0, self.aspect);
                }
            }
            Message::North => self.orbit.north(),
            Message::Legend => self.legend = !self.legend,
            Message::Panel(p) => {
                self.panel = p;
                self.refresh_rows();
            }
            Message::Open(id) => {
                if !self.open.remove(&id) {
                    self.open.insert(id);
                }
                self.refresh_rows();
            }
            Message::Choose(o) => return self.choose(o),
            Message::Eye(o) => {
                let v = self.view.as_mut()?;
                if v.is_hidden(o) { v.unhide(o) } else { v.hide([o]) }
                self.changed();
            }
            Message::Hide => {
                let v = self.view.as_mut()?;
                let o = v.selected()?;
                v.hide([o]);
                self.changed();
            }
            Message::Isolate => {
                let v = self.view.as_mut()?;
                let o = v.selected()?;
                v.isolate([o]);
                self.changed();
            }
            Message::ShowAll => {
                self.view.as_mut()?.show_all();
                self.changed();
            }
            Message::Spaces | Message::Openings => {
                let v = self.view.as_mut()?;
                if matches!(message, Message::Spaces) { v.spaces = !v.spaces } else { v.openings = !v.openings }
                self.changed();
            }
            Message::SeeThrough => self.see_through = !self.see_through,
            Message::Cut(cut) => {
                let (lo, hi) = self.span(cut)?;
                self.section = Section::Plane { cut, at: (lo + hi) / 2.0, flip: false };
            }
            Message::Boxed => {
                let v = self.view.as_ref()?;
                let b = v.bounds_under(v.selected()?);
                if !b.is_empty() {
                    self.section = Section::Box(section::about(&b));
                }
            }
            Message::Uncut => self.section = Section::None,
            Message::At(at) => {
                if let Section::Plane { at: now, .. } = &mut self.section {
                    *now = at;
                }
            }
            Message::Flip => {
                if let Section::Plane { flip, .. } = &mut self.section {
                    *flip = !*flip;
                }
            }
            Message::Fold(title) => {
                if !self.folded.remove(&title) {
                    self.folded.insert(title);
                }
            }
            Message::Search(words) => self.query = words,
            Message::TreeWidth(w) => self.tree_width = w,
            Message::DetailsWidth(w) => self.details_width = w,
            Message::Cancel => return Some(Out::Cancel),
            Message::Close => return Some(Out::Close),
        }
        None
    }

    /// The objects the search finds: every word of it in the object's name, class or GlobalId, letters' case and
    /// Turkish dotted and dotless i alike; in the file's order.
    pub fn found(&self) -> Vec<usize> {
        let words: Vec<String> = fold(&self.query).split_whitespace().map(str::to_owned).collect();
        let Some(v) = self.view.as_ref().filter(|_| !words.is_empty()) else { return Vec::new() };
        v.input()
            .objects
            .iter()
            .enumerate()
            .filter(|(_, o)| {
                let hay = fold(&format!("{} {} {}", o.label, o.class, o.key));
                words.iter().all(|w| hay.contains(w.as_str()))
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn gesture(&mut self, g: Gesture) -> Option<Out> {
        if !matches!(g, Gesture::Size(..)) {
            // The user moved: the model stays where they put it.
            self.refit = false;
        }
        match g {
            Gesture::Size(width, height) => {
                if width > 0.0 && height > 0.0 {
                    self.aspect = f64::from(width) / f64::from(height);
                }
                if std::mem::take(&mut self.refit) {
                    self.look(self.orbit.preset().unwrap_or(0));
                }
            }
            Gesture::Orbit(dx, dy) => self.orbit.orbit(f64::from(dx), f64::from(dy)),
            Gesture::Pan(dx, dy) => self.orbit.pan(f64::from(dx), f64::from(dy)),
            Gesture::Dolly(steps) => self.orbit.dolly(f64::from(steps)),
            Gesture::Pick(x, y, width, height) => {
                let vp = Viewport { width: f64::from(width), height: f64::from(height) };
                let planes = self.planes();
                self.picked = self.scene.pick_cut(&self.camera(), &vp, &Exaggeration::NONE, f64::from(x), f64::from(y), &planes);
                let o = self.picked.as_ref().and_then(|p| p.owner.as_deref()).and_then(|k| self.view.as_ref()?.find(k));
                let picked = self.picked.take();
                let out = self.choose(o);
                self.picked = picked;
                if let Some(o) = o {
                    let keys: Vec<String> = self.view.as_ref().map_or_else(Vec::new, |v| {
                        v.ancestors(o).into_iter().map(|a| v.input().objects[a].key.clone()).collect()
                    });
                    self.open.extend(keys);
                    self.refresh_rows();
                }
                return out;
            }
        }
        None
    }

    /// An object chosen (none: nothing), the point clicked forgotten; the host is told.
    fn choose(&mut self, object: Option<usize>) -> Option<Out> {
        let v = self.view.as_mut()?;
        if v.selected() == object {
            return None;
        }
        v.select(object);
        let key = object.map(|o| (o, v.input().objects[o].key.clone()));
        self.picked = None;
        self.changed();
        Some(Out::Chosen(key))
    }

    /// What shows changed: the scene and the tree made again.
    fn changed(&mut self) {
        self.rebuild();
    }

    /// The scene of the bodies that show, each its object's key so a click finds it; the tree's rows with it.
    fn rebuild(&mut self) {
        let Some(v) = &self.view else { return };
        let keys = &v.input().objects;
        let sources: Vec<Source> = v
            .sources()
            .into_iter()
            .map(|s| Source {
                part: s.part,
                name: s.name,
                owner: Some(keys[s.object].key.clone()),
                vertices: s.vertices,
                triangles: s.triangles,
                smooth: false,
                through: s.through,
            })
            .collect();
        let count = sources.len();
        self.scene = Arc::new(Scene::of(sources, count));
        self.refresh_rows();
    }

    fn refresh_rows(&mut self) {
        let Some(v) = &self.view else { return };
        self.rows = match self.panel {
            Panel::Structure => v.rows(TreeMode::Structure, &self.open),
            Panel::Class => v.rows(TreeMode::Class, &self.open),
            Panel::NotShown => Vec::new(),
        };
    }

    fn look(&mut self, i: usize) {
        let b = self.scene.bounds();
        self.orbit.look_in(i, &b, &Exaggeration::NONE, 1.0, self.aspect);
    }

    fn fit(&mut self) {
        let b = self.scene.bounds();
        self.orbit.fit_in(&b, &Exaggeration::NONE, 1.0, self.aspect);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use kentos_model_view::{BodyIn, Category, Cut, Input, ObjectIn, Section};
    use kentos_scene3d::Viewport;

    use super::*;

    /// A project, a storey, two unit cubes side by side along east (the first at 0, the second at 3) and a space.
    fn model() -> Input {
        let object = |key: &str, category, parent| ObjectIn {
            key: key.into(),
            class: format!("{category:?}"),
            label: key.into(),
            category,
            parent,
            storey: None,
        };
        let cube = |x: f64| {
            let v = vec![
                [x, 0.0, 0.0],
                [x + 1.0, 0.0, 0.0],
                [x + 1.0, 1.0, 0.0],
                [x, 1.0, 0.0],
                [x, 0.0, 1.0],
                [x + 1.0, 0.0, 1.0],
                [x + 1.0, 1.0, 1.0],
                [x, 1.0, 1.0],
            ];
            let t = vec![[0, 2, 1], [0, 3, 2], [4, 5, 6], [4, 6, 7], [0, 1, 5], [0, 5, 4], [1, 2, 6], [1, 6, 5], [2, 3, 7], [2, 7, 6], [3, 0, 4], [3, 4, 7]];
            (Arc::new(v), t)
        };
        let (a, ta) = cube(0.0);
        let (b, tb) = cube(3.0);
        Input {
            title: "deneme".into(),
            objects: vec![
                object("proje", Category::Project, None),
                object("kat", Category::Storey, Some(0)),
                object("duvar-1", Category::Wall, Some(1)),
                object("duvar-2", Category::Wall, Some(1)),
            ],
            bodies: vec![BodyIn { object: 2, positions: a, triangles: ta }, BodyIn { object: 3, positions: b, triangles: tb }],
            not_shown: Vec::new(),
            apart: None,
        }
    }

    fn loaded() -> Viewer {
        let mut v = Viewer::new("deneme.ifc");
        v.load(model(), Vec::new());
        v
    }

    /// A click where the camera, looking from the south, sees the point `p`.
    fn click_at(v: &mut Viewer, p: [f64; 3]) -> Option<Out> {
        let vp = Viewport { width: 800.0, height: 600.0 };
        let (x, y, _) = v.camera().project(p, &vp).expect("in front");
        v.update(Message::Gesture(Gesture::Pick(x as f32, y as f32, 800.0, 600.0)))
    }

    #[test]
    fn a_model_loads_shown_with_its_tree_open_to_its_storeys() {
        let v = loaded();
        assert_eq!(v.stage, Stage::Shown);
        let rows: Vec<(&str, usize)> = v.rows().iter().map(|r| (r.id.as_str(), r.depth)).collect();
        assert_eq!(rows, [("proje", 0), ("kat", 1)], "the storey closed, its parents open");
        assert_eq!(v.view().expect("loaded").counts(), (2, 2));
        assert_eq!(v.scene().sources(), 2);
    }

    #[test]
    fn choosing_tells_the_host_and_hiding_isolating_and_showing_all_follow_the_choice() {
        let mut v = loaded();
        assert_eq!(v.update(Message::Choose(Some(2))), Some(Out::Chosen(Some((2, "duvar-1".into())))));
        assert_eq!(v.update(Message::Choose(Some(2))), None, "the same choice again says nothing");
        let _ = v.update(Message::Isolate);
        assert_eq!(v.view().expect("loaded").counts(), (1, 2));
        let _ = v.update(Message::ShowAll);
        let _ = v.update(Message::Hide);
        assert_eq!(v.view().expect("loaded").counts(), (1, 2));
        let _ = v.update(Message::Eye(2));
        assert_eq!(v.view().expect("loaded").counts(), (2, 2), "its eye shows it again");
        assert_eq!(v.update(Message::Choose(None)), Some(Out::Chosen(None)));
        assert_eq!(v.update(Message::Hide), None, "nothing chosen: nothing hidden");
        assert_eq!(v.update(Message::Cancel), Some(Out::Cancel));
        assert_eq!(v.update(Message::Close), Some(Out::Close));
    }

    #[test]
    fn a_click_chooses_what_it_meets_and_opens_the_tree_to_it() {
        let mut v = loaded();
        let _ = v.update(Message::View(3));
        let out = click_at(&mut v, [3.5, 0.5, 1.0]);
        assert_eq!(out, Some(Out::Chosen(Some((3, "duvar-2".into())))));
        assert!(v.picked.is_some());
        assert!(v.rows().iter().any(|r| r.id == "duvar-2" && r.chosen), "its row shows chosen");
    }

    #[test]
    fn a_section_cuts_what_shows_and_what_a_click_meets() {
        let mut v = loaded();
        let _ = v.update(Message::Cut(Cut::NorthSouth));
        assert_eq!(v.section, Section::Plane { cut: Cut::NorthSouth, at: 2.0, flip: false }, "at the model's middle");
        assert_eq!(v.planes().len(), 1);
        // From above: the second cube is east of the plane, cut away; a click there meets nothing.
        let _ = v.update(Message::View(3));
        assert_eq!(click_at(&mut v, [3.5, 0.5, 1.0]), None);
        assert!(v.picked.is_none());
        let _ = v.update(Message::Flip);
        assert!(matches!(click_at(&mut v, [3.5, 0.5, 1.0]), Some(Out::Chosen(Some((3, _))))));
        let _ = v.update(Message::At(10.0));
        assert_eq!(v.section, Section::Plane { cut: Cut::NorthSouth, at: 10.0, flip: true });
        // A box about the chosen cube keeps it alone.
        let _ = v.update(Message::Boxed);
        let Section::Box(b) = v.section else { panic!("a box") };
        assert!(b.min[0] < 3.0 && b.max[0] > 4.0 && b.min[0] > 1.0, "{b:?}");
        let _ = v.update(Message::Uncut);
        assert!(v.planes().is_empty());
    }
}

/// `s` as the search compares it: lower case, a dotted capital I's dot dropped and the dotless i a dotted one, so “iç”
/// finds “İç_duvar” and “IFCWALL” finds “IfcWall”.
fn fold(s: &str) -> String {
    s.to_lowercase().replace('\u{0307}', "").replace('ı', "i")
}
