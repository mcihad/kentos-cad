//! The objects written in document schema 2, 3 when one has its own line
//! weight, 4 when one has vertex elevations, 5 when an area has parts, or 6
//! with blocks (docs/specs/kcad-v2.md §6.6, docs/adr/0139, docs/adr/0142,
//! docs/adr/0143, docs/adr/0144): each a one-key map, its kind and then its
//! fields, whose keys are sorted per object (they depend on the kind); every
//! object of the drawing with its persistent id, unique and not nil, a block
//! definition's objects without one.

use std::collections::{BTreeMap, HashSet};

use kentos_contracts::{
    AreaPart, BlockId, DocumentSnapshotV2, Entity, EntityId, HatchPattern, MAX_LINE_WEIGHT,
    RingGeometry, Vec2,
};

use super::Encoder;
use super::names::{dimension_style, hatch_pattern};
use crate::cbor::{Seg, key_order};
use crate::error::{Code, KcadError};
use crate::watch::{EVERY, Step};

/// One field of an object, before the fields are sorted by key.
pub(super) enum Val<'d> {
    Text(&'d str),
    Name(&'static str),
    Float(f64),
    Bool(bool),
    Uid(&'d EntityId),
    Block(&'d BlockId),
    Point(&'d Vec2),
    Points(&'d [Vec2]),
    Floats(&'d [f64]),
    Attrs(&'d BTreeMap<String, String>),
    /// Elevations and the number of vertices they belong to (one each, §6.6).
    Elevations(&'d [Option<f64>], usize),
    Rings(&'d [RingGeometry]),
    /// A multi-part area's parts past its first (§6.6).
    Parts(&'d [AreaPart]),
    Loops(&'d [Vec<Vec2>]),
    Pattern(&'d HatchPattern),
}

impl<'d> Encoder<'d> {
    // ── Objects ─────────────────────────────────────────────────────────

    pub(super) fn objects(&mut self, doc: &'d DocumentSnapshotV2) -> Result<(), KcadError> {
        self.open(doc.entities.len(), false)?;
        let mut seen = HashSet::with_capacity(doc.uids.len());
        let mut fields = Vec::with_capacity(16);
        let total = doc.entities.len();
        for (i, (entity, uid)) in doc.entities.iter().zip(&doc.uids).enumerate() {
            if i % EVERY == 0 {
                self.report(Step::Writing { done: i, total })?;
            }
            self.path.push(Seg::Index(i));
            if !seen.insert(uid.0) {
                return Err(self.fail(
                    Code::DuplicateUid,
                    &format!("kalıcı kimlik {uid} iki nesnede var"),
                ));
            }
            self.object(entity, Some(uid), &mut fields)?;
            self.path.pop();
        }
        self.close();
        self.report(Step::Writing { done: total, total })
    }

    // Out of line on purpose: a browser's WebAssembly engine runs a function in its
    // baseline code until the function is called again (no on-stack replacement), so the
    // work of each object must be a function called once per object, not inlined into the
    // one loop that runs once for the whole drawing (docs/adr/0030).
    //
    // `uid` is `None` for a block definition's object (docs/adr/0144).
    #[inline(never)]
    pub(super) fn object(
        &mut self,
        entity: &'d Entity,
        uid: Option<&'d EntityId>,
        f: &mut Vec<(&'static str, Val<'d>)>,
    ) -> Result<(), KcadError> {
        let kind = entity.kind();
        let base = entity.base();
        f.clear();
        if let Some(uid) = uid {
            f.push(("uid", Val::Uid(uid)));
        }
        f.push(("attrs", Val::Attrs(&base.attrs)));
        f.push(("layerId", Val::Text(&base.layer_id)));
        if let Some(c) = &base.color {
            f.push(("color", Val::Text(c)));
        }
        if let Some(l) = &base.label {
            f.push(("label", Val::Text(l)));
        }
        if let Some(s) = &base.symbol {
            f.push(("symbol", Val::Text(s)));
        }
        if let Some(w) = base.line_weight {
            if !(0.0..=MAX_LINE_WEIGHT).contains(&w) {
                self.path.push(Seg::Name(kind));
                self.path.push(Seg::Name("lineWeight"));
                return Err(self.fail(
                    Code::BadValue,
                    &format!("çizgi kalınlığı {w} mm; 0 ile {MAX_LINE_WEIGHT} arasında olmalı"),
                ));
            }
            f.push(("lineWeight", Val::Float(w)));
        }
        match entity {
            Entity::Point(e) => {
                f.push(("p", Val::Point(&e.p)));
                if let Some(z) = e.z {
                    f.push(("z", Val::Float(z)));
                }
            }
            Entity::Line(e) => {
                f.push(("a", Val::Point(&e.a)));
                f.push(("b", Val::Point(&e.b)));
                if let Some(z) = e.za {
                    f.push(("za", Val::Float(z)));
                }
                if let Some(z) = e.zb {
                    f.push(("zb", Val::Float(z)));
                }
            }
            Entity::Polyline(e) | Entity::Polygon(e) => {
                f.push(("pts", Val::Points(&e.pts)));
                if let Some(b) = &e.bulges {
                    f.push(("bulges", Val::Floats(b)));
                }
                if let Some(z) = &e.zs {
                    f.push(("zs", Val::Elevations(z, e.pts.len())));
                }
                if let Some(h) = &e.holes {
                    if matches!(entity, Entity::Polyline(_)) {
                        self.path.push(Seg::Name(kind));
                        return Err(self.fail(
                            Code::BadValue,
                            "çoklu çizginin adası (deliği) olamaz; yalnız kapalı alanın olur",
                        ));
                    }
                    f.push(("holes", Val::Rings(h)));
                }
                if let Some(parts) = &e.parts {
                    if matches!(entity, Entity::Polyline(_)) {
                        self.path.push(Seg::Name(kind));
                        return Err(self.fail(
                            Code::BadValue,
                            "çoklu çizginin parçası olamaz; yalnız kapalı alan çok parçalı olur",
                        ));
                    }
                    f.push(("parts", Val::Parts(parts)));
                }
            }
            Entity::Circle(e) => {
                f.push(("c", Val::Point(&e.c)));
                f.push(("r", Val::Float(e.r)));
            }
            Entity::Arc(e) => {
                f.push(("c", Val::Point(&e.c)));
                f.push(("r", Val::Float(e.r)));
                f.push(("a0", Val::Float(e.a0)));
                f.push(("a1", Val::Float(e.a1)));
            }
            Entity::Ellipse(e) => {
                f.push(("c", Val::Point(&e.c)));
                f.push(("major", Val::Point(&e.major)));
                f.push(("ratio", Val::Float(e.ratio)));
                f.push(("t0", Val::Float(e.t0)));
                f.push(("t1", Val::Float(e.t1)));
            }
            Entity::Spline(e) => {
                f.push(("pts", Val::Points(&e.pts)));
                f.push(("closed", Val::Bool(e.closed)));
            }
            Entity::Xline(e) | Entity::Ray(e) => {
                f.push(("p", Val::Point(&e.p)));
                f.push(("dir", Val::Point(&e.dir)));
            }
            Entity::Text(e) => {
                f.push(("p", Val::Point(&e.p)));
                f.push(("text", Val::Text(&e.text)));
                f.push(("height", Val::Float(e.height)));
                f.push(("rotation", Val::Float(e.rotation)));
            }
            Entity::Dimension(e) => {
                f.push(("a", Val::Point(&e.a)));
                f.push(("b", Val::Point(&e.b)));
                f.push(("offset", Val::Float(e.offset)));
                f.push(("height", Val::Float(e.height)));
                if let Some(t) = &e.text {
                    f.push(("text", Val::Text(t)));
                }
                if let Some(s) = e.style {
                    f.push(("style", Val::Name(dimension_style(s))));
                }
                if let Some(a) = e.angle {
                    f.push(("angle", Val::Float(a)));
                }
                if let Some(c) = &e.c {
                    f.push(("c", Val::Point(c)));
                }
            }
            Entity::Hatch(e) => {
                f.push(("ring", Val::Points(&e.ring)));
                if let Some(h) = &e.holes {
                    f.push(("holes", Val::Loops(h)));
                }
                f.push(("pattern", Val::Pattern(&e.pattern)));
            }
            // The scale was checked with the block rules (`body`).
            Entity::Insert(e) => {
                f.push(("block", Val::Block(&e.block)));
                f.push(("p", Val::Point(&e.p)));
                f.push(("scale", Val::Float(e.scale)));
                f.push(("rotation", Val::Float(e.rotation)));
                if e.mirror {
                    f.push(("mirror", Val::Bool(true)));
                }
            }
        }
        f.sort_by(|a, b| key_order(a.0, b.0));
        self.open(1, true)?;
        self.key(kind);
        self.path.push(Seg::Name(kind));
        self.open(f.len(), true)?;
        for (k, v) in f.drain(..) {
            self.key(k);
            self.path.push(Seg::Name(k));
            self.val(v)?;
            self.path.pop();
        }
        self.close();
        self.path.pop();
        self.close();
        Ok(())
    }

    /// A list of elevations, one per vertex (§6.6): a vertex without one is
    /// written as `null`; the place is on the path.
    fn elevations(&mut self, list: &'d [Option<f64>], vertices: usize) -> Result<(), KcadError> {
        if list.len() != vertices {
            return Err(self.fail(
                Code::BadValue,
                &format!(
                    "{} kot var ama {vertices} köşe var; her köşenin bir kotu olmalı (kotsuz köşe için null)",
                    list.len()
                ),
            ));
        }
        self.open(list.len(), false)?;
        for (i, z) in list.iter().enumerate() {
            self.at(Seg::Index(i), |e| match z {
                Some(x) => e.float(*x),
                None => {
                    e.w.null();
                    Ok(())
                }
            })?;
        }
        self.close();
        Ok(())
    }

    /// A polygon's (or a part's) holes, each ring's keys in encoded order.
    fn rings(&mut self, rings: &'d [RingGeometry]) -> Result<(), KcadError> {
        self.open(rings.len(), false)?;
        for (i, ring) in rings.iter().enumerate() {
            self.at(Seg::Index(i), |e| {
                let optional = usize::from(ring.bulges.is_some()) + usize::from(ring.zs.is_some());
                e.open(1 + optional, true)?;
                // zs (2), pts (3), bulges (6).
                if let Some(zs) = &ring.zs {
                    e.key("zs");
                    e.at(Seg::Name("zs"), |e| e.elevations(zs, ring.pts.len()))?;
                }
                e.key("pts");
                e.at(Seg::Name("pts"), |e| e.points(&ring.pts))?;
                if let Some(b) = &ring.bulges {
                    e.key("bulges");
                    e.at(Seg::Name("bulges"), |e| e.floats(b))?;
                }
                e.close();
                Ok(())
            })?;
        }
        self.close();
        Ok(())
    }

    fn val(&mut self, v: Val<'d>) -> Result<(), KcadError> {
        match v {
            Val::Text(t) => self.text(t),
            Val::Name(t) => {
                self.w.text(t);
                Ok(())
            }
            Val::Float(x) => self.float(x),
            Val::Bool(b) => {
                self.w.bool(b);
                Ok(())
            }
            Val::Uid(id) => self.id(&id.0),
            Val::Block(id) => self.id(&id.0),
            Val::Point(p) => self.point(p),
            Val::Points(list) => self.points(list),
            Val::Floats(list) => self.floats(list),
            Val::Elevations(list, vertices) => self.elevations(list, vertices),
            Val::Attrs(attrs) => {
                let mut pairs: Vec<(&String, &String)> = attrs.iter().collect();
                pairs.sort_by(|a, b| key_order(a.0, b.0));
                self.open(pairs.len(), true)?;
                for (k, v) in pairs {
                    self.text(k)?;
                    self.at(Seg::Key(k), |e| e.text(v))?;
                }
                self.close();
                Ok(())
            }
            Val::Rings(rings) => self.rings(rings),
            Val::Parts(parts) => {
                self.open(parts.len(), false)?;
                for (i, part) in parts.iter().enumerate() {
                    self.at(Seg::Index(i), |e| {
                        let optional = usize::from(part.zs.is_some())
                            + usize::from(part.holes.is_some())
                            + usize::from(part.bulges.is_some());
                        e.open(1 + optional, true)?;
                        // zs (2), pts (3), holes (5), bulges (6).
                        if let Some(zs) = &part.zs {
                            e.key("zs");
                            e.at(Seg::Name("zs"), |e| e.elevations(zs, part.pts.len()))?;
                        }
                        e.key("pts");
                        e.at(Seg::Name("pts"), |e| e.points(&part.pts))?;
                        if let Some(h) = &part.holes {
                            e.key("holes");
                            e.at(Seg::Name("holes"), |e| e.rings(h))?;
                        }
                        if let Some(b) = &part.bulges {
                            e.key("bulges");
                            e.at(Seg::Name("bulges"), |e| e.floats(b))?;
                        }
                        e.close();
                        Ok(())
                    })?;
                }
                self.close();
                Ok(())
            }
            Val::Loops(loops) => {
                self.open(loops.len(), false)?;
                for (i, list) in loops.iter().enumerate() {
                    self.at(Seg::Index(i), |e| e.points(list))?;
                }
                self.close();
                Ok(())
            }
            Val::Pattern(p) => {
                self.open(3, true)?;
                // type (4), angle (5), spacing (7).
                self.key("type");
                self.w.text(hatch_pattern(p.kind));
                self.key("angle");
                self.at(Seg::Name("angle"), |e| e.float(p.angle))?;
                self.key("spacing");
                self.at(Seg::Name("spacing"), |e| e.float(p.spacing))?;
                self.close();
                Ok(())
            }
        }
    }
}
