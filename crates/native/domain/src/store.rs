//! The document's objects: by slot, in document order, per layer in document
//! order, and by persistent id.
//!
//! Document order is the order objects entered the document, as on the web
//! (apps/web/src/model/document.ts): changing an object keeps its place, even
//! on another layer, and an object removed and put back (undo, redo, a rolled
//! back transaction) returns to its place (docs/adr/0020). It is the drawing
//! order and the order a saved file lists the objects in.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ops::Range;
use std::sync::{Arc, OnceLock};

use kentos_contracts::Entity;

use crate::hash::SlotMap;
use crate::identity::{Slot, Uuid};

/// An object as the document keeps it: its persistent id and its data, whose
/// `id` is the slot. Never changed in place: an edit stores a new value, so the
/// store and the undo records share the unchanged ones.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Stored {
    pub uid: Uuid,
    pub entity: Arc<Entity>,
}

impl Stored {
    pub fn slot(&self) -> Slot {
        Slot(self.entity.base().id)
    }

    fn layer(&self) -> &str {
        &self.entity.base().layer_id
    }
}

#[derive(Clone, Debug)]
struct Item {
    /// Place in the document: larger is later.
    seq: u64,
    stored: Stored,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Store {
    /// Derived wall reach boxes; shared immutably by reading copies, never serialized.
    pub(crate) wall_candidates: OnceLock<Arc<crate::wall_candidates::WallCandidates>>,
    items: SlotMap<Item>,
    order: BTreeMap<u64, Slot>,
    layers: HashMap<String, BTreeMap<u64, Slot>>,
    uids: HashMap<Uuid, Slot>,
    next_seq: u64,
    /// The places of removed slots. Slots are never given twice, so a slot
    /// that comes back is the same object, and it takes its place again.
    vacated: SlotMap<u64>,
    /// The texts that write an object's label, by that object's persistent
    /// id (docs/adr/0175 §4): what a step that changes the object updates.
    links: HashMap<Uuid, BTreeSet<Slot>>,
    /// The hatches whose region follows an object, by that object's
    /// persistent id (docs/adr/0186 §6).
    ties: HashMap<Uuid, BTreeSet<Slot>>,
    /// The dimensions with a broken reference (docs/adr/0249 §3): what the drawing marks.
    broken: BTreeSet<Slot>,
    /// The spaces whose walls no longer close round them (docs/adr/0342 §3): what the drawing marks.
    lost: BTreeSet<Slot>,
    /// Explicit virtual room boundaries; derived, never serialized.
    space_separators: BTreeSet<Slot>,
    /// The texts whose fields read an object, by that object's persistent id (docs/adr/0325 §2).
    fielded: HashMap<Uuid, BTreeSet<Slot>>,
    /// The texts with a project's or a layer's field: written again at every step.
    always: BTreeSet<Slot>,
    /// Every text with fields.
    with_fields: BTreeSet<Slot>,
    /// The tables with fields, formulas or a block schedule (docs/adr/0327): worked out before a step is recorded.
    computed: BTreeSet<Slot>,
    /// The tags (docs/adr/0344 §1): written again when their objects change.
    tags: BTreeSet<Slot>,
    /// The view marks (docs/adr/0345 §2): written again when their targets change.
    viewmarks: BTreeSet<Slot>,
    /// The Kots bound to a level (docs/adr/0721 §2) and the ramps bound to theirs (docs/adr/0723 §2): written again
    /// when the levels change.
    level_marks: BTreeSet<Slot>,
}

/// Whether a table has cells the document works out (docs/adr/0327).
fn computed_table(stored: &Stored) -> bool {
    matches!(&*stored.entity, Entity::Table(t)
        if !t.fields.is_empty()
            || !t.formulas.is_empty()
            || matches!(
                t.source,
                Some(kentos_contracts::TableSource::Blocks { .. } | kentos_contracts::TableSource::Schedule { .. })
            ))
}

/// The objects a text's fields read, and whether one reads the project or a layer's name (docs/adr/0325 §2).
fn fields_of(stored: &Stored) -> Option<(Vec<Uuid>, bool)> {
    let Entity::Text(t) = &*stored.entity else {
        return None;
    };
    if t.paragraph.fields.is_empty() {
        return None;
    }
    let mut of = Vec::new();
    let mut always = false;
    for f in &t.paragraph.fields {
        match &f.source {
            kentos_contracts::FieldSource::Object { uid, value, .. } => {
                of.push(Uuid::from_bytes(uid.0));
                always |= *value == kentos_contracts::ObjectValue::Layer;
            }
            kentos_contracts::FieldSource::Project { .. } => always = true,
        }
    }
    of.sort_unstable();
    of.dedup();
    Some((of, always))
}

/// The objects a hatch's region follows: its closed object or its face's
/// line work, islands and cutouts (docs/adr/0186 §6, 0328 §3); the objects an associative dimension's points
/// follow while their references hold (docs/adr/0249 §3).
fn ties_of(stored: &Stored) -> Vec<Uuid> {
    match &*stored.entity {
        Entity::Hatch(h) => h.assoc.as_ref().map_or_else(Vec::new, |a| {
            a.objects().map(|id| Uuid::from_bytes(id.0)).collect()
        }),
        Entity::Dimension(d) => d.refs.as_ref().map_or_else(Vec::new, |r| {
            let mut of: Vec<Uuid> = r
                .each()
                .filter(|(_, s)| s.live())
                .map(|(_, s)| Uuid::from_bytes(s.uid.0))
                .collect();
            of.dedup();
            of
        }),
        // A leader's tips while they follow their places, and its content blocks (docs/adr/0326 §2, §3).
        Entity::Leader(l) => {
            let mut of: Vec<Uuid> = std::iter::once(l.tip.as_ref())
                .chain(l.arms.iter().map(|a| a.tip.as_ref()))
                .flatten()
                .filter(|t| t.live())
                .map(|t| Uuid::from_bytes(t.uid.0))
                .chain(l.blocks.iter().map(|b| Uuid::from_bytes(b.0)))
                .collect();
            of.sort_unstable();
            of.dedup();
            of
        }
        // An opening's wall (docs/adr/0341 §3).
        Entity::Opening(o) => Uuid::parse_str(&o.opening.host).into_iter().collect(),
        // A tag's object (docs/adr/0344 §1).
        Entity::Tag(t) => Uuid::parse_str(&t.tag.of).into_iter().collect(),
        // A column's grids (docs/adr/0343 §3).
        Entity::Column(c) => c
            .column
            .grids
            .iter()
            .flatten()
            .filter_map(|g| Uuid::parse_str(g).ok())
            .collect(),
        _ => Vec::new(),
    }
}

/// Whether a dimension has a broken reference (docs/adr/0249 §3).
fn has_broken(stored: &Stored) -> bool {
    matches!(&*stored.entity, Entity::Dimension(d)
        if d.refs.as_ref().is_some_and(|r| r.each().any(|(_, s)| !s.live())))
}

/// Whether a space lost its walls (docs/adr/0342 §3), or a column its grids (docs/adr/0343 §3).
fn has_lost(stored: &Stored) -> bool {
    match &*stored.entity {
        Entity::Space(s) => s.space.broken.is_some(),
        Entity::Column(c) => c.column.broken.is_some(),
        Entity::Tag(t) => t.tag.broken.is_some(),
        Entity::ViewMark(m) => m.mark.broken.is_some(),
        _ => false,
    }
}

/// The object a text writes the label of, when it is a linked text (docs/adr/0175 §4).
fn link_of(stored: &Stored) -> Option<Uuid> {
    match &*stored.entity {
        Entity::Text(t) => t.label_of.map(|id| Uuid::from_bytes(id.0)),
        _ => None,
    }
}

impl Store {
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn get(&self, slot: Slot) -> Option<&Stored> {
        self.items.get(&slot).map(|item| &item.stored)
    }

    pub fn slot_of(&self, uid: Uuid) -> Option<Slot> {
        self.uids.get(&uid).copied()
    }

    /// The texts that write the label of the object with persistent id `of`, in slot order.
    pub fn linked_to(&self, of: Uuid) -> impl Iterator<Item = Slot> + '_ {
        self.links.get(&of).into_iter().flatten().copied()
    }

    /// Whether any text writes an object's label.
    pub fn has_links(&self) -> bool {
        !self.links.is_empty()
    }

    /// The persistent ids of the objects whose label a text writes.
    pub fn linked_objects(&self) -> impl Iterator<Item = Uuid> + '_ {
        self.links.keys().copied()
    }

    /// The hatches whose region follows the object with persistent id `of`, in slot order.
    pub fn tied_to(&self, of: Uuid) -> impl Iterator<Item = Slot> + '_ {
        self.ties.get(&of).into_iter().flatten().copied()
    }

    /// Whether any hatch follows an object.
    pub fn has_ties(&self) -> bool {
        !self.ties.is_empty()
    }

    /// Every object that follows another (hatches and dimensions), each once, in slot order.
    pub fn tied_slots(&self) -> Vec<Slot> {
        let all: BTreeSet<Slot> = self.ties.values().flatten().copied().collect();
        all.into_iter().collect()
    }

    /// The texts whose fields read the object with persistent id `of`, in slot order.
    pub fn fielded_from(&self, of: Uuid) -> impl Iterator<Item = Slot> + '_ {
        self.fielded.get(&of).into_iter().flatten().copied()
    }

    /// The texts with a project's or a layer's field, in slot order.
    pub fn always_fielded_slots(&self) -> impl Iterator<Item = Slot> + '_ {
        self.always.iter().copied()
    }

    /// Whether any text has fields.
    pub fn has_fields(&self) -> bool {
        !self.with_fields.is_empty()
    }

    /// How many texts have fields.
    pub fn fielded_count(&self) -> usize {
        self.with_fields.len()
    }

    /// Whether any table has worked out cells.
    pub fn has_computed_tables(&self) -> bool {
        !self.computed.is_empty()
    }

    /// The tables with worked out cells, in slot order.
    pub fn computed_table_slots(&self) -> impl Iterator<Item = Slot> + '_ {
        self.computed.iter().copied()
    }

    /// Whether the drawing has a tag.
    pub fn has_tags(&self) -> bool {
        !self.tags.is_empty()
    }

    /// Tags in slot order, without scanning unrelated drawing objects.
    pub(crate) fn tag_slots(&self) -> impl Iterator<Item = Slot> + '_ {
        self.tags.iter().copied()
    }

    pub fn has_space_separators(&self) -> bool { !self.space_separators.is_empty() }
    pub fn space_separator_slots(&self) -> impl Iterator<Item=Slot> + '_ {
        self.space_separators.iter().copied()
    }

    /// Whether the drawing has a view mark.
    pub fn has_viewmarks(&self) -> bool {
        !self.viewmarks.is_empty()
    }

    /// The view marks, in slot order.
    pub fn viewmark_slots(&self) -> impl Iterator<Item = Slot> + '_ {
        self.viewmarks.iter().copied()
    }

    /// Whether the drawing has a Kot bound to a level.
    pub fn has_level_marks(&self) -> bool {
        !self.level_marks.is_empty()
    }

    /// The Kots bound to a level, in slot order.
    pub fn level_mark_slots(&self) -> impl Iterator<Item = Slot> + '_ {
        self.level_marks.iter().copied()
    }

    fn link(&mut self, stored: &Stored) {
        if crate::space_follow::separator_path(&stored.entity).is_some() {
            self.space_separators.insert(stored.slot());
        }
        if matches!(&*stored.entity, Entity::Wall(_)) {
            self.wall_candidates.take();
        }
        if computed_table(stored) {
            self.computed.insert(stored.slot());
        }
        if matches!(&*stored.entity, Entity::Tag(_)) {
            self.tags.insert(stored.slot());
        }
        if matches!(&*stored.entity, Entity::ViewMark(_)) {
            self.viewmarks.insert(stored.slot());
        }
        if crate::level_marks::level_bound(&stored.entity) {
            self.level_marks.insert(stored.slot());
        }
        if let Some((of, always)) = fields_of(stored) {
            for uid in of {
                self.fielded.entry(uid).or_default().insert(stored.slot());
            }
            if always {
                self.always.insert(stored.slot());
            }
            self.with_fields.insert(stored.slot());
        }
        if let Some(of) = link_of(stored) {
            self.links.entry(of).or_default().insert(stored.slot());
        }
        for of in ties_of(stored) {
            self.ties.entry(of).or_default().insert(stored.slot());
        }
        if has_broken(stored) {
            self.broken.insert(stored.slot());
        }
        if has_lost(stored) {
            self.lost.insert(stored.slot());
        }
    }

    fn unlink(&mut self, stored: &Stored) {
        self.space_separators.remove(&stored.slot());
        if matches!(&*stored.entity, Entity::Wall(_)) {
            self.wall_candidates.take();
        }
        self.computed.remove(&stored.slot());
        self.tags.remove(&stored.slot());
        self.viewmarks.remove(&stored.slot());
        self.level_marks.remove(&stored.slot());
        if let Some((of, _)) = fields_of(stored) {
            for uid in of {
                if let Some(texts) = self.fielded.get_mut(&uid) {
                    texts.remove(&stored.slot());
                    if texts.is_empty() {
                        self.fielded.remove(&uid);
                    }
                }
            }
            self.always.remove(&stored.slot());
            self.with_fields.remove(&stored.slot());
        }
        if let Some(of) = link_of(stored)
            && let Some(texts) = self.links.get_mut(&of)
        {
            texts.remove(&stored.slot());
            if texts.is_empty() {
                self.links.remove(&of);
            }
        }
        for of in ties_of(stored) {
            if let Some(hatches) = self.ties.get_mut(&of) {
                hatches.remove(&stored.slot());
                if hatches.is_empty() {
                    self.ties.remove(&of);
                }
            }
        }
        self.broken.remove(&stored.slot());
        self.lost.remove(&stored.slot());
    }

    /// The dimensions with a broken reference, in slot order (docs/adr/0249 §3).
    pub fn broken_dimensions(&self) -> impl Iterator<Item = Slot> + '_ {
        self.broken.iter().copied()
    }

    /// The spaces that lost their walls and the columns their grids, in slot order (docs/adr/0342 §3, 0343 §3).
    pub fn lost_spaces(&self) -> impl Iterator<Item = Slot> + '_ {
        self.lost.iter().copied()
    }

    /// Every object in document order.
    pub fn iter(&self) -> impl Iterator<Item = &Stored> {
        self.order.values().filter_map(|slot| self.get(*slot))
    }

    /// The objects of one layer in document order.
    pub fn on_layer(&self, layer: &str) -> impl Iterator<Item = &Stored> {
        self.layers
            .get(layer)
            .into_iter()
            .flat_map(|list| list.values())
            .filter_map(|slot| self.get(*slot))
    }

    /// The objects of one layer whose places fall in `places`, in document order, with their places.
    pub fn on_layer_placed(
        &self,
        layer: &str,
        places: Range<u64>,
    ) -> impl Iterator<Item = (u64, &Stored)> {
        self.layers
            .get(layer)
            .into_iter()
            .flat_map(move |list| list.range(places.clone()))
            .filter_map(|(seq, slot)| self.get(*slot).map(|stored| (*seq, stored)))
    }

    /// The places and slots of a layer's objects in document order, without reading the objects.
    pub fn layer_slots(&self, layer: &str) -> impl Iterator<Item = (u64, Slot)> {
        self.layers
            .get(layer)
            .into_iter()
            .flat_map(|list| list.iter().map(|(seq, slot)| (*seq, *slot)))
    }

    /// An object's place in the document.
    pub fn place(&self, slot: Slot) -> Option<u64> {
        self.items.get(&slot).map(|item| item.seq)
    }

    pub fn count(&self, layer: &str) -> usize {
        self.layers.get(layer).map_or(0, BTreeMap::len)
    }

    /// Room for `n` more objects: a large addition grows the tables once.
    pub fn reserve(&mut self, n: usize) {
        self.items.reserve(n);
        self.uids.reserve(n);
    }

    /// Preview additions are temporary, unlike undone real additions. Restore their allocation clock
    /// and remove their vacated places after reverting, without cloning the full indexed store.
    pub(crate) fn preview_clock(&self) -> u64 {
        self.next_seq
    }
    pub(crate) fn restore_preview_clock(&mut self, clock: u64, first: u64, end: u64) {
        for id in first..end {
            if let Ok(id) = u32::try_from(id) {
                self.vacated.remove(&Slot(id));
            }
        }
        self.next_seq = clock;
    }

    /// An object into its layer's list; the layer's id is copied only for
    /// a layer not listed yet (a run adds hundreds of thousands at once).
    fn list(&mut self, layer: &str, seq: u64, slot: Slot) {
        match self.layers.get_mut(layer) {
            Some(list) => {
                list.insert(seq, slot);
            }
            None => {
                self.layers
                    .insert(layer.to_owned(), BTreeMap::from([(seq, slot)]));
            }
        }
    }

    /// Sets an object. A known slot keeps its place in the document (and in
    /// its new layer's list when the layer changed); a new one goes last.
    pub fn put(&mut self, stored: Stored) {
        let slot = stored.slot();
        if let Some(old) = self.items.get(&slot).map(|item| item.stored.clone()) {
            self.unlink(&old);
        }
        self.link(&stored);
        let Some(item) = self.items.get_mut(&slot) else {
            let seq = self.vacated.remove(&slot).unwrap_or_else(|| {
                self.next_seq += 1;
                self.next_seq - 1
            });
            self.order.insert(seq, slot);
            self.list(stored.layer(), seq, slot);
            self.uids.insert(stored.uid, slot);
            self.items.insert(slot, Item { seq, stored });
            return;
        };
        if item.stored.layer() != stored.layer() {
            if let Some(list) = self.layers.get_mut(item.stored.layer()) {
                list.remove(&item.seq);
            }
            let seq = item.seq;
            self.list(stored.layer(), seq, slot);
            if let Some(item) = self.items.get_mut(&slot) {
                if item.stored.uid != stored.uid {
                    self.uids.remove(&item.stored.uid);
                    self.uids.insert(stored.uid, slot);
                }
                item.stored = stored;
            }
            return;
        }
        if item.stored.uid != stored.uid {
            self.uids.remove(&item.stored.uid);
            self.uids.insert(stored.uid, slot);
        }
        item.stored = stored;
    }

    pub fn remove(&mut self, slot: Slot) -> Option<Stored> {
        let item = self.items.remove(&slot)?;
        self.unlink(&item.stored);
        self.order.remove(&item.seq);
        self.vacated.insert(slot, item.seq);
        if let Some(list) = self.layers.get_mut(item.stored.layer()) {
            list.remove(&item.seq);
        }
        self.uids.remove(&item.stored.uid);
        Some(item.stored)
    }
}
