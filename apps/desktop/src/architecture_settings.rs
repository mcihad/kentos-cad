//! Active tool defaults, separate from the selected object's inspector.
use crate::app::{App, Message};
use iced::widget::{column, container};
use iced::{Element, Fill, Task};
use kentos_ui::label;
use kentos_ui::widget::property_grid::{self, PropertySheet};
use kentos_ui::widget::{EditCell, Menu};

#[derive(Clone, Debug)]
pub struct Commit {
    pub tool: &'static str,
    pub key: &'static str,
    pub value: String,
}

impl App {
    pub(crate) fn architecture_settings_view(&self) -> Element<'_, Message> {
        let rows = self.session.settings(&self.format());
        if rows.is_empty() {
            return iced::widget::space().height(0).into();
        }
        let tool = self.session.tool_id();
        let mut sheet = PropertySheet::new();
        for setting in rows {
            let key = setting.key;
            let cell: Element<'_, Message> = if !setting.editable {
                property_grid::value(format!("{} (sabit)", setting.value), false, None)
            } else if setting.choices.is_empty() {
                EditCell::new(setting.value, move |value| {
                    Message::ToolSetting(Commit { tool, key, value })
                })
                .into()
            } else {
                let choices = setting.choices;
                property_grid::choice(setting.value, None, move || {
                    let mut menu = Menu::new();
                    for c in &choices {
                        menu = if let Some(command) = c.command {
                            menu.item(c.label.clone(), Message::Run(command))
                        } else {
                            menu.item(
                                c.label.clone(),
                                Message::ToolSetting(Commit {
                                    tool,
                                    key,
                                    value: c.typed.clone(),
                                }),
                            )
                        };
                    }
                    menu
                })
            };
            sheet = sheet.row(setting.label, cell);
        }
        column![
            container(
                column![
                    label::strong("Yeni nesnenin araç ayarları"),
                    label::caption("Enter veya alandan çıkış uygular")
                ]
                .spacing(4)
            )
            .padding([6, 8]),
            sheet,
            container(label::strong("Seçili nesnenin özellikleri")).padding([10, 12]),
        ]
        .width(Fill)
        .into()
    }
    pub(crate) fn tool_setting(&mut self, event: Commit) -> Task<Message> {
        if self.session.tool_id() != event.tool || !self.session.is_running() {
            return Task::none();
        }
        if !self
            .session
            .settings(&self.format())
            .iter()
            .any(|s| s.key == event.key && s.editable)
        {
            return Task::none();
        }
        // The tool says why an invalid commit is refused. Do not replace its specific explanation.
        self.with_tool(|s, cx| s.configure(event.key, &event.value, cx));
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files_testing::{app_with_drawing, make_cad};
    #[test]
    fn defaults_are_separate_from_the_selection_and_stale_commits_are_ignored() {
        let mut app = app_with_drawing();
        make_cad(&mut app);
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let _ = app.start_tool("wallRectangle");
        let _ = app.tool_setting(Commit {
            tool: "wallRectangle",
            key: "E",
            value: "250mm".into(),
        });
        assert_eq!(app.memory.wall_width, 0.25);
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
        let _ = app.start_tool("door");
        let _ = app.tool_setting(Commit {
            tool: "wallRectangle",
            key: "E",
            value: "5m".into(),
        });
        assert_eq!(app.memory.wall_width, 0.25);
        let _ = app.tool_setting(Commit {
            tool: "door",
            key: "E",
            value: "900mm".into(),
        });
        assert!(
            app.session
                .settings(&app.format())
                .iter()
                .any(|r| r.key == "E" && r.value.contains("0.900"))
        );
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
    }
    #[test]
    fn starting_an_architectural_tool_reveals_its_settings_but_regular_line_does_not() {
        let mut app = app_with_drawing();
        make_cad(&mut app);
        app.docks.close(crate::app::Panel::Properties);
        let _ = app.start_tool("line");
        assert!(!app.docks.is_shown(crate::app::Panel::Properties));
        let _ = app.start_tool("space");
        assert!(app.docks.is_shown(crate::app::Panel::Properties));
    }
}
