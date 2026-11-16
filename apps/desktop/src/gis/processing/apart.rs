//! İşlemler's tool window in a window of its own (docs/adr/8060). Ayrı pencerede aç in its header takes it out of the
//! main window into an operating-system window that can go to another screen; Ana pencereye al brings it back over
//! the drawing. Apart, the main window stays free: it takes the mouse and the keyboard as usual (pan, zoom, select,
//! the panels) while the tool runs from its own window with its progress, Durdur and results as over the drawing.
//! The choice is kept (shell/windows/memory.rs): the next tool opens the same way, in the next run too.
//!
//! It is a window of its own through the windows' contract ([`ToolWindow`], shell/windows/surface.rs): moving it, its
//! header button, its framing and the choice are the windows'; this module gives what is the tool window's own (its
//! title, its view, its close, its keys, a pick in the main window).
//!
//! While one of its fields is picked on the drawing (Sahneden seç) the window says what the main window asks and the
//! main window gets the keyboard; the form comes back when the pick ends. İfade oluşturucu, larger than the tool
//! window, opens over the main window, which takes the keyboard for it.

use iced::keyboard::key::Named;
use iced::widget::{Row, button, column, row, space};
use iced::{Center, Element, Task};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::theme::{metrics, typography};
use kentos_ui::widget::Dialog;
use kentos_ui::{label, style};

use super::{Event, Picking, ToolDialog};
use crate::app::{App, Dialog as Asking, Message};
use crate::gis::GisDialogKind;
use crate::keys::KeyPress;
use crate::windows::{OwnWindow, Surface};

/// The surface this module shows (shell/windows/).
const SURFACE: Surface = Surface::ProcessingTool;

/// The header's button: out of the main window into a window of its own, or back into it.
pub(super) fn placed<'a>(dialog: Dialog<'a, Message>, apart: bool) -> Dialog<'a, Message> {
    crate::windows::apart_button(dialog, SURFACE, apart)
}

/// İşlemler's tool window as a window of its own (shell/windows/surface.rs).
pub(crate) struct ToolWindow;

impl OwnWindow for ToolWindow {
    fn kept_as(&self) -> &'static str {
        "islem-araci"
    }

    /// The in-canvas window's 940 × 660.
    fn sizes(&self) -> (iced::Size, iced::Size) {
        (iced::Size::new(940.0, 660.0), iced::Size::new(640.0, 420.0))
    }

    fn what(&self) -> &'static str {
        "aracı"
    }

    fn apart_tip(&self) -> &'static str {
        "Aracı kendi penceresinde açar: pencereyi başka bir ekrana taşıyıp aracı çalıştırırken haritayı izleyebilirsiniz. Sonraki araçlar da ayrı pencerede açılır."
    }

    fn return_tip(&self) -> &'static str {
        "Aracı yeniden ana pencerede, çizimin üstünde açar. Sonraki araçlar da orada açılır."
    }

    fn dialog(&self) -> Asking {
        Asking::Gis(GisDialogKind::Processing)
    }

    /// A tool open, or one of its fields picked on the drawing.
    fn is_open(&self, app: &App) -> bool {
        app.gis.processing.dialog.is_some() || app.processing_picking().is_some()
    }

    /// Over the drawing only with its form: a pick on the drawing waits in its own window.
    fn has_dialog(&self, app: &App) -> bool {
        app.gis.processing.dialog.is_some()
    }

    fn work_in_main(&self, app: &App) -> bool {
        app.processing_work_in_main()
    }

    fn title(&self, app: &App) -> String {
        app.processing_window_title()
    }

    fn view<'a>(&self, app: &'a App) -> Element<'a, Message> {
        app.processing_window_view()
    }

    fn close(&self, app: &mut App) -> Task<Message> {
        app.processing_window_closed()
    }

    fn on_key(&self, app: &mut App, press: &KeyPress) -> Task<Message> {
        app.processing_window_key(press)
    }
}

impl App {
    /// Whether the tool windows open in a window of their own: the user's last choice, kept between runs.
    pub(crate) fn processing_is_apart(&self) -> bool {
        self.is_apart(SURFACE)
    }

    /// Whether the tool's work is in the main window now: a field picked on the drawing, or İfade oluşturucu open
    /// for one of its fields. The main window takes the keyboard meanwhile (shell/windows/app.rs).
    pub(crate) fn processing_work_in_main(&self) -> bool {
        self.processing_picking().is_some()
            || self
                .gis
                .builder
                .as_ref()
                .is_some_and(|b| matches!(b.target, crate::expression::Target::Processing(_)))
    }

    /// The tool window waiting while one of its fields is picked on the drawing: the window, the field and whether
    /// it takes objects (else a point).
    fn processing_picking(&self) -> Option<(&ToolDialog, &str, bool)> {
        match &self.gis.processing.picking {
            Some(Picking::Point { window, name, .. }) => Some((window, name, false)),
            Some(Picking::Objects { window, name, .. }) => Some((window, name, true)),
            _ => None,
        }
    }

    /// The tool's own window's name: the tool's (“Tampon”), or its Toplu çalıştır's.
    pub(crate) fn processing_window_title(&self) -> String {
        let tool = self
            .gis
            .processing
            .dialog
            .as_ref()
            .or_else(|| self.processing_picking().map(|(window, ..)| window))
            .map_or_else(|| "İşlemler".to_owned(), |w| w.tool.label.clone());
        if self.gis.processing.batch.is_some() {
            format!("Toplu çalıştır: {tool}")
        } else {
            tool
        }
    }

    /// The tool shows: in its own window when apart (given the keyboard), else over the drawing.
    pub(crate) fn show_processing(&mut self) {
        self.show_surface(SURFACE);
    }

    /// The tool window's place over the drawing let go; another window of the main window stays open.
    pub(crate) fn processing_dialog_away(&mut self) {
        self.surface_away(SURFACE);
    }

    /// The tool's own window: its form or Toplu çalıştır's table with Ana pencereye al in the header; while a field
    /// is picked on the drawing, what the main window asks for it.
    pub(crate) fn processing_window_view(&self) -> Element<'_, Message> {
        if let Some((window, name, objects)) = self.processing_picking() {
            return picking_view(window, name, objects);
        }
        match self.processing_box() {
            Some((dialog, _)) => placed(dialog, true).window(),
            None => space().into(),
        }
    }

    /// The tool's own window closed by its close button: Kapat. A pick going on is given up first, as Esc would.
    pub(crate) fn processing_window_closed(&mut self) -> Task<Message> {
        let gave_up = if self.processing_picking().is_some() {
            self.run("tool.cancel")
        } else {
            Task::none()
        };
        if self.processing_picking().is_some() {
            self.gis.processing.picking = None;
        }
        Task::batch([gave_up, self.processing_event(Event::Close)])
    }

    /// A key no field of the tool's own window took, as over the drawing: Esc is Kapat (Vazgeç while a field is
    /// picked), Ctrl+Enter Çalıştır, Tab goes round the window's fields. Nothing else: the main window's shortcuts
    /// and the drawing's commands stay in the main window.
    pub(crate) fn processing_window_key(&mut self, press: &KeyPress) -> Task<Message> {
        if self.processing_picking().is_some() {
            return match press.named() {
                Some(Named::Escape) => self.run("tool.cancel"),
                _ => Task::none(),
            };
        }
        match press.named() {
            Some(Named::Tab) if !press.modifiers.control() && !press.modifiers.alt() => {
                kentos_ui::focus::next(press.modifiers.shift())
            }
            Some(Named::Escape) => self.processing_event(Event::Close),
            Some(Named::Enter) if press.modifiers.control() => self.processing_event(Event::Run),
            _ => Task::none(),
        }
    }
}

/// While a field is picked on the drawing the main window has the work: the tool's window says what it asks there,
/// with Vazgeç (and Bitti for objects), the main window's own answers.
fn picking_view<'a>(window: &'a ToolDialog, name: &str, objects: bool) -> Element<'a, Message> {
    let field = window
        .tool
        .parameters
        .iter()
        .find(|p| p.name == name)
        .map_or_else(|| name.to_owned(), |p| p.label.clone());
    let ask = if objects {
        format!(
            "{field}: ana pencerede nesneleri tıklayın ya da pencereyle seçin. Bitti seçilenleri alır, Vazgeç önceki seçimi geri getirir."
        )
    } else {
        format!(
            "{field}: ana pencerede haritada bir nokta gösterin ya da komut satırına Y,X yazın. Vazgeç alanı olduğu gibi bırakır."
        )
    };
    let answer = |text: &'static str, primary: bool, command: &'static str| {
        button(label::body(text))
            .padding(metrics::action())
            .style(if primary {
                style::button::primary
            } else {
                style::button::secondary
            })
            .on_press(Message::Run(command))
    };
    let mut answers = Row::new()
        .spacing(6)
        .align_y(Center)
        .push(space::horizontal())
        .push(answer("Vazgeç", false, "tool.cancel"));
    if objects {
        answers = answers.push(answer("Bitti", true, "tool.confirm"));
    }
    Dialog::new(window.tool.label.clone())
        .push(
            column![
                row![
                    icon(Icon::Crosshair).size(16.0).tone(Tone::Accent),
                    label::body("Çizimde seçiliyor").font(typography::ui_strong()),
                ]
                .spacing(8)
                .align_y(Center),
                label::body(ask).style(style::text::muted),
            ]
            .spacing(8),
        )
        .footer(answers)
        .window()
}
