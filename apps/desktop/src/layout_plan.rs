//! The workbench's layout as the web keeps it (the web's app/layoutPlan.ts
//! and the ribbon's parts of app/ribbon.ts; docs/adr/0115): which panels are
//! open, their sizes, the dock's and the ribbon's states. What is kept, its
//! defaults, how a stored value is read (a value of the wrong type or out
//! of its domain is not taken; a field no longer kept, the web's toolbox's,
//! is dropped: docs/adr/0155), and the sizes as shown: a kept size is the
//! user's wish, shown within what the window allows.
//! fixtures/shell/v1/layout.json holds every answer; both platforms play it
//! (`layout_plan_tests.rs`). The web keeps it in localStorage `kentos.ui.v1`,
//! the desktop in its own file with the same rules (layout.rs).

use serde_json::{Map, Value, json};

/// Where the web keeps the layout.
#[cfg(test)]
pub(crate) const LAYOUT_KEY: &str = "kentos.ui.v1";

/// Every field is written together this long (ms) after the last change: a
/// drag is written once, not at every step.
pub(crate) const SAVE_MS: u64 = 250;

/// The layout's fields and their defaults, in the web's order.
pub(crate) fn defaults() -> Map<String, Value> {
    let Value::Object(map) = json!({
        "theme": "dark",
        "rightVisible": true,
        "dockWidth": 312,
        "layersFraction": 0.5,
        "bottomExpanded": false,
        "bottomHeight": 190,
        "bottomTab": "history",
        "dockTab": "layers",
        "processingTab": "tools",
        "processingFolded": [],
        "ribbonTab": "home",
        "ribbonCollapsed": false,
        "ribbonQuickAccess": [],
        "ribbonSplits": {},
        "overview": false,
        "magnifier": false,
        "magnifierZoom": 4,
    }) else {
        unreachable!("an object")
    };
    map
}

/// A size's limits: the least, the most, the most as a share of the
/// window, and what a double click on its edge puts back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Limits {
    pub min: f64,
    pub max: Option<f64>,
    pub max_share: Option<f64>,
    pub reset: f64,
}

/// The right dock's width (px): dragged between `min` and the lesser of
/// `max` and half the window's width.
pub(crate) const DOCK_WIDTH: Limits = Limits {
    min: 240.0,
    max: Some(560.0),
    max_share: Some(0.5),
    reset: 312.0,
};

/// The layer tree's share of the dock's height.
pub(crate) const LAYERS_FRACTION: Limits = Limits {
    min: 0.15,
    max: Some(0.85),
    max_share: None,
    reset: 0.5,
};

/// The bottom panel's height (px): at least `min`, at most 0.6 of the window's height.
pub(crate) const BOTTOM_HEIGHT: Limits = Limits {
    min: 96.0,
    max: None,
    max_share: Some(0.6),
    reset: 190.0,
};

/// What a field's stored value must be to be taken.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Rule {
    /// A text of the list.
    Enum(&'static [&'static str]),
    Boolean,
    /// A finite number, brought within its limits when it is outside them.
    Number {
        min: Option<f64>,
        max: Option<f64>,
    },
    Text,
    /// A list: its texts are kept, anything else in it dropped.
    Texts,
    /// An object: its entries with a text value are kept.
    TextMap,
}

/// Every field's rule, in the web's order.
pub(crate) const FIELDS: [(&str, Rule); 17] = [
    ("theme", Rule::Enum(&["dark", "light"])),
    ("rightVisible", Rule::Boolean),
    (
        "dockWidth",
        Rule::Number {
            min: Some(240.0),
            max: Some(560.0),
        },
    ),
    (
        "layersFraction",
        Rule::Number {
            min: Some(0.15),
            max: Some(0.85),
        },
    ),
    ("bottomExpanded", Rule::Boolean),
    (
        "bottomHeight",
        Rule::Number {
            min: Some(96.0),
            max: None,
        },
    ),
    (
        "bottomTab",
        Rule::Enum(&[
            "history",
            "coords",
            "points",
            "table",
            "search",
            "topology",
            "serviceInfo",
            "messages",
        ]),
    ),
    (
        "dockTab",
        Rule::Enum(&["layers", "processing", "blocks", "templates", "sources"]),
    ),
    ("processingTab", Rule::Enum(&["tools", "history"])),
    ("processingFolded", Rule::Texts),
    ("ribbonTab", Rule::Text),
    ("ribbonCollapsed", Rule::Boolean),
    ("ribbonQuickAccess", Rule::Texts),
    ("ribbonSplits", Rule::TextMap),
    // Genel bakış and Büyüteç over the drawing, and the magnifier's zoom (docs/adr/0181).
    ("overview", Rule::Boolean),
    ("magnifier", Rule::Boolean),
    (
        "magnifierZoom",
        Rule::Number {
            min: Some(2.0),
            max: Some(16.0),
        },
    ),
];

/// A stored value read by its field's rule; `None` when it cannot be taken.
fn read_field(rule: Rule, v: &Value) -> Option<Value> {
    match rule {
        Rule::Enum(values) => v.as_str().filter(|s| values.contains(s)).map(Value::from),
        Rule::Boolean => v.as_bool().map(Value::from),
        Rule::Number { min, max } => {
            let n = v.as_f64().filter(|n| n.is_finite())?;
            let n = n
                .max(min.unwrap_or(f64::NEG_INFINITY))
                .min(max.unwrap_or(f64::INFINITY));
            Some(number(n))
        }
        Rule::Text => v.as_str().map(Value::from),
        Rule::Texts => v
            .as_array()
            .map(|list| Value::Array(list.iter().filter(|x| x.is_string()).cloned().collect())),
        Rule::TextMap => v.as_object().map(|map| {
            Value::Object(
                map.iter()
                    .filter(|(_, x)| x.is_string())
                    .map(|(k, x)| (k.clone(), x.clone()))
                    .collect(),
            )
        }),
    }
}

/// A number as JSON writes it: a whole one without a fraction (as the web's).
pub(crate) fn number(n: f64) -> Value {
    if n.fract() == 0.0 && n.abs() < 9.007_199_254_740_992e15 {
        Value::from(n as i64)
    } else {
        Value::from(n)
    }
}

/// The layout from what was stored (`None`: nothing): every field the store
/// holds and its rule takes; the default for the rest, and for everything
/// when the text is not a JSON object. Fields the layout does not know are
/// dropped: the web's toolbox's among them (docs/adr/0155).
pub(crate) fn read_layout(text: Option<&str>) -> Map<String, Value> {
    let stored = text.and_then(|t| serde_json::from_str::<Value>(&finite_numbers(t)).ok());
    let mut out = defaults();
    let Some(Value::Object(saved)) = stored else {
        return out;
    };
    for (key, rule) in FIELDS {
        if let Some(v) = saved.get(key).and_then(|v| read_field(rule, v)) {
            out.insert(key.to_owned(), v);
        }
    }
    out
}

/// The text with every number too large for a double written as `null`:
/// JavaScript reads it as an infinity, which no field takes, as it takes no
/// `null`; serde_json would refuse the whole text instead.
fn finite_numbers(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.char_indices().peekable();
    let mut in_string = false;
    let mut escaped = false;
    while let Some((i, c)) = chars.next() {
        if in_string {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        if c == '"' {
            in_string = true;
            out.push(c);
            continue;
        }
        if c == '-' || c.is_ascii_digit() {
            let mut end = i + c.len_utf8();
            while let Some(&(j, d)) = chars.peek() {
                if d.is_ascii_digit() || matches!(d, '.' | 'e' | 'E' | '+' | '-') {
                    end = j + d.len_utf8();
                    chars.next();
                } else {
                    break;
                }
            }
            let token = &text[i..end];
            match token.parse::<f64>() {
                Ok(n) if n.is_infinite() => out.push_str("null"),
                _ => out.push_str(token),
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// JavaScript's `Math.round`: halves go up.
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// The dock's width as shown: the kept width within the limits the window
/// allows now (the upper one wins in a very narrow window).
pub(crate) fn dock_width_on(kept: f64, window_width: f64) -> f64 {
    let upper = DOCK_WIDTH
        .max
        .unwrap_or(f64::INFINITY)
        .min(window_width * DOCK_WIDTH.max_share.unwrap_or(1.0));
    js_round(kept.max(DOCK_WIDTH.min).min(upper))
}

/// The bottom panel's height as shown: the kept height within the limits
/// the window allows now.
pub(crate) fn bottom_height_on(kept: f64, window_height: f64) -> f64 {
    js_round(
        kept.max(BOTTOM_HEIGHT.min)
            .min(window_height * BOTTOM_HEIGHT.max_share.unwrap_or(1.0)),
    )
}

/// The layer tree's share after its edge is dragged `dy` px down from
/// `start` in a dock `height` px tall.
// The dock's own sash gives the share, clamped in layout.rs; the web's rule is played here.
#[cfg(test)]
pub(crate) fn dragged_layers_fraction(start: f64, dy: f64, height: f64) -> f64 {
    let max = LAYERS_FRACTION.max.unwrap_or(1.0);
    (start + dy / height).max(LAYERS_FRACTION.min).min(max)
}

/// Always on the quick access bar; the user may add more (and remove what they added).
pub(crate) const QUICK_ACCESS: [&str; 3] = ["file.save", "edit.undo", "edit.redo"];

/// The quick access bar from the kept list (`ribbonQuickAccess`): the fixed
/// commands, then the ones the user added that this app has, once each, in
/// the order added.
pub(crate) fn quick_access_of(kept: &[String], exists: impl Fn(&str) -> bool) -> Vec<String> {
    let mut bar: Vec<String> = QUICK_ACCESS.iter().map(|s| (*s).to_owned()).collect();
    for id in kept {
        if exists(id) && !bar.contains(id) {
            bar.push(id.clone());
        }
    }
    bar
}

/// The ribbon's tab at start: the kept one (`ribbonTab`) while the work
/// mode shows it and it is not contextual; else Giriş.
pub(crate) fn start_tab<'a>(kept: &'a str, tabs: &[(&str, bool)]) -> &'a str {
    if tabs
        .iter()
        .any(|(id, contextual)| *id == kept && !contextual)
    {
        kept
    } else {
        "home"
    }
}

/// A split choice as the layout keeps it (`ribbonSplits`, by the button's
/// key): its command and its method's option.
pub(crate) fn split_choice_key(command: &str, option: Option<&str>) -> String {
    format!("{command}|{}", option.unwrap_or(""))
}

/// A split button's entry on top: the one last chosen, or its first when
/// none was or the kept one is gone.
pub(crate) fn split_current(entries: &[(&str, Option<&str>)], kept: Option<&str>) -> usize {
    entries
        .iter()
        .position(|(command, option)| Some(split_choice_key(command, *option).as_str()) == kept)
        .unwrap_or(0)
}
