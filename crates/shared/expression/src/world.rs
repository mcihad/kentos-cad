//! Başka katmanlara bakan işlevler (docs/adr/0214 §2.6–§2.8, §3): the
//! aggregates over the object's own layer (`topla($alan, Mahalle)`) or
//! another (`katman_toplamı`), the spatial relations to another layer's
//! objects (`kesişir('Sit alanı')`, `en_yakın('Durak', Ad)`) and a value from
//! another layer (`katmandan`). Compiling turns each call into a
//! `WorldCall`: the layer's name, its constant text and its inner
//! expressions, computed on the other objects. The caller gives the layers
//! (`World`: their objects and the geometry store they are in); a `Session`
//! computes what a call needs on a layer once and then answers object by
//! object, for the column engine and the one-object walk alike.

pub mod aggregate;
pub mod roles;
mod spatial;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::spatial_query::Geometry;
use kentos_geometry_core::store::Store;

pub use aggregate::Aggregate;

use crate::Expr;
use crate::arrays::Key;
use crate::compound::Item;
use crate::exec::{self, BATCH, Regs, Source, Txt};
use crate::js::text::{fold_turkish, trim};
use crate::library::Func;
use crate::scalar::{R, Scratch, V, truthy};

/// A call of a function that looks at other objects, compiled.
#[derive(Clone, Debug)]
pub struct WorldCall {
    pub func: Func,
    /// The other layer's name as written; None: the object's own layer.
    pub layer: Option<String>,
    /// `katman_toplamı`'s operation, `katmandan`'s field, `değerleri_birleştir`'s separator.
    pub text: Option<String>,
    /// What is computed on each of the other objects.
    pub value: Option<Box<Expr>>,
    /// The aggregates' group; `katmandan`'s key field.
    pub group: Option<Box<Expr>>,
    /// The condition the other objects must meet.
    pub condition: Option<Box<Expr>>,
}

/// A layer the functions look at: its objects in the document's order (their
/// ids in the geometry store) and their values for the inner expressions.
pub struct WorldLayer<'w> {
    /// The name the layer tree shows.
    pub name: String,
    pub ids: Vec<f64>,
    pub objects: &'w dyn LayerObjects,
}

/// A layer's objects as the column engine reads them.
pub trait LayerObjects {
    /// A source for expression `e` over the layer's objects, in `ids`' order.
    fn source<'s>(&'s self, e: &'s Expr) -> Box<dyn Source<'s> + 's>;
}

/// What the functions may look at: the layers, in the tree's order (a name
/// two layers share means the first), and the geometry store every object
/// (the evaluated ones too) is in.
pub struct World<'w> {
    pub layers: Vec<WorldLayer<'w>>,
    pub store: Option<&'w Store>,
}

/// The answers the engine asks of a world while it evaluates an expression.
pub trait WorldCalls {
    /// Call `k` of the expression (`Expr::world`) for the object `id`, with
    /// its own arguments (`uzaklık`, `değer`).
    fn call(&self, k: usize, id: f64, own: &[V], out: &mut String) -> R<'static>;
}

/// A source with the world its expression looks at.
pub struct WithWorld<'s, 'a> {
    pub inner: &'s dyn Source<'a>,
    pub world: &'s dyn WorldCalls,
}

impl<'a> Source<'a> for WithWorld<'_, 'a> {
    fn fill(&self, load: crate::program::Load, start: usize, slot: exec::Slot<'_, 'a>) {
        self.inner.fill(load, start, slot);
    }

    fn world(&self) -> Option<&dyn WorldCalls> {
        Some(self.world)
    }
}

/// A layer's name as it is looked for: Turkish letters and case aside.
pub fn layer_key(name: &str) -> String {
    fold_turkish(trim(name))
}

/// What a call computed on a layer's objects.
#[derive(Default)]
struct Table {
    /// The value expression's, one per object.
    values: Vec<Item>,
    /// Which objects meet the condition (None: all).
    mask: Option<Vec<bool>>,
    /// Each object's group key (aggregates with a group).
    groups: Option<Vec<Option<Key>>>,
    /// An aggregate's result per group (None: the empty group, or no group).
    results: HashMap<Option<Key>, Item>,
    /// `katmandan`: the first object of each key.
    first: HashMap<Key, u32>,
}

#[derive(Default)]
struct State {
    tables: HashMap<(usize, usize), Rc<Table>>,
    /// Per layer, its objects' geometry as the relations read it, made when first asked.
    shapes: HashMap<usize, Vec<Option<Rc<Option<Geometry>>>>>,
    /// The evaluated objects' geometry, the last ones asked.
    current: HashMap<u64, Rc<Option<Geometry>>>,
    /// Per layer, the box around its objects.
    extents: HashMap<usize, Option<Bounds>>,
}

/// An expression's calls answered over a world.
pub struct Session<'w> {
    calls: &'w [WorldCall],
    world: &'w World<'w>,
    by_name: HashMap<String, usize>,
    /// Each object's layer and place in it.
    places: HashMap<u64, (u32, u32)>,
    state: RefCell<State>,
}

impl<'w> Session<'w> {
    pub fn new(expr: &'w Expr, world: &'w World<'w>) -> Session<'w> {
        let mut by_name = HashMap::new();
        let mut places = HashMap::new();
        for (l, layer) in world.layers.iter().enumerate() {
            by_name.entry(layer_key(&layer.name)).or_insert(l);
            for (k, id) in layer.ids.iter().enumerate() {
                places.insert(id.to_bits(), (l as u32, k as u32));
            }
        }
        Session {
            calls: &expr.world,
            world,
            by_name,
            places,
            state: RefCell::new(State::default()),
        }
    }

    fn place(&self, id: f64) -> Option<(usize, usize)> {
        self.places
            .get(&id.to_bits())
            .map(|&(l, k)| (l as usize, k as usize))
    }

    /// What call `k` needs of layer `l`, computed the first time.
    fn table(&self, k: usize, l: usize) -> Rc<Table> {
        if let Some(t) = self.state.borrow().tables.get(&(k, l)) {
            return Rc::clone(t);
        }
        let t = Rc::new(self.make_table(&self.calls[k], l));
        self.state.borrow_mut().tables.insert((k, l), Rc::clone(&t));
        t
    }

    fn make_table(&self, c: &WorldCall, l: usize) -> Table {
        let layer = &self.world.layers[l];
        let run = |e: &Expr| items(e, &*layer.objects.source(e), layer.ids.len());
        let mut t = Table {
            values: c.value.as_deref().map(run).unwrap_or_default(),
            mask: c
                .condition
                .as_deref()
                .map(|e| run(e).iter().map(|it| truthy(it.view())).collect()),
            ..Table::default()
        };
        let keys = || -> Vec<Option<Key>> {
            c.group
                .as_deref()
                .map(|e| run(e).iter().map(|it| Key::of(it.view())).collect())
                .unwrap_or_default()
        };
        match c.func {
            Func::FromLayer => {
                for (k, key) in keys().into_iter().enumerate() {
                    if let Some(key) = key
                        && t.mask.as_ref().is_none_or(|m| m[k])
                    {
                        t.first.entry(key).or_insert(k as u32);
                    }
                }
            }
            f => {
                if let Some(op) = Aggregate::of(f).or_else(|| {
                    (f == Func::Aggregate)
                        .then(|| c.text.as_deref().and_then(Aggregate::from_name))
                        .flatten()
                }) {
                    let groups = (f != Func::Aggregate && c.group.is_some()).then(keys);
                    t.results = aggregate::results(op, &t, groups.as_deref(), c);
                    t.groups = groups;
                }
            }
        }
        t
    }
}

impl WorldCalls for Session<'_> {
    fn call(&self, k: usize, id: f64, own: &[V], out: &mut String) -> R<'static> {
        let Some(c) = self.calls.get(k) else {
            return R::V(V::Null);
        };
        let layer = match &c.layer {
            None => match self.place(id) {
                Some((l, _)) => l,
                None => return R::V(V::Null),
            },
            Some(name) => match self.by_name.get(&layer_key(name)) {
                Some(&l) => l,
                None => return R::V(V::Null),
            },
        };
        let table = self.table(k, layer);
        match c.func {
            Func::Aggregate => give(table.results.get(&None), out),
            Func::FromLayer => {
                let Some(key) = own.first().and_then(|&v| Key::of(v)) else {
                    return R::V(V::Null);
                };
                match table.first.get(&key) {
                    Some(&pos) => give(table.values.get(pos as usize), out),
                    None => R::V(V::Null),
                }
            }
            f if let Some(op) = Aggregate::of(f) => {
                let Some((_, pos)) = self.place(id) else {
                    return R::V(V::Null);
                };
                let key = table.groups.as_ref().and_then(|g| g[pos].clone());
                match table.results.get(&key) {
                    Some(it) => give(Some(it), out),
                    // No object of the group meets the condition: what nothing gives (`say` 0).
                    None => give(
                        Some(&aggregate::over(op, &[], aggregate::separator(c))),
                        out,
                    ),
                }
            }
            _ => self.spatial(c, layer, &table, id, own, out),
        }
    }
}

/// An item as a result: numbers and true/false as they are, text written.
fn give(it: Option<&Item>, out: &mut String) -> R<'static> {
    match it {
        None | Some(Item::Null) => R::V(V::Null),
        Some(Item::Num(x)) => R::V(V::Num(*x)),
        Some(Item::Bool(b)) => R::V(V::Bool(*b)),
        Some(Item::Text(s) | Item::Nested(s)) => {
            out.push_str(s);
            R::Made
        }
    }
}

/// An expression's values over `n` objects of a source, as items (arrays
/// and maps keep their marks; what threw is empty).
pub(crate) fn items<'a>(e: &Expr, source: &dyn Source<'a>, n: usize) -> Vec<Item> {
    let p = e.program();
    let r = p.registers();
    let (mut kinds, mut nums, mut txts) = (
        vec![exec::NULL; r * BATCH],
        vec![0.0; r * BATCH],
        vec![Txt::default(); r * BATCH],
    );
    let mut made = vec![String::new(); r];
    let mut regs = Regs {
        kinds: &mut kinds,
        nums: &mut nums,
        txts: &mut txts,
        made: &mut made,
        stride: BATCH,
    };
    exec::prepare(p, &mut regs, BATCH);
    let mut scratch = Scratch::default();
    let mut out = Vec::with_capacity(n);
    for start in (0..n).step_by(BATCH) {
        let len = BATCH.min(n - start);
        exec::run(p, source, start, len, &mut regs, &mut scratch);
        let col = exec::result(p, &regs, len);
        out.extend((0..len).map(|i| col.view(i).map_or(Item::Null, Item::of)));
    }
    out
}
