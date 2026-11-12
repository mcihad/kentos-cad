//! Ofis paletleri (docs/adr/1014): the bound standards' published tool palettes in Şablonlar, above the templates.
//! Each tool is a catalog command with its whole input; pressing it runs that command (the same registry Python and
//! MCP call), as one undo step, and the answer is said. A tool the command does not take on this drawing is shown
//! dimmed with the command's reason. The palettes are read from the packages when the panel opens and after a tool
//! runs (`cad.standards.palettes`, which checks each tool on a copy of the drawing): not on every edit.
//!
//! The user's own (docs/adr/1016), in `arac-paleti.json` beside the settings, never in the drawing: favourites of
//! office tools by key and digest (one whose tool changed in a new revision says so and does not run until chosen
//! again; one no longer published says that) and the user's own tools, each the command and input of a call the
//! drawing's journal holds (“Son komutu araç yap”). The search of Şablonlar filters them all by title.

use iced::widget::{button, column, container, row};
use iced::{Element, Fill, Task};
use kentos_commands::{CallOp, PaletteState, StandardsPalettes};
use kentos_interaction::Level;
use kentos_ui::{label, style};

use crate::app::{App, Message};

/// The palettes as last read, and the user's own choices.
#[derive(Debug, Default)]
pub(crate) struct OfficePalettes {
    pub(crate) palettes: Vec<PaletteState>,
    pub(crate) personal: PersonalTools,
}

const PERSONAL_FILE: &str = "arac-paleti.json";
const PERSONAL_LIMIT: usize = 100;

/// A favourite office tool: its key and the digest of the recipe chosen.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct Favorite {
    pub(crate) key: String,
    pub(crate) digest: String,
}

/// The user's palette: favourites and own tools (docs/adr/1016).
#[derive(Debug, Default)]
pub(crate) struct PersonalTools {
    path: Option<std::path::PathBuf>,
    pub(crate) favorites: Vec<Favorite>,
    pub(crate) tools: Vec<kentos_commands::PaletteTool>,
}

impl PersonalTools {
    /// Reads `arac-paleti.json` in `folder`; a file that does not read is kept aside, said.
    pub(crate) fn open(folder: &std::path::Path) -> (Self, Option<String>) {
        let path = folder.join(PERSONAL_FILE);
        let mut out = Self { path: Some(path.clone()), ..Self::default() };
        match std::fs::read_to_string(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (out, None),
            Err(e) => (out, Some(format!("Araç paleti okunamadı ({}): {e}", path.display()))),
            Ok(text) => {
                let value: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
                if value["format"] != "kentos.tool-palette" || value["version"] != 1 {
                    return (out, Some(format!("“{}” bir KentOS araç paleti değil; yok sayıldı.", path.display())));
                }
                out.favorites = serde_json::from_value(value["favorites"].clone()).unwrap_or_default();
                out.tools = serde_json::from_value(value["tools"].clone()).unwrap_or_default();
                out.favorites.truncate(PERSONAL_LIMIT);
                out.tools.truncate(PERSONAL_LIMIT);
                (out, None)
            }
        }
    }

    fn save(&self) -> std::io::Result<()> {
        let Some(path) = &self.path else { return Ok(()) };
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder)?;
        }
        let partial = path.with_extension("json.yaziliyor");
        let text = serde_json::to_vec_pretty(&serde_json::json!({
            "format": "kentos.tool-palette", "version": 1, "favorites": self.favorites, "tools": self.tools,
        }))?;
        std::fs::write(&partial, text)?;
        std::fs::rename(partial, path)
    }

    /// A favourite chosen again or taken off.
    pub(crate) fn toggle(&mut self, key: &str, digest: &str) -> std::io::Result<()> {
        match self.favorites.iter().position(|f| f.key == key) {
            Some(i) if self.favorites[i].digest == digest => {
                self.favorites.remove(i);
            }
            // A changed tool chosen again: its new recipe is the one kept.
            Some(i) => self.favorites[i].digest = digest.to_owned(),
            None if self.favorites.len() >= PERSONAL_LIMIT => {
                return Err(std::io::Error::other("En fazla 100 araç favoriye eklenebilir"));
            }
            None => self.favorites.push(Favorite { key: key.to_owned(), digest: digest.to_owned() }),
        }
        self.save()
    }
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

    /// The user's palette, read where the settings are.
    pub(crate) fn open_personal_tools(&mut self, folder: &std::path::Path) -> Option<String> {
        let (personal, problem) = PersonalTools::open(folder);
        self.office_palettes.personal = personal;
        problem
    }

    /// A tool pressed: an office tool by its key (`standart:…`) or one of the user's own (`kendi:<i>`).
    pub(crate) fn run_palette_tool(&mut self, key: &str) -> Task<Message> {
        let tool = match key.strip_prefix("kendi:").and_then(|i| i.parse::<usize>().ok()) {
            Some(i) => self.office_palettes.personal.tools.get(i).cloned(),
            None => self.office_palettes.palettes.iter().flat_map(|p| &p.tools).find(|t| t.key == key).map(|t| t.tool.clone()),
        };
        let Some(tool) = tool else {
            return Task::none();
        };
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let answer = kentos_native_application::registry::run(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            &tool.command,
            CallOp::Execute,
            tool.input.clone(),
        );
        match answer {
            Ok(v) if v["status"] == "completed" => {
                self.say(Level::Success, format!("“{}” aracı çalıştı ({}). Geri almak için Geri al.", tool.title, tool.command));
            }
            Ok(v) => {
                let words = v["error"]["message"].as_str().unwrap_or("komut reddetti").to_owned();
                self.say(Level::Error, format!("“{}” aracı çalışmadı: {words}", tool.title));
            }
            Err(unrun) => self.say(Level::Error, format!("“{}” aracı çalışmadı: {}", tool.title, unrun.message)),
        }
        self.read_office_palettes();
        Task::none()
    }

    /// ★ on an office tool: favourite by its key and its recipe's digest.
    pub(crate) fn toggle_tool_favorite(&mut self, key: &str) {
        let Some(digest) = self.office_palettes.palettes.iter().flat_map(|p| &p.tools).find(|t| t.key == key).map(|t| t.digest.clone()) else {
            return;
        };
        if let Err(e) = self.office_palettes.personal.toggle(key, &digest) {
            self.warn(format!("Araç favorisi kaydedilemedi: {e}"));
        }
    }

    /// Son komutu araç yap: the newest call of a command in the drawing's journal, with its input, as one of the
    /// user's own tools.
    pub(crate) fn tool_from_last_call(&mut self) {
        let Some(doc) = self.document.as_ref() else { return };
        let Some(row) = doc.model.journal().filter(|r| r.command.is_some() && r.input.is_some()).last().cloned() else {
            self.warn("Çizimin günlüğünde araç yapılacak bir komut yok: önce bir komut çalıştırın.");
            return;
        };
        let command = row.command.clone().unwrap_or_default();
        // The command's catalog title (“Çizgi oluştur”), not its undo step's word.
        let title = crate::history::text::TITLES
            .iter()
            .find(|(id, _)| *id == command)
            .map(|(_, t)| (*t).to_owned())
            .or_else(|| row.label.clone())
            .unwrap_or_else(|| command.clone());
        let personal = &mut self.office_palettes.personal;
        if personal.tools.len() >= PERSONAL_LIMIT {
            self.warn("En fazla 100 kendi aracı kaydedilebilir.");
            return;
        }
        personal.tools.push(kentos_commands::PaletteTool {
            id: format!("kendi-{}", personal.tools.len() + 1),
            title: title.clone(),
            description: None,
            category: None,
            command: command.clone(),
            command_version: row.version.unwrap_or(1),
            input: row.input.clone().unwrap_or_default(),
        });
        match personal.save() {
            Ok(()) => self.say(Level::Success, format!("“{title}” ({command}) kendi araçlarına eklendi.")),
            Err(e) => self.warn(format!("Araç paleti kaydedilemedi: {e}")),
        }
    }

    /// The section above the templates: Favoriler, Kendi araçlarım, then the office palettes, each tool filtered by
    /// the search; Son komutu araç yap at the end. Nothing when there is nothing to show and no drawing.
    pub(crate) fn office_palettes_view(&self) -> Option<Element<'_, Message>> {
        use crate::templates_panel::Event;
        let query = crate::app::fold(self.templates_panel.query());
        let shows = |title: &str| query.is_empty() || crate::app::fold(title).contains(&query);
        let personal = &self.office_palettes.personal;
        let tool_row = |words: String, press: Option<Message>, star: Option<(bool, Message)>| -> Element<'_, Message> {
            let mut r = row![button(row![label::body(words)].width(Fill)).width(Fill).padding([3, 8]).style(style::button::ghost).on_press_maybe(press)];
            if let Some((on, toggle)) = star {
                r = r.push(button(label::body(if on { "★" } else { "☆" })).padding([3, 6]).style(style::button::ghost).on_press(toggle));
            }
            r.into()
        };
        let mut body = column![].spacing(4);
        let mut any = false;
        // Favoriler: the office tools the user chose, each checked against its digest.
        let mut favorites = column![].spacing(2);
        for f in &personal.favorites {
            let found = self.office_palettes.palettes.iter().flat_map(|p| &p.tools).find(|t| t.key == f.key);
            let (words, press) = match found {
                Some(t) if t.digest == f.digest => (t.tool.title.clone(), t.ready.then(|| Message::TemplatesPanel(Event::Tool(t.key.clone())))),
                Some(t) => (format!("{} — ofis revizyonunda değişti; yeniden seçmek için ★", t.tool.title), None),
                None => (format!("{} — artık yayımlanmıyor", f.key), None),
            };
            if shows(&words) {
                any = true;
                favorites = favorites.push(tool_row(words, press, Some((true, Message::TemplatesPanel(Event::ToolFavorite(f.key.clone()))))));
            }
        }
        if !personal.favorites.is_empty() {
            body = body.push(label::caption("Favori araçlar")).push(favorites);
        }
        let mut own = column![].spacing(2);
        for (i, t) in personal.tools.iter().enumerate() {
            if shows(&t.title) {
                any = true;
                own = own.push(tool_row(format!("{} · {}", t.title, t.command), Some(Message::TemplatesPanel(Event::Tool(format!("kendi:{i}")))), None));
            }
        }
        if !personal.tools.is_empty() {
            body = body.push(label::caption("Kendi araçlarım")).push(own);
        }
        if !self.office_palettes.palettes.is_empty() {
            body = body.push(label::caption("Ofis paletleri"));
        }
        for p in &self.office_palettes.palettes {
            let mut group = column![].spacing(2);
            let mut shown = 0;
            for t in &p.tools {
                if !shows(&t.tool.title) {
                    continue;
                }
                shown += 1;
                let words = match &t.problem {
                    None => t.tool.title.clone(),
                    Some(problem) => format!("{} — {}", t.tool.title, problem.message),
                };
                let press = t.ready.then(|| Message::TemplatesPanel(Event::Tool(t.key.clone())));
                let on = personal.favorites.iter().any(|f| f.key == t.key && f.digest == t.digest);
                group = group.push(tool_row(words, press, Some((on, Message::TemplatesPanel(Event::ToolFavorite(t.key.clone()))))));
            }
            if shown > 0 {
                any = true;
                body = body.push(label::strong(format!("{} · {} {}", p.name, p.standard, p.revision))).push(group);
            }
        }
        if self.document.is_none() {
            return None;
        }
        body = body.push(
            button(label::body("Son komutu araç yap"))
                .padding([3, 8])
                .style(style::button::secondary)
                .on_press(Message::TemplatesPanel(Event::ToolFromLast)),
        );
        let _ = any;
        Some(container(body).padding([6, 8]).width(Fill).into())
    }
}

/// The picture of docs/adr/1014: Şablonlar with Ofis A's palette, one tool ready and one dimmed with its reason.
/// `KENTOS_SHOTS_ONLY=ofis-paleti cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture`.
#[cfg(test)]
pub(crate) fn scenes() -> Vec<crate::tools_screens::Scene> {
    vec![
        ("ofis-paleti", |app| {
            tests::bound(app);
            let _ = app.show_templates_panel();
        }),
        // docs/adr/1016: the line tool a favourite, run once and made the user's own from the journal.
        ("ofis-paleti-favori", |app| {
            use crate::templates_panel::Event;
            let _ = app.open_personal_tools(&crate::files_testing::scratch("arac-paleti-resim"));
            tests::bound(app);
            let _ = app.show_templates_panel();
            let key = app.office_palettes.palettes[0].tools[0].key.clone();
            let _ = app.update(Message::TemplatesPanel(Event::ToolFavorite(key.clone())));
            let _ = app.update(Message::TemplatesPanel(Event::Tool(key)));
            let _ = app.update(Message::TemplatesPanel(Event::ToolFromLast));
        }),
    ]
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    /// The test drawing with Ofis A bound: a palette of a line on the drawing's first layer and one on a layer it
    /// lacks.
    pub(crate) fn bound(app: &mut App) {
        bound_at(app, 487010.0);
    }

    /// [`bound`] with the line's end at `x` (a new revision of the tool when it differs).
    pub(crate) fn bound_at(app: &mut App, x: f64) {
        crate::files_testing::make_cad(app);
        let doc = app.document.as_mut().expect("a drawing");
        let layer = doc.model.layers().active().to_owned();
        let package = json!({"format": "kentos.drawing-standard", "version": 2, "id": "ofis-a", "revision": "2026.1",
            "standard": {"name": "Ofis A", "layers": []},
            "resources": {"toolPalettes": [{"id": "mimari", "name": "Mimari", "tools": [
                {"id": "duvar", "title": "Duvar çizgisi", "command": "cad.line.create", "commandVersion": 1,
                 "input": {"layerId": layer, "a": {"x": 487000.0, "y": 4420000.0}, "b": {"x": x, "y": 4420000.0}}},
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

    #[test]
    fn a_favourite_keeps_its_recipe_and_an_own_tool_is_the_journals_last_call() {
        use crate::templates_panel::Event;
        let mut app = crate::files_testing::app_with_drawing();
        let dir = crate::files_testing::scratch("arac-paleti");
        assert!(app.open_personal_tools(&dir).is_none());
        bound(&mut app);
        let _ = app.show_templates_panel();
        let key = app.office_palettes.palettes[0].tools[0].key.clone();
        let _ = app.update(Message::TemplatesPanel(Event::ToolFavorite(key.clone())));
        assert_eq!(app.office_palettes.personal.favorites.len(), 1);
        let (again, problem) = PersonalTools::open(&dir);
        assert!(problem.is_none());
        assert_eq!(again.favorites, app.office_palettes.personal.favorites, "kept in arac-paleti.json");
        // A new revision changes the tool: the same key, another digest; the favourite does not follow it unasked.
        bound_at(&mut app, 487020.0);
        let _ = app.show_templates_panel();
        let now = app.office_palettes.palettes[0].tools[0].digest.clone();
        assert_ne!(app.office_palettes.personal.favorites[0].digest, now);
        // Chosen again, it keeps the new recipe.
        let _ = app.update(Message::TemplatesPanel(Event::ToolFavorite(key.clone())));
        assert_eq!(app.office_palettes.personal.favorites[0].digest, now);
        // The office tool run, then made one of the user's own from the journal; the own tool runs the same call.
        let _ = app.update(Message::TemplatesPanel(Event::Tool(key)));
        let _ = app.update(Message::TemplatesPanel(Event::ToolFromLast));
        let own = &app.office_palettes.personal.tools;
        assert_eq!(own.len(), 1);
        assert_eq!(own[0].command, "cad.line.create");
        assert_eq!(own[0].title, "Çizgi oluştur");
        let before = app.document.as_ref().unwrap().entity_count();
        let _ = app.update(Message::TemplatesPanel(Event::Tool("kendi:0".into())));
        assert_eq!(app.document.as_ref().unwrap().entity_count(), before + 1);
        assert_eq!(PersonalTools::open(&dir).0.tools.len(), 1, "kept in arac-paleti.json");
    }
}
