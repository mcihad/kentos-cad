//! Malzemeler (docs/adr/0785; `arch.materials`): the project's building materials, each with what is bound to it (wall
//! styles, walls, slabs). The chosen material's name, category, way of counting, cut hatch and cut pen are typed here;
//! before Uygula the plan of `cad.materials.edit` says what follows (how many styles, walls and slabs, and how many stay
//! on a locked layer). Yeni malzeme, Sil (refused while anything is bound) and Metinlerden al (every text material taken
//! into the table) write through the same command; Stile bağla binds a wall style's component to the chosen material.
//! Every write is one undo step “Malzeme”. Kitaplık (docs/adr/0791, `material_library.rs`) shows the material
//! libraries outside the drawing: KentOS's and the office's folders; Seçilenleri al takes a library's materials in,
//! Kitaplığa yaz… writes the project's to a `.kmalzeme` file.

use iced::widget::{Column, button, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    ArchMaterial, CommandResult, ComponentEdge, ComponentHatch, Entity, MaterialCategory, MaterialChange,
    MaterialQuantity, MaterialReport, MaterialTarget, MaterialsEdit, MaterialsEdited, MaterialsOperation,
};
use kentos_interaction::Level;
use kentos_ui::icon::icon;
use kentos_ui::widget::{Choice, Dialog as Frame, Select};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Malzemeler";
const CLOSE: &str = "Kapat";
const APPLY: &str = "Uygula";
const NEW: &str = "Yeni malzeme";
const REMOVE: &str = "Sil";
const ADOPT: &str = "Metinlerden al";
const BIND: &str = "Stile bağla";

/// What is bound to a material: its wall styles' names, its walls and slabs counted.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Use {
    pub styles: Vec<String>,
    pub walls: usize,
    pub slabs: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Window {
    pub list: Vec<(ArchMaterial, Use)>,
    pub chosen: usize,
    pub name: String,
    pub category: MaterialCategory,
    pub quantity: MaterialQuantity,
    pub pattern: String,
    pub scale: String,
    pub angle: String,
    pub hatch_color: String,
    pub pen: String,
    pub pen_color: String,
    /// The wall style components Stile bağla offers (style id, component, the words), and the one chosen.
    pub targets: Vec<(String, u32, String)>,
    pub target: Option<usize>,
    /// What the change typed would do; its refusal's words when refused.
    pub preview: Option<Result<MaterialsEdited, String>>,
    /// Kitaplık shown instead of the project's materials (docs/adr/0791).
    pub tab: Tab,
    /// The libraries, the one chosen, its materials ticked, and how one the project has is taken.
    pub entries: Vec<crate::material_library::Entry>,
    pub entry: usize,
    pub picked: Vec<bool>,
    pub conflict: Conflict,
    /// What Seçilenleri al would do; its refusal's words when refused.
    pub take_preview: Option<Result<MaterialsEdited, String>>,
}

/// The window's two views.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Project,
    Library,
}

impl std::fmt::Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Tab::Project => "Projenin malzemeleri",
            Tab::Library => "Kitaplık",
        })
    }
}

/// What a library material the project has of the same id does (`onConflict`): kept or replaced.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Conflict {
    #[default]
    Keep,
    Replace,
}

impl std::fmt::Display for Conflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Conflict::Keep => "Projeninkini koru",
            Conflict::Replace => "Kitaplığınkiyle değiştir",
        })
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Choose(usize),
    Name(String),
    Category(usize),
    Quantity(usize),
    Pattern(String),
    Scale(String),
    Angle(String),
    HatchColor(String),
    Pen(String),
    PenColor(String),
    Target(usize),
    Bind,
    New,
    Remove,
    Adopt,
    Apply,
    Close,
    Tab(Tab),
    Entry(usize),
    Pick(usize),
    PickAll(bool),
    Conflict(Conflict),
    Take,
    AddFolder,
    FolderChosen(Option<std::path::PathBuf>),
    RemoveFolder,
    Export,
    Exported(Option<std::path::PathBuf>),
}

fn msg(e: Event) -> Message {
    Message::Materials(e)
}

/// What a report line says about `r`.
pub(crate) fn report_words(r: &MaterialReport) -> String {
    let mut lead = Vec::new();
    if !r.taken.is_empty() {
        lead.push(format!("{} malzeme alınır", r.taken.len()));
    }
    if !r.replaced.is_empty() {
        lead.push(format!("{} malzeme değişir", r.replaced.len()));
    }
    if !r.kept.is_empty() {
        lead.push(format!("{} malzemede projeninki korunur", r.kept.len()));
    }
    // An import that touches nothing bound says only what it takes.
    let mut parts = if lead.is_empty() || !(r.styles.is_empty() && r.walls.is_empty() && r.slabs.is_empty()) {
        vec![format!("{} stil, {} duvar, {} döşeme izler", r.styles.len(), r.walls.len(), r.slabs.len())]
    } else {
        Vec::new()
    };
    if !r.locked.is_empty() {
        parts.push(format!("{} kilitli katmanda eski değerleriyle kalır", r.locked.len()));
    }
    if !r.left.is_empty() {
        parts.push(format!("{} metin farklı görünüşlü bileşenleri yüzünden bağlanmadı", r.left.len()));
    }
    lead.extend(parts);
    lead.join(" · ")
}

fn number(v: f64) -> String {
    if v.fract() == 0.0 { format!("{v:.0}") } else { format!("{v}") }
}

/// The window's fields filled from the material `m`.
fn fill(w: &mut Window, m: &ArchMaterial) {
    w.name = m.name.clone();
    w.category = m.category;
    w.quantity = m.quantity;
    let h = m.hatch.as_ref();
    w.pattern = h.map_or_else(String::new, |h| h.pattern.clone());
    w.scale = h.map_or_else(String::new, |h| number(h.scale));
    w.angle = h.map_or_else(String::new, |h| number(h.angle));
    w.hatch_color = h.and_then(|h| h.color.clone()).unwrap_or_default();
    let c = m.cut.as_ref();
    w.pen = c.and_then(|c| c.line_weight).map_or_else(String::new, number);
    w.pen_color = c.and_then(|c| c.color.clone()).unwrap_or_default();
    w.preview = None;
}

/// A decimal typed with a point or a comma; none for empty.
fn typed(t: &str, what: &str) -> Result<Option<f64>, String> {
    let t = t.trim();
    if t.is_empty() {
        return Ok(None);
    }
    t.replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .map(Some)
        .ok_or_else(|| format!("{what}: “{t}” okunamadı; bir sayı yazın."))
}

/// The change the window's fields make to `m`; none when they make none.
fn change_of(w: &Window, m: &ArchMaterial) -> Option<Result<MaterialChange, String>> {
    let mut c = MaterialChange::default();
    let name = w.name.trim();
    if !name.is_empty() && name != m.name {
        c.name = Some(name.to_owned());
    }
    if w.category != m.category {
        c.category = Some(w.category);
    }
    if w.quantity != m.quantity {
        c.quantity = Some(w.quantity);
    }
    let hatch = if w.pattern.trim().is_empty() {
        None
    } else {
        let scale = match typed(&w.scale, "Tarama ölçeği") {
            Ok(v) => v.unwrap_or(1.0),
            Err(e) => return Some(Err(e)),
        };
        let angle = match typed(&w.angle, "Tarama açısı") {
            Ok(v) => v.unwrap_or(0.0),
            Err(e) => return Some(Err(e)),
        };
        let color = w.hatch_color.trim();
        Some(ComponentHatch {
            pattern: w.pattern.trim().to_owned(),
            scale,
            angle,
            color: (!color.is_empty()).then(|| color.to_owned()),
        })
    };
    if hatch != m.hatch {
        c.hatch = Some(hatch);
    }
    let weight = match typed(&w.pen, "Kesit kalemi") {
        Ok(v) => v,
        Err(e) => return Some(Err(e)),
    };
    let color = w.pen_color.trim();
    let cut = (weight.is_some() || !color.is_empty()).then(|| ComponentEdge {
        color: (!color.is_empty()).then(|| color.to_owned()),
        line_weight: weight,
        line_type: m.cut.as_ref().and_then(|c| c.line_type),
    });
    if cut != m.cut {
        c.cut = Some(cut);
    }
    (!c.is_empty()).then_some(Ok(c))
}

impl App {
    /// The project's materials, each with what is bound to it.
    fn material_uses(&self) -> Vec<(ArchMaterial, Use)> {
        let Some(doc) = self.document.as_ref() else {
            return Vec::new();
        };
        let model = &doc.model;
        model
            .materials()
            .iter()
            .map(|m| {
                let mut u = Use::default();
                for s in &model.settings().wall_styles {
                    if s.components.iter().any(|c| c.material_id.as_deref() == Some(m.id.as_str())) {
                        u.styles.push(s.name.clone());
                    }
                }
                for e in model.entities() {
                    match e {
                        Entity::Wall(w) if w.wall.components.iter().any(|c| c.material_id.as_deref() == Some(m.id.as_str())) => {
                            u.walls += 1;
                        }
                        Entity::Slab(s) if s.slab.material_id.as_deref() == Some(m.id.as_str()) => u.slabs += 1,
                        _ => {}
                    }
                }
                (m.clone(), u)
            })
            .collect()
    }

    /// The wall style components Stile bağla offers.
    fn material_targets(&self) -> Vec<(String, u32, String)> {
        let Some(doc) = self.document.as_ref() else {
            return Vec::new();
        };
        doc.model
            .settings()
            .wall_styles
            .iter()
            .flat_map(|s| {
                s.components.iter().enumerate().map(move |(i, c)| {
                    let bound = c.material.as_deref().filter(|_| c.material_id.is_some());
                    let words = match bound {
                        Some(m) => format!("{} › {}. {} · {m}", s.name, i + 1, c.name),
                        None => format!("{} › {}. {}", s.name, i + 1, c.name),
                    };
                    (s.id.clone(), i as u32, words)
                })
            })
            .collect()
    }

    /// The window read again, the material `keep` (an id) chosen if it is still there.
    pub(crate) fn materials_read(&mut self, keep: Option<String>) {
        let list = self.material_uses();
        let chosen = keep
            .and_then(|id| list.iter().position(|(m, _)| m.id == id))
            .unwrap_or(0);
        let (tab, entry, conflict) = self
            .materials
            .as_ref()
            .map_or((Tab::Project, 0, Conflict::Keep), |w| (w.tab, w.entry, w.conflict));
        let mut w = Window {
            list,
            chosen,
            targets: self.material_targets(),
            tab,
            entry,
            conflict,
            ..Window::default()
        };
        if let Some((m, _)) = w.list.get(chosen).cloned() {
            fill(&mut w, &m);
        }
        self.materials = Some(w);
        self.material_library_read();
    }

    /// Malzemeler (`arch.materials`): the window.
    pub(crate) fn open_materials(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        self.materials_read(None);
        self.dialog = Some(Dialog::Materials);
    }

    pub(crate) fn materials_chosen(&self) -> Option<ArchMaterial> {
        let w = self.materials.as_ref()?;
        w.list.get(w.chosen).map(|(m, _)| m.clone())
    }

    /// The plan of the change typed: what it would do, nothing written.
    fn materials_preview(&mut self) {
        let (Some(w), Some(m)) = (self.materials.as_ref(), self.materials_chosen()) else {
            return;
        };
        let preview = match change_of(w, &m) {
            None => None,
            Some(Err(why)) => Some(Err(why)),
            Some(Ok(change)) => self.document.as_ref().map(|doc| {
                let input = MaterialsEdit {
                    operation: MaterialsOperation::Update { material: m.id.clone(), change },
                    expected_revision: None,
                };
                match kentos_native_application::materials_edit::plan_in(&doc.model, &input) {
                    CommandResult::Completed { output, .. } => Ok(output),
                    CommandResult::Failed { error }
                    | CommandResult::Conflict { error }
                    | CommandResult::NeedsInput { error } => Err(error.message),
                    _ => Err("Önizlenemedi.".into()),
                }
            }),
        };
        if let Some(w) = self.materials.as_mut() {
            w.preview = preview;
        }
    }

    /// `cad.materials.edit` over the drawing; its answer, or none with the refusal said.
    pub(crate) fn materials_command(&mut self, operation: MaterialsOperation) -> Option<MaterialsEdited> {
        let doc = self.document.as_mut()?;
        let result = kentos_native_application::materials_edit::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            MaterialsEdit { operation, expected_revision: None },
        );
        match result {
            CommandResult::Completed { output, warnings, .. } => {
                for w in warnings {
                    self.warn(w.message);
                }
                Some(output)
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.warn(error.message);
                None
            }
            _ => None,
        }
    }

    pub(crate) fn materials_event(&mut self, event: Event) -> Task<Message> {
        let edit = |app: &mut App, f: &dyn Fn(&mut Window)| {
            if let Some(w) = app.materials.as_mut() {
                f(w);
            }
            app.materials_preview();
        };
        match event {
            Event::Choose(i) => {
                if let Some(w) = self.materials.as_mut()
                    && let Some((m, _)) = w.list.get(i).cloned()
                {
                    w.chosen = i;
                    fill(w, &m);
                }
            }
            Event::Name(t) => edit(self, &|w| w.name = t.clone()),
            Event::Category(i) => edit(self, &|w| w.category = MaterialCategory::ALL.get(i).copied().unwrap_or_default()),
            Event::Quantity(i) => edit(self, &|w| w.quantity = MaterialQuantity::ALL.get(i).copied().unwrap_or_default()),
            Event::Pattern(t) => edit(self, &|w| w.pattern = t.clone()),
            Event::Scale(t) => edit(self, &|w| w.scale = t.clone()),
            Event::Angle(t) => edit(self, &|w| w.angle = t.clone()),
            Event::HatchColor(t) => edit(self, &|w| w.hatch_color = t.clone()),
            Event::Pen(t) => edit(self, &|w| w.pen = t.clone()),
            Event::PenColor(t) => edit(self, &|w| w.pen_color = t.clone()),
            Event::Target(i) => {
                if let Some(w) = self.materials.as_mut() {
                    w.target = Some(i);
                }
            }
            Event::Bind => {
                let (Some(m), Some((style, component, words))) = (
                    self.materials_chosen(),
                    self.materials.as_ref().and_then(|w| w.target.and_then(|i| w.targets.get(i).cloned())),
                ) else {
                    return Task::none();
                };
                let op = MaterialsOperation::Assign {
                    material: Some(m.id.clone()),
                    targets: vec![MaterialTarget::WallStyle { style, component }],
                };
                if let Some(done) = self.materials_command(op) {
                    self.say(Level::Success, format!("{words} → {}: {}.", m.name, report_words(&done.report)));
                }
                self.materials_read(Some(m.id));
            }
            Event::Apply => {
                let Some(m) = self.materials_chosen() else {
                    return Task::none();
                };
                let change = self.materials.as_ref().and_then(|w| change_of(w, &m));
                match change {
                    Some(Ok(change)) => {
                        if let Some(done) =
                            self.materials_command(MaterialsOperation::Update { material: m.id.clone(), change })
                        {
                            self.say(Level::Success, format!("{} değişti: {}.", m.name, report_words(&done.report)));
                        }
                    }
                    Some(Err(why)) => {
                        self.warn(why);
                        return Task::none();
                    }
                    None => {}
                }
                self.materials_read(Some(m.id));
            }
            Event::New => {
                let names: Vec<String> = self
                    .materials
                    .as_ref()
                    .map(|w| w.list.iter().map(|(m, _)| m.name.to_lowercase()).collect())
                    .unwrap_or_default();
                let mut name = "Yeni malzeme".to_owned();
                let mut n = 2;
                while names.contains(&name.to_lowercase()) {
                    name = format!("Yeni malzeme {n}");
                    n += 1;
                }
                if let Some(done) = self.materials_command(MaterialsOperation::Define {
                    name: name.clone(),
                    id: None,
                    change: MaterialChange::default(),
                }) {
                    self.say(Level::Success, format!("{name} eklendi."));
                    self.materials_read(done.material);
                }
            }
            Event::Remove => {
                let Some(m) = self.materials_chosen() else {
                    return Task::none();
                };
                if self.materials_command(MaterialsOperation::Remove { material: m.id.clone() }).is_some() {
                    self.say(Level::Success, format!("{} silindi.", m.name));
                }
                self.materials_read(None);
            }
            Event::Adopt => {
                let keep = self.materials_chosen().map(|m| m.id);
                if let Some(done) = self.materials_command(MaterialsOperation::Adopt {}) {
                    self.say(
                        Level::Success,
                        format!("Metin malzemeler alındı: {} malzeme · {}.", done.materials.len(), report_words(&done.report)),
                    );
                }
                self.materials_read(keep);
            }
            Event::Close => {
                self.materials = None;
                self.dialog = None;
            }
            Event::Tab(_)
            | Event::Entry(_)
            | Event::Pick(_)
            | Event::PickAll(_)
            | Event::Conflict(_)
            | Event::Take
            | Event::AddFolder
            | Event::FolderChosen(_)
            | Event::RemoveFolder
            | Event::Export
            | Event::Exported(_) => return self.material_library_event(event),
        }
        Task::none()
    }

    pub(crate) fn materials_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.materials else {
            return iced::widget::text("").into();
        };
        let mut list = Column::new().spacing(4);
        for (i, (m, u)) in w.list.iter().enumerate() {
            let mut facts = format!("{} · {}", m.category.label(), m.quantity.label());
            let bound = u.styles.len() + u.walls + u.slabs;
            if bound == 0 {
                facts.push_str(" · bağlı yok");
            } else {
                facts.push_str(&format!(" · {} stil, {} duvar, {} döşeme", u.styles.len(), u.walls, u.slabs));
            }
            let text = Column::new().spacing(2).push(label::body(m.name.clone())).push(label::caption(facts));
            let entry = row![icon(crate::icons::from_web(Some("materials"))).size(18.0), container(text).width(Fill)]
                .spacing(8)
                .align_y(Center);
            list = list.push(
                button(entry)
                    .on_press(msg(Event::Choose(i)))
                    .padding([6, 8])
                    .width(Fill)
                    .style(style::button::row(i == w.chosen)),
            );
        }
        if w.list.is_empty() {
            list = list.push(label::caption(
                "Projede malzeme yok. Yeni malzeme bir tane açar; Metinlerden al duvar stillerinin, duvarların ve döşemelerin metin malzemelerinden açar.",
            ));
        }
        let tabs = kentos_ui::widget::Segmented::new([Tab::Project, Tab::Library], w.tab, |t| msg(Event::Tab(t)));
        if w.tab == Tab::Library {
            return self.material_library_view(w, tabs.into());
        }
        let left = container(iced::widget::scrollable(list).height(Fill)).width(280.0);
        let right: Element<'_, Message> = match w.list.get(w.chosen) {
            None => label::caption("").into(),
            Some((m, u)) => self.material_form(w, m, u),
        };
        let body = row![left, container(right).width(Fill)].spacing(16).height(440.0);
        let chosen = !w.list.is_empty();
        let changed = w.list.get(w.chosen).is_some_and(|(m, _)| change_of(w, m).is_some());
        Frame::new(TITLE)
            .push(tabs)
            .push(body)
            .action(words::secondary(NEW, Some(msg(Event::New))))
            .action(words::secondary(REMOVE, chosen.then(|| msg(Event::Remove))))
            .action(words::secondary(ADOPT, Some(msg(Event::Adopt))))
            .action(words::secondary(CLOSE, Some(msg(Event::Close))))
            .action(words::primary(APPLY, (chosen && changed).then(|| msg(Event::Apply))))
            .width(920.0)
            .modal(msg(Event::Close))
    }

    fn material_form<'a>(&'a self, w: &'a Window, m: &'a ArchMaterial, u: &'a Use) -> Element<'a, Message> {
        let text = |placeholder: &str, value: &'a str, on: fn(String) -> Event| {
            kentos_ui::widget::text_field(placeholder, value).on_input(move |t| msg(on(t)))
        };
        let category = Select::new(
            MaterialCategory::ALL.iter().map(|c| Choice::new(c.label())),
            MaterialCategory::ALL.iter().position(|c| *c == w.category),
            |i| msg(Event::Category(i)),
        )
        .searchable(false);
        let quantity = Select::new(
            MaterialQuantity::ALL.iter().map(|q| Choice::new(q.label())),
            MaterialQuantity::ALL.iter().position(|q| *q == w.quantity),
            |i| msg(Event::Quantity(i)),
        )
        .searchable(false);
        let cells: Vec<Element<'a, Message>> = vec![
            words::field("Ad", text("Malzemenin adı", &w.name, Event::Name), Some(format!("Kimlik: {}", m.id))),
            words::field("Kategori", category, None),
            words::field("Metraj", quantity, None),
            words::field("Tarama deseni", text("Yok", &w.pattern, Event::Pattern), Some("ANSI31, ANSI37, SOLID…".into())),
            words::field("Tarama ölçeği", text("1", &w.scale, Event::Scale), None),
            words::field("Tarama açısı (°)", text("0", &w.angle, Event::Angle), None),
            words::field("Tarama rengi", text("Duvarınki", &w.hatch_color, Event::HatchColor), Some("#RRGGBB".into())),
            words::field("Kesit kalemi (mm)", text("Duvarınki", &w.pen, Event::Pen), None),
            words::field("Kalem rengi", text("Duvarınki", &w.pen_color, Event::PenColor), Some("#RRGGBB".into())),
        ];
        let mut grid = Column::new().spacing(10);
        let mut cells = cells.into_iter();
        loop {
            let a: Vec<Element<'a, Message>> = cells.by_ref().take(3).collect();
            if a.is_empty() {
                break;
            }
            let mut r = row![].spacing(12).align_y(iced::Top);
            let n = a.len();
            for c in a {
                r = r.push(container(c).width(Fill));
            }
            for _ in n..3 {
                r = r.push(container(label::caption("")).width(Fill));
            }
            grid = grid.push(r);
        }
        let styles = if u.styles.is_empty() { "—".to_owned() } else { u.styles.join(", ") };
        let mut summary = vec![words::text_line(
            Kind::Ok,
            format!("Bağlı: {} stil ({styles}), {} duvar, {} döşeme.", u.styles.len(), u.walls, u.slabs),
        )];
        match &w.preview {
            Some(Ok(done)) => summary.push(words::text_line(
                if done.report.locked.is_empty() { Kind::Ok } else { Kind::Warn },
                format!("Uygula: {}.", report_words(&done.report)),
            )),
            Some(Err(why)) => summary.push(words::text_line(Kind::Warn, why.clone())),
            None => summary.push(words::text_line(
                Kind::Ok,
                "Değer yazınca Uygula'dan önce bağlı stiller, duvarlar ve döşemeler sayılır.",
            )),
        }
        let target = Select::new(
            w.targets.iter().map(|(_, _, t)| Choice::new(t.clone())),
            w.target,
            |i| msg(Event::Target(i)),
        )
        .placeholder("Duvar stili bileşeni");
        let bind = row![
            container(target).width(Fill),
            words::secondary(BIND, w.target.map(|_| msg(Event::Bind)))
        ]
        .spacing(8)
        .align_y(Center);
        Column::new()
            .spacing(12)
            .push(words::summary(summary))
            .push(
                iced::widget::scrollable(container(grid).padding(iced::Padding { right: 16.0, ..iced::Padding::ZERO }))
                    .height(Fill),
            )
            .push(words::field("Bileşeni bu malzemeye bağla", bind, None))
            .into()
    }

    /// The window's buttons by their words (a trace's `dialog` step).
    pub(crate) fn materials_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.materials.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(APPLY) => Some(msg(Event::Apply)),
            Control::Press(NEW) => Some(msg(Event::New)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(ADOPT) => Some(msg(Event::Adopt)),
            Control::Press(BIND) => Some(msg(Event::Bind)),
            Control::Press(crate::material_library::TAKE) => Some(msg(Event::Take)),
            Control::Press(crate::material_library::EXPORT) => Some(msg(Event::Export)),
            Control::Press(crate::material_library::ADD_FOLDER) => Some(msg(Event::AddFolder)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

#[cfg(test)]
#[path = "materials_tests.rs"]
mod tests;
