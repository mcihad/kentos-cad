//! Katman süzgeci (docs/adr/0211 §3): a layer's filter compiled and
//! evaluated over its objects, as İfadeyle seç evaluates a condition: the
//! language's column engine over a table of what the condition reads of each
//! object, its geometry values from the object's shape. An object passes when
//! the condition is true for it (false, empty or an error: it does not) and,
//! with a list, when the list names its persistent id. The desktop's geometry
//! store keeps the answers (`kentos_interaction::spatial`), the command counts
//! with them; the web's is `apps/web/src/model/layerFilter.ts`.

use std::collections::HashSet;

use kentos_commands::{Entity, EntityId, LayerFilter};
use kentos_expression::rows::{As, RowsInput, evaluate_rows_on, evaluate_rows_on_with};
use kentos_expression::{Expr, Schema, Unit, compile_with};
use kentos_geometry_core::entity::Shape;

/// A filter ready to ask.
#[derive(Clone, Debug)]
pub struct CompiledFilter {
    expr: Option<Expr>,
    objects: Option<HashSet<EntityId>>,
}

/// A filter compiled, its condition in `unit` (a filter compares readings:
/// the project's drawing unit, `expression_unit`); the condition's error as
/// the dialog says it (“12. karakterde: …”). `$sıra` and `$ölçek` are
/// refused: a filter must not change with the run or the drawing's scale.
pub fn compile_filter(filter: &LayerFilter, unit: Unit) -> Result<CompiledFilter, String> {
    let expr = match &filter.expression {
        None => None,
        Some(text) => {
            // The extensions' functions the host offers are known, as in a label (docs/adr/8057 §8): answered
            // without waiting, the filter worked out again when the answers come (`Spatial::sync`).
            let schema = Schema {
                unit,
                externals: crate::label_texts::external_defs(),
                ..Schema::default()
            };
            let e = compile_with(text, &schema).map_err(|e| e.text())?;
            if e.needs.index {
                return Err(
                    "Süzgeçte $sıra kullanılamaz: süzgeç çalıştırmaya göre değişmemeli.".into(),
                );
            }
            if e.needs.scale {
                return Err(
                    "Süzgeçte $ölçek kullanılamaz: süzgeç çizimin ölçeğine göre değişmemeli."
                        .into(),
                );
            }
            Some(e)
        }
    };
    let objects = (!filter.objects.is_empty()).then(|| filter.objects.iter().copied().collect());
    Ok(CompiledFilter { expr, objects })
}

/// The kind of object as the language names it (`$tür`; the web's `ENTITY_KIND_LABEL`).
pub use kentos_forms::kind_label;

/// Corners of a path or area, holes and every part's included (`$köşe`).
pub fn vertex_count(e: &Entity) -> Option<f64> {
    let ring = |pts: usize, holes: &Option<Vec<kentos_commands::RingGeometry>>| {
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
        Entity::Leader(l) => l.pts.len(),
        _ => return None,
    };
    Some(n as f64)
}

impl CompiledFilter {
    /// Its condition compiled (what a source can be sent of it, docs/adr/0581 §3).
    pub fn expr(&self) -> Option<&Expr> {
        self.expr.as_ref()
    }

    /// The objects its list names, by persistent id; none without a list.
    pub fn listed(&self) -> Option<&HashSet<EntityId>> {
        self.objects.as_ref()
    }

    /// Whether it reads the objects' geometry (their shapes are asked).
    pub fn reads_geometry(&self) -> bool {
        self.expr.as_ref().is_some_and(|e| {
            e.needs.measured || e.needs.centroid || e.needs.bounds || e.needs.vertices
        })
    }

    /// Whether it has a condition (without one, the list alone decides).
    pub fn has_condition(&self) -> bool {
        self.expr.is_some()
    }

    /// For each object (with its persistent id), whether it passes:
    /// `layer_name` names a layer by its id (`$katman`), `shape(i)` is object
    /// `i`'s shape when the condition reads geometry.
    pub fn passes<'s>(
        &self,
        list: &[(&Entity, EntityId)],
        layer_name: &dyn Fn(&str) -> String,
        shape: impl Fn(usize) -> Option<&'s Shape>,
    ) -> Vec<bool> {
        let mut out: Vec<bool> = match &self.objects {
            Some(ids) => list.iter().map(|(_, id)| ids.contains(id)).collect(),
            None => vec![true; list.len()],
        };
        if self.expr.is_none() {
            return out;
        }
        // Only the objects the list let through are asked.
        let asked: Vec<usize> = (0..list.len()).filter(|i| out[*i]).collect();
        let entities: Vec<&Entity> = asked.iter().map(|&i| list[i].0).collect();
        let met = self.condition(&entities, layer_name, |k| shape(asked[k]));
        for (k, &i) in asked.iter().enumerate() {
            out[i] = met[k];
        }
        out
    }

    /// For each object, whether the condition holds for it, the list aside
    /// (all of them without a condition): `layer_name` names a layer by its
    /// id (`$katman`), `shape(i)` is object `i`'s shape when the condition
    /// reads geometry.
    pub fn condition<'s>(
        &self,
        list: &[&Entity],
        layer_name: &dyn Fn(&str) -> String,
        shape: impl Fn(usize) -> Option<&'s Shape>,
    ) -> Vec<bool> {
        let Some(e) = &self.expr else {
            return vec![true; list.len()];
        };
        if list.is_empty() {
            return Vec::new();
        }
        let needs = e.needs;
        let mut texts = String::new();
        let mut lens: Vec<i32> = Vec::with_capacity(list.len() * (e.fields.len() + 3));
        let mut numbers: Vec<f64> = Vec::new();
        let put = |texts: &mut String, lens: &mut Vec<i32>, v: Option<&str>| match v {
            None => lens.push(-1),
            Some(v) => {
                texts.push_str(v);
                lens.push(v.encode_utf16().count() as i32);
            }
        };
        for o in list {
            let base = o.base();
            for f in &e.fields {
                put(&mut texts, &mut lens, base.attrs.get(f).map(String::as_str));
            }
            if needs.label {
                put(&mut texts, &mut lens, base.label.as_deref());
            }
            if needs.layer {
                let name = layer_name(&base.layer_id);
                put(&mut texts, &mut lens, Some(&name));
            }
            if needs.kind {
                put(&mut texts, &mut lens, Some(kind_label(o)));
            }
            if needs.id {
                numbers.push(f64::from(base.id));
            }
            if needs.vertices {
                numbers.push(vertex_count(o).unwrap_or(f64::NAN));
            }
        }
        let input = RowsInput {
            n: list.len(),
            texts: &texts,
            text_lens: &lens,
            numbers: &numbers,
            measures: &[],
            scale: f64::NAN,
        };
        // A table that does not read (it cannot: it was built for this
        // condition) lets nothing through rather than everything. An extension's function is answered by the host
        // without waiting: an answer not had yet is empty (the object left out until it comes).
        let calls = (!e.externals().is_empty())
            .then(crate::label_texts::external_calls)
            .flatten();
        let read = match &calls {
            Some(c) => {
                let c: &dyn kentos_expression::external::ExternalCalls = &**c;
                evaluate_rows_on_with(e, &input, As::Bool, shape, Some(c))
            }
            None => evaluate_rows_on(e, &input, As::Bool, shape),
        };
        let Ok(column) = read else {
            return vec![false; list.len()];
        };
        // 3: a true/false value, and 1 in its number when true.
        (0..list.len())
            .map(|k| column.kinds.get(k) == Some(&3) && column.numbers.get(k) == Some(&1.0))
            .collect()
    }
}

/// How many of `layer`'s objects its filter lets through (docs/adr/0651 §2); none when the layer has no filter.
pub fn passing(doc: &kentos_domain::Document, layer: &str) -> Option<u32> {
    doc.layers().get(layer)?.filter.as_ref()?;
    let out = left_out(doc);
    let n = doc
        .by_layer_with_uids(layer)
        .filter(|(_, e)| !out.contains(&kentos_domain::Slot(e.base().id)))
        .count();
    Some(u32::try_from(n).unwrap_or(u32::MAX))
}

/// The objects of `doc` their layers' filters leave out (docs/adr/0211 §1):
/// what İşlemler, Öznitelik tablosu and Veride ara do not see, asked of the
/// document itself (a run's copy has no geometry store). A layer whose
/// condition does not compile leaves out all its objects.
pub fn left_out(doc: &kentos_domain::Document) -> HashSet<kentos_domain::Slot> {
    let unit = crate::expression_unit::reading_unit(doc.settings());
    let mut out = HashSet::new();
    for node in doc.layers().leaves() {
        let Some(filter) = &node.filter else {
            continue;
        };
        if node.service.is_some() {
            continue;
        }
        let list: Vec<(&Entity, EntityId)> = doc
            .by_layer_with_uids(&node.id)
            .map(|(uid, e)| (e, EntityId(*uid.as_bytes())))
            .collect();
        let pass = match compile_filter(filter, unit) {
            Err(_) => vec![false; list.len()],
            Ok(c) => {
                let shapes: Vec<Shape> = if c.reads_geometry() {
                    list.iter()
                        .map(|(e, _)| crate::geometry::shape(e))
                        .collect()
                } else {
                    Vec::new()
                };
                let names = |id: &str| {
                    doc.layers()
                        .get(id)
                        .map_or_else(|| id.to_owned(), |n| n.name.clone())
                };
                c.passes(&list, &names, |i| shapes.get(i))
            }
        };
        for ((e, _), p) in list.iter().zip(pass) {
            if !p {
                out.insert(kentos_domain::Slot(e.base().id));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use kentos_commands::{Entity, LayerFilter};
    use kentos_expression::Unit;
    use kentos_expression::external::{Arg, ExternalCalls, ExternalDef};

    use super::compile_filter;

    const NAME: &str = "ext.tr.ornek.imar-denetim.ada-parsel";

    /// The host: "ada/parsel" once it has answered, empty before.
    struct Host(AtomicBool);

    impl ExternalCalls for Host {
        fn call(&self, name: &str, rows: &[Vec<Arg>]) -> Vec<Arg> {
            assert_eq!(name, NAME);
            rows.iter()
                .map(|r| match (&r[0], &r[1], self.0.load(Ordering::Relaxed)) {
                    (Arg::Text(a), Arg::Text(p), true) => Arg::Text(format!("{a}/{p}")),
                    _ => Arg::Null,
                })
                .collect()
        }
    }

    fn parcel(id: u32, ada: &str, parsel: &str) -> Entity {
        serde_json::from_value(serde_json::json!({
            "kind": "polygon", "id": id, "layerId": "parsel", "attrs": { "ada": ada, "parsel": parsel },
            "pts": [{"x": 0, "y": 0}, {"x": 10, "y": 0}, {"x": 10, "y": 10}]
        }))
        .expect("a parcel")
    }

    #[test]
    fn a_filter_calls_an_extensions_function_answered_without_waiting() {
        // docs/adr/8057 §8: the host answers what it has at once; before its answer the object is left out, after
        // it the filter lets the object it holds for through (Spatial works the filter out again when answers come).
        let host = Arc::new(Host(AtomicBool::new(false)));
        crate::label_texts::set_externals(
            vec![ExternalDef {
                name: NAME.into(),
                description: "Ada/parsel".into(),
            }],
            host.clone(),
        );
        let filter = LayerFilter {
            expression: Some(format!("{NAME}(ada, parsel) = '101/2'")),
            objects: Vec::new(),
        };
        let compiled =
            compile_filter(&filter, Unit::METRE).expect("it compiles with the function offered");
        let (a, b) = (parcel(1, "101", "1"), parcel(2, "101", "2"));
        let names = |id: &str| id.to_owned();
        let passes = || compiled.condition(&[&a, &b], &names, |_| None);
        assert_eq!(passes(), [false, false], "not answered yet: left out");
        host.0.store(true, Ordering::Relaxed);
        assert_eq!(passes(), [false, true], "answered: 101/2 passes");
    }
}
