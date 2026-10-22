//! Plot ink is derived output data. It never rewrites model colours, layers or paper items.
use crate::{
    display::{DisplayList, Prim},
    pdf::{MapContent, MapStroke},
    style::Stroke,
};
use super::{PlotColor, PlotProfile, PlotStyle, StyleKey, Target};
use std::collections::BTreeMap;
/// Resolve once per page/layer pass; no per-primitive scan of all assignments.
pub struct Resolved<'a> {
    paper: BTreeMap<&'a str, &'a PlotStyle>,
    model: BTreeMap<&'a str, &'a PlotStyle>,
    default: Option<&'a PlotStyle>,
}
impl<'a> Resolved<'a> {
    pub fn new(profile: &'a PlotProfile) -> Self {
        let mut indexed = BTreeMap::new();
        let mut named = BTreeMap::new();
        for e in &profile.entries {
            match &e.key {
                StyleKey::Aci { index } => {
                    indexed.insert(*index, &e.style);
                }
                StyleKey::Named { name } => {
                    named.insert(name.as_str(), &e.style);
                }
            }
        }
        let default = profile
            .default_style
            .as_deref()
            .and_then(|n| named.get(n).copied());
        let mut out = Self {
            paper: BTreeMap::new(),
            model: BTreeMap::new(),
            default,
        };
        for a in &profile.assignments {
            let style = match &a.key {
                StyleKey::Aci { index } => indexed.get(index).copied(),
                StyleKey::Named { name } => named.get(name.as_str()).copied(),
            };
            if let Some(s) = style {
                match &a.target {
                    Target::PaperItem { id } => {
                        out.paper.insert(id, s);
                    }
                    Target::ModelLayer { id } => {
                        out.model.insert(id, s);
                    }
                }
            }
        }
        out
    }
    pub fn paper(&self, id: &str) -> Option<&PlotStyle> {
        self.paper.get(id).copied().or(self.default)
    }
    pub fn explicit_paper(&self, id: &str) -> Option<&PlotStyle> {
        self.paper.get(id).copied()
    }
    pub fn model(&self, id: &str) -> Option<&PlotStyle> {
        self.model.get(id).copied().or(self.default)
    }
}
/// sRGB luminance coefficients and white-paper screening; source alpha stays separate from ink coverage.
/// Paper white is a knockout/mask and is never turned into black ink.
pub fn color(source: &str, style: &PlotStyle) -> String {
    let Some(h) = source.strip_prefix('#') else {
        return source.into();
    };
    if ![6, 8].contains(&h.len()) || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
        return source.into();
    }
    let mut rgba = [255_u8; 4];
    for (i, v) in rgba.iter_mut().enumerate().take(h.len() / 2) {
        let Ok(n) = u8::from_str_radix(&h[i * 2..i * 2 + 2], 16) else {
            return source.into();
        };
        *v = n;
    }
    if rgba[..3] == [255, 255, 255] {
        return source.into();
    }
    let mut rgb = [f64::from(rgba[0]), f64::from(rgba[1]), f64::from(rgba[2])];
    match &style.color {
        PlotColor::Object => {}
        PlotColor::Black => rgb.fill(0.),
        PlotColor::Grayscale => {
            let g = rgb[0] * 0.2126 + rgb[1] * 0.7152 + rgb[2] * 0.0722;
            rgb.fill(g);
        }
        PlotColor::Color { value } => {
            if !crate::style::is_color(value) {
                return source.into();
            }
            if let Some(h) = value.strip_prefix('#') {
                for (i, v) in rgb.iter_mut().enumerate() {
                    if let Ok(n) = u8::from_str_radix(&h[i * 2..i * 2 + 2], 16) {
                        *v = f64::from(n);
                    }
                }
                if h.len() == 8
                    && let Ok(n) = u8::from_str_radix(&h[6..8], 16)
                {
                    rgba[3] = n;
                }
            }
        }
    }
    let coverage = f64::from(style.screening.min(100)) / 100.;
    for (i, v) in rgb.iter().enumerate() {
        rgba[i] = (255. - (255. - v) * coverage).round().clamp(0., 255.) as u8;
    }
    if let Some(alpha) = style.opacity {
        rgba[3] = ((u16::from(alpha.min(100)) * 255 + 50) / 100) as u8;
    }
    let mut out = format!("#{:02x}{:02x}{:02x}", rgba[0], rgba[1], rgba[2]);
    if rgba[3] < 255 {
        out.push_str(&format!("{:02x}", rgba[3]));
    }
    out
}
/// PDF/SVG dash semantics: an odd list repeats once with alternating roles; zero-length ink with round caps makes dots.
pub fn dash_um(values: &[i32]) -> Vec<i32> {
    let mut out = values.to_vec();
    if out.len() % 2 == 1 {
        out.extend_from_slice(values);
    }
    out
}
pub fn stroke(s: &mut Stroke, style: &PlotStyle) {
    s.color = color(&s.color, style);
    if let Some(w) = style.line_weight {
        s.width = w;
    }
    if let Some(d) = &style.dash {
        s.dash = dash_um(d);
    }
    if let Some(c) = style.cap {
        s.cap = c;
    }
    if let Some(j) = style.join {
        s.join = j;
    }
}
pub fn map_stroke(s: &mut MapStroke, style: &PlotStyle) {
    s.color = color(&s.color, style);
    if let Some(w) = style.line_weight {
        s.width = f64::from(w) / 1000.;
    }
    if let Some(d) = &style.dash {
        s.dash = dash_um(d).iter().map(|v| f64::from(*v) / 1000.).collect();
    }
    if let Some(c) = style.cap {
        s.cap = c;
    }
    if let Some(j) = style.join {
        s.join = j;
    }
}
pub(crate) fn paper(
    list: &mut DisplayList,
    profile: &PlotProfile,
    materialized: &std::collections::BTreeSet<String>,
) {
    let resolved = Resolved::new(profile);
    for p in &mut list.prims {
        let Some(original) = resolved.paper(p.item()) else {
            continue;
        };
        let overridden = materialized.contains(p.item());
        let mut style = original.clone();
        if overridden && style.opacity.is_some() {
            style.opacity = Some(100);
        }
        let style = &style;
        match p {
            Prim::Rect(r) => {
                if let Some(c) = &mut r.fill {
                    *c = color(c, style);
                }
                if let Some(s) = &mut r.stroke {
                    stroke(s, style);
                }
            }
            Prim::Path(r) => {
                if let Some(c) = &mut r.fill {
                    *c = color(c, style);
                }
                if let Some(s) = &mut r.stroke {
                    stroke(s, style);
                }
            }
            Prim::Text(t) => t.color = color(&t.color, style),
            Prim::Image(i) if !overridden => {
                if let Some(alpha) = style.opacity {
                    i.opacity = alpha;
                }
            }
            // Image pixels, halos, knockout white and container opacity keep their own semantics.
            _ => {}
        }
    }
}
pub fn map(content: &mut MapContent, profile: &PlotProfile) {
    let MapContent::Vector(v) = content else {
        return;
    };
    let resolved = Resolved::new(profile);
    for layer in &mut v.layers {
        if let Some(style) = resolved.model(&layer.id) {
            for p in &mut layer.paths {
                if let Some(c) = &mut p.fill {
                    *c = color(c, style);
                }
                if let Some(s) = &mut p.stroke {
                    map_stroke(s, style);
                }
            }
            for t in &mut layer.texts {
                t.color = color(&t.color, style);
            }
        }
    }
}
