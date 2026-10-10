//! Mekânsal ilişki (docs/adr/0214 §2.7): the evaluated object against
//! another layer's objects by Konuma göre seç's relations (ADR 0200:
//! `spatial_query::relate`, arcs exact, edges on the boundary count), its
//! nearest one and the overlaps (ADR 0201's overlay). Candidates come from
//! the geometry store's index by their boxes, then each is decided exactly;
//! the object itself is never one of them.

use std::rc::Rc;

use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::jsmath::js_max;
use kentos_geometry_core::ops::geoprocess::{self, Class, clip};
use kentos_geometry_core::ops::spatial_query::{self, Geometry, Relation, geometry_of};
use kentos_geometry_core::store::Store;

use super::{Session, Table, WorldCall};
use crate::compound::{self, Item};
use crate::library::Func;
use crate::scalar::{R, V, to_number};

/// What a relation gives for an object with no geometry the relations read (a text, a block …).
fn nothing(f: Func, out: &mut String) -> R<'static> {
    match f {
        Func::Intersects | Func::Encloses | Func::Inside | Func::CenterIn => R::V(V::Bool(false)),
        Func::IntersectCount | Func::OverlapArea | Func::OverlapLength => R::V(V::Num(0.0)),
        Func::Intersecting => {
            compound::push_array(out, std::iter::empty::<&Item>());
            R::Made
        }
        _ => R::V(V::Null),
    }
}

/// The most evaluated objects' geometries kept.
const KEPT: usize = 1024;

fn padded(b: &Bounds, r: f64) -> Bounds {
    Bounds {
        min_x: b.min_x - r,
        min_y: b.min_y - r,
        max_x: b.max_x + r,
        max_y: b.max_y + r,
    }
}

fn covers(outer: &Bounds, inner: &Bounds) -> bool {
    outer.min_x <= inner.min_x
        && outer.min_y <= inner.min_y
        && outer.max_x >= inner.max_x
        && outer.max_y >= inner.max_y
}

/// An object's geometry as the relations read it.
fn geometry(store: &Store, id: f64) -> Option<Geometry> {
    store.get(id).and_then(|it| geometry_of(&it.shape))
}

impl Session<'_> {
    /// The evaluated object's geometry, kept for the next call that asks.
    fn current(&self, store: &Store, id: f64) -> Rc<Option<Geometry>> {
        let mut st = self.state.borrow_mut();
        if let Some(g) = st.current.get(&id.to_bits()) {
            return Rc::clone(g);
        }
        if st.current.len() >= KEPT {
            st.current.clear();
        }
        let g = Rc::new(geometry(store, id));
        st.current.insert(id.to_bits(), Rc::clone(&g));
        g
    }

    /// Object `pos` of layer `l`'s geometry, made the first time.
    fn other(&self, store: &Store, l: usize, pos: usize) -> Rc<Option<Geometry>> {
        let mut st = self.state.borrow_mut();
        let n = self.world.layers[l].ids.len();
        let list = st.shapes.entry(l).or_insert_with(|| vec![None; n]);
        if let Some(Some(g)) = list.get(pos) {
            return Rc::clone(g);
        }
        let id = self.world.layers[l].ids[pos];
        let g = Rc::new(geometry(store, id));
        if let Some(slot) = list.get_mut(pos) {
            *slot = Some(Rc::clone(&g));
        }
        g
    }

    /// Layer `l`'s objects near the box `b` (padded): their places in the
    /// layer, those the condition keeps, the object `id` left out.
    fn near(&self, store: &Store, l: usize, t: &Table, b: &Bounds, id: f64) -> Vec<usize> {
        let mut out: Vec<usize> = store
            .in_box(b)
            .into_iter()
            .filter(|&other| other.to_bits() != id.to_bits())
            .filter_map(|other| self.place(other))
            .filter(|&(ol, pos)| ol == l && t.mask.as_ref().is_none_or(|m| m[pos]))
            .map(|(_, pos)| pos)
            .collect();
        out.sort_unstable();
        out
    }

    /// The places of layer `l`'s objects in `relation` to `g`, in the layer's
    /// order; only the first when `first`.
    #[allow(clippy::too_many_arguments)]
    fn related(
        &self,
        store: &Store,
        l: usize,
        t: &Table,
        g: &Geometry,
        id: f64,
        relation: Relation,
        within: f64,
        first: bool,
    ) -> Vec<usize> {
        let reach = spatial_query::reach(relation, within);
        let mut out = Vec::new();
        for pos in self.near(store, l, t, &padded(&g.bounds, reach), id) {
            let other = self.other(store, l, pos);
            if let Some(o) = other.as_ref()
                && spatial_query::relate(g, o, relation, within)
            {
                out.push(pos);
                if first {
                    break;
                }
            }
        }
        out
    }

    /// Layer `l`'s nearest object to `g` and its distance: the box around
    /// `g` grows until the nearest found is within it, or it holds the layer.
    fn nearest(
        &self,
        store: &Store,
        l: usize,
        t: &Table,
        g: &Geometry,
        id: f64,
    ) -> Option<(usize, f64)> {
        let extent = {
            let mut st = self.state.borrow_mut();
            *st.extents
                .entry(l)
                .or_insert_with(|| store.extent(Some(&self.world.layers[l].ids)))
        }?;
        let b = &g.bounds;
        let mut r = js_max(js_max(b.max_x - b.min_x, b.max_y - b.min_y), 1.0);
        loop {
            let reach = padded(b, r);
            let mut best: Option<(usize, f64)> = None;
            for pos in self.near(store, l, t, &reach, id) {
                let other = self.other(store, l, pos);
                let Some(o) = other.as_ref() else { continue };
                let d = spatial_query::distance(g, o);
                // Ties keep the first in the layer's order (places come sorted).
                if best.is_none_or(|(_, bd)| d < bd) {
                    best = Some((pos, d));
                }
            }
            if best.is_some_and(|(_, d)| d <= r) || covers(&reach, &extent) {
                return best;
            }
            if !r.is_finite() {
                return best;
            }
            r *= 4.0;
        }
    }

    pub(super) fn spatial(
        &self,
        c: &WorldCall,
        l: usize,
        t: &Table,
        id: f64,
        own: &[V],
        out: &mut String,
    ) -> R<'static> {
        let Some(store) = self.world.store else {
            return R::V(V::Null);
        };
        let current = self.current(store, id);
        let Some(g) = current.as_ref() else {
            return nothing(c.func, out);
        };
        let distance = own.first().and_then(|&v| to_number(v)).filter(|d| *d > 0.0);
        let (by_distance, within) = match distance {
            Some(d) => (Relation::Near, d),
            None => (Relation::Intersects, 0.0),
        };
        let related =
            |relation, within, first| self.related(store, l, t, g, id, relation, within, first);
        match c.func {
            Func::Intersects => R::V(V::Bool(!related(by_distance, within, true).is_empty())),
            Func::IntersectCount => R::V(V::Num(related(by_distance, within, false).len() as f64)),
            Func::Intersecting => {
                let found = related(by_distance, within, false);
                let values: Vec<&Item> = found.iter().filter_map(|&p| t.values.get(p)).collect();
                compound::push_array(out, values);
                R::Made
            }
            Func::Encloses => R::V(V::Bool(!related(Relation::Contains, 0.0, true).is_empty())),
            Func::Inside => R::V(V::Bool(!related(Relation::Within, 0.0, true).is_empty())),
            Func::CenterIn => R::V(V::Bool(!related(Relation::CenterIn, 0.0, true).is_empty())),
            Func::Distance => match self.nearest(store, l, t, g, id) {
                Some((_, d)) => R::V(V::Num(d)),
                None => R::V(V::Null),
            },
            Func::Nearest => match self.nearest(store, l, t, g, id) {
                Some((pos, _)) => super::give(t.values.get(pos), out),
                None => R::V(V::Null),
            },
            Func::OverlapArea => {
                if g.areas.is_empty() {
                    return R::V(V::Num(0.0));
                }
                let mut sum = 0.0;
                for pos in related(Relation::Intersects, 0.0, false) {
                    let other = self.other(store, l, pos);
                    if let Some(o) = other.as_ref()
                        && !o.areas.is_empty()
                    {
                        sum += geoprocess::areas_measure(&geoprocess::intersect_all(
                            &g.areas, &o.areas,
                        ));
                    }
                }
                R::V(V::Num(sum))
            }
            Func::OverlapLength => {
                let paths = Class::Paths(geoprocess::chain(g.edges.clone()));
                let mut sum = 0.0;
                for pos in related(Relation::Intersects, 0.0, false) {
                    let other = self.other(store, l, pos);
                    if let Some(o) = other.as_ref()
                        && !o.areas.is_empty()
                    {
                        sum += geoprocess::measure(&clip::within(&paths, &o.areas, true));
                    }
                }
                R::V(V::Num(sum))
            }
            _ => R::V(V::Null),
        }
    }
}
