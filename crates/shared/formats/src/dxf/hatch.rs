//! HATCH groups: boundary paths (polyline paths with bulges, or edge paths
//! of lines, arcs, elliptic arcs and splines) and the pattern. The same
//! group codes mean different things in different places, so the groups
//! are walked in order with a cursor, not looked up by code.

use super::lexer::Pair;
use crate::num::{parse_int, parse_real};

#[derive(Clone, Debug)]
pub enum Edge {
    Line {
        a: [f64; 2],
        b: [f64; 2],
    },
    /// Angles in degrees as stored; `ccw` false: the arc runs clockwise and DXF stores the angles negated.
    Arc {
        c: [f64; 2],
        r: f64,
        a0: f64,
        a1: f64,
        ccw: bool,
    },
    /// Major axis end relative to the centre; parameters in degrees.
    Ellipse {
        c: [f64; 2],
        major: [f64; 2],
        ratio: f64,
        a0: f64,
        a1: f64,
        ccw: bool,
    },
    Spline {
        degree: usize,
        knots: Vec<f64>,
        ctrl: Vec<[f64; 2]>,
        weights: Vec<f64>,
    },
}

#[derive(Clone, Debug)]
pub enum Path {
    Poly {
        pts: Vec<[f64; 2]>,
        bulges: Vec<f64>,
    },
    Edges(Vec<Edge>),
}

#[derive(Clone, Debug)]
pub struct PatternLine {
    pub angle: f64,
    pub offset: [f64; 2],
    pub dashes: usize,
}

#[derive(Clone, Debug)]
pub struct Hatch {
    pub elevation: f64,
    pub name: String,
    pub solid: bool,
    pub gradient: bool,
    /// 0 odd parity, 1 outermost only, 2 ignore islands.
    pub style: i64,
    pub angle: f64,
    pub scale: f64,
    pub double: bool,
    pub lines: Vec<PatternLine>,
    pub paths: Vec<Path>,
}

struct Cursor<'g, 'a> {
    list: &'g [Pair<'a>],
    at: usize,
}

impl<'g, 'a> Cursor<'g, 'a> {
    fn peek(&self) -> Option<i32> {
        self.list.get(self.at).map(|p| p.code)
    }

    /// The next group, which must have this code.
    fn take(&mut self, code: i32) -> Result<&'g Pair<'a>, String> {
        match self.list.get(self.at) {
            Some(p) if p.code == code => {
                self.at += 1;
                Ok(p)
            }
            Some(p) => Err(format!(
                "tarama sınırı beklenmedik biçimde sürüyor (grup {code} yerine {}, satır {})",
                p.code, p.line
            )),
            None => Err(format!("tarama sınırı eksik (grup {code} bekleniyordu)")),
        }
    }

    fn real(&mut self, code: i32) -> Result<f64, String> {
        let p = self.take(code)?;
        parse_real(p.text())
            .ok_or_else(|| format!("sayı okunamadı (grup {code}, satır {})", p.line))
    }

    fn int(&mut self, code: i32) -> Result<i64, String> {
        let p = self.take(code)?;
        parse_int(p.text())
            .ok_or_else(|| format!("tam sayı okunamadı (grup {code}, satır {})", p.line))
    }

    fn xy(&mut self, x: i32) -> Result<[f64; 2], String> {
        Ok([self.real(x)?, self.real(x + 10)?])
    }

    fn optional_real(&mut self, code: i32) -> Result<Option<f64>, String> {
        if self.peek() == Some(code) {
            self.real(code).map(Some)
        } else {
            Ok(None)
        }
    }

    /// A count read from the file, bounded by what the groups can hold.
    fn count(&mut self, code: i32) -> Result<usize, String> {
        let n = self.int(code)?;
        usize::try_from(n)
            .ok()
            .filter(|&n| n <= self.list.len())
            .ok_or_else(|| format!("sayı geçersiz (grup {code}: {n})"))
    }
}

fn edge(c: &mut Cursor, fit_data: bool) -> Result<Edge, String> {
    match c.int(72)? {
        1 => Ok(Edge::Line {
            a: c.xy(10)?,
            b: c.xy(11)?,
        }),
        2 => {
            let centre = c.xy(10)?;
            let r = c.real(40)?;
            let a0 = c.real(50)?;
            let a1 = c.real(51)?;
            Ok(Edge::Arc {
                c: centre,
                r,
                a0,
                a1,
                ccw: c.int(73)? != 0,
            })
        }
        3 => {
            let centre = c.xy(10)?;
            let major = c.xy(11)?;
            let ratio = c.real(40)?;
            let a0 = c.real(50)?;
            let a1 = c.real(51)?;
            Ok(Edge::Ellipse {
                c: centre,
                major,
                ratio,
                a0,
                a1,
                ccw: c.int(73)? != 0,
            })
        }
        4 => {
            let degree = usize::try_from(c.int(94)?).unwrap_or(0);
            let rational = c.int(73)? != 0;
            let _periodic = c.int(74)?;
            let nk = c.count(95)?;
            let nc = c.count(96)?;
            let mut knots = Vec::with_capacity(nk);
            for _ in 0..nk {
                knots.push(c.real(40)?);
            }
            let mut ctrl = Vec::with_capacity(nc);
            let mut weights = Vec::new();
            for _ in 0..nc {
                ctrl.push(c.xy(10)?);
                if let Some(w) = c.optional_real(42)? {
                    weights.push(w);
                }
            }
            if !rational {
                weights.clear();
            }
            // DXF 2010 and later add fit data to spline edges (a count, points, end tangents).
            if fit_data && c.peek() == Some(97) {
                let nf = c.count(97)?;
                for _ in 0..nf {
                    c.xy(11)?;
                }
                if c.peek() == Some(12) {
                    c.xy(12)?;
                }
                if c.peek() == Some(13) {
                    c.xy(13)?;
                }
            }
            Ok(Edge::Spline {
                degree,
                knots,
                ctrl,
                weights,
            })
        }
        t => Err(format!("bilinmeyen tarama sınırı kenarı ({t})")),
    }
}

fn path(c: &mut Cursor, fit_data: bool) -> Result<Path, String> {
    let flags = c.int(92)?;
    let p = if flags & 2 != 0 {
        let has_bulge = c.int(72)? != 0;
        let _closed = c.int(73)?;
        let n = c.count(93)?;
        let mut pts = Vec::with_capacity(n);
        let mut bulges = Vec::with_capacity(n);
        for _ in 0..n {
            pts.push(c.xy(10)?);
            let b = if has_bulge {
                c.optional_real(42)?
            } else {
                None
            };
            bulges.push(b.unwrap_or(0.0));
        }
        Path::Poly { pts, bulges }
    } else {
        let n = c.count(93)?;
        let mut edges = Vec::with_capacity(n);
        for _ in 0..n {
            edges.push(edge(c, fit_data)?);
        }
        Path::Edges(edges)
    };
    // Source objects of an associative hatch: their count and handles.
    if c.peek() == Some(97) {
        let n = c.count(97)?;
        for _ in 0..n {
            if c.peek() == Some(330) {
                c.take(330)?;
            }
        }
    }
    Ok(p)
}

/// A HATCH's groups (common groups included; they are skipped).
pub fn parse_hatch(list: &[Pair<'_>], fit_data: bool) -> Result<Hatch, String> {
    let mut h = Hatch {
        elevation: 0.0,
        name: String::new(),
        solid: false,
        gradient: false,
        style: 0,
        angle: 0.0,
        scale: 1.0,
        double: false,
        lines: Vec::new(),
        paths: Vec::new(),
    };
    let mut c = Cursor { list, at: 0 };
    let mut in_hatch = false;
    while let Some(code) = c.peek() {
        let p = c.list[c.at];
        // Codes before the AcDbHatch marker are the common ones.
        if !in_hatch {
            if p.code == 100 && p.text() == "AcDbHatch" {
                in_hatch = true;
            }
            c.at += 1;
            continue;
        }
        match code {
            30 => h.elevation = c.real(30)?,
            2 => h.name = c.take(2)?.text().to_string(),
            70 => h.solid = c.int(70)? == 1,
            91 => {
                let n = c.count(91)?;
                for _ in 0..n {
                    h.paths.push(path(&mut c, fit_data)?);
                }
            }
            75 => h.style = c.int(75)?,
            52 => h.angle = c.real(52)?,
            41 => h.scale = c.real(41)?,
            77 => h.double = c.int(77)? != 0,
            78 => {
                let n = c.count(78)?;
                for _ in 0..n {
                    let angle = c.real(53)?;
                    let _base = [c.real(43)?, c.real(44)?];
                    let offset = [c.real(45)?, c.real(46)?];
                    let dashes = c.count(79)?;
                    for _ in 0..dashes {
                        c.real(49)?;
                    }
                    h.lines.push(PatternLine {
                        angle,
                        offset,
                        dashes,
                    });
                }
            }
            450 => h.gradient = c.int(450)? != 0,
            98 => {
                let n = c.count(98)?;
                for _ in 0..n {
                    c.xy(10)?;
                }
            }
            _ => c.at += 1,
        }
    }
    // Writers without the subclass markers: read the groups again from the start.
    if !in_hatch && h.paths.is_empty() && list.iter().any(|p| p.code == 91) {
        let start = list
            .iter()
            .position(|p| p.code == 100 || p.code == 10)
            .unwrap_or(0);
        let marked: Vec<Pair<'_>> = std::iter::once(Pair {
            code: 100,
            value: b"AcDbHatch",
            line: 0,
        })
        .chain(list[start..].iter().copied())
        .collect();
        return parse_hatch(&marked, fit_data);
    }
    Ok(h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dxf::lexer::Lexer;

    fn groups(text: &str) -> Vec<Pair<'_>> {
        let mut l = Lexer::new(text.as_bytes());
        let mut out = Vec::new();
        while let Some(p) = l.next().expect("pairs") {
            out.push(p);
        }
        out
    }

    #[test]
    fn a_polyline_path_and_an_edge_path_with_an_arc() {
        let text = "100\nAcDbHatch\n10\n0\n20\n0\n30\n0\n2\nANSI31\n70\n0\n71\n0\n91\n2\n92\n3\n72\n1\n73\n1\n93\n2\n10\n0\n20\n0\n42\n1\n10\n2\n20\n0\n42\n1\n97\n0\n92\n0\n93\n2\n72\n1\n10\n0\n20\n0\n11\n1\n21\n0\n72\n2\n10\n0.5\n20\n0\n40\n0.5\n50\n0\n51\n180\n73\n1\n97\n0\n75\n1\n76\n1\n52\n0\n41\n1\n77\n0\n78\n1\n53\n45\n43\n0\n44\n0\n45\n-2.245\n46\n2.245\n79\n0\n98\n0\n";
        let h = parse_hatch(&groups(text), false).expect("hatch");
        assert_eq!(h.name, "ANSI31");
        assert_eq!(h.paths.len(), 2);
        let Path::Poly { pts, bulges } = &h.paths[0] else {
            panic!()
        };
        assert_eq!(pts, &vec![[0.0, 0.0], [2.0, 0.0]]);
        assert_eq!(bulges, &vec![1.0, 1.0]);
        let Path::Edges(edges) = &h.paths[1] else {
            panic!()
        };
        assert!(matches!(edges[1], Edge::Arc { r, ccw: true, .. } if r == 0.5));
        assert_eq!((h.style, h.lines.len(), h.lines[0].angle), (1, 1, 45.0));
    }

    #[test]
    fn a_broken_path_says_where() {
        let text = "100\nAcDbHatch\n91\n1\n92\n2\n72\n0\n73\n1\n93\n3\n10\n0\n20\n0\n";
        let e = parse_hatch(&groups(text), false).expect_err("error");
        assert!(e.contains("tarama sınırı eksik"), "{e}");
    }
}
