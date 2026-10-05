//! Drawing with an object template (docs/adr/0176 §3; the web's
//! `tools/objectTemplates.ts`). Choosing a template makes its layer active
//! (opened under its path, one undo step “Katman ekle”, when the drawing lacks
//! it), takes its colour and line weight into the draft and starts its tool,
//! with its method; every object the tool writes takes its symbol, attributes
//! and label (`Context::template`). The run ends with the tool: Esc, another
//! command or another template give the colour and weight back; the active
//! layer stays. Son komutu yinele starts the template again. A locked layer
//! keeps the template from starting, and says so.

use iced::Task;
use kentos_interaction::templates::{
    LayerAnswer, Stamp, TemplateLayer, find_layer, locked_text, open_layer,
};
use kentos_interaction::{DraftColor, Level, Prompt};
use kentos_native_style::library::ItemKind;
use kentos_native_style::object_template::{self, Recipe, template_issues};

use crate::app::{App, Message};
use crate::catalog::catalog;

/// A template being drawn with: what the tools' context carries, and what
/// its end gives back.
pub(crate) struct TemplateRun {
    /// The template's library id (the Şablonlar panel marks its rows) and
    /// name (the prompt says it before the tool's).
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) stamp: Stamp,
    /// The session's tool run it is (`Session::runs`): another one ends it.
    runs: u64,
    /// The draft's colour and line weight before it.
    before: (Option<DraftColor>, Option<f64>),
}

impl App {
    /// `template.draw`: draws with the library's template `id` (the Stil
    /// yöneticisi's Şablonla çiz, a trace's `template` step).
    pub(crate) fn draw_template(&mut self, id: &str) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok. Önce bir çizim açın (Ctrl+O).");
            return Task::none();
        }
        let item = self
            .styles
            .library
            .get(id)
            .map(|(item, _)| item)
            .filter(|item| item.kind() == ItemKind::Template);
        let Some(item) = item else {
            self.warn(format!(
                "“{id}” kimlikli şablon kitaplıkta yok: silinmiş olabilir. Şablonu Stil yöneticisinde seçin."
            ));
            return Task::none();
        };
        let name = item.name().to_owned();
        let template = item.template().cloned().unwrap_or_default();
        let Some(recipe) = object_template::read(&template) else {
            let issue = template_issues(&template, &format!("“{name}” şablonu"))
                .into_iter()
                .next()
                .unwrap_or_default();
            self.warn(format!("{issue}; şablonu Stil yöneticisinde düzeltin."));
            return Task::none();
        };
        if !self.template_layer(&recipe, &name) {
            return Task::none();
        }
        // Another template's run gives way, its colour and weight kept as the ones to give back.
        let before = self
            .template
            .take()
            .map_or((self.draft.color, self.draft.line_weight), |run| run.before);
        self.draft.color = recipe.color.as_deref().and_then(DraftColor::new);
        self.draft.line_weight = recipe.line_weight;
        self.field = None;
        if !self.session.start(&recipe.tool) {
            (self.draft.color, self.draft.line_weight) = before;
            return Task::none();
        }
        self.template = Some(TemplateRun {
            id: id.to_owned(),
            name: name.clone(),
            stamp: Stamp {
                symbol: recipe.symbol.clone(),
                attrs: recipe.attrs.clone(),
                label: recipe.label.clone(),
            },
            runs: self.session.runs(),
            before,
        });
        self.last_template = Some(id.to_owned());
        // The Şablonlar panel's first group (templates_panel.rs).
        self.recent_templates.retain(|r| r != id);
        self.recent_templates.insert(0, id.to_owned());
        self.recent_templates
            .truncate(crate::templates_panel::RECENT);
        let title = catalog()
            .get(&format!("tool.{}", recipe.tool))
            .map_or(recipe.tool.as_str(), |command| command.title);
        self.say(Level::Command, format!("{name} · {title}"));
        self.with_tool(|s, cx| s.activate(cx));
        // Its method, as the ribbon's menu starts one (Daire: 2 nokta).
        if let Some(method) = &recipe.method
            && self.session.is_running()
            && self.with_tool(|s, cx| s.input(method, cx)) != Some(true)
        {
            self.warn(crate::ribbon_plan::texts::cannot_start(title, method));
        }
        self.follow_template();
        Task::none()
    }

    /// Makes the template's layer active, opening it when the drawing lacks
    /// it; false (said) when it, or the group it would go in, is locked or the
    /// drawing refuses it.
    fn template_layer(&mut self, recipe: &Recipe, name: &str) -> bool {
        let Some(doc) = self.document.as_mut() else {
            return false;
        };
        let layer = TemplateLayer {
            path: recipe.layer_path.clone(),
            name: recipe.layer_name.clone(),
            color: recipe.layer_color.clone(),
            line_type: recipe.layer_line_type,
            line_weight: recipe.layer_line_weight,
        };
        let layers = doc.model.layers();
        let refusal = match find_layer(layers.nodes(), &layer, |id| layers.is_locked(id)) {
            LayerAnswer::Found(id) => {
                doc.model.set_active_layer(&id);
                None
            }
            LayerAnswer::Locked(id) => Some(
                layers
                    .get(&id)
                    .map_or_else(String::new, |node| locked_text(node, name)),
            ),
            LayerAnswer::Open { parent, create } => {
                open_layer(&mut doc.model, &layer, parent.as_deref(), &create)
                    .err()
                    .map(|refusal| refusal.to_string())
            }
        };
        match refusal {
            Some(text) => {
                self.warn(text);
                false
            }
            None => true,
        }
    }

    /// After every message: a template's run ends with its tool (Esc, another
    /// command, the tool done) and gives the colour and weight back.
    pub(crate) fn follow_template(&mut self) {
        if self
            .template
            .as_ref()
            .is_some_and(|run| run.runs != self.session.runs() || !self.session.is_running())
        {
            self.release_template();
        }
    }

    /// Ends a template's run, if one is: the colour and weight it found come back.
    pub(crate) fn release_template(&mut self) {
        if let Some(run) = self.template.take() {
            (self.draft.color, self.draft.line_weight) = run.before;
        }
    }

    /// The running command's prompt, its template's name before the tool's
    /// (`Parsel sınırı · Kapalı alan: …`); a tool run over it (the point
    /// calculator) speaks for itself.
    pub(crate) fn prompt(&self) -> Prompt {
        let mut prompt = self.session.prompt();
        if !self.session.nested() {
            prompt.template = self.template.as_ref().map(|run| run.name.clone());
        }
        prompt
    }

    /// The last template's name, for the drawing menu's Yinele.
    pub(crate) fn last_template_name(&self) -> Option<&str> {
        let id = self.last_template.as_deref()?;
        self.styles.library.get(id).map(|(item, _)| item.name())
    }
}

#[cfg(test)]
mod tests {
    //! Drawing with a template as the user does it: the run's colour and
    //! weight, what ends it, what it gives back, the locked layer, the prompt
    //! and Son komutu yinele. The shared trace is
    //! fixtures/interaction/v1/template-draw.json.

    use kentos_contracts::ProjectStyles;
    use serde_json::json;

    use crate::app::{App, Message};
    use crate::files_testing::{app_with_drawing, last_said};
    use kentos_interaction::DraftColor;

    /// The sample drawing with two project templates: Parsel sınırı (Kapalı
    /// alan on Kadastro › Parsel, red, 0.5 mm) and Bina (Dikdörtgen on a new
    /// Yapı layer, brown).
    fn app() -> App {
        let mut app = app_with_drawing();
        let template = |id: &str, name: &str, template: serde_json::Value| json!({ "kind": "template", "id": id, "name": name, "path": ["Şablonlar"], "template": template });
        let styles = ProjectStyles {
            items: vec![
                template(
                    "p-parsel",
                    "Parsel sınırı",
                    json!({ "tool": "polygon", "layer": { "path": ["Kadastro"], "name": "Parsel" }, "color": "#E5484D", "lineWeight": 0.5, "label": "P" }),
                ),
                template(
                    "p-bina",
                    "Bina",
                    json!({ "tool": "rectangle", "layer": { "path": [], "name": "Yapı" }, "color": "#7A5C3E" }),
                ),
            ],
            categories: vec![json!({ "path": ["Şablonlar"] })],
        };
        app.document
            .as_mut()
            .expect("a drawing")
            .model
            .set_styles(styles);
        // The library takes the drawing's part after a message (follow_document).
        let _ = app.update(Message::Swallowed);
        app
    }

    fn draft(app: &App) -> (Option<String>, Option<f64>) {
        (app.draft.color_text(), app.draft.line_weight)
    }

    #[test]
    fn a_template_to_another_then_esc_gives_back_the_colour_found_first() {
        let mut app = app();
        app.draft.color = DraftColor::new("#4F8EF7");
        app.draft.line_weight = Some(0.25);
        let _ = app.update(Message::DrawTemplate("p-parsel".into()));
        assert_eq!(draft(&app), (Some("#E5484D".into()), Some(0.5)));
        assert_eq!(app.session.tool_id(), "polygon");
        assert_eq!(
            app.prompt().text(),
            "Parsel sınırı · Kapalı alan: ilk noktayı belirtin"
        );
        // Another template while the first runs: its colour, no weight (the layer's).
        let _ = app.update(Message::DrawTemplate("p-bina".into()));
        assert_eq!(draft(&app), (Some("#7A5C3E".into()), None));
        assert_eq!(app.session.tool_id(), "rectangle");
        // Esc: the colour and weight from before the first.
        let _ = app.update(Message::Run("tool.cancel"));
        assert_eq!(app.session.tool_id(), "select");
        assert_eq!(draft(&app), (Some("#4F8EF7".into()), Some(0.25)));
        // The active layer stays the template's, opened for it.
        let doc = &app.document.as_ref().expect("a drawing").model;
        let active = doc.layers().get(doc.layers().active()).expect("a layer");
        assert_eq!(active.name, "Yapı");
    }

    #[test]
    fn another_command_ends_the_run_and_repeat_starts_the_template_again() {
        let mut app = app();
        let _ = app.update(Message::DrawTemplate("p-parsel".into()));
        let _ = app.update(Message::Run("tool.line"));
        assert_eq!(app.session.tool_id(), "line");
        assert_eq!(draft(&app), (None, None));
        assert_eq!(app.prompt().text(), "Çizgi: ilk noktayı belirtin");
        // Çizgi is now the last command; the template comes back when started again.
        let _ = app.update(Message::Run("tool.cancel"));
        let _ = app.update(Message::Run("tool.repeat"));
        assert_eq!(app.session.tool_id(), "line");
        let _ = app.update(Message::Run("tool.cancel"));
        let _ = app.update(Message::DrawTemplate("p-parsel".into()));
        let _ = app.update(Message::Run("tool.cancel"));
        let _ = app.update(Message::Run("tool.repeat"));
        assert_eq!(app.session.tool_id(), "polygon");
        assert_eq!(draft(&app), (Some("#E5484D".into()), Some(0.5)));
        assert_eq!(app.last_template_name(), Some("Parsel sınırı"));
    }

    #[test]
    fn a_missing_template_or_a_locked_layer_starts_nothing_and_says_why() {
        let mut app = app();
        let _ = app.update(Message::DrawTemplate("yok".into()));
        assert_eq!(app.session.tool_id(), "select");
        assert_eq!(
            last_said(&app),
            "“yok” kimlikli şablon kitaplıkta yok: silinmiş olabilir. Şablonu Stil yöneticisinde seçin."
        );
        // Kadastro › Bina is locked in the sample drawing: a template for it does not start.
        let doc = app.document.as_mut().expect("a drawing");
        let mut styles = doc.model.styles().clone();
        styles.items.push(json!({ "kind": "template", "id": "p-kilitli", "name": "Kilitli bina", "path": [], "template": { "tool": "line", "layer": { "path": ["Kadastro"], "name": "Bina" } } }));
        doc.model.set_styles(styles);
        let _ = app.update(Message::Swallowed);
        let _ = app.update(Message::DrawTemplate("p-kilitli".into()));
        assert_eq!(app.session.tool_id(), "select");
        assert_eq!(
            last_said(&app),
            "“Bina” katmanı kilitli; “Kilitli bina” şablonu bu katmana çizer. Kilidi Katmanlar panelinden açın."
        );
        assert!(app.template.is_none());
    }
}
