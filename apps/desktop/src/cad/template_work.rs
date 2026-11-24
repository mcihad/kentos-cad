//! Şablonlar, the object templates every module draws with (docs/adr/0176; docs/adr/9195 §5: CBS keeps them too, as
//! its feature templates): the panel, drawing with a template, the editor and Şablonu kaldır. They are the core's, not a
//! module's (docs/adr/9199 §4.8): their state is `App::template_work`, their messages `Message::TemplateWork`, their
//! windows `Dialog::TemplateWork`.

use iced::{Element, Task};

use crate::app::{App, Message};

/// The templates' panel, the template being drawn with and the editor.
#[derive(Default)]
pub struct TemplateWorkState {
    /// The Şablonlar panel (templates_panel.rs), and the templates drawn with
    /// in this session, the newest first (templates.rs).
    pub templates_panel: crate::templates_panel::PanelState,
    /// Şablon düzenleyici while it is open (template_editor.rs).
    pub template_editor: Option<crate::template_editor::Editor>,
    /// The object template being drawn with (templates.rs, docs/adr/0176 §3):
    /// its stamp is in the tools' context while its tool runs.
    pub template: Option<crate::templates::TemplateRun>,
}

/// The templates' messages, `Message::TemplateWork`.
#[derive(Debug, Clone)]
pub enum Event {
    /// Şablonlar panel (templates_panel.rs).
    TemplatesPanel(crate::templates_panel::Event),
    /// Şablon düzenleyici (template_editor.rs).
    TemplateEditor(crate::template_editor::Event),
}

/// The templates' windows, `Dialog::TemplateWork`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateDialogKind {
    /// Şablonlar → Sil (templates_panel.rs); the template is the panel's `deleting`.
    RemoveTemplate,
    /// Şablon düzenleyici (template_editor.rs); its form is `ArchUiState::template_editor`.
    TemplateEditor,
}

impl App {
    /// A message of the templates; a window's own handler is in its file.
    pub(crate) fn template_work_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::TemplatesPanel(event) => self.templates_panel_event(event),
            Event::TemplateEditor(event) => self.template_editor_event(event),
        }
    }

    /// A window of the templates.
    pub(crate) fn template_work_dialog_view(&self, kind: TemplateDialogKind) -> Element<'_, Message> {
        match kind {
            TemplateDialogKind::RemoveTemplate => self.remove_template_question(),
            TemplateDialogKind::TemplateEditor => self.template_editor_view(),
        }
    }
}
