//! Commands of the web's Görünüm and Araçlar menus that arrange the window
//! or ask the server (app/commands.ts): the side panels (F4), full screen,
//! the command search (Alt+Q) and the server check. Koordinat sistemi… is
//! Proje ayarları on its page (project/).

use iced::Task;
use iced::window;
use kentos_cloud::Cloud;
use kentos_commands::{CONTRACTS_VERSION, Health};
use kentos_interaction::Level;
use kentos_ui::widget::docking::Side;

use crate::app::{App, Message, Panel};

/// The web command ids this module runs.
pub const COMMANDS: [&str; 6] = [
    "view.rightPanel",
    "view.assistant",
    "view.properties",
    "view.fullscreen",
    "view.commandSearch",
    "server.check",
];

/// The side panels F4 hides and shows together (the web's right panel): the program's own of them
/// (module_surfaces.rs). Kaynaklar shares Katmanlar' slot and goes with them too.
const SIDE_PANELS: [Panel; Panel::ALL.len()] = Panel::ALL;

impl App {
    pub(crate) fn view_command(&mut self, id: &'static str) -> Task<Message> {
        match id {
            "view.rightPanel" => self.toggle_right_panel(),
            // Asistan (docs/adr/8056 §4): its tab in front, the side panels shown; the composer takes the keyboard.
            "view.assistant" => return self.show_assistant(),
            "view.properties" => self.docks.show(Panel::Properties, Side::Right),
            "view.fullscreen" => return self.toggle_fullscreen(),
            // Komut ara in the tab row takes the keyboard (ribbon_search.rs).
            "view.commandSearch" => return self.search_focus(),
            "server.check" => return self.check_server(),
            _ => {}
        }
        Task::none()
    }

    /// `view.rightPanel` (F4): the layers and properties panels go away
    /// together and come back as they were: their sides, tabs, sizes and
    /// floating windows (the web hides its right panel whole).
    pub(crate) fn toggle_right_panel(&mut self) {
        // Katman paneli (UR-019): open with another tab in front (Bloklar, İşlemler…), it brings the layer list forward
        // where it is docked rather than closing; with the list in front it closes as before.
        let layers = self.shows_panel(Panel::Layers);
        if layers && self.right_panel_shown() && self.layers_hidden_by_another() {
            if !self.docks.contains(Panel::Layers) {
                self.docks.show(Panel::Layers, Side::Right);
            }
            self.docks
                .update(kentos_ui::widget::docking::Event::Selected(Panel::Layers));
            return;
        }
        if self.right_panel_shown() {
            self.hidden_docks = Some(self.docks.clone());
            for panel in SIDE_PANELS {
                self.docks.close(panel);
            }
        } else {
            match self.hidden_docks.take() {
                // As it was, the layer list in front (UR-019).
                Some(docks) => {
                    self.docks = docks;
                    if layers && self.layers_hidden_by_another() {
                        self.docks
                            .update(kentos_ui::widget::docking::Event::Selected(Panel::Layers));
                    }
                }
                // Folded to their titles: opened where they are; Katmanlar in front of its slot, as at start.
                None => {
                    for panel in SIDE_PANELS {
                        if self.shows_panel(panel) {
                            self.docks.show(panel, Side::Right);
                        }
                    }
                    self.docks
                        .update(kentos_ui::widget::docking::Event::Selected(Panel::Layers));
                }
            }
        }
    }

    /// Whether the layer list's stack has another panel than the pair (Katmanlar, Öznitelikler) in front: Bloklar,
    /// İşlemler… (UR-019).
    fn layers_hidden_by_another(&self) -> bool {
        use kentos_ui::widget::docking::Slot;
        match self.docks.slot(Panel::Layers) {
            Some(Slot::Docked(side, i)) => self
                .docks
                .stacks(side)
                .get(i)
                .and_then(|s| s.active())
                .is_some_and(|front| !matches!(front, Panel::Layers | Panel::Properties)),
            _ => false,
        }
    }

    /// Whether either side panel is on screen (the command's check mark).
    pub(crate) fn right_panel_shown(&self) -> bool {
        SIDE_PANELS
            .iter()
            .any(|p| self.shows_panel(*p) && self.docks.is_shown(*p))
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

    /// Asks the server once at start, as the web does when it is idle after
    /// start: the server cell says the answer, the log says nothing.
    pub(crate) fn check_server_quietly(&mut self) -> Task<Message> {
        self.server_quiet = true;
        self.check_server()
    }

    /// The server answered a cloud request (a sign-in, the connection's
    /// return): the server cell asks it again quietly unless it already says
    /// it is there (the web asks again when the network returns).
    pub(crate) fn server_answers_again(&mut self) -> Task<Message> {
        if self.server_checking || matches!(self.server_health, Some(Ok(_))) {
            return Task::none();
        }
        self.check_server_quietly()
    }

    /// `server.check`: asks the KentOS server now (the web's `ServerStatus.check`).
    fn check_server(&mut self) -> Task<Message> {
        let server = self.cloud.client.as_ref().map_or_else(
            || self.settings.text("cloud.server"),
            |c| c.server().to_owned(),
        );
        match Cloud::new(&server) {
            Ok(client) => {
                // The last answer stays while the new one is on its way (the cell keeps saying it).
                self.server_checking = true;
                if !self.server_quiet {
                    self.output(format!("Sunucuya soruluyor: {server}…"));
                }
                // Asked when the task runs, not when it is made.
                Task::perform(async move { client.health().await }, |answer| {
                    Message::ServerChecked(answer.map_err(|e| e.to_string()))
                })
            }
            Err(e) => {
                let quiet = std::mem::take(&mut self.server_quiet);
                self.server_health = Some(Err(e.to_string()));
                if !quiet {
                    self.say(
                        Level::Info,
                        format!("Sunucu yok. {e} Çizim sunucusuz çalışmaya devam ediyor."),
                    );
                }
                Task::none()
            }
        }
    }

    /// What the server answered, in the web's words.
    pub(crate) fn server_checked(&mut self, answer: Result<Health, String>) {
        self.server_checking = false;
        self.server_health = Some(answer.clone());
        // The check at start says its answer in the server cell only.
        if std::mem::take(&mut self.server_quiet) {
            return;
        }
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

impl App {
    /// The server as KentOS CAD hakkında names it (the web's `serverText`):
    /// its service, version and commit from the last check, or why there is
    /// none.
    pub(crate) fn server_text(&self) -> String {
        if self.server_checking {
            return "soruluyor…".to_owned();
        }
        match &self.server_health {
            Some(Ok(h)) => {
                let commit = h.commit.as_deref().map_or(String::new(), |c| {
                    format!(" ({})", c.chars().take(8).collect::<String>())
                });
                let name = format!("{} {}{commit}", h.service, h.version);
                if h.status == "ok" && h.contracts == CONTRACTS_VERSION {
                    name
                } else {
                    format!("{name}, uyumsuz")
                }
            }
            _ => "bağlı değil".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use kentos_commands::{CONTRACTS_VERSION, Health};

    use kentos_ui::widget::docking::Side;

    use crate::app::{App, Dialog, Message, Panel};
    use crate::files_testing::{app_with_drawing, last_said};

    fn run(app: &mut App, id: &'static str) {
        let _ = app.update(Message::Run(id));
    }

    /// The ribbon's ? is the web's Yardım menu, from the inventory (docs/adr/0064).
    #[test]
    fn the_ribbons_help_menu_is_the_webs() {
        assert_eq!(
            crate::catalog::catalog().menu("help"),
            [
                vec!["view.commandSearch", "help.shortcuts"],
                vec!["help.about"]
            ]
        );
    }

    /// UR-019: Katman paneli with Bloklar in front of the layers' stack brings the layer list forward instead of
    /// closing; with the list in front it closes, and opened again the list is in front.
    #[test]
    fn the_layer_panel_button_brings_the_layer_list_forward() {
        let mut app = app_with_drawing();
        if !app.docks.contains(Panel::Blocks) {
            app.docks.show(Panel::Blocks, Side::Right);
        }
        app.docks.update(kentos_ui::widget::docking::Event::Selected(Panel::Blocks));
        assert!(app.docks.is_shown(Panel::Blocks) && !app.docks.is_shown(Panel::Layers));
        run(&mut app, "view.rightPanel");
        assert!(app.docks.is_shown(Panel::Layers), "the list in front");
        assert!(app.right_panel_shown(), "not closed");
        run(&mut app, "view.rightPanel");
        assert!(!app.right_panel_shown(), "with the list in front it closes");
        run(&mut app, "view.rightPanel");
        assert!(app.docks.is_shown(Panel::Layers), "opened again with the list in front");
    }

    /// UR-019's pictures (`.run/gui-evidence/kullanim-UR-019/`): Bloklar in front, then Katman paneli pressed.
    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn layer_panel_screens() {
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kullanim-UR-019");
        std::fs::create_dir_all(&out).expect("a folder");
        for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
            let mut app = app_with_drawing();
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            if !app.docks.contains(Panel::Blocks) {
                app.docks.show(Panel::Blocks, Side::Right);
            }
            app.docks.update(kentos_ui::widget::docking::Event::Selected(Panel::Blocks));
            let mut snapshot = kentos_ui::snapshot::Snapshot::new(iced::Size::new(1440.0, 900.0)).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            for step in ["once", "sonra"] {
                if step == "sonra" {
                    run(&mut app, "view.rightPanel");
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("katman-paneli-{step}-{name}.png"));
                snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
                println!("{}", file.display());
            }
        }
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
        let _ = app.update(Message::Dock(Event::Moved(
            Panel::Properties,
            Target::Tab(slot, 1),
        )));
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
            project_changes_versions: Vec::new(),
        };
        assert_eq!(app.server_text(), "bağlı değil", "not asked yet");
        app.server_checked(Ok(health(CONTRACTS_VERSION)));
        assert_eq!(last_said(&app), "Sunucu bağlı: kentosd 0.1.0.");
        // KentOS CAD hakkında names it as the web's `serverText`.
        assert_eq!(app.server_text(), "kentosd 0.1.0");
        app.server_checked(Ok(health(CONTRACTS_VERSION + 1)));
        assert!(
            last_said(&app).starts_with("Sunucu uyumsuz."),
            "{}",
            last_said(&app)
        );
        assert_eq!(app.server_text(), "kentosd 0.1.0, uyumsuz");
        app.server_checked(Err("Sunucuya ulaşılamadı.".into()));
        assert!(last_said(&app).ends_with("Çizim sunucusuz çalışmaya devam ediyor."));
        assert_eq!(app.server_text(), "bağlı değil");
    }
}

#[cfg(test)]
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use iced::futures::StreamExt as _;
    use kentos_commands::Health;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in [
                "odak",
                "komut-ara",
                "koordinat-sistemi",
                "sunucu",
                "yardim",
                "hakkinda",
            ] {
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
                    // KentOS CAD hakkında, the server having answered.
                    "hakkinda" => {
                        app.server_checked(Ok(Health {
                            status: "ok".into(),
                            service: "kentosd".into(),
                            version: "0.1.0".into(),
                            commit: Some("0123456789abcdef".into()),
                            contracts: CONTRACTS_VERSION,
                            project_changes_versions: Vec::new(),
                        }));
                        app.update(Message::Run("help.about"))
                    }
                    // An unsaved change (the accent dot before the name), and Yardım open.
                    "yardim" => {
                        let _ = app.update(Message::Properties(crate::properties::Event::Color(
                            vec![kentos_domain::Slot(1)],
                            Some("#E5484D".into()),
                        )));
                        snapshot.settle(&mut app, App::view, &mut update);
                        snapshot.input(
                            &mut app,
                            App::view,
                            &mut update,
                            kentos_ui::snapshot::Input::Click(iced::Point::new(
                                width - HELP_X,
                                19.0,
                            )),
                        );
                        Task::none()
                    }
                    // Görünüm › Uygulama's Sunucuyu denetle (a CBS project's ribbon).
                    _ => {
                        app.tab = "view";
                        app.output("Sunucuya soruluyor: http://127.0.0.1:8787…");
                        app.server_checked(Ok(Health {
                            status: "ok".into(),
                            service: "kentosd".into(),
                            version: "0.1.0".into(),
                            commit: None,
                            contracts: CONTRACTS_VERSION,
                            project_changes_versions: Vec::new(),
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

/// The ribbon's ? from the window's right edge (read off the pictures).
#[cfg(test)]
const HELP_X: f32 = 45.0;
