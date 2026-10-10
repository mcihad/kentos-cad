//! Expressions over objects (the web's `model/expression/expression.ts`):
//! the style core's language (`kentos_style_core::expr`), evaluated for each
//! object with what it reads of it, and the dialog's one-line preview.

use kentos_contracts::Entity;
use kentos_expression::exec::{Slot, Source};
use kentos_expression::geometry::Shapes;
use kentos_expression::host::objects_source_of;
use kentos_expression::world::{LayerObjects, World};
use kentos_expression::{Builtin, FieldType, Objects};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::store::Store;
use kentos_geometry_core::store::draw::measure_record;
use kentos_native_application::geometry::shape;
/// The language's variables and functions with their help: the expression
/// field's Değişkenler and İşlevler menus (the web's `exprCatalog`).
pub use kentos_style_core::expr::library::{FUNCTIONS, FuncDef, VARIABLES, VarDef};
use kentos_style_core::expr::rows::{As, MEASURE_STRIDE};
use kentos_style_core::expr::{Expr, Geometry, Value};
use std::cell::OnceCell;

use crate::features::kind_label;
use crate::types::Returns;

/// Corners of a path or area (holes included, every part of a multi-part
/// area: docs/adr/0143), for `$köşe`.
pub fn vertex_count(e: &Entity) -> Option<f64> {
    let ring = |pts: usize, holes: &Option<Vec<kentos_contracts::RingGeometry>>| {
        pts + holes.iter().flatten().map(|h| h.pts.len()).sum::<usize>()
    };
    let n = match e {
        Entity::Polyline(p) => p.pts.len(),
        Entity::Polygon(p) => {
            ring(p.pts.len(), &p.holes)
                + p.parts
                    .iter()
                    .flatten()
                    .map(|q| ring(q.pts.len(), &q.holes))
                    .sum::<usize>()
        }
        Entity::Line(_) => 2,
        Entity::Spline(s) => s.pts.len(),
        Entity::Point(_) => 1,
        _ => return None,
    };
    Some(n as f64)
}

/// The geometry values of objects without a store: the core's, one object
/// at a time (the web's `measuresOf`).
pub fn measures_of(list: &[&Entity]) -> Vec<f64> {
    let mut out = Vec::with_capacity(list.len() * MEASURE_STRIDE);
    for e in list {
        measure_record(Some(&shape(e)), &mut out);
    }
    out
}

/// Objects as the column engine reads them (docs/adr/0100): their
/// attributes, kinds, layers' names, labels and ids, and their geometry
/// values from their shapes (the store's when there is one, else made once).
/// A world's layer is read the same way (docs/adr/0214 §3).
pub struct EntityObjects<'a> {
    list: Vec<&'a Entity>,
    layers: Vec<String>,
    store: Option<&'a Store>,
    /// The shapes made here when no store holds them, by place.
    made: OnceCell<Vec<Shape>>,
}

impl<'a> EntityObjects<'a> {
    /// `layer_name` names an object's layer; `store` holds their shapes, when one does.
    pub fn new(
        list: &[&'a Entity],
        layer_name: &dyn Fn(&str) -> String,
        store: Option<&'a Store>,
    ) -> Self {
        let mut names: Vec<(&str, String)> = Vec::new();
        let layers = list
            .iter()
            .map(|e| {
                let id = e.base().layer_id.as_str();
                match names.iter().find(|(l, _)| *l == id) {
                    Some((_, n)) => n.clone(),
                    None => {
                        let n = layer_name(id);
                        names.push((id, n.clone()));
                        n
                    }
                }
            })
            .collect();
        Self {
            list: list.to_vec(),
            layers,
            store,
            made: OnceCell::new(),
        }
    }

    pub fn ids(&self) -> Vec<f64> {
        self.list.iter().map(|e| f64::from(e.base().id)).collect()
    }

    fn shape(&self, i: usize) -> Option<&Shape> {
        let e = self.list.get(i)?;
        if let Some(store) = self.store
            && let Some(it) = store.get(f64::from(e.base().id))
        {
            return Some(&it.shape);
        }
        self.made
            .get_or_init(|| self.list.iter().map(|e| shape(e)).collect())
            .get(i)
    }
}

/// The objects as the engine reads them for as long as `'s`: their texts
/// and their layers' names borrowed for it.
#[derive(Clone, Copy)]
pub struct View<'s, 'a>(pub &'s EntityObjects<'a>);

impl<'s, 'a: 's> Objects<'s> for View<'s, 'a> {
    fn len(&self) -> usize {
        self.0.list.len()
    }

    fn field(&self, name: &str, _ty: FieldType, start: usize, mut slot: Slot<'_, 's>) {
        for i in 0..slot.len() {
            let v = self
                .0
                .list
                .get(start + i)
                .and_then(|e| e.base().attrs.get(name))
                .map(String::as_str);
            slot.text(i, v);
        }
    }

    fn geometry(&self, what: Geometry, start: usize, slot: Slot<'_, 's>) {
        if what == Geometry::Vertices {
            let mut slot = slot;
            for i in 0..slot.len() {
                slot.number(i, self.0.list.get(start + i).and_then(|e| vertex_count(e)));
            }
            return;
        }
        Shapes::new(|i| self.0.shape(i)).fill(what, start, slot);
    }

    fn builtin(&self, what: Builtin, start: usize, mut slot: Slot<'_, 's>) {
        for i in 0..slot.len() {
            let Some(e) = self.0.list.get(start + i) else {
                slot.text(i, None);
                continue;
            };
            match what {
                Builtin::Kind => slot.text(i, Some(kind_label(e.kind()))),
                Builtin::Layer => slot.text(i, self.0.layers.get(start + i).map(String::as_str)),
                Builtin::Label => slot.text(i, e.base().label.as_deref()),
                Builtin::Id => slot.number(i, Some(f64::from(e.base().id))),
                Builtin::Scale => slot.number(i, None),
            }
        }
    }
}

impl LayerObjects for EntityObjects<'_> {
    fn source<'s>(&'s self, e: &'s Expr) -> Box<dyn Source<'s> + 's> {
        Box::new(objects_source_of(e, View(self)))
    }
}

/// What an evaluation reads besides the objects: their layers' names, the
/// store their shapes are in, and the world calls to other objects are
/// answered from (docs/adr/0214 §3).
pub struct Evaluation<'a> {
    pub layer_name: &'a dyn Fn(&str) -> String,
    pub store: Option<&'a Store>,
    pub world: Option<&'a World<'a>>,
}

/// An expression's value for each object, as `want` asks, a batch at a time
/// (the column engine), with the world when the expression looks at other objects.
pub fn evaluate_in(
    expr: &Expr,
    list: &[&Entity],
    how: &Evaluation,
    want: As,
) -> Vec<Value<'static>> {
    let objects = EntityObjects::new(list, how.layer_name, how.store);
    expr.evaluate_objects_in(&View(&objects), want, how.world)
        .values()
}

/// An expression's value for each object, as `want` asks (no store, no world).
/// `measures` is no longer read: the geometry values come from the shapes.
pub fn evaluate_all(
    expr: &Expr,
    list: &[&Entity],
    layer_name: &dyn Fn(&str) -> String,
    _measures: &mut dyn FnMut() -> Vec<f64>,
    want: As,
) -> Vec<Value<'static>> {
    evaluate_in(
        expr,
        list,
        &Evaluation {
            layer_name,
            store: None,
            world: None,
        },
        want,
    )
}

/// The dialog's line for an expression on the objects it will read:
/// "12 / 68 nesne koşulu sağlıyor." or the first value (the web's `previewExpression`).
pub fn preview_expression(
    expr: &Expr,
    entities: &[&Entity],
    returns: Returns,
    how: &Evaluation,
) -> String {
    let Some(first_object) = entities.first() else {
        return "Önizleme için uygun nesne yok.".into();
    };
    let missing: Vec<String> = expr
        .fields
        .iter()
        .filter(|f| !entities.iter().any(|e| e.base().attrs.contains_key(*f)))
        .map(|f| format!("“{f}”"))
        .collect();
    let note = if missing.is_empty() {
        String::new()
    } else {
        format!(" {} alanı bu nesnelerde yok.", missing.join(", "))
    };
    let list = &entities[..entities.len().min(20_000)];
    if returns == Returns::Condition {
        let values = evaluate_in(expr, list, how, As::Bool);
        let hits = values.iter().filter(|v| **v == Value::Bool(true)).count();
        return format!("{hits} / {} nesne koşulu sağlıyor.{note}", list.len());
    }
    let values = evaluate_in(expr, list, how, As::Text);
    let empty = values.iter().filter(|v| **v == Value::Null).count();
    let who = first_object
        .base()
        .label
        .as_deref()
        .filter(|l| !l.is_empty())
        .map(|l| format!(" ({l})"))
        .unwrap_or_default();
    let first = match values.first() {
        Some(Value::Text(t)) => format!("“{t}”"),
        _ => "boş".into(),
    };
    let empties = if empty > 0 {
        format!(" {empty} nesnede sonuç boş.")
    } else {
        String::new()
    };
    format!("İlk nesnede{who}: {first}.{empties}{note}")
}
