//! The objects written in document schema 2 (docs/specs/kcad-v2.md §6.6):
//! each a one-key map, its kind and then its fields, whose keys are sorted per
//! object (they depend on the kind); every object with its persistent id,
//! unique and not nil.

use std::collections::{BTreeMap, HashSet};

use kentos_contracts::{DocumentSnapshotV2, Entity, EntityId, HatchPattern, RingGeometry, Vec2};

use super::Encoder;
use super::names::{dimension_style, hatch_pattern};
use crate::cbor::{Seg, key_order};
use crate::error::{Code, KcadError};

/// One field of an object, before the fields are sorted by key.
enum Val<'d> {
    Text(&'d str),
    Name(&'static str),
    Float(f64),
    Bool(bool),
    Uid(&'d EntityId),
    Point(&'d Vec2),
    Points(&'d [Vec2]),
    Floats(&'d [f64]),
    Attrs(&'d BTreeMap<String, String>),
    Rings(&'d [RingGeometry]),
    Loops(&'d [Vec<Vec2>]),
    Pattern(&'d HatchPattern),
}

impl<'d> Encoder<'d> {
    // ── Objects ─────────────────────────────────────────────────────────

    pub(super) fn objects(&mut self, doc: &'d DocumentSnapshotV2) -> Result<(), KcadError> {
        self.open(doc.entities.len(), false)?;
        let mut seen = HashSet::with_capacity(doc.uids.len());
        let mut fields = Vec::with_capacity(16);
        for (i, (entity, uid)) in doc.entities.iter().zip(&doc.uids).enumerate() {
            self.path.push(Seg::Index(i));
            if !seen.insert(uid.0) {
                return Err(self.fail(
                    Code::DuplicateUid,
                    &format!("kalıcı kimlik {uid} iki nesnede var"),
                ));
            }
            self.object(entity, uid, &mut fields)?;
            self.path.pop();
        }
        self.close();
        Ok(())
    }

    fn object(
        &mut self,
        entity: &'d Entity,
        uid: &'d EntityId,
        f: &mut Vec<(&'static str, Val<'d>)>,
    ) -> Result<(), KcadError> {
        let kind = entity.kind();
        let base = entity.base();
        f.clear();
        f.push(("uid", Val::Uid(uid)));
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
            }
            Entity::Polyline(e) | Entity::Polygon(e) => {
                f.push(("pts", Val::Points(&e.pts)));
                if let Some(b) = &e.bulges {
                    f.push(("bulges", Val::Floats(b)));
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
            Val::Point(p) => self.point(p),
            Val::Points(list) => self.points(list),
            Val::Floats(list) => self.floats(list),
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
            Val::Rings(rings) => {
                self.open(rings.len(), false)?;
                for (i, ring) in rings.iter().enumerate() {
                    self.at(Seg::Index(i), |e| {
                        e.open(1 + usize::from(ring.bulges.is_some()), true)?;
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
