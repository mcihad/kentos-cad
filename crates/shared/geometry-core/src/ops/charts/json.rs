//! The charts' words across the WASM boundary (the web's `ChartWork`): a
//! spec, a frame and a palette read; summaries, details, a model, a layout
//! (its bulk points as a typed array apart) and a tooltip written.

use std::fmt::Write;

use super::fields::{Detail, Summary};
use super::layout::{Align, Baseline, Frame, Layout, Prim, Role};
use super::model::{Aggregate, ChartKind, Model, Sort, Spec};
use super::svg::Palette;
use crate::api::json::{Json, to_string};

fn str_of(v: &Json) -> Option<String> {
    match v {
        Json::Str(s) => Some(s.clone()),
        _ => None,
    }
}

fn num_of(v: &Json) -> Option<f64> {
    match v {
        Json::Num(x) => Some(*x),
        _ => None,
    }
}

/// A spec from its JSON (`{"kind":"bar","category":"Kullanım",…}`); the
/// choices left out take their defaults.
pub fn spec_from_json(text: &str) -> Result<Spec, String> {
    let v = Json::parse(text)?;
    let kind = str_of(v.get("kind"))
        .and_then(|k| ChartKind::from_word(&k))
        .ok_or("grafiğin türü yok")?;
    let mut spec = Spec::new(kind);
    spec.category = str_of(v.get("category"));
    spec.value = str_of(v.get("value"));
    if let Some(a) = str_of(v.get("aggregate")).and_then(|a| Aggregate::from_word(&a)) {
        spec.aggregate = a;
    }
    if let Some(s) = str_of(v.get("sort")).and_then(|s| Sort::from_word(&s)) {
        spec.sort = s;
    }
    spec.bins = num_of(v.get("bins"))
        .filter(|b| *b >= 1.0)
        .map(|b| b as u32);
    if let Json::Bool(b) = v.get("guides") {
        spec.guides = *b;
    }
    spec.group = str_of(v.get("group"));
    spec.x = str_of(v.get("x"));
    if let Json::Arr(items) = v.get("y") {
        spec.y = items.iter().filter_map(str_of).collect();
    }
    spec.series = str_of(v.get("series"));
    if let Json::Bool(b) = v.get("trend") {
        spec.trend = *b;
    }
    Ok(spec)
}

/// A spec as JSON (the platforms keep the last one).
pub fn spec_json(s: &Spec) -> String {
    let mut out = String::from("{");
    let _ = write!(out, r#""kind":{}"#, to_string(s.kind.word()));
    for (name, v) in [
        ("category", &s.category),
        ("value", &s.value),
        ("group", &s.group),
        ("x", &s.x),
        ("series", &s.series),
    ] {
        if let Some(v) = v {
            let _ = write!(out, r#","{name}":{}"#, to_string(v));
        }
    }
    let _ = write!(
        out,
        r#","aggregate":{},"sort":{},"guides":{},"trend":{},"y":{}"#,
        to_string(s.aggregate.word()),
        to_string(s.sort.word()),
        s.guides,
        s.trend,
        to_string(&s.y)
    );
    if let Some(b) = s.bins {
        let _ = write!(out, r#","bins":{b}"#);
    }
    out.push('}');
    out
}

pub fn frame_from_json(text: &str) -> Result<Frame, String> {
    let v = Json::parse(text)?;
    Ok(Frame {
        width: num_of(v.get("width")).unwrap_or(0.0) as f32,
        height: num_of(v.get("height")).unwrap_or(0.0) as f32,
        font: num_of(v.get("font")).unwrap_or(12.0) as f32,
        family: str_of(v.get("family")).unwrap_or_default(),
    })
}

pub fn palette_from_json(text: &str) -> Result<Palette, String> {
    let v = Json::parse(text)?;
    let s = |k: &str| str_of(v.get(k)).unwrap_or_else(|| "#000000".to_owned());
    let series = match v.get("series") {
        Json::Arr(items) => items.iter().filter_map(str_of).collect(),
        _ => Vec::new(),
    };
    Ok(Palette {
        background: s("background"),
        surface: s("surface"),
        grid: s("grid"),
        axis: s("axis"),
        ink1: s("ink1"),
        ink2: s("ink2"),
        ink3: s("ink3"),
        series,
        other: s("other"),
        selection: s("selection"),
        font: str_of(v.get("font")).unwrap_or_else(|| "sans-serif".to_owned()),
    })
}

crate::json_struct!(out Summary {
    key,
    label,
    kind,
    objects,
    filled,
    empty,
    invalid,
    distinct,
    min,
    max,
    mean,
    sum
});

/// A field's statistics without its value rows' rows (asked one at a time).
pub fn detail_json(d: &Detail) -> String {
    let mut out = String::from("{");
    let _ = write!(out, r#""summary":{}"#, to_string(&d.summary));
    out.push_str(r#","stats":["#);
    for (i, s) in d.stats.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            r#"{{"label":{},"value":{}}}"#,
            to_string(&s.label),
            to_string(&s.value)
        );
    }
    out.push_str(r#"],"values":["#);
    for (i, v) in d.values.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            r#"{{"text":{},"count":{},"share":{},"mark":{}}}"#,
            to_string(&v.text),
            v.count,
            to_string(&v.share),
            to_string(v.mark.word())
        );
    }
    let _ = write!(
        out,
        r#"],"more":{},"notes":{}}}"#,
        d.more,
        to_string(&d.notes)
    );
    out
}

/// A model's words: its kind, titles, notes, table and how many elements it has.
pub fn model_json(m: &Model) -> String {
    format!(
        r#"{{"kind":{},"xTitle":{},"yTitle":{},"notes":{},"elements":{},"table":{{"columns":{},"rows":{}}}}}"#,
        to_string(m.kind.word()),
        to_string(&m.x_title),
        to_string(&m.y_title),
        to_string(&m.notes),
        m.elements.len(),
        to_string(&m.table.columns),
        to_string(&m.table.rows)
    )
}

/// A pixel value: two fraction digits are plenty.
fn px(out: &mut String, v: f32) {
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    out.push_str(if s == "-0" { "0" } else { s });
}

fn role(out: &mut String, r: Role) {
    let _ = write!(out, "\"{}\"", r.word());
}

/// The layout's primitives (its points apart: `Layout::points`).
pub fn layout_json(l: &Layout) -> String {
    let mut out = String::from(r#"{"plot":["#);
    for (i, v) in l.plot.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        px(&mut out, *v);
    }
    out.push_str(r#"],"prims":["#);
    for (i, p) in l.prims.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('{');
        let num = |out: &mut String, name: &str, v: f32| {
            let _ = write!(out, r#","{name}":"#);
            px(out, v);
        };
        match p {
            Prim::Rect {
                x,
                y,
                w,
                h,
                r_top,
                r_bottom,
                fill,
                alpha,
                elem,
            } => {
                out.push_str(r#""t":"rect""#);
                num(&mut out, "x", *x);
                num(&mut out, "y", *y);
                num(&mut out, "w", *w);
                num(&mut out, "h", *h);
                num(&mut out, "rt", *r_top);
                num(&mut out, "rb", *r_bottom);
                out.push_str(r#","fill":"#);
                role(&mut out, *fill);
                num(&mut out, "a", *alpha);
                let _ = write!(out, r#","e":{elem}"#);
            }
            Prim::Line {
                x1,
                y1,
                x2,
                y2,
                stroke,
                width,
                alpha,
            } => {
                out.push_str(r#""t":"line""#);
                num(&mut out, "x1", *x1);
                num(&mut out, "y1", *y1);
                num(&mut out, "x2", *x2);
                num(&mut out, "y2", *y2);
                out.push_str(r#","stroke":"#);
                role(&mut out, *stroke);
                num(&mut out, "w", *width);
                num(&mut out, "a", *alpha);
            }
            Prim::Poly {
                pts,
                closed,
                stroke,
                width,
                alpha,
            } => {
                out.push_str(r#""t":"poly","pts":["#);
                for (k, v) in pts.iter().enumerate() {
                    if k > 0 {
                        out.push(',');
                    }
                    px(&mut out, *v);
                }
                let _ = write!(out, r#"],"closed":{closed},"stroke":"#);
                role(&mut out, *stroke);
                num(&mut out, "w", *width);
                num(&mut out, "a", *alpha);
            }
            Prim::Circle {
                x,
                y,
                r,
                fill,
                stroke,
                width,
                alpha,
                elem,
            } => {
                out.push_str(r#""t":"circle""#);
                num(&mut out, "x", *x);
                num(&mut out, "y", *y);
                num(&mut out, "r", *r);
                if let Some(f) = fill {
                    out.push_str(r#","fill":"#);
                    role(&mut out, *f);
                }
                if let Some(s) = stroke {
                    out.push_str(r#","stroke":"#);
                    role(&mut out, *s);
                }
                num(&mut out, "w", *width);
                num(&mut out, "a", *alpha);
                let _ = write!(out, r#","e":{elem}"#);
            }
            Prim::Wedge {
                cx,
                cy,
                r0,
                r1,
                a0,
                a1,
                fill,
                alpha,
                elem,
            } => {
                out.push_str(r#""t":"wedge""#);
                num(&mut out, "cx", *cx);
                num(&mut out, "cy", *cy);
                num(&mut out, "r0", *r0);
                num(&mut out, "r1", *r1);
                let _ = write!(out, r#","a0":{a0},"a1":{a1},"fill":"#);
                role(&mut out, *fill);
                num(&mut out, "a", *alpha);
                let _ = write!(out, r#","e":{elem}"#);
            }
            Prim::Cross {
                x,
                y,
                size,
                stroke,
                width,
            } => {
                out.push_str(r#""t":"cross""#);
                num(&mut out, "x", *x);
                num(&mut out, "y", *y);
                num(&mut out, "s", *size);
                out.push_str(r#","stroke":"#);
                role(&mut out, *stroke);
                num(&mut out, "w", *width);
            }
            Prim::Text {
                x,
                y,
                text,
                size,
                weight,
                align,
                baseline,
                color,
            } => {
                out.push_str(r#""t":"text""#);
                num(&mut out, "x", *x);
                num(&mut out, "y", *y);
                let _ = write!(out, r#","text":{}"#, to_string(text));
                num(&mut out, "size", *size);
                let align = match align {
                    Align::Start => "start",
                    Align::Middle => "middle",
                    Align::End => "end",
                };
                let base = match baseline {
                    Baseline::Top => "top",
                    Baseline::Middle => "middle",
                    Baseline::Bottom => "bottom",
                };
                let _ = write!(
                    out,
                    r#","weight":{weight},"align":"{align}","baseline":"{base}","color":"#
                );
                role(&mut out, *color);
            }
            Prim::Points {
                start,
                count,
                r,
                ring,
                fill,
                alpha,
            } => {
                let _ = write!(out, r#""t":"points","start":{start},"count":{count}"#);
                num(&mut out, "r", *r);
                let _ = write!(out, r#","ring":{ring},"fill":"#);
                role(&mut out, *fill);
                num(&mut out, "a", *alpha);
            }
        }
        out.push('}');
    }
    out.push_str("]}");
    out
}

/// A layout with its bulk points inline (a saved picture's: drawn once).
pub fn layout_with_points_json(l: &Layout) -> String {
    let mut out = layout_json(l);
    out.pop();
    out.push_str(r#","points":["#);
    for (i, v) in l.points.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        px(&mut out, *v);
    }
    out.push_str("]}");
    out
}

/// A tooltip's lines: `[[label, value], …]`.
pub fn tip_json(lines: &[(String, String)]) -> String {
    let pairs: Vec<Vec<String>> = lines
        .iter()
        .map(|(l, v)| vec![l.clone(), v.clone()])
        .collect();
    to_string(&pairs)
}
