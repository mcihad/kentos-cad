//! Katman stili on the desktop (docs/STYLE.md §4, docs/adr/0091; the web's
//! `ui/style/LayerStyleDialog.ts` and `rulesEditor.ts`): how a layer's
//! objects are drawn, as in QGIS's layer styling. Basit is the layer's own
//! colour and line type; Tek sembol gives every object one symbol per
//! geometry; Kategorili picks by an expression's value, Aralıklı by a
//! number's class; Kurallar by conditions and scale ranges; the thematic
//! kinds (docs/adr/0213, [`thematic`]) colour, size, dot, chart, heat,
//! cluster, spread or invert. Classes are made from the data
//! (`kentos_native_style::classify`, held to the web's by
//! `fixtures/style/v1/classify.json`) and then edited; nothing reaches the
//! drawing until Uygula or Tamam, which write the layer's renderer with
//! `cad.layers.renderer` as one undo step “Katman stili”.
//!
//! Every kind keeps its own draft, so switching back and forth loses
//! nothing. The counts follow the drawing (`kentos_native_style::tally`):
//! what each category, class and rule will draw.

mod panels;
mod rules;
#[cfg(test)]
mod tests;
mod thematic;
mod widgets;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use iced::Task;
use kentos_contracts::Entity;
use kentos_native_style::classify::{
    self, CLASS_COUNT_DEFAULT, DEFAULT_RAMP, Evaluated, ExprScope, Method, OTHER_COLOR, Present,
    categories_of, class_count, classes_present, graduated_of, js_number, new_category,
    numbers_per_object, plain_symbols, texts, unique_values, values_of,
};
use kentos_native_style::renderer::{
    Categorized, GeometryClass, Graduated, Renderer, Rule, Rules, Single, SymbolSet,
};
use kentos_native_style::simple::symbols_of_layer_style;
use kentos_native_style::tally::{RuleCount, drawable, rule_counts};
use serde_json::Value;

use crate::app::{App, Dialog, Message};

/// The renderer kinds the window offers, as its list names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Simple,
    Single,
    Categorized,
    Graduated,
    Unclassed,
    Proportional,
    Bivariate,
    Rules,
    DotDensity,
    Chart,
    Heatmap,
    Cluster,
    Displacement,
    Inverted,
    /// A renderer this version cannot read: kept as it is until another kind is chosen.
    Unknown,
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Kind::Simple => "Basit",
            Kind::Single => "Tek sembol",
            Kind::Categorized => "Kategorili",
            Kind::Graduated => "Aralıklı",
            Kind::Unclassed => "Sürekli renk",
            Kind::Proportional => "Orantılı sembol",
            Kind::Bivariate => "İki değişkenli renk",
            Kind::Rules => "Kurallar",
            Kind::DotDensity => "Nokta yoğunluğu",
            Kind::Chart => "Grafik",
            Kind::Heatmap => "Isı haritası",
            Kind::Cluster => "Kümeleme",
            Kind::Displacement => "Yayma",
            Kind::Inverted => "Ters alan",
            Kind::Unknown => "Bilinmeyen",
        })
    }
}

impl Kind {
    /// Its icon (the web's `ui/icons.ts`, the same on both).
    pub fn icon(self) -> &'static str {
        match self {
            Kind::Simple => "rendererSimple",
            Kind::Single => "rendererSingle",
            Kind::Categorized => "rendererCategorized",
            Kind::Graduated => "rendererGraduated",
            Kind::Unclassed => "rendererUnclassed",
            Kind::Proportional => "rendererProportional",
            Kind::Bivariate => "rendererBivariate",
            Kind::Rules => "rendererRules",
            Kind::DotDensity => "rendererDotDensity",
            Kind::Chart => "rendererChart",
            Kind::Heatmap => "rendererHeatmap",
            Kind::Cluster => "rendererCluster",
            Kind::Displacement => "rendererDisplacement",
            Kind::Inverted => "rendererInverted",
            Kind::Unknown => "warning",
        }
    }

    /// What it does, under its name in the list.
    pub fn detail(self) -> &'static str {
        match self {
            Kind::Simple => "Katmanın kendi rengi, çizgi tipi ve dolgusu.",
            Kind::Single => "Her nesne aynı sembolle.",
            Kind::Categorized => "Değere göre bir sembol.",
            Kind::Graduated => "Sayının sınıfına göre bir sembol.",
            Kind::Unclassed => "Sayı, rampada sürekli bir renk.",
            Kind::Proportional => "Sayı, sembolün boyu.",
            Kind::Bivariate => "İki sayının sınıfları, bir renk ızgarası.",
            Kind::Rules => "İfadeler ve ölçek aralıklarıyla.",
            Kind::DotDensity => "Alanın içinde değerle orantılı noktalar.",
            Kind::Chart => "Nesnenin üstünde pasta ya da çubuk.",
            Kind::Heatmap => "Noktaların yoğunluğu, renkli resim.",
            Kind::Cluster => "Yakın noktalar tek işaret ve sayısı.",
            Kind::Displacement => "Üst üste binen noktalar çevreye dağılır.",
            Kind::Inverted => "Alanların dışı boyanır.",
            Kind::Unknown => "KentOS'un bu sürümünün okuyamadığı stil.",
        }
    }

    /// The list's group it starts (docs/adr/0213 §4).
    pub fn group(self) -> Option<&'static str> {
        match self {
            Kind::Categorized => Some("Değere göre"),
            Kind::Rules => Some("Kurallar"),
            Kind::DotDensity => Some("Tematik"),
            Kind::Cluster => Some("Noktalar"),
            Kind::Inverted => Some("Alanlar"),
            _ => None,
        }
    }
}

/// The kinds in the list's order (the web's `KINDS`).
pub const KINDS: [Kind; 14] = [
    Kind::Simple,
    Kind::Single,
    Kind::Categorized,
    Kind::Graduated,
    Kind::Unclassed,
    Kind::Proportional,
    Kind::Bivariate,
    Kind::Rules,
    Kind::DotDensity,
    Kind::Chart,
    Kind::Heatmap,
    Kind::Cluster,
    Kind::Displacement,
    Kind::Inverted,
];

/// Where a symbol set sits in the window.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SetAt {
    Single,
    Category(usize),
    Other,
    Class(usize),
    Rule(Vec<usize>),
    Unclassed,
    UnclassedOther,
    Proportional,
    ProportionalOther,
    Bivariate,
    BivariateOther,
    /// Nokta yoğunluğu's and Grafik's ground: what the objects draw under the dots and charts.
    DotGround,
    ChartGround,
    Inverted,
    /// Kümeleme's mark and Yayma's centre: a marker each, not a set.
    ClusterMark,
    DisplacementCenter,
}

/// An expression field of the window.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Field {
    Categories,
    Classes,
    Rule(Vec<usize>),
    Unclassed,
    Proportional,
    BivariateX,
    BivariateY,
    /// Isı haritası's weight.
    Weight,
    /// A value of Nokta yoğunluğu's or Grafik's list.
    DotValue(usize),
    ChartValue(usize),
}

/// An edit of one rule.
#[derive(Clone, Debug)]
pub enum RuleEdit {
    Enabled(bool),
    Label(String),
    Else(bool),
    MinScale(String),
    MaxScale(String),
    Up,
    Down,
    AddChild,
    Remove,
}

/// What the window asks for.
#[derive(Clone, Debug)]
pub enum Event {
    /// Opens the window for a layer (Katmanlar's menu), or the active one.
    Open(Option<String>),
    Kind(Kind),
    /// → or ← with no field holding the keyboard: the next or the previous kind (the web's segmented control).
    Step(bool),
    /// Vazgeç, Esc or the backdrop: closes, or asks first when changes were not applied.
    Close,
    Apply,
    /// Tamam, or the question's “Uygula ve kapat”: applied, then closed.
    Done,
    /// The question's “Uygulamadan kapat”: closed, the changes dropped.
    Discard,
    /// The question's Vazgeç: the window stays as it was.
    Stay,
    /// An expression field's text as typed.
    Expr(Field, String),
    /// Text put at the end of an expression field (a field, a variable, a function).
    Insert(Field, String),
    /// Değerlerden sınıfla.
    Classify,
    AddCategory,
    ClearCategories,
    CategoryEnabled(usize, bool),
    CategoryValue(usize, String),
    CategoryLabel(usize, String),
    RemoveCategory(usize),
    /// Diğer değerler drawn or not.
    Other(bool),
    Method(Method),
    /// The class count as typed.
    Count(String),
    Ramp(&'static str),
    /// Sınıfla.
    Graduate,
    ClassMin(usize, String),
    ClassMax(usize, String),
    ClassLabel(usize, String),
    RemoveClass(usize),
    Rule(Vec<usize>, RuleEdit),
    AddRule,
    AddElse,
    /// A slot's symbol chosen: a set's class takes a symbol, or none (Basit görünüşe dön).
    Symbol(SetAt, GeometryClass, Option<Value>),
    /// A slot's “Kitaplıktan seç…”: Stil yöneticisi picks for it (its title, its library symbol).
    Pick(SetAt, GeometryClass, String, Option<String>),
    /// A slot's “Kitaplığıma kaydet”: its own symbol goes to Kitaplığım (its title, the symbol).
    Keep(SetAt, GeometryClass, String, Value),
    /// A slot's “Düzenle…” (“Kopyasını burada düzenle…” for a library symbol):
    /// Sembol tasarımcısı edits the symbol (its title, the symbol to start from)
    /// and Uygula writes it into the style.
    Design(SetAt, GeometryClass, String, Value),
    /// An edit of a thematic kind's panel (docs/adr/0213 §4).
    Thematic(thematic::Edit),
}

/// What each rule takes, by its path.
pub type Counts = Rc<HashMap<Vec<usize>, RuleCount>>;

/// The values the window read of the drawing, kept while it does not change.
#[derive(Default)]
struct Cache {
    generation: Option<u64>,
    values: HashMap<String, Rc<Evaluated<Option<String>>>>,
    numbers: HashMap<String, Rc<Evaluated<Option<f64>>>>,
    rules: Option<(String, Counts)>,
    drawn: Option<Rc<Vec<bool>>>,
}

/// What the window reads of the drawing.
pub struct Source<'a> {
    pub doc: &'a kentos_domain::Document,
    pub store: &'a kentos_processing::Store,
}

impl Source<'_> {
    /// Layer names from the tree, geometry values from the drawing's geometry store.
    fn scope<T>(&self, f: impl FnOnce(&ExprScope) -> T) -> T {
        let layers = self.doc.layers();
        let name = |id: &str| {
            layers
                .get(id)
                .map_or_else(|| id.to_owned(), |n| n.name.clone())
        };
        let store = self.store;
        let measures = |list: &[&Entity]| {
            let ids: Vec<f64> = list.iter().map(|e| f64::from(e.base().id)).collect();
            store.measures(&ids)
        };
        f(&ExprScope {
            layer_name: &name,
            measures: &measures,
        })
    }
}

static NEXT_RULE: AtomicU64 = AtomicU64::new(0);

/// A new rule's id: time and a sequence in base 36, as the web's `newRuleId`.
fn new_rule_id() -> String {
    fn base36(mut n: u128) -> String {
        const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
        let mut out = Vec::new();
        loop {
            out.push(DIGITS[(n % 36) as usize]);
            n /= 36;
            if n == 0 {
                break;
            }
        }
        out.reverse();
        String::from_utf8(out).unwrap_or_default()
    }
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let seq = NEXT_RULE.fetch_add(1, Ordering::Relaxed);
    format!("r{}{}", base36(ms), base36(u128::from(seq)))
}

/// The layer style window of one layer.
pub struct LayerStyleWindow {
    pub layer: String,
    pub kind: Kind,
    /// The layer's objects by geometry class when the window opened.
    pub present: Present,
    /// The classes the slots offer: those the layer has, all three when none.
    pub classes: Vec<GeometryClass>,
    pub single: SymbolSet,
    pub categorized: Categorized,
    pub graduated: Graduated,
    pub rules: Vec<Rule>,
    /// The thematic kinds' drafts (docs/adr/0213).
    pub drafts: thematic::Drafts,
    /// İki değişkenli renk's Sınıfla: its method and its colours' scheme.
    pub bivariate_method: Method,
    pub bivariate_scheme: usize,
    /// What a cluster's or a displacement's single points draw with: another kind's draft.
    pub inner: Kind,
    /// The layer's simple look: what a class without a symbol falls back to.
    pub simple: SymbolSet,
    /// A renderer this version could not read, and why.
    pub unknown: Option<(Value, String)>,
    pub method: Method,
    /// The class count field as typed.
    pub count: String,
    pub ramp: &'static str,
    /// The renderer the layer has now (None: its simple look).
    applied: Option<Value>,
    /// The last thing the window said, and whether it warns.
    said: Option<(String, bool)>,
    /// Closing asked about the changes not applied (the web's `askUnsaved`).
    pub asking: bool,
    /// Fields whose typed text is kept apart from the value it gives (expressions, numbers).
    pub(super) typed: HashMap<String, String>,
    cache: RefCell<Cache>,
}

impl LayerStyleWindow {
    /// The window of layer `id`, with the style it has; a group has none.
    pub fn open(doc: &kentos_domain::Document, id: &str) -> Result<Self, &'static str> {
        let node = doc
            .layers()
            .get(id)
            .filter(|n| n.kind == kentos_contracts::LayerNodeType::Layer)
            .ok_or("Katman stili yalnızca katmanlar için açılır; bir grup seçili.")?;
        let entities: Vec<&Entity> = doc.by_layer(id).collect();
        let present = classes_present(&entities);
        let classes = present.classes();
        let current = node.style.renderer.clone();
        let simple = SymbolSet::from_value(&symbols_of_layer_style(
            &node.style,
            &node.style.color,
            node.style.line_weight,
            false,
        ))
        .only(&classes);
        let (read, unknown) = match current.as_ref().map(Renderer::from_value) {
            None => (None, None),
            Some(Ok(r)) => (Some(r), None),
            Some(Err(why)) => (None, current.clone().map(|v| (v, why))),
        };
        let kind = match (&read, &unknown) {
            (Some(r), _) => kind_of(r),
            (None, Some(_)) => Kind::Unknown,
            (None, None) => Kind::Simple,
        };
        let field = guess_field(&entities);
        let names: Vec<String> = field_names(&entities);
        let drafts = thematic::Drafts::start(&node.style.color, &simple, &field, &names);
        let mut window = LayerStyleWindow {
            layer: id.to_owned(),
            kind,
            present,
            classes,
            single: Single::default().symbols,
            categorized: Categorized {
                expr: guess_field(&entities),
                ..Categorized::default()
            },
            graduated: Graduated {
                expr: "$alan".into(),
                ..Graduated::default()
            },
            rules: vec![Rule {
                symbols: Some(simple.clone()),
                ..Rule::new("r1".into(), "Bütün nesneler", false)
            }],
            drafts,
            bivariate_method: Method::Count,
            bivariate_scheme: 0,
            inner: Kind::Simple,
            simple: simple.clone(),
            unknown,
            method: Method::Interval,
            count: CLASS_COUNT_DEFAULT.to_string(),
            ramp: DEFAULT_RAMP,
            applied: current,
            said: None,
            asking: false,
            typed: HashMap::new(),
            cache: RefCell::new(Cache::default()),
        };
        window.single = simple;
        if let Some(r) = read {
            window.take(r);
        }
        Ok(window)
    }

    /// A renderer read into its kind's draft; a cluster's or a displacement's single points'
    /// into theirs (Tek noktalar).
    fn take(&mut self, r: Renderer) {
        match r {
            Renderer::Single(r) => self.single = r.symbols,
            Renderer::Categorized(r) => self.categorized = r,
            Renderer::Graduated(r) => self.graduated = r,
            Renderer::Rules(r) => self.rules = r.rules,
            Renderer::Unclassed(r) => self.drafts.unclassed = r,
            Renderer::Proportional(r) => self.drafts.proportional = r,
            Renderer::Bivariate(r) => self.drafts.bivariate = r,
            Renderer::DotDensity(r) => self.drafts.dot_density = r,
            Renderer::Chart(r) => self.drafts.chart = r,
            Renderer::Heatmap(r) => self.drafts.heatmap = r,
            Renderer::Cluster(mut r) => {
                self.take_inner(r.renderer.take());
                self.drafts.cluster = r;
            }
            Renderer::Displacement(mut r) => {
                self.take_inner(r.renderer.take());
                self.drafts.displacement = r;
            }
            Renderer::Inverted(r) => self.drafts.inverted = r,
        }
    }

    fn take_inner(&mut self, inner: Option<Value>) {
        if let Some(r) = inner.as_ref().and_then(|v| Renderer::from_value(v).ok()) {
            let kind = kind_of(&r);
            if thematic::INNER_KINDS.iter().any(|(k, _)| *k == kind) {
                self.inner = kind;
                self.take(r);
            }
        }
    }

    /// The renderer the window stands for now (None: the simple look).
    pub fn renderer(&self) -> Option<Value> {
        self.draft_of(self.kind)
    }

    /// A kind's draft as the renderer it writes.
    fn draft_of(&self, kind: Kind) -> Option<Value> {
        let trimmed = |s: &str| kentos_processing::text::js_trim(s).to_owned();
        let d = &self.drafts;
        match kind {
            Kind::Unclassed => Some(Renderer::Unclassed(d.unclassed.clone()).to_value()),
            Kind::Proportional => Some(Renderer::Proportional(d.proportional.clone()).to_value()),
            Kind::Bivariate => Some(Renderer::Bivariate(d.bivariate.clone()).to_value()),
            Kind::DotDensity => Some(Renderer::DotDensity(d.dot_density.clone()).to_value()),
            Kind::Chart => Some(Renderer::Chart(d.chart.clone()).to_value()),
            Kind::Heatmap => Some(Renderer::Heatmap(d.heatmap.clone()).to_value()),
            Kind::Inverted => Some(Renderer::Inverted(d.inverted.clone()).to_value()),
            Kind::Cluster => {
                let mut r = d.cluster.clone();
                r.renderer = self.draft_of(self.inner);
                Some(Renderer::Cluster(r).to_value())
            }
            Kind::Displacement => {
                let mut r = d.displacement.clone();
                r.renderer = self.draft_of(self.inner);
                Some(Renderer::Displacement(r).to_value())
            }
            Kind::Simple => None,
            Kind::Unknown => self.unknown.as_ref().map(|(v, _)| v.clone()),
            Kind::Single => Some(
                Renderer::Single(Single {
                    symbols: self.single.clone(),
                    ..Single::default()
                })
                .to_value(),
            ),
            Kind::Categorized => {
                let mut r = self.categorized.clone();
                r.expr = trimmed(&r.expr);
                Some(Renderer::Categorized(r).to_value())
            }
            Kind::Graduated => {
                let mut r = self.graduated.clone();
                r.expr = trimmed(&r.expr);
                Some(Renderer::Graduated(r).to_value())
            }
            Kind::Rules => Some(
                Renderer::Rules(Rules {
                    rules: self.rules.clone(),
                    ..Rules::default()
                })
                .to_value(),
            ),
        }
    }

    /// Whether the window holds changes Uygula has not written.
    pub fn unapplied(&self) -> bool {
        !same_renderer(&self.renderer(), &self.applied)
    }

    /// What the footer says: that edits wait for Uygula, else the last word.
    pub fn status(&self) -> Option<(String, bool)> {
        if self.unapplied() {
            return Some(("Değişiklikler henüz uygulanmadı.".into(), true));
        }
        self.said.clone()
    }

    pub(crate) fn say(&mut self, text: impl Into<String>, warn: bool) {
        self.said = Some((text.into(), warn));
    }

    /// Writes the renderer into the layer's style with `cad.layers.renderer`, one undo step
    /// “Katman stili”. Returns false when the command refuses it (the layer gone, a value out
    /// of its range: the footer says which).
    pub fn apply(&mut self, doc: &mut kentos_domain::Document) -> bool {
        use kentos_contracts::{CommandResult, LayersRenderer};
        use kentos_native_application::{ExecutionContext, layers_renderer};
        let r = self.renderer();
        // What the layer has (a renderer this version cannot read, kept): nothing to write.
        if same_renderer(&r, &self.applied) {
            return true;
        }
        let input = LayersRenderer {
            layer: self.layer.clone(),
            renderer: r.clone(),
            expected_revision: None,
        };
        match layers_renderer::execute(&mut ExecutionContext::new(doc), input) {
            CommandResult::Completed { .. } => {
                self.say(
                    if r.is_some() {
                        "Stil haritaya uygulandı."
                    } else {
                        "Katman basit görünüşüne döndü."
                    },
                    false,
                );
                self.applied = r;
                true
            }
            CommandResult::Failed { error } => {
                self.say(format!("Uygulanmadı: {}", error.message), true);
                false
            }
            _ => {
                self.say("Uygulanmadı: çizim değişti.", true);
                false
            }
        }
    }

    // ── What the drawing gives ─────────────────────────────────────────

    fn fresh(&self, doc: &kentos_domain::Document) {
        let mut c = self.cache.borrow_mut();
        if c.generation != Some(doc.generation()) {
            *c = Cache {
                generation: Some(doc.generation()),
                ..Cache::default()
            };
        }
    }

    fn entities<'a>(&self, doc: &'a kentos_domain::Document) -> Vec<&'a Entity> {
        doc.by_layer(&self.layer).collect()
    }

    /// The categorized expression's text per object.
    pub fn values(&self, src: &Source<'_>) -> Rc<Evaluated<Option<String>>> {
        self.fresh(src.doc);
        let expr = kentos_processing::text::js_trim(&self.categorized.expr).to_owned();
        if let Some(hit) = self.cache.borrow().values.get(&expr) {
            return hit.clone();
        }
        let list = self.entities(src.doc);
        let made = Rc::new(if expr.is_empty() {
            Evaluated {
                values: Vec::new(),
                error: None,
                at: 0,
            }
        } else {
            src.scope(|scope| values_of(&list, &expr, scope))
        });
        self.cache.borrow_mut().values.insert(expr, made.clone());
        made
    }

    /// The graduated expression's number per object (None: none).
    pub fn numbers(&self, src: &Source<'_>) -> Rc<Evaluated<Option<f64>>> {
        self.numbers_for(src, &self.graduated.expr)
    }

    /// An expression's number per object (None: none), kept while the drawing does not change.
    pub fn numbers_for(&self, src: &Source<'_>, expr: &str) -> Rc<Evaluated<Option<f64>>> {
        self.fresh(src.doc);
        let expr = kentos_processing::text::js_trim(expr).to_owned();
        if let Some(hit) = self.cache.borrow().numbers.get(&expr) {
            return hit.clone();
        }
        let list = self.entities(src.doc);
        let made = Rc::new(if expr.is_empty() {
            Evaluated {
                values: Vec::new(),
                error: None,
                at: 0,
            }
        } else {
            src.scope(|scope| numbers_per_object(&list, &expr, scope))
        });
        self.cache.borrow_mut().numbers.insert(expr, made.clone());
        made
    }

    /// Whether the style engine draws each of the layer's objects (not texts and dimensions).
    pub fn drawn(&self, src: &Source<'_>) -> Rc<Vec<bool>> {
        self.fresh(src.doc);
        if let Some(hit) = &self.cache.borrow().drawn {
            return hit.clone();
        }
        let made = Rc::new(drawable(&self.entities(src.doc)));
        self.cache.borrow_mut().drawn = Some(made.clone());
        made
    }

    /// What each rule takes, by its path.
    pub fn rule_counts(&self, src: &Source<'_>) -> Counts {
        self.fresh(src.doc);
        let key = serde_json::to_string(&self.rules).unwrap_or_default();
        if let Some((k, hit)) = &self.cache.borrow().rules
            && *k == key
        {
            return hit.clone();
        }
        let list = self.entities(src.doc);
        let made = Rc::new(src.scope(|scope| rule_counts(&self.rules, &list, scope)));
        self.cache.borrow_mut().rules = Some((key, made.clone()));
        made
    }

    /// The layer's attribute names with how many objects have each, in Turkish order.
    pub fn fields(&self, doc: &kentos_domain::Document) -> Vec<(String, usize)> {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for e in doc.by_layer(&self.layer) {
            for k in e.base().attrs.keys() {
                *counts.entry(k.as_str()).or_default() += 1;
            }
        }
        let mut out: Vec<(String, usize)> =
            counts.into_iter().map(|(k, n)| (k.to_owned(), n)).collect();
        out.sort_by(|a, b| classify::compare_text(&a.0, &b.0));
        out
    }

    // ── Edits ──────────────────────────────────────────────────────────

    /// An edit of the form; the drawing is read for Değerlerden sınıfla and Sınıfla.
    pub fn edit(&mut self, e: Event, src: &Source<'_>) {
        match e {
            Event::Kind(k) => self.kind = k,
            Event::Step(next) => {
                if let Some(i) = KINDS.iter().position(|k| *k == self.kind) {
                    let j = if next {
                        (i + 1).min(KINDS.len() - 1)
                    } else {
                        i.saturating_sub(1)
                    };
                    self.kind = KINDS[j];
                }
            }
            Event::Expr(field, text) => self.set_expr(&field, text),
            Event::Insert(field, text) => {
                let before = self.expr_text(&field);
                let pad = if before.is_empty() || before.ends_with([' ', '(', ',']) {
                    ""
                } else {
                    " "
                };
                self.set_expr(&field, format!("{before}{pad}{text}"));
            }
            Event::Classify => {
                let found = unique_values(&self.values(src).values);
                if found.is_empty() {
                    self.say(texts::NO_VALUES, true);
                } else {
                    self.categorized.categories =
                        categories_of(&found, &self.present, &self.categorized.categories);
                    self.typed.clear();
                    self.say(texts::found(found.len()), false);
                }
            }
            Event::AddCategory => {
                let n = self.categorized.categories.len();
                self.categorized
                    .categories
                    .push(new_category(n, &self.present));
            }
            Event::ClearCategories => {
                self.categorized.categories.clear();
                self.typed.clear();
            }
            Event::CategoryEnabled(i, on) => {
                if let Some(k) = self.categorized.categories.get_mut(i) {
                    k.enabled = if on { None } else { Some(false) };
                }
            }
            Event::CategoryValue(i, text) => {
                if let Some(k) = self.categorized.categories.get_mut(i) {
                    k.value = text;
                }
            }
            Event::CategoryLabel(i, text) => {
                if let Some(k) = self.categorized.categories.get_mut(i) {
                    k.label = text;
                }
            }
            Event::RemoveCategory(i) => {
                if i < self.categorized.categories.len() {
                    self.categorized.categories.remove(i);
                    self.typed.clear();
                }
            }
            Event::Other(on) => {
                self.categorized.other = on.then(|| plain_symbols(OTHER_COLOR, &self.present));
            }
            Event::Method(m) => self.method = m,
            Event::Count(text) => self.count = text,
            Event::Ramp(r) => self.ramp = r,
            Event::Graduate => {
                let numbers: Vec<f64> =
                    self.numbers(src).values.iter().flatten().copied().collect();
                if numbers.is_empty() {
                    return;
                }
                let classes = graduated_of(
                    &numbers,
                    self.method,
                    class_count(&self.count),
                    self.ramp,
                    &self.present,
                );
                let n = classes.len();
                self.graduated.classes = classes;
                self.typed.clear();
                self.say(texts::classified(n, numbers.len()), false);
            }
            Event::ClassMin(i, text) => self.class_bound(i, text, true),
            Event::ClassMax(i, text) => self.class_bound(i, text, false),
            Event::ClassLabel(i, text) => {
                if let Some(c) = self.graduated.classes.get_mut(i) {
                    c.label = text;
                }
            }
            Event::RemoveClass(i) => {
                if i < self.graduated.classes.len() {
                    self.graduated.classes.remove(i);
                    self.typed.clear();
                }
            }
            Event::Rule(path, edit) => self.edit_rule(&path, edit),
            Event::AddRule => {
                self.rules
                    .push(Rule::new(new_rule_id(), "Yeni kural", false));
            }
            Event::AddElse => {
                self.rules.push(Rule::new(new_rule_id(), "Diğerleri", true));
            }
            Event::Symbol(at, class, symbol) => self.put_symbol(&at, class, symbol),
            Event::Thematic(e) => self.thematic(e, src),
            Event::Open(_)
            | Event::Close
            | Event::Apply
            | Event::Done
            | Event::Discard
            | Event::Stay
            | Event::Pick(..)
            | Event::Keep(..)
            | Event::Design(..) => {}
        }
    }

    /// The typed text of an expression field.
    pub fn expr_text(&self, field: &Field) -> String {
        let d = &self.drafts;
        let value = |list: &[kentos_native_style::renderer::Field], i: usize| {
            list.get(i).map(|f| f.expr.clone()).unwrap_or_default()
        };
        match field {
            Field::Categories => self.categorized.expr.clone(),
            Field::Classes => self.graduated.expr.clone(),
            Field::Unclassed => d.unclassed.expr.clone(),
            Field::Proportional => d.proportional.expr.clone(),
            Field::BivariateX => d.bivariate.expr_x.clone(),
            Field::BivariateY => d.bivariate.expr_y.clone(),
            Field::Weight => d.heatmap.weight.clone().unwrap_or_default(),
            Field::DotValue(i) => value(&d.dot_density.fields, *i),
            Field::ChartValue(i) => value(&d.chart.fields, *i),
            Field::Rule(path) => self
                .typed
                .get(&filter_key(path))
                .cloned()
                .unwrap_or_else(|| {
                    rule_at(&self.rules, path)
                        .and_then(|r| r.filter.clone())
                        .unwrap_or_default()
                }),
        }
    }

    fn set_expr(&mut self, field: &Field, text: String) {
        let d = &mut self.drafts;
        match field {
            Field::Categories => self.categorized.expr = text,
            Field::Classes => self.graduated.expr = text,
            Field::Unclassed => d.unclassed.expr = text,
            Field::Proportional => d.proportional.expr = text,
            Field::BivariateX => d.bivariate.expr_x = text,
            Field::BivariateY => d.bivariate.expr_y = text,
            Field::Weight => {
                d.heatmap.weight =
                    (!kentos_processing::text::js_trim(&text).is_empty()).then_some(text);
            }
            Field::DotValue(i) => {
                if let Some(f) = d.dot_density.fields.get_mut(*i) {
                    f.expr = text;
                }
            }
            Field::ChartValue(i) => {
                if let Some(f) = d.chart.fields.get_mut(*i) {
                    f.expr = text;
                }
            }
            Field::Rule(path) => {
                let filter = kentos_processing::text::js_trim(&text).to_owned();
                if let Some(r) = rule_at_mut(&mut self.rules, path) {
                    r.filter = (!filter.is_empty()).then_some(filter);
                }
                self.typed.insert(filter_key(path), text);
            }
        }
    }

    /// A class bound as typed: taken when it reads as a number (a comma for the point).
    fn class_bound(&mut self, i: usize, text: String, min: bool) {
        let x = js_number(&text.replacen(',', ".", 1));
        if let Some(c) = self.graduated.classes.get_mut(i)
            && x.is_finite()
        {
            if min {
                c.min = x;
            } else {
                c.max = x;
            }
        }
        self.typed.insert(bound_key(i, min), text);
    }

    /// A slot's symbol: chosen in its menu, or from the symbol designer's Uygula.
    pub(crate) fn put_symbol(&mut self, at: &SetAt, class: GeometryClass, symbol: Option<Value>) {
        match at {
            SetAt::ClusterMark => self.drafts.cluster.symbol = symbol,
            SetAt::DisplacementCenter => self.drafts.displacement.center = symbol,
            _ => {
                if let Some(set) = self.set_mut(at) {
                    set.set(class, symbol);
                }
            }
        }
    }

    fn set_mut(&mut self, at: &SetAt) -> Option<&mut SymbolSet> {
        match at {
            SetAt::Single => Some(&mut self.single),
            SetAt::Category(i) => self
                .categorized
                .categories
                .get_mut(*i)
                .map(|k| &mut k.symbols),
            SetAt::Other => self.categorized.other.as_mut(),
            SetAt::Class(i) => self.graduated.classes.get_mut(*i).map(|c| &mut c.symbols),
            SetAt::Rule(path) => rule_at_mut(&mut self.rules, path)
                .map(|r| r.symbols.get_or_insert_with(SymbolSet::default)),
            SetAt::Unclassed => Some(&mut self.drafts.unclassed.symbols),
            SetAt::UnclassedOther => self.drafts.unclassed.other.as_mut(),
            SetAt::Proportional => Some(&mut self.drafts.proportional.symbols),
            SetAt::ProportionalOther => self.drafts.proportional.other.as_mut(),
            SetAt::Bivariate => Some(&mut self.drafts.bivariate.symbols),
            SetAt::BivariateOther => self.drafts.bivariate.other.as_mut(),
            SetAt::DotGround => Some(
                self.drafts
                    .dot_density
                    .symbols
                    .get_or_insert_with(SymbolSet::default),
            ),
            SetAt::ChartGround => Some(
                self.drafts
                    .chart
                    .symbols
                    .get_or_insert_with(SymbolSet::default),
            ),
            SetAt::Inverted => Some(&mut self.drafts.inverted.symbols),
            SetAt::ClusterMark | SetAt::DisplacementCenter => None,
        }
    }

    fn edit_rule(&mut self, path: &[usize], edit: RuleEdit) {
        let Some((&last, parent)) = path.split_last() else {
            return;
        };
        match edit {
            RuleEdit::Up | RuleEdit::Down | RuleEdit::Remove => {
                let Some(siblings) = siblings_mut(&mut self.rules, parent) else {
                    return;
                };
                match edit {
                    RuleEdit::Up if last > 0 && last < siblings.len() => {
                        siblings.swap(last, last - 1)
                    }
                    RuleEdit::Down if last + 1 < siblings.len() => siblings.swap(last, last + 1),
                    RuleEdit::Remove if last < siblings.len() => {
                        siblings.remove(last);
                    }
                    _ => return,
                }
                self.typed.clear();
            }
            RuleEdit::AddChild => {
                if let Some(r) = rule_at_mut(&mut self.rules, path) {
                    let child = Rule {
                        filter: None,
                        ..Rule::new(new_rule_id(), "Alt kural", false)
                    };
                    r.children.get_or_insert_with(Vec::new).push(child);
                }
            }
            RuleEdit::Enabled(on) => {
                if let Some(r) = rule_at_mut(&mut self.rules, path) {
                    r.enabled = if on { None } else { Some(false) };
                }
            }
            RuleEdit::Label(text) => {
                if let Some(r) = rule_at_mut(&mut self.rules, path) {
                    r.label = text;
                }
            }
            RuleEdit::Else(on) => {
                if let Some(r) = rule_at_mut(&mut self.rules, path) {
                    r.is_else = on.then_some(true);
                    if on {
                        r.filter = None;
                    }
                }
                self.typed.remove(&filter_key(path));
            }
            RuleEdit::MinScale(text) => self.rule_scale(path, text, true),
            RuleEdit::MaxScale(text) => self.rule_scale(path, text, false),
        }
    }

    /// A scale as typed, 1:N: dots and blanks between thousands are dropped,
    /// a comma is the point; empty (or not a positive number) is no bound.
    fn rule_scale(&mut self, path: &[usize], text: String, min: bool) {
        let cleaned: String = text
            .chars()
            .filter(|c| *c != '.' && !c.is_whitespace())
            .collect();
        let v = js_number(&cleaned.replacen(',', ".", 1));
        let value = (!text.trim().is_empty() && v.is_finite() && v > 0.0).then_some(v);
        if let Some(r) = rule_at_mut(&mut self.rules, path) {
            if min {
                r.min_scale = value;
            } else {
                r.max_scale = value;
            }
        }
        self.typed.insert(scale_key(path, min), text);
    }
}

/// Where a filter's typed text is kept.
fn filter_key(path: &[usize]) -> String {
    format!("filter/{path:?}")
}

/// Where a class bound's typed text is kept.
pub(super) fn bound_key(i: usize, min: bool) -> String {
    format!("{}/{i}", if min { "min" } else { "max" })
}

/// Where a rule's scale's typed text is kept.
pub(super) fn scale_key(path: &[usize], min: bool) -> String {
    format!("{}/{path:?}", if min { "minScale" } else { "maxScale" })
}

pub(super) fn rule_at<'a>(rules: &'a [Rule], path: &[usize]) -> Option<&'a Rule> {
    let (first, rest) = path.split_first()?;
    let r = rules.get(*first)?;
    if rest.is_empty() {
        Some(r)
    } else {
        rule_at(r.children(), rest)
    }
}

fn rule_at_mut<'a>(rules: &'a mut [Rule], path: &[usize]) -> Option<&'a mut Rule> {
    let (first, rest) = path.split_first()?;
    let r = rules.get_mut(*first)?;
    if rest.is_empty() {
        Some(r)
    } else {
        rule_at_mut(r.children.as_deref_mut()?, rest)
    }
}

/// The list a rule at `parent` + [i] sits in.
fn siblings_mut<'a>(rules: &'a mut Vec<Rule>, parent: &[usize]) -> Option<&'a mut Vec<Rule>> {
    match parent.split_first() {
        None => Some(rules),
        Some((first, rest)) => {
            let r = rules.get_mut(*first)?;
            siblings_mut(r.children.get_or_insert_with(Vec::new), rest)
        }
    }
}

/// Whether two renderers' JSON are the same, numbers by their value: a draft writes its
/// numbers as floats, a file may hold whole ones (4000 and 4000.0 alike).
fn same_json(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same_json(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| same_json(v, w)))
        }
        _ => a == b,
    }
}

fn same_renderer(a: &Option<Value>, b: &Option<Value>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => same_json(a, b),
        (None, None) => true,
        _ => false,
    }
}

/// The kind of a renderer read.
fn kind_of(r: &Renderer) -> Kind {
    match r {
        Renderer::Single(_) => Kind::Single,
        Renderer::Categorized(_) => Kind::Categorized,
        Renderer::Graduated(_) => Kind::Graduated,
        Renderer::Rules(_) => Kind::Rules,
        Renderer::Unclassed(_) => Kind::Unclassed,
        Renderer::Proportional(_) => Kind::Proportional,
        Renderer::Bivariate(_) => Kind::Bivariate,
        Renderer::DotDensity(_) => Kind::DotDensity,
        Renderer::Chart(_) => Kind::Chart,
        Renderer::Heatmap(_) => Kind::Heatmap,
        Renderer::Cluster(_) => Kind::Cluster,
        Renderer::Displacement(_) => Kind::Displacement,
        Renderer::Inverted(_) => Kind::Inverted,
    }
}

/// The objects' attribute names in Turkish order (the web's `fields()`).
fn field_names(entities: &[&Entity]) -> Vec<String> {
    let mut names: Vec<String> = entities
        .iter()
        .flat_map(|e| e.base().attrs.keys().cloned())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    names.sort_by(|a, b| classify::compare_text(a, b));
    names
}

/// The attribute most objects have: a good first guess for categories (`guessField`).
fn guess_field(entities: &[&Entity]) -> String {
    let mut order: Vec<(&str, usize)> = Vec::new();
    for e in entities {
        for k in e.base().attrs.keys() {
            match order.iter_mut().find(|(n, _)| *n == k.as_str()) {
                Some((_, c)) => *c += 1,
                None => order.push((k, 1)),
            }
        }
    }
    order.sort_by_key(|o| std::cmp::Reverse(o.1));
    order
        .first()
        .map(|(k, _)| (*k).to_owned())
        .unwrap_or_default()
}

impl App {
    /// Katman stili for layer `id` (the Katmanlar menu's), or the active layer (`style.layerStyle`).
    pub(crate) fn open_layer_style(&mut self, id: Option<String>) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        let id = id.unwrap_or_else(|| doc.model.layers().active().to_owned());
        match LayerStyleWindow::open(&doc.model, &id) {
            Ok(window) => {
                self.styles.layer_style = Some(window);
                self.dialog = Some(Dialog::LayerStyle);
            }
            Err(why) => self.warn(why),
        }
    }

    /// Whether the window may close now (Esc, Vazgeç, the backdrop): with
    /// changes not applied it asks first; asked, Esc answers “stay”.
    pub(crate) fn layer_style_may_close(&mut self) -> bool {
        let Some(window) = &mut self.styles.layer_style else {
            return true;
        };
        if window.asking {
            window.asking = false;
            return false;
        }
        if window.unapplied() {
            window.asking = true;
            return false;
        }
        self.styles.layer_style = None;
        true
    }

    pub(crate) fn layer_style_event(&mut self, event: Event) -> Task<Message> {
        let event = match event {
            Event::Open(id) => {
                self.open_layer_style(id);
                return Task::none();
            }
            // The library's windows (style/manager/).
            Event::Pick(at, class, title, current) => {
                return self.pick_for_slot(at, class, &title, current);
            }
            Event::Keep(at, class, title, symbol) => {
                self.keep_slot_symbol(at, class, &title, symbol);
                return Task::none();
            }
            // Sembol tasarımcısı over the window (style/designer/).
            Event::Design(at, class, title, symbol) => {
                self.open_slot_designer(at, class, title, symbol);
                return Task::none();
            }
            other => other,
        };
        let (Some(window), Some(doc)) = (&mut self.styles.layer_style, &mut self.document) else {
            return Task::none();
        };
        match event {
            Event::Close => self.close_dialog(),
            Event::Discard => {
                self.styles.layer_style = None;
                self.dialog = None;
            }
            Event::Stay => window.asking = false,
            // The kinds do not change under the question.
            Event::Step(_) if window.asking => {}
            Event::Apply => {
                window.apply(&mut doc.model);
            }
            Event::Done => {
                window.asking = false;
                if window.apply(&mut doc.model) {
                    self.styles.layer_style = None;
                    self.dialog = None;
                }
            }
            e => {
                let src = Source {
                    doc: &doc.model,
                    store: self.spatial.store(),
                };
                window.edit(e, &src);
            }
        }
        Task::none()
    }
}
