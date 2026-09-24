//! A KentOS dimension as a DXF DIMENSION, and back (ADR 0009 “DXF yazma”):
//! the dimension type and the definition points a CAD program measures
//! from, taken from the shared core's layout of the dimension. The writer
//! and the reader compute them the same way, so the reader can tell a
//! dimension as KentOS wrote it from one another program has changed.

use kentos_contracts::{DimensionEntity, DimensionStyle, EntityBase, Vec2};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::geom::dimension::{DimensionGeom, DimensionLayout, layout_dimension};

use super::strings::mtext_lines;
use super::xdata::{DimMeta, caret_decode};
use crate::geom::v;
use crate::math::sin_cos_deg;

/// DXF dimension types (group 70, low bits).
pub const ROTATED: i64 = 0;
pub const ALIGNED: i64 = 1;
pub const DIAMETER: i64 = 3;
pub const RADIUS: i64 = 4;
pub const ANGULAR_3P: i64 = 5;
/// Group 70 bit: the block is this dimension's alone.
pub const OWN_BLOCK: i64 = 32;

/// Where a DIMENSION puts a KentOS dimension.
#[derive(Clone, Debug, PartialEq)]
pub struct Definition {
    /// The DXF type (group 70 without the flags).
    pub kind: i64,
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
}

fn core(p: Vec2) -> CoreVec2 {
    CoreVec2::new(p.x, p.y)
}

fn app(p: CoreVec2) -> Vec2 {
    v(p.x, p.y)
}

/// The style's name as the core's layout takes it (None: aligned).
pub fn style_name(s: Option<DimensionStyle>) -> Option<&'static str> {
    s.map(|s| match s {
        DimensionStyle::Aligned => "aligned",
        DimensionStyle::Linear => "linear",
        DimensionStyle::Angular => "angular",
        DimensionStyle::Radius => "radius",
        DimensionStyle::Diameter => "diameter",
    })
}

pub fn style_from_name(s: &str) -> Option<Option<DimensionStyle>> {
    Some(match s {
        "" => None,
        "aligned" => Some(DimensionStyle::Aligned),
        "linear" => Some(DimensionStyle::Linear),
        "angular" => Some(DimensionStyle::Angular),
        "radius" => Some(DimensionStyle::Radius),
        "diameter" => Some(DimensionStyle::Diameter),
        _ => return None,
    })
}

/// The dimension as the core lays it out (lines, ticks, where the value goes).
pub fn layout(d: &DimensionEntity) -> Option<DimensionLayout> {
    layout_dimension(&DimensionGeom {
        a: core(d.a),
        b: core(d.b),
        offset: d.offset,
        height: d.height,
        style: style_name(d.style).map(str::to_string),
        angle: d.angle,
        c: d.c.map(core),
    })
}

/// The DXF type and definition points of a dimension laid out as `l`.
pub fn definition(d: &DimensionEntity, l: &DimensionLayout) -> Definition {
    let plain = Definition {
        kind: ALIGNED,
        p10: app(l.d2),
        p13: Some(d.a),
        p14: Some(d.b),
        p15: None,
        angle: None,
        leader: None,
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
        },
        Some(DimensionStyle::Diameter) => Definition {
            kind: DIAMETER,
            p10: app(l.d1),
            p13: None,
            p14: None,
            p15: Some(d.b),
            leader: Some(d.offset.max(0.0)),
            angle: None,
        },
    }
}

/// What a DIMENSION says, as the reader found it.
#[derive(Clone, Debug, PartialEq)]
pub struct Groups {
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
/// DIMENSION still has the type and definition point KentOS wrote for it
/// (to the bit); None when another program changed it.
pub fn read_back(g: &Groups, k: &DimMeta, base: EntityBase) -> Option<DimensionEntity> {
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
    };
    // The text as KentOS had it, while group 1 still says it; else what group 1 says now.
    let text = match &k.text {
        Some(t) if mtext_value(t) == g.text => Some(t.clone()),
        _ if g.text.is_empty() => None,
        _ => Some(mtext_lines(&caret_decode(&g.text)).join("\n")),
    };
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
    };
    let def = definition(&d, &layout(&d)?);
    let same = |p: Vec2, q: Vec2| p.x == q.x && p.y == q.y;
    (def.kind == g.flags & 7 && same(def.p10, g.p10?)).then_some(d)
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
