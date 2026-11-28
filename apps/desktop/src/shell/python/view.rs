//! The Python tab, drawn with KentOS UI's Python console (`PythonRepl`): the
//! runs and what they printed above, the code below. Enter runs code that is
//! whole (a block ends with an empty line), Shift+Enter starts a new line,
//! Ctrl+Enter always runs; ↑ and ↓ in a one-line box, or with Ctrl, go
//! through the runs; Ctrl+Space or typing opens the completion list, whose
//! names the console's Python gives (assist.rs). The header holds the
//! desktop's own controls: Konsol | Betik, the agents' link, Betik aç…,
//! Yeniden başlat, and the signature of the call being typed.

use iced::widget::{Column, button, container, row, text};
use iced::{Element, Fill, Task, Theme};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::python::PythonRepl;
use kentos_ui::widget::{Elided, Tip, tip};

use super::Event;
use crate::app::{App, Message};

const OUTPUT: &str = "python-cikti";
/// The console's code box: Konsol puts the keyboard there.
pub(crate) const INPUT: &str = "python-girdi";

/// The output follows its newest line.
pub fn follow() -> Task<Message> {
    iced::widget::operation::snap_to_end(OUTPUT)
}

fn ev(e: Event) -> Message {
    Message::Python(e)
}

/// The call the console's cursor is in: the parameter being written, then
/// its label shortened to the room left (the whole of it in the tip).
fn signature_view(s: &super::assist::Signature) -> Element<'_, Message> {
    let mut line = row![].spacing(6).align_y(iced::Center);
    if let Some(argument) = &s.argument {
        line = line.push(
            text(format!("▸ {argument}"))
                .font(typography::mono())
                .size(typography::caption())
                .wrapping(iced::widget::text::Wrapping::None)
                .style(|theme: &Theme| text::Style {
                    color: Some(Tokens::of(theme).accent),
                }),
        );
    }
    line = line.push(
        Elided::new(s.label.as_str())
            .font(typography::mono())
            .size(typography::caption())
            .width(Fill),
    );
    let about = if s.doc.is_empty() {
        Tip::new(s.label.clone())
    } else {
        Tip::new(s.label.clone()).body(s.doc.clone())
    };
    tip(
        container(line)
            .padding([2, 8])
            .width(Fill)
            .style(style::container::keycap),
        about,
        iced::widget::tooltip::Position::Top,
    )
}

fn small(glyph: Icon, about: Tip, message: Option<Message>) -> Element<'static, Message> {
    tip(
        button(icon(glyph).size(15.0))
            .on_press_maybe(message)
            .padding(kentos_ui::theme::metrics::icon_small(15.0))
            .style(style::button::flat),
        about,
        iced::widget::tooltip::Position::Top,
    )
}

/// The agents' link: open (lit, with its socket and connections in its tip) or closed; a module's program says how
/// an agent names its link (docs/adr/0802).
fn link_toggle(c: &super::Console, program: Option<kentos_launch::Module>) -> Element<'_, Message> {
    let about = match &c.link {
        Some(open) => Tip::new("Ajan bağlantısını kapat").detail(format!(
            "{} · {} ajan bağlı",
            open.path.display(),
            c.agents.len()
        )),
        None => Tip::new("Ajanlara aç").detail(match program {
            None => "MCP'deki bir ajan açık çizimi okuyup komutlarla yazabilir".to_owned(),
            Some(_) => format!(
                "MCP'deki bir ajan ({}) açık çizimi okuyup komutlarla yazabilir",
                super::link::attach_words(program)
            ),
        }),
    };
    tip(
        button(icon(Icon::Link).size(15.0))
            .on_press(ev(Event::Link))
            .padding(kentos_ui::theme::metrics::icon_small(15.0))
            .style(style::button::toggle(c.link.is_some())),
        about,
        iced::widget::tooltip::Position::Top,
    )
}

/// Konsol | Betik, at the start of each side's bar.
pub(super) fn mode_switch(mode: super::Mode) -> Element<'static, Message> {
    let side = |label: &'static str, this: super::Mode| {
        button(text(label).size(typography::body()))
            .on_press(ev(Event::Mode(this)))
            .padding([3, 10])
            .style(style::button::segment(mode == this))
    };
    row![
        side("Konsol", super::Mode::Console),
        side("Betik", super::Mode::Script)
    ]
    .spacing(0)
    .into()
}

impl App {
    /// The bottom panel's Python tab: the console, or the script beside it.
    pub(crate) fn python_tab(&self) -> Element<'_, Message> {
        match self.python.mode {
            super::Mode::Console => self.python_console(true),
            super::Mode::Script => self
                .python_script_view(mode_switch(super::Mode::Script), self.python_console(false)),
        }
    }

    /// The code box's first example, fitting the project (GIS-021): a CBS project's reads its layers, changing
    /// nothing; a CAD project's draws.
    pub(crate) fn python_example(&self) -> &'static str {
        let gis = self
            .document
            .as_ref()
            .is_some_and(|d| d.settings().workspace == Some(kentos_commands::Workspace::Gis));
        if gis {
            "print([k.name for k in doc.layers()])"
        } else {
            "cad.polygon.create(doc, layer_id=…, pts=[…])"
        }
    }

    /// The console: KentOS UI's REPL, its header carrying the desktop's
    /// controls (with `switch`, Konsol | Betik first).
    fn python_console(&self, switch: bool) -> Element<'_, Message> {
        let c = &self.python;
        let running = c.running.is_some();
        let restart_tip = match &c.ready {
            Some(ready) => Tip::new("Yeniden başlat").detail(ready.clone()),
            None => Tip::new("Yeniden başlat")
                .detail("Kod kendi Python'unda çalışır: doc açık çizim, cad kentos.cad"),
        };
        // Konsol | Betik and Betik aç… on the Konsol side; beside the script
        // its own bar has them.
        let mut tools = row![].spacing(4).align_y(iced::Center);
        if switch {
            tools = tools.push(mode_switch(super::Mode::Console)).push(small(
                Icon::Open,
                Tip::new("Betik aç…").detail(".py dosyasını çalıştırır"),
                (!running).then_some(ev(Event::Open)),
            ));
        }
        // Yeniden başlat after a start that failed too: it looks at the Python chosen now (docs/adr/9336).
        let unready = c.failed.is_some() || matches!(c.check, Some(Err(_)));
        tools = tools
            .push(small(
                Icon::Retry,
                restart_tip,
                (c.started() || running || unready).then_some(ev(Event::Restart)),
            ))
            .push(link_toggle(c, self.program));
        if let Some(signature) = &c.signature
            && !running
        {
            tools = tools.push(signature_view(signature));
        }
        // Compact: the bottom panel is short; Çalıştır sits beside the code box.
        let example = self.python_example();
        let greeting = c.greeting();
        let repl = PythonRepl::new(&c.repl, |e| ev(Event::Repl(e)))
            .title("Python")
            .placeholder(example)
            .greeting(greeting)
            .idle(match (&c.ready, c.started()) {
                (Some(_), _) => None,
                (None, true) => Some("Denetleniyor"),
                (None, false) => Some("Başlatılmadı"),
            })
            .output_id(OUTPUT)
            .input_id(INPUT)
            .compact(true)
            .toolbar(tools);
        // The head (docs/adr/9336): which Python and how it stands; the way to set one up when it holds no kentos.
        let mut column = Column::new();
        if let Some((words, setup)) = c.head() {
            let said = kentos_ui::label::caption(words).style(if setup {
                kentos_ui::style::text::warning
            } else {
                kentos_ui::style::text::muted
            });
            let mut line = iced::widget::row![container(said).width(Fill)]
                .spacing(8)
                .align_y(iced::Center);
            if setup {
                line = line.push(crate::exchange::words::secondary(
                    "Python ortamını yapılandır",
                    Some(ev(Event::Configure)),
                ));
            }
            column = column.push(container(line).padding([4, 10]).width(Fill));
        }
        column
            .push(container(repl).padding([6, 8]).width(Fill).height(Fill))
            .height(Fill)
            .into()
    }
}
