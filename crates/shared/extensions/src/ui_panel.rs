//! An extension's panel, `ui/<id>.panel.json` (docs/adr/8057 §24): what a dock shows of it, declared in the package,
//! drawn by KentOS with its own components. The extension's code never gets a widget or a toolkit type (a plugin
//! ecosystem's lesson in plan/eklenti.md: 58 % of its plugins were left behind when its toolkit changed).
//!
//! A button runs one of the extension's commands (`entrypoints.commands`) with the panel's values as its parameters
//! (`ctx.params`, by each input's id); the rows the command answers fill the panel's table. The command runs as any
//! other: in the extension's process, through the broker, one undo step, Durdur, its log.
//!
//! Checked here when the package is read, each problem with its JSON pointer.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::layer_schema::FIELD_KINDS;
use crate::problem::Problem;

/// `format`.
pub const FORMAT: &str = "kentos.ui.panel";
/// The `schemaVersion` this KentOS draws.
pub const VERSION: u64 = 1;
/// Components a panel may have.
pub const MOST_COMPONENTS: usize = 100;

pub mod crs;
mod files;
/// The components' Python builders, `kentos.ui_panel` (docs/adr/8057 §34), made from [`json_schema`].
#[cfg(feature = "schema")]
pub mod python;
pub mod rows;
pub mod rules;
pub mod tabs;
pub mod values;

pub use rules::Rules;

/// A panel as KentOS draws it, read after [`check`] passed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PanelDefinition {
    pub title: String,
    pub body: Vec<Component>,
    /// Each component's rules, in the body's order ([`rules`]): written beside its keys, read by [`read`].
    #[serde(skip)]
    #[cfg_attr(feature = "schema", schemars(skip))]
    pub rules: Vec<Rules>,
}

/// One of the kinds of file a file picker offers: its name and its extensions, without the dot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FileFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

/// One of a select's choices.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Choice {
    pub value: String,
    pub label: String,
}

/// A panel's component, in KentOS's words.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Component {
    /// A line of text.
    Text {
        #[serde(default)]
        id: Option<String>,
        text: String,
    },
    /// A heading that gathers the components after it, up to the next section: the user folds it to hide them
    /// (`collapsed`: folded when the panel opens). A folded section's values still go to the commands, and one of its
    /// fields left wrong opens it.
    Section {
        #[serde(default)]
        id: Option<String>,
        label: String,
        #[serde(default)]
        collapsed: bool,
    },
    /// A text the user types.
    TextField {
        id: String,
        label: String,
        /// The user must give it: left empty, it is said by its label and nothing runs.
        #[serde(default)]
        required: bool,
        #[serde(default)]
        placeholder: Option<String>,
        #[serde(default)]
        value: Option<String>,
    },
    /// A number the user types, in its bounds.
    NumberField {
        id: String,
        label: String,
        /// The user must give it: left empty, it is said by its label and nothing runs.
        #[serde(default)]
        required: bool,
        #[serde(default)]
        value: Option<f64>,
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
        #[serde(default)]
        unit: Option<String>,
    },
    /// One of its choices.
    Select {
        id: String,
        label: String,
        options: Vec<Choice>,
        #[serde(default)]
        value: Option<String>,
        /// A command run with the panel's values when the user changes it (its rows to the panel's first table).
        #[serde(default, rename = "onChange")]
        on_change: Option<String>,
    },
    /// One of the open drawing's layers, of `geometry`'s type (and the untyped ones) when given: its id.
    LayerPicker {
        id: String,
        label: String,
        /// The user must give it: left empty, it is said by its label and nothing runs.
        #[serde(default)]
        required: bool,
        #[serde(default)]
        geometry: Option<String>,
        #[serde(default, rename = "onChange")]
        on_change: Option<String>,
    },
    /// One of the fields of the layer the panel's layer picker `layer` holds, of `kinds` only when given (the layer
    /// schema's words): its name, none when none is chosen.
    FieldPicker {
        id: String,
        label: String,
        /// The user must give it: left empty, it is said by its label and nothing runs.
        #[serde(default)]
        required: bool,
        layer: String,
        #[serde(default)]
        kinds: Vec<String>,
        #[serde(default, rename = "onChange")]
        on_change: Option<String>,
    },
    /// A file the user gives to be read: its path typed, or chosen in the computer's file window (Seç…), of
    /// `filters`' kinds when given. Its value is the file's absolute path, none until one is given; a path that is no
    /// file, or of another kind, is said by its label and nothing runs. Its extension asks for `files.userSelected`.
    /// With `save`, where a file is to be written (docs/adr/8057 §44): the file need not be there, its folder must; a
    /// name without an extension takes the first kind's; its extension asks for `files.userSelectedWrite`.
    FilePicker {
        id: String,
        label: String,
        /// The user must give it: left empty, it is said by its label and nothing runs.
        #[serde(default)]
        required: bool,
        #[serde(default)]
        filters: Vec<FileFilter>,
        #[serde(default)]
        placeholder: Option<String>,
        #[serde(default, rename = "onChange")]
        on_change: Option<String>,
        /// Where a file is to be written, not one to be read.
        #[serde(default)]
        save: bool,
        /// The name the save window offers (`parseller.csv`).
        #[serde(default, rename = "defaultName")]
        default_name: Option<String>,
    },
    /// A coordinate system of KentOS's list (docs/adr/8057 §45): its value `EPSG:<code>`, `value` when given, else
    /// the open drawing's own (none for a drawing without one). The code reads it as it is (`pyproj.CRS(value)` too);
    /// KentOS's `cad.crs.transform` takes its code (`CrsSide(srid=…)`).
    CrsPicker {
        id: String,
        label: String,
        /// The user must give it: left empty, it is said by its label and nothing runs.
        #[serde(default)]
        required: bool,
        #[serde(default)]
        value: Option<String>,
        #[serde(default, rename = "onChange")]
        on_change: Option<String>,
    },
    /// A point of the drawing (docs/adr/8057 §57): typed as `x, y` or picked on the map with "Haritadan seç" (snaps
    /// apply); its value `[x, y]` in the drawing's plane, metres, none when empty.
    PointPicker {
        id: String,
        label: String,
        /// The user must give it: left empty, it is said by its label and nothing runs.
        #[serde(default)]
        required: bool,
        #[serde(default)]
        value: Option<[f64; 2]>,
        #[serde(default, rename = "onChange")]
        on_change: Option<String>,
    },
    /// The objects selected in the drawing, taken when the user asks: of the layer the panel's layer picker `layer`
    /// holds and of `geometry`'s type, when given. Its value is their persistent ids (`multiple`, the default), else
    /// one id or none.
    FeaturePicker {
        id: String,
        label: String,
        /// The user must give it: left empty, it is said by its label and nothing runs.
        #[serde(default)]
        required: bool,
        #[serde(default)]
        layer: Option<String>,
        #[serde(default)]
        geometry: Option<String>,
        #[serde(default = "many")]
        multiple: bool,
        #[serde(default, rename = "onChange")]
        on_change: Option<String>,
    },
    /// Several of its options the user picks, each a chip that toggles: its value is the chosen options' values in the
    /// options' order, an empty list when none is chosen.
    MultiSelect {
        id: String,
        label: String,
        /// The user must give it: left empty, it is said by its label and nothing runs.
        #[serde(default)]
        required: bool,
        options: Vec<Choice>,
        #[serde(default)]
        value: Vec<String>,
        #[serde(default, rename = "onChange")]
        on_change: Option<String>,
    },
    /// A date the user types (GG.AA.YYYY or YYYY-AA-GG) or picks from KentOS's calendar; its value is the ISO date
    /// (`2026-11-15`), none when empty. `value`, `min` and `max` are ISO dates.
    DateField {
        id: String,
        label: String,
        /// The user must give it: left empty, it is said by its label and nothing runs.
        #[serde(default)]
        required: bool,
        #[serde(default)]
        value: Option<String>,
        #[serde(default)]
        min: Option<String>,
        #[serde(default)]
        max: Option<String>,
        #[serde(default, rename = "onChange")]
        on_change: Option<String>,
    },
    /// A time of day the user types (14:30, 14.30, 9:05) or picks from KentOS's time picker; its value is `HH:MM`
    /// (24 hours), none when empty. `value`, `min` and `max` are `HH:MM`.
    TimeField {
        id: String,
        label: String,
        /// The user must give it: left empty, it is said by its label and nothing runs.
        #[serde(default)]
        required: bool,
        #[serde(default)]
        value: Option<String>,
        #[serde(default)]
        min: Option<String>,
        #[serde(default)]
        max: Option<String>,
        #[serde(default, rename = "onChange")]
        on_change: Option<String>,
    },
    /// A day and a time the user types (`15.11.2026 14:30`, `2026-11-15 14:30`) or picks from KentOS's calendar and
    /// clock; its value is the local ISO date and time, `2026-11-15T14:30` (no zone: the user's own clock), none when
    /// empty. `value`, `min` and `max` are written so.
    DateTimeField {
        id: String,
        label: String,
        /// The user must give it: left empty, it is said by its label and nothing runs.
        #[serde(default)]
        required: bool,
        #[serde(default)]
        value: Option<String>,
        #[serde(default)]
        min: Option<String>,
        #[serde(default)]
        max: Option<String>,
        #[serde(default, rename = "onChange")]
        on_change: Option<String>,
    },
    /// Yes or no.
    Checkbox {
        id: String,
        label: String,
        #[serde(default)]
        value: bool,
        #[serde(default, rename = "onChange")]
        on_change: Option<String>,
    },
    /// Runs `command` with the panel's values; its rows fill `table` (none: the panel's first table).
    Button {
        id: String,
        label: String,
        command: String,
        #[serde(default)]
        table: Option<String>,
        #[serde(default)]
        primary: bool,
    },
    /// The rows a button's command answered.
    Table {
        id: String,
        #[serde(default)]
        label: Option<String>,
    },
    /// The rows a button's command answered, grouped by `group`'s columns (one to three) into nodes that open and
    /// close, each with its count; a row with a `uid` is an object.
    Tree {
        id: String,
        #[serde(default)]
        label: Option<String>,
        group: Vec<String>,
    },
    /// The rows a button's command answered, a figure each (KentOS's stat card): `title`'s column names it (none: the
    /// first column), `value`'s is its figure (none: the second), `unit`'s and `note`'s beside and under it; the cards
    /// side by side as the panel's width allows. A row with a `uid` is an object.
    Stats {
        id: String,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        value: Option<String>,
        #[serde(default)]
        unit: Option<String>,
        #[serde(default)]
        note: Option<String>,
    },
    /// A chart of rows, drawn by KentOS's charts (the Grafik tab's engine): bars or a pie of `category`'s values,
    /// counted or with `value`'s column summed, averaged … (`aggregate`); a histogram of `value`'s numbers; a line of
    /// `value` over `category`. Its rows are the ones a button's `table` sends it or, with `of`, another component's (a
    /// table's, a tree's, a card's, a figure's): the same rows drawn twice, each its own way.
    Chart {
        id: String,
        #[serde(default)]
        label: Option<String>,
        /// None: bars.
        #[serde(default)]
        kind: Option<ChartKind>,
        #[serde(default)]
        of: Option<String>,
        #[serde(default)]
        category: Option<String>,
        #[serde(default)]
        value: Option<String>,
        /// None: a count, or a sum when `value` is given (bars and a pie).
        #[serde(default)]
        aggregate: Option<ChartAggregate>,
    },
    /// A picture its code gives (`ctx.panel.set(id, image=…)`, a PNG): a chart drawn in Python, a preview. `alt` says
    /// what it shows and stands in its place until the code gives one; `height` in pixels (80–600, none: 200), the
    /// picture fitted inside it.
    Image {
        id: String,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        alt: Option<String>,
        #[serde(default)]
        height: Option<f64>,
    },
    /// The rows a button's command answered, a card each: `title`'s column heads it (none: the first column), `tag`'s
    /// beside it, the other columns under it by name; a card with a `uid` is an object. Where a table's columns do not
    /// fit (a narrow dock), a card writes them one under the other.
    Card {
        id: String,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        tag: Option<String>,
    },
}

/// A feature picker takes many objects unless it says otherwise.
fn many() -> bool {
    true
}

/// What a chart draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum ChartKind {
    Bar,
    Pie,
    Histogram,
    Line,
}

/// How a category's rows give one number on a chart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum ChartAggregate {
    Count,
    Sum,
    Mean,
    Min,
    Max,
    Median,
}

/// A chart's kinds and aggregates as a panel writes them.
const CHART_KINDS: [&str; 4] = ["bar", "pie", "histogram", "line"];
const CHART_AGGREGATES: [&str; 6] = ["count", "sum", "mean", "min", "max", "median"];

impl Component {
    /// Its id, when it has one.
    pub fn id(&self) -> Option<&str> {
        match self {
            Component::Text { id, .. } | Component::Section { id, .. } => id.as_deref(),
            Component::TextField { id, .. }
            | Component::NumberField { id, .. }
            | Component::Select { id, .. }
            | Component::LayerPicker { id, .. }
            | Component::FieldPicker { id, .. }
            | Component::FeaturePicker { id, .. }
            | Component::FilePicker { id, .. }
            | Component::CrsPicker { id, .. }
            | Component::PointPicker { id, .. }
            | Component::DateField { id, .. }
            | Component::TimeField { id, .. }
            | Component::DateTimeField { id, .. }
            | Component::MultiSelect { id, .. }
            | Component::Checkbox { id, .. }
            | Component::Button { id, .. }
            | Component::Table { id, .. }
            | Component::Tree { id, .. }
            | Component::Stats { id, .. }
            | Component::Chart { id, .. }
            | Component::Image { id, .. }
            | Component::Card { id, .. } => Some(id),
        }
    }

    /// The command it runs when the user changes it, when it names one.
    pub fn on_change(&self) -> Option<&str> {
        match self {
            Component::Select { on_change, .. }
            | Component::LayerPicker { on_change, .. }
            | Component::FieldPicker { on_change, .. }
            | Component::FeaturePicker { on_change, .. }
            | Component::FilePicker { on_change, .. }
            | Component::CrsPicker { on_change, .. }
            | Component::PointPicker { on_change, .. }
            | Component::DateField { on_change, .. }
            | Component::TimeField { on_change, .. }
            | Component::DateTimeField { on_change, .. }
            | Component::MultiSelect { on_change, .. }
            | Component::Checkbox { on_change, .. } => on_change.as_deref(),
            _ => None,
        }
    }

    /// Whether the user must give it a value (its `required`).
    pub fn required(&self) -> bool {
        match self {
            Component::TextField { required, .. }
            | Component::NumberField { required, .. }
            | Component::LayerPicker { required, .. }
            | Component::FieldPicker { required, .. }
            | Component::FeaturePicker { required, .. }
            | Component::FilePicker { required, .. }
            | Component::CrsPicker { required, .. }
            | Component::PointPicker { required, .. }
            | Component::MultiSelect { required, .. }
            | Component::DateField { required, .. }
            | Component::TimeField { required, .. }
            | Component::DateTimeField { required, .. } => *required,
            _ => false,
        }
    }

    /// Whether the user gives it a value (a command's parameter).
    pub fn is_input(&self) -> bool {
        matches!(
            self,
            Component::TextField { .. }
                | Component::NumberField { .. }
                | Component::Select { .. }
                | Component::LayerPicker { .. }
                | Component::FieldPicker { .. }
                | Component::FeaturePicker { .. }
                | Component::FilePicker { .. }
                | Component::CrsPicker { .. }
                | Component::PointPicker { .. }
                | Component::DateField { .. }
                | Component::TimeField { .. }
                | Component::DateTimeField { .. }
                | Component::MultiSelect { .. }
                | Component::Checkbox { .. }
        )
    }
}

/// `kentos.ui.panel` 1 as a JSON Schema, made from the types KentOS reads a panel into (an editor's checking and
/// completion, an agent's reference): the components, their keys and kinds, with `format` and `schemaVersion`. What a
/// schema cannot say (a button's command among the extension's, a picker's layer picker in the panel, ids once) is
/// Kapı 1's: [`check`] stays the judge.
#[cfg(feature = "schema")]
pub fn json_schema() -> Value {
    let mut schema = schemars::schema_for!(PanelDefinition).to_value();
    if let Some(o) = schema.as_object_mut() {
        o.insert(
            "title".into(),
            Value::from("KentOS panel (kentos.ui.panel 1)"),
        );
        if let Some(Value::Object(props)) = o.get_mut("properties") {
            props.insert("format".into(), serde_json::json!({ "const": FORMAT }));
            props.insert(
                "schemaVersion".into(),
                serde_json::json!({ "const": VERSION }),
            );
        }
        o.insert(
            "required".into(),
            serde_json::json!(["format", "schemaVersion", "title", "body"]),
        );
        o.insert("additionalProperties".into(), Value::from(false));
    }
    // The rules (rules.rs) are read beside a component's keys, not into its type: each component's schema says the
    // ones it takes, for an editor's completion.
    if let Some(Value::Array(kinds)) = schema.pointer_mut("/$defs/Component/oneOf") {
        for kind in kinds {
            let name = kind
                .pointer("/properties/type/const")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let Some(known) = keys_of(&name) else {
                continue;
            };
            if let Some(Value::Object(props)) = kind.get_mut("properties") {
                for key in rules::RULE_KEYS
                    .iter()
                    .filter(|r| takes_rule(&name, known, r))
                {
                    let words = match *key {
                        "visibleWhen" => {
                            "Shown only while this expression over the panel's inputs (by id) is true."
                        }
                        "requiredWhen" => "Asked of the user while this expression is true.",
                        _ => "Usable (pressed, typed in) only while this expression is true.",
                    };
                    props.insert(
                        (*key).to_owned(),
                        serde_json::json!({ "type": "string", "description": words }),
                    );
                }
                for key in values::VALUE_KEYS
                    .iter()
                    .filter(|k| values::takes_value(&name, k))
                {
                    let words = match *key {
                        "calculate" => {
                            "Its value, worked out from the panel's inputs (by id) as they change: not typed, not set by \
                             code (a text's words; its text shows while this gives nothing)."
                        }
                        _ => {
                            "Its first value, worked out when the panel opens (bugün(), şimdi() and the inputs above it)."
                        }
                    };
                    props.insert(
                        (*key).to_owned(),
                        serde_json::json!({ "type": "string", "description": words }),
                    );
                }
                props.insert(
                    tabs::TAB_KEY.to_owned(),
                    serde_json::json!({ "type": "string", "maxLength": 40, "description":
                        "The tab (its words) whose page this component is on: the components with the same tab share \
                         a page, the bar lists the tabs in the order they first appear; without one, on every page." }),
                );
                if rows::takes_row(&name) {
                    props.insert(
                        rows::ROW_KEY.to_owned(),
                        serde_json::json!({ "type": "string", "maxLength": 40, "description":
                            "The row (its name) whose line this component shares: the components next to each other \
                             with the same row stand side by side where each has room, one under another where not." }),
                    );
                }
            }
        }
    }
    schema
}

/// The panel, when `value` passed [`check`]; none otherwise.
pub fn read(value: &Value) -> Option<PanelDefinition> {
    let mut panel: PanelDefinition = serde_json::from_value(value.clone()).ok()?;
    panel.rules = value
        .get("body")
        .and_then(Value::as_array)
        .map(|body| body.iter().map(Rules::of).collect())
        .unwrap_or_default();
    panel.rules.resize(panel.body.len(), Rules::default());
    Some(panel)
}

/// Whether component `kind` (its keys `known`) takes rule `key`: every component is shown by a rule, an input that
/// can be asked for is asked by one, a button or a typed field is usable by one.
fn takes_rule(kind: &str, known: &[&str], key: &str) -> bool {
    match key {
        "visibleWhen" => true,
        "requiredWhen" => known.contains(&"required"),
        "enabledWhen" => matches!(
            kind,
            "button"
                | "textField"
                | "numberField"
                | "dateField"
                | "timeField"
                | "dateTimeField"
                | "filePicker"
        ),
        _ => false,
    }
}

/// The component types and the keys each takes besides `type`.
fn keys_of(kind: &str) -> Option<&'static [&'static str]> {
    Some(match kind {
        "text" => &["id", "text"],
        "section" => &["id", "label", "collapsed"],
        "textField" => &["id", "label", "placeholder", "value", "required"],
        "numberField" => &["id", "label", "value", "min", "max", "unit", "required"],
        "select" => &["id", "label", "options", "value", "onChange"],
        "layerPicker" => &["id", "label", "geometry", "onChange", "required"],
        "fieldPicker" => &["id", "label", "layer", "kinds", "onChange", "required"],
        "featurePicker" => &[
            "id", "label", "layer", "geometry", "multiple", "onChange", "required",
        ],
        "filePicker" => &[
            "id",
            "label",
            "filters",
            "placeholder",
            "onChange",
            "required",
            "save",
            "defaultName",
        ],
        "crsPicker" | "pointPicker" => &["id", "label", "value", "onChange", "required"],
        "checkbox" => &["id", "label", "value", "onChange"],
        "dateField" | "timeField" | "dateTimeField" => {
            &["id", "label", "value", "min", "max", "onChange", "required"]
        }
        "multiSelect" => &["id", "label", "options", "value", "onChange", "required"],
        "button" => &["id", "label", "command", "table", "primary"],
        "table" => &["id", "label"],
        "tree" => &["id", "label", "group"],
        "card" => &["id", "label", "title", "tag"],
        "image" => &["id", "label", "alt", "height"],
        "stats" => &["id", "label", "title", "value", "unit", "note"],
        "chart" => &[
            "id",
            "label",
            "kind",
            "of",
            "category",
            "value",
            "aggregate",
        ],
        _ => return None,
    })
}

const TYPES: &str = "text, section, textField, numberField, dateField, timeField, dateTimeField, select, multiSelect, layerPicker, \
                     fieldPicker, featurePicker, filePicker, crsPicker, pointPicker, checkbox, button, table, tree, card, \
                     stats, chart, image";

/// A point given as JSON (`[x, y]`, two finite numbers); none otherwise.
pub fn point_value(value: &Value) -> Option<[f64; 2]> {
    let xy = value.as_array().filter(|a| a.len() == 2)?;
    let (x, y) = (xy[0].as_f64()?, xy[1].as_f64()?);
    (x.is_finite() && y.is_finite()).then_some([x, y])
}

/// A point as a field shows it: `x, y` to the millimetre.
pub fn point_words([x, y]: [f64; 2]) -> String {
    format!("{x:.3}, {y:.3}")
}

/// A point as the user types it: `x, y` or `x y` with decimal points, or `x; y` with decimal commas too; none for
/// anything else.
pub fn typed_point(text: &str) -> Option<[f64; 2]> {
    let text = text.trim();
    let parts: Vec<String> = if text.contains(';') {
        text.split(';')
            .map(|p| p.trim().replace(',', "."))
            .collect()
    } else if text.split_whitespace().count() == 2 && !text.contains(", ") {
        text.split_whitespace().map(str::to_owned).collect()
    } else {
        text.split(',').map(|p| p.trim().to_owned()).collect()
    };
    let [x, y] = parts.as_slice() else {
        return None;
    };
    let (x, y) = (x.parse::<f64>().ok()?, y.parse::<f64>().ok()?);
    (x.is_finite() && y.is_finite()).then_some([x, y])
}

/// The component types Kapı 1 knows, in its words' order: the SDK's guide and ADR 8057 §25 name each.
pub fn component_types() -> impl Iterator<Item = &'static str> {
    TYPES.split(", ")
}

/// An ISO date (`2026-11-15`), a real day of the calendar, as (year, month, day); none otherwise.
pub fn iso_date(text: &str) -> Option<(u32, u32, u32)> {
    let mut parts = text.split('-');
    let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || y.len() != 4 || m.len() != 2 || d.len() != 2 {
        return None;
    }
    let (y, m, d): (u32, u32, u32) = (y.parse().ok()?, m.parse().ok()?, d.parse().ok()?);
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let days = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    (y >= 1 && (1..=days).contains(&d)).then_some((y, m, d))
}

/// A date as a user writes it, `15.11.2026` or `2026-11-15` (one-digit day and month too): its ISO form.
pub fn typed_date(text: &str) -> Option<String> {
    let text = text.trim();
    if let Some((y, m, d)) = iso_date(text) {
        return Some(format!("{y:04}-{m:02}-{d:02}"));
    }
    let mut parts = text.split('.');
    let (d, m, y) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some()
        || y.len() != 4
        || !(1..=2).contains(&d.len())
        || !(1..=2).contains(&m.len())
    {
        return None;
    }
    let iso = format!("{y}-{m:0>2}-{d:0>2}");
    iso_date(&iso).map(|_| iso)
}

/// A time of day as a value is written, `14:30` (24 hours, two digits each), as (hour, minute); none otherwise.
pub fn iso_time(text: &str) -> Option<(u32, u32)> {
    let (h, m) = text.split_once(':')?;
    if h.len() != 2 || m.len() != 2 || !h.bytes().chain(m.bytes()).all(|b| b.is_ascii_digit()) {
        return None;
    }
    let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then_some((h, m))
}

/// A time as a user writes it, `14:30`, `14.30` or `9:05` (a one-digit hour too): its `HH:MM` form.
pub fn typed_time(text: &str) -> Option<String> {
    let text = text.trim();
    let (h, m) = text.split_once(':').or_else(|| text.split_once('.'))?;
    if !(1..=2).contains(&h.len()) || m.len() != 2 {
        return None;
    }
    let iso = format!("{h:0>2}:{m}");
    iso_time(&iso).map(|_| iso)
}

/// A day of the calendar: (year, month, day).
pub type Ymd = (u32, u32, u32);

/// A time of day: (hour, minute).
pub type Hm = (u32, u32);

/// A local date and time as a value is written, `2026-11-15T14:30`: its date and time; none otherwise.
pub fn iso_date_time(text: &str) -> Option<(Ymd, Hm)> {
    let (date, time) = text.split_once('T')?;
    Some((iso_date(date)?, iso_time(time)?))
}

/// A date and time as a user writes them, `15.11.2026 14:30`, `2026-11-15 14:30` or `2026-11-15T14:30` (the date's
/// and the time's own spellings): the value's form, `2026-11-15T14:30`.
pub fn typed_date_time(text: &str) -> Option<String> {
    let text = text.trim();
    let (date, time) = text
        .split_once('T')
        .or_else(|| text.split_once(char::is_whitespace))?;
    Some(format!(
        "{}T{}",
        typed_date(date)?,
        typed_time(time.trim())?
    ))
}

/// A component's id: lowercase letters, digits, `_` and `-`, a letter first, at most 63: the key its value is given to
/// the command under (`ctx.params["sorgu"]`).
fn is_component_id(id: &str) -> bool {
    let b = id.as_bytes();
    !b.is_empty()
        && b.len() <= 63
        && b[0].is_ascii_lowercase()
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, b'_' | b'-'))
}

/// A key's text, when it is there and not blank.
fn text<'a>(c: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    c.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
}

/// What is wrong with `value`, a panel read as JSON (`file` its path in the package; `commands` the extension's
/// commands, `entrypoints.commands`' ids), each problem with its JSON pointer.
pub fn check(file: &str, value: &Value, commands: &[&str]) -> Vec<Problem> {
    let mut out = Vec::new();
    let mut add = |at: String, message: String| {
        out.push(
            Problem::new("ui_panel", message)
                .file(file.to_owned())
                .at(at),
        );
    };
    let Some(top) = value.as_object() else {
        add(
            String::new(),
            "Panel bir JSON nesnesi ({…}) olmalı.".to_owned(),
        );
        return out;
    };
    for key in top
        .keys()
        .filter(|k| !["format", "schemaVersion", "title", "body"].contains(&k.as_str()))
    {
        add(
            format!("/{key}"),
            format!(
                "“{key}” panelde bilinmeyen bir alan; olanlar: format, schemaVersion, title, body."
            ),
        );
    }
    if top.get("format").and_then(Value::as_str) != Some(FORMAT) {
        add("/format".to_owned(), format!("Biçim “{FORMAT}” olmalı."));
    }
    match top.get("schemaVersion").and_then(Value::as_u64) {
        Some(VERSION) => {}
        Some(v) if v > VERSION => add(
            "/schemaVersion".to_owned(),
            format!(
                "Panel sürüm {v}; bu KentOS {VERSION}. sürümü çizer: daha yeni bir KentOS gerekir."
            ),
        ),
        _ => add(
            "/schemaVersion".to_owned(),
            format!("schemaVersion {VERSION} olmalı."),
        ),
    }
    match top.get("title").and_then(Value::as_str) {
        Some(t) if !t.trim().is_empty() && t.chars().count() <= 80 => {}
        _ => add(
            "/title".to_owned(),
            "Başlık boş olmayan, en çok 80 karakterlik bir metin olmalı.".to_owned(),
        ),
    }
    let Some(body) = top.get("body").and_then(Value::as_array) else {
        add(
            "/body".to_owned(),
            "body bir bileşen listesi ([…]) olmalı.".to_owned(),
        );
        return out;
    };
    if body.is_empty() || body.len() > MOST_COMPONENTS {
        add(
            "/body".to_owned(),
            format!("Panelde 1 ile {MOST_COMPONENTS} arasında bileşen olmalı."),
        );
    }
    let of_type = |kind: &str| -> BTreeSet<&str> {
        body.iter()
            .filter(|c| c.get("type").and_then(Value::as_str) == Some(kind))
            .filter_map(|c| c.get("id").and_then(Value::as_str))
            .collect()
    };
    // Where a button's rows may go: a table, a tree, cards or figures.
    // What shows a command's rows: a button's `table` names one; a chart's `of`, one of the others.
    let shows: BTreeSet<&str> = ["table", "tree", "card", "stats"]
        .into_iter()
        .flat_map(of_type)
        .collect();
    let mut tables = shows.clone();
    tables.extend(of_type("chart"));
    let layer_pickers = of_type("layerPicker");
    // What the components' rules read: the inputs by id, typed (rules.rs).
    let inputs = rules::inputs_of(body);
    let mut ids = BTreeSet::new();
    // The calculations: each one's place, id and what it reads, ordered after the loop (a round is refused).
    let mut calculated: Vec<(usize, Option<String>, Vec<String>)> = Vec::new();
    for (i, component) in body.iter().enumerate() {
        let at = format!("/body/{i}");
        let Some(c) = component.as_object() else {
            add(at, "Bileşen bir nesne ({…}) olmalı.".to_owned());
            continue;
        };
        let kind = c.get("type").and_then(Value::as_str).unwrap_or_default();
        let Some(known) = keys_of(kind) else {
            add(
                format!("{at}/type"),
                format!("Bileşenin türü (type) şunlardan biri olmalı: {TYPES}."),
            );
            continue;
        };
        for key in c.keys().filter(|k| {
            k.as_str() != "type"
                && k.as_str() != tabs::TAB_KEY
                && k.as_str() != rows::ROW_KEY
                && !known.contains(&k.as_str())
                && !takes_rule(kind, known, k.as_str())
                && !values::takes_value(kind, k.as_str())
        }) {
            let mut takes: Vec<&str> = known.to_vec();
            takes.extend(
                rules::RULE_KEYS
                    .iter()
                    .copied()
                    .filter(|r| takes_rule(kind, known, r)),
            );
            takes.extend(
                values::VALUE_KEYS
                    .iter()
                    .copied()
                    .filter(|k| values::takes_value(kind, k)),
            );
            takes.push(tabs::TAB_KEY);
            if rows::takes_row(kind) {
                takes.push(rows::ROW_KEY);
            }
            add(
                format!("{at}/{key}"),
                format!(
                    "“{key}” {kind} bileşeninde bilinmeyen bir alan; olanlar: {}.",
                    takes.join(", ")
                ),
            );
        }
        // Its rules compile against the panel's inputs.
        for key in rules::RULE_KEYS
            .iter()
            .filter(|r| takes_rule(kind, known, r))
        {
            match c.get(*key) {
                None => {}
                Some(Value::String(source)) if !source.trim().is_empty() => {
                    if let Err(why) = rules::compile(source, &inputs) {
                        add(format!("{at}/{key}"), why);
                    }
                }
                Some(_) => add(
                    format!("{at}/{key}"),
                    format!("{key} bir ifade (metin) olmalı, ör. \"tur = 'Okul'\"."),
                ),
            }
        }
        // Its worked-out values compile against the inputs and the moment; a calculation is not typed, set or asked.
        for key in values::VALUE_KEYS
            .iter()
            .filter(|k| values::takes_value(kind, k))
        {
            match c.get(*key) {
                None => {}
                Some(Value::String(source)) if !source.trim().is_empty() => {
                    match values::compile(source, &inputs, &values::Moment::checking()) {
                        Ok(expr) if *key == "calculate" => calculated.push((
                            i,
                            c.get("id").and_then(Value::as_str).map(str::to_owned),
                            expr.fields.clone(),
                        )),
                        Ok(_) => {}
                        Err(why) => add(format!("{at}/{key}"), why),
                    }
                }
                Some(_) => add(
                    format!("{at}/{key}"),
                    format!("{key} bir ifade (metin) olmalı, ör. \"ilk + adet - 1\"."),
                ),
            }
        }
        if c.contains_key("calculate") {
            for (other, why) in [
                ("value", "değerini hesabı yazar"),
                ("defaultExpr", "değerini hesabı yazar"),
                ("required", "kullanıcıya sorulmaz"),
                ("requiredWhen", "kullanıcıya sorulmaz"),
                ("enabledWhen", "yazılmaz, hep kapalıdır"),
                ("onChange", "kullanıcı değiştirmez, onChange çalışmaz"),
            ] {
                let given = match c.get(other) {
                    None | Some(Value::Null) | Some(Value::Bool(false)) => false,
                    Some(_) => true,
                };
                if given {
                    add(
                        format!("{at}/{other}"),
                        format!(
                            "calculate ile “{other}” birlikte olmaz: hesaplanan bileşen {why}."
                        ),
                    );
                }
            }
        } else if c.contains_key("defaultExpr") && c.get("value").is_some_and(|v| !v.is_null()) {
            add(
                format!("{at}/value"),
                "defaultExpr ile “value” birlikte olmaz: ilk değeri biri verir.".to_owned(),
            );
        }
        match c.get("id") {
            None if kind == "text" || kind == "section" => {}
            None => add(
                at.clone(),
                format!("{kind} bileşeni bir kimlik (id) ister."),
            ),
            Some(id) => match id.as_str() {
                Some(id) if !is_component_id(id) => add(
                    format!("{at}/id"),
                    format!(
                        "“{id}” bir bileşen kimliği değil: küçük harf, rakam, _ ve -, harfle başlar (ör. sorgu)."
                    ),
                ),
                Some(id) if !ids.insert(id.to_owned()) => add(
                    format!("{at}/id"),
                    format!("“{id}” kimlikli iki bileşen var."),
                ),
                Some(_) => {}
                None => add(format!("{at}/id"), "Kimlik metin olmalı.".to_owned()),
            },
        }
        if kind == "text" {
            if text(c, "text").is_none() {
                add(
                    at.clone(),
                    "text bileşeni boş olmayan bir metin (text) ister.".to_owned(),
                );
            }
            continue;
        }
        if c.get("required").is_some_and(|v| !v.is_boolean()) {
            add(
                format!("{at}/required"),
                "required true ya da false olmalı.".to_owned(),
            );
        }
        if c.get("collapsed").is_some_and(|v| !v.is_boolean()) {
            add(
                format!("{at}/collapsed"),
                "collapsed true ya da false olmalı: bölüm panel açılırken kapalı mı.".to_owned(),
            );
        }
        if !matches!(
            kind,
            "table" | "tree" | "card" | "stats" | "chart" | "image"
        ) && text(c, "label").is_none()
        {
            add(
                at.clone(),
                format!("{kind} bileşeni boş olmayan bir ad (label) ister."),
            );
        }
        // A value changed runs a command, as a button does: the command must be the extension's.
        if let Some(command) = c.get("onChange")
            && !command.as_str().is_some_and(|c| commands.contains(&c))
        {
            add(
                format!("{at}/onChange"),
                "onChange eklentinin bir komutu olmalı (manifestin entrypoints.commands listesinde): değer \
                 değişince panelin değerleriyle o çalışır."
                    .to_owned(),
            );
        }
        match kind {
            "numberField" => {
                let number = |key: &str| c.get(key).map(Value::as_f64);
                for key in ["value", "min", "max"] {
                    if let Some(None) = number(key) {
                        add(format!("{at}/{key}"), format!("“{key}” bir sayı olmalı."));
                    }
                }
                if let (Some(Some(min)), Some(Some(max))) = (number("min"), number("max"))
                    && min > max
                {
                    add(
                        format!("{at}/min"),
                        "En küçük değer (min) en büyükten (max) büyük.".to_owned(),
                    );
                }
            }
            "select" | "multiSelect" => {
                let options = c.get("options").and_then(Value::as_array);
                let mut values = BTreeSet::new();
                match options {
                    Some(options) if !options.is_empty() => {
                        for (j, option) in options.iter().enumerate() {
                            let oat = format!("{at}/options/{j}");
                            match (
                                option
                                    .get("value")
                                    .and_then(Value::as_str)
                                    .filter(|v| !v.is_empty()),
                                option
                                    .get("label")
                                    .and_then(Value::as_str)
                                    .filter(|l| !l.trim().is_empty()),
                            ) {
                                (Some(v), Some(_)) => {
                                    if !values.insert(v.to_owned()) {
                                        add(
                                            format!("{oat}/value"),
                                            format!("“{v}” seçeneği iki kez."),
                                        );
                                    }
                                }
                                _ => add(
                                    oat,
                                    "Seçenek boş olmayan bir değer (value) ve ad (label) ister."
                                        .to_owned(),
                                ),
                            }
                        }
                    }
                    _ => add(
                        format!("{at}/options"),
                        "Seçim en az bir seçenek (options) ister.".to_owned(),
                    ),
                }
                match (kind, c.get("value")) {
                    (_, None) => {}
                    ("select", Some(value)) => {
                        if !value.as_str().is_some_and(|v| values.contains(v)) {
                            add(
                                format!("{at}/value"),
                                "Başlangıç değeri seçeneklerden biri olmalı.".to_owned(),
                            );
                        }
                    }
                    (_, Some(value)) => {
                        let chosen: Vec<&str> = value
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(Value::as_str)
                            .collect();
                        let fits = value.as_array().is_some_and(|v| v.len() == chosen.len())
                            && chosen.iter().all(|v| values.contains(*v))
                            && chosen.iter().collect::<BTreeSet<_>>().len() == chosen.len();
                        if !fits {
                            add(
                                format!("{at}/value"),
                                "Başlangıçta seçilenler (value) seçeneklerin değerlerinden bir liste olmalı, her biri bir \
                                 kez."
                                    .to_owned(),
                            );
                        }
                    }
                }
            }
            "layerPicker" | "featurePicker" => {
                if let Some(geometry) = c.get("geometry")
                    && !geometry
                        .as_str()
                        .is_some_and(|g| ["point", "line", "area", "annotation"].contains(&g))
                {
                    add(
                        format!("{at}/geometry"),
                        "Katman türü point, line, area ya da annotation olmalı.".to_owned(),
                    );
                }
                if kind == "featurePicker" {
                    if let Some(layer) = c.get("layer")
                        && !layer.as_str().is_some_and(|l| layer_pickers.contains(l))
                    {
                        add(
                            format!("{at}/layer"),
                            "Nesne seçicinin katmanı (layer) bu paneldeki bir katman seçicinin (layerPicker) \
                             kimliği olmalı: nesneler orada seçilen katmandan alınır."
                                .to_owned(),
                        );
                    }
                    if c.get("multiple").is_some_and(|v| !v.is_boolean()) {
                        add(
                            format!("{at}/multiple"),
                            "multiple true ya da false olmalı.".to_owned(),
                        );
                    }
                }
            }
            "fieldPicker" => {
                match c.get("layer") {
                    None => add(
                        at.clone(),
                        "Alan seçici, alanlarını listeleyeceği katman seçicinin kimliğini (layer) ister."
                            .to_owned(),
                    ),
                    Some(layer) if !layer.as_str().is_some_and(|l| layer_pickers.contains(l)) => add(
                        format!("{at}/layer"),
                        "Alan seçicinin katmanı (layer) bu paneldeki bir katman seçicinin (layerPicker) kimliği \
                         olmalı: alanlar orada seçilen katmanınkilerdir."
                            .to_owned(),
                    ),
                    Some(_) => {}
                }
                match c.get("kinds").map(Value::as_array) {
                    None => {}
                    Some(Some(kinds)) => {
                        for (j, k) in kinds.iter().enumerate() {
                            if !k.as_str().is_some_and(|k| FIELD_KINDS.contains(&k)) {
                                add(
                                    format!("{at}/kinds/{j}"),
                                    format!(
                                        "Alan türü şunlardan biri olmalı: {}.",
                                        FIELD_KINDS.join(", ")
                                    ),
                                );
                            }
                        }
                    }
                    Some(None) => add(
                        format!("{at}/kinds"),
                        "kinds alan türlerinin listesi ([…]) olmalı.".to_owned(),
                    ),
                }
            }
            "dateField" => {
                let date = |key: &str| c.get(key).map(|v| v.as_str().and_then(iso_date));
                for key in ["value", "min", "max"] {
                    if let Some(None) = date(key) {
                        add(
                            format!("{at}/{key}"),
                            format!(
                                "“{key}” bir tarih olmalı, YYYY-AA-GG biçiminde (ör. 2026-11-15)."
                            ),
                        );
                    }
                }
                if let (Some(Some(min)), Some(Some(max))) = (date("min"), date("max"))
                    && min > max
                {
                    add(
                        format!("{at}/min"),
                        "En erken tarih (min) en geç tarihten (max) sonra.".to_owned(),
                    );
                }
            }
            "timeField" | "dateTimeField" => {
                let (valid, words): (fn(&str) -> bool, &str) = if kind == "timeField" {
                    (
                        |t| iso_time(t).is_some(),
                        "bir saat olmalı, SS:DD biçiminde (ör. 14:30)",
                    )
                } else {
                    (
                        |t| iso_date_time(t).is_some(),
                        "bir tarih ve saat olmalı, YYYY-AA-GGTSS:DD biçiminde (ör. 2026-11-15T14:30)",
                    )
                };
                let given = |key: &str| c.get(key).map(|v| v.as_str().filter(|t| valid(t)));
                for key in ["value", "min", "max"] {
                    if let Some(None) = given(key) {
                        add(format!("{at}/{key}"), format!("“{key}” {words}."));
                    }
                }
                // Both written alike, so text order is time order.
                if let (Some(Some(min)), Some(Some(max))) = (given("min"), given("max"))
                    && min > max
                {
                    add(
                        format!("{at}/min"),
                        "En erken değer (min) en geç değerden (max) sonra.".to_owned(),
                    );
                }
            }
            "checkbox" => {
                if c.get("value").is_some_and(|v| !v.is_boolean()) {
                    add(
                        format!("{at}/value"),
                        "Onay kutusunun değeri true ya da false olmalı.".to_owned(),
                    );
                }
            }
            "button" => {
                match c.get("command").and_then(Value::as_str) {
                    Some(command) if commands.contains(&command) => {}
                    Some(command) => add(
                        format!("{at}/command"),
                        format!(
                            "“{command}” eklentinin komutlarında yok: manifestin entrypoints.commands listesine ekleyin \
                             (düğme bir komut çalıştırır)."
                        ),
                    ),
                    None => add(
                        at.clone(),
                        "Düğme çalıştıracağı komutu (command) ister.".to_owned(),
                    ),
                }
                if let Some(table) = c.get("table")
                    && !table.as_str().is_some_and(|t| tables.contains(t))
                {
                    add(
                        format!("{at}/table"),
                        "Düğmenin tablosu (table) bu paneldeki bir tablonun, ağacın, kartın, göstergenin ya da grafiğin \
                         kimliği olmalı."
                            .to_owned(),
                    );
                }
                if c.get("primary").is_some_and(|v| !v.is_boolean()) {
                    add(
                        format!("{at}/primary"),
                        "primary true ya da false olmalı.".to_owned(),
                    );
                }
            }
            "tree" => {
                let given = c.get("group").and_then(Value::as_array);
                let names: Vec<&str> = given
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .filter(|s| !s.trim().is_empty())
                    .collect();
                let fits = given.is_some_and(|g| g.len() == names.len())
                    && (1..=3).contains(&names.len())
                    && names.iter().collect::<BTreeSet<_>>().len() == names.len();
                if !fits {
                    add(
                        format!("{at}/group"),
                        "Ağaç, satırlarını gruplayacağı 1–3 farklı sütun adı (group) ister: ör. [\"ada\"]."
                            .to_owned(),
                    );
                }
            }
            "stats" => {
                for key in ["title", "value", "unit", "note"] {
                    if c.get(key).is_some() && text(c, key).is_none() {
                        add(
                            format!("{at}/{key}"),
                            format!(
                                "“{key}” gösterge kartında okunacak bir sütunun adı olmalı (boş olmayan metin)."
                            ),
                        );
                    }
                }
            }
            // Its kinds, and where a file is to be written (files.rs, docs/adr/8057 §41, §44).
            "filePicker" => files::check(c, &at, &mut add),
            // Its value a system of KentOS's list (crs.rs, docs/adr/8057 §45).
            "crsPicker" => crs::check(c, &at, &mut add),
            // Its value a point: two finite numbers (docs/adr/8057 §57).
            "pointPicker" => {
                if let Some(value) = c.get("value").filter(|v| !v.is_null())
                    && point_value(value).is_none()
                {
                    add(
                        format!("{at}/value"),
                        "value bir nokta olmalı: iki sayılık liste [x, y] (çizimin düzleminde, metre).".to_owned(),
                    );
                }
            }
            "image" => {
                if let Some(height) = c.get("height")
                    && !height.as_f64().is_some_and(|h| (80.0..=600.0).contains(&h))
                {
                    add(
                        format!("{at}/height"),
                        "Görselin yüksekliği (height) 80 ile 600 piksel arasında bir sayı olmalı."
                            .to_owned(),
                    );
                }
                if c.get("alt").is_some() && text(c, "alt").is_none() {
                    add(
                        format!("{at}/alt"),
                        "alt görselin ne gösterdiğini söyleyen boş olmayan bir metin olmalı."
                            .to_owned(),
                    );
                }
            }
            "chart" => {
                let kind = match c.get("kind") {
                    None => "bar",
                    Some(v) => match v.as_str().filter(|k| CHART_KINDS.contains(k)) {
                        Some(k) => k,
                        None => {
                            add(
                                format!("{at}/kind"),
                                format!(
                                    "Grafiğin türü şunlardan biri olmalı: {}.",
                                    CHART_KINDS.join(", ")
                                ),
                            );
                            "bar"
                        }
                    },
                };
                for key in ["category", "value"] {
                    if c.get(key).is_some() && text(c, key).is_none() {
                        add(
                            format!("{at}/{key}"),
                            format!(
                                "“{key}” grafikte okunacak bir sütunun adı olmalı (boş olmayan metin)."
                            ),
                        );
                    }
                }
                let aggregate = c
                    .get("aggregate")
                    .map(|v| v.as_str().filter(|a| CHART_AGGREGATES.contains(a)));
                if let Some(None) = aggregate {
                    add(
                        format!("{at}/aggregate"),
                        format!(
                            "Toplama şunlardan biri olmalı: {}.",
                            CHART_AGGREGATES.join(", ")
                        ),
                    );
                }
                // What each kind reads: a category for bars and a pie, numbers for a histogram, both for a line.
                let needs: &[&str] = match kind {
                    "histogram" => &["value"],
                    "line" => &["category", "value"],
                    _ => &["category"],
                };
                for key in needs {
                    if c.get(*key).is_none() {
                        add(
                            at.clone(),
                            format!(
                                "“{kind}” grafiği “{key}” ister: {}.",
                                if *key == "category" {
                                    "gruplanacak ya da yatay eksene konacak sütun"
                                } else {
                                    "sayıları okunacak sütun"
                                }
                            ),
                        );
                    }
                }
                if matches!(kind, "bar" | "pie")
                    && aggregate.flatten().is_some_and(|a| a != "count")
                    && c.get("value").is_none()
                {
                    add(
                        format!("{at}/aggregate"),
                        "Sayımdan başka bir toplama (sum, mean …) bir değer sütunu (value) ister."
                            .to_owned(),
                    );
                }
                if let Some(of) = c.get("of")
                    && !of.as_str().is_some_and(|o| shows.contains(o))
                {
                    add(
                        format!("{at}/of"),
                        "Grafiğin kaynağı (of) bu paneldeki bir tablonun, ağacın, kartın ya da göstergenin kimliği \
                         olmalı."
                            .to_owned(),
                    );
                }
            }
            "card" => {
                for key in ["title", "tag"] {
                    if c.get(key).is_some() && text(c, key).is_none() {
                        add(
                            format!("{at}/{key}"),
                            format!(
                                "“{key}” kartta gösterilecek bir sütunun adı olmalı (boş olmayan metin)."
                            ),
                        );
                    }
                }
                if let (Some(title), Some(tag)) = (text(c, "title"), text(c, "tag"))
                    && title == tag
                {
                    add(
                        format!("{at}/tag"),
                        "Kartın etiketi (tag) başlığıyla (title) aynı sütun: başka bir sütun seçin.".to_owned(),
                    );
                }
            }
            "textField" => {
                for key in ["placeholder", "value"] {
                    if c.get(key).is_some_and(|v| !v.is_string()) {
                        add(format!("{at}/{key}"), format!("“{key}” metin olmalı."));
                    }
                }
            }
            _ => {}
        }
    }
    // The calculations in an order each after those it reads: a round of them has none.
    let reads: Vec<(Option<String>, Vec<String>)> = calculated
        .iter()
        .map(|(_, id, fields)| (id.clone(), fields.clone()))
        .collect();
    if let Err((at, why)) = values::calc_order(&reads) {
        add(format!("/body/{}/calculate", calculated[at].0), why);
    }
    // Its tabs: each a tab's words, at most `tabs::MOST_TABS` (docs/adr/8057 §43).
    for (at, why) in tabs::check(body) {
        add(at, why);
    }
    // Its rows: each a row's name, its cells next to each other, small, on one tab (docs/adr/8057 §55).
    for (at, why) in rows::check(body) {
        add(at, why);
    }
    out
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn panel() -> Value {
        json!({
            "format": "kentos.ui.panel", "schemaVersion": 1, "title": "Yapı ara",
            "body": [
                { "type": "text", "text": "Numarası yazılan yapıları listeler." },
                { "type": "textField", "id": "sorgu", "label": "Yapı no", "placeholder": "Y-01" },
                { "type": "numberField", "id": "en-az", "label": "En az alan", "min": 0, "unit": "m²" },
                { "type": "select", "id": "durum", "label": "Durum", "value": "hepsi",
                  "options": [{ "value": "hepsi", "label": "Hepsi" }, { "value": "numarasiz", "label": "Numarasız" }] },
                { "type": "checkbox", "id": "secili", "label": "Yalnız seçililer" },
                { "type": "layerPicker", "id": "katman", "label": "Katman", "geometry": "area" },
                { "type": "button", "id": "ara", "label": "Ara", "command": "yapi-ara", "table": "sonuc", "primary": true },
                { "type": "table", "id": "sonuc", "label": "Sonuçlar" },
                { "type": "fieldPicker", "id": "alan", "label": "Alan", "layer": "katman", "kinds": ["integer", "decimal"],
                  "onChange": "yapi-ara" },
                { "type": "featurePicker", "id": "nesneler", "label": "Nesneler", "layer": "katman", "geometry": "area" },
                { "type": "tree", "id": "agac", "label": "Adalara göre", "group": ["ada"] },
                { "type": "card", "id": "kartlar", "label": "Yapılar", "title": "no", "tag": "durum" },
                { "type": "dateField", "id": "tarih", "label": "Ruhsat tarihi", "min": "2020-01-01",
                  "onChange": "yapi-ara" },
                { "type": "multiSelect", "id": "katlar", "label": "Katlar", "value": ["1"],
                  "options": [{ "value": "1", "label": "Bir" }, { "value": "2", "label": "İki" }] },
                { "type": "timeField", "id": "saat", "label": "Saat", "value": "09:00", "min": "08:00", "max": "17:30",
                  "onChange": "yapi-ara" },
                { "type": "dateTimeField", "id": "gecerli", "label": "Geçerlilik sonu",
                  "min": "2026-01-01T00:00" },
                { "type": "chart", "id": "grafik", "label": "Adalara göre", "kind": "bar", "of": "agac",
                  "category": "ada" }
            ]
        })
    }

    /// A point is two numbers: typed with a decimal point and a comma or a space between, or decimal commas with a
    /// semicolon; given as JSON `[x, y]`; shown to the millimetre.
    #[test]
    fn a_point_is_typed_as_two_numbers() {
        assert_eq!(
            typed_point("500123.45, 4401234.5"),
            Some([500123.45, 4401234.5])
        );
        assert_eq!(typed_point(" 10 20 "), Some([10.0, 20.0]));
        assert_eq!(typed_point("10,5; 20,25"), Some([10.5, 20.25]));
        assert_eq!(typed_point("10.5,20"), Some([10.5, 20.0]));
        for wrong in ["", "10", "a, b", "1, 2, 3", "10,5 20,5"] {
            assert_eq!(typed_point(wrong), None, "{wrong:?}");
        }
        assert_eq!(point_value(&serde_json::json!([1.0, 2])), Some([1.0, 2.0]));
        assert_eq!(point_value(&serde_json::json!([1.0])), None);
        assert_eq!(point_words([500123.456_78, 2.0]), "500123.457, 2.000");
    }

    #[test]
    fn a_panel_passes_and_reads_as_kentos_draws_it() {
        let value = panel();
        assert!(check("ui/ara.panel.json", &value, &["yapi-ara"]).is_empty());
        let read = read(&value).expect("read");
        assert_eq!(read.title, "Yapı ara");
        assert_eq!(read.body.len(), 17);
        assert_eq!(read.body.iter().filter(|c| c.is_input()).count(), 11);
        assert!(
            matches!(&read.body[16], Component::Chart { kind: Some(ChartKind::Bar), of: Some(o), category: Some(c),
                                                        aggregate: None, .. } if o == "agac" && c == "ada")
        );
        assert!(
            matches!(&read.body[14], Component::TimeField { value: Some(v), max: Some(m), .. }
            if v == "09:00" && m == "17:30")
        );
        assert_eq!(read.body[14].on_change(), Some("yapi-ara"));
        assert!(
            matches!(&read.body[15], Component::DateTimeField { min: Some(m), .. } if m == "2026-01-01T00:00")
        );
        assert!(matches!(&read.body[13], Component::MultiSelect { value, .. } if value == &["1"]));
        assert_eq!(read.body[12].on_change(), Some("yapi-ara"));
        assert!(
            matches!(&read.body[5], Component::LayerPicker { geometry: Some(g), .. } if g == "area")
        );
        assert!(
            matches!(&read.body[6], Component::Button { command, table: Some(t), .. }
            if command == "yapi-ara" && t == "sonuc")
        );
        assert!(
            matches!(&read.body[8], Component::FieldPicker { layer, kinds, .. }
            if layer == "katman" && kinds == &["integer", "decimal"])
        );
        assert!(
            matches!(&read.body[9], Component::FeaturePicker { layer: Some(l), multiple: true, .. } if l == "katman"),
            "it takes many objects unless it says otherwise"
        );
        assert_eq!(read.body[8].on_change(), Some("yapi-ara"));
        assert_eq!(read.body[9].on_change(), None);
        assert!(matches!(&read.body[10], Component::Tree { group, .. } if group == &["ada"]));
        assert!(
            matches!(&read.body[11], Component::Card { title: Some(t), tag: Some(g), .. }
            if t == "no" && g == "durum")
        );
        // A button's rows may go to a card; a card needs no title or tag.
        let mut to_card = panel();
        to_card["body"][6]["table"] = json!("kartlar");
        to_card["body"][11] = json!({ "type": "card", "id": "kartlar" });
        assert!(check("ui/ara.panel.json", &to_card, &["yapi-ara"]).is_empty());
        // And to figures: their columns named or the first two.
        let mut to_stats = panel();
        to_stats["body"][6]["table"] = json!("ozet");
        to_stats["body"][11] =
            json!({ "type": "stats", "id": "ozet", "label": "Özet", "unit": "birim" });
        assert!(check("ui/ara.panel.json", &to_stats, &["yapi-ara"]).is_empty());
        to_stats["body"][11]["value"] = json!(" ");
        assert_eq!(
            check("ui/ara.panel.json", &to_stats, &["yapi-ara"])[0]
                .at
                .as_deref(),
            Some("/body/11/value")
        );
    }

    /// ADR 8057 §25, the coverage report of the panel's components, names every type Kapı 1 knows: a new type is
    /// written there with what it covers.
    #[test]
    fn every_component_type_is_in_the_coverage_report() {
        let adr = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../docs/adr/8057-phase-3-extension-packages.md"),
        )
        .expect("ADR 8057");
        let report = adr.split("\n## §25.").nth(1).expect("its §25");
        let report = report.split("\n## §").next().unwrap_or(report);
        for kind in component_types() {
            assert!(keys_of(kind).is_some(), "{kind}");
            assert!(
                report.contains(&format!("`{kind}`")),
                "ADR 8057 §25 does not name `{kind}`: add it to the coverage report"
            );
        }
    }

    #[test]
    fn a_components_rules_are_checked_where_they_are_written() {
        // docs/adr/8057 §33: every component may be shown by a rule, an input that can be asked for asked by one, a
        // button or a typed field usable by one; a rule compiles against the panel's inputs; read, the definition
        // carries them in the body's order.
        let mut p = panel();
        p["body"][0]["visibleWhen"] = json!("secili");
        p["body"][1]["requiredWhen"] = json!("durum = 'hepsi'");
        p["body"][6]["enabledWhen"] = json!("[en-az] > 0 or sorgu <> ''");
        assert!(
            check("ui/ara.panel.json", &p, &["yapi-ara"]).is_empty(),
            "{:?}",
            check("ui/ara.panel.json", &p, &["yapi-ara"])
        );
        let read = read(&p).expect("read");
        assert_eq!(read.rules.len(), read.body.len());
        assert_eq!(read.rules[0].visible_when.as_deref(), Some("secili"));
        assert_eq!(
            read.rules[6].enabled_when.as_deref(),
            Some("[en-az] > 0 or sorgu <> ''")
        );
        assert!(read.rules[2].is_empty());
        // A rule a component does not take, one that names no input, one that is not text.
        p["body"][7]["requiredWhen"] = json!("secili");
        p["body"][3]["enabledWhen"] = json!("secili");
        p["body"][5]["visibleWhen"] = json!("mahalle = 'Merkez'");
        p["body"][4]["visibleWhen"] = json!(true);
        let said: Vec<(String, String)> = check("ui/ara.panel.json", &p, &["yapi-ara"])
            .into_iter()
            .map(|pr| (pr.at.unwrap_or_default(), pr.message))
            .collect();
        let at: Vec<&str> = said.iter().map(|(a, _)| a.as_str()).collect();
        assert_eq!(
            at,
            [
                "/body/3/enabledWhen",
                "/body/4/visibleWhen",
                "/body/5/visibleWhen",
                "/body/7/requiredWhen"
            ],
            "{said:#?}"
        );
        assert!(
            said[0].1.contains("bilinmeyen bir alan") && said[0].1.contains("visibleWhen"),
            "{}",
            said[0].1
        );
        assert!(
            said[2]
                .1
                .starts_with("“mahalle” paneldeki bir girdinin kimliği değil"),
            "{}",
            said[2].1
        );
    }

    #[test]
    fn each_mistake_is_said_where_it_is() {
        let mut p = panel();
        p["body"][1]["id"] = json!("Sorgu");
        p["body"][2]["max"] = json!(-1);
        p["body"][3]["value"] = json!("yok");
        p["body"][5]["geometry"] = json!("polygon");
        p["body"][6]["command"] = json!("ara");
        p["body"][6]["table"] = json!("liste");
        p["body"][8]["layer"] = json!("sonuc");
        p["body"][8]["kinds"] = json!(["sayi"]);
        p["body"][9]["multiple"] = json!("evet");
        p["body"][9]["onChange"] = json!("say");
        p["body"][10]["group"] = json!(["ada", "ada"]);
        p["body"][11]["tag"] = json!("no");
        p["body"][12]["max"] = json!("2019-02-30");
        p["body"][13]["value"] = json!(["1", "1"]);
        p["body"][14]["value"] = json!("25:00");
        p["body"][15]["max"] = json!("2025-12-31T23:59");
        p["body"][1]["required"] = json!("evet");
        p["body"][16]["kind"] = json!("radar");
        p["body"][16]["aggregate"] = json!("sum");
        p["body"][16]["of"] = json!("sorgu");
        p["body"]
            .as_array_mut()
            .unwrap()
            .push(json!({ "type": "slider", "id": "x" }));
        p["body"]
            .as_array_mut()
            .unwrap()
            .push(json!({ "type": "fieldPicker", "id": "y", "label": "Y" }));
        p["renk"] = json!("mavi");
        let at: Vec<String> = check("ui/ara.panel.json", &p, &["yapi-ara"])
            .into_iter()
            .filter_map(|p| p.at)
            .collect();
        assert_eq!(
            at,
            [
                "/renk",
                "/body/1/id",
                "/body/1/required",
                "/body/2/min",
                "/body/3/value",
                "/body/5/geometry",
                "/body/6/command",
                "/body/6/table",
                "/body/8/layer",
                "/body/8/kinds/0",
                "/body/9/onChange",
                "/body/9/multiple",
                "/body/10/group",
                "/body/11/tag",
                "/body/12/max",
                "/body/13/value",
                "/body/14/value",
                "/body/15/min",
                "/body/16/kind",
                "/body/16/aggregate",
                "/body/16/of",
                "/body/17/type",
                "/body/18"
            ]
        );
    }
}

#[cfg(test)]
mod chart_tests {
    use serde_json::json;

    use super::*;

    /// A chart reads what its kind draws: a category for bars and a pie, numbers for a histogram, both for a line; a
    /// button may send its rows to it, and its `of` names another component's rows, not a chart's.
    #[test]
    fn a_chart_asks_for_the_columns_its_kind_draws() {
        let panel = |chart: serde_json::Value| {
            json!({ "format": "kentos.ui.panel", "schemaVersion": 1, "title": "Grafik", "body": [
                { "type": "button", "id": "say", "label": "Say", "command": "say", "table": "grafik" },
                { "type": "table", "id": "liste" },
                chart
            ] })
        };
        let problems = |chart: serde_json::Value| -> Vec<String> {
            check("ui/g.panel.json", &panel(chart), &["say"])
                .into_iter()
                .map(|p| p.message)
                .collect()
        };
        let bars = problems(json!({ "type": "chart", "id": "grafik", "category": "Değer" }));
        assert!(
            bars.is_empty(),
            "bars counting a category, the button's rows: {bars:?}"
        );
        assert!(
            problems(
                json!({ "type": "chart", "id": "grafik", "kind": "pie", "category": "mahalle",
                             "value": "alan", "aggregate": "sum", "of": "liste" })
            )
            .is_empty()
        );
        assert!(
            problems(json!({ "type": "chart", "id": "grafik", "kind": "histogram" }))[0]
                .contains("“value” ister")
        );
        let line =
            problems(json!({ "type": "chart", "id": "grafik", "kind": "line", "value": "nufus" }));
        assert_eq!(line.len(), 1, "{line:?}");
        assert!(line[0].contains("“category” ister"));
        assert!(
            problems(json!({ "type": "chart", "id": "grafik", "category": "x", "of": "grafik" }))
                [0]
            .contains("kaynağı (of)"),
            "not a chart's rows, not its own"
        );
    }
}

#[cfg(test)]
mod date_tests {
    use super::*;

    #[test]
    fn a_date_is_a_real_day_written_either_way() {
        assert_eq!(iso_date("2024-02-29"), Some((2024, 2, 29)));
        assert_eq!(iso_date("2023-02-29"), None, "not a leap year");
        assert_eq!(iso_date("2026-13-01"), None);
        assert_eq!(iso_date("2026-1-01"), None, "ISO writes two digits");
        assert_eq!(typed_date("15.11.2026").as_deref(), Some("2026-11-15"));
        assert_eq!(typed_date(" 5.1.2026 ").as_deref(), Some("2026-01-05"));
        assert_eq!(typed_date("2026-11-15").as_deref(), Some("2026-11-15"));
        assert_eq!(typed_date("31.04.2026"), None, "April has thirty days");
        assert_eq!(typed_date("15/11/2026"), None);
    }

    #[test]
    fn a_time_and_a_moment_are_written_either_way() {
        assert_eq!(iso_time("14:30"), Some((14, 30)));
        assert_eq!(iso_time("24:00"), None);
        assert_eq!(iso_time("9:05"), None, "a value writes two digits");
        assert_eq!(typed_time("9:05").as_deref(), Some("09:05"));
        assert_eq!(typed_time(" 14.30 ").as_deref(), Some("14:30"));
        assert_eq!(typed_time("14:3"), None);
        assert_eq!(typed_time("1430"), None);
        assert_eq!(
            iso_date_time("2026-11-15T14:30"),
            Some(((2026, 11, 15), (14, 30)))
        );
        assert_eq!(iso_date_time("2026-11-15 14:30"), None, "a value writes T");
        for typed in [
            "15.11.2026 14:30",
            "2026-11-15 14:30",
            "2026-11-15T14:30",
            "15.11.2026  9.05",
        ] {
            assert!(typed_date_time(typed).is_some(), "{typed}");
        }
        assert_eq!(
            typed_date_time("15.11.2026 9.05").as_deref(),
            Some("2026-11-15T09:05")
        );
        assert_eq!(typed_date_time("15.11.2026"), None, "a time too");
        assert_eq!(typed_date_time("31.04.2026 10:00"), None);
    }
}

#[cfg(test)]
mod section_tests {
    use serde_json::json;

    use super::*;

    fn problems(body: Value) -> Vec<(String, String)> {
        let panel = json!({ "format": FORMAT, "schemaVersion": VERSION, "title": "Bölümler", "body": body });
        check("ui/b.panel.json", &panel, &[])
            .into_iter()
            .map(|p| (p.at.unwrap_or_default(), p.message))
            .collect()
    }

    /// docs/adr/8057 §38: a section is a heading with a label, its id optional, folded at first when `collapsed`, shown
    /// by a rule as any component; without a label, with a `collapsed` that is not yes or no, it is said where.
    #[test]
    fn a_section_is_a_labelled_heading_folded_when_asked() {
        let body = json!([
            { "type": "textField", "id": "ad", "label": "Ad" },
            { "type": "section", "label": "Gelişmiş", "collapsed": true },
            { "type": "numberField", "id": "adet", "label": "Adet" },
            { "type": "section", "id": "yardim", "label": "Yardım", "visibleWhen": "adet > 0" },
            { "type": "text", "text": "Bir yardım satırı." }
        ]);
        assert!(problems(body.clone()).is_empty(), "{:?}", problems(body.clone()));
        let panel = read(&json!({ "format": FORMAT, "schemaVersion": VERSION, "title": "B", "body": body })).unwrap();
        assert_eq!(
            panel.body[1],
            Component::Section {
                id: None,
                label: "Gelişmiş".into(),
                collapsed: true
            }
        );
        assert_eq!(panel.body[3].id(), Some("yardim"));
        assert!(!panel.body[1].is_input());
        let wrong = problems(json!([
            { "type": "section", "label": " " },
            { "type": "section", "label": "Bir", "collapsed": "evet" }
        ]));
        assert_eq!(
            wrong,
            [
                ("/body/0".to_owned(), "section bileşeni boş olmayan bir ad (label) ister.".to_owned()),
                (
                    "/body/1/collapsed".to_owned(),
                    "collapsed true ya da false olmalı: bölüm panel açılırken kapalı mı.".to_owned()
                )
            ]
        );
    }
}

#[cfg(test)]
mod image_tests {
    use serde_json::json;

    use super::*;

    /// docs/adr/8057 §40: an image takes an id, and optionally its label, `alt` and `height` (80–600 px); a height out
    /// of bounds and an empty `alt` are said where they are written.
    #[test]
    fn an_image_has_its_alt_and_height_in_bounds() {
        let panel = |image: Value| json!({ "format": FORMAT, "schemaVersion": VERSION, "title": "Görsel", "body": [image] });
        let fits = panel(
            json!({ "type": "image", "id": "grafik", "label": "Alanlar", "alt": "Çubuklar", "height": 140 }),
        );
        assert!(check("ui/g.panel.json", &fits, &[]).is_empty());
        assert_eq!(
            read(&fits).unwrap().body[0],
            Component::Image {
                id: "grafik".into(),
                label: Some("Alanlar".into()),
                alt: Some("Çubuklar".into()),
                height: Some(140.0)
            }
        );
        let wrong = check(
            "ui/g.panel.json",
            &panel(json!({ "type": "image", "id": "grafik", "alt": " ", "height": 900 })),
            &[],
        );
        let mut at: Vec<&str> = wrong.iter().filter_map(|p| p.at.as_deref()).collect();
        at.sort_unstable();
        assert_eq!(at, ["/body/0/alt", "/body/0/height"], "{wrong:?}");
    }
}
