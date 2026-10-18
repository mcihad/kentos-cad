//! Constraints in other edits (docs/adr/0337): whether writes relax them, the solver's part as the contract's, and a
//! set without some of its constraints, their dimensions' parameters with them as a deletion takes them.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use kentos_contracts::{
    CadElement, ConstraintSet, Entity, EntityId, ParamUnits, SubPlace, param_expr_names,
    parse_param_expr,
};
use kentos_geometry_core::ops::constraints::{Element, PointAt};

use crate::constraints::{core_shape, in_parameter_unit, measure_with, value_text};
use crate::document::Document;
use crate::identity::Uuid;

/// Whether writes relax the constraints (AutoCAD's Shift while editing): the session's word, not the drawing's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Relax {
    /// Every write that changes constrained objects keeps their constraints, or writes nothing.
    #[default]
    Off,
    /// The next such write relaxes them, then `Off` again (the desktop's “Gevşeterek düzenle”).
    Next,
    /// Every such write relaxes them until set back (Python's `with doc.relaxed():`).
    On,
}

/// The contract's part for the solver's.
pub fn cad_element(e: &Element) -> CadElement {
    match *e {
        Element::Point(at) => CadElement::Point {
            place: match at {
                PointAt::Start => SubPlace::Start,
                PointAt::End => SubPlace::End,
                PointAt::Center => SubPlace::Center,
                PointAt::Node(part) => SubPlace::Node { part },
                PointAt::Vertex { part, ring, index } => SubPlace::Vertex { part, ring, index },
            },
        },
        Element::Segment { part, ring, index } => CadElement::Segment { part, ring, index },
        Element::Circle => CadElement::Circle,
    }
}

impl Document {
    /// What owns an object's geometry other than the constraints (docs/adr/0338 §2): an object that follows another
    /// (an associative hatch its boundary, a linked text its object, an associative dimension its points) or an
    /// associative array's item (the array places it). The constraints do not name such an object: the relation it
    /// follows is its one authority, and it follows the constrained objects after their solve. A hosted opening
    /// (`CAD-ARCH-13`) joins this list when it comes.
    pub fn governed(&self, e: &Entity) -> Option<&'static str> {
        match e {
            Entity::Hatch(h) if h.assoc.is_some() => {
                Some("ilişkili tarama sınırını izler; kısıtı sınıra koyun")
            }
            Entity::Text(t) if t.label_of.is_some() => {
                Some("bağlı yazı nesnesini izler; kısıtı nesneye koyun")
            }
            Entity::Dimension(d) if d.refs.is_some() => {
                Some("ilişkili ölçü bağlı olduğu nesneleri izler; ölçü kısıtı kullanın")
            }
            _ => {
                let uid = EntityId(
                    self.store
                        .get(crate::identity::Slot(e.base().id))?
                        .uid
                        .into_bytes(),
                );
                self.arrays
                    .iter()
                    .any(|a| a.items.iter().any(|i| i.uid == Some(uid)))
                    .then_some("ilişkili dizinin öğesi; yerini dizi verir (Diziyi düzenle)")
            }
        }
    }

    /// Whether writes relax the constraints.
    pub fn relax(&self) -> Relax {
        self.relax
    }

    /// Sets whether writes relax the constraints.
    pub fn set_relax(&mut self, relax: Relax) {
        self.relax = relax;
    }

    /// Whether this write relaxes the constraints: `On` stays, `Next` is used up.
    pub fn take_relax(&mut self) -> bool {
        match self.relax {
            Relax::Off => false,
            Relax::On => true,
            Relax::Next => {
                self.relax = Relax::Off;
                true
            }
        }
    }

    /// `set` without its constraints at `drop` (docs/adr/0334 §1's rule for a deleted object's): a dimension's
    /// parameter goes with it unless another parameter reads it; then it stays a user parameter with the value it had
    /// (a reference dimension's measured from the objects, or from `removed` for one the step deleted).
    pub fn constraints_without(&self, set: &ConstraintSet, drop: &[usize]) -> ConstraintSet {
        self.constraints_without_in(set, drop, &HashMap::new())
    }

    pub(crate) fn constraints_without_in(
        &self,
        set: &ConstraintSet,
        drop: &[usize],
        removed: &HashMap<EntityId, Arc<Entity>>,
    ) -> ConstraintSet {
        let mut next = set.clone();
        let (dropped, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut next.constraints)
            .into_iter()
            .enumerate()
            .partition(|(i, _)| drop.contains(i));
        next.constraints = kept.into_iter().map(|(_, c)| c).collect();
        let read: BTreeSet<String> = next
            .parameters
            .iter()
            .filter_map(|p| parse_param_expr(&p.expression).ok())
            .flat_map(|e| param_expr_names(&e))
            .collect();
        let units = ParamUnits::of(self.settings());
        for (_, c) in &dropped {
            let Some(name) = c.rule.parameter() else {
                continue;
            };
            if !read.contains(name) {
                next.parameters.retain(|p| p.name != name);
                continue;
            }
            let Some(p) = next.parameters.iter_mut().find(|p| p.name == name) else {
                continue;
            };
            if p.expression.is_empty() {
                let shape_of = |uid: EntityId| {
                    let e = match self
                        .store
                        .slot_of(Uuid::from_bytes(uid.0))
                        .and_then(|s| self.store.get(s))
                    {
                        Some(s) => s.entity.clone(),
                        None => removed.get(&uid)?.clone(),
                    };
                    core_shape(&e)
                };
                let v = measure_with(&c.rule, &shape_of)
                    .map_or(0.0, |v| in_parameter_unit(units, p.kind, v));
                p.expression = value_text(v);
            }
        }
        next
    }
}
