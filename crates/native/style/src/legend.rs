//! The legend of a drawing (plan açıklamaları; the web's `style/legend.ts`):
//! for each layer, what its symbols mean, and the picture Lejant saves.
//!
//! - [`legend_of`]: a layer without a renderer shows its own look; single,
//!   categorized, graduated and rule-based renderers one row per class, only
//!   for the geometry the layer has (named “(alan)”, “(çizgi)”, “(nokta)”
//!   when there are several); objects with a symbol of their own add that
//!   symbol once, under its library name.
//! - [`legend_layers`]: the layers it reads, the top of the list first.
//! - [`legend_layout`]: where everything of the picture goes, in logical
//!   pixels, drawn at [`LegendLayout::scale`] on white paper.
//!
//! Both platforms are held to `fixtures/style/v1/legend.json`.

use kentos_contracts::{Entity, LayerStyle};
use serde::Serialize;
use serde_json::Value;

use crate::classify::classes_present;
use crate::renderer::{GeometryClass, Renderer, Rule, SymbolSet, ref_id};
use crate::simple::symbols_of_layer_style;

/// A row of a layer's legend: what it says and the symbol drawn beside it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LegendEntry {
    pub label: String,
    pub symbol: Option<Value>,
}

/// A layer's rows.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegendGroup {
    pub layer_id: String,
    pub layer_name: String,
    pub entries: Vec<LegendEntry>,
}

/// A layer the legend reads.
#[derive(Clone, Copy, Debug)]
pub struct LegendLayer<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub style: &'a LayerStyle,
}

/// What the legend reads besides the layers: their objects and the library.
pub trait LegendSources {
    fn entities(&self, layer: &str) -> Vec<&Entity>;
    /// A library symbol by id.
    fn symbol(&self, id: &str) -> Option<Value>;
    /// A library item's name by id.
    fn item_name(&self, id: &str) -> Option<String>;
}

const ORDER: [GeometryClass; 3] = [
    GeometryClass::Fill,
    GeometryClass::Line,
    GeometryClass::Marker,
];

fn class_name(class: GeometryClass) -> &'static str {
    match class {
        GeometryClass::Fill => "alan",
        GeometryClass::Line => "çizgi",
        GeometryClass::Marker => "nokta",
    }
}

/// The legend's rows, layer by layer (`legendOf`); a layer without rows is left out.
pub fn legend_of(layers: &[LegendLayer<'_>], src: &impl LegendSources) -> Vec<LegendGroup> {
    let mut out = Vec::new();
    for layer in layers {
        let entities = src.entities(layer.id);
        let present = classes_present(&entities);
        let classes: Vec<GeometryClass> =
            ORDER.into_iter().filter(|c| present.of(*c) > 0).collect();
        if classes.is_empty() {
            continue;
        }
        let resolve = |symbol: Option<&Value>| -> Option<Value> {
            let symbol = symbol?;
            match ref_id(symbol) {
                Some(id) => src.symbol(id),
                None => Some(symbol.clone()),
            }
        };
        // The symbols of a set for the layer's geometry, one row each when there are several.
        let set_entries = |set: Option<&SymbolSet>, label: &str| -> Vec<LegendEntry> {
            let found: Vec<(GeometryClass, Value)> = classes
                .iter()
                .filter_map(|c| resolve(set.and_then(|s| s.get(*c))).map(|s| (*c, s)))
                .collect();
            if let [(_, symbol)] = found.as_slice() {
                return vec![LegendEntry {
                    label: label.to_owned(),
                    symbol: Some(symbol.clone()),
                }];
            }
            found
                .into_iter()
                .map(|(c, symbol)| LegendEntry {
                    label: format!("{label} ({})", class_name(c)),
                    symbol: Some(symbol),
                })
                .collect()
        };
        let mut entries = Vec::new();
        match layer.style.renderer.as_ref().map(Renderer::from_value) {
            None => {
                let simple = SymbolSet::from_value(&symbols_of_layer_style(
                    layer.style,
                    &layer.style.color,
                    layer.style.line_weight,
                    false,
                ));
                entries.extend(set_entries(Some(&simple), layer.name));
            }
            Some(Ok(Renderer::Single(r))) => {
                entries.extend(set_entries(Some(&r.symbols), layer.name))
            }
            Some(Ok(Renderer::Categorized(r))) => {
                for k in r.categories.iter().filter(|k| k.enabled != Some(false)) {
                    let label = if k.label.is_empty() {
                        &k.value
                    } else {
                        &k.label
                    };
                    entries.extend(set_entries(Some(&k.symbols), label));
                }
                if let Some(other) = &r.other {
                    entries.extend(set_entries(Some(other), "Diğer değerler"));
                }
            }
            Some(Ok(Renderer::Graduated(r))) => {
                for k in &r.classes {
                    entries.extend(set_entries(Some(&k.symbols), &k.label));
                }
            }
            Some(Ok(Renderer::Rules(r))) => {
                fn walk(
                    rules: &[Rule],
                    prefix: &str,
                    out: &mut Vec<LegendEntry>,
                    set_entries: &dyn Fn(Option<&SymbolSet>, &str) -> Vec<LegendEntry>,
                ) {
                    for rule in rules.iter().filter(|r| r.enabled()) {
                        let label = if prefix.is_empty() {
                            rule.label.clone()
                        } else {
                            format!("{prefix} › {}", rule.label)
                        };
                        out.extend(set_entries(rule.symbols.as_ref(), &label));
                        walk(rule.children(), &label, out, set_entries);
                    }
                }
                walk(&r.rules, "", &mut entries, &set_entries);
            }
            // A renderer this version cannot read shows no rows of its own.
            Some(Err(_)) => {}
        }
        // Objects drawn with their own symbol: each symbol once, under its library name.
        let mut own: Vec<&str> = Vec::new();
        for e in &entities {
            if let Some(id) = e.base().symbol.as_deref()
                && !own.contains(&id)
            {
                own.push(id);
            }
        }
        for id in own {
            if let Some(symbol) = src.symbol(id) {
                entries.push(LegendEntry {
                    label: src.item_name(id).unwrap_or_else(|| id.to_owned()),
                    symbol: Some(symbol),
                });
            }
        }
        if !entries.is_empty() {
            out.push(LegendGroup {
                layer_id: layer.id.to_owned(),
                layer_name: layer.name.to_owned(),
                entries,
            });
        }
    }
    out
}

/// The layers a legend reads: the top of the list first, only the visible ones when asked (`legendLayers`).
pub fn legend_layers<'a>(
    leaves: &[(LegendLayer<'a>, bool)],
    visible_only: bool,
) -> Vec<LegendLayer<'a>> {
    leaves
        .iter()
        .filter(|(_, visible)| !visible_only || *visible)
        .map(|(l, _)| *l)
        .collect()
}

// ── The window and its picture ─────────────────────────────────────────

/// What the legend window says (`LEGEND_TEXTS`).
pub mod texts {
    pub const TITLE: &str = "Lejant";
    pub const SAVE: &str = "PNG olarak kaydet";
    pub const CLOSE: &str = "Kapat";
    pub const VISIBLE_ONLY: &str = "Yalnızca görünen katmanlar";
    pub const HEADINGS: &str = "Katman adlarını başlık yaz";
    pub const NOTHING: &str = "Lejanta girecek çizilmiş nesne yok.";
    pub const NO_ROWS: &str = "Lejantta satır yok.";
    pub const SAVED: &str = "Lejant PNG olarak kaydedildi (beyaz kâğıt, 2× çözünürlük).";
    pub const FILE: &str = "lejant.png";
    pub const HEADING: &str = "LEJANT";

    /// The footer's count.
    pub fn rows(n: usize) -> String {
        format!("{n} satır")
    }
}

/// The picture's colours over the screen's: white paper, black ink, whatever the theme (`LEGEND_PAPER`).
pub struct LegendPaper {
    pub background: [f32; 4],
    pub ink: &'static str,
    pub paper: &'static str,
    pub fg: &'static str,
    pub fg_dim: &'static str,
}

pub const PAPER: LegendPaper = LegendPaper {
    background: [1.0, 1.0, 1.0, 1.0],
    ink: "#000000",
    paper: "#FFFFFF",
    fg: "#111111",
    fg_dim: "#555555",
};

const FONT: &str = "Arial, \"Liberation Sans\", sans-serif";

/// A line of text on the picture; `align` right means `x` is where it ends.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LegendText {
    pub text: String,
    pub font: String,
    pub color: String,
    pub x: i64,
    pub y: i64,
    pub align: &'static str,
}

/// Where an entry's symbol is drawn: on white, framed.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegendPicture {
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
    pub frame: String,
    pub frame_width: f64,
}

/// A row of the picture: a layer's heading or an entry.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LegendRow {
    pub kind: &'static str,
    pub label: LegendText,
    pub picture: Option<LegendPicture>,
}

/// The legend picture in logical pixels (`legendLayout`), drawn at `scale`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LegendLayout {
    pub scale: i64,
    pub width: i64,
    pub height: i64,
    pub background: String,
    pub heading: LegendText,
    pub name: LegendText,
    pub rows: Vec<LegendRow>,
}

/// A row's height and where the first one starts.
pub const ROW: i64 = 30;
pub const TOP: i64 = 56;
/// The margin under the last row.
pub const BOTTOM: i64 = 16;

fn text(
    text: &str,
    weight: u16,
    size: u16,
    color: &str,
    (x, y): (i64, i64),
    align: &'static str,
) -> LegendText {
    LegendText {
        text: text.to_owned(),
        font: format!("{weight} {size}px {FONT}"),
        color: color.to_owned(),
        x,
        y,
        align,
    }
}

/// Where everything of the picture goes: the groups shown, with or without layer headings.
pub fn legend_layout(groups: &[LegendGroup], headings: bool, drawing_name: &str) -> LegendLayout {
    const W: i64 = 520;
    let count: usize = groups
        .iter()
        .map(|g| g.entries.len() + usize::from(headings))
        .sum();
    let mut rows = Vec::with_capacity(count);
    let mut y = TOP;
    for group in groups {
        if headings {
            rows.push(LegendRow {
                kind: "heading",
                label: text(&group.layer_name, 700, 12, "#000000", (20, y + 19), "left"),
                picture: None,
            });
            y += ROW;
        }
        for e in &group.entries {
            rows.push(LegendRow {
                kind: "entry",
                label: text(&e.label, 400, 12, "#000000", (90, y + 19), "left"),
                picture: e.symbol.as_ref().map(|_| LegendPicture {
                    x: 20,
                    y: y + 3,
                    w: 56,
                    h: 24,
                    frame: "#BBBBBB".to_owned(),
                    frame_width: 0.5,
                }),
            });
            y += ROW;
        }
    }
    LegendLayout {
        scale: 2,
        width: W,
        height: TOP + i64::try_from(count).unwrap_or(i64::MAX / ROW) * ROW + BOTTOM,
        background: "#FFFFFF".to_owned(),
        heading: text(texts::HEADING, 700, 18, "#000000", (20, 34), "left"),
        name: text(drawing_name, 400, 11, "#555555", (W - 20, 34), "right"),
        rows,
    }
}
