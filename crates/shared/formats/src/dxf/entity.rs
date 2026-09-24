//! One DXF entity's groups read into a small model the converter works
//! from. Coordinates stay as the file wrote them (object coordinates where
//! DXF uses them); nothing is transformed here. A number that does not read
//! as a finite number makes the whole entity unusable (reported by the
//! caller), never a zero.

use super::hatch::{Hatch, parse_hatch};
use super::lexer::Pair;
use super::strings::Decoder;
use crate::num::{parse_int, parse_real};

pub type P3 = [f64; 3];

/// Colour as DXF gives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    ByLayer,
    ByBlock,
    Aci(u8),
    /// 0xRRGGBB.
    True(i64),
}

#[derive(Clone, Debug)]
pub struct Common {
    pub layer: String,
    pub color: Color,
    pub extrusion: P3,
    pub paper: bool,
    pub invisible: bool,
}

#[derive(Clone, Debug)]
pub struct Vertex {
    pub p: P3,
    pub bulge: f64,
    pub flags: i64,
}

#[derive(Clone, Debug)]
pub enum Kind {
    Line {
        a: P3,
        b: P3,
    },
    Point {
        p: P3,
    },
    Circle {
        c: P3,
        r: f64,
    },
    /// Degrees, counter-clockwise in object coordinates.
    Arc {
        c: P3,
        r: f64,
        a0: f64,
        a1: f64,
    },
    Ellipse {
        c: P3,
        major: P3,
        ratio: f64,
        t0: f64,
        t1: f64,
    },
    LwPolyline {
        pts: Vec<[f64; 2]>,
        bulges: Vec<f64>,
        closed: bool,
        elevation: f64,
        width: bool,
    },
    Polyline {
        flags: i64,
        verts: Vec<Vertex>,
        elevation: f64,
        width: bool,
    },
    Spline {
        flags: i64,
        degree: i64,
        knots: Vec<f64>,
        weights: Vec<f64>,
        ctrl: Vec<P3>,
        fit: Vec<P3>,
    },
    Text {
        p: P3,
        p2: Option<P3>,
        height: f64,
        rotation: f64,
        text: String,
        halign: i64,
        valign: i64,
        width: f64,
        style: String,
        hidden: bool,
    },
    MText {
        p: P3,
        height: f64,
        attach: i64,
        xdir: Option<P3>,
        rotation: Option<f64>,
        text: String,
        spacing: f64,
        style: String,
    },
    /// SOLID and TRACE (object coordinates, corner order 1 2 4 3) or 3DFACE (world, 1 2 3 4).
    Face {
        pts: [P3; 4],
        solid: bool,
    },
    /// `bad_attribs`: attributes that could not be read (reason, line), reported with the insert.
    Insert {
        name: String,
        p: P3,
        scale: P3,
        rotation: f64,
        cols: i64,
        rows: i64,
        dc: f64,
        dr: f64,
        attribs: Vec<Parsed>,
        bad_attribs: Vec<(String, u32)>,
    },
    Hatch(Box<Hatch>),
    /// DIMENSION, ACAD_TABLE and others drawn by an anonymous block.
    Block {
        block: String,
        what: &'static str,
    },
    Xline {
        p: P3,
        dir: P3,
        ray: bool,
    },
    Leader {
        pts: Vec<P3>,
    },
    Unsupported(String),
}

#[derive(Clone, Debug)]
pub struct Parsed {
    pub kind: Kind,
    pub common: Common,
    pub line: u32,
    pub name: String,
}

/// Why an entity could not be read.
pub struct Unreadable {
    pub reason: String,
}

fn bad(reason: &str) -> Unreadable {
    Unreadable {
        reason: reason.to_string(),
    }
}

/// Reads groups by code, remembering the first bad number.
pub struct Groups<'g, 'a> {
    pub list: &'g [Pair<'a>],
    pub dec: Decoder,
}

impl<'g, 'a> Groups<'g, 'a> {
    /// The value of the first group with this code, as a number.
    pub fn num(&self, code: i32) -> Result<Option<f64>, Unreadable> {
        match self.list.iter().find(|p| p.code == code) {
            None => Ok(None),
            Some(p) => parse_real(p.text())
                .map(Some)
                .ok_or_else(|| bad(&format!("sayı okunamadı (grup {code}, satır {})", p.line))),
        }
    }

    pub fn num_or(&self, code: i32, default: f64) -> Result<f64, Unreadable> {
        Ok(self.num(code)?.unwrap_or(default))
    }

    pub fn int(&self, code: i32) -> i64 {
        self.list
            .iter()
            .find(|p| p.code == code)
            .and_then(|p| parse_int(p.text()))
            .unwrap_or(0)
    }

    pub fn has(&self, code: i32) -> bool {
        self.list.iter().any(|p| p.code == code)
    }

    pub fn string(&self, code: i32) -> String {
        self.list
            .iter()
            .find(|p| p.code == code)
            .map(|p| self.dec.string(p.value))
            .unwrap_or_default()
    }

    /// A point from its x code (y is x + 10, z is x + 20).
    pub fn point(&self, x: i32) -> Result<Option<P3>, Unreadable> {
        let Some(px) = self.num(x)? else {
            return Ok(None);
        };
        Ok(Some([
            px,
            self.num_or(x + 10, 0.0)?,
            self.num_or(x + 20, 0.0)?,
        ]))
    }

    pub fn point_or_zero(&self, x: i32) -> Result<P3, Unreadable> {
        Ok(self.point(x)?.unwrap_or([0.0, 0.0, 0.0]))
    }
}

/// Every value of a repeated point code in order (control points, leader vertices).
fn points(list: &[Pair<'_>], x: i32) -> Result<Vec<P3>, Unreadable> {
    let mut out: Vec<P3> = Vec::new();
    for p in list {
        let v = || {
            parse_real(p.text()).ok_or_else(|| {
                bad(&format!(
                    "sayı okunamadı (grup {}, satır {})",
                    p.code, p.line
                ))
            })
        };
        if p.code == x {
            out.push([v()?, 0.0, 0.0]);
        } else if (p.code == x + 10 || p.code == x + 20)
            && let Some(last) = out.last_mut()
        {
            last[if p.code == x + 10 { 1 } else { 2 }] = v()?;
        }
    }
    Ok(out)
}

fn reals(list: &[Pair<'_>], code: i32) -> Result<Vec<f64>, Unreadable> {
    list.iter()
        .filter(|p| p.code == code)
        .map(|p| {
            parse_real(p.text())
                .ok_or_else(|| bad(&format!("sayı okunamadı (grup {code}, satır {})", p.line)))
        })
        .collect()
}

/// The groups before extended data (1001) and embedded objects (101): what
/// follows uses the same codes for other things (an embedded object's 10 is
/// not the entity's point).
fn own_groups<'g, 'a>(list: &'g [Pair<'a>]) -> &'g [Pair<'a>] {
    let end = list
        .iter()
        .position(|p| p.code == 1001 || p.code == 101)
        .unwrap_or(list.len());
    &list[..end]
}

fn common(list: &[Pair<'_>], dec: Decoder) -> Result<Common, Unreadable> {
    let g = Groups { list, dec };
    let color = match (g.has(420), g.has(62)) {
        (true, _) => Color::True(g.int(420)),
        (false, true) => match g.int(62) {
            0 => Color::ByBlock,
            256 => Color::ByLayer,
            n => Color::Aci(u8::try_from(n.unsigned_abs().min(255)).unwrap_or(7)),
        },
        _ => Color::ByLayer,
    };
    let layer = g.string(8);
    Ok(Common {
        layer: if layer.trim().is_empty() {
            "0".to_string()
        } else {
            layer
        },
        color,
        extrusion: [
            g.num_or(210, 0.0)?,
            g.num_or(220, 0.0)?,
            g.num_or(230, 1.0)?,
        ],
        paper: g.int(67) == 1,
        invisible: g.int(60) == 1,
    })
}

/// An entity from its type name and groups; `after` are the VERTEX or ATTRIB entities that followed it.
pub fn parse(
    name: &str,
    line: u32,
    list: &[Pair<'_>],
    dec: Decoder,
    after: Vec<(u32, Vec<Pair<'_>>)>,
    fit_data_in_hatch_splines: bool,
) -> Result<Parsed, Unreadable> {
    let list = own_groups(list);
    let common = common(list, dec)?;
    let g = Groups { list, dec };
    let kind = match name {
        "LINE" => Kind::Line {
            a: g.point_or_zero(10)?,
            b: g.point_or_zero(11)?,
        },
        "POINT" => Kind::Point {
            p: g.point_or_zero(10)?,
        },
        "CIRCLE" => Kind::Circle {
            c: g.point_or_zero(10)?,
            r: g.num_or(40, 0.0)?,
        },
        "ARC" => Kind::Arc {
            c: g.point_or_zero(10)?,
            r: g.num_or(40, 0.0)?,
            a0: g.num_or(50, 0.0)?,
            a1: g.num_or(51, 360.0)?,
        },
        "ELLIPSE" => Kind::Ellipse {
            c: g.point_or_zero(10)?,
            major: g.point_or_zero(11)?,
            ratio: g.num_or(40, 1.0)?,
            t0: g.num_or(41, 0.0)?,
            t1: g.num_or(42, crate::math::TAU)?,
        },
        "LWPOLYLINE" => {
            let mut pts: Vec<[f64; 2]> = Vec::new();
            let mut bulges: Vec<f64> = Vec::new();
            let mut width = g.num_or(43, 0.0)? != 0.0;
            for p in list {
                let v = || {
                    parse_real(p.text()).ok_or_else(|| {
                        bad(&format!(
                            "sayı okunamadı (grup {}, satır {})",
                            p.code, p.line
                        ))
                    })
                };
                match p.code {
                    10 => {
                        pts.push([v()?, 0.0]);
                        bulges.push(0.0);
                    }
                    20 => {
                        if let Some(last) = pts.last_mut() {
                            last[1] = v()?;
                        }
                    }
                    42 => {
                        if let Some(last) = bulges.last_mut() {
                            *last = v()?;
                        }
                    }
                    40 | 41 => width |= v()? != 0.0,
                    _ => {}
                }
            }
            Kind::LwPolyline {
                pts,
                bulges,
                closed: g.int(70) & 1 == 1,
                elevation: g.num_or(38, 0.0)?,
                width,
            }
        }
        "POLYLINE" => {
            let mut verts = Vec::new();
            let mut width = g.num_or(40, 0.0)? != 0.0 || g.num_or(41, 0.0)? != 0.0;
            for (_, vl) in &after {
                let vg = Groups {
                    list: own_groups(vl),
                    dec,
                };
                width |= vg.num_or(40, 0.0)? != 0.0 || vg.num_or(41, 0.0)? != 0.0;
                verts.push(Vertex {
                    p: vg.point_or_zero(10)?,
                    bulge: vg.num_or(42, 0.0)?,
                    flags: vg.int(70),
                });
            }
            Kind::Polyline {
                flags: g.int(70),
                verts,
                elevation: g.point_or_zero(10)?[2],
                width,
            }
        }
        "SPLINE" => Kind::Spline {
            flags: g.int(70),
            degree: g.int(71),
            knots: reals(list, 40)?,
            weights: reals(list, 41)?,
            ctrl: points(list, 10)?,
            fit: points(list, 11)?,
        },
        "TEXT" | "ATTRIB" => Kind::Text {
            p: g.point_or_zero(10)?,
            p2: g.point(11)?,
            height: g.num_or(40, 0.0)?,
            rotation: g.num_or(50, 0.0)?,
            text: g.string(1),
            halign: g.int(72),
            valign: if name == "ATTRIB" {
                g.int(74)
            } else {
                g.int(73)
            },
            width: g.num_or(41, 1.0)?,
            style: g.string(7),
            hidden: name == "ATTRIB" && g.int(70) & 1 == 1,
        },
        "MTEXT" => {
            // The text is split over 3 groups (250-character chunks) and a final 1.
            let mut text = String::new();
            for p in list.iter().filter(|p| p.code == 3 || p.code == 1) {
                text.push_str(&dec.string(p.value));
            }
            Kind::MText {
                p: g.point_or_zero(10)?,
                height: g.num_or(40, 0.0)?,
                attach: g.int(71),
                xdir: g.point(11)?,
                rotation: g.num(50)?,
                text,
                spacing: g.num_or(44, 1.0)?,
                style: g.string(7),
            }
        }
        "SOLID" | "TRACE" | "3DFACE" => {
            let a = g.point_or_zero(10)?;
            let b = g.point_or_zero(11)?;
            let c = g.point_or_zero(12)?;
            let d = g.point(13)?.unwrap_or(c);
            Kind::Face {
                pts: [a, b, c, d],
                solid: name != "3DFACE",
            }
        }
        "INSERT" => {
            let mut attribs = Vec::new();
            let mut bad_attribs = Vec::new();
            for (l, al) in after {
                match parse("ATTRIB", l, &al, dec, Vec::new(), fit_data_in_hatch_splines) {
                    Ok(a) => attribs.push(a),
                    Err(u) => bad_attribs.push((u.reason, l)),
                }
            }
            Kind::Insert {
                name: g.string(2),
                p: g.point_or_zero(10)?,
                scale: [g.num_or(41, 1.0)?, g.num_or(42, 1.0)?, g.num_or(43, 1.0)?],
                rotation: g.num_or(50, 0.0)?,
                cols: g.int(70).max(1),
                rows: g.int(71).max(1),
                dc: g.num_or(44, 0.0)?,
                dr: g.num_or(45, 0.0)?,
                attribs,
                bad_attribs,
            }
        }
        "HATCH" => Kind::Hatch(Box::new(
            parse_hatch(list, fit_data_in_hatch_splines).map_err(|r| bad(&r))?,
        )),
        "DIMENSION" | "ARC_DIMENSION" | "LARGE_RADIAL_DIMENSION" => Kind::Block {
            block: g.string(2),
            what: "Ölçü (DIMENSION)",
        },
        "ACAD_TABLE" => Kind::Block {
            block: g.string(2),
            what: "Tablo (ACAD_TABLE)",
        },
        "XLINE" | "RAY" => Kind::Xline {
            p: g.point_or_zero(10)?,
            dir: g.point_or_zero(11)?,
            ray: name == "RAY",
        },
        "LEADER" => Kind::Leader {
            pts: points(list, 10)?,
        },
        other => Kind::Unsupported(other.to_string()),
    };
    Ok(Parsed {
        kind,
        common,
        line,
        name: name.to_string(),
    })
}
