//! Katman stili's forms of the thematic renderers on the desktop (docs/adr/0213 §4; the web's
//! `ui/style/thematicPanels.ts`): Sürekli renk, Orantılı sembol, İki değişkenli renk, Nokta
//! yoğunluğu, Grafik, Isı haritası, Kümeleme, Yayma and Ters alan. Each kind keeps its draft in
//! the window; a number field keeps its typed text and takes the value when it reads and is in
//! range (as the classes' bounds do); a cluster's and a displacement's single points draw with
//! another kind's draft (Tek noktalar).

use iced::widget::{Column, Row, button, column, container, row, slider, space};
use iced::{Center, Element, Length};
use kentos_native_style::classify::{
    Method, QUALITATIVE, RAMPS, equal_count, equal_interval, js_number,
};
use kentos_native_style::renderer::{
    Bivariate, Chart, Cluster, Displacement, DotDensity, Field as ValueField, GeometryClass,
    Heatmap, Inverted, Proportional, SizeBy, Stroke, SymbolSet, Unclassed,
};
use kentos_native_style::thematic::{
    BIVARIATE_SCHEMES, HEAT_RAMPS, bivariate_colors, js_text, ramp_at,
};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::Segmented;
use kentos_ui::widget::color::{ColorPicker, parse_hex, to_hex};
use kentos_ui::widget::select::{Choice, Select};
use serde_json::{Value, json};

use super::widgets::{Env, check, ev, expression, help, input, problem, slots_in, tool};
use super::{Event, Field, Kind, LayerStyleWindow, SetAt, Source};
use crate::app::Message;

/// The label column's width (the web's `lsty__label--w`).
const LABEL: f32 = 110.0;
/// A number field's width (the web's `lsty__n2`).
const NUMBER: f32 = 78.0;

/// Which renderer a shared control (a ramp, a unit, a value list) belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Of {
    Unclassed,
    Proportional,
    Bivariate,
    Dots,
    Chart,
    Heat,
    Cluster,
    Displacement,
}

/// The panels' number fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Num {
    UnMin,
    UnMax,
    PrMinValue,
    PrMaxValue,
    PrMinSize,
    PrMaxSize,
    DotValue,
    DotSize,
    Seed,
    ChSize,
    ChBarWidth,
    ChMaxValue,
    SbMinValue,
    SbMaxValue,
    SbMinSize,
    SbMaxSize,
    OutlineWidth,
    Radius,
    HeatMax,
    Distance,
    Tolerance,
    Spacing,
    CircleWidth,
}

/// An edit of a thematic panel.
#[derive(Clone, Debug)]
pub enum Edit {
    /// A number field as typed.
    Number(Num, String),
    /// Verilerden al: the range (or the largest value) from the objects' values.
    FromData(Of),
    /// A ramp of the list, by its place.
    Ramp(Of, usize),
    /// Ters: the ramp's colours the other way round.
    Reverse(Of),
    /// Değeri olmayanlar çizilsin.
    Other(Of, bool),
    Unit(Of, &'static str),
    Scaling(&'static str),
    BiCount(usize),
    BiMethod(Method),
    BiClassify,
    BiScheme(usize),
    FieldColor(Of, usize, String),
    FieldLabel(Of, usize, String),
    /// A value's row one up (true) or down.
    FieldMove(Of, usize, bool),
    FieldRemove(Of, usize),
    FieldAdd(Of),
    ChartKind(&'static str),
    SizeBy(bool),
    Outline(bool),
    OutlineColor(String),
    /// En büyük değer: Sabit (true) or Dinamik.
    HeatFixed(bool),
    Quality(u8),
    Opacity(f32),
    Count(bool),
    Grow(bool),
    Placement(&'static str),
    Circle(bool),
    CircleColor(String),
    Merge(bool),
    /// Tek noktalar: the kind a cluster's or a displacement's single points draw with.
    Inner(Kind),
}

fn th(e: Edit) -> Message {
    ev(Event::Thematic(e))
}

/// Where a number field's typed text is kept.
pub(super) fn num_key(n: Num) -> String {
    format!("th/{n:?}")
}

/// The kinds a cluster's or a displacement's single points draw with, as Tek noktalar lists them.
pub const INNER_KINDS: [(Kind, &str); 8] = [
    (Kind::Simple, "Basit görünüş"),
    (Kind::Single, "Tek sembol"),
    (Kind::Categorized, "Kategorili"),
    (Kind::Graduated, "Aralıklı"),
    (Kind::Unclassed, "Sürekli renk"),
    (Kind::Proportional, "Orantılı sembol"),
    (Kind::Bivariate, "İki değişkenli renk"),
    (Kind::Rules, "Kurallar"),
];

/// The thematic renderers' drafts of a window: the layer's, else a start from its look
/// (the web's `LayerStyleDialog` constructor).
pub struct Drafts {
    pub unclassed: Unclassed,
    pub proportional: Proportional,
    pub bivariate: Bivariate,
    pub dot_density: DotDensity,
    pub chart: Chart,
    pub heatmap: Heatmap,
    pub cluster: Cluster,
    pub displacement: Displacement,
    pub inverted: Inverted,
}

/// A marker the thematic renderers start with: a circle in the layer's colour.
fn circle_of(color: &str) -> Value {
    json!({ "type": "marker", "layers": [{ "id": "c", "type": "shape", "shape": "circle", "size": 4,
        "fill": color, "stroke": "#FFFFFF", "strokeWidth": 0.2 }] })
}

fn strings(stops: &[&str]) -> Vec<String> {
    stops.iter().map(|s| (*s).to_owned()).collect()
}

impl Drafts {
    /// The starts: `plain` the layer's look for the classes it has, `field` its most used
    /// attribute, `fields` its attributes in Turkish order.
    pub fn start(color: &str, plain: &SymbolSet, field: &str, fields: &[String]) -> Drafts {
        let expr = if field.is_empty() { "$alan" } else { field }.to_owned();
        let mut proportional = plain.clone();
        if proportional.marker.is_none() {
            proportional.marker = Some(circle_of(color));
        }
        let ramp = kentos_native_style::classify::ramp(kentos_native_style::classify::DEFAULT_RAMP);
        let chart_fields: Vec<&str> = if fields.is_empty() {
            vec![""]
        } else {
            fields.iter().take(3).map(String::as_str).collect()
        };
        Drafts {
            unclassed: Unclassed {
                expr: expr.clone(),
                min: 0.0,
                max: 100.0,
                ramp: strings(ramp.stops),
                symbols: plain.clone(),
                other: None,
                extra: Default::default(),
            },
            proportional: Proportional {
                expr: expr.clone(),
                min_value: 0.0,
                max_value: 100.0,
                min_size: 2.0,
                max_size: 10.0,
                unit: Some("mm".into()),
                scaling: Some("area".into()),
                symbols: proportional,
                other: None,
                extra: Default::default(),
            },
            bivariate: Bivariate {
                expr_x: fields.first().cloned().unwrap_or_default(),
                expr_y: fields.get(1).cloned().unwrap_or_default(),
                breaks_x: vec![1.0, 2.0],
                breaks_y: vec![1.0, 2.0],
                colors: bivariate_colors(&BIVARIATE_SCHEMES[0].2, 3),
                symbols: plain.clone(),
                other: None,
                extra: Default::default(),
            },
            dot_density: DotDensity {
                fields: vec![value_field(field, QUALITATIVE[0])],
                dot_value: 1.0,
                dot_size: Some(1.0),
                unit: Some("mm".into()),
                seed: Some(0.0),
                symbols: plain.line.clone().map(|l| SymbolSet {
                    line: Some(l),
                    ..SymbolSet::default()
                }),
                extra: Default::default(),
            },
            chart: Chart {
                kind: Some("pie".into()),
                fields: chart_fields
                    .iter()
                    .enumerate()
                    .map(|(i, f)| value_field(f, QUALITATIVE[i % QUALITATIVE.len()]))
                    .collect(),
                size: 8.0,
                unit: Some("mm".into()),
                size_by: None,
                max_value: None,
                bar_width: None,
                outline: Some(Stroke {
                    color: "#FFFFFF".into(),
                    width: 0.2,
                }),
                symbols: Some(plain.clone()),
                extra: Default::default(),
            },
            heatmap: Heatmap {
                radius: 20.0,
                unit: Some("px".into()),
                weight: None,
                max: None,
                ramp: strings(HEAT_RAMPS[0].2),
                quality: Some(2.0),
                opacity: Some(1.0),
                extra: Default::default(),
            },
            cluster: Cluster {
                distance: 40.0,
                unit: Some("px".into()),
                symbol: None,
                count: None,
                grow: None,
                renderer: None,
                extra: Default::default(),
            },
            displacement: Displacement {
                tolerance: 4.0,
                unit: Some("px".into()),
                placement: Some("ring".into()),
                spacing: None,
                center: None,
                circle: Some(Stroke {
                    color: "#7D7D7D".into(),
                    width: 1.0,
                }),
                renderer: None,
                extra: Default::default(),
            },
            inverted: Inverted {
                symbols: SymbolSet {
                    fill: Some(json!({ "type": "fill", "layers": [
                        { "id": "f", "type": "simpleFill", "color": "#FFFFFFB3" },
                        { "id": "l", "type": "simpleLine", "color": color, "width": 0.5 },
                    ] })),
                    ..SymbolSet::default()
                },
                merge: None,
                extra: Default::default(),
            },
        }
    }
}

fn value_field(expr: &str, color: &str) -> ValueField {
    ValueField {
        expr: expr.to_owned(),
        label: None,
        color: color.to_owned(),
        extra: Default::default(),
    }
}

/// An axis's breaks from its values (`breaksOf`): equal count (Aralıklı's slices), else equal
/// interval when the values repeat too much; none when they cannot make `n` classes.
fn breaks_of(values: &[f64], n: usize, method: Method) -> Option<Vec<f64>> {
    let pick = |cls: Vec<kentos_native_style::classify::NumericClass>| -> Vec<f64> {
        cls.iter().skip(1).map(|c| c.min).collect()
    };
    let mut b = match method {
        Method::Count => pick(equal_count(values, n)),
        Method::Interval => pick(equal_interval(values, n)),
    };
    if b.len() + 1 != n && method == Method::Count {
        b = pick(equal_interval(values, n));
    }
    (b.len() + 1 == n).then_some(b)
}

/// The smallest and largest of the values given.
fn range_of(values: &[Option<f64>]) -> Option<(f64, f64)> {
    values.iter().flatten().fold(None, |s, &v| match s {
        None => Some((v, v)),
        Some((a, b)) => Some((a.min(v), b.max(v))),
    })
}

impl LayerStyleWindow {
    /// A thematic panel's edit; the drawing is read for Verilerden al and Sınıfla.
    pub(super) fn thematic(&mut self, e: Edit, src: &Source<'_>) {
        match e {
            Edit::Number(n, text) => self.number(n, text),
            Edit::FromData(of) => self.take_from_data(of, src),
            Edit::Ramp(of, i) => {
                let list = ramp_list(of);
                let Some((_, _, stops)) = list.get(i) else {
                    return;
                };
                let mut next = strings(stops);
                if ramp_match(&list, self.ramp_of(of)).is_some_and(|(_, rev)| rev) {
                    next.reverse();
                }
                self.set_ramp(of, next);
            }
            Edit::Reverse(of) => {
                let mut next = self.ramp_of(of).to_vec();
                next.reverse();
                self.set_ramp(of, next);
            }
            Edit::Other(of, on) => {
                let simple = on.then(|| self.simple.clone());
                match of {
                    Of::Unclassed => self.drafts.unclassed.other = simple,
                    Of::Proportional => self.drafts.proportional.other = simple,
                    Of::Bivariate => self.drafts.bivariate.other = simple,
                    _ => {}
                }
            }
            Edit::Unit(of, u) => {
                let u = Some(u.to_owned());
                match of {
                    Of::Proportional => self.drafts.proportional.unit = u,
                    Of::Dots => self.drafts.dot_density.unit = u,
                    Of::Chart => self.drafts.chart.unit = u,
                    Of::Heat => self.drafts.heatmap.unit = u,
                    Of::Cluster => self.drafts.cluster.unit = u,
                    Of::Displacement => self.drafts.displacement.unit = u,
                    _ => {}
                }
            }
            Edit::Scaling(s) => self.drafts.proportional.scaling = Some(s.to_owned()),
            Edit::BiCount(n) => self.classify_bivariate(n, src),
            Edit::BiMethod(m) => {
                self.bivariate_method = m;
                self.classify_bivariate(self.drafts.bivariate.breaks_x.len() + 1, src);
            }
            Edit::BiClassify => {
                self.classify_bivariate(self.drafts.bivariate.breaks_x.len() + 1, src);
            }
            Edit::BiScheme(i) => {
                self.bivariate_scheme = i.min(BIVARIATE_SCHEMES.len() - 1);
                let n = self.drafts.bivariate.breaks_x.len() + 1;
                self.drafts.bivariate.colors =
                    bivariate_colors(&BIVARIATE_SCHEMES[self.bivariate_scheme].2, n);
            }
            Edit::FieldColor(of, i, c) => {
                if let Some(f) = self.value_fields(of).and_then(|l| l.get_mut(i)) {
                    f.color = c;
                }
            }
            Edit::FieldLabel(of, i, text) => {
                if let Some(f) = self.value_fields(of).and_then(|l| l.get_mut(i)) {
                    let t = kentos_processing::text::js_trim(&text).to_owned();
                    f.label = (!t.is_empty()).then_some(text);
                }
            }
            Edit::FieldMove(of, i, up) => {
                if let Some(l) = self.value_fields(of) {
                    match up {
                        true if i > 0 && i < l.len() => l.swap(i, i - 1),
                        false if i + 1 < l.len() => l.swap(i, i + 1),
                        _ => return,
                    }
                }
                self.typed.clear();
            }
            Edit::FieldRemove(of, i) => {
                if let Some(l) = self.value_fields(of)
                    && l.len() > 1
                    && i < l.len()
                {
                    l.remove(i);
                }
                self.typed.clear();
            }
            Edit::FieldAdd(of) => {
                let names: Vec<String> = self.fields(src.doc).into_iter().map(|(n, _)| n).collect();
                if let Some(l) = self.value_fields(of)
                    && l.len() < 12
                {
                    let next = names
                        .iter()
                        .find(|n| !l.iter().any(|f| f.expr == **n))
                        .cloned()
                        .unwrap_or_default();
                    let color = QUALITATIVE[l.len() % QUALITATIVE.len()];
                    l.push(value_field(&next, color));
                }
            }
            Edit::ChartKind(k) => {
                let c = &mut self.drafts.chart;
                c.kind = Some(k.to_owned());
                if k != "pie" && c.max_value.is_none() {
                    let top = self.chart_largest(src).0;
                    self.drafts.chart.max_value = Some(if top > 0.0 { top } else { 1.0 });
                }
            }
            Edit::SizeBy(on) => {
                if on {
                    let (top, lo) = self.chart_largest(src);
                    let size = self.drafts.chart.size;
                    self.drafts.chart.size_by = Some(SizeBy {
                        min_value: lo,
                        max_value: top.max(lo),
                        min_size: size / 2.0,
                        max_size: size,
                    });
                } else {
                    self.drafts.chart.size_by = None;
                }
                self.forget(&[
                    Num::SbMinValue,
                    Num::SbMaxValue,
                    Num::SbMinSize,
                    Num::SbMaxSize,
                ]);
            }
            Edit::Outline(on) => {
                self.drafts.chart.outline = on.then(|| Stroke {
                    color: "#FFFFFF".into(),
                    width: 0.2,
                });
                self.forget(&[Num::OutlineWidth]);
            }
            Edit::OutlineColor(c) => {
                if let Some(o) = &mut self.drafts.chart.outline {
                    o.color = c;
                }
            }
            Edit::HeatFixed(on) => {
                let h = &mut self.drafts.heatmap;
                h.max = if on {
                    Some(h.max.unwrap_or(10.0))
                } else {
                    None
                };
                self.forget(&[Num::HeatMax]);
            }
            Edit::Quality(q) => self.drafts.heatmap.quality = Some(f64::from(q)),
            Edit::Opacity(o) => {
                self.drafts.heatmap.opacity = Some(f64::from((o / 100.0).clamp(0.0, 1.0)));
            }
            Edit::Count(on) => self.drafts.cluster.count = (!on).then_some(false),
            Edit::Grow(on) => self.drafts.cluster.grow = on.then_some(true),
            Edit::Placement(p) => self.drafts.displacement.placement = Some(p.to_owned()),
            Edit::Circle(on) => {
                self.drafts.displacement.circle = on.then(|| Stroke {
                    color: "#7D7D7D".into(),
                    width: 1.0,
                });
                self.forget(&[Num::CircleWidth]);
            }
            Edit::CircleColor(c) => {
                if let Some(s) = &mut self.drafts.displacement.circle {
                    s.color = c;
                }
            }
            Edit::Merge(on) => self.drafts.inverted.merge = on.then_some(true),
            Edit::Inner(k) => self.inner = k,
        }
    }

    fn forget(&mut self, nums: &[Num]) {
        for n in nums {
            self.typed.remove(&num_key(*n));
        }
    }

    fn ramp_of(&self, of: Of) -> &[String] {
        match of {
            Of::Heat => &self.drafts.heatmap.ramp,
            _ => &self.drafts.unclassed.ramp,
        }
    }

    fn set_ramp(&mut self, of: Of, stops: Vec<String>) {
        match of {
            Of::Heat => self.drafts.heatmap.ramp = stops,
            _ => self.drafts.unclassed.ramp = stops,
        }
    }

    fn value_fields(&mut self, of: Of) -> Option<&mut Vec<ValueField>> {
        match of {
            Of::Dots => Some(&mut self.drafts.dot_density.fields),
            Of::Chart => Some(&mut self.drafts.chart.fields),
            _ => None,
        }
    }

    /// A number field as typed: the value when it reads and is in range.
    fn number(&mut self, n: Num, text: String) {
        let trimmed = kentos_processing::text::js_trim(&text).to_owned();
        let x = js_number(&trimmed.replacen(',', ".", 1));
        self.typed.insert(num_key(n), text);
        let (lo, hi) = self.bounds(n);
        if trimmed.is_empty() || !x.is_finite() || x < lo || x > hi {
            return;
        }
        let d = &mut self.drafts;
        match n {
            Num::UnMin => d.unclassed.min = x,
            Num::UnMax => d.unclassed.max = x,
            Num::PrMinValue => d.proportional.min_value = x,
            Num::PrMaxValue => d.proportional.max_value = x,
            Num::PrMinSize => d.proportional.min_size = x,
            Num::PrMaxSize => d.proportional.max_size = x,
            Num::DotValue => d.dot_density.dot_value = x,
            Num::DotSize => d.dot_density.dot_size = Some(x),
            Num::Seed => {
                d.dot_density.seed = Some(kentos_native_style::classify::js_round(x));
            }
            Num::ChSize => d.chart.size = x,
            Num::ChBarWidth => d.chart.bar_width = Some(x),
            Num::ChMaxValue => d.chart.max_value = Some(x),
            Num::SbMinValue => {
                if let Some(s) = &mut d.chart.size_by {
                    s.min_value = x;
                }
            }
            Num::SbMaxValue => {
                if let Some(s) = &mut d.chart.size_by {
                    s.max_value = x;
                }
            }
            Num::SbMinSize => {
                if let Some(s) = &mut d.chart.size_by {
                    s.min_size = x;
                }
            }
            Num::SbMaxSize => {
                if let Some(s) = &mut d.chart.size_by {
                    s.max_size = x;
                }
            }
            Num::OutlineWidth => {
                if let Some(o) = &mut d.chart.outline {
                    o.width = x;
                }
            }
            Num::Radius => d.heatmap.radius = x,
            Num::HeatMax => d.heatmap.max = Some(x),
            Num::Distance => d.cluster.distance = x,
            Num::Tolerance => d.displacement.tolerance = x,
            Num::Spacing => d.displacement.spacing = Some(x),
            Num::CircleWidth => {
                if let Some(c) = &mut d.displacement.circle {
                    c.width = x;
                }
            }
        }
    }

    /// The range a number field takes (the style core's rules, docs/adr/0213 §5).
    fn bounds(&self, n: Num) -> (f64, f64) {
        let px = |u: &Option<String>| u.as_deref() != Some("m");
        let d = &self.drafts;
        match n {
            Num::UnMin | Num::UnMax | Num::PrMinValue | Num::PrMaxValue => {
                (f64::NEG_INFINITY, f64::INFINITY)
            }
            Num::SbMinValue | Num::SbMaxValue => (f64::NEG_INFINITY, f64::INFINITY),
            Num::PrMinSize | Num::PrMaxSize | Num::ChSize | Num::SbMinSize | Num::SbMaxSize => {
                (0.01, 200.0)
            }
            Num::DotValue | Num::ChMaxValue | Num::HeatMax => (1e-9, f64::INFINITY),
            Num::DotSize => (0.01, 20.0),
            Num::Seed => (0.0, 2_147_483_647.0),
            Num::ChBarWidth => (0.01, 50.0),
            Num::OutlineWidth | Num::CircleWidth => (0.0, 10.0),
            Num::Radius => (
                0.01,
                if px(&d.heatmap.unit) {
                    500.0
                } else {
                    f64::INFINITY
                },
            ),
            Num::Distance => (
                0.01,
                if px(&d.cluster.unit) {
                    500.0
                } else {
                    f64::INFINITY
                },
            ),
            Num::Tolerance => (
                0.01,
                if px(&d.displacement.unit) {
                    100.0
                } else {
                    f64::INFINITY
                },
            ),
            Num::Spacing => (0.0, 100.0),
        }
    }

    /// A number field's text: as typed, else the value.
    fn num_text(&self, n: Num) -> String {
        if let Some(t) = self.typed.get(&num_key(n)) {
            return t.clone();
        }
        let d = &self.drafts;
        let v = match n {
            Num::UnMin => Some(d.unclassed.min),
            Num::UnMax => Some(d.unclassed.max),
            Num::PrMinValue => Some(d.proportional.min_value),
            Num::PrMaxValue => Some(d.proportional.max_value),
            Num::PrMinSize => Some(d.proportional.min_size),
            Num::PrMaxSize => Some(d.proportional.max_size),
            Num::DotValue => Some(d.dot_density.dot_value),
            Num::DotSize => Some(d.dot_density.dot_size.unwrap_or(1.0)),
            Num::Seed => Some(d.dot_density.seed.unwrap_or(0.0)),
            Num::ChSize => Some(d.chart.size),
            Num::ChBarWidth => Some(d.chart.bar_width.unwrap_or(d.chart.size / 4.0)),
            Num::ChMaxValue => d.chart.max_value,
            Num::SbMinValue => d.chart.size_by.map(|s| s.min_value),
            Num::SbMaxValue => d.chart.size_by.map(|s| s.max_value),
            Num::SbMinSize => d.chart.size_by.map(|s| s.min_size),
            Num::SbMaxSize => d.chart.size_by.map(|s| s.max_size),
            Num::OutlineWidth => Some(d.chart.outline.as_ref().map_or(0.2, |o| o.width)),
            Num::Radius => Some(d.heatmap.radius),
            Num::HeatMax => d.heatmap.max,
            Num::Distance => Some(d.cluster.distance),
            Num::Tolerance => Some(d.displacement.tolerance),
            Num::Spacing => Some(d.displacement.spacing.unwrap_or(0.0)),
            Num::CircleWidth => Some(d.displacement.circle.as_ref().map_or(1.0, |c| c.width)),
        };
        v.map(js_text).unwrap_or_default()
    }

    fn take_from_data(&mut self, of: Of, src: &Source<'_>) {
        match of {
            Of::Unclassed | Of::Proportional => {
                let expr = match of {
                    Of::Unclassed => self.drafts.unclassed.expr.clone(),
                    _ => self.drafts.proportional.expr.clone(),
                };
                let Some((lo, hi)) = range_of(&self.numbers_for(src, &expr).values) else {
                    return;
                };
                if of == Of::Unclassed {
                    self.drafts.unclassed.min = lo;
                    self.drafts.unclassed.max = hi;
                    self.forget(&[Num::UnMin, Num::UnMax]);
                } else {
                    self.drafts.proportional.min_value = lo;
                    self.drafts.proportional.max_value = hi;
                    self.forget(&[Num::PrMinValue, Num::PrMaxValue]);
                }
            }
            Of::Chart => {
                let (top, lo) = self.chart_largest(src);
                if top <= 0.0 {
                    self.say("Değerlerin hiçbiri sıfırdan büyük değil.", true);
                    return;
                }
                let c = &mut self.drafts.chart;
                if c.kind.as_deref().unwrap_or("pie") == "pie" {
                    let (min_size, max_size) = c
                        .size_by
                        .map_or((c.size / 2.0, c.size), |s| (s.min_size, s.max_size));
                    c.size_by = Some(SizeBy {
                        min_value: lo,
                        max_value: top,
                        min_size,
                        max_size,
                    });
                } else {
                    c.max_value = Some(top);
                }
                self.forget(&[Num::SbMinValue, Num::SbMaxValue, Num::ChMaxValue]);
            }
            _ => {}
        }
    }

    /// The largest total (a pie's, stacked bars') or value (bars'), and the smallest above zero.
    fn chart_largest(&self, src: &Source<'_>) -> (f64, f64) {
        let bars = self.drafts.chart.kind.as_deref() == Some("bar");
        let cols: Vec<_> = self
            .drafts
            .chart
            .fields
            .iter()
            .map(|f| self.numbers_for(src, &f.expr))
            .collect();
        let n = cols.first().map_or(0, |c| c.values.len());
        let (mut top, mut lo) = (0.0_f64, f64::INFINITY);
        for i in 0..n {
            let vals = cols
                .iter()
                .map(|c| c.values.get(i).copied().flatten().unwrap_or(0.0).max(0.0));
            let v = if bars {
                vals.fold(0.0_f64, f64::max)
            } else {
                vals.sum()
            };
            top = top.max(v);
            if v > 0.0 {
                lo = lo.min(v);
            }
        }
        (top, if lo.is_finite() { lo } else { 0.0 })
    }

    /// Sınıfla: both axes' breaks for `n` classes and the scheme's colours.
    fn classify_bivariate(&mut self, n: usize, src: &Source<'_>) {
        let values = |expr: &str| -> Vec<f64> {
            self.numbers_for(src, expr)
                .values
                .iter()
                .flatten()
                .copied()
                .collect()
        };
        let xs = values(&self.drafts.bivariate.expr_x);
        let ys = values(&self.drafts.bivariate.expr_y);
        let (Some(bx), Some(by)) = (
            breaks_of(&xs, n, self.bivariate_method),
            breaks_of(&ys, n, self.bivariate_method),
        ) else {
            self.say(
                "Değerler bu kadar sınıfa ayrılamıyor: çoğu aynı. Daha az sınıf deneyin.",
                true,
            );
            return;
        };
        let b = &mut self.drafts.bivariate;
        b.breaks_x = bx;
        b.breaks_y = by;
        b.colors = bivariate_colors(&BIVARIATE_SCHEMES[self.bivariate_scheme].2, n);
        self.say(format!("{n} × {n} sınıf."), false);
    }
}

/// A ramp list as (key, label, stops).
fn ramp_list(of: Of) -> Vec<(&'static str, &'static str, &'static [&'static str])> {
    match of {
        Of::Heat => HEAT_RAMPS.to_vec(),
        _ => RAMPS.iter().map(|r| (r.key, r.label, r.stops)).collect(),
    }
}

/// The list's ramp `stops` is (its place), and whether reversed.
fn ramp_match(
    list: &[(&'static str, &'static str, &'static [&'static str])],
    stops: &[String],
) -> Option<(usize, bool)> {
    let same = |a: &[&str], b: &mut dyn Iterator<Item = &String>| {
        let b: Vec<&String> = b.collect();
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.eq_ignore_ascii_case(y))
    };
    list.iter().enumerate().find_map(|(i, (_, _, s))| {
        if same(s, &mut stops.iter()) {
            Some((i, false))
        } else if same(s, &mut stops.iter().rev()) {
            Some((i, true))
        } else {
            None
        }
    })
}

// ── View ────────────────────────────────────────────────────────────────

/// A row of a panel: its label in a fixed column, then its controls.
fn line<'a>(text: &str, controls: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    let mut r = Row::new().spacing(8).align_y(Center).push(
        container(label::caption(text.to_owned()).style(style::text::muted))
            .width(Length::Fixed(typography::scaled(LABEL))),
    );
    for c in controls {
        r = r.push(c);
    }
    r.into()
}

fn muted<'a>(text: impl Into<String>) -> Element<'a, Message> {
    label::caption(text.into()).style(style::text::muted).into()
}

fn caption<'a>(text: &'static str) -> Element<'a, Message> {
    label::caption(text).into()
}

fn num<'a>(w: &LayerStyleWindow, n: Num, hint: &str) -> Element<'a, Message> {
    input(hint, &w.num_text(n))
        .on_input(move |t| th(Edit::Number(n, t)))
        .align_x(iced::alignment::Horizontal::Right)
        .width(Length::Fixed(typography::scaled(NUMBER)))
        .into()
}

fn small<'a>(text: &'static str, press: Option<Message>) -> Element<'a, Message> {
    button(label::body(text))
        .padding([4, 10])
        .style(style::button::secondary)
        .on_press_maybe(press)
        .into()
}

/// A switch and its words, the web's `lsty__check`.
fn switch<'a>(on: bool, text: &'static str, press: Message) -> Element<'a, Message> {
    row![check(on, press), label::body(text)]
        .spacing(4)
        .align_y(Center)
        .into()
}

/// A segmented choice by its labels; `f` gets the chosen one's place.
fn choice<'a, const N: usize>(
    labels: [&'static str; N],
    at: usize,
    f: impl Fn(usize) -> Message,
) -> Segmented<'a, Message> {
    let current = labels[at.min(N - 1)];
    Segmented::new(labels, current, move |l| {
        f(labels.iter().position(|x| *x == l).unwrap_or(0))
    })
}

fn units<'a>(of: Of, options: [&'static str; 2], value: Option<&str>) -> Element<'a, Message> {
    let current = options
        .into_iter()
        .find(|o| Some(*o) == value)
        .unwrap_or(options[0]);
    Segmented::new(options, current, move |u| th(Edit::Unit(of, u)))
        .compact()
        .into()
}

/// A colour's swatch that opens the picker.
fn swatch<'a>(color: &str, on_change: impl Fn(String) -> Message + 'a) -> Element<'a, Message> {
    let c = parse_hex(color).unwrap_or(iced::Color::WHITE);
    let size = typography::scaled(22.0);
    let face = container(space())
        .width(size)
        .height(size)
        .style(move |t: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(c)),
            border: iced::Border {
                color: kentos_ui::theme::Tokens::of(t).muted,
                width: 1.0,
                radius: kentos_ui::theme::shape::radius(3.0).into(),
            },
            ..container::Style::default()
        });
    ColorPicker::new(c, move |c| on_change(to_hex(c)))
        .anchor(face)
        .into()
}

/// A ramp's colours side by side (data colours, not the theme's).
fn strip<'a>(stops: &[String]) -> Element<'a, Message> {
    let mut out = Row::new().spacing(0);
    const STEPS: usize = 24;
    for k in 0..STEPS {
        let c = ramp_at(stops, k as f64 / (STEPS - 1) as f64);
        let color = parse_hex(&c).unwrap_or(iced::Color::TRANSPARENT);
        out = out.push(
            container(space())
                .width(Length::Fixed(typography::scaled(5.0)))
                .height(Length::Fixed(typography::scaled(14.0)))
                .style(style::container::solid(color)),
        );
    }
    container(out)
        .padding(1)
        .style(style::container::bordered)
        .into()
}

/// The ramps a renderer may take by name, Ters, and the strip.
fn ramp_controls<'a>(of: Of, stops: &[String]) -> Vec<Element<'a, Message>> {
    let list = ramp_list(of);
    let found = ramp_match(&list, stops);
    let mut choices: Vec<Choice> = list
        .iter()
        .map(|(_, l, s)| {
            let last = s
                .last()
                .and_then(|c| parse_hex(c))
                .unwrap_or(iced::Color::BLACK);
            Choice::new(*l).color(last)
        })
        .collect();
    if found.is_none() {
        choices.push(Choice::new("Bu stilin renkleri"));
    }
    let count = list.len();
    let select = Select::new(choices, Some(found.map_or(count, |f| f.0)), move |i| {
        // “Bu stilin renkleri” is what the style has: choosing it changes nothing.
        th(Edit::Ramp(of, if i < count { i } else { usize::MAX }))
    })
    .searchable(false);
    vec![
        container(select)
            .width(Length::Fixed(typography::scaled(180.0)))
            .into(),
        switch(found.is_some_and(|f| f.1), "Ters", th(Edit::Reverse(of))),
        strip(stops),
    ]
}

/// The objects with and without a value of an expression.
fn values_note<'a>(values: &[Option<f64>], other: bool) -> Element<'a, Message> {
    let n = values.iter().filter(|v| v.is_some()).count();
    let rest = values.len() - n;
    help(if rest > 0 {
        format!(
            "{n} nesnenin değeri var; {rest} nesnenin yok: {}.",
            if other {
                "“Değeri olmayanlar” sembolüyle çizilir"
            } else {
                "çizilmez"
            }
        )
    } else {
        format!("{n} nesnenin değeri var.")
    })
}

fn other_row<'a>(
    env: &Env<'_>,
    of: Of,
    at: SetAt,
    other: Option<&SymbolSet>,
) -> Element<'a, Message> {
    let mut controls = vec![switch(
        other.is_some(),
        "Çizilsin",
        th(Edit::Other(of, other.is_none())),
    )];
    if let Some(set) = other {
        controls.push(slots_in(env, at, set, "Değeri olmayanlar", env.classes));
    }
    line("Değeri olmayanlar", controls)
}

/// An expression row and the problem under it.
fn expr_rows<'a>(
    out: Column<'a, Message>,
    title: &str,
    field: Field,
    text: &str,
    placeholder: &str,
    fields: &[(String, usize)],
    error: Option<String>,
) -> Column<'a, Message> {
    let out = out.push(line(
        title,
        vec![
            container(expression(
                field,
                text,
                placeholder,
                fields,
                error.is_some(),
                false,
            ))
            .width(iced::Fill)
            .into(),
        ],
    ));
    match error {
        Some(e) => out.push(row![
            space().width(typography::scaled(LABEL + 8.0)),
            problem(e)
        ]),
        None => out,
    }
}

fn panel<'a>(about: &'static str) -> Column<'a, Message> {
    Column::new().spacing(10).push(help(about))
}

/// The view of a thematic kind's panel (none for the others).
pub(super) fn panel_of<'a>(
    w: &LayerStyleWindow,
    env: &Env<'_>,
    src: &Source<'_>,
    fields: &[(String, usize)],
) -> Option<Element<'a, Message>> {
    Some(match w.kind {
        Kind::Unclassed => unclassed(w, env, src, fields),
        Kind::Proportional => proportional(w, env, src, fields),
        Kind::Bivariate => bivariate(w, env, src, fields),
        Kind::DotDensity => dot_density(w, env, fields),
        Kind::Chart => chart(w, env, fields),
        Kind::Heatmap => heatmap(w, src, fields),
        Kind::Cluster => cluster(w, env),
        Kind::Displacement => displacement(w, env),
        Kind::Inverted => inverted(w, env),
        _ => return None,
    })
}

/// The values of an expression as the panels read them: none for an empty one.
fn values_of(
    w: &LayerStyleWindow,
    src: &Source<'_>,
    expr: &str,
) -> (Vec<Option<f64>>, Option<String>) {
    if kentos_processing::text::js_trim(expr).is_empty() {
        return (Vec::new(), None);
    }
    let e = w.numbers_for(src, expr);
    (e.values.clone(), e.error_text())
}

fn unclassed<'a>(
    w: &LayerStyleWindow,
    env: &Env<'_>,
    src: &Source<'_>,
    fields: &[(String, usize)],
) -> Element<'a, Message> {
    let r = &w.drafts.unclassed;
    let (values, error) = values_of(w, src, &r.expr);
    let any = range_of(&values).is_some();
    let mut out = panel(
        "Değer, en küçük ile en büyük arasında rampanın sürekli bir rengine döner; sembolün ana rengi o olur (çerçeveler kalır).",
    );
    out = expr_rows(
        out,
        "Değer",
        Field::Unclassed,
        &r.expr,
        "Sayı veren ifade: Nüfus, $alan",
        fields,
        error.clone(),
    );
    out = out
        .push(line(
            "Aralık",
            vec![
                num(w, Num::UnMin, "En küçük"),
                muted("–"),
                num(w, Num::UnMax, "En büyük"),
                small(
                    "Verilerden al",
                    any.then(|| th(Edit::FromData(Of::Unclassed))),
                ),
            ],
        ))
        .push(line("Renkler", ramp_controls(Of::Unclassed, &r.ramp)))
        .push(line(
            "Sembol",
            vec![slots_in(
                env,
                SetAt::Unclassed,
                &r.symbols,
                "Sürekli renk",
                env.classes,
            )],
        ))
        .push(other_row(
            env,
            Of::Unclassed,
            SetAt::UnclassedOther,
            r.other.as_ref(),
        ));
    if !r.expr.trim().is_empty() && error.is_none() {
        out = out.push(values_note(&values, r.other.is_some()));
    }
    out.into()
}

fn proportional<'a>(
    w: &LayerStyleWindow,
    env: &Env<'_>,
    src: &Source<'_>,
    fields: &[(String, usize)],
) -> Element<'a, Message> {
    let r = &w.drafts.proportional;
    let (values, error) = values_of(w, src, &r.expr);
    let any = range_of(&values).is_some();
    // Points and areas take the marker symbol (an area at its inside point, over its own fill), lines the line symbol.
    let mut classes = Vec::new();
    if w.present.marker > 0 || w.present.fill > 0 {
        classes.push(GeometryClass::Marker);
    }
    if w.present.line > 0 {
        classes.push(GeometryClass::Line);
    }
    if classes.is_empty() {
        classes.push(GeometryClass::Marker);
    }
    let scaling = r.scaling.as_deref().unwrap_or("area");
    let scalings = ["area", "radius", "flannery"];
    let pick = scalings.into_iter().position(|s| s == scaling).unwrap_or(0);
    let mut out = panel(
        "Değer sembolün boyunu verir: nokta ve alanın iç noktasında işaretin boyu, çizgide kalınlık.",
    );
    out = expr_rows(
        out,
        "Değer",
        Field::Proportional,
        &r.expr,
        "Sayı veren ifade: Nüfus, $alan",
        fields,
        error.clone(),
    );
    out = out
        .push(line(
            "Değerler",
            vec![
                num(w, Num::PrMinValue, "En küçük değer"),
                muted("–"),
                num(w, Num::PrMaxValue, "En büyük değer"),
                small(
                    "Verilerden al",
                    any.then(|| th(Edit::FromData(Of::Proportional))),
                ),
            ],
        ))
        .push(line(
            "Boylar",
            vec![
                num(w, Num::PrMinSize, "En küçük boy"),
                muted("–"),
                num(w, Num::PrMaxSize, "En büyük boy"),
                units(Of::Proportional, ["mm", "px"], r.unit.as_deref()),
            ],
        ))
        .push(line(
            "Ölçekleme",
            vec![
                choice(["Alan", "Yarıçap", "Flannery"], pick, move |i| {
                    th(Edit::Scaling(scalings[i.min(2)]))
                })
                .hints([
                    "Sembolün alanı değerle orantılı (karekök).",
                    "Sembolün boyu değerle doğru orantılı.",
                    "Gözün büyük daireleri küçük görmesine göre düzeltilmiş (üs 0,57).",
                ])
                .compact()
                .into(),
            ],
        ))
        .push(line(
            "Sembol",
            vec![slots_in(
                env,
                SetAt::Proportional,
                &r.symbols,
                "Orantılı sembol",
                &classes,
            )],
        ));
    if w.present.fill > 0 {
        out = out.push(line(
            "Alanın zemini",
            vec![slots_in(
                env,
                SetAt::Proportional,
                &r.symbols,
                "Alanın zemini",
                &[GeometryClass::Fill],
            )],
        ));
    }
    out = out.push(other_row(
        env,
        Of::Proportional,
        SetAt::ProportionalOther,
        r.other.as_ref(),
    ));
    if !r.expr.trim().is_empty() && error.is_none() {
        out = out.push(values_note(&values, r.other.is_some()));
    }
    out.into()
}

fn bivariate<'a>(
    w: &LayerStyleWindow,
    env: &Env<'_>,
    src: &Source<'_>,
    fields: &[(String, usize)],
) -> Element<'a, Message> {
    let r = &w.drafts.bivariate;
    let n = r.breaks_x.len() + 1;
    let (_, ex) = values_of(w, src, &r.expr_x);
    let (_, ey) = values_of(w, src, &r.expr_y);
    let ready =
        !r.expr_x.trim().is_empty() && !r.expr_y.trim().is_empty() && ex.is_none() && ey.is_none();
    let mut out = panel(
        "İki değerin sınıfları bir renk ızgarasında bir renge döner: X sağa, Y yukarı artar. Sembolün ana rengi o olur.",
    );
    out = expr_rows(
        out,
        "X değeri",
        Field::BivariateX,
        &r.expr_x,
        "Yatay eksenin değeri",
        fields,
        ex,
    );
    out = expr_rows(
        out,
        "Y değeri",
        Field::BivariateY,
        &r.expr_y,
        "Düşey eksenin değeri",
        fields,
        ey,
    );
    let methods = [Method::Count, Method::Interval];
    let schemes: Vec<Choice> = BIVARIATE_SCHEMES
        .iter()
        .map(|(_, l, c)| Choice::new(*l).color(parse_hex(c[3]).unwrap_or(iced::Color::BLACK)))
        .collect();
    // The grid: Y's classes upward, X's rightward, each cell's colour.
    let cell = typography::scaled(22.0);
    let mut grid = Column::new().spacing(1);
    for j in (0..n).rev() {
        let mut r_ = Row::new().spacing(1);
        for i in 0..n {
            let c = r
                .colors
                .get(j * n + i)
                .and_then(|c| parse_hex(c))
                .unwrap_or(iced::Color::TRANSPARENT);
            r_ = r_.push(
                container(space())
                    .width(cell)
                    .height(cell)
                    .style(style::container::solid(c)),
            );
        }
        grid = grid.push(r_);
    }
    let fmt = |b: &[f64]| {
        if b.is_empty() {
            "—".to_owned()
        } else {
            b.iter()
                .map(|x| kentos_native_style::thematic::legend_number(*x))
                .collect::<Vec<_>>()
                .join(" · ")
        }
    };
    out.push(line(
        "Sınıflar",
        vec![
            choice(["2 × 2", "3 × 3", "4 × 4"], n.clamp(2, 4) - 2, |i| {
                th(Edit::BiCount(i + 2))
            })
            .compact()
            .into(),
            choice(
                ["Eşit sayı", "Eşit aralık"],
                usize::from(w.bivariate_method == Method::Interval),
                move |i| th(Edit::BiMethod(methods[i.min(1)])),
            )
            .compact()
            .into(),
            small("Sınıfla", ready.then(|| th(Edit::BiClassify))),
        ],
    ))
    .push(line(
        "Renkler",
        vec![
            container(
                Select::new(schemes, Some(w.bivariate_scheme), |i| th(Edit::BiScheme(i)))
                    .searchable(false),
            )
            .width(Length::Fixed(typography::scaled(180.0)))
            .into(),
            container(grid)
                .padding(1)
                .style(style::container::bordered)
                .into(),
        ],
    ))
    .push(line(
        "Sınırlar",
        vec![muted(format!(
            "X: {}   Y: {}",
            fmt(&r.breaks_x),
            fmt(&r.breaks_y)
        ))],
    ))
    .push(line(
        "Sembol",
        vec![slots_in(
            env,
            SetAt::Bivariate,
            &r.symbols,
            "İki değişkenli renk",
            env.classes,
        )],
    ))
    .push(other_row(
        env,
        Of::Bivariate,
        SetAt::BivariateOther,
        r.other.as_ref(),
    ))
    .into()
}

/// A value list (Nokta yoğunluğu, Grafik): colour, expression, label, and the row's tools.
fn fields_table<'a>(
    of: Of,
    list: &[ValueField],
    fields: &[(String, usize)],
) -> Element<'a, Message> {
    let fixed = |w: f32| Length::Fixed(typography::scaled(w));
    let head = |t: &'static str, w: Length| -> Element<'a, Message> {
        container(label::caption(t).style(style::text::muted))
            .width(w)
            .into()
    };
    let mut table = Column::new().spacing(4).push(
        row![
            head("Renk", fixed(34.0)),
            head("Değer", Length::FillPortion(3)),
            head("Etiket", Length::FillPortion(2)),
            head("", fixed(84.0)),
        ]
        .spacing(8)
        .align_y(Center),
    );
    let n = list.len();
    for (i, f) in list.iter().enumerate() {
        let field = if of == Of::Dots {
            Field::DotValue(i)
        } else {
            Field::ChartValue(i)
        };
        table = table.push(
            row![
                container(swatch(&f.color, move |c| th(Edit::FieldColor(of, i, c))))
                    .width(fixed(34.0)),
                container(expression(
                    field,
                    &f.expr,
                    "Alan adı ya da ifade",
                    fields,
                    false,
                    true
                ))
                .width(Length::FillPortion(3)),
                container(
                    input(&f.expr, f.label.as_deref().unwrap_or_default())
                        .on_input(move |t| th(Edit::FieldLabel(of, i, t)))
                        .width(iced::Fill)
                )
                .width(Length::FillPortion(2)),
                row![
                    tool(
                        "chevronUp",
                        "Yukarı",
                        (i > 0).then(|| th(Edit::FieldMove(of, i, true)))
                    ),
                    tool(
                        "chevronDown",
                        "Aşağı",
                        (i + 1 < n).then(|| th(Edit::FieldMove(of, i, false)))
                    ),
                    tool(
                        "trash",
                        "Sil",
                        (n > 1).then(|| th(Edit::FieldRemove(of, i)))
                    ),
                ]
                .spacing(2)
                .width(fixed(84.0)),
            ]
            .spacing(8)
            .align_y(Center),
        );
    }
    let add = button(
        row![icon(Icon::Plus).size(13.0), label::body("Değer ekle")]
            .spacing(5)
            .align_y(Center),
    )
    .padding([4, 10])
    .style(style::button::secondary)
    .on_press_maybe((n < 12).then(|| th(Edit::FieldAdd(of))));
    column![table, add].spacing(8).into()
}

fn dot_density<'a>(
    w: &LayerStyleWindow,
    env: &Env<'_>,
    fields: &[(String, usize)],
) -> Element<'a, Message> {
    let r = &w.drafts.dot_density;
    let ground = r.symbols.clone().unwrap_or_default();
    panel("Her alanın içine değeri / nokta değeri kadar nokta rastgele ama hep aynı yerlere konur; her değer kendi renginde.")
        .push(fields_table(Of::Dots, &r.fields, fields))
        .push(line(
            "Nokta",
            vec![
                muted("1 nokta ="),
                num(w, Num::DotValue, "Nokta değeri"),
                caption("Boy"),
                num(w, Num::DotSize, "Nokta boyu"),
                units(Of::Dots, ["mm", "px"], r.unit.as_deref()),
            ],
        ))
        .push(line(
            "Yerleşim",
            vec![
                caption("Tohum"),
                num(w, Num::Seed, "Tohum"),
                muted("Başka bir sayı noktaları başka yerlere koyar."),
            ],
        ))
        .push(line(
            "Zemin",
            vec![slots_in(env, SetAt::DotGround, &ground, "Zemin", env.classes)],
        ))
        .into()
}

fn chart<'a>(
    w: &LayerStyleWindow,
    env: &Env<'_>,
    fields: &[(String, usize)],
) -> Element<'a, Message> {
    let r = &w.drafts.chart;
    let kind = r.kind.as_deref().unwrap_or("pie");
    let kinds = ["pie", "bar", "stacked"];
    let pick = kinds.into_iter().position(|k| k == kind).unwrap_or(0);
    let pie = kind == "pie";
    let unit = r.unit.as_deref().unwrap_or("mm").to_owned();
    let mut size = vec![
        num(w, Num::ChSize, "Boy"),
        units(Of::Chart, ["mm", "px"], r.unit.as_deref()),
    ];
    if pie {
        size.push(switch(
            r.size_by.is_some(),
            "Toplama göre",
            th(Edit::SizeBy(r.size_by.is_none())),
        ));
    } else {
        size.push(caption("Genişlik"));
        size.push(num(w, Num::ChBarWidth, "Çubuk genişliği"));
    }
    let mut out = panel("Her nesnenin üstüne (alanın iç noktasına, çizginin ortasına) değerlerinin grafiği çizilir.")
        .push(line(
            "Tür",
            vec![
                choice(["Pasta", "Çubuk", "Yığılmış çubuk"], pick, move |i| {
                    th(Edit::ChartKind(kinds[i.min(2)]))
                })
                .compact()
                .into(),
            ],
        ))
        .push(fields_table(Of::Chart, &r.fields, fields))
        .push(line(if pie { "Çap" } else { "Yükseklik" }, size));
    if !pie {
        out = out.push(line(
            "En büyük değer",
            vec![
                num(w, Num::ChMaxValue, "En büyük değer"),
                muted(format!("bu değer {} {unit} yüksekliktir", js_text(r.size))),
                small("Verilerden al", Some(th(Edit::FromData(Of::Chart)))),
            ],
        ));
    } else if r.size_by.is_some() {
        out = out.push(line(
            "Toplam",
            vec![
                num(w, Num::SbMinValue, "En küçük toplam"),
                muted("–"),
                num(w, Num::SbMaxValue, "En büyük toplam"),
                caption("Çap"),
                num(w, Num::SbMinSize, "En küçük çap"),
                muted("–"),
                num(w, Num::SbMaxSize, "En büyük çap"),
                small("Verilerden al", Some(th(Edit::FromData(Of::Chart)))),
            ],
        ));
    }
    let outline = r.outline.as_ref();
    let mut frame = vec![switch(
        outline.is_some(),
        "Çiz",
        th(Edit::Outline(outline.is_none())),
    )];
    if let Some(o) = outline {
        frame.push(swatch(&o.color, |c| th(Edit::OutlineColor(c))));
        frame.push(num(w, Num::OutlineWidth, "Çerçevenin kalınlığı"));
    }
    let ground = r.symbols.clone().unwrap_or_default();
    out.push(line("Çerçeve", frame))
        .push(line(
            "Zemin",
            vec![slots_in(
                env,
                SetAt::ChartGround,
                &ground,
                "Zemin",
                env.classes,
            )],
        ))
        .into()
}

fn heatmap<'a>(
    w: &LayerStyleWindow,
    src: &Source<'_>,
    fields: &[(String, usize)],
) -> Element<'a, Message> {
    let r = &w.drafts.heatmap;
    let weight = r.weight.clone().unwrap_or_default();
    let (_, error) = values_of(w, src, &weight);
    let fixed = r.max.is_some();
    let quality = r.quality.map_or(2, |q| q as u8).clamp(1, 5);
    let opacity = (r.opacity.unwrap_or(1.0) * 100.0) as f32;
    let mut most = vec![
        choice(["Dinamik", "Sabit"], usize::from(fixed), |i| {
            th(Edit::HeatFixed(i == 1))
        })
        .hints([
            "Görünümdeki en yoğun yer en koyu renk.",
            "Bu değer ve üstü en koyu renk; renkler yakınlaşınca değişmez.",
        ])
        .compact()
        .into(),
    ];
    if fixed {
        most.push(num(w, Num::HeatMax, "En büyük değer"));
    }
    let mut out = panel("Noktaların yoğunluğu, görünümün renkli resmi olur. Yalnız noktalar sayılır; katmanın öbür nesneleri çizilmez.")
        .push(line(
            "Yarıçap",
            vec![num(w, Num::Radius, "Yarıçap"), units(Of::Heat, ["px", "m"], r.unit.as_deref())],
        ));
    out = expr_rows(
        out,
        "Ağırlık",
        Field::Weight,
        &weight,
        "Boş: her nokta bir",
        fields,
        error,
    );
    out.push(line("En büyük değer", most))
        .push(line("Renkler", ramp_controls(Of::Heat, &r.ramp)))
        .push(line(
            "Kalite",
            vec![
                choice(
                    ["1 (en iyi)", "2", "3", "4", "5"],
                    usize::from(quality - 1),
                    |i| th(Edit::Quality(i as u8 + 1)),
                )
                .hints([
                    "Hücre 1 piksel",
                    "Hücre 2 piksel",
                    "Hücre 3 piksel",
                    "Hücre 4 piksel",
                    "Hücre 5 piksel",
                ])
                .compact()
                .into(),
            ],
        ))
        .push(line(
            "Opaklık",
            vec![
                container(slider(0.0..=100.0, opacity, |v| th(Edit::Opacity(v))).step(1.0))
                    .width(Length::Fixed(typography::scaled(160.0)))
                    .into(),
                muted(format!("%{}", opacity.round() as i32)),
            ],
        ))
        .into()
}

fn inner_row<'a>(inner: Kind) -> Element<'a, Message> {
    let choices: Vec<Choice> = INNER_KINDS.iter().map(|(_, l)| Choice::new(*l)).collect();
    let at = INNER_KINDS
        .iter()
        .position(|(k, _)| *k == inner)
        .unwrap_or(0);
    line(
        "Tek noktalar",
        vec![
            container(
                Select::new(choices, Some(at), |i| {
                    th(Edit::Inner(INNER_KINDS[i.min(INNER_KINDS.len() - 1)].0))
                })
                .searchable(false),
            )
            .width(Length::Fixed(typography::scaled(200.0)))
            .into(),
            muted(if inner == Kind::Simple {
                "katmanın görünüşü"
            } else {
                "ayarları kendi türünün sayfasında"
            }),
        ],
    )
}

fn marker_slot<'a>(
    env: &Env<'_>,
    at: SetAt,
    symbol: Option<&Value>,
    title: &str,
) -> Element<'a, Message> {
    let set = SymbolSet {
        marker: symbol.cloned(),
        ..SymbolSet::default()
    };
    slots_in(env, at, &set, title, &[GeometryClass::Marker])
}

fn cluster<'a>(w: &LayerStyleWindow, env: &Env<'_>) -> Element<'a, Message> {
    let r = &w.drafts.cluster;
    let mut mark = vec![marker_slot(
        env,
        SetAt::ClusterMark,
        r.symbol.as_ref(),
        "Küme işareti",
    )];
    if r.symbol.is_none() {
        mark.push(muted("katmanın renginde daire"));
    }
    panel("Ekranda birbirine bu uzaklıktan yakın noktalar tek bir küme işareti olur; sayıları içinde yazar. Yakınlaştıkça kümeler açılır.")
        .push(line(
            "Uzaklık",
            vec![num(w, Num::Distance, "Uzaklık"), units(Of::Cluster, ["px", "m"], r.unit.as_deref())],
        ))
        .push(line("Küme işareti", mark))
        .push(line(
            "Seçenekler",
            vec![
                switch(r.count != Some(false), "Sayısını yaz", th(Edit::Count(r.count == Some(false)))),
                switch(r.grow == Some(true), "Sayısıyla büyüsün", th(Edit::Grow(r.grow != Some(true)))),
            ],
        ))
        .push(inner_row(w.inner))
        .into()
}

fn displacement<'a>(w: &LayerStyleWindow, env: &Env<'_>) -> Element<'a, Message> {
    let r = &w.drafts.displacement;
    let places = ["ring", "rings", "grid"];
    let pick = places
        .into_iter()
        .position(|p| Some(p) == r.placement.as_deref())
        .unwrap_or(0);
    let mut circle = vec![switch(
        r.circle.is_some(),
        "Çiz",
        th(Edit::Circle(r.circle.is_none())),
    )];
    if let Some(c) = &r.circle {
        circle.push(swatch(&c.color, |c| th(Edit::CircleColor(c))));
        circle.push(num(w, Num::CircleWidth, "Halkanın kalınlığı"));
        circle.push(muted("px"));
    }
    panel("Üst üste binen noktalar ortalarının çevresine dağıtılır; her biri kendi sembolüyle görünür.")
        .push(line(
            "Tolerans",
            vec![
                num(w, Num::Tolerance, "Tolerans"),
                units(Of::Displacement, ["px", "m"], r.unit.as_deref()),
            ],
        ))
        .push(line(
            "Yerleşim",
            vec![
                choice(["Halka", "İç içe halkalar", "Izgara"], pick, move |i| {
                    th(Edit::Placement(places[i.min(2)]))
                })
                .compact()
                .into(),
                caption("Aralık"),
                num(w, Num::Spacing, "Aralık"),
                muted("px"),
            ],
        ))
        .push(line(
            "Merkez işareti",
            vec![marker_slot(env, SetAt::DisplacementCenter, r.center.as_ref(), "Merkez işareti")],
        ))
        .push(line("Halka", circle))
        .push(inner_row(w.inner))
        .into()
}

fn inverted<'a>(w: &LayerStyleWindow, env: &Env<'_>) -> Element<'a, Message> {
    let r = &w.drafts.inverted;
    let merged = r.merge == Some(true);
    panel("Katmanın alanlarının dışı dolgu sembolüyle boyanır; alanlar boş kalır, kenarları sembolün çizgileriyle çizilir. Çalışma alanının dışını örtmek için.")
        .push(line(
            "Dolgu",
            vec![slots_in(env, SetAt::Inverted, &r.symbols, "Ters alan", &[GeometryClass::Fill])],
        ))
        .push(line(
            "Örtüşenler",
            vec![
                switch(merged, "Boş kalsın", th(Edit::Merge(!merged))),
                muted("kapalıyken iki alanın örtüştüğü yer yeniden boyanır"),
            ],
        ))
        .into()
}
