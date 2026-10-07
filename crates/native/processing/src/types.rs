//! The contract of a processing tool (the web's `processing/types.ts`):
//! parameters and their values, outputs, where a tool runs, what a run
//! receives and what it gives back. A tool never touches the document: it
//! receives resolved, read-only inputs and returns a [`ChangeSet`], which the
//! runner applies as one undo step.

use std::collections::BTreeMap;

use kentos_contracts::fields::LayerField;
use kentos_contracts::{AngleUnit, Entity, LabelStyle, LayerStyle, PointStyle, Vec2};
use kentos_domain::{Document, Slot};
use kentos_native_application::geometry::drawing_font;
use kentos_style_core::expr::Expr;
use serde_json::{Map, Value, json};

use crate::geometry::RunGeometry;

/// Values keyed by parameter name, as the dialog, history and models hold
/// them: JSON, the web's `Record<string, unknown>`.
pub type Values = Map<String, Value>;

/// What defaults may depend on: the project's settings when the dialog opens
/// (the web's `DefaultsContext`).
#[derive(Clone, Debug, PartialEq)]
pub struct Defaults {
    pub length_decimals: u32,
    pub area_decimals: u32,
    pub angle_unit: AngleUnit,
    pub plot_scale: f64,
    /// The drawing's typeface ("barlow"): texts a tool places are measured in it.
    pub drawing_font: &'static str,
    pub active_layer: String,
}

impl Defaults {
    /// The document's, as the web's `ProcessingRunner.defaults` reads them.
    pub fn of(doc: &Document) -> Self {
        let s = doc.settings();
        Self {
            length_decimals: s.length_decimals,
            area_decimals: s.area_decimals,
            angle_unit: s.angle_unit,
            plot_scale: s.plot_scale,
            drawing_font: drawing_font(s.drawing_font).id(),
            active_layer: doc.layers().active().to_owned(),
        }
    }

    /// As the shared cases write them (`documents.<file>.defaults`).
    pub fn to_json(&self) -> Value {
        json!({
            "lengthDecimals": self.length_decimals,
            "areaDecimals": self.area_decimals,
            "angleUnit": serde_json::to_value(self.angle_unit).unwrap_or(Value::Null),
            "plotScale": self.plot_scale,
            "drawingFont": self.drawing_font,
            "activeLayer": self.active_layer,
        })
    }
}

/// A fixed default, or one read from the project when the dialog opens.
#[derive(Clone)]
pub enum DefaultValue {
    Value(Value),
    From(fn(&Defaults) -> Value),
}

/// Which objects a features parameter may read (the dialog's scopes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeKind {
    Selection,
    Visible,
    All,
    Layer,
}

impl ScopeKind {
    pub fn id(self) -> &'static str {
        match self {
            ScopeKind::Selection => "selection",
            ScopeKind::Visible => "visible",
            ScopeKind::All => "all",
            ScopeKind::Layer => "layer",
        }
    }
}

/// What an expression parameter gives: true or false per object, or a value to write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Returns {
    Condition,
    Value,
}

/// A choice of an enum parameter.
#[derive(Clone, Debug, PartialEq)]
pub struct EnumOption {
    pub value: String,
    pub label: String,
    pub hint: Option<String>,
}

impl EnumOption {
    pub fn new(value: &str, label: &str) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            hint: None,
        }
    }

    pub fn hint(mut self, hint: &str) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

/// The style of a layer a tool creates for its output, over the new layer's
/// default style (the web's `Partial<LayerStyle>`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NewLayerStyle {
    pub color: Option<String>,
    pub point: Option<PointStyle>,
    pub label: Option<LabelStyle>,
}

impl NewLayerStyle {
    /// `{ ...base, ...this }`.
    pub fn over(&self, mut base: LayerStyle) -> LayerStyle {
        if let Some(color) = &self.color {
            base.color.clone_from(color);
        }
        if self.point.is_some() {
            base.point.clone_from(&self.point);
        }
        if self.label.is_some() {
            base.label.clone_from(&self.label);
        }
        base
    }
}

/// A parameter's type and the options that go with it.
#[derive(Clone)]
pub enum ParamKind {
    /// Objects to work on. `kinds`: the object kinds the tool takes (others in
    /// scope are left out); `scopes`: those offered (default: all but ids);
    /// `writes`: the tool changes these objects, so those on locked layers
    /// are left out when the input is resolved.
    Features {
        kinds: Option<Vec<String>>,
        scopes: Option<Vec<ScopeKind>>,
        writes: bool,
    },
    Number {
        min: Option<f64>,
        max: Option<f64>,
        integer: bool,
        /// Shown next to the field: "m", "m²", "mm", "°", "adet" or "".
        unit: String,
    },
    /// The web's `string`.
    Text {
        placeholder: Option<String>,
        max_length: Option<usize>,
        /// A required text may still be empty (an empty prefix).
        allow_empty: bool,
    },
    Boolean,
    /// The web's `enum`.
    Choice {
        options: Vec<EnumOption>,
    },
    /// Where the tool writes: an existing layer, or a new one made when the tool writes to it.
    Layer {
        new_layer_style: NewLayerStyle,
    },
    Point,
    /// An expression over each object (`of`: the features parameter it reads).
    Expression {
        returns: Returns,
        of: Option<String>,
        placeholder: Option<String>,
    },
    /// An attribute name of the objects of a features parameter, or a column
    /// of a file parameter's table; `allow_new` lets a new one be typed. `of`
    /// names the parameters the names come from, the first shown read (a
    /// source that is a layer or a file, docs/adr/0200 §6); `multiple`:
    /// several names, written with commas between them.
    Field {
        of: Vec<String>,
        allow_new: bool,
        multiple: bool,
    },
    /// A file the user chooses (docs/adr/0200 §7): a table read when it is
    /// chosen (CSV, TXT, XLSX: Tablo ekle's reader, the first sheet), its first
    /// row the column names; `accept`: the extensions offered (".csv").
    File {
        accept: Vec<String>,
    },
}

/// When a parameter is shown (the web's `visibleWhen`).
#[derive(Clone)]
pub enum ShownWhen {
    /// A rule in code (the built-in tools).
    Rule(fn(&Values) -> bool),
    /// While another parameter has this value: a rule written as data
    /// (`{ param, equals }`), as a model's or the shared cases' tools write it.
    Equals { param: String, value: Value },
}

impl ShownWhen {
    pub fn holds(&self, values: &Values) -> bool {
        match self {
            ShownWhen::Rule(rule) => rule(values),
            ShownWhen::Equals { param, value } => values.get(param) == Some(value),
        }
    }
}

/// A parameter: its name (the key in the values), what the dialog shows and
/// how its value is checked.
#[derive(Clone)]
pub struct ParamDef {
    pub name: String,
    pub label: String,
    pub description: Option<String>,
    /// May be left empty (null). Parameters are required by default.
    pub optional: bool,
    /// Folded under "Gelişmiş ayarlar" in the dialog.
    pub advanced: bool,
    /// Shown, and checked, only when this holds.
    pub visible_when: Option<ShownWhen>,
    pub default: Option<DefaultValue>,
    pub kind: ParamKind,
    /// A choice one of whose options is a point picked on the drawing (the
    /// numbering's start vertex, docs/adr/0088): the option, and the point
    /// parameter the pick fills. The window offers “Sahneden seç” beside it.
    pub picks: Option<(String, String)>,
}

impl ParamDef {
    pub fn new(name: &str, label: &str, kind: ParamKind) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            description: None,
            optional: false,
            advanced: false,
            visible_when: None,
            default: None,
            kind,
            picks: None,
        }
    }

    /// Picking a point on the drawing chooses `option` and fills `point`.
    pub fn picks_point(mut self, option: &str, point: &str) -> Self {
        self.picks = Some((option.into(), point.into()));
        self
    }

    pub fn describe(mut self, text: &str) -> Self {
        self.description = Some(text.into());
        self
    }

    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    pub fn advanced(mut self) -> Self {
        self.advanced = true;
        self
    }

    pub fn shown_when(mut self, when: fn(&Values) -> bool) -> Self {
        self.visible_when = Some(ShownWhen::Rule(when));
        self
    }

    pub fn default_value(mut self, value: Value) -> Self {
        self.default = Some(DefaultValue::Value(value));
        self
    }

    pub fn default_from(mut self, from: fn(&Defaults) -> Value) -> Self {
        self.default = Some(DefaultValue::From(from));
        self
    }

    /// The web's `type`: what models match sources by.
    pub fn type_name(&self) -> &'static str {
        match self.kind {
            ParamKind::Features { .. } => "features",
            ParamKind::Number { .. } => "number",
            ParamKind::Text { .. } => "string",
            ParamKind::Boolean => "boolean",
            ParamKind::Choice { .. } => "enum",
            ParamKind::Layer { .. } => "layer",
            ParamKind::Point => "point",
            ParamKind::Expression { .. } => "expression",
            ParamKind::Field { .. } => "field",
            ParamKind::File { .. } => "file",
        }
    }
}

/// What an output is: objects (a model feeds their ids to the next step), a
/// number or a text; or a table (`{ columns, rows }`) the dialog shows after
/// the run (docs/adr/0200 §7), not passed to a model's next step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputKind {
    Features,
    Number,
    Text,
    Table,
}

impl OutputKind {
    pub fn type_name(self) -> &'static str {
        match self {
            OutputKind::Features => "features",
            OutputKind::Number => "number",
            OutputKind::Text => "string",
            OutputKind::Table => "table",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OutputDef {
    pub name: String,
    pub label: String,
    pub kind: OutputKind,
}

impl OutputDef {
    pub fn new(name: &str, label: &str, kind: OutputKind) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            kind,
        }
    }
}

/// Where a tool can run. The desktop runs tools in the application
/// (`Client`); the others are the web's and the server's, declared by the
/// tools so the same definitions serve every host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Client,
    Worker,
    Server,
    Postgis,
}

impl Target {
    pub fn id(self) -> &'static str {
        match self {
            Target::Client => "client",
            Target::Worker => "worker",
            Target::Server => "server",
            Target::Postgis => "postgis",
        }
    }
}

/// What a tool does with its values: pure, over the document as it is.
pub type RunFn = fn(&Resolved<'_>, &RunContext<'_>, &mut dyn Feedback) -> RunResult;

/// A processing tool, declared (the web's `ProcessingTool`).
#[derive(Clone)]
pub struct Tool {
    /// "alan.eylem", English and stable.
    pub id: String,
    /// The Turkish name in the toolbox, menus and dialog title; the undo step's name.
    pub label: String,
    pub category: String,
    /// One sentence: what it does.
    pub description: String,
    /// Paragraphs separated by blank lines.
    pub help: Option<String>,
    pub keywords: Vec<String>,
    /// Short names for the command line ("KOSENUMARA").
    pub aliases: Vec<String>,
    pub icon: Option<String>,
    pub parameters: Vec<ParamDef>,
    pub outputs: Vec<OutputDef>,
    /// Where it runs, preferred first.
    pub targets: Vec<Target>,
    /// Checks across parameters: a message for the user, or none.
    pub validate: Option<fn(&Values) -> Option<String>>,
    /// The dialog's live one-line preview ("P00001, P00002 …").
    pub preview: Option<fn(&Values) -> Option<String>>,
    /// None for a model: its steps run one by one (`model_runner::run_model`).
    pub run: Option<RunFn>,
}

/// The objects a features parameter resolved to, with a short description ("12 kapalı alan; seçili nesneler").
#[derive(Clone, Debug, Default)]
pub struct FeatureSet<'d> {
    pub entities: Vec<&'d Entity>,
    pub description: String,
}

/// Where a layer parameter writes: an existing layer, or one the runner creates on apply.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TargetLayer {
    pub id: String,
    pub name: String,
    pub is_new: bool,
}

/// What `run` receives (the web's `ResolvedValues`): objects and layers
/// resolved, expressions compiled, the rest as entered.
#[derive(Default)]
pub struct Resolved<'d> {
    pub values: Values,
    pub features: BTreeMap<String, FeatureSet<'d>>,
    pub layers: BTreeMap<String, TargetLayer>,
    pub exprs: BTreeMap<String, Expr>,
    empty: FeatureSet<'d>,
    none: TargetLayer,
}

impl<'d> Resolved<'d> {
    pub fn new(values: Values) -> Self {
        Self {
            values,
            ..Self::default()
        }
    }

    pub fn number(&self, name: &str) -> Option<f64> {
        self.values.get(name).and_then(Value::as_f64)
    }

    pub fn text(&self, name: &str) -> &str {
        self.values.get(name).and_then(Value::as_str).unwrap_or("")
    }

    pub fn flag(&self, name: &str) -> bool {
        self.values
            .get(name)
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }

    pub fn point(&self, name: &str) -> Option<Vec2> {
        crate::values::point(self.values.get(name)?)
    }

    pub fn features(&self, name: &str) -> &FeatureSet<'d> {
        self.features.get(name).unwrap_or(&self.empty)
    }

    pub fn layer(&self, name: &str) -> &TargetLayer {
        self.layers.get(name).unwrap_or(&self.none)
    }

    /// A compiled expression; none when it was left empty.
    pub fn expr(&self, name: &str) -> Option<&Expr> {
        self.exprs.get(name)
    }
}

/// What a run may consult beyond its inputs (the web's `RunContext`).
pub struct RunContext<'d> {
    pub doc: &'d Document,
    pub units: &'d Defaults,
    /// Selected when the run started (selection tools combine with it).
    pub selection: &'d [Slot],
    /// The geometry of the run's features inputs, from the shared core.
    pub geometry: &'d RunGeometry,
    pub(crate) layer_names: BTreeMap<String, String>,
}

impl RunContext<'_> {
    /// A layer's name for an id (expressions, summaries); the id when unknown.
    pub fn layer_name<'a>(&'a self, id: &'a str) -> &'a str {
        self.layer_names.get(id).map_or(id, String::as_str)
    }

    /// A layer's field of that name (docs/adr/0199 §1): what a value written to the attribute becomes.
    pub fn field(&self, layer_id: &str, name: &str) -> Option<&LayerField> {
        self.doc
            .layers()
            .get(layer_id)?
            .fields
            .iter()
            .find(|f| f.name == name)
    }
}

/// Progress, messages and cancellation, shared with the dialog.
pub trait Feedback {
    /// Share done in 0..1, with a step label.
    fn progress(&mut self, fraction: f64, label: &str);
    fn info(&mut self, message: String);
    fn warn(&mut self, message: String);
    fn canceled(&self) -> bool;
}

/// A change to one object: its whole attribute table and, when it changes, its label.
#[derive(Clone, Debug, PartialEq)]
pub struct Patch {
    pub id: Slot,
    pub attrs: Option<BTreeMap<String, String>>,
    pub label: Option<Option<String>>,
}

/// The document edits a run asks for, applied by the runner in one transaction.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChangeSet {
    /// New objects; their ids are given on apply.
    pub add: Vec<Entity>,
    pub update: Vec<Patch>,
    pub remove: Vec<Slot>,
}

/// What a run gives back (the web's `RunResult`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunResult {
    pub changes: Option<ChangeSet>,
    /// The selection after the run, for tools that select rather than edit.
    pub select: Option<Vec<Slot>>,
    /// Number and text outputs, and id lists for features outputs.
    pub outputs: Values,
    /// One line for the log and history.
    pub summary: Option<String>,
    /// The run refuses, and says why (a file without the key column):
    /// nothing changes and the run ends as an error with this message as it is.
    pub refused: Option<String>,
}

impl RunResult {
    /// A run that refuses with this message.
    pub fn refused(message: String) -> Self {
        Self {
            refused: Some(message),
            ..Self::default()
        }
    }
}
