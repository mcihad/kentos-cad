//! A KentOS dimension as a DXF DIMENSION, and back (ADR 0009 “DXF yazma”):
//! the dimension type and the definition points a CAD program measures
//! from, taken from the shared core's layout of the dimension. The writer
//! and the reader compute them the same way, so the reader can tell a
//! dimension as KentOS wrote it from one another program has changed.
//!
//! The kinds of docs/adr/0147 §8, as AutoCAD writes them (read from its own
//! files): an ordinate is a DIMENSION of type 6 measured from (0, 0) (10),
//! its point (13) and its line's end (14), bit 64 when it gives the east
//! (AutoCAD's X, KentOS's Y); an arc length an ARC_DIMENSION of type 5, a
//! point on its dimension arc (10), the arc's ends counter-clockwise (13,
//! 14), its centre (15) and their angles (40, 41); a jogged radius a
//! LARGE_RADIAL_DIMENSION of type 9, the true centre (10), the centre shown
//! (13), the jog's middle (14) and the point on the arc (15). Semt and Eğim
//! have no DXF kind: an aligned DIMENSION drawn by its block, KentOS's data
//! giving it back. Another program's ordinate (from the origin), arc length
//! and jogged radius come in as KentOS's own (`foreign`).

use kentos_contracts::{DimensionEntity, DimensionStyle, EntityBase, Vec2};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::geom::dimension::{
    DimensionGeom, DimensionLayout, dimension_fault, layout_dimension,
};

use super::strings::mtext_lines;
use super::xdata::{DimMeta, caret_decode};
use crate::geom::v;
use crate::math::{atan2, hypot, norm_angle, sin_cos_deg};

/// DXF dimension types (group 70, low bits).
pub const ROTATED: i64 = 0;
pub const ALIGNED: i64 = 1;
pub const DIAMETER: i64 = 3;
pub const RADIUS: i64 = 4;
pub const ANGULAR_3P: i64 = 5;
/// An ordinate (a DIMENSION), and the bit that makes it measure the east
/// (AutoCAD's X type; KentOS's Y).
pub const ORDINATE: i64 = 6;
pub const ORDINATE_EAST: i64 = 64;
/// An ARC_DIMENSION's type (AutoCAD writes 5, as an angle's).
pub const ARC_LENGTH: i64 = 5;
/// A LARGE_RADIAL_DIMENSION's type.
pub const LARGE_RADIAL: i64 = 9;
/// Group 70 bit: the block is this dimension's alone.
pub const OWN_BLOCK: i64 = 32;
/// Group 70: the type without its bits (32 and up).
const TYPE: i64 = 31;

/// What the report says when another program's ordinate is not measured
/// from the drawing's origin.
pub const ORDINATE_ORIGIN: &str =
    "koordinat ölçüsünün başlangıcı (0, 0) değil; değeri göreli olduğundan bloğunun çizgileri ve değeriyle alındı";

/// Where a DIMENSION puts a KentOS dimension.
#[derive(Clone, Debug, PartialEq)]
pub struct Definition {
    /// The DXF entity: DIMENSION, ARC_DIMENSION or LARGE_RADIAL_DIMENSION.
    pub entity: &'static str,
    /// The DXF type (group 70 without the flags).
    pub kind: i64,
    /// An ordinate of the east (bit 64 of group 70).
    pub east: bool,
    /// Group 10: on the dimension line at the second extension line (linear
    /// kinds), on the arc (angular), the centre (radius), the far side of the
    /// circle (diameter).
    pub p10: Vec2,
    /// Groups 13, 14: the measured points; 15: the angle's vertex, or the point on the circle.
    pub p13: Option<Vec2>,
    pub p14: Option<Vec2>,
    pub p15: Option<Vec2>,
    /// Group 50: the rotated dimension's direction in degrees (none: 0).
    pub angle: Option<f64>,
    /// Group 40: the radius or diameter dimension's leader past the circle.
    pub leader: Option<f64>,
    /// An ARC_DIMENSION's arc: its ends' angles, radians (40, 41).
    pub angles: Option<(f64, f64)>,
}

fn core(p: Vec2) -> CoreVec2 {
    CoreVec2::new(p.x, p.y)
}

fn app(p: CoreVec2) -> Vec2 {
    v(p.x, p.y)
}

/// The style's name as the core's layout takes it (None: aligned).
pub fn style_name(s: Option<DimensionStyle>) -> Option<&'static str> {
    s.map(DimensionStyle::name)
}

pub fn style_from_name(s: &str) -> Option<Option<DimensionStyle>> {
    if s.is_empty() {
        return Some(None);
    }
    DimensionStyle::from_name(s).map(Some)
}

/// The dimension as the core lays it out (lines, ticks or arrowheads, where
/// the value goes), in its look (docs/adr/0183 §3).
pub fn layout(d: &DimensionEntity) -> Option<DimensionLayout> {
    layout_dimension(&DimensionGeom {
        a: core(d.a),
        b: core(d.b),
        offset: d.offset,
        height: d.height,
        style: style_name(d.style).map(str::to_string),
        angle: d.angle,
        c: d.c.map(core),
        za: d.za,
        zb: d.zb,
        look: look_of(&d.look),
    })
}

/// A dimension's look as the core lays it out (the arrowheads, sizes and the
/// value's place; what writes the value is the app's).
pub fn look_of(l: &kentos_contracts::DimensionLook) -> kentos_geometry_core::geom::dimension::Look {
    use kentos_geometry_core::geom::dimension::{Arrow, Look};
    Look {
        style: l.dim_style.clone(),
        arrow: l.arrow.and_then(|a| Arrow::from_name(a.name())),
        arrow_size: l.arrow_size,
        ext_offset: l.ext_offset,
        ext_beyond: l.ext_beyond,
        text_gap: l.text_gap,
        centre: l.text_place == Some(kentos_contracts::DimensionTextPlace::Centre),
        decimals: l.decimals,
        unit: l.unit.map(|u| u.mark().to_owned()),
        prefix: l.prefix.clone(),
        suffix: l.suffix.clone(),
        font: l
            .font
            .map(|f| kentos_geometry_core::text::Font::from_id(f.id())),
    }
}

/// The DXF type and definition points of a dimension laid out as `l`.
pub fn definition(d: &DimensionEntity, l: &DimensionLayout) -> Definition {
    let plain = Definition {
        entity: "DIMENSION",
        kind: ALIGNED,
        east: false,
        p10: app(l.d2),
        p13: Some(d.a),
        p14: Some(d.b),
        p15: None,
        angle: None,
        leader: None,
        angles: None,
    };
    match d.style {
        None | Some(DimensionStyle::Aligned) => plain,
        Some(DimensionStyle::Linear) => Definition {
            kind: ROTATED,
            angle: d.angle,
            ..plain
        },
        Some(DimensionStyle::Angular) => Definition {
            kind: ANGULAR_3P,
            p10: app(l.handle),
            p15: d.c,
            ..plain
        },
        Some(DimensionStyle::Radius) => Definition {
            kind: RADIUS,
            p10: d.a,
            p13: None,
            p14: None,
            p15: Some(d.b),
            leader: Some(d.offset.max(0.0)),
            angle: None,
            ..plain
        },
        Some(DimensionStyle::Diameter) => Definition {
            kind: DIAMETER,
            p10: app(l.d1),
            p13: None,
            p14: None,
            p15: Some(d.b),
            leader: Some(d.offset.max(0.0)),
            angle: None,
            ..plain
        },
        // From the drawing's origin: its point, its line's end.
        Some(DimensionStyle::Ordinate) => Definition {
            kind: ORDINATE,
            east: d.angle.unwrap_or(0.0) == 0.0,
            p10: v(0.0, 0.0),
            ..plain
        },
        // A point on the dimension arc (its middle), the arc's ends, its centre and their angles.
        Some(DimensionStyle::ArcLength) => {
            let c = d.c.unwrap_or(d.a);
            let angle = |p: Vec2| norm_angle(atan2(p.y - c.y, p.x - c.x));
            Definition {
                entity: "ARC_DIMENSION",
                kind: ARC_LENGTH,
                p10: app(l.handle),
                p15: d.c,
                angles: Some((angle(d.a), angle(d.b))),
                ..plain
            }
        }
        // The true centre, the centre shown, the jog's middle, the point on the arc.
        Some(DimensionStyle::Jogged) => Definition {
            entity: "LARGE_RADIAL_DIMENSION",
            kind: LARGE_RADIAL,
            p10: d.a,
            p13: d.c,
            p14: Some(jog_middle(d)),
            p15: Some(d.b),
            ..plain
        },
        // No DXF kind: an aligned one that its block draws, KentOS's data giving it back.
        Some(DimensionStyle::Azimuth | DimensionStyle::Slope) => plain,
    }
}

/// A jogged radius's jog, its middle (where AutoCAD keeps it): from the
/// centre shown along the radius for the offset (kept within its room), then
/// halfway across the jog (docs/adr/0147 §2).
fn jog_middle(d: &DimensionEntity) -> Vec2 {
    let (a, b) = (d.a, d.b);
    let c = d.c.unwrap_or(a);
    let r = hypot(b.x - a.x, b.y - a.y);
    if !(r > 0.0) {
        return b;
    }
    let u = v((b.x - a.x) / r, (b.y - a.y) / r);
    let n = v(-u.y, u.x);
    let s = (c.x - b.x) * n.x + (c.y - b.y) * n.y;
    let t = (b.x - c.x) * u.x + (b.y - c.y) * u.y;
    let along = d.offset.max(0.0).min((t - s.abs()).max(0.0)) + s.abs() / 2.0;
    v(c.x + u.x * along - n.x * s / 2.0, c.y + u.y * along - n.y * s / 2.0)
}

/// What a DIMENSION (an ARC_DIMENSION, a LARGE_RADIAL_DIMENSION) says, as the reader found it.
#[derive(Clone, Debug, PartialEq)]
pub struct Groups {
    /// The entity's name.
    pub entity: String,
    /// Group 70 as written (type and flags).
    pub flags: i64,
    /// Points as found; None when missing or unreadable.
    pub p10: Option<Vec2>,
    pub p13: Option<Vec2>,
    pub p14: Option<Vec2>,
    pub p15: Option<Vec2>,
    pub angle: Option<f64>,
    /// Group 1, in MTEXT's notation ("": the measured value).
    pub text: String,
}

/// The KentOS dimension a DIMENSION with KentOS's data is, while the
/// DIMENSION still has the entity, type and definition point KentOS wrote
/// for it (to the bit); None when another program changed it. `mask`: KentOS's
/// data says its value is drawn over the background.
pub fn read_back(g: &Groups, k: &DimMeta, base: EntityBase, mask: bool) -> Option<DimensionEntity> {
    let style = style_from_name(&k.style)?;
    let (a, b, c, angle) = match style {
        None | Some(DimensionStyle::Aligned) => (g.p13?, g.p14?, None, None),
        Some(DimensionStyle::Linear) => (g.p13?, g.p14?, None, g.angle),
        Some(DimensionStyle::Angular) => (g.p13?, g.p14?, Some(g.p15?), None),
        Some(DimensionStyle::Radius) => (g.p10?, g.p15?, None, None),
        Some(DimensionStyle::Diameter) => {
            let (x, y) = k.center?;
            (v(x, y), g.p15?, None, None)
        }
        Some(DimensionStyle::Ordinate) => {
            let east = g.flags & ORDINATE_EAST != 0;
            (g.p13?, g.p14?, None, Some(if east { 0.0 } else { 90.0 }))
        }
        Some(DimensionStyle::ArcLength) => (g.p13?, g.p14?, Some(g.p15?), None),
        Some(DimensionStyle::Jogged) => (g.p10?, g.p15?, Some(g.p13?), None),
        Some(DimensionStyle::Azimuth | DimensionStyle::Slope) => (g.p13?, g.p14?, None, None),
    };
    // The text as KentOS had it, while group 1 still says it; else what group 1 says now.
    let text = match &k.text {
        Some(t) if mtext_value(t) == g.text => Some(t.clone()),
        _ if g.text.is_empty() => None,
        _ => Some(mtext_lines(&caret_decode(&g.text)).join("\n")),
    };
    let slope = style == Some(DimensionStyle::Slope);
    let d = DimensionEntity {
        base,
        a,
        b,
        offset: k.offset,
        height: k.height,
        text,
        style,
        angle,
        c,
        mask,
        za: k.za.filter(|_| slope),
        zb: k.zb.filter(|_| slope),
        look: Default::default(),
    };
    let def = definition(&d, &layout(&d)?);
    let same = |p: Vec2, q: Vec2| p.x == q.x && p.y == q.y;
    (def.entity == g.entity
        && def.kind == g.flags & TYPE
        && def.east == (g.flags & ORDINATE_EAST != 0 && def.kind == ORDINATE)
        && same(def.p10, g.p10?))
    .then_some(d)
}

/// What becomes of another program's dimension (docs/adr/0147 §8).
#[derive(Clone, Debug, PartialEq)]
pub enum Foreign {
    /// KentOS's own of it.
    Taken(Box<DimensionEntity>),
    /// Its block draws it; what the report says, when it says something.
    Block(Option<&'static str>),
}

/// Another program's dimension as KentOS's own, from its groups: an ordinate
/// measured from the drawing's origin (its point 13, its line's end 14, the
/// east when bit 64 is set), an arc length (its arc's ends 13 and 14, its
/// centre 15, the dimension arc through 10) and a jogged radius (the true
/// centre 10, the point on the arc 15, the centre shown 13, the jog's
/// middle 14). `height` is its style's text height, `mask` its fill. Any
/// other kind, or one KentOS cannot draw, is left to its block.
pub fn foreign(g: &Groups, base: EntityBase, height: f64, mask: bool) -> Foreign {
    let made = |a: Vec2, b: Vec2, offset: f64, style: DimensionStyle, angle: Option<f64>, c: Option<Vec2>| {
        DimensionEntity {
            base: base.clone(),
            a,
            b,
            offset,
            height,
            // "" is the measured value, and so is a text around it ("<>", "R<>"): KentOS writes its own
            // value with its own prefix; another text is the dimension's own.
            text: (!g.text.is_empty() && !g.text.contains("<>"))
                .then(|| mtext_lines(&caret_decode(&g.text)).join("\n")),
            style: Some(style),
            angle,
            c,
            mask,
            za: None,
            zb: None,
            look: Default::default(),
        }
    };
    let dist = |p: Vec2, q: Vec2| hypot(p.x - q.x, p.y - q.y);
    let d = match (g.entity.as_str(), g.flags & TYPE) {
        ("DIMENSION", ORDINATE) => {
            let (Some(o), Some(a), Some(b)) = (g.p10, g.p13, g.p14) else {
                return Foreign::Block(None);
            };
            if o.x != 0.0 || o.y != 0.0 {
                return Foreign::Block(Some(ORDINATE_ORIGIN));
            }
            let east = g.flags & ORDINATE_EAST != 0;
            made(a, b, 0.0, DimensionStyle::Ordinate, Some(if east { 0.0 } else { 90.0 }), None)
        }
        ("ARC_DIMENSION", _) => {
            let (Some(p), Some(a), Some(b), Some(c)) = (g.p10, g.p13, g.p14, g.p15) else {
                return Foreign::Block(None);
            };
            made(a, b, dist(p, c) - dist(a, c), DimensionStyle::ArcLength, None, Some(c))
        }
        ("LARGE_RADIAL_DIMENSION", _) => {
            let (Some(a), Some(shown), Some(jog), Some(b)) = (g.p10, g.p13, g.p14, g.p15) else {
                return Foreign::Block(None);
            };
            let r = dist(a, b);
            if !(r > 0.0) {
                return Foreign::Block(None);
            }
            // The jog's distance from the centre shown, along the radius (`jog_middle` backwards).
            let u = v((b.x - a.x) / r, (b.y - a.y) / r);
            let n = v(-u.y, u.x);
            let s = (shown.x - b.x) * n.x + (shown.y - b.y) * n.y;
            let t = (b.x - shown.x) * u.x + (b.y - shown.y) * u.y;
            let along = (jog.x - shown.x) * u.x + (jog.y - shown.y) * u.y - s.abs() / 2.0;
            let offset = along.max(0.0).min((t - s.abs()).max(0.0));
            made(a, b, offset, DimensionStyle::Jogged, None, Some(shown))
        }
        _ => return Foreign::Block(None),
    };
    let geom = DimensionGeom {
        a: core(d.a),
        b: core(d.b),
        offset: d.offset,
        height: d.height,
        style: style_name(d.style).map(str::to_string),
        angle: d.angle,
        c: d.c.map(core),
        za: None,
        zb: None,
        look: Default::default(),
    };
    if !(d.height > 0.0) || dimension_fault(&geom).is_some() || layout_dimension(&geom).is_none() {
        return Foreign::Block(Some("KentOS bu ölçüyü çizemiyor; bloğunun çizgileri ve değeriyle alındı"));
    }
    Foreign::Taken(Box::new(d))
}

/// Where the value's middle is (group 11, the MTEXT in the block): the
/// layout puts its baseline centre `height · 0.35` off the line.
pub fn text_middle(l: &DimensionLayout, height: f64) -> Vec2 {
    let (s, c) = sin_cos_deg(l.rotation);
    v(
        l.text_at.x - s * height / 2.0,
        l.text_at.y + c * height / 2.0,
    )
}

/// Text for MTEXT and a dimension's own text: MTEXT's codes kept literal
/// (backslash, braces), the caret in DXF's notation, "%%" codes literal
/// (as TEXT does), control characters as spaces (MTEXT breaks lines with
/// its own code). The reader's MTEXT cleaning undoes it.
pub fn mtext_value(s: &str) -> String {
    let percent = s.contains("%%");
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            c if c.is_control() => out.push(' '),
            '\\' => out.push_str("\\\\"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '^' => out.push_str("^ "),
            '%' if percent => out.push_str("%%%"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dxf::strings::mtext_lines;
    use crate::dxf::xdata::caret_decode;

    #[test]
    fn mtext_values_read_back_as_written() {
        for s in [
            "10.00",
            "R 2.500",
            "Ø 5",
            "a\\b {c} ^d %%d 45%",
            "%%c 100",
            "Çağ 33°",
        ] {
            let back = mtext_lines(&caret_decode(&mtext_value(s)));
            assert_eq!(back, vec![s.to_string()], "{s}");
        }
    }
}
