//! One length unit in another, exactly, and every length of a drawing's
//! objects and block definitions put in it (docs/adr/0165 §2, docs/adr/0256
//! §1). Geometry is in metres; a local project's drawing unit is where the
//! user meets the numbers. A scale is a ratio of whole numbers (`mul ÷
//! div`), each exact in a float64: a millimetre is ÷1000, an inch ×254
//! ÷10000, as a value typed in a local project's unit is turned into metres.
//!
//! What a scale puts in the new unit: coordinates, lengths, radii, the
//! heights and sizes of texts, dimensions, leaders and tables, elevations,
//! a hatch's spacing and its pattern's metres per unit, a picture's and a
//! raster's frame, a curved text's curve, a paragraph's box, block
//! definitions with their attribute definitions. What it leaves: angles,
//! ratios (bulges, a dimension's and a leader's sizes, which are times
//! their height), an insert's own scale (its definition is scaled with the
//! drawing), paper sizes (pen widths), a route's measures (M, the route's
//! own unit) and a point cloud's points, which are in its files.

use crate::{
    AreaPart, AttributeDefinition, BlockDefinition, Bounds, DrawingUnit, Entity, HatchPatternType,
    RingGeometry, Vec2,
};

/// What every number of the command catalog's schemas is in (docs/adr/0256
/// §3): its `x-unit` is one of these, on the number or on the field holding
/// it (a point's `x`, `y` take the unit of the field that is the point).
/// Directions are counter-clockwise from the plane's east (x) axis unless a
/// field says otherwise.
pub const UNITS: &[(&str, &str)] = &[
    (
        "length",
        "Projenin düzleminde uzunluk ya da koordinat: metre, coğrafi sistemde derece (units.length).",
    ),
    (
        "area",
        "Projenin düzleminde alan: m², coğrafi sistemde derece² (units.area).",
    ),
    ("elevation", "Düşey metre: kot, z, yükseklik."),
    (
        "m",
        "Her sistemde metre: izdüşümün parametreleri, ölçme doğrulukları.",
    ),
    ("measure", "Rotanın ölçüsü (M), rotanın kendi biriminde."),
    (
        "constraint",
        "Ölçü kısıtının değeri: uzunlukta metre, açıda radyan.",
    ),
    (
        "parameter",
        "Kısıt parametresinin kendi biriminde: uzunluk çizim biriminde, açı projenin açı biriminde, sayı birimsiz.",
    ),
    (
        "paper-mm",
        "Kâğıtta milimetre: kalınlık, kâğıttaki yazı yüksekliği, pafta boyları.",
    ),
    ("px", "Ekranda mantıksal piksel."),
    (
        "deg",
        "Derece; yön ise doğudan saat yönünün tersine (alanın açıklaması başkasını söylemiyorsa).",
    ),
    (
        "rad",
        "Radyan; yön ise doğudan saat yönünün tersine (alanın açıklaması başkasını söylemiyorsa).",
    ),
    ("azimuth-deg", "Kuzeyden saat yönünde derece."),
    (
        "1",
        "Birimsiz sayı: çarpan, oran, büküm, saydamlık (0–1), ağırlık, öncelik, katsayı.",
    ),
    ("percent", "Yüzde (0–100)."),
    ("ppm", "Milyonda bir."),
    (
        "pattern",
        "Tarama deseninin tanımının birimi; taramanın ölçeğiyle çarpılınca metre.",
    ),
    ("count", "Adet."),
    ("index", "0'dan başlayan sıra."),
    ("id", "Çizimdeki yuva numarası (kalıcı kimlik değil)."),
    (
        "insunits",
        "DXF'in $INSUNITS birim kodu: 1 inç, 2 fit, 4 mm, 5 cm, 6 m, 21 ABD ölçme fiti …; 0 birimsiz.",
    ),
    (
        "epsg",
        "Koordinat sisteminin EPSG kodu; 0: yok (yerel proje).",
    ),
    ("scale", "Ölçeğin paydası: 1:N'nin N'si."),
    ("zoom", "Karo düzeyi."),
    ("bytes", "Bayt."),
    ("version", "Sürüm numarası."),
    (
        "value",
        "Verinin kendi birimi: rasterin değeri, kuralın eşiği, alanın değeri.",
    ),
    ("km/h", "Kilometre bölü saat."),
    ("system", "Adı verilen koordinat sisteminin kendi birimi."),
    ("arcsec", "Yay saniyesi (″)."),
    (
        "size-unit",
        "Birimi yanındaki sizeUnit alanının söylediği: piksel ya da projenin uzunluğu.",
    ),
    (
        "px/length",
        "Görünümün ölçeği: ekran pikseli bölü projenin uzunluk birimi (px/m).",
    ),
];

/// Whether `unit` is one of [`UNITS`].
pub fn is_unit(unit: &str) -> bool {
    UNITS.iter().any(|(u, _)| *u == unit)
}

/// The unit a DXF's `$INSUNITS` code names, in metres, and its name; none
/// for a drawing without units (0) and the codes AutoCAD does not define.
/// The ratios are the definitions: an inch is 25.4 mm, a US survey foot
/// 1200/3937 m (docs/adr/0165 §2, docs/adr/0257 §1).
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

/// The greatest common divisor.
fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// `value × mul ÷ div`: one length unit in another.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scale {
    pub mul: f64,
    pub div: f64,
}

impl Scale {
    pub const ONE: Scale = Scale { mul: 1.0, div: 1.0 };

    /// A local project's unit in metres: a value in `u` × this is metres.
    pub fn from_unit(u: DrawingUnit) -> Scale {
        let (mul, div) = u.metres();
        Scale { mul, div }
    }

    /// Metres in a local project's unit.
    pub fn to_unit(u: DrawingUnit) -> Scale {
        let (mul, div) = u.metres();
        Scale { mul: div, div: mul }
    }

    /// What keeps a drawing's numbers when its unit goes from `from` to
    /// `to` (docs/adr/0256 §1): the metres a number stood for in `from`
    /// become the metres it stands for in `to`. The units' ratio of whole
    /// numbers in lowest terms: ×1000 from mm to m, ÷10 from cm to mm,
    /// ×127 ÷5 from mm to in (docs/adr/0262 §1).
    pub fn keeping_numbers(from: DrawingUnit, to: DrawingUnit) -> Scale {
        let ((fm, fd), (tm, td)) = (from.metres(), to.metres());
        // Whole numbers below 2^53: exact in a float64, and in a u64.
        let (mul, div) = ((tm * fd) as u64, (td * fm) as u64);
        let g = gcd(mul, div).max(1);
        Scale {
            mul: (mul / g) as f64,
            div: (div / g) as f64,
        }
    }

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

    /// The factor as one number (its rounding when neither part is 1).
    pub fn factor(self) -> f64 {
        self.mul / self.div
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

/// Every coordinate and length of `e` in the scale's unit.
pub fn scale_entity(e: &mut Entity, s: Scale) {
    match e {
        Entity::Point(p) => {
            s.point(&mut p.p);
            s.height(&mut p.z);
            for part in p.parts.iter_mut().flatten() {
                s.point(&mut part.p);
                s.height(&mut part.z);
            }
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
            // A polyline's widths are lengths in the drawing (docs/adr/0258 §2).
            for w in p.widths.iter_mut().flatten().flatten() {
                *w = s.apply(*w);
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
        // Knots and weights have no unit; fit points and the tolerance are lengths, tangents directions.
        Entity::Nurbs(n) => {
            s.points(&mut n.ctrl);
            if let Some(f) = &mut n.fit {
                s.points(f);
            }
            if let Some(t) = &mut n.fit_tolerance {
                *t = s.apply(*t);
            }
        }
        Entity::Xline(c) | Entity::Ray(c) => s.point(&mut c.p),
        Entity::Text(t) => {
            s.point(&mut t.p);
            t.height = s.apply(t.height);
            if let Some(w) = &mut t.paragraph.box_width {
                *w = s.apply(*w);
            }
            // The curve is in the text's own frame, metres (docs/adr/0196 §1).
            if let Some(path) = &mut t.path {
                s.points(&mut path.pts);
            }
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
            // The exact boundary (docs/adr/0328 §1): its points, an ellipse's centre and axis, a curve's control
            // points; bulges, ratios, parameters, knots and weights have no length.
            for l in h.boundary.iter_mut().flatten() {
                s.points(&mut l.pts);
                for c in l.curves.iter_mut().flatten().flatten() {
                    match c {
                        crate::HatchCurve::Ellipse { c, major, .. } => {
                            s.point(c);
                            s.point(major);
                        }
                        crate::HatchCurve::Nurbs { ctrl, .. } => s.points(ctrl),
                    }
                }
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
            if let Some(w) = &mut t.frame {
                *w = s.apply(*w);
            }
        }
        // Its clip is in its own fractions: unchanged (docs/adr/0192 §1).
        Entity::Image(i) => {
            s.point(&mut i.image.p);
            i.image.width = s.apply(i.image.width);
            i.image.height = s.apply(i.image.height);
        }
        // Its corner and its pixels' steps: every number of its affine is a length (docs/adr/0204 §2).
        Entity::Raster(r) => {
            for v in &mut r.raster.affine {
                *v = s.apply(*v);
            }
        }
        // Its path, width, offset, components' thicknesses and height are lengths (docs/adr/0339 §1).
        Entity::Wall(e) => {
            let w = &mut e.wall;
            s.points(&mut w.pts);
            w.width = s.apply(w.width);
            w.offset = s.apply(w.offset);
            for c in &mut w.components {
                if let Some(t) = &mut c.thickness {
                    *t = s.apply(*t);
                }
            }
            s.height(&mut w.height);
        }
        // Its place along its wall, width, frame, jambs and heights are lengths (docs/adr/0341 §1).
        Entity::Opening(e) => {
            let o = &mut e.opening;
            o.at = s.apply(o.at);
            o.width = s.apply(o.width);
            if let Some(f) = &mut o.frame {
                *f = s.apply(*f);
            }
            s.points(&mut o.place);
            s.height(&mut o.sill);
            s.height(&mut o.head);
        }
        // A cloud's points are in its files, which no unit scales (docs/adr/0207 §11).
        Entity::PointCloud(_) | Entity::Record(_) => {}
    }
}

/// A block definition in the scale's unit: its base point, its objects and
/// its attribute definitions, so that an insert, scaled with them, places
/// the same drawing.
pub fn scale_block(b: &mut BlockDefinition, s: Scale) {
    s.point(&mut b.base);
    for e in &mut b.entities {
        scale_entity(e, s);
    }
    for a in &mut b.attributes {
        scale_attribute(a, s);
    }
}

fn scale_attribute(a: &mut AttributeDefinition, s: Scale) {
    s.point(&mut a.p);
    a.height = s.apply(a.height);
}

/// A box in the scale's unit.
pub fn scale_bounds(b: &mut Bounds, s: Scale) {
    b.min_x = s.apply(b.min_x);
    b.min_y = s.apply(b.min_y);
    b.max_x = s.apply(b.max_x);
    b.max_y = s.apply(b.max_y);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The factor keeping the numbers is one whole number each way, so a
    /// value goes there and back unchanged.
    /// Inches and feet keep the numbers by their ratio in lowest terms
    /// (docs/adr/0262 §1): an inch is 25.4 mm, a foot 12 inches.
    #[test]
    fn inches_and_feet_keep_the_numbers_by_their_ratio() {
        use DrawingUnit::{Ft, In, M, Mm};
        for (from, to, mul, div) in [
            (Mm, In, 127.0, 5.0),
            (In, Mm, 5.0, 127.0),
            (In, Ft, 12.0, 1.0),
            (Ft, M, 1250.0, 381.0),
        ] {
            let s = Scale::keeping_numbers(from, to);
            assert_eq!((s.mul, s.div), (mul, div), "{from:?} → {to:?}");
        }
        assert_eq!(Scale::from_unit(In).apply(10.0), 0.254);
        assert_eq!(Scale::from_unit(Ft).apply(1.0), 0.3048);
    }

    #[test]
    fn keeping_the_numbers_is_one_whole_factor() {
        use DrawingUnit::{Cm, M, Mm};
        let cases = [
            (Mm, M, 1000.0, 1.0),
            (M, Mm, 1.0, 1000.0),
            (Cm, Mm, 1.0, 10.0),
            (Mm, Cm, 10.0, 1.0),
            (Cm, M, 100.0, 1.0),
            (M, Cm, 1.0, 100.0),
            (M, M, 1.0, 1.0),
        ];
        for (from, to, mul, div) in cases {
            let s = Scale::keeping_numbers(from, to);
            assert_eq!((s.mul, s.div), (mul, div), "{from:?} → {to:?}");
            let back = Scale::keeping_numbers(to, from);
            for v in [0.1, 4.0, 1234.567, -0.003] {
                assert_eq!(back.apply(s.apply(v)), v, "{v} {from:?} → {to:?} → back");
            }
        }
        // The shown number stays: 4000 mm is 4 m of geometry; kept as 4000 m it is 4000 m.
        let g = Scale::from_unit(Mm).apply(4000.0);
        assert_eq!(g, 4.0);
        assert_eq!(Scale::keeping_numbers(Mm, M).apply(g), 4000.0);
    }
}
