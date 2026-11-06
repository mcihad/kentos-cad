//! Ofis paletleri (docs/adr/1014): the bound standards' published tool palettes in Şablonlar, above the templates.
//! Each tool is a catalog command with its whole input; pressing it runs that command (the same registry Python and
//! MCP call), as one undo step, and the answer is said. A tool the command does not take on this drawing is shown
//! dimmed with the command's reason. The palettes are read from the packages when the panel opens and after a tool
//! runs (`cad.standards.palettes`, which checks each tool on a copy of the drawing): not on every edit.

use iced::widget::{button, column, container, row};
use iced::{Element, Fill, Task};
use kentos_commands::{CallOp, PaletteState, StandardsPalettes};
use kentos_interaction::Level;
use kentos_ui::{label, style};

use crate::app::{App, Message};

/// The palettes as last read.
#[derive(Debug, Default)]
pub(crate) struct OfficePalettes {
    pub(crate) palettes: Vec<PaletteState>,
}

impl App {
    /// Reads the bound standards' palettes again.
    pub(crate) fn read_office_palettes(&mut self) {
        let Some(doc) = self.document.as_mut() else {
            self.office_palettes.palettes.clear();
            return;
        };
        let cx = kentos_native_application::ExecutionContext::new(&mut doc.model);
        self.office_palettes.palettes = match kentos_native_application::standards_palettes::plan(&cx, &StandardsPalettes {}) {
            kentos_commands::CommandResult::Completed { output, .. } => output.palettes,
            _ => Vec::new(),
        };
    }

    /// A tool pressed: its command run with its input.
    pub(crate) fn run_palette_tool(&mut self, key: &str) -> Task<Message> {
        let Some(tool) = self
            .office_palettes
            .palettes
            .iter()
            .flat_map(|p| &p.tools)
            .find(|t| t.key == key)
            .cloned()
        else {
            return Task::none();
        };
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let answer = kentos_native_application::registry::run(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            &tool.tool.command,
            CallOp::Execute,
            tool.tool.input.clone(),
        );
        match answer {
            Ok(v) if v["status"] == "completed" => {
                self.say(Level::Success, format!("“{}” aracı çalıştı ({}). Geri almak için Geri al.", tool.tool.title, tool.tool.command));
            }
            Ok(v) => {
                let words = v["error"]["message"].as_str().unwrap_or("komut reddetti").to_owned();
                self.say(Level::Error, format!("“{}” aracı çalışmadı: {words}", tool.tool.title));
            }
            Err(unrun) => self.say(Level::Error, format!("“{}” aracı çalışmadı: {}", tool.tool.title, unrun.message)),
        }
        self.read_office_palettes();
        Task::none()
    }

    /// The section above the templates; nothing when no bound standard publishes a palette.
    pub(crate) fn office_palettes_view(&self) -> Option<Element<'_, Message>> {
        if self.office_palettes.palettes.is_empty() {
            return None;
        }
        let mut body = column![label::caption("Ofis paletleri")].spacing(4);
        for p in &self.office_palettes.palettes {
            body = body.push(label::strong(format!("{} · {} {}", p.name, p.standard, p.revision)));
            for t in &p.tools {
                let words = match &t.problem {
                    None => t.tool.title.clone(),
                    Some(problem) => format!("{} — {}", t.tool.title, problem.message),
                };
                let press = t.ready.then(|| {
                    Message::TemplatesPanel(crate::templates_panel::Event::Tool(t.key.clone()))
                });
                body = body.push(
                    button(row![label::body(words)].width(Fill))
                        .width(Fill)
                        .padding([3, 8])
                        .style(style::button::ghost)
                        .on_press_maybe(press),
                );
            }
        }
        Some(container(body).padding([6, 8]).width(Fill).into())
    }
}

/// The picture of docs/adr/1014: Şablonlar with Ofis A's palette, one tool ready and one dimmed with its reason.
/// `KENTOS_SHOTS_ONLY=ofis-paleti cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture`.
#[cfg(test)]
pub(crate) fn scenes() -> Vec<crate::tools_screens::Scene> {
    vec![("ofis-paleti", |app| {
        tests::bound(app);
        let _ = app.show_templates_panel();
    })]
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    /// The test drawing with Ofis A bound: a palette of a line on the drawing's first layer and one on a layer it
    /// lacks.
    pub(crate) fn bound(app: &mut App) {
        crate::files_testing::make_cad(app);
        let doc = app.document.as_mut().expect("a drawing");
        let layer = doc.model.layers().active().to_owned();
        let package = json!({"format": "kentos.drawing-standard", "version": 2, "id": "ofis-a", "revision": "2026.1",
            "standard": {"name": "Ofis A", "layers": []},
            "resources": {"toolPalettes": [{"id": "mimari", "name": "Mimari", "tools": [
                {"id": "duvar", "title": "Duvar çizgisi", "command": "cad.line.create", "commandVersion": 1,
                 "input": {"layerId": layer, "a": {"x": 487000.0, "y": 4420000.0}, "b": {"x": 487010.0, "y": 4420000.0}}},
                {"id": "eksik", "title": "Eksik katmana", "command": "cad.line.create", "commandVersion": 1,
                 "input": {"layerId": "yok", "a": {"x": 0.0, "y": 0.0}, "b": {"x": 1.0, "y": 0.0}}}]}]}});
        let mut settings = doc.settings().clone();
        settings.standard_docs = vec![kentos_commands::ProjectStandardDocument { text: package.to_string() }];
        doc.model.set_settings(settings);
    }

    #[test]
    fn a_ready_tool_runs_its_command_and_a_dimmed_one_cannot_be_pressed() {
        let mut app = crate::files_testing::app_with_drawing();
        bound(&mut app);
        let _ = app.show_templates_panel();
        let tools = &app.office_palettes.palettes[0].tools;
        assert!(tools[0].ready);
        assert!(!tools[1].ready);
        assert_eq!(tools[1].problem.as_ref().map(|p| p.code.as_str()), Some("layer_not_found"));
        let before = app.document.as_ref().unwrap().entity_count();
        let key = tools[0].key.clone();
        let _ = app.update(Message::TemplatesPanel(crate::templates_panel::Event::Tool(key)));
        let doc = app.document.as_ref().unwrap();
        assert_eq!(doc.entity_count(), before + 1, "the tool drew its line");
        assert!(doc.model.can_undo());
        assert!(app.log.lines().any(|l| l.text.contains("“Duvar çizgisi” aracı çalıştı")));
    }
}
