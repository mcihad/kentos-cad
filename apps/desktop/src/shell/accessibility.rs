//! The macOS accessibility representation of the architectural drawing controls.

#[path = "accessibility/macos.rs"]
mod macos;

use iced::advanced::widget::{Operation, operation::Outcome};
use iced::futures::channel::mpsc;
use iced::{Rectangle, Task, window};
use kentos_interaction::Level;

use crate::app::{App, COMMAND_INPUT, Message};

pub(super) const ARCH_TAB: &str = "arch";
pub(super) const COMMAND_LINE: &str = "accessibility.command-line";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Room,
    Door,
}

impl Tool {
    const ALL: [Self; 2] = [Self::Room, Self::Door];

    pub(super) fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tool| tool.id() == id)
    }

    pub(super) fn id(self) -> &'static str {
        match self {
            Self::Room => "tool.room",
            Self::Door => "tool.door",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Control {
    ArchTab,
    Tool(Tool),
    Prompt,
    Input,
}

#[derive(Clone, Debug, PartialEq)]
struct Node {
    control: Control,
    label: String,
    value: String,
    hint: String,
    enabled: bool,
    selected: bool,
    focused: bool,
}

#[derive(Clone, Debug)]
pub enum Event {
    Ready,
    Activate(Tool),
    OpenArchTab,
    FocusInput,
    SetInput(String),
    Bounds(usize, Layout),
}

#[derive(Clone, Debug, Default)]
pub struct Layout {
    tab: Option<Rectangle>,
    tools: [Option<Rectangle>; 2],
    line: Option<Rectangle>,
    input: Option<Rectangle>,
}

impl Layout {
    fn bounds(&self, control: Control) -> Option<Rectangle> {
        match control {
            Control::ArchTab => self.tab,
            Control::Tool(Tool::Room) => self.tools[0],
            Control::Tool(Tool::Door) => self.tools[1],
            Control::Prompt => {
                let mut line = self.line?;
                if let Some(input) = self.input {
                    line.width = (input.x - line.x).max(1.0);
                }
                Some(line)
            }
            Control::Input => self.input.or(self.line),
        }
    }
}

/// The traversal uses the real ribbon and line layout, including the ribbon's compact forms.
#[derive(Default)]
struct ReadLayout {
    layout: Layout,
}

impl Operation<Layout> for ReadLayout {
    fn traverse(&mut self, next: &mut dyn FnMut(&mut dyn Operation<Layout>)) {
        next(self);
    }

    fn container(&mut self, id: Option<&iced::advanced::widget::Id>, bounds: Rectangle) {
        for (place, tool) in self.layout.tools.iter_mut().zip(Tool::ALL) {
            if id == Some(&tool.id().into()) {
                *place = Some(bounds);
            }
        }
        if id == Some(&ARCH_TAB.into()) {
            self.layout.tab = Some(bounds);
        }
        if id == Some(&COMMAND_LINE.into()) {
            self.layout.line = Some(bounds);
        }
    }

    fn focusable(
        &mut self,
        id: Option<&iced::advanced::widget::Id>,
        bounds: Rectangle,
        _state: &mut dyn iced::advanced::widget::operation::Focusable,
    ) {
        if id == Some(&COMMAND_INPUT.into()) {
            self.layout.input = Some(bounds);
        }
    }

    fn finish(&self) -> Outcome<Layout> {
        Outcome::Some(self.layout.clone())
    }
}

#[derive(Default)]
pub(crate) struct State {
    sender: Option<mpsc::UnboundedSender<Event>>,
    nodes: Vec<Node>,
    bounds_version: usize,
    size: iced::Size,
    announcement: Option<String>,
    log_id: u64,
    tool_active: bool,
    ribbon_open: bool,
    status: String,
    prompt: String,
}

impl State {
    pub(crate) fn start(&mut self) -> Task<Message> {
        let (sender, events) = mpsc::unbounded();
        let _ = sender.unbounded_send(Event::Ready);
        self.sender = Some(sender);
        Task::run(events, Message::Accessibility)
    }
}

impl App {
    fn accessible_drawing(&self) -> bool {
        self.document.is_some() && !self.sheets.is_active() && !self.window_over()
    }

    fn accessible_tools(&self) -> bool {
        self.accessible_drawing() && self.shown_tab() == ARCH_TAB
    }

    pub(crate) fn follow_accessibility(&mut self, pointer_only: bool) -> Task<Message> {
        if self.accessibility.sender.is_none()
            || pointer_only && self.accessibility.log_id == self.log.last_id()
        {
            return Task::none();
        }
        let shown = self.accessible_tools();
        let active = shown && self.session.is_running();
        let ribbon_open = !self.ribbon_collapsed || self.ribbon_peek;
        let mut nodes = Vec::new();
        if self.accessible_drawing()
            && let Some(tab) = self.ribbon_tabs().find(|tab| tab.id == ARCH_TAB)
        {
            nodes.push(Node {
                control: Control::ArchTab,
                label: tab.label.to_owned(),
                value: String::new(),
                hint: String::new(),
                enabled: true,
                selected: shown && ribbon_open,
                focused: false,
            });
        }
        if shown {
            for tool in Tool::ALL {
                let command = crate::catalog::catalog()
                    .get(tool.id())
                    .expect("ribbon command");
                nodes.push(Node {
                    control: Control::Tool(tool),
                    label: command.short.trim_end_matches('…').to_owned(),
                    value: String::new(),
                    hint: self.why_disabled(tool.id()).unwrap_or_default(),
                    enabled: self.available(tool.id()),
                    selected: self.session.is_running()
                        && tool.id().strip_prefix("tool.") == Some(self.session.tool_id()),
                    focused: false,
                });
            }
            let hint = self.prompt();
            let prompt = hint
                .title()
                .filter(|_| active)
                .map(|title| {
                    let step = self.step_words(&hint.step_with_notes());
                    let options = hint.options.iter().map(|option| match &option.value {
                        Some(value) => format!("{} ({}): {value}", option.label, option.key),
                        None => format!("{} ({})", option.label, option.key),
                    });
                    std::iter::once(format!("{title}. {step}."))
                        .chain(options)
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();
            let result = (active || self.accessibility.tool_active)
                .then(|| {
                    self.log.lines().next_back().filter(|line| {
                        line.id > self.accessibility.log_id
                            && matches!(line.level, Level::Success | Level::Warn | Level::Error)
                    })
                })
                .flatten();
            let state = &mut self.accessibility;
            if let Some(line) = result {
                state.status.clone_from(&line.text);
            } else if prompt != state.prompt {
                state.status.clear();
            }
            let prompt_changed = prompt != state.prompt;
            state.prompt = prompt;
            let status = [state.status.as_str(), state.prompt.as_str()]
                .into_iter()
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            if !status.is_empty() {
                if result.is_some() || prompt_changed {
                    // Options are available when reading the status again. Movement to a new
                    // drawing step announces just the result or the new instruction.
                    state.announcement = result.map(|line| line.text.clone()).or_else(|| {
                        hint.title()
                            .map(|title| format!("{title}. {}", hint.step_with_notes()))
                    });
                }
                nodes.push(Node {
                    control: Control::Prompt,
                    label: "Komut durumu".into(),
                    value: status,
                    hint: String::new(),
                    enabled: true,
                    selected: false,
                    focused: false,
                });
            }
            nodes.push(Node {
                control: Control::Input,
                label: "Komut satırı".into(),
                value: self.command_input.clone(),
                hint: "Komut, seçenek veya koordinat yazın. Enter onaylar.".into(),
                enabled: true,
                selected: false,
                focused: self.line_focused,
            });
        }
        let state = &mut self.accessibility;
        state.tool_active = active;
        state.log_id = self.log.last_id();
        if !shown {
            state.status.clear();
            state.prompt.clear();
            state.announcement = None;
        }
        if nodes == state.nodes
            && state.size == self.window_size
            && ribbon_open == state.ribbon_open
        {
            return Task::none();
        }
        state.ribbon_open = ribbon_open;
        state.nodes = nodes;
        state.size = self.window_size;
        state.bounds_version += 1;
        let version = state.bounds_version;
        iced::advanced::widget::operate(ReadLayout::default())
            .map(move |layout| Message::Accessibility(Event::Bounds(version, layout)))
    }

    pub(crate) fn accessibility_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Ready => Task::none(),
            Event::OpenArchTab
                if self.accessible_drawing()
                    && self.ribbon_tabs().any(|tab| tab.id == ARCH_TAB) =>
            {
                if self.shown_tab() != ARCH_TAB || self.ribbon_collapsed && !self.ribbon_peek {
                    return Task::done(Message::RibbonTab(ARCH_TAB));
                }
                Task::none()
            }
            Event::Activate(tool) if self.accessible_tools() && self.available(tool.id()) => {
                self.run(tool.id()).chain(self.run("commandline.focus"))
            }
            Event::FocusInput if self.accessible_tools() => self.run("commandline.focus"),
            Event::SetInput(value) if self.accessible_tools() => {
                self.command_input = value;
                self.run("commandline.focus")
            }
            Event::Bounds(version, layout) if version == self.accessibility.bounds_version => {
                let Some(window) = self.windows.main() else {
                    return Task::none();
                };
                let Some(sender) = self.accessibility.sender.clone() else {
                    return Task::none();
                };
                let elements: Vec<_> = self
                    .accessibility
                    .nodes
                    .iter()
                    .filter_map(|node| Some((node.clone(), layout.bounds(node.control)?)))
                    .collect();
                let announcement = self.accessibility.announcement.take();
                window::run(window, move |window| {
                    macos::sync(window, elements, sender, announcement.as_deref());
                })
                .discard()
            }
            _ => Task::none(),
        }
    }
}
