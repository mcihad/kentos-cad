//! Drawing with an object template (docs/adr/0176 §3; the web's
//! `tools/objectTemplates.ts`). Choosing a template makes its layer active
//! (opened under its path, one undo step “Katman ekle”, when the drawing lacks
//! it), takes its colour and line weight into the draft and starts its tool,
//! with its method; every object the tool writes takes its symbol, attributes
//! and label (`Context::template`). A point, text or block template also sets
//! its tool's own options for the run (docs/adr/0176 §3b): Nokta's Ad and Kod
//! (a template's names go on from run to run in the drawing), Yazı's height
//! on paper, alignment and mask, Blok ekle's block, found by its name (a
//! template whose block the drawing lacks does not start, and says so). The
//! run ends with the tool: Esc, another command or another template give the
//! colour, the weight and the tool's own options back; the active layer stays.
//! Son komutu yinele starts the template again. A locked layer keeps the
//! template from starting, and says so.

use iced::Task;
use kentos_contracts::{BlockId, TextAlign};
use kentos_interaction::templates::{
    LayerAnswer, Stamp, TemplateLayer, find_layer, locked_text, open_layer,
};
use kentos_interaction::{DraftColor, Level, Name, Prompt};
use kentos_native_style::library::ItemKind;
use kentos_native_style::object_template::{self, Recipe, member_issues, template_issues};

use crate::app::{App, Message};
use crate::catalog::catalog;
use crate::template_members::RunMember;

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
    /// The tool's own options it set, as they were before it.
    tool_back: ToolBack,
    /// A group template's members: their objects are written with each
    /// object the tool writes, in its undo step (template_members.rs).
    pub(crate) members: Vec<RunMember>,
}

/// A tool's own options as a point, text or block template found them
/// (docs/adr/0176 §3b), given back at the run's end; none for what the
/// template leaves alone.
#[derive(Clone, Copy, Default)]
struct ToolBack {
    /// Nokta's Ad, when the template names its points, and its Kod.
    point_name: Option<Name>,
    point_code: Option<Name>,
    /// Yazı's height on paper, alignment and mask.
    text: Option<(f64, Option<TextAlign>, bool)>,
    /// Blok ekle's block.
    block: Option<Option<BlockId>>,
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
        // A block template's block, by its name: without it the template does not start.
        let block = match recipe
            .block
            .as_deref()
            .filter(|_| recipe.tool == "blockInsert")
        {
            Some(block_name) => {
                let found = self.document.as_ref().and_then(|doc| {
                    doc.model
                        .blocks()
                        .iter()
                        .find(|b| b.name == block_name)
                        .map(|b| b.id)
                });
                let Some(found) = found else {
                    self.warn(missing_block_text(block_name, &name));
                    return Task::none();
                };
                Some(found)
            }
            None => None,
        };
        // A group template's members, from the library: what keeps one from
        // starting is said, and nothing changes.
        let (issue, members) = {
            let lib = &self.styles.library;
            let find = |mid: &str| {
                lib.get(mid)
                    .filter(|(item, _)| item.kind() == ItemKind::Template)
                    .and_then(|(item, _)| Some((item.name(), item.template()?)))
            };
            let issue = member_issues(&template, find).into_iter().next();
            let members: Vec<(object_template::RecipeMember, String, Recipe)> = recipe
                .members
                .iter()
                .filter_map(|m| {
                    let (item, _) = lib.get(&m.template)?;
                    let own = object_template::read(item.template()?)?;
                    Some((m.clone(), item.name().to_owned(), own))
                })
                .collect();
            (issue, members)
        };
        if let Some(issue) = issue {
            self.warn(format!("“{name}” grup şablonu başlamaz: {issue}"));
            return Task::none();
        }
        let layers: Vec<TemplateLayer> = std::iter::once(&recipe)
            .chain(members.iter().map(|(_, _, own)| own))
            .map(layer_of)
            .collect();
        let Some(layer_ids) = self.template_layers(&layers, &name) else {
            return Task::none();
        };
        // Another template's run gives way, its colour and weight kept as the
        // ones to give back; its tool's own options come back now.
        let previous = self.template.take();
        let before = previous
            .as_ref()
            .map_or((self.draft.color, self.draft.line_weight), |run| run.before);
        if let Some(run) = &previous {
            self.give_tool_back(run);
        }
        self.draft.color = recipe.color.as_deref().and_then(DraftColor::new);
        self.draft.line_weight = recipe.line_weight;
        self.field = None;
        let tool_back = self.seed_tool(id, &recipe, block);
        let mut run = TemplateRun {
            id: id.to_owned(),
            name: name.clone(),
            stamp: Stamp {
                symbol: recipe.symbol.clone(),
                attrs: recipe.attrs.clone(),
                label: recipe.label.clone(),
            },
            runs: 0,
            before,
            tool_back,
            members: members
                .into_iter()
                .zip(layer_ids.into_iter().skip(1))
                .map(|((m, member_name, own), layer_id)| {
                    RunMember::new(&m, member_name, &own, layer_id)
                })
                .collect(),
        };
        if !self.session.start(&recipe.tool) {
            (self.draft.color, self.draft.line_weight) = before;
            self.give_tool_back(&run);
            return Task::none();
        }
        run.runs = self.session.runs();
        self.template = Some(run);
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

    /// Finds or opens the template's layers (its own first, then its
    /// members'), the opened ones in one step “Katman ekle”, and makes the
    /// first active: their ids, in order; none (said) when one of them, or the
    /// group it would go in, is locked or the drawing refuses it, and then
    /// nothing changes.
    fn template_layers(&mut self, layers: &[TemplateLayer], name: &str) -> Option<Vec<String>> {
        let doc = self.document.as_mut()?;
        let locked = layers.iter().find_map(|layer| {
            let tree = doc.model.layers();
            match find_layer(tree.nodes(), layer, |id| tree.is_locked(id)) {
                LayerAnswer::Locked(id) => Some(
                    tree.get(&id)
                        .map_or_else(String::new, |node| locked_text(node, name)),
                ),
                _ => None,
            }
        });
        if let Some(text) = locked {
            self.warn(text);
            return None;
        }
        let group = doc.model.begin_group(kentos_domain::labels::LAYER_ADD);
        let mut ids = Vec::new();
        // One after another: a layer opened for one is found for the next.
        for layer in layers {
            let tree = doc.model.layers();
            let answer = find_layer(tree.nodes(), layer, |id| tree.is_locked(id));
            let id = match answer {
                LayerAnswer::Found(id) => Ok(id),
                LayerAnswer::Locked(id) => Err(doc
                    .model
                    .layers()
                    .get(&id)
                    .map_or_else(String::new, |node| locked_text(node, name))),
                LayerAnswer::Open { parent, create } => {
                    open_layer(&mut doc.model, layer, parent.as_deref(), &create)
                        .map_err(|refusal| refusal.to_string())
                }
            };
            match id {
                Ok(id) => ids.push(id),
                Err(text) => {
                    doc.model.cancel_group(group);
                    self.warn(text);
                    return None;
                }
            }
        }
        doc.model.end_group(group);
        doc.model.set_active_layer(&ids[0]);
        Some(ids)
    }

    /// After every message: a template's run ends with its tool (Esc, another
    /// command, the tool done) and gives back what it set.
    pub(crate) fn follow_template(&mut self) {
        if self
            .template
            .as_ref()
            .is_some_and(|run| run.runs != self.session.runs() || !self.session.is_running())
        {
            self.release_template();
        }
    }

    /// Ends a template's run, if one is: the colour, the weight and the
    /// tool's own options it found come back.
    pub(crate) fn release_template(&mut self) {
        if let Some(run) = self.template.take() {
            (self.draft.color, self.draft.line_weight) = run.before;
            self.give_tool_back(&run);
        }
    }

    /// Sets a point, text or block template's own options in its tool's
    /// memory (docs/adr/0176 §3b); what they were, to give back. A point
    /// template's names go on from its last run; one without a first name
    /// leaves Nokta's own series alone. A text template without its text
    /// part leaves Yazı's options alone.
    fn seed_tool(&mut self, id: &str, recipe: &Recipe, block: Option<BlockId>) -> ToolBack {
        let next_name = self.template_names.get(id).and_then(|n| Name::new(n));
        let m = &mut self.memory;
        let mut back = ToolBack::default();
        match recipe.tool.as_str() {
            "point" => {
                back.point_code = Some(m.point_code);
                m.point_code = recipe
                    .point_code
                    .as_deref()
                    .and_then(Name::new)
                    .unwrap_or(Name::EMPTY);
                let first = recipe
                    .point_name
                    .as_deref()
                    .filter(|n| !n.is_empty())
                    .and_then(Name::new);
                if let Some(first) = first {
                    back.point_name = Some(m.point_name);
                    m.point_name = next_name.unwrap_or(first);
                }
            }
            "text" => {
                if let Some(height_mm) = recipe.text_height {
                    back.text = Some((m.text_height_mm, m.text_align, m.text_mask));
                    m.text_height_mm = height_mm;
                    m.text_align = recipe.text_align.as_deref().and_then(TextAlign::from_name);
                    m.text_mask = recipe.text_mask;
                }
            }
            "blockInsert" => {
                if let Some(block) = block {
                    back.block = Some(m.block_insert);
                    m.block_insert = Some(block);
                }
            }
            _ => {}
        }
        back
    }

    /// Gives a template run's tool options back; a point template's next
    /// name is kept for its next run.
    fn give_tool_back(&mut self, run: &TemplateRun) {
        let back = run.tool_back;
        if let Some(name) = back.point_name {
            self.template_names
                .insert(run.id.clone(), self.memory.point_name.as_str().to_owned());
            self.memory.point_name = name;
        }
        if let Some(code) = back.point_code {
            self.memory.point_code = code;
        }
        if let Some((height_mm, align, mask)) = back.text {
            self.memory.text_height_mm = height_mm;
            self.memory.text_align = align;
            self.memory.text_mask = mask;
        }
        if let Some(block) = back.block {
            self.memory.block_insert = block;
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

/// The layer a recipe's objects go on.
fn layer_of(recipe: &Recipe) -> TemplateLayer {
    TemplateLayer {
        path: recipe.layer_path.clone(),
        name: recipe.layer_name.clone(),
        color: recipe.layer_color.clone(),
        line_type: recipe.layer_line_type,
        line_weight: recipe.layer_line_weight,
    }
}

/// Why a block template does not start: the drawing lacks its block.
pub(crate) fn missing_block_text(block: &str, template: &str) -> String {
    format!(
        "Çizimde “{block}” bloğu yok: “{template}” şablonu bu bloğu yerleştirir. Bloğu Blok oluştur ile tanımlayın ya da bir DXF'ten alın."
    )
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
