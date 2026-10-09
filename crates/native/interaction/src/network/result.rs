//! The layers the network tools write their results to (docs/adr/0209 §5–§7;
//! the web's `tools/resultLayer.ts`): Rota, Hizmet alanı and its lines, En
//! yakın tesis. A layer of that name (compared trimmed and Turkish-folded, as
//! İşlemler's new layers are) is written to; without one it is opened, with
//! its own look, at the top of the tree, in the write's undo step. The objects
//! take the layer's look (no colour of their own).

use std::collections::BTreeMap;

use kentos_contracts::{EntityGeometry, LineType};
use kentos_domain::NewLayer;
use kentos_geometry_core::ops::network::Line;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::{ExecutionContext, create};

use crate::log::Level;
use crate::points::{wire, written};
use crate::prompt::fold_tr;
use crate::tool::Context;

/// A result layer: its name and look.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResultLayer {
    pub name: &'static str,
    pub color: &'static str,
    pub weight: f64,
    pub fill: Option<&'static str>,
}

pub const ROUTE_LAYER: ResultLayer = ResultLayer {
    name: "Rota",
    color: "#D32F2F",
    weight: 0.5,
    fill: None,
};
pub const AREA_LAYER: ResultLayer = ResultLayer {
    name: "Hizmet alanı",
    color: "#1976D2",
    weight: 0.25,
    fill: Some("#1976D233"),
};
pub const AREA_LINES_LAYER: ResultLayer = ResultLayer {
    name: "Hizmet alanı çizgileri",
    color: "#1565C0",
    weight: 0.35,
    fill: None,
};
pub const CLOSEST_LAYER: ResultLayer = ResultLayer {
    name: "En yakın tesis",
    color: "#7B1FA2",
    weight: 0.5,
    fill: None,
};

/// One layer's objects of a write.
pub struct ResultWrite<'a> {
    pub layer: &'a ResultLayer,
    pub objects: Vec<(EntityGeometry, BTreeMap<String, String>)>,
}

/// A way as a polyline's geometry: its points, and its bulges when it has an arc.
pub fn line_geometry(l: &Line) -> EntityGeometry {
    let arcs = l.bulges.iter().any(|b| *b != 0.0);
    EntityGeometry::Polyline {
        pts: l
            .pts
            .iter()
            .map(|p| wire(crate::Vec2::new(p[0], p[1])))
            .collect(),
        bulges: arcs.then(|| l.bulges.clone()),
        zs: None,
        parts: None,
    }
}

/// The drawing's layer of a result layer's name, if it has one.
fn layer_of(name: &str, cx: &Context<'_>) -> Option<String> {
    let key = fold_tr(js_trim(name));
    cx.doc
        .layers()
        .leaves()
        .into_iter()
        .find(|l| fold_tr(js_trim(&l.name)) == key)
        .map(|l| l.id.clone())
}

/// The id a result layer opens with: `ag-` and its folded name, numbered when taken.
fn fresh_id(name: &str, cx: &Context<'_>) -> String {
    let folded = fold_tr(js_trim(name)).to_lowercase();
    let mut base = String::from("ag-");
    let mut dash = false;
    for c in folded.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            base.push(c);
            dash = false;
        } else if !dash {
            base.push('-');
            dash = true;
        }
    }
    let mut id = base.clone();
    let mut k = 2;
    while cx.doc.layers().get(&id).is_some() {
        id = format!("{base}-{k}");
        k += 1;
    }
    id
}

/// Writes the objects to their result layers in one undo step named `step`, opening the layers the drawing lacks
/// in it; nothing stays when one is refused (its words are said). The objects written, or none.
pub fn write_results(
    step: &str,
    writes: &[ResultWrite<'_>],
    cx: &mut Context<'_>,
) -> Option<usize> {
    let group = cx.doc.begin_group(step);
    let mut opened: Vec<&str> = Vec::new();
    let mut count = 0;
    for w in writes {
        if w.objects.is_empty() {
            continue;
        }
        let layer_id = match layer_of(w.layer.name, cx) {
            Some(id) => id,
            None => {
                let id = fresh_id(w.layer.name, cx);
                let mut layer = NewLayer::layer(w.layer.name);
                layer.id = Some(id.clone());
                layer.style.color = w.layer.color.to_owned();
                layer.style.line_type = LineType::Continuous;
                layer.style.line_weight = w.layer.weight;
                layer.style.fill = w.layer.fill.map(str::to_owned);
                if let Err(refusal) = cx.doc.add_layer(layer, None, false) {
                    cx.doc.cancel_group(group);
                    cx.say(Level::Warn, refusal.to_string());
                    return None;
                }
                opened.push(w.layer.name);
                id
            }
        };
        let input = kentos_contracts::EntitiesCreate {
            layer_id,
            objects: w
                .objects
                .iter()
                .map(|(geometry, attrs)| kentos_contracts::NewObject {
                    geometry: geometry.clone(),
                    color: None,
                    line_weight: None,
                    attrs: Some(attrs.clone()),
                    label: None,
                    label_of: None,
                    label_scale: None,
                    symbol: None,
                })
                .collect(),
            operation: None,
            expected_revision: None,
        };
        let result = create::execute(&mut ExecutionContext::new(cx.doc), input);
        match written(result, cx) {
            Some(out) => count += out.ids.len(),
            None => {
                cx.doc.cancel_group(group);
                return None;
            }
        }
    }
    cx.doc.end_group(group);
    for name in opened {
        cx.say(
            Level::Info,
            format!(
                "“{name}” katmanı çizimde yoktu; {} için açıldı.",
                crate::prompt::lower_tr(step)
            ),
        );
    }
    Some(count)
}
