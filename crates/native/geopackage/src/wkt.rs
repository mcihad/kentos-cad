//! ISO well-known text (OGC 06-103r4, ISO 19125-1 and the curve types of ISO 13249-3) for [`Geometry`]: what a CSV's
//! geometry column holds (GDAL's CSV driver, QGIS's delimited text; docs/adr/0522). Written with Z and M as ISO does
//! (`POINT Z (1 2 3)`, `LINESTRING M (…)`, `ZM`), numbers as the shortest text that reads back to them; read as ISO,
//! EWKT's `SRID=n;` prefix and the old `POINT(1 2 3)` with a third ordinate as Z too. Reading never panics: a text too
//! deep or not WKT is an error.

use crate::geom::{Coord, Curve, Geometry};

/// How deep a text may nest (a collection of collections …).
const DEPTH: usize = 32;

fn num(v: f64) -> String {
    if v == v.trunc() && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

fn coord(c: &Coord, z: bool, m: bool, out: &mut String) {
    out.push_str(&num(c.x));
    out.push(' ');
    out.push_str(&num(c.y));
    if z {
        out.push(' ');
        out.push_str(&num(c.z.unwrap_or(0.0)));
    }
    if m {
        out.push(' ');
        out.push_str(&c.m.map_or_else(|| "NaN".to_owned(), num));
    }
}

fn coords(cs: &[Coord], z: bool, m: bool, out: &mut String) {
    if cs.is_empty() {
        out.push_str("EMPTY");
        return;
    }
    out.push('(');
    for (k, c) in cs.iter().enumerate() {
        if k > 0 {
            out.push_str(", ");
        }
        coord(c, z, m, out);
    }
    out.push(')');
}

/// A curve inside a compound curve or a curve polygon: a line string bare, the others named.
fn curve(c: &Curve, z: bool, m: bool, named: bool, out: &mut String) {
    match c {
        Curve::Line(p) => {
            if named {
                out.push_str("LINESTRING ");
            }
            coords(p, z, m, out);
        }
        Curve::Arc(p) => {
            out.push_str("CIRCULARSTRING ");
            coords(p, z, m, out);
        }
        Curve::Compound(parts) => {
            out.push_str("COMPOUNDCURVE ");
            if parts.is_empty() {
                out.push_str("EMPTY");
                return;
            }
            out.push('(');
            for (k, p) in parts.iter().enumerate() {
                if k > 0 {
                    out.push_str(", ");
                }
                curve(p, z, m, false, out);
            }
            out.push(')');
        }
    }
}

fn rings(rs: &[Curve], z: bool, m: bool, curved: bool, out: &mut String) {
    if rs.is_empty() {
        out.push_str("EMPTY");
        return;
    }
    out.push('(');
    for (k, r) in rs.iter().enumerate() {
        if k > 0 {
            out.push_str(", ");
        }
        // In a curve polygon a linear ring stays bare; a curved one is named.
        curve(r, z, m, curved && r.is_curved(), out);
    }
    out.push(')');
}

fn dims_word(z: bool, m: bool) -> &'static str {
    match (z, m) {
        (false, false) => "",
        (true, false) => " Z",
        (false, true) => " M",
        (true, true) => " ZM",
    }
}

fn body(g: &Geometry, z: bool, m: bool, out: &mut String) {
    match g {
        Geometry::Point(None) => out.push_str("EMPTY"),
        Geometry::Point(Some(c)) => coords(std::slice::from_ref(c), z, m, out),
        Geometry::Curve(c) => match c {
            Curve::Line(p) | Curve::Arc(p) => coords(p, z, m, out),
            Curve::Compound(parts) => {
                if parts.is_empty() {
                    out.push_str("EMPTY");
                    return;
                }
                out.push('(');
                for (k, p) in parts.iter().enumerate() {
                    if k > 0 {
                        out.push_str(", ");
                    }
                    curve(p, z, m, false, out);
                }
                out.push(')');
            }
        },
        Geometry::Polygon(rs) => rings(rs, z, m, g.is_curved(), out),
        Geometry::MultiPoint(ps) => {
            if ps.is_empty() {
                out.push_str("EMPTY");
                return;
            }
            out.push('(');
            for (k, p) in ps.iter().enumerate() {
                if k > 0 {
                    out.push_str(", ");
                }
                coords(std::slice::from_ref(p), z, m, out);
            }
            out.push(')');
        }
        Geometry::MultiCurve(cs) => {
            if cs.is_empty() {
                out.push_str("EMPTY");
                return;
            }
            let curved = g.is_curved();
            out.push('(');
            for (k, c) in cs.iter().enumerate() {
                if k > 0 {
                    out.push_str(", ");
                }
                curve(c, z, m, curved && c.is_curved(), out);
            }
            out.push(')');
        }
        Geometry::MultiPolygon(ps) => {
            if ps.is_empty() {
                out.push_str("EMPTY");
                return;
            }
            let curved = g.is_curved();
            out.push('(');
            for (k, p) in ps.iter().enumerate() {
                if k > 0 {
                    out.push_str(", ");
                }
                if curved && p.iter().any(Curve::is_curved) {
                    out.push_str("CURVEPOLYGON ");
                }
                rings(p, z, m, curved, out);
            }
            out.push(')');
        }
        Geometry::Collection(gs) => {
            if gs.is_empty() {
                out.push_str("EMPTY");
                return;
            }
            out.push('(');
            for (k, x) in gs.iter().enumerate() {
                if k > 0 {
                    out.push_str(", ");
                }
                write_into(x, out);
            }
            out.push(')');
        }
    }
}

fn write_into(g: &Geometry, out: &mut String) {
    let (z, m) = g.dims();
    out.push_str(g.type_name());
    out.push_str(dims_word(z, m));
    out.push(' ');
    body(g, z, m, out);
}

/// The geometry as ISO WKT.
pub fn write_wkt(g: &Geometry) -> String {
    let mut out = String::new();
    write_into(g, &mut out);
    out
}

// ── Reading ─────────────────────────────────────────────────────────────

struct Parser<'a> {
    s: &'a [u8],
    at: usize,
}

type R<T> = Result<T, String>;

impl Parser<'_> {
    fn space(&mut self) {
        while self.at < self.s.len() && self.s[self.at].is_ascii_whitespace() {
            self.at += 1;
        }
    }

    fn word(&mut self) -> String {
        self.space();
        let start = self.at;
        while self.at < self.s.len() && self.s[self.at].is_ascii_alphabetic() {
            self.at += 1;
        }
        String::from_utf8_lossy(&self.s[start..self.at]).to_ascii_uppercase()
    }

    fn peek(&mut self) -> Option<u8> {
        self.space();
        self.s.get(self.at).copied()
    }

    fn eat(&mut self, c: u8) -> R<()> {
        if self.peek() == Some(c) {
            self.at += 1;
            Ok(())
        } else {
            Err(format!(
                "{}. karakterde “{}” bekleniyordu",
                self.at + 1,
                c as char
            ))
        }
    }

    fn number(&mut self) -> R<f64> {
        self.space();
        let start = self.at;
        while self.at < self.s.len()
            && (self.s[self.at].is_ascii_alphanumeric() || b"+-.".contains(&self.s[self.at]))
        {
            self.at += 1;
        }
        let t = String::from_utf8_lossy(&self.s[start..self.at]);
        match t.to_ascii_lowercase().as_str() {
            "nan" => Ok(f64::NAN),
            _ => t
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or_else(|| format!("{}. karakterde sayı bekleniyordu: “{t}”", start + 1)),
        }
    }

    /// `ZM`, `Z`, `M` or nothing after a type's name.
    fn dims(&mut self) -> (Option<bool>, Option<bool>) {
        self.space();
        let save = self.at;
        let w = self.word();
        match w.as_str() {
            "ZM" => (Some(true), Some(true)),
            "Z" => (Some(true), Some(false)),
            "M" => (Some(false), Some(true)),
            _ => {
                self.at = save;
                (None, None)
            }
        }
    }

    fn empty(&mut self) -> bool {
        self.space();
        let save = self.at;
        if self.word() == "EMPTY" {
            true
        } else {
            self.at = save;
            false
        }
    }

    fn coord(&mut self, z: Option<bool>, m: Option<bool>) -> R<Coord> {
        let x = self.number()?;
        let y = self.number()?;
        let mut more = Vec::new();
        while !matches!(self.peek(), Some(b',') | Some(b')') | None) {
            more.push(self.number()?);
            if more.len() > 2 {
                return Err("bir noktada dörtten çok sayı".into());
            }
        }
        let (zv, mv) = match (z, m, more.len()) {
            (Some(true), Some(true), 2) => (Some(more[0]), Some(more[1])),
            (Some(true), Some(false), 1) => (Some(more[0]), None),
            (Some(false), Some(true), 1) => (None, Some(more[0])),
            (Some(false), Some(false), 0) => (None, None),
            // Without a word: the third ordinate is Z, the fourth M.
            (None, None, 0) => (None, None),
            (None, None, 1) => (Some(more[0]), None),
            (None, None, 2) => (Some(more[0]), Some(more[1])),
            _ => return Err("noktanın sayı sayısı türünün boyutlarıyla uyuşmuyor".into()),
        };
        Ok(Coord {
            x,
            y,
            z: zv,
            m: mv.filter(|v| !v.is_nan()),
        })
    }

    fn coords(&mut self, z: Option<bool>, m: Option<bool>) -> R<Vec<Coord>> {
        if self.empty() {
            return Ok(Vec::new());
        }
        self.eat(b'(')?;
        let mut out = Vec::new();
        loop {
            // A multipoint's points may be in parentheses each.
            if self.peek() == Some(b'(') {
                self.at += 1;
                out.push(self.coord(z, m)?);
                self.eat(b')')?;
            } else {
                out.push(self.coord(z, m)?);
            }
            if self.peek() == Some(b',') {
                self.at += 1;
                continue;
            }
            self.eat(b')')?;
            return Ok(out);
        }
    }

    /// A curve in a compound curve, a curve polygon or a multicurve: bare is a line string.
    fn curve(&mut self, z: Option<bool>, m: Option<bool>, depth: usize) -> R<Curve> {
        if depth > DEPTH {
            return Err("WKT çok derin".into());
        }
        if self.peek() == Some(b'(') {
            return Ok(Curve::Line(self.coords(z, m)?));
        }
        let w = self.word();
        let (z2, m2) = self.dims();
        let (z, m) = (z2.or(z), m2.or(m));
        match w.as_str() {
            "LINESTRING" => Ok(Curve::Line(self.coords(z, m)?)),
            "CIRCULARSTRING" => Ok(Curve::Arc(self.coords(z, m)?)),
            "COMPOUNDCURVE" => {
                if self.empty() {
                    return Ok(Curve::Compound(Vec::new()));
                }
                self.eat(b'(')?;
                let mut parts = Vec::new();
                loop {
                    parts.push(self.curve(z, m, depth + 1)?);
                    if self.peek() == Some(b',') {
                        self.at += 1;
                        continue;
                    }
                    self.eat(b')')?;
                    return Ok(Curve::Compound(parts));
                }
            }
            other => Err(format!("“{other}” bir eğri değil")),
        }
    }

    fn curves(&mut self, z: Option<bool>, m: Option<bool>, depth: usize) -> R<Vec<Curve>> {
        if self.empty() {
            return Ok(Vec::new());
        }
        self.eat(b'(')?;
        let mut out = Vec::new();
        loop {
            out.push(self.curve(z, m, depth + 1)?);
            if self.peek() == Some(b',') {
                self.at += 1;
                continue;
            }
            self.eat(b')')?;
            return Ok(out);
        }
    }

    fn geometry(&mut self, depth: usize) -> R<Geometry> {
        if depth > DEPTH {
            return Err("WKT çok derin".into());
        }
        let w = self.word();
        let (z, m) = self.dims();
        Ok(match w.as_str() {
            "POINT" => {
                let mut p = self.coords(z, m)?;
                if p.len() > 1 {
                    return Err("POINT'te birden çok nokta".into());
                }
                Geometry::Point(p.pop())
            }
            "LINESTRING" => Geometry::Curve(Curve::Line(self.coords(z, m)?)),
            "CIRCULARSTRING" => Geometry::Curve(Curve::Arc(self.coords(z, m)?)),
            "COMPOUNDCURVE" => Geometry::Curve(Curve::Compound(self.curves(z, m, depth)?)),
            "POLYGON" | "CURVEPOLYGON" | "TRIANGLE" => Geometry::Polygon(self.curves(z, m, depth)?),
            "MULTIPOINT" => Geometry::MultiPoint(self.coords(z, m)?),
            "MULTILINESTRING" | "MULTICURVE" => Geometry::MultiCurve(self.curves(z, m, depth)?),
            "MULTIPOLYGON" | "MULTISURFACE" | "POLYHEDRALSURFACE" | "TIN" => {
                if self.empty() {
                    Geometry::MultiPolygon(Vec::new())
                } else {
                    self.eat(b'(')?;
                    let mut out = Vec::new();
                    loop {
                        // A polygon bare (rings in parentheses), or a named curve polygon.
                        if self.peek() == Some(b'(') {
                            out.push(self.curves(z, m, depth + 1)?);
                        } else {
                            let w = self.word();
                            let (z2, m2) = self.dims();
                            if !matches!(w.as_str(), "POLYGON" | "CURVEPOLYGON" | "TRIANGLE") {
                                return Err(format!("“{w}” bir yüzey değil"));
                            }
                            out.push(self.curves(z2.or(z), m2.or(m), depth + 1)?);
                        }
                        if self.peek() == Some(b',') {
                            self.at += 1;
                            continue;
                        }
                        self.eat(b')')?;
                        break;
                    }
                    Geometry::MultiPolygon(out)
                }
            }
            "GEOMETRYCOLLECTION" => {
                if self.empty() {
                    Geometry::Collection(Vec::new())
                } else {
                    self.eat(b'(')?;
                    let mut out = Vec::new();
                    loop {
                        out.push(self.geometry(depth + 1)?);
                        if self.peek() == Some(b',') {
                            self.at += 1;
                            continue;
                        }
                        self.eat(b')')?;
                        break;
                    }
                    Geometry::Collection(out)
                }
            }
            "" => return Err("WKT boş".into()),
            other => return Err(format!("“{other}” bir WKT geometri türü değil")),
        })
    }
}

/// The geometry of an ISO WKT (or EWKT) text, or why it is none.
pub fn read_wkt(text: &str) -> Result<Geometry, String> {
    let t = text.trim();
    // EWKT's `SRID=n;` prefix: the system is the caller's to know.
    let t = match t.split_once(';') {
        Some((head, rest)) if head.trim().to_ascii_uppercase().starts_with("SRID=") => rest,
        _ => t,
    };
    let mut p = Parser {
        s: t.as_bytes(),
        at: 0,
    };
    let g = p.geometry(0)?;
    p.space();
    if p.at != p.s.len() {
        return Err(format!("{}. karakterden sonrası fazla", p.at + 1));
    }
    Ok(g)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texts_go_round() {
        for t in [
            "POINT (487500 4420000)",
            "POINT Z (487500.5 4420000.25 105.2)",
            "POINT EMPTY",
            "LINESTRING M (0 0 0, 10 0 10.5)",
            "LINESTRING ZM (0 0 1 0, 10 0 2 10)",
            "CIRCULARSTRING (0 0, 1 1, 2 0)",
            "COMPOUNDCURVE ((0 0, 1 0), CIRCULARSTRING (1 0, 2 1, 3 0))",
            "POLYGON ((0 0, 10 0, 10 10, 0 0), (1 1, 2 1, 2 2, 1 1))",
            "CURVEPOLYGON (CIRCULARSTRING (0 0, 2 0, 0 0), (0.5 0.5, 1 0.5, 1 1, 0.5 0.5))",
            "MULTIPOINT ((0 0), (1 1))",
            "MULTILINESTRING ((0 0, 1 1), (2 2, 3 3))",
            "MULTICURVE ((0 0, 1 1), CIRCULARSTRING (2 2, 3 3, 4 2))",
            "MULTIPOLYGON (((0 0, 1 0, 1 1, 0 0)), ((5 5, 6 5, 6 6, 5 5)))",
            "MULTISURFACE (CURVEPOLYGON (CIRCULARSTRING (0 0, 2 0, 0 0)), ((5 5, 6 5, 6 6, 5 5)))",
            "GEOMETRYCOLLECTION (POINT (1 2), LINESTRING (0 0, 1 1))",
        ] {
            let g = read_wkt(t).unwrap_or_else(|e| panic!("{t}: {e}"));
            assert_eq!(write_wkt(&g), t, "{t}");
        }
    }

    #[test]
    fn other_forms_read_too() {
        assert_eq!(
            write_wkt(&read_wkt("SRID=5256;POINT(1 2 3)").expect("ewkt")),
            "POINT Z (1 2 3)"
        );
        assert_eq!(
            write_wkt(&read_wkt("multipoint (0 0, 1 1)").expect("bare points")),
            "MULTIPOINT ((0 0), (1 1))"
        );
        assert!(read_wkt("POINT (1)").is_err());
        assert!(read_wkt("BOX(0 0, 1 1)").is_err());
        assert!(read_wkt("POINT (1 2) x").is_err());
        assert!(read_wkt(&"GEOMETRYCOLLECTION (".repeat(100)).is_err());
        assert!(read_wkt("POINT (inf 2)").is_err());
    }
}
