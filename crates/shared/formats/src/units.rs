//! A file's length unit against a local project's (docs/adr/0165 §2): the
//! units DXF's $INSUNITS names, as metres, and one scale put on every
//! coordinate and length of a drawing's objects and block definitions, so
//! that the drawing keeps metres whatever the file was written in.
//!
//! A scale is a ratio of whole numbers (`mul ÷ div`), each exact in a
//! float64: a millimetre is ÷1000 and an inch ×254 ÷10000, as a value typed
//! in a local project's unit is turned into metres (`Format::to_metres`,
//! the web's `toMetres`). Angles, ratios, an insert's own scale and pen
//! widths (millimetres on paper) are not lengths in the drawing and stay.

use kentos_contracts::{
    AreaPart, AttributeDefinition, BlockDefinition, Bounds, DrawingUnit, Entity, HatchPatternType,
    RingGeometry, Vec2,
};

/// `value × mul ÷ div`: one length unit in another.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scale {
    pub mul: f64,
    pub div: f64,
}

impl Scale {
    pub const ONE: Scale = Scale { mul: 1.0, div: 1.0 };

    /// A value in the scale's unit.
    pub fn apply(self, v: f64) -> f64 {
        if self.mul == 1.0 {
            v / self.div
        } else if self.div == 1.0 {
            v * self.mul
        } else {
            v * self.mul / self.div
        }
    }

    pub fn is_one(self) -> bool {
        self.mul == self.div
    }

    fn point(self, p: &mut Vec2) {
        p.x = self.apply(p.x);
        p.y = self.apply(p.y);
    }

    fn points(self, pts: &mut [Vec2]) {
        for p in pts {
            self.point(p);
        }
    }

    fn heights(self, zs: &mut Option<Vec<Option<f64>>>) {
        for z in zs.iter_mut().flatten().flatten() {
            *z = self.apply(*z);
        }
    }

    fn height(self, z: &mut Option<f64>) {
        if let Some(z) = z {
            *z = self.apply(*z);
        }
    }

    fn ring(self, r: &mut RingGeometry) {
        self.points(&mut r.pts);
        self.heights(&mut r.zs);
    }

    fn part(self, a: &mut AreaPart) {
        self.points(&mut a.pts);
        self.heights(&mut a.zs);
        for h in a.holes.iter_mut().flatten() {
            self.ring(h);
        }
    }
}

/// Metres in one of a local project's drawing units.
pub fn from_unit(u: DrawingUnit) -> Scale {
    Scale {
        mul: 1.0,
        div: u.per_metre(),
    }
}

/// A local project's drawing unit in metres: what a writer writes.
pub fn to_unit(u: DrawingUnit) -> Scale {
    Scale {
        mul: u.per_metre(),
        div: 1.0,
    }
}

/// The unit an $INSUNITS code names, in metres, and its name; none for a
/// drawing without units (0) and the codes AutoCAD does not define.
pub fn insunits(code: i64) -> Option<(Scale, &'static str)> {
    let s = |mul: f64, div: f64| Scale { mul, div };
    Some(match code {
        1 => (s(254.0, 10_000.0), "inç"),
        2 => (s(3048.0, 10_000.0), "fit"),
        3 => (s(1_609_344.0, 1000.0), "mil"),
        4 => (s(1.0, 1000.0), "milimetre"),
        5 => (s(1.0, 100.0), "santimetre"),
        6 => (Scale::ONE, "metre"),
        7 => (s(1000.0, 1.0), "kilometre"),
        8 => (s(254.0, 1e10), "mikroinç"),
        9 => (s(254.0, 1e7), "binde bir inç"),
        10 => (s(9144.0, 10_000.0), "yarda"),
        11 => (s(1.0, 1e10), "angstrom"),
        12 => (s(1.0, 1e9), "nanometre"),
        13 => (s(1.0, 1e6), "mikrometre"),
        14 => (s(1.0, 10.0), "desimetre"),
        15 => (s(10.0, 1.0), "dekametre"),
        16 => (s(100.0, 1.0), "hektometre"),
        17 => (s(1e9, 1.0), "gigametre"),
        18 => (s(149_597_870_700.0, 1.0), "astronomik birim"),
        19 => (s(9_460_730_472_580_800.0, 1.0), "ışık yılı"),
        // 648 000 / π astronomical units, as the float64 nearest to it.
        20 => (s(3.085_677_581_491_367e16, 1.0), "parsek"),
        21 => (s(1200.0, 3937.0), "ABD ölçme fiti"),
        22 => (s(100.0, 3937.0), "ABD ölçme inci"),
        23 => (s(3600.0, 3937.0), "ABD ölçme yardası"),
        24 => (s(6_336_000.0, 3937.0), "ABD ölçme mili"),
        _ => return None,
    })
}

/// The $INSUNITS code of a local project's drawing unit.
pub fn insunits_of(u: DrawingUnit) -> i64 {
    match u {
        DrawingUnit::Mm => 4,
        DrawingUnit::Cm => 5,
        DrawingUnit::M => 6,
    }
}

/// The unit's name as a report says it: “milimetre”.
pub fn name_of(u: DrawingUnit) -> &'static str {
    match u {
        DrawingUnit::Mm => "milimetre",
        DrawingUnit::Cm => "santimetre",
        DrawingUnit::M => "metre",
    }
}

/// Every coordinate and length of `e` in the scale's unit.
pub fn entity(e: &mut Entity, s: Scale) {
    match e {
        Entity::Point(p) => {
            s.point(&mut p.p);
            s.height(&mut p.z);
        }
        Entity::Line(l) => {
            s.point(&mut l.a);
            s.point(&mut l.b);
            s.height(&mut l.za);
            s.height(&mut l.zb);
        }
        Entity::Polyline(p) | Entity::Polygon(p) => {
            s.points(&mut p.pts);
            s.heights(&mut p.zs);
            for h in p.holes.iter_mut().flatten() {
                s.ring(h);
            }
            for a in p.parts.iter_mut().flatten() {
                s.part(a);
            }
        }
        Entity::Circle(c) => {
            s.point(&mut c.c);
            c.r = s.apply(c.r);
        }
        Entity::Arc(a) => {
            s.point(&mut a.c);
            a.r = s.apply(a.r);
        }
        Entity::Ellipse(el) => {
            s.point(&mut el.c);
            s.point(&mut el.major);
        }
        Entity::Spline(sp) => s.points(&mut sp.pts),
        Entity::Xline(c) | Entity::Ray(c) => s.point(&mut c.p),
        Entity::Text(t) => {
            s.point(&mut t.p);
            t.height = s.apply(t.height);
        }
        Entity::Dimension(d) => {
            s.point(&mut d.a);
            s.point(&mut d.b);
            if let Some(c) = &mut d.c {
                s.point(c);
            }
            d.offset = s.apply(d.offset);
            d.height = s.apply(d.height);
            s.height(&mut d.za);
            s.height(&mut d.zb);
        }
        Entity::Hatch(h) => {
            s.points(&mut h.ring);
            for hole in h.holes.iter_mut().flatten() {
                s.points(hole);
            }
            // Only lines are apart by a length; the other kinds' spacing is 1 and unread.
            if matches!(
                h.pattern.kind,
                HatchPatternType::Lines | HatchPatternType::Cross
            ) {
                h.pattern.spacing = s.apply(h.pattern.spacing);
            }
            // A pattern's metres per unit of its definition: its definition stays.
            if let Some(scale) = &mut h.pattern.scale {
                *scale = s.apply(*scale);
            }
            if let Some(a) = &mut h.assoc {
                s.point(&mut a.seed);
            }
        }
        // Its definition is scaled with the drawing: its own scale stays.
        Entity::Insert(i) => s.point(&mut i.p),
        Entity::Leader(l) => {
            s.points(&mut l.pts);
            l.height = s.apply(l.height);
        }
        Entity::Table(t) => {
            s.point(&mut t.p);
            t.height = s.apply(t.height);
            for x in t.rows.iter_mut().chain(t.columns.iter_mut()) {
                *x = s.apply(*x);
            }
        }
        // Its clip is in its own fractions: unchanged (docs/adr/0192 §1).
        Entity::Image(i) => {
            s.point(&mut i.image.p);
            i.image.width = s.apply(i.image.width);
            i.image.height = s.apply(i.image.height);
        }
    }
}

/// A block definition in the scale's unit: its base point, its objects and
/// its attribute definitions, so that an insert, scaled with them, places
/// the same drawing.
pub fn block(b: &mut BlockDefinition, s: Scale) {
    s.point(&mut b.base);
    for e in &mut b.entities {
        entity(e, s);
    }
    for a in &mut b.attributes {
        attribute(a, s);
    }
}

fn attribute(a: &mut AttributeDefinition, s: Scale) {
    s.point(&mut a.p);
    a.height = s.apply(a.height);
}

/// A box in the scale's unit.
pub fn bounds(b: &mut Bounds, s: Scale) {
    b.min_x = s.apply(b.min_x);
    b.min_y = s.apply(b.min_y);
    b.max_x = s.apply(b.max_x);
    b.max_y = s.apply(b.max_y);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ratios are the definitions: an inch is 25.4 mm, a US survey foot
    /// 1200/3937 m; a millimetre is divided, as a typed one is.
    #[test]
    fn units_are_their_definitions() {
        let metres = |code: i64, v: f64| insunits(code).map(|(s, _)| s.apply(v));
        assert_eq!(metres(4, 250.0), Some(0.25));
        assert_eq!(metres(4, 0.1), Some(0.1 / 1000.0));
        assert_eq!(metres(5, 12.5), Some(0.125));
        assert_eq!(metres(6, 12.5), Some(12.5));
        assert_eq!(metres(7, 1.5), Some(1500.0));
        assert_eq!(metres(1, 100.0), Some(2.54));
        assert_eq!(metres(2, 10.0), Some(3.048));
        assert_eq!(metres(21, 3937.0), Some(1200.0));
        assert_eq!(metres(0, 1.0), None);
        assert_eq!(metres(25, 1.0), None);
        assert_eq!(metres(-1, 1.0), None);
        for u in [DrawingUnit::Mm, DrawingUnit::Cm, DrawingUnit::M] {
            let (scale, name) = insunits(insunits_of(u)).expect("a unit");
            assert_eq!((scale, name), (from_unit(u), name_of(u)));
            assert_eq!(to_unit(u).apply(from_unit(u).apply(12.0)), 12.0);
        }
        assert!(from_unit(DrawingUnit::M).is_one() && !to_unit(DrawingUnit::Mm).is_one());
    }
}
