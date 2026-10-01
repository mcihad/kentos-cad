//! The workbench's layout kept between runs (docs/adr/0115): the web's
//! `kentos.ui.v1` (app/layoutPlan.ts) in `yerlesim.json` beside the
//! settings file, read with layout_plan.rs's rules, applied at start and
//! written 250 ms after the last change, all fields together.
//!
//! What it keeps: whether the side panels are shown (F4), the dock's width,
//! the layer tree's share of it, which of Katmanlar and İşlemler is in
//! front, the bottom panel (open, its height, its tab), İşlemler's tab and
//! folded categories, the ribbon's tab, whether it is folded, the commands
//! put on its quick access bar and each split button's choice. A kept size
//! is the user's wish: the window shows it within what it allows now, and a
//! narrower window does not change what is kept. The fields the desktop has
//! no part for (the classic toolbox, the theme, a setting here) are written
//! back as read.
//! A file that cannot be written is passed over, as the web's storage.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use iced::futures::channel::oneshot;
use iced::futures::{Stream, StreamExt, stream};
use iced::{Size, Subscription};
use kentos_ui::theme::typography;
use kentos_ui::widget::docking::{Event as DockEvent, Side};
use serde_json::{Map, Value};

use crate::app::{App, Message, Panel};
use crate::bottom::BottomTab;
use crate::catalog::catalog;
use crate::layout_plan::{self as plan, BOTTOM_HEIGHT, DOCK_WIDTH, LAYERS_FRACTION};
use crate::processing::panel::Tab as ProcessingTab;

/// The layout's file, beside `ayarlar.json`.
pub(crate) const FILE_NAME: &str = "yerlesim.json";

/// The kept layout and when it is to be written.
#[derive(Debug)]
pub(crate) struct Keeper {
    path: Option<PathBuf>,
    kept: Map<String, Value>,
    due: Option<Instant>,
}

impl Keeper {
    /// A layout kept in memory only (tests, a machine with no configuration folder).
    pub(crate) fn memory() -> Self {
        Self {
            path: None,
            kept: plan::read_layout(None),
            due: None,
        }
    }

    /// The layout kept in `folder`: what the file holds and its rules take.
    pub(crate) fn open(folder: &Path) -> Self {
        let path = folder.join(FILE_NAME);
        let text = std::fs::read_to_string(&path).ok();
        Self {
            kept: plan::read_layout(text.as_deref()),
            path: Some(path),
            due: None,
        }
    }

    fn number(&self, key: &str) -> f64 {
        self.kept
            .get(key)
            .and_then(Value::as_f64)
            .unwrap_or_default()
    }

    fn flag(&self, key: &str) -> bool {
        self.kept
            .get(key)
            .and_then(Value::as_bool)
            .unwrap_or_default()
    }

    fn text(&self, key: &str) -> &str {
        self.kept.get(key).and_then(Value::as_str).unwrap_or("")
    }

    fn texts(&self, key: &str) -> impl Iterator<Item = &str> {
        self.kept
            .get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
    }

    /// Keeps `value`; a change is written `SAVE_MS` after the last one.
    fn keep(&mut self, key: &str, value: Value, now: Instant) {
        if self.kept.get(key) != Some(&value) {
            self.kept.insert(key.to_owned(), value);
            self.due = Some(now + Duration::from_millis(plan::SAVE_MS));
        }
    }

    /// The commands the user put on the quick access bar
    /// (`ribbonQuickAccess`), in the order added.
    pub(crate) fn quick_access(&self) -> Vec<String> {
        self.texts("ribbonQuickAccess").map(str::to_owned).collect()
    }

    /// Keeps the user's quick access list.
    pub(crate) fn keep_quick_access(&mut self, list: &[String], now: Instant) {
        self.keep("ribbonQuickAccess", Value::from(list.to_vec()), now);
    }

    /// A split button's kept choice (`ribbonSplits`, by the button's key).
    pub(crate) fn split_choice(&self, key: &str) -> Option<&str> {
        self.kept
            .get("ribbonSplits")
            .and_then(|splits| splits.get(key))
            .and_then(Value::as_str)
    }

    /// Keeps a split button's choice (`komut|seçenek`).
    pub(crate) fn keep_split_choice(&mut self, key: &str, choice: String, now: Instant) {
        let mut splits = self
            .kept
            .get("ribbonSplits")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        splits.insert(key.to_owned(), Value::from(choice));
        self.keep("ribbonSplits", Value::Object(splits), now);
    }

    /// When the kept layout is to be written.
    pub(crate) fn due(&self) -> Option<Instant> {
        self.due
    }

    /// Writes the kept layout once it is due (`force`: now, the window
    /// closes). A file that cannot be written is passed over.
    pub(crate) fn write(&mut self, now: Instant, force: bool) {
        let Some(due) = self.due else {
            return;
        };
        if !force && now < due {
            return;
        }
        self.due = None;
        let Some(path) = &self.path else {
            return;
        };
        let Ok(text) = serde_json::to_string_pretty(&Value::Object(self.kept.clone())) else {
            return;
        };
        let _ = write_whole(path, &text);
    }

    #[cfg(test)]
    pub(crate) fn kept(&self) -> &Map<String, Value> {
        &self.kept
    }
}

/// Writes `text` through a file beside `path`, renamed over it: a crash
/// leaves the old layout or the new one, never half of one.
fn write_whole(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let partial = path.with_extension("json.yaziliyor");
    std::fs::write(&partial, text)?;
    std::fs::rename(&partial, path)
}

/// One message at `due`, from a thread of its own.
fn wake_at(due: &Instant) -> impl Stream<Item = Message> + use<> {
    let due = *due;
    let (done, wait) = oneshot::channel();
    std::thread::spawn(move || {
        std::thread::sleep(due.saturating_duration_since(Instant::now()));
        let _ = done.send(());
    });
    stream::once(wait).map(|_| Message::LayoutSave)
}

fn tab_key(tab: BottomTab) -> &'static str {
    match tab {
        BottomTab::History => "history",
        BottomTab::Coords => "coords",
        BottomTab::Points => "points",
        BottomTab::Messages => "messages",
        BottomTab::Python => "python",
    }
}

impl App {
    /// The bottom panel's tab strip: the web's panel height counts it.
    fn bottom_bar() -> f32 {
        kentos_ui::widget::tabs::height()
    }

    /// The kept layout on screen (at start, and when the window changes size
    /// for the sizes).
    pub(crate) fn apply_layout(&mut self) {
        let keeper = &self.layout;
        let ribbon_tab = keeper.text("ribbonTab").to_owned();
        let tabs: Vec<(&str, bool)> = catalog().tabs().map(|t| (t.id, t.contextual)).collect();
        let start = plan::start_tab(&ribbon_tab, &tabs);
        if let Some(tab) = catalog().tabs().find(|t| t.id == start) {
            self.tab = tab.id;
        }
        self.ribbon_collapsed = keeper.flag("ribbonCollapsed");
        self.command_expanded = keeper.flag("bottomExpanded");
        self.bottom_tab = match keeper.text("bottomTab") {
            "coords" => BottomTab::Coords,
            "points" => BottomTab::Points,
            "messages" => BottomTab::Messages,
            "python" => BottomTab::Python,
            _ => BottomTab::History,
        };
        self.processing.panel.tab = if keeper.text("processingTab") == "history" {
            ProcessingTab::History
        } else {
            ProcessingTab::Tools
        };
        self.processing.panel.folded = keeper
            .texts("processingFolded")
            .map(str::to_owned)
            .collect();
        let in_front = match keeper.text("dockTab") {
            "processing" => Panel::Processing,
            "blocks" => Panel::Blocks,
            _ => Panel::Layers,
        };
        let fraction = keeper.number("layersFraction");
        let visible = keeper.flag("rightVisible");
        let docks = self.hidden_docks.as_mut().unwrap_or(&mut self.docks);
        docks.update(DockEvent::Selected(in_front));
        docks.update(DockEvent::Shared(
            Side::Right,
            vec![fraction as f32, (1.0 - fraction) as f32],
        ));
        self.apply_sizes();
        if visible != self.right_panel_shown() {
            self.toggle_right_panel();
        }
    }

    /// The kept sizes as the window shows them now.
    pub(crate) fn apply_sizes(&mut self) {
        let window = self.window_size;
        let width = plan::dock_width_on(self.layout.number("dockWidth"), f64::from(window.width));
        let docks = self.hidden_docks.as_mut().unwrap_or(&mut self.docks);
        docks.set_size(Side::Right, typography::unscaled(width as f32));
        let panel =
            plan::bottom_height_on(self.layout.number("bottomHeight"), f64::from(window.height));
        self.bottom_log = Some((panel as f32 - Self::bottom_bar()).max(0.0));
    }

    /// The window changed size: the kept sizes are shown within it.
    pub(crate) fn window_resized(&mut self, size: Size) {
        self.window_size = size;
        self.apply_sizes();
        // The key tips go when the window changes size (the web's).
        self.key_tips = None;
    }

    /// The dock's edge was dragged: the width is shown by the web's rule and kept so.
    pub(crate) fn dock_dragged(&mut self, event: &DockEvent<Panel>, now: Instant) {
        match event {
            DockEvent::Resized(Side::Right, size) => {
                let shown = f64::from(typography::scaled(*size));
                let width = plan::dock_width_on(shown, f64::from(self.window_size.width));
                self.layout.keep("dockWidth", plan::number(width), now);
                self.apply_sizes();
            }
            DockEvent::Reset(Side::Right) => self.dock_width_reset(now),
            DockEvent::Shared(Side::Right, weights) if weights.len() == 2 => {
                let total = f64::from(weights[0] + weights[1]);
                if total > 0.0 {
                    let fraction = (f64::from(weights[0]) / total)
                        .clamp(LAYERS_FRACTION.min, LAYERS_FRACTION.max.unwrap_or(1.0));
                    self.layout
                        .keep("layersFraction", plan::number(fraction), now);
                    self.docks.update(DockEvent::Shared(
                        Side::Right,
                        vec![fraction as f32, (1.0 - fraction) as f32],
                    ));
                }
            }
            _ => {}
        }
    }

    /// The bottom panel's edge was dragged to `log` (its lists' height) or,
    /// `None`, double-clicked: the panel's height by the web's rule, kept so.
    pub(crate) fn bottom_dragged(&mut self, log: Option<f32>, now: Instant) {
        let panel = log.map_or(BOTTOM_HEIGHT.reset, |log| {
            f64::from(log + Self::bottom_bar())
        });
        let height = plan::bottom_height_on(panel, f64::from(self.window_size.height));
        self.layout.keep("bottomHeight", plan::number(height), now);
        self.apply_sizes();
    }

    /// The dock's width back to its first size (a double click on its edge).
    pub(crate) fn dock_width_reset(&mut self, now: Instant) {
        self.layout
            .keep("dockWidth", plan::number(DOCK_WIDTH.reset), now);
        self.apply_sizes();
    }

    /// After every message: the layout's states as they are now are kept
    /// (a change is written 250 ms later). Sizes are kept where they are
    /// dragged, not read back: a narrower window shows less but keeps them.
    pub(crate) fn follow_layout(&mut self, now: Instant) {
        let visible = self.right_panel_shown();
        let docks = self.hidden_docks.as_ref().unwrap_or(&self.docks);
        // The tab in front in Katmanlar' slot: Katmanlar, İşlemler or Bloklar.
        let in_front = docks.slot(Panel::Layers).and_then(|slot| {
            docks
                .stacks(Side::Right)
                .iter()
                .enumerate()
                .find(|(i, _)| slot == kentos_ui::widget::docking::Slot::Docked(Side::Right, *i))
                .and_then(|(_, stack)| stack.active())
        });
        let keeper = &mut self.layout;
        keeper.keep("rightVisible", Value::from(visible), now);
        keeper.keep(
            "dockTab",
            Value::from(match in_front {
                Some(Panel::Processing) => "processing",
                Some(Panel::Blocks) => "blocks",
                _ => "layers",
            }),
            now,
        );
        keeper.keep("bottomExpanded", Value::from(self.command_expanded), now);
        keeper.keep("bottomTab", Value::from(tab_key(self.bottom_tab)), now);
        keeper.keep(
            "processingTab",
            Value::from(match self.processing.panel.tab {
                ProcessingTab::Tools => "tools",
                ProcessingTab::History => "history",
            }),
            now,
        );
        let folded = &self.processing.panel.folded;
        let same = keeper.texts("processingFolded").count() == folded.len()
            && keeper
                .texts("processingFolded")
                .all(|id| folded.contains(id));
        if !same {
            keeper.keep(
                "processingFolded",
                Value::from(folded.iter().cloned().collect::<Vec<_>>()),
                now,
            );
        }
        keeper.keep("ribbonTab", Value::from(self.tab), now);
        keeper.keep("ribbonCollapsed", Value::from(self.ribbon_collapsed), now);
    }

    /// When the kept layout is written.
    pub(crate) fn layout_subscription(&self) -> Subscription<Message> {
        match self.layout.due() {
            Some(due) => Subscription::run_with(due, wake_at),
            None => Subscription::none(),
        }
    }
}

impl App {
    /// The bottom panel's lists' least and most heights: the web's panel
    /// limits in this window, less the tab row.
    pub(crate) fn bottom_log_range(&self) -> (f32, f32) {
        let bar = Self::bottom_bar();
        let most = f64::from(self.window_size.height) * BOTTOM_HEIGHT.max_share.unwrap_or(1.0);
        (BOTTOM_HEIGHT.min as f32 - bar, most as f32 - bar)
    }
}
