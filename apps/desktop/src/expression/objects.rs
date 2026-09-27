//! The objects the builder previews on (the web's `BuilderObjects`): the
//! expression's input objects in run order, one at a time, their geometry
//! values from the drawing's geometry store (every `$` value, the centroid
//! and the box too), and a field's values.

use kentos_contracts::Entity;
use kentos_domain::{Document, Slot};
use kentos_expression::{Geometry, Measured, Schema, Scope, Value, geometry};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::store::Store;
use kentos_processing::expression::vertex_count;
use kentos_processing::features::kind_label;

/// The objects an expression runs on, in run order (`$sıra` is the place + 1).
#[derive(Clone, Debug, Default)]
pub(crate) struct Objects {
    pub slots: Vec<Slot>,
}

/// One object as the expression reads it.
struct One<'a> {
    e: &'a Entity,
    fields: &'a [String],
    layer: String,
    index: usize,
    shape: Option<&'a Shape>,
}

impl One<'_> {
    fn geometry_value(&self, what: Geometry) -> Option<f64> {
        geometry::value(self.shape?, what)
    }
}

impl Scope for One<'_> {
    fn field(&self, i: usize) -> Option<&str> {
        let name = self.fields.get(i)?;
        self.e.base().attrs.get(name).map(String::as_str)
    }

    fn measured(&self) -> Measured {
        let anchor = self
            .geometry_value(Geometry::AnchorY)
            .zip(self.geometry_value(Geometry::AnchorX));
        Measured {
            length: self.geometry_value(Geometry::Length),
            area: self.geometry_value(Geometry::Area),
            anchor,
        }
    }

    fn vertices(&self) -> Option<f64> {
        vertex_count(self.e)
    }

    fn kind(&self) -> &str {
        kind_label(self.e.kind())
    }

    fn layer(&self) -> &str {
        &self.layer
    }

    fn label(&self) -> Option<&str> {
        self.e.base().label.as_deref()
    }

    fn index(&self) -> f64 {
        (self.index + 1) as f64
    }

    fn id(&self) -> f64 {
        f64::from(self.e.base().id)
    }

    fn scale(&self) -> Option<f64> {
        None
    }

    fn geometry(&self, what: Geometry) -> Option<f64> {
        match what {
            Geometry::Vertices => vertex_count(self.e),
            _ => self.geometry_value(what),
        }
    }
}

impl Objects {
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// The expression's value on object `i`, or why it has none (its error, “8. karakterde: …”).
    pub fn value(
        &self,
        source: &str,
        schema: &Schema,
        i: usize,
        doc: &Document,
        store: &Store,
    ) -> Result<Value<'static>, String> {
        let expr = kentos_expression::compile_with(source, schema).map_err(|e| e.text())?;
        let slot = *self.slots.get(i).ok_or_else(String::new)?;
        let e = doc.get(slot).ok_or_else(String::new)?;
        let layer = if expr.needs.layer {
            doc.layers()
                .get(&e.base().layer_id)
                .map_or_else(|| e.base().layer_id.clone(), |l| l.name.clone())
        } else {
            String::new()
        };
        let one = One {
            e,
            fields: &expr.fields,
            layer,
            index: i,
            shape: store.get(f64::from(slot.0)).map(|item| &item.shape),
        };
        Ok(expr.evaluate(&one).into_owned())
    }

    /// A field's distinct values in object order, at most `limit` of them when given.
    pub fn values(&self, field: &str, limit: Option<usize>, doc: &Document) -> Vec<String> {
        let mut seen: Vec<String> = Vec::new();
        let mut known = std::collections::HashSet::new();
        for e in self.slots.iter().filter_map(|s| doc.get(*s)) {
            let Some(v) = e.base().attrs.get(field) else {
                continue;
            };
            if known.insert(v.as_str()) {
                seen.push(v.clone());
                if limit.is_some_and(|n| seen.len() >= n) {
                    break;
                }
            }
        }
        seen
    }

    /// How the stepper names object `i`: “Kapalı alan 12”.
    pub fn describe(&self, i: usize, doc: &Document) -> String {
        let Some(e) = self.slots.get(i).and_then(|s| doc.get(*s)) else {
            return String::new();
        };
        match e.base().label.as_deref().filter(|l| !l.is_empty()) {
            Some(label) => format!("{} {label}", kind_label(e.kind())),
            None => kind_label(e.kind()).to_owned(),
        }
    }

    /// The place of an object among them.
    pub fn position(&self, slot: Slot) -> Option<usize> {
        self.slots.iter().position(|s| *s == slot)
    }
}
