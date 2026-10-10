//! Zaman and Senaryo on the desktop (docs/adr/0210; the web's
//! `app/timeSlider.ts`, `app/scenarios.ts` and `app/timeCommands.ts`).
//!
//! - **Zaman sürgüsü** ([`Slider`]): the session's slider over the shown
//!   temporal layers. Its range is the geometry store's (the objects' times,
//!   `Spatial::time_summary`), its positions the core's
//!   (`kentos_geometry_core::time`); its window goes to the store, which
//!   leaves out of every query what it does not show, and the styled layers
//!   follow it part by part (style/scene.rs). Nothing of it is saved.
//! - **Senaryolar**: which scenario the drawing shows, Senaryoyu göster and
//!   Mevcut durum as changes of the tree's visibility (as the eye: an edit,
//!   not an undo step), Senaryo oluştur and Senaryoyu uygula through
//!   `cad.scenarios.edit`.
//! - The windows: Zaman ayarları ([`layer`]) and Senaryo oluştur
//!   ([`scenario`]); the bar under the drawing ([`bar`]).

pub mod bar;
pub mod layer;
pub mod scenario;

use std::time::{Duration, Instant};

use iced::Task;
use kentos_contracts::{LayerNode, LayerNodeType, scenario_pairs};
use kentos_geometry_core::time::{self, Step, Unit, Window};
use kentos_interaction::Level;

use crate::app::{App, Message};

/// The speeds of playback, in steps a second.
pub const SPEEDS: [f64; 4] = [0.5, 1.0, 2.0, 4.0];

/// Zaman sürgüsü's state (docs/adr/0210 §5).
#[derive(Debug, Clone)]
pub struct Slider {
    pub open: bool,
    /// Aralık: the window runs to the next position; Anlık: the position's moment.
    pub ranged: bool,
    pub step: Step,
    /// The range the positions cover.
    pub extent: Option<(f64, f64)>,
    /// The positions: the anchor and the last one's number (0 … `last`).
    pub anchor: f64,
    pub last: i64,
    pub position: i64,
    pub playing: bool,
    /// An index into [`SPEEDS`].
    pub speed: usize,
    pub looping: bool,
    /// Adım's number as typed.
    pub count: String,
    /// When playback last took a step.
    stepped: Option<Instant>,
}

impl Default for Slider {
    fn default() -> Self {
        Self {
            open: false,
            ranged: false,
            step: Step {
                n: 1,
                unit: Unit::Year,
            },
            extent: None,
            anchor: 0.0,
            last: 0,
            position: 0,
            playing: false,
            speed: 1,
            looping: false,
            count: "1".to_owned(),
            stepped: None,
        }
    }
}

/// The reason the slider does not open over a drawing without temporal objects.
pub const NOTHING: &str = "Görünen zamansal katmanlarda zamanı olan nesne yok. Bir katmana Zaman ayarları’ndan başlangıç alanı verin.";

/// The shown rasters that follow the slider (docs/adr/0243 §7): their count and
/// their steps' range, joined to the temporal objects' (`count`, `extent`).
pub fn with_rasters(
    (count, extent): (usize, Option<(f64, f64)>),
    model: &kentos_domain::Document,
) -> (usize, Option<(f64, f64)>) {
    let layers = model.layers();
    let mut out = (count, extent);
    for e in model.entities() {
        let kentos_contracts::Entity::Raster(r) = e else {
            continue;
        };
        let Some(d) = r.raster.dataset.as_ref().filter(|d| d.follow_time) else {
            continue;
        };
        let Some(k) = d.time_dim() else { continue };
        if !layers.is_visible(&e.base().layer_id) {
            continue;
        }
        let v = &d.dims[k].values;
        let (Some(&a), Some(&b)) = (v.first(), v.last()) else {
            continue;
        };
        out.0 += 1;
        out.1 = Some(match out.1 {
            Some((lo, hi)) => (lo.min(a), hi.max(b)),
            None => (a, b),
        });
    }
    out
}
/// The reason a step is refused.
pub const TOO_MANY: &str =
    "Zaman aralığı bu adımla 100 000 konumdan fazla; daha büyük bir adım seçin.";

impl Slider {
    /// The window the slider gives now; none while it is closed.
    pub fn window(&self) -> Option<Window> {
        self.open
            .then(|| Window::at(self.anchor, self.step, self.position, self.ranged))
    }

    /// Position `k`'s moment.
    pub fn moment_at(&self, k: i64) -> f64 {
        time::position(self.anchor, self.step, k)
    }

    /// The moment of the position now.
    pub fn moment(&self) -> f64 {
        self.moment_at(self.position)
    }

    /// Position `k`'s moment as the step's unit shows it (the slider's ends).
    pub fn position_text(&self, k: i64) -> String {
        time::show(self.moment_at(k), self.step.unit)
    }

    /// What the position shows: its date, or a period's two (its date once
    /// when inside one day).
    pub fn label(&self) -> String {
        self.window()
            .map(|w| time::show_window(&w, self.step.unit))
            .unwrap_or_default()
    }

    /// The slider's ends: their clocks when both lie in one day under a
    /// day's step, else their dates (docs/adr/0210 §5).
    pub fn ends(&self) -> [String; 2] {
        time::show_ends(self.moment_at(0), self.moment_at(self.last), self.step.unit)
    }

    /// The positions for the range and the step; the position at `at`'s
    /// nearest one (none: kept within them). False past the limit.
    fn place(&mut self, at: Option<f64>) -> bool {
        let Some(extent) = self.extent else {
            return false;
        };
        let Some((anchor, last)) = time::positions(extent, self.step) else {
            return false;
        };
        self.anchor = anchor;
        self.last = last;
        match at {
            Some(t) => self.position = self.nearest(t),
            None => self.position = self.position.min(last),
        }
        true
    }

    /// The position whose moment is nearest `t` (the earlier of two as near).
    fn nearest(&self, t: f64) -> i64 {
        let (mut lo, mut hi) = (0, self.last);
        while lo < hi {
            let mid = (lo + hi) / 2;
            if self.moment_at(mid) < t {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        if lo > 0 && t - self.moment_at(lo - 1) <= self.moment_at(lo) - t {
            lo - 1
        } else {
            lo
        }
    }
}

/// What the bar and the windows tell the app.
#[derive(Debug, Clone)]
pub enum Event {
    First,
    Prev,
    PlayStop,
    Next,
    Last,
    /// The slider dragged to a position.
    Go(i64),
    /// Adım's number typed, and taken.
    Count(String),
    CountDone,
    /// Adım's unit, by its index in `Unit::ALL`.
    Unit(usize),
    Ranged(bool),
    /// Hız, by its index in [`SPEEDS`].
    Speed(usize),
    Loop,
    Close,
    /// Playback's clock.
    Tick,
    /// The layer tree's menu: Zaman ayarları over a layer, a scenario shown, compared, applied; Mevcut durum.
    OpenLayer(String),
    Show(String),
    Base,
    Compare(String),
    AskApply(String),
    Layer(layer::Event),
    Scenario(scenario::Event),
}

/// The open windows of Zaman and Senaryo.
#[derive(Debug, Default)]
pub struct State {
    pub slider: Slider,
    pub layer: Option<layer::Window>,
    pub scenario: Option<scenario::Window>,
}

pub(crate) fn msg(event: Event) -> Message {
    Message::Time(event)
}

/// Playback's clock: a tick every 40 ms while it plays; each is handled after the drawing showed the last step.
pub fn ticks() -> impl iced::futures::Stream<Item = Message> {
    use iced::futures::SinkExt;
    let (mut out, ticks) = iced::futures::channel::mpsc::channel(1);
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_millis(40));
            if iced::futures::executor::block_on(out.send(msg(Event::Tick))).is_err() {
                break;
            }
        }
    });
    ticks
}

/// What the drawing shows: the field state, one scenario, or a mix of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shown {
    Base,
    Scenario(String),
    Mixed,
}

/// The scenario groups of the tree, in its order.
pub fn scenario_groups(tree: &[LayerNode]) -> Vec<&LayerNode> {
    fn walk<'a>(nodes: &'a [LayerNode], out: &mut Vec<&'a LayerNode>) {
        for n in nodes {
            if n.kind == LayerNodeType::Group && n.scenario.is_some() {
                out.push(n);
            }
            walk(&n.children, out);
        }
    }
    let mut out = Vec::new();
    walk(tree, &mut out);
    out
}

/// The base layers scenario `id` stands in for, each with its layer in the scenario.
pub fn pairs_of(layers: &kentos_domain::LayerTree, id: &str) -> Vec<(String, String)> {
    layers
        .get(id)
        .filter(|g| g.scenario.is_some())
        .map(|g| {
            scenario_pairs(layers.nodes(), g)
                .into_iter()
                .map(|(b, l)| (b.to_owned(), l.to_owned()))
                .collect()
        })
        .unwrap_or_default()
}

/// Gösterilen senaryo: one scenario group shown and the base layers it stands
/// in for hidden, that scenario; none shown, Mevcut durum; anything else a mix.
pub fn shown_scenario(layers: &kentos_domain::LayerTree) -> Shown {
    let shown: Vec<&LayerNode> = scenario_groups(layers.nodes())
        .into_iter()
        .filter(|g| layers.is_visible(&g.id))
        .collect();
    match shown.as_slice() {
        [] => Shown::Base,
        [one]
            if pairs_of(layers, &one.id)
                .iter()
                .all(|(b, _)| !layers.is_visible(b)) =>
        {
            Shown::Scenario(one.id.clone())
        }
        _ => Shown::Mixed,
    }
}

/// The scenario a layer or a group is in (itself when it is one).
pub fn scenario_of<'a>(layers: &'a kentos_domain::LayerTree, id: &str) -> Option<&'a LayerNode> {
    let mut at = layers.get(id);
    while let Some(n) = at {
        if n.scenario.is_some() {
            return Some(n);
        }
        at = layers.parent(&n.id);
    }
    None
}

impl App {
    /// The slider's window into the geometry store, which every query and the styled layers follow.
    fn time_apply(&mut self) {
        let window = self.time.slider.window();
        self.spatial.set_time_window(window);
    }

    /// Opens the slider at its last position (the latest state); the reason when it does not.
    pub(crate) fn time_show(&mut self) -> Option<&'static str> {
        let doc = self.document.as_ref()?;
        self.spatial.sync(&doc.model);
        let (count, extent) = with_rasters(self.spatial.time_summary(), &doc.model);
        let (true, Some(extent)) = (count > 0, extent) else {
            return Some(NOTHING);
        };
        let layers = doc.model.layers();
        // One ranged layer shown: moments; only instant ones: periods.
        let ranged = !layers
            .leaves()
            .iter()
            .any(|l| l.time.as_ref().is_some_and(|t| t.end.is_some()) && layers.is_visible(&l.id));
        let s = &mut self.time.slider;
        s.ranged = ranged;
        s.step = time::auto_step(extent);
        s.count = s.step.n.to_string();
        s.extent = Some(extent);
        if !s.place(None) {
            return Some(TOO_MANY);
        }
        s.position = s.last;
        s.open = true;
        self.time_apply();
        None
    }

    pub(crate) fn time_close(&mut self) {
        let s = &mut self.time.slider;
        s.open = false;
        s.playing = false;
        self.time_apply();
    }

    /// Zaman sürgüsü (`time.slider`): opens or closes it.
    pub(crate) fn time_toggle(&mut self) {
        if self.time.slider.open {
            self.time_close();
        } else if let Some(why) = self.time_show() {
            self.warn(why);
        }
    }

    fn time_go(&mut self, k: i64) {
        let s = &mut self.time.slider;
        let next = k.clamp(0, s.last);
        if next != s.position {
            s.position = next;
            self.time_apply();
        }
    }

    fn time_step(&mut self, step: Step) {
        let s = &mut self.time.slider;
        let at = s.moment();
        let before = s.step;
        s.step = step;
        if !s.place(Some(at)) {
            s.step = before;
            s.count = before.n.to_string();
            self.warn(TOO_MANY);
            return;
        }
        s.count = step.n.to_string();
        self.time_apply();
    }

    /// The drawing changed: the range and the positions again, the position at the nearest moment.
    pub(crate) fn time_refresh(&mut self) {
        if !self.time.slider.open {
            return;
        }
        let summary = self.spatial.time_summary();
        let (count, extent) = match self.document.as_ref() {
            Some(doc) => with_rasters(summary, &doc.model),
            None => summary,
        };
        let (true, Some(extent)) = (count > 0, extent) else {
            self.time_close();
            return;
        };
        let s = &mut self.time.slider;
        if s.extent == Some(extent) {
            return;
        }
        let at = s.moment();
        s.extent = Some(extent);
        // A range the step would cut into too many positions takes the step it would take by itself.
        if !s.place(Some(at)) {
            s.step = time::auto_step(extent);
            s.count = s.step.n.to_string();
            s.place(Some(at));
        }
        self.time_apply();
    }

    pub(crate) fn time_event(&mut self, event: Event) -> Task<Message> {
        let last = self.time.slider.last;
        let position = self.time.slider.position;
        match event {
            Event::First => {
                self.time.slider.playing = false;
                self.time_go(0);
            }
            Event::Prev => {
                self.time.slider.playing = false;
                self.time_go(position - 1);
            }
            Event::Next => {
                self.time.slider.playing = false;
                self.time_go(position + 1);
            }
            Event::Last => {
                self.time.slider.playing = false;
                self.time_go(last);
            }
            Event::Go(k) => {
                self.time.slider.playing = false;
                self.time_go(k);
            }
            Event::PlayStop => {
                let s = &mut self.time.slider;
                if s.playing {
                    s.playing = false;
                } else if s.open {
                    // At the end, playback starts again from the first position.
                    if s.position >= s.last {
                        self.time_go(0);
                    }
                    let s = &mut self.time.slider;
                    s.playing = true;
                    s.stepped = Some(Instant::now());
                }
            }
            Event::Tick => {
                let s = &self.time.slider;
                // A tick comes after the drawing showed the last step; the next waits for the speed.
                let due = Duration::from_secs_f64(1.0 / SPEEDS[s.speed.min(SPEEDS.len() - 1)]);
                if !s.playing || s.stepped.is_some_and(|t| t.elapsed() < due) {
                    return Task::none();
                }
                if s.position >= s.last {
                    if s.looping {
                        self.time_go(0);
                    } else {
                        self.time.slider.playing = false;
                        return Task::none();
                    }
                } else {
                    self.time_go(position + 1);
                }
                self.time.slider.stepped = Some(Instant::now());
            }
            Event::Count(text) => self.time.slider.count = text,
            Event::CountDone => {
                let s = &self.time.slider;
                let n = s
                    .count
                    .trim()
                    .parse::<i64>()
                    .ok()
                    .filter(|n| (1..=999).contains(n));
                match n {
                    Some(n) if n != s.step.n => self.time_step(Step {
                        n,
                        unit: s.step.unit,
                    }),
                    Some(_) => {}
                    None => {
                        self.time.slider.count = self.time.slider.step.n.to_string();
                        self.warn("Adım 1 ile 999 birim arasında olmalı.");
                    }
                }
            }
            Event::Unit(i) => {
                if let Some(unit) = Unit::ALL.get(i).copied() {
                    let n = self.time.slider.step.n;
                    self.time_step(Step { n, unit });
                }
            }
            Event::Ranged(ranged) => {
                if self.time.slider.ranged != ranged {
                    self.time.slider.ranged = ranged;
                    self.time_apply();
                }
            }
            Event::Speed(i) => self.time.slider.speed = i.min(SPEEDS.len() - 1),
            Event::Loop => self.time.slider.looping = !self.time.slider.looping,
            Event::Close => self.time_close(),
            Event::OpenLayer(id) => self.open_time_layer(Some(id)),
            Event::Show(id) => self.show_scenario(&id),
            Event::Base => self.show_base(),
            Event::Compare(id) => self.open_scenario_compare(Some(id)),
            Event::AskApply(id) => self.ask_scenario_apply(Some(id)),
            Event::Layer(e) => return self.time_layer_event(e),
            Event::Scenario(e) => return self.scenario_event(e),
        }
        Task::none()
    }

    // ── Senaryolar ──────────────────────────────────────────────────────

    /// Senaryoyu göster: its group shown, the other scenario groups hidden,
    /// the base layers it stands in for hidden, those only others stand in
    /// for shown. An active base layer hidden gives way to its layer in the scenario.
    pub(crate) fn show_scenario(&mut self, id: &str) {
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let model = &mut doc.model;
        let Some(name) = model
            .layers()
            .get(id)
            .filter(|g| g.scenario.is_some())
            .map(|g| g.name.clone())
        else {
            return;
        };
        let mine = pairs_of(model.layers(), id);
        let others: Vec<(String, Vec<(String, String)>)> = scenario_groups(model.layers().nodes())
            .into_iter()
            .filter(|g| g.id != id)
            .map(|g| (g.id.clone(), pairs_of(model.layers(), &g.id)))
            .collect();
        for (g, pairs) in others {
            model.set_layer_visible(&g, false);
            for (b, _) in pairs {
                if !mine.iter().any(|(m, _)| *m == b) {
                    model.set_layer_visible(&b, true);
                }
            }
        }
        model.set_layer_visible(id, true);
        for (b, _) in &mine {
            model.set_layer_visible(b, false);
        }
        let active = model.layers().active().to_owned();
        if let Some((_, l)) = mine.iter().find(|(b, _)| *b == active) {
            model.set_active_layer(l);
        }
        self.say(Level::Info, format!("Senaryo gösteriliyor: {name}."));
    }

    /// Mevcut durum: every scenario group hidden, the base layers they stand
    /// in for shown; an active scenario layer hidden gives way to its base layer.
    pub(crate) fn show_base(&mut self) {
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let model = &mut doc.model;
        let active = model.layers().active().to_owned();
        let groups: Vec<(String, Vec<(String, String)>)> = scenario_groups(model.layers().nodes())
            .into_iter()
            .map(|g| (g.id.clone(), pairs_of(model.layers(), &g.id)))
            .collect();
        for (g, pairs) in groups {
            model.set_layer_visible(&g, false);
            for (b, l) in pairs {
                model.set_layer_visible(&b, true);
                if l == active {
                    model.set_active_layer(&b);
                }
            }
        }
        self.say(Level::Info, "Mevcut durum gösteriliyor.");
    }

    /// The scenario a command means: the selected or the active layer's, else the one shown, else the first.
    pub(crate) fn scenario_at_hand(&self) -> Option<String> {
        let doc = self.document.as_ref()?;
        let model = &doc.model;
        let layers = model.layers();
        let selected = self
            .selection
            .ids()
            .iter()
            .find_map(|slot| model.get(*slot).map(|e| e.base().layer_id.clone()));
        selected
            .and_then(|l| scenario_of(layers, &l).map(|g| g.id.clone()))
            .or_else(|| scenario_of(layers, layers.active()).map(|g| g.id.clone()))
            .or_else(|| match shown_scenario(layers) {
                Shown::Scenario(id) => Some(id),
                _ => None,
            })
            .or_else(|| {
                scenario_groups(layers.nodes())
                    .first()
                    .map(|g| g.id.clone())
            })
    }

    /// The temporal layer a command means: the active one when it is, else the first temporal layer shown.
    pub(crate) fn temporal_at_hand(&self) -> Option<LayerNode> {
        let doc = self.document.as_ref()?;
        let layers = doc.model.layers();
        layers
            .get(layers.active())
            .filter(|n| n.time.is_some())
            .or_else(|| {
                layers
                    .leaves()
                    .into_iter()
                    .find(|l| l.time.is_some() && layers.is_visible(&l.id))
            })
            .cloned()
    }

    /// Zamanı karşılaştır (`time.compare`, docs/adr/0210 §8): Veri karşılaştır
    /// over the temporal layer at hand, at the position before the slider's
    /// and its own (closed, the layer's own first and last moments).
    pub(crate) fn open_time_compare(&mut self) {
        let Some(layer) = self.temporal_at_hand() else {
            self.warn(
                "Zamansal katman yok; önce bir katmana Zaman ayarları’ndan başlangıç alanı verin.",
            );
            return;
        };
        let Some(rule) = layer.time.clone() else {
            return;
        };
        let s = &self.time.slider;
        let dates = if s.open {
            Some((
                s.position_text((s.position - 1).max(0)),
                s.position_text(s.position),
            ))
        } else {
            let Some(doc) = &self.document else {
                return;
            };
            let (_, summary) = time::layer_times(
                time::Rule {
                    ranged: rule.end.is_some(),
                    cumulative: rule.cumulative,
                },
                doc.model.by_layer(&layer.id).map(|e| {
                    let a = &e.base().attrs;
                    (
                        a.get(&rule.start).map(String::as_str),
                        rule.end.as_ref().and_then(|k| a.get(k)).map(String::as_str),
                    )
                }),
            );
            summary
                .extent
                .map(|(a, b)| (time::show(a, Unit::Day), time::show(b, Unit::Day)))
        };
        let Some((old_date, new_date)) = dates else {
            self.warn(format!(
                "“{}” katmanında zamanı olan nesne yok.",
                layer.name
            ));
            return;
        };
        self.open_data_compare_with(crate::data_compare::Preset {
            old_node: Some(layer.id.clone()),
            new_node: Some(layer.id.clone()),
            old_date,
            new_date,
            key: rule.key,
            ..crate::data_compare::Preset::default()
        });
    }

    /// Senaryoyu karşılaştır (`scenario.compare`, docs/adr/0210 §8): Veri
    /// karşılaştır with a base layer as Eski and its layer in the scenario as
    /// Yeni: the active layer's pair, else the scenario's first.
    pub(crate) fn open_scenario_compare(&mut self, id: Option<String>) {
        let Some(id) = id.or_else(|| self.scenario_at_hand()) else {
            self.warn("Projede senaryo yok; önce Senaryo oluştur ile bir senaryo yapın.");
            return;
        };
        let Some(doc) = &self.document else {
            return;
        };
        let layers = doc.model.layers();
        let name = layers
            .get(&id)
            .map_or_else(|| id.clone(), |n| n.name.clone());
        let pairs = pairs_of(layers, &id);
        let active = layers.active();
        let Some((base, layer)) = pairs
            .iter()
            .find(|(b, l)| b == active || l == active)
            .or_else(|| pairs.first())
            .cloned()
        else {
            self.warn(format!(
                "“{name}” senaryosunda bir ana katmanın yerine geçen katman yok; karşılaştırılacak bir şey yok."
            ));
            return;
        };
        self.open_data_compare_with(crate::data_compare::Preset {
            old_node: Some(base),
            new_node: Some(layer),
            ..crate::data_compare::Preset::default()
        });
    }

    /// The status bar's words for the scenario shown (“Senaryo: Mevcut durum”), while the project has one.
    pub(crate) fn scenario_words(&self) -> Option<String> {
        let doc = self.document.as_ref()?;
        let layers = doc.model.layers();
        if scenario_groups(layers.nodes()).is_empty() {
            return None;
        }
        Some(format!(
            "Senaryo: {}",
            match shown_scenario(layers) {
                Shown::Base => "Mevcut durum".to_owned(),
                Shown::Mixed => "Karışık".to_owned(),
                Shown::Scenario(id) => layers.get(&id).map_or(id, |n| n.name.clone()),
            }
        ))
    }

    /// The status bar's Senaryo cell (docs/adr/0210 §10): its words while
    /// `words`, the icon alone once tight; its menu shows another scenario or Mevcut durum.
    pub(crate) fn scenario_cell(&self, words: bool) -> Option<iced::Element<'_, Message>> {
        use kentos_ui::widget::Menu;
        use kentos_ui::widget::status_bar::Readout;
        let text = self.scenario_words()?;
        let doc = self.document.as_ref()?;
        let layers = doc.model.layers();
        let now = shown_scenario(layers);
        let groups: Vec<(String, String, Option<String>)> = scenario_groups(layers.nodes())
            .into_iter()
            .map(|g| {
                (
                    g.id.clone(),
                    g.name.clone(),
                    g.scenario.as_ref().and_then(|s| s.note.clone()),
                )
            })
            .collect();
        let menu = move || {
            let mut menu = Menu::new()
                .header("Gösterilen")
                .radio("Mevcut durum", now == Shown::Base, Some(msg(Event::Base)))
                .icon(crate::icons::from_web(Some("scenarioBase")));
            for (id, name, _) in &groups {
                menu = menu.radio(
                    name.clone(),
                    now == Shown::Scenario(id.clone()),
                    Some(msg(Event::Show(id.clone()))),
                );
            }
            menu.separator()
                .item("Senaryo oluştur…", Message::Run("scenario.create"))
                .icon(crate::icons::from_web(Some("scenarioCreate")))
                .item("Senaryoyu karşılaştır…", Message::Run("scenario.compare"))
                .icon(crate::icons::from_web(Some("scenarioCompare")))
                .item("Senaryoyu uygula…", Message::Run("scenario.apply"))
                .icon(crate::icons::from_web(Some("scenarioApply")))
        };
        Some(
            Readout::new(kentos_ui::label::muted(if words { text } else { String::new() }))
                .icon(crate::icons::from_web(Some("scenario")))
                .menu(menu)
                .tip("Senaryo: çizimin gösterdiği (Mevcut durum, bir senaryo ya da karışık görünürlük). Başkasını göstermek ya da senaryo oluşturmak için tıklayın.")
                .into(),
        )
    }

    /// Whether the project has a scenario (the scenario commands are on then).
    pub(crate) fn has_scenarios(&self) -> bool {
        self.document
            .as_ref()
            .is_some_and(|d| !scenario_groups(d.model.layers().nodes()).is_empty())
    }
}
