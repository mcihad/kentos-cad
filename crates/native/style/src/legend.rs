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
    /// The rows of one Orantılı sembol share a scale (CSS px per paper mm):
    /// their sizes compare (docs/adr/0213 §2.2).
    #[serde(rename = "pxPerMm", skip_serializing_if = "Option::is_none")]
    pub px_per_mm: Option<f64>,
}

fn row(label: impl Into<String>, symbol: Option<Value>) -> LegendEntry {
    LegendEntry {
        label: label.into(),
        symbol,
        px_per_mm: None,
    }
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
                return vec![row(label, Some(symbol.clone()))];
            }
            found
                .into_iter()
                .map(|(c, symbol)| row(format!("{label} ({})", class_name(c)), Some(symbol)))
                .collect()
        };
        // A set's rows with every symbol changed (a thematic renderer's colour or size).
        let changed = |set: Option<&SymbolSet>,
                       label: &str,
                       f: &dyn Fn(&Value) -> Value|
         -> Vec<LegendEntry> {
            set_entries(set, label)
                .into_iter()
                .map(|e| LegendEntry {
                    symbol: e.symbol.as_ref().map(f),
                    ..e
                })
                .collect()
        };
        let mut entries = Vec::new();
        let renderer = layer.style.renderer.as_ref().map(Renderer::from_value);
        rows_of(
            renderer,
            layer,
            &present,
            &resolve,
            &set_entries,
            &changed,
            &mut entries,
        );
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
                entries.push(row(
                    src.item_name(id).unwrap_or_else(|| id.to_owned()),
                    Some(symbol),
                ));
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

/// A set's rows with its symbols changed (a ramp's colour, a size).
type ChangedRows<'a> =
    dyn Fn(Option<&SymbolSet>, &str, &dyn Fn(&Value) -> Value) -> Vec<LegendEntry> + 'a;

/// A renderer's rows (a cluster's and a displacement's single points' renderer in turn).
fn rows_of(
    renderer: Option<Result<Renderer, String>>,
    layer: &LegendLayer<'_>,
    present: &crate::classify::Present,
    resolve: &dyn Fn(Option<&Value>) -> Option<Value>,
    set_entries: &dyn Fn(Option<&SymbolSet>, &str) -> Vec<LegendEntry>,
    changed: &ChangedRows<'_>,
    entries: &mut Vec<LegendEntry>,
) {
    use crate::thematic::{legend_number, ramp_at, range_label, with_color, with_size};
    use kentos_style_core::style::thematic::{share, size_at, step};
    match renderer {
        None => {
            let simple = SymbolSet::from_value(&symbols_of_layer_style(
                layer.style,
                &layer.style.color,
                layer.style.line_weight,
                false,
            ));
            entries.extend(set_entries(Some(&simple), layer.name));
        }
        Some(Ok(Renderer::Single(r))) => entries.extend(set_entries(Some(&r.symbols), layer.name)),
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
            walk(&r.rules, "", entries, &set_entries);
        }
        Some(Ok(Renderer::Unclassed(r))) => {
            for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
                let c = ramp_at(&r.ramp, f64::from(step(t)) / 255.0);
                let c = c.get(..7).unwrap_or(&c).to_owned();
                entries.extend(changed(
                    Some(&r.symbols),
                    &legend_number(r.min + (r.max - r.min) * t),
                    &|s| with_color(s, &c),
                ));
            }
            if let Some(o) = &r.other {
                entries.extend(set_entries(Some(o), "Değeri olmayanlar"));
            }
        }
        Some(Ok(Renderer::Proportional(r))) => {
            let unit = r.unit.as_deref().unwrap_or("mm");
            let e = match r.scaling.as_deref() {
                Some("radius") => 1.0,
                Some("flannery") => 0.57,
                _ => 0.5,
            };
            // The largest as four fifths of a row's picture (24 px high); paper mm only.
            let shared = (unit == "mm" && r.max_size > 0.0).then(|| 0.8 * 24.0 / r.max_size);
            for v in [r.min_value, (r.min_value + r.max_value) / 2.0, r.max_value] {
                let size = size_at(
                    f64::from(step(share(v, r.min_value, r.max_value))) / 255.0,
                    r.min_size,
                    r.max_size,
                    e,
                );
                // Areas and points by the marker symbol, lines by the line symbol.
                let mut shown = Vec::new();
                if present.line > 0
                    && let Some(s) = resolve(r.symbols.line.as_ref())
                {
                    shown.push((GeometryClass::Line, s));
                }
                if (present.fill > 0 || present.marker > 0)
                    && let Some(s) = resolve(r.symbols.marker.as_ref())
                {
                    shown.push((GeometryClass::Marker, s));
                }
                let many = shown.len() > 1;
                for (c, s) in shown {
                    let label = if many {
                        format!("{} ({})", legend_number(v), class_name(c))
                    } else {
                        legend_number(v)
                    };
                    entries.push(LegendEntry {
                        label,
                        symbol: Some(with_size(&s, size, unit)),
                        px_per_mm: if c == GeometryClass::Marker {
                            shared
                        } else {
                            None
                        },
                    });
                }
            }
            if let Some(o) = &r.other {
                entries.extend(set_entries(Some(o), "Değeri olmayanlar"));
            }
        }
        Some(Ok(Renderer::Bivariate(r))) => {
            let n = r.breaks_x.len() + 1;
            for j in 0..n {
                for i in 0..n {
                    let Some(c) = r.colors.get(j * n + i) else {
                        continue;
                    };
                    let c = c.get(..7).unwrap_or(c).to_owned();
                    let label = format!(
                        "{} {}, {} {}",
                        r.expr_x,
                        range_label(&r.breaks_x, i),
                        r.expr_y,
                        range_label(&r.breaks_y, j)
                    );
                    entries.extend(changed(Some(&r.symbols), &label, &|s| with_color(s, &c)));
                }
            }
            if let Some(o) = &r.other {
                entries.extend(set_entries(Some(o), "Değeri olmayanlar"));
            }
        }
        Some(Ok(Renderer::DotDensity(r))) => {
            entries.push(row(
                format!("1 nokta = {}", legend_number(r.dot_value)),
                None,
            ));
            for f in &r.fields {
                let symbol = serde_json::json!({ "type": "marker", "layers": [{ "id": "d", "type": "shape", "shape": "circle",
                        "size": r.dot_size.unwrap_or(1.0), "unit": r.unit.as_deref().unwrap_or("mm"), "fill": f.color }] });
                entries.push(row(
                    f.label
                        .clone()
                        .filter(|l| !l.is_empty())
                        .unwrap_or_else(|| f.expr.clone()),
                    Some(symbol),
                ));
            }
        }
        Some(Ok(Renderer::Chart(r))) => {
            for f in &r.fields {
                let symbol = serde_json::json!({ "type": "fill", "layers": [{ "id": "f", "type": "simpleFill", "color": f.color }] });
                entries.push(row(
                    f.label
                        .clone()
                        .filter(|l| !l.is_empty())
                        .unwrap_or_else(|| f.expr.clone()),
                    Some(symbol),
                ));
            }
        }
        Some(Ok(Renderer::Heatmap(r))) => {
            for (label, t) in [("Az", 0.25), ("Orta", 0.5), ("Çok", 1.0)] {
                let symbol = serde_json::json!({ "type": "fill", "layers": [{ "id": "f", "type": "simpleFill", "color": ramp_at(&r.ramp, t) }] });
                entries.push(row(label, Some(symbol)));
            }
        }
        Some(Ok(Renderer::Cluster(r))) => {
            let symbol = resolve(r.symbol.as_ref()).unwrap_or_else(|| {
                    serde_json::json!({ "type": "marker", "layers": [{ "id": "k", "type": "shape", "shape": "circle", "size": 24,
                        "unit": "px", "fill": layer.style.color, "stroke": "#FFFFFF", "strokeWidth": 1.5 }] })
                });
            entries.push(row("Küme", Some(symbol)));
            rows_of(
                r.renderer.as_ref().map(Renderer::from_value),
                layer,
                present,
                resolve,
                set_entries,
                changed,
                entries,
            );
        }
        Some(Ok(Renderer::Displacement(r))) => {
            rows_of(
                r.renderer.as_ref().map(Renderer::from_value),
                layer,
                present,
                resolve,
                set_entries,
                changed,
                entries,
            );
        }
        Some(Ok(Renderer::Inverted(r))) => {
            entries.push(row("Dışı", resolve(r.symbols.fill.as_ref())));
        }
        // A renderer this version cannot read shows no rows of its own.
        Some(Err(_)) => {}
    }
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
