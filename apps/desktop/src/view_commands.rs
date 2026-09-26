//! Commands of the web's Görünüm and Araçlar menus that arrange the window
//! or ask the server (app/commands.ts): the side panels (F4), full screen,
//! the command search (Alt+Q) and the server check. Koordinat sistemi… is
//! Proje ayarları on its page (project/).

use iced::Task;
use iced::window;
use kentos_cloud::Cloud;
use kentos_contracts::{CONTRACTS_VERSION, Health};
use kentos_interaction::Level;
use kentos_ui::widget::command_line;
use kentos_ui::widget::docking::Side;

use crate::app::{App, COMMAND_INPUT, Message, Panel};

/// The web command ids this module runs.
pub const COMMANDS: [&str; 4] = [
    "view.rightPanel",
    "view.fullscreen",
    "view.commandSearch",
    "server.check",
];

/// The side panels F4 hides and shows together (the web's right panel).
const SIDE_PANELS: [Panel; 2] = [Panel::Layers, Panel::Properties];

impl App {
    pub(crate) fn view_command(&mut self, id: &'static str) -> Task<Message> {
        match id {
            "view.rightPanel" => self.toggle_right_panel(),
            "view.fullscreen" => return self.toggle_fullscreen(),
            // The ribbon has no search box yet: the command line lists every
            // command and finds one by its name or alias (the web's classic way).
            "view.commandSearch" => {
                self.command_input.clear();
                return command_line::show_commands(COMMAND_INPUT);
            }
            "server.check" => return self.check_server(),
            _ => {}
        }
        Task::none()
    }

    /// `view.rightPanel` (F4): the layers and properties panels go away
    /// together and come back as they were: their sides, tabs, sizes and
    /// floating windows (the web hides its right panel whole).
    fn toggle_right_panel(&mut self) {
        if self.right_panel_shown() {
            self.hidden_docks = Some(self.docks.clone());
            for panel in SIDE_PANELS {
                self.docks.close(panel);
            }
        } else {
            match self.hidden_docks.take() {
                Some(docks) => self.docks = docks,
                // Folded to their titles: opened where they are.
                None => {
                    for panel in SIDE_PANELS {
                        self.docks.show(panel, Side::Right);
                    }
                }
            }
        }
    }

    /// Whether either side panel is on screen (the command's check mark).
    pub(crate) fn right_panel_shown(&self) -> bool {
        SIDE_PANELS.iter().any(|p| self.docks.is_shown(*p))
    }

    /// `view.fullscreen`: the window over the whole screen, or back.
    pub(crate) fn toggle_fullscreen(&mut self) -> Task<Message> {
        self.fullscreen = !self.fullscreen;
        let mode = if self.fullscreen {
            window::Mode::Fullscreen
        } else {
            window::Mode::Windowed
        };
        window::latest().and_then(move |id| window::set_mode(id, mode))
    }

    /// `server.check`: asks the KentOS server now (the web's `ServerStatus.check`).
    fn check_server(&mut self) -> Task<Message> {
        let server = self
            .cloud
            .client
            .as_ref()
            .map_or_else(|| self.settings.text("cloud.server"), |c| c.server().to_owned());
        match Cloud::new(&server) {
            Ok(client) => {
                self.server_checking = true;
                self.output(format!("Sunucuya soruluyor: {server}…"));
                // Asked when the task runs, not when it is made.
                Task::perform(async move { client.health().await }, |answer| {
                    Message::ServerChecked(answer.map_err(|e| e.to_string()))
                })
            }
            Err(e) => {
                self.say(
                    Level::Info,
                    format!("Sunucu yok. {e} Çizim sunucusuz çalışmaya devam ediyor."),
                );
                Task::none()
            }
        }
    }

    /// What the server answered, in the web's words.
    pub(crate) fn server_checked(&mut self, answer: Result<Health, String>) {
        self.server_checking = false;
        match answer {
            Ok(h) if h.status != "ok" => self.say(
                Level::Info,
                "Sunucu yok. Yanıt KentOS sağlık sözleşmesine uymuyor: durum “ok” değil. Çizim sunucusuz çalışmaya devam ediyor.",
            ),
            Ok(h) if h.contracts != CONTRACTS_VERSION => self.warn(format!(
                "Sunucu uyumsuz. Sunucu sözleşme sürümü {}, uygulama {CONTRACTS_VERSION} bekliyor. Uygulamayı ya da sunucuyu güncelleyin.",
                h.contracts
            )),
            Ok(h) => self.say(
                Level::Success,
                format!("Sunucu bağlı: {} {}.", h.service, h.version),
            ),
            Err(why) => self.say(
                Level::Info,
                format!("Sunucu yok. {why} Çizim sunucusuz çalışmaya devam ediyor."),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use kentos_contracts::{CONTRACTS_VERSION, Health};

    use kentos_ui::widget::docking::Side;

    use crate::app::{App, Dialog, Message, Panel};
    use crate::files_testing::{app_with_drawing, last_said};

    fn run(app: &mut App, id: &'static str) {
        let _ = app.update(Message::Run(id));
    }

    #[test]
    fn f4_closes_and_opens_both_side_panels() {
        let mut app = app_with_drawing();
        assert!(app.right_panel_shown());
        run(&mut app, "view.rightPanel");
        assert!(!app.docks.is_shown(Panel::Layers) && !app.docks.is_shown(Panel::Properties));
        assert_eq!(app.checked("view.rightPanel"), Some(false));
        run(&mut app, "view.rightPanel");
        assert!(app.docks.is_shown(Panel::Layers) && app.docks.is_shown(Panel::Properties));
        assert_eq!(app.checked("view.rightPanel"), Some(true));
    }

    #[test]
    fn full_screen_is_a_toggle_and_esc_leaves_it_when_nothing_else_takes_it() {
        let mut app = app_with_drawing();
        run(&mut app, "view.fullscreen");
        assert!(app.fullscreen);
        assert_eq!(app.checked("view.fullscreen"), Some(true));
        // Esc with nothing running and nothing selected leaves full screen.
        run(&mut app, "tool.cancel");
        assert!(!app.fullscreen);
        // With a selection, Esc clears it first.
        run(&mut app, "view.fullscreen");
        run(&mut app, "edit.selectAll");
        run(&mut app, "tool.cancel");
        assert!(app.fullscreen && app.selection.is_empty());
    }

    #[test]
    fn f4_brings_the_panels_back_as_they_were() {
        use kentos_ui::widget::docking::{Event, Target};
        let mut app = app_with_drawing();
        // Properties as a tab beside Layers, the area wider.
        let slot = app.docks.slot(Panel::Layers).expect("docked");
        let _ = app.update(Message::Dock(Event::Moved(Panel::Properties, Target::Tab(slot, 1))));
        let _ = app.update(Message::Dock(Event::Resized(Side::Right, 400.0)));
        let before = app.docks.clone();
        run(&mut app, "view.rightPanel");
        run(&mut app, "view.rightPanel");
        assert_eq!(app.docks, before);
    }

    #[test]
    fn koordinat_sistemi_opens_the_project_settings() {
        let mut app = app_with_drawing();
        run(&mut app, "crs.set");
        assert_eq!(app.dialog, Some(Dialog::Project));
    }

    #[test]
    fn the_server_check_waits_for_its_answer() {
        let mut app = app_with_drawing();
        run(&mut app, "server.check");
        assert!(!app.available("server.check"));
        let _ = app.update(Message::ServerChecked(Err("Sunucuya ulaşılamadı.".into())));
        assert!(app.available("server.check"));
    }

    #[test]
    fn the_servers_answer_is_said_in_the_webs_words() {
        let mut app = app_with_drawing();
        let health = |contracts| Health {
            status: "ok".into(),
            service: "kentosd".into(),
            version: "0.1.0".into(),
            commit: None,
            contracts,
        };
        app.server_checked(Ok(health(CONTRACTS_VERSION)));
        assert_eq!(last_said(&app), "Sunucu bağlı: kentosd 0.1.0.");
        app.server_checked(Ok(health(CONTRACTS_VERSION + 1)));
        assert!(last_said(&app).starts_with("Sunucu uyumsuz."), "{}", last_said(&app));
        app.server_checked(Err("Sunucuya ulaşılamadı.".into()));
        assert!(last_said(&app).ends_with("Çizim sunucusuz çalışmaya devam ediyor."));
    }
}

#[cfg(test)]
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use iced::futures::StreamExt as _;
    use kentos_contracts::Health;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in ["odak", "komut-ara", "koordinat-sistemi", "sunucu"] {
                let mut app = crate::files_testing::app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let task = match name {
                    // F4 and full screen: the drawing alone under the ribbon.
                    "odak" => {
                        app.tab = "view";
                        let _ = app.update(Message::Run("view.rightPanel"));
                        app.fullscreen = true;
                        Task::none()
                    }
                    "komut-ara" => app.update(Message::Run("view.commandSearch")),
                    "koordinat-sistemi" => app.update(Message::Run("crs.set")),
                    _ => {
                        app.tab = "tools";
                        app.output("Sunucuya soruluyor: http://127.0.0.1:8787…");
                        app.server_checked(Ok(Health {
                            status: "ok".into(),
                            service: "kentosd".into(),
                            version: "0.1.0".into(),
                            commit: None,
                            contracts: CONTRACTS_VERSION,
                        }));
                        app.output("Sunucuya soruluyor: http://127.0.0.1:8787…");
                        app.server_checked(Err(
                            "Sunucuya ulaşılamadı; bağlantınızı ve sunucu adresini denetleyin."
                                .into(),
                        ));
                        Task::none()
                    }
                };
                // The command list opens by a widget operation, as the runtime runs it.
                if let Some(mut stream) = iced_runtime::task::into_stream(task) {
                    while let Some(action) = iced::futures::executor::block_on(stream.next()) {
                        if let iced_runtime::Action::Widget(operation) = action {
                            snapshot.operate(app.view(), operation);
                        }
                    }
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("gorunum-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
