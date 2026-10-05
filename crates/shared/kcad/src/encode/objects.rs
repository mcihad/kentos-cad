//! The objects written in document schema 2, 3 when one has its own line
//! weight, 4 when one has vertex elevations, 5 when an area has parts, 6
//! with blocks, 7 when a text has an alignment, a width factor or a mask, or
//! 8 with a leader (docs/specs/kcad-v2.md §6.6, docs/adr/0139, docs/adr/0142,
//! docs/adr/0143, docs/adr/0144, docs/adr/0145, docs/adr/0146, docs/adr/0147): each a one-key map, its kind and then its
//! fields, whose keys are sorted per object (they depend on the kind); every
//! object of the drawing with its persistent id, unique and not nil, a block
//! definition's objects without one.

use std::collections::{BTreeMap, HashSet};

use kentos_contracts::{
    AreaPart, BlockId, DimensionStyle, DocumentSnapshotV2, Entity, EntityId, HatchPattern,
    MAX_LINE_WEIGHT, MAX_WIDTH_FACTOR, PointPart, RingGeometry, Vec2, width_factor_ok,
};

use super::Encoder;
use super::names::{dimension_style, hatch_pattern};
use crate::cbor::{Seg, key_order};
use crate::error::{Code, KcadError};
use crate::watch::{EVERY, Step};

/// Why a width factor is refused (a text's or an attribute definition's, §6.6).
pub(super) fn width_factor_words(w: f64) -> String {
    format!("genişlik çarpanı {w}; 0'dan büyük, en çok {MAX_WIDTH_FACTOR} olmalı")
}

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
    /// A multi-part area's or polyline's parts past its first (§6.6).
    Parts(&'d [AreaPart]),
    /// A multi-point object's points past its first (§6.6).
    PointParts(&'d [PointPart]),
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
                if let Some(parts) = &e.parts {
                    f.push(("parts", Val::PointParts(parts)));
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
                    // A polyline's parts (docs/adr/0174): open, of two vertices or more, without holes.
                    if matches!(entity, Entity::Polyline(_)) {
                        let wrong = parts.iter().enumerate().find_map(|(i, part)| {
                            if part.holes.is_some() {
                                Some((
                                    i,
                                    "çoklu çizginin parçasının adası (deliği) olamaz".to_owned(),
                                ))
                            } else if part.pts.len() < 2 {
                                Some((
                                    i,
                                    format!(
                                        "çoklu çizginin parçasının {} köşesi var; en az iki olmalı",
                                        part.pts.len()
                                    ),
                                ))
                            } else {
                                None
                            }
                        });
                        if let Some((i, why)) = wrong {
                            self.path.push(Seg::Name(kind));
                            self.path.push(Seg::Name("parts"));
                            self.path.push(Seg::Index(i));
                            return Err(self.fail(Code::BadValue, &why));
                        }
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
                if let Some(a) = e.align {
                    f.push(("align", Val::Name(a.name())));
                }
                if let Some(w) = e.width_factor {
                    // Not finite: the float's own refusal (`non_finite`), with its place.
                    if w.is_finite() && !width_factor_ok(w) {
                        self.path.push(Seg::Name(kind));
                        self.path.push(Seg::Name("widthFactor"));
                        return Err(self.fail(Code::BadValue, &width_factor_words(w)));
                    }
                    f.push(("widthFactor", Val::Float(w)));
                }
                if e.mask {
                    f.push(("mask", Val::Bool(true)));
                }
            }
            Entity::Dimension(e) => {
                // What a style needs, and what only a slope has (docs/adr/0147).
                let refuse = |this: &mut Self, code: Code, field: &'static str, words: &str| {
                    this.path.push(Seg::Name(kind));
                    this.path.push(Seg::Name(field));
                    Err(this.fail(code, words))
                };
                let slope = e.style == Some(DimensionStyle::Slope);
                let centred = matches!(
                    e.style,
                    Some(DimensionStyle::ArcLength | DimensionStyle::Jogged)
                );
                if centred && e.c.is_none() {
                    return refuse(self, Code::MissingField, "c", "zorunlu alan yok (c)");
                }
                if slope && (e.za.is_none() || e.zb.is_none()) {
                    let field = if e.za.is_none() { "za" } else { "zb" };
                    let words = format!("zorunlu alan yok ({field})");
                    return refuse(self, Code::MissingField, field, &words);
                }
                if !slope && (e.za.is_some() || e.zb.is_some()) {
                    let field = if e.za.is_some() { "za" } else { "zb" };
                    let words = "kot (za, zb) yalnız eğim ölçüsünde yazılır";
                    return refuse(self, Code::BadValue, field, words);
                }
                if e.style == Some(DimensionStyle::Ordinate)
                    && let Some(a) = e.angle
                    && a != 0.0
                    && a != 90.0
                {
                    let words = format!("koordinat ölçüsünün ekseni {a}; 0 (Y) ya da 90 (X) olmalı");
                    return refuse(self, Code::BadValue, "angle", &words);
                }
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
                if e.mask {
                    f.push(("mask", Val::Bool(true)));
                }
                if let Some(z) = e.za {
                    f.push(("za", Val::Float(z)));
                }
                if let Some(z) = e.zb {
                    f.push(("zb", Val::Float(z)));
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
            Entity::Leader(e) => {
                let refuse = |this: &mut Self, field: &'static str, words: &str| {
                    this.path.push(Seg::Name(kind));
                    this.path.push(Seg::Name(field));
                    Err(this.fail(Code::BadValue, words))
                };
                if e.pts.len() < 2 {
                    let words = format!("kılavuzun {} köşesi var; en az iki olmalı", e.pts.len());
                    return refuse(self, "pts", &words);
                }
                // Not finite: the float's own refusal (`non_finite`), with its place.
                if e.height.is_finite() && e.height <= 0.0 {
                    let words = format!("kılavuzun yüksekliği {}; 0'dan büyük olmalı", e.height);
                    return refuse(self, "height", &words);
                }
                f.push(("pts", Val::Points(&e.pts)));
                f.push(("height", Val::Float(e.height)));
                f.push(("rotation", Val::Float(e.rotation)));
                if let Some(t) = &e.text {
                    if t.is_empty() {
                        return refuse(
                            self,
                            "text",
                            "kılavuzun notu boş; notsuz kılavuzun not alanı yazılmaz",
                        );
                    }
                    f.push(("text", Val::Text(t)));
                }
                if let Some(a) = e.arrow {
                    f.push(("arrow", Val::Name(a.name())));
                }
                if e.mask {
                    f.push(("mask", Val::Bool(true)));
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
            Val::PointParts(parts) => {
                self.open(parts.len(), false)?;
                for (i, part) in parts.iter().enumerate() {
                    self.at(Seg::Index(i), |e| {
                        e.open(1 + usize::from(part.z.is_some()), true)?;
                        // p, z.
                        e.key("p");
                        e.at(Seg::Name("p"), |e| e.point(&part.p))?;
                        if let Some(z) = part.z {
                            e.key("z");
                            e.at(Seg::Name("z"), |e| e.float(z))?;
                        }
                        e.close();
                        Ok(())
                    })?;
                }
                self.close();
                Ok(())
            }
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
