//! The right dock's İşlemler tab (the web's `ProcessingPanel`, QGIS's
//! Processing Toolbox): Araçlar, the tools by category with the Modeller
//! branch first and a search box; Geçmiş, this session's runs. A tool's row
//! opens its window with one click (the toolbox is a launcher, not a
//! selection list); a run opens again with its values, and selects what it
//! made while those objects are still there.

use std::collections::BTreeSet;
use std::fmt;

use iced::widget::{Column, button, column, container, row, scrollable, text_input};
use iced::{Alignment, Center, Element, Fill};
use kentos_processing::registry::CategoryNode;
use kentos_processing::text::fold_turkish;
use kentos_processing::{RunRecord, Status};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::tree_view::{Column as TreeColumn, Node, TreeView};
use kentos_ui::widget::{Segmented, Tip, tip};

use crate::app::{App, Message};
use crate::catalog::catalog;

/// Which view of the tab is in front.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Tools,
    History,
}

/// The tab's state: the view, the search and the folded categories.
#[derive(Debug, Default)]
pub struct PanelState {
    pub tab: Tab,
    pub search: String,
    /// Categories closed by hand; a search opens every one.
    pub folded: BTreeSet<String>,
}

/// What the tab asks for.
#[derive(Clone, Debug)]
pub enum Event {
    Tab(Tab),
    Search(String),
    /// A category opened or closed.
    Fold(String),
    /// Yeniden aç: a run's window again, with its values (by the run's number).
    Reopen(u64),
    /// n nesneyi seç: what a run made, while it is there.
    SelectRun(u64),
    /// Modeli düzenle (a user's model) or Kopyasını düzenle (a built-in one).
    EditModel(String),
    /// A user's model opened to run (it has no command of the catalog's).
    RunModel(String),
}

/// The switch's buttons: Araçlar, and Geçmiş with how many runs there are.
#[derive(Clone, Copy, PartialEq)]
struct TabButton(Tab, usize);

impl fmt::Display for TabButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TabButton(Tab::Tools, _) => f.write_str("Araçlar"),
            TabButton(Tab::History, 0) => f.write_str("Geçmiş"),
            TabButton(Tab::History, n) => write!(f, "Geçmiş ({n})"),
        }
    }
}

fn ev(e: Event) -> Message {
    Message::Processing(super::Event::Panel(e))
}

/// The models' branch sits first; it is not a registry category.
const MODELS: &str = "__models";

impl App {
    /// The tab's header note: “4 araç” or “3 kayıt”.
    pub(crate) fn processing_meta(&self) -> String {
        match self.processing.panel.tab {
            Tab::Tools => format!("{} araç", self.processing.registry.tools().len()),
            Tab::History => format!("{} kayıt", self.processing.runner.history().len()),
        }
    }

    /// The tab's body.
    pub(crate) fn processing_panel(&self) -> Element<'_, Message> {
        let state = &self.processing.panel;
        let runs = self.processing.runner.history();
        let switch = Segmented::new(
            [
                TabButton(Tab::Tools, runs.len()),
                TabButton(Tab::History, runs.len()),
            ],
            TabButton(state.tab, runs.len()),
            |b| ev(Event::Tab(b.0)),
        )
        .width(Fill);
        let body: Element<'_, Message> = match state.tab {
            Tab::Tools => self.processing_tools(),
            Tab::History => self.processing_history(runs),
        };
        column![container(switch).padding([6, 8]), body]
            .height(Fill)
            .into()
    }

    fn processing_tools(&self) -> Element<'_, Message> {
        let state = &self.processing.panel;
        let registry = &self.processing.registry;
        let query = state.search.trim();
        let search = container(
            row![
                icon(Icon::Search).size(13.0).tone(Tone::Muted),
                text_input("İşlem ara: numara, kenar, parsel…", &state.search)
                    .on_input(|t| ev(Event::Search(t)))
                    .font(typography::ui())
                    .size(typography::body())
                    .padding([3, 0])
                    .style(style::field::bare_input),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([0, 8])
        .style(style::container::field_box);
        let open = |id: &str| !query.is_empty() || !state.folded.contains(id);
        let hits: Option<BTreeSet<String>> = (!query.is_empty()).then(|| {
            registry
                .search(query)
                .iter()
                .map(|t| t.id.clone())
                .collect()
        });
        let folded_query = fold_turkish(query);
        let mut roots: Vec<Node<'_, Message>> = Vec::new();
        // Modeller: the models (a search matches their name and description), then Yeni model….
        let models: Vec<_> = registry
            .models()
            .iter()
            .filter(|m| {
                query.is_empty()
                    || fold_turkish(&format!("{} {}", m.label, m.description))
                        .contains(&folded_query)
            })
            .collect();
        if !models.is_empty() || query.is_empty() {
            let mut branch =
                category_row("Modeller", "processing", models.len(), open(MODELS), MODELS);
            for m in &models {
                let command = catalog()
                    .get(&format!("processing.model.{}", m.id))
                    .map(|c| c.id);
                let words = if registry.is_builtin_model(&m.id) {
                    "Kopyasını düzenle"
                } else {
                    "Modeli düzenle"
                };
                let edit = tip(
                    button(icon(crate::icons::from_web(Some("edit"))).size(14.0))
                        .padding(3)
                        .style(style::button::ghost)
                        .on_press(ev(Event::EditModel(m.id.clone()))),
                    Tip::new(words),
                    iced::widget::tooltip::Position::Left,
                );
                let mut node = Node::new(m.label.as_str())
                    .icon(icon(crate::icons::from_web(Some("processing"))).size(15.0))
                    .cells([edit]);
                node = match command {
                    Some(id) => node.on_press(Message::Run(id)),
                    None => node.on_press(ev(Event::RunModel(m.id.clone()))),
                };
                branch = branch.push(node);
            }
            if query.is_empty() {
                branch = branch.push(
                    Node::new("Yeni model…")
                        .icon(icon(crate::icons::from_web(Some("modelNew"))).size(15.0))
                        .on_press(Message::Run("processing.newModel")),
                );
            }
            roots.push(branch);
        }
        let keep = |t: &kentos_processing::Tool| hits.as_ref().is_none_or(|h| h.contains(&t.id));
        for node in registry.tree(&keep) {
            roots.push(category_node(node, &open));
        }
        let empty = if query.is_empty() {
            "Kayıtlı işlem aracı yok."
        } else {
            "Aramayla eşleşen işlem yok. Başka bir kelime deneyin."
        };
        let tree = TreeView::new([
            TreeColumn::new("Ad").width(Fill),
            TreeColumn::new("Sayı").width(36).align_right(),
        ])
        .header(false)
        .extend(roots)
        .empty(empty)
        .height(Fill);
        column![container(search).padding([0, 8]), tree]
            .spacing(6)
            .height(Fill)
            .into()
    }

    fn processing_history<'a>(&'a self, runs: &'a [RunRecord]) -> Element<'a, Message> {
        if runs.is_empty() {
            return container(
                label::body(
                    "Bu oturumda henüz işlem çalıştırılmadı. Araçlar sekmesinden bir işlem açıp çalıştırın; burada yeniden açabilirsiniz.",
                )
                .style(style::text::muted),
            )
            .padding(12)
            .into();
        }
        let list = runs.iter().fold(Column::new().spacing(1), |list, r| {
            list.push(self.run_card(r))
        });
        scrollable(list)
            .direction(style::field::body_scrollbar())
            .height(Fill)
            .into()
    }

    /// One run: its state, name, when, what it did, how long it took, and its buttons.
    fn run_card<'a>(&'a self, r: &'a RunRecord) -> Element<'a, Message> {
        let (glyph, tone) = match r.status {
            Status::Ok => (Icon::Success, Tone::Success),
            Status::Canceled => (Icon::Warning, Tone::Warning),
            Status::Error => (Icon::Error, Tone::Danger),
        };
        let alive = self.alive(r).len();
        let known = self.processing.registry.get(&r.tool_id).is_some();
        let took = if r.ms < 1000 {
            format!("{} ms", r.ms)
        } else {
            format!("{:.1} sn", r.ms as f64 / 1000.0).replace('.', ",")
        };
        let mut actions = row![].spacing(6).align_y(Center);
        if alive > 0 {
            actions = actions.push(
                button(label::caption(format!("{alive} nesneyi seç")))
                    .padding([3, 8])
                    .style(style::button::ghost)
                    .on_press(ev(Event::SelectRun(r.seq))),
            );
        }
        actions = actions.push(
            button(label::caption("Yeniden aç"))
                .padding([3, 8])
                .style(style::button::secondary)
                .on_press_maybe(known.then_some(ev(Event::Reopen(r.seq)))),
        );
        let head = row![
            label::body(r.label.clone())
                .font(typography::ui_strong())
                .width(Fill),
            label::caption(crate::cloud::words::ago_ms(r.started)).style(style::text::muted),
        ]
        .spacing(8)
        .align_y(Center);
        let foot = row![
            label::mono_caption(took)
                .style(style::text::muted)
                .width(Fill),
            actions
        ]
        .align_y(Center);
        container(
            row![
                icon(glyph).size(16.0).tone(tone),
                column![head, label::caption(r.summary.clone()), foot].spacing(4)
            ]
            .spacing(8)
            .align_y(Alignment::Start),
        )
        .padding([10, 12])
        .width(Fill)
        .into()
    }

    /// What a run made (else what it changed or selected) that is still in the drawing.
    fn alive(&self, r: &RunRecord) -> Vec<kentos_domain::Slot> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let ids = if r.added.is_empty() {
            &r.touched
        } else {
            &r.added
        };
        ids.iter()
            .copied()
            .filter(|id| doc.model.get(*id).is_some())
            .collect()
    }

    pub(crate) fn processing_panel_event(&mut self, e: Event) {
        match e {
            Event::Tab(tab) => self.processing.panel.tab = tab,
            Event::Search(text) => self.processing.panel.search = text,
            Event::Fold(id) => {
                let folded = &mut self.processing.panel.folded;
                if !folded.remove(&id) {
                    folded.insert(id);
                }
            }
            Event::Reopen(seq) => {
                let run = self
                    .processing
                    .runner
                    .history()
                    .iter()
                    .find(|r| r.seq == seq)
                    .map(|r| (r.tool_id.clone(), r.values.clone()));
                if let Some((tool, values)) = run {
                    self.open_processing(&tool, Some(values));
                }
            }
            Event::SelectRun(seq) => {
                let ids = self
                    .processing
                    .runner
                    .history()
                    .iter()
                    .find(|r| r.seq == seq)
                    .map(|r| self.alive(r))
                    .unwrap_or_default();
                if !ids.is_empty() {
                    self.selection.set(ids);
                    self.zoom_selection();
                }
            }
            Event::EditModel(id) => {
                // The model's own window steps aside for the designer (the web's).
                if self.dialog == Some(crate::app::Dialog::Processing) {
                    self.processing.dialog = None;
                    self.dialog = None;
                }
                self.open_model_designer(Some(&id));
            }
            Event::RunModel(id) => {
                let _ = self.processing_command(&format!("processing.model.{id}"));
            }
        }
    }
}

fn category_row<'a>(
    label: &'a str,
    glyph: &'a str,
    count: usize,
    open: bool,
    id: &str,
) -> Node<'a, Message> {
    Node::new(label)
        .folder()
        .icon(icon(crate::icons::from_web(Some(glyph))).size(15.0))
        .cells([label::caption(count.to_string())
            .style(style::text::muted)
            .into()])
        .expanded(open, ev(Event::Fold(id.to_owned())))
}

/// A category with its sub-categories and tools; a tool's row opens its window.
fn category_node<'a>(node: CategoryNode, open: &dyn Fn(&str) -> bool) -> Node<'a, Message> {
    let count = count_tools(&node);
    let c = node.category;
    let mut branch = category_row(c.label, c.icon, count, open(c.id), c.id);
    for child in node.children {
        branch = branch.push(category_node(child, open));
    }
    for tool in node.tools {
        let command = catalog()
            .get(&format!("processing.run.{}", tool.id))
            .map(|c| c.id);
        let mut row = Node::new(tool.label.clone())
            .icon(icon(crate::icons::from_web(tool.icon.as_deref())).size(15.0));
        if let Some(id) = command {
            row = row.on_press(Message::Run(id));
        }
        branch = branch.push(row);
    }
    branch
}

fn count_tools(node: &CategoryNode) -> usize {
    node.tools.len() + node.children.iter().map(count_tools).sum::<usize>()
}
