//! İş grafiği (docs/adr/1940; `road.project.graph`): a road or map project's chain from its sources to its sheets, read
//! only, opened from the project tree's panel. The summary counts the drawings not up to date and the sheets that
//! cannot be delivered, and says the project's system, epoch and heights; the table lists every drawing with its
//! sources (a source changed since it was drawn, or gone, said by name), its objects written and there now, its state
//! and the sheets whose maps show it (Güncel olmayanlar: only the drawings not up to date). Tarifi kaydet writes the
//! graph as JSON: every drawing's recipe (command, version, input), its sources' stamps then and now, the system.
//! Tariften üret (docs/adr/3240; `road.project.produce`) draws a saved recipe's drawings in this project, one undo
//! step: what it drew and what it left (a drawing the project has, one that does not draw here) are said above the
//! table, which is read again. Since docs/adr/3640 the recipe picked is shown first: its plan (what would be drawn,
//! left, and where each of the recipe's layers goes: found by its name, added with the recipe's look) and a choice per
//! recipe layer of this project's layer to draw on instead; Üret draws it, Vazgeç drops it.

use crate::road;
use std::collections::BTreeMap;
use std::path::PathBuf;

use iced::widget::container;
use iced::{Element, Task};
use kentos_commands::{
    CommandResult, RoadEdgeKind as E, RoadGraphKind as G, RoadLayerMatch, RoadNodeState as S, RoadProducedLayer, RoadProjectGraph,
    RoadProjectProduce, RoadProjectProduced,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, road_project_graph, road_project_produce};
use kentos_ui::widget::grow::Grows;
use kentos_ui::widget::{Dialog as Frame, Form, Segmented};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, num, sheet};
use crate::style::fields;
use crate::traces::Control;

pub const TITLE: &str = "İş grafiği";
/// The project tree panel's button that opens it.
pub const OPEN: &str = "İş grafiği…";
const CLOSE: &str = "Kapat";
const SAVE: &str = "Tarifi kaydet…";
const PRODUCE: &str = "Tariften üret…";
/// The picked recipe drawn, or dropped (docs/adr/3640).
const CONFIRM: &str = "Üret";
const CANCEL: &str = "Vazgeç";
/// Docs/adr/5240: the found layer's choice.
const ADD_FIELDS: &str = "tarifin eksik alanlarını ekle";
/// A recipe layer left to its name: found by it, or added with the recipe's look.
const AUTO: &str = "";

/// Which drawings the table shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Show {
    #[default]
    All,
    Outdated,
}

impl std::fmt::Display for Show {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Show::All => "Tümü",
            Show::Outdated => "Güncel olmayanlar",
        })
    }
}

#[derive(Debug, Default)]
pub struct State {
    pub show: Show,
    pub answer: Option<Result<RoadProjectGraph, String>>,
    /// The last Tariften üret's answer (docs/adr/3240).
    pub produced: Option<RoadProjectProduced>,
    /// What each layer of its recipe takes when it is added, beyond its style (docs/adr/4440, 4860), by its id there.
    pub looks: BTreeMap<String, Takes>,
    /// The recipe picked, before it is drawn (docs/adr/3640).
    pub recipe: Option<Recipe>,
}

/// A recipe picked for Tariften üret (docs/adr/3640): its file; each layer its drawings go on (its id and name in the
/// recipe) with the layer of this project chosen for it (empty: by its name, or added); the plan of it as chosen and
/// the plan's warnings.
#[derive(Debug)]
pub struct Recipe {
    pub path: PathBuf,
    pub layers: Vec<(String, String, String)>,
    pub plan: Result<RoadProjectProduced, String>,
    pub warnings: Vec<String>,
    /// What each of its layers takes when it is added, beyond its style (docs/adr/4440, 4860), by its id in the recipe.
    pub looks: BTreeMap<String, Takes>,
    /// Docs/adr/5240: a layer found here takes the recipe's fields it lacks.
    pub add_fields: bool,
}

/// What a recipe layer takes when it is added, beyond its style: its transparency, blending mode, type and Yazdır said
/// as Tariften üret says them (docs/adr/4440); its fields, whether with subtypes, and the names of the value lists they
/// link (docs/adr/4860).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Takes {
    pub look: Option<String>,
    pub fields: usize,
    pub subtypes: bool,
    pub lists: Vec<String>,
    /// Docs/adr/5240: its value combinations, record rules, indexes and forms, said.
    pub more: Option<String>,
}

impl Recipe {
    /// The layers chosen, as `road.project.produce`'s `layers`.
    fn given(&self) -> std::collections::BTreeMap<String, String> {
        self.layers.iter().filter(|(_, _, to)| !to.is_empty()).map(|(from, _, to)| (from.clone(), to.clone())).collect()
    }
}

/// The layers a recipe file's drawings go on, each once in the recipe's order: its id and name (the recipe's definition;
/// a recipe from before: the id). None when the file is not read (the plan says why).
pub fn recipe_layers(path: &std::path::Path) -> Vec<(String, String)> {
    let Some(graph) = std::fs::read(path).ok().and_then(|b| serde_json::from_slice::<RoadProjectGraph>(&b).ok()) else {
        return Vec::new();
    };
    let mut out: Vec<(String, String)> = Vec::new();
    for r in graph.nodes.iter().filter_map(|n| n.recipe.as_ref()) {
        let Some(id) = r.input.get("layerId").and_then(serde_json::Value::as_str) else { continue };
        if !out.iter().any(|(x, _)| x == id) {
            out.push((id.to_owned(), r.layer.as_ref().map_or_else(|| id.to_owned(), |l| l.name.clone())));
        }
    }
    out
}

/// What each layer of a recipe file takes when it is added, beyond its style (docs/adr/4440, 4860): its transparency,
/// blending mode, type and Yazdır, said as Tariften üret says them, and its fields; by its id in the recipe, only those
/// that take any.
pub fn recipe_looks(path: &std::path::Path) -> BTreeMap<String, Takes> {
    let Some(graph) = std::fs::read(path).ok().and_then(|b| serde_json::from_slice::<RoadProjectGraph>(&b).ok()) else {
        return BTreeMap::new();
    };
    graph
        .nodes
        .iter()
        .filter_map(|n| n.recipe.as_ref()?.layer.as_ref())
        .filter_map(|l| {
            let lists = l
                .fields
                .iter()
                .filter_map(|f| f.domain.as_ref())
                .filter_map(|id| l.domains.iter().find(|d| d.id == *id))
                .fold(Vec::new(), |mut names: Vec<String>, d| {
                    if !names.contains(&d.name) {
                        names.push(d.name.clone());
                    }
                    names
                });
            let takes = Takes {
                look: road_project_produce::look_said(l),
                fields: l.fields.len(),
                subtypes: l.subtypes.is_some(),
                lists,
                more: more_words(l),
            };
            (takes != Takes::default()).then(|| (l.id.clone(), takes))
        })
        .collect()
}

/// What a recipe layer keeps beyond its fields (docs/adr/5240): “2 değer bileşimi, 1 kayıt kuralı ve 1 dizini de
/// tarifteki gibi. “Sınır denetimi” formu taslak olarak.”; none when it keeps none.
pub fn more_words(l: &kentos_commands::RoadRecipeLayer) -> Option<String> {
    let mut parts = Vec::new();
    for (n, word) in [(l.contingents.len(), "değer bileşimi"), (l.rules.len(), "kayıt kuralı"), (l.indexes.len(), "dizin")] {
        if n > 0 {
            parts.push(format!("{n} {word}"));
        }
    }
    let join = |items: &[String]| match items.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} ve {last}", rest.join(", ")),
        Some((last, _)) => last.clone(),
        None => String::new(),
    };
    let mut out = Vec::new();
    if !parts.is_empty() {
        out.push(format!("{} de tarifteki gibi.", join(&parts)));
    }
    if !l.forms.is_empty() {
        let names: Vec<String> = l.forms.iter().map(|f| format!("“{}”", f.name)).collect();
        out.push(format!("{} {} taslak olarak.", join(&names), if names.len() > 1 { "formları" } else { "formu" }));
    }
    (!out.is_empty()).then(|| out.join(" "))
}

/// What a layer added takes of its recipe layer's fields (docs/adr/4860), in the plan or (`done`) after Üret: their
/// number and its subtypes, then each value list they link: added with it, added under another name beside this
/// project's list of its name holding other values, or this project's list of the same values linked.
fn fields_words(l: &RoadProducedLayer, done: bool, t: &Takes) -> Option<String> {
    if t.fields == 0 {
        return None;
    }
    let verb = if done { "eklendi" } else { "eklenecek" };
    let mut out = format!("Alanları ({}){} tarifteki gibi.", t.fields, if t.subtypes { " ve alt türleri" } else { "" });
    if let Some(more) = &t.more {
        out.push(' ');
        out.push_str(more);
    }
    for name in &t.lists {
        let renamed = l.domains_added.iter().find(|a| *a != name && a.starts_with(&format!("{name}_")));
        if l.domains_added.contains(name) {
            out.push_str(&format!(" “{name}” değer listesi de {verb}."));
        } else if let Some(here) = renamed {
            out.push_str(&format!(
                " Bu projenin “{name}” listesi başka değerli, değişmez; tarifinki “{here}” adıyla {verb}."
            ));
        } else {
            let link = if done { "bağlandı" } else { "bağlanacak" };
            out.push_str(&format!(" Alanları bu projenin aynı değerli “{name}” listesine {link}."));
        }
    }
    Some(out)
}

/// What adding a recipe layer brings (docs/adr/3640, 4040), in the plan or (`done`) after Üret: the recipe's style, the
/// groups it goes in, and the groups and the line type added with it; what it takes beyond its style (`takes`,
/// docs/adr/4440, 4860).
fn added_words(l: &RoadProducedLayer, done: bool, takes: Option<&Takes>) -> String {
    let verb = if done { "eklendi" } else { "eklenecek" };
    let into = if l.group.is_empty() { String::new() } else { format!(" “{}” grubuna", l.group.join(" › ")) };
    let mut out = if done {
        format!("“{}” katmanı tarifteki stiliyle{into} {verb}.", l.name)
    } else {
        format!("“{}” katmanı bu projede yok; tarifteki stiliyle{into} {verb}.", l.name)
    };
    let mut also = Vec::new();
    match l.groups_added.as_slice() {
        [] => {}
        [one] => also.push(format!("“{one}” grubu")),
        many => also.push(format!("{} grupları", many.iter().map(|g| format!("“{g}”")).collect::<Vec<_>>().join(", "))),
    }
    if let Some(t) = &l.line_type_added {
        also.push(format!("“{t}” çizgi türü"));
    }
    if !also.is_empty() {
        out.push_str(&format!(" {} de {verb}.", also.join(" ve ")));
    }
    if let Some(look) = takes.and_then(|t| t.look.as_ref()) {
        out.push(' ');
        out.push_str(look);
    }
    if let Some(fields) = takes.and_then(|t| fields_words(l, done, t)) {
        out.push(' ');
        out.push_str(&fields);
    }
    out
}

/// What the picked recipe would do (docs/adr/3640): the drawings it would draw and leave, where each recipe layer goes.
pub fn plan_lines(r: &Recipe) -> Vec<(Kind, String)> {
    let mut lines = Vec::new();
    let out = match &r.plan {
        Ok(out) => out,
        Err(why) => return vec![(Kind::Error, why.clone())],
    };
    if out.produced.is_empty() {
        lines.push((Kind::Info, "Bu tariften üretilecek yeni çizim yok.".to_owned()));
    } else {
        let list: Vec<String> = out.produced.iter().map(|p| format!("{} ({})", p.from, p.source.name)).collect();
        lines.push((Kind::Info, format!("Üretilecek: {} çizim: {}.", out.produced.len(), list.join(", "))));
    }
    for l in &out.layers {
        lines.push(match l.how {
            RoadLayerMatch::Name => (Kind::Ok, format!("“{}” katmanı adıyla bulundu: {}.", l.name, l.to_name)),
            RoadLayerMatch::Created => (Kind::Info, added_words(l, false, r.looks.get(&l.from))),
            RoadLayerMatch::Given => (Kind::Ok, format!("“{}” katmanının yerine seçilen: {}.", l.name, l.to_name)),
            RoadLayerMatch::Id => (Kind::Info, format!("“{}” katmanına kimliğiyle (eski tarif).", l.to_name)),
        });
    }
    for l in &out.left {
        let kind = if l.why == "exists" { Kind::Info } else { Kind::Warn };
        lines.push((kind, format!("Tarifin {} çizimi bırakılacak: {}", l.drawing, l.message)));
    }
    for w in &r.warnings {
        lines.push((Kind::Warn, w.clone()));
    }
    lines
}

#[derive(Clone, Debug)]
pub enum Event {
    Open,
    Show(Show),
    Save,
    Saved(Option<PathBuf>),
    Produce,
    Picked(Option<PathBuf>),
    /// A recipe layer's layer here chosen (docs/adr/3640): its place in the recipe's layers, the layer's id (empty: by
    /// its name, or added).
    Layer(usize, String),
    /// Docs/adr/5240: a layer found here takes the recipe's fields it lacks.
    AddFields(bool),
    Confirm,
    Cancel,
    Close,
}

fn msg(e: Event) -> Message {
    Message::Road(road::Event::ProjectGraph(e))
}

/// Whether a drawing's state is one the print check stops: stale, missing, edited, gone.
fn outdated(s: S) -> bool {
    matches!(s, S::Stale | S::Missing | S::Edited | S::Gone)
}

/// What Tariften üret did (docs/adr/3240): the drawings drawn (each kept here, from its main source by name) and those
/// left with why; a layer added with what it took beyond its style (`looks`, docs/adr/4440).
pub fn produced_lines(out: &RoadProjectProduced, looks: &BTreeMap<String, Takes>) -> Vec<(Kind, String)> {
    let mut lines = Vec::new();
    if out.produced.is_empty() {
        lines.push((Kind::Info, "Tariften üretilecek yeni çizim yok.".to_owned()));
    } else {
        let list: Vec<String> = out.produced.iter().map(|p| format!("{} ({})", p.drawing, p.source.name)).collect();
        let objects: usize = out.produced.iter().map(|p| p.uids.len()).sum();
        lines.push((
            Kind::Ok,
            format!("Tariften üretildi: {} çizim, {objects} nesne: {}.", out.produced.len(), list.join(", ")),
        ));
    }
    // Where the recipe's layers went (docs/adr/3640); a recipe from before, by the layer's id, is no news.
    for l in &out.layers {
        match l.how {
            RoadLayerMatch::Created => lines.push((Kind::Info, added_words(l, true, looks.get(&l.from)))),
            RoadLayerMatch::Name => {
                lines.push((Kind::Ok, format!("“{}” katmanı adıyla bulundu: {}.{}", l.name, l.to_name, fields_added(l))))
            }
            RoadLayerMatch::Given => {
                lines.push((Kind::Ok, format!("“{}” katmanının yerine seçilen: {}.{}", l.name, l.to_name, fields_added(l))))
            }
            RoadLayerMatch::Id => {}
        }
    }
    for l in &out.left {
        lines.push((Kind::Info, format!("Tarifin {} çizimi bırakıldı: {}", l.drawing, l.message)));
    }
    lines
}

/// The recipe's fields a found layer took (docs/adr/5240), said after it.
fn fields_added(l: &RoadProducedLayer) -> String {
    if l.fields_added.is_empty() {
        return String::new();
    }
    let names: Vec<String> = l.fields_added.iter().map(|n| format!("“{n}”")).collect();
    format!(" Tarifin eksik alanları eklendi: {}.", names.join(", "))
}

/// A drawing's table row: its name, its sources (each changed or gone source said), its objects, state and sheets.
pub fn drawing_rows(g: &RoadProjectGraph, only_outdated: bool) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    for (k, n) in g.nodes.iter().enumerate().filter(|(_, n)| n.kind == G::Drawing) {
        if only_outdated && !outdated(n.state) {
            continue;
        }
        let into = |kind: E| g.edges.iter().filter(move |e| e.kind == kind && e.to as usize == k);
        let sources: Vec<String> = into(E::Source)
            .map(|e| {
                let name = g.nodes.get(e.from as usize).map_or("?", |s| s.name.as_str());
                match (&e.kept, &e.now) {
                    (_, None) => format!("{name} (yok)"),
                    (Some(then), Some(now)) if then != now => format!("{name} (değişti)"),
                    _ => name.to_owned(),
                }
            })
            .collect();
        let sheets: Vec<String> = g
            .edges
            .iter()
            .filter(|e| e.kind == E::Shows && e.from as usize == k)
            .map(|e| {
                let sheet = g.nodes.get(e.to as usize).map_or("?", |s| s.name.as_str());
                format!("{sheet} › {}", e.map.as_deref().unwrap_or(""))
            })
            .collect();
        // The version that draws it again, and (docs/adr/2840) the one that drew it when the record says another; the
        // revision its write made, as its record keeps it (an older file's record: none).
        let (command, objects, revision) = n.recipe.as_ref().map_or((String::new(), String::new(), String::new()), |r| {
            let short = r.command.rsplit('.').next().unwrap_or(&r.command).to_owned();
            let made = match r.made {
                Some(m) if m != r.version => format!(" (v{m} çizdi)"),
                _ => String::new(),
            };
            (
                format!("{short} v{}{made}", r.version),
                format!("{}/{}", r.objects, r.written),
                r.revision.clone().unwrap_or_else(|| "—".to_owned()),
            )
        });
        rows.push(vec![
            n.name.clone(),
            command,
            sources.join(", "),
            objects,
            revision,
            crate::road_project_panel::state_name(n.state).to_owned(),
            if sheets.is_empty() { "—".to_owned() } else { sheets.join("; ") },
        ]);
    }
    rows
}

impl App {
    fn project_graph_read(&mut self) {
        let deep = self.road.road_project.deep;
        let Some(doc) = self.document.as_ref() else { return };
        self.road.project_graph.answer = Some(road_project_graph::preview(&doc.model, deep));
    }

    /// İş grafiği from the project tree's panel: read at the panel's depth.
    pub(crate) fn open_project_graph(&mut self) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return Task::none();
        }
        self.road.project_graph = State::default();
        self.project_graph_read();
        self.dialog = Some(Dialog::Road(road::RoadDialogKind::ProjectGraph));
        Task::none()
    }

    pub(crate) fn project_graph_save(&mut self, path: PathBuf) {
        let Some(Ok(g)) = &self.road.project_graph.answer else { return };
        let text = serde_json::to_string_pretty(g).unwrap_or_default();
        match std::fs::write(&path, text) {
            Ok(()) => self.say(
                Level::Success,
                format!("{TITLE}: {} çizimin tarifi {} dosyasına yazıldı.", g.drawings, path.display()),
            ),
            Err(e) => self.warn(format!("{TITLE}: {} yazılamadı: {e}. Başka bir klasör seçin.", path.display())),
        }
    }

    /// The picked recipe planned as chosen (docs/adr/3640): nothing written.
    fn project_graph_plan(&mut self) {
        let Some(doc) = self.document.as_mut() else { return };
        let Some(r) = self.road.project_graph.recipe.as_mut() else { return };
        let input = RoadProjectProduce { path: r.path.display().to_string(), drawings: None, layers: r.given(), add_fields: r.add_fields, expected_revision: None };
        (r.plan, r.warnings) = match road_project_produce::plan(&ExecutionContext::new(&mut doc.model), &input) {
            CommandResult::Completed { output, warnings } => {
                // The added layers are said by the plan's own line.
                // Docs/adr/5240: the found layer's fields said as what Üret will do.
                let said = warnings
                    .into_iter()
                    .filter(|w| w.code != "recipe_layer_created")
                    .map(|w| match w.code.as_str() {
                        "recipe_layer_fields_added" => w.message.replace("eklendi", "eklenecek").replace("değişmedi", "değişmeyecek"),
                        _ => w.message,
                    })
                    .collect();
                (Ok(output), said)
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } => (Err(error.message), Vec::new()),
            other => (Err(format!("Tariften üret planlanamadı ({other:?}).")), Vec::new()),
        };
    }

    /// A recipe picked (docs/adr/3640): shown with its plan before anything is drawn.
    pub(crate) fn project_graph_preview(&mut self, path: PathBuf) {
        let layers = recipe_layers(&path).into_iter().map(|(id, name)| (id, name, AUTO.to_owned())).collect();
        let looks = recipe_looks(&path);
        self.road.project_graph.recipe =
            Some(Recipe { path, layers, plan: Err(String::new()), warnings: Vec::new(), looks, add_fields: false });
        self.project_graph_plan();
    }

    /// Tariften üret (docs/adr/3240): the recipe's drawings drawn in this project with the layers chosen (docs/adr/3640),
    /// one undo step; the graph read again.
    pub(crate) fn project_graph_produce(&mut self) {
        let Some(r) = self.road.project_graph.recipe.take() else { return };
        let Some(doc) = self.document.as_mut() else { return };
        let path = r.path.clone();
        let input = RoadProjectProduce { path: path.display().to_string(), drawings: None, layers: r.given(), add_fields: r.add_fields, expected_revision: None };
        let result = road_project_produce::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            let left = if out.left.is_empty() { String::new() } else { format!("; {} çizim bırakıldı", out.left.len()) };
            self.say(
                if out.produced.is_empty() { Level::Info } else { Level::Success },
                format!("{TITLE}: {} çizim tariften üretildi{left} ({}).", out.produced.len(), path.display()),
            );
            self.road.project_graph.produced = Some(out);
            self.road.project_graph.looks = r.looks;
            self.project_graph_read();
        }
    }

    pub(crate) fn project_graph_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Open => return self.open_project_graph(),
            Event::Show(v) => self.road.project_graph.show = v,
            Event::Save => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title(TITLE)
                            .set_file_name("is-grafigi.json")
                            .add_filter("JSON", &["json"])
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::Saved(p)),
                );
            }
            Event::Saved(Some(path)) => self.project_graph_save(path),
            Event::Saved(None) => {}
            Event::Produce => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title(PRODUCE.trim_end_matches('…'))
                            .add_filter("JSON", &["json"])
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::Picked(p)),
                );
            }
            Event::Picked(Some(path)) => self.project_graph_preview(path),
            Event::Picked(None) => {}
            Event::Layer(k, to) => {
                if let Some((_, _, chosen)) = self.road.project_graph.recipe.as_mut().and_then(|r| r.layers.get_mut(k)) {
                    *chosen = to;
                }
                self.project_graph_plan();
            }
            Event::AddFields(on) => {
                if let Some(r) = self.road.project_graph.recipe.as_mut() {
                    r.add_fields = on;
                }
                self.project_graph_plan();
            }
            Event::Confirm => self.project_graph_produce(),
            Event::Cancel => self.road.project_graph.recipe = None,
            Event::Close => {
                self.road.project_graph = State::default();
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn project_graph_view(&self) -> Element<'_, Message> {
        let s = &self.road.project_graph;
        let mut lines = Vec::new();
        let mut rows = Vec::new();
        match &s.answer {
            Some(Ok(g)) => {
                for (kind, said) in s.produced.iter().flat_map(|out| produced_lines(out, &s.looks)) {
                    lines.push(words::text_line(kind, said));
                }
                let fine = g.outdated == 0 && g.blocked == 0;
                lines.push(words::text_line(
                    if fine { Kind::Ok } else { Kind::Warn },
                    format!(
                        "{} çizim: {} güncel değil. {} pafta bir çizimi gösteriyor: {} teslim edilemez.",
                        g.drawings, g.outdated, g.sheets, g.blocked
                    ),
                ));
                for n in g.nodes.iter().filter(|n| n.kind == G::Sheet && n.state == S::Stale) {
                    lines.push(words::text_line(Kind::Error, format!("{}: {}", n.name, n.why.join(" "))));
                }
                let system = match (&g.system, &g.epsg) {
                    (Some(name), Some(code)) => format!("{name} ({code})"),
                    _ => "yerel (koordinat sistemi yok)".to_owned(),
                };
                let epoch = g.epoch.map_or("söylenmedi".to_owned(), |e| format!("{e}"));
                let heights = g.heights.clone().unwrap_or_else(|| "söylenmedi".to_owned());
                lines.push(words::text_line(Kind::Info, format!("Sistem {system}; epok {epoch}; kotlar {heights}.")));
                lines.push(words::text_line(
                    Kind::Info,
                    "Zincir: katman → yüzey → çizim → pafta. Güncel olmayan çizimi gösteren pafta baskı öncesi denetimde durur; çizimi Proje ağacından yeniden üretin. Çizime bir şey yazılmaz.",
                ));
                rows = drawing_rows(g, s.show == Show::Outdated);
            }
            Some(Err(why)) => lines.push(words::text_line(Kind::Error, why.clone())),
            None => {}
        }
        let table = sheet(
            vec![
                col("Çizim", 180.0),
                col("Komut", 100.0),
                col("Kaynaklar", 200.0),
                num("Nesne", 56.0),
                num("Revizyon", 64.0),
                col("Durum", 100.0),
                col("Paftalar", 190.0),
            ],
            rows,
            "Bu süzgeçte çizim yok.",
            300.0,
        );
        let mut body = Grows::new().spacing(12.0).width(iced::Fill);
        // The recipe picked (docs/adr/3640): its layers' choices and its plan, drawn only on Üret.
        if let Some(r) = &s.recipe {
            let here: Vec<(String, String)> = std::iter::once((AUTO.to_owned(), "Adıyla bul, yoksa ekle".to_owned()))
                .chain(
                    self.document
                        .iter()
                        .flat_map(|d| d.model.layers().leaves().into_iter().map(|n| (n.id.clone(), n.name.clone())).collect::<Vec<_>>()),
                )
                .collect();
            let file = r.path.file_name().map_or_else(|| r.path.display().to_string(), |n| n.to_string_lossy().into_owned());
            let mut form = Form::new().label_width(170.0).section("Tariften üret").field("Tarif", kentos_ui::label::body(file));
            for (k, (_, name, to)) in r.layers.iter().enumerate() {
                form = form.field(
                    format!("“{name}” katmanı"),
                    container(fields::select_owned(here.clone(), to, move |v| msg(Event::Layer(k, v)))).width(260),
                );
            }
            if !r.layers.is_empty() {
                form = form.help("Tarifin katmanı bu projede adıyla bulunur; yoksa tarifteki stili, grupları, saydamlığı, türü, Yazdır'ı, alanları, değer bileşimleri, kayıt kuralları, dizinleri ve formlarıyla eklenir. Başka bir katmana çizmek için seçin.");
            }
            // Docs/adr/5240: a layer found here may take the recipe's fields it lacks.
            if r.looks.values().any(|t| t.fields > 0) {
                form = form
                    .field(
                        "Bulunan katman",
                        words::check(r.add_fields, ADD_FIELDS, Some(msg(Event::AddFields(!r.add_fields)))),
                    )
                    .help("Seçilirse bu projede bulunan katmana tarifin onda olmayan alanları, kendi alanlarının sonuna eklenir; kendi alanları, kuralları ve formları değişmez.");
            }
            // What it would do, read before Üret.
            let ready = matches!(&r.plan, Ok(out) if !out.produced.is_empty());
            form = form
                .wide(words::summary(plan_lines(r).into_iter().map(|(kind, said)| words::text_line(kind, said)).collect()))
                .row(
                    iced::widget::row![
                        words::primary(CONFIRM, ready.then(|| msg(Event::Confirm))),
                        words::secondary(CANCEL, Some(msg(Event::Cancel)))
                    ]
                    .spacing(8),
                );
            body = body.push(form);
        }
        let body = body
            .push(words::summary(lines))
            .push(Segmented::new([Show::All, Show::Outdated], s.show, |v| msg(Event::Show(v))))
            .push_fit(table, kentos_ui::theme::typography::scaled(120.0));
        let ready = matches!(&s.answer, Some(Ok(_)));
        Frame::new(TITLE)
            .scroll(body)
            .aside(words::secondary(SAVE, ready.then(|| msg(Event::Save))))
            .aside(words::secondary(PRODUCE, ready.then(|| msg(Event::Produce))))
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(1000.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn project_graph_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(PRODUCE) => Some(msg(Event::Produce)),
            Control::Press(CONFIRM) => Some(msg(Event::Confirm)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
