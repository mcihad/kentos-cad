//! Extended WKB, the binary geometry PostGIS reads (`ST_GeomFromEWKB`) and
//! writes (`ST_AsEWKB`). Coordinates travel as raw IEEE doubles, so a value
//! comes back with every bit (text forms such as WKT round to 15 digits).
//! Only what KentOS stores is supported: points (optionally with Z), line
//! strings and polygons, with an SRID.

use crate::Vec2;

const SRID_FLAG: u32 = 0x2000_0000;
const Z_FLAG: u32 = 0x8000_0000;

#[derive(Clone, Debug, PartialEq)]
pub enum Geometry {
    Point {
        p: Vec2,
        z: Option<f64>,
    },
    LineString(Vec<Vec2>),
    /// Rings without the closing point (written closed, read back open).
    Polygon(Vec<Vec<Vec2>>),
}

fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_f64(out: &mut Vec<u8>, v: f64) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_points(out: &mut Vec<u8>, pts: &[Vec2], close: bool) {
    let closing = close && pts.first() != pts.last();
    put_u32(out, (pts.len() + usize::from(closing)) as u32);
    for p in pts.iter().chain(closing.then(|| &pts[0])) {
        put_f64(out, p.x);
        put_f64(out, p.y);
    }
}

/// Little-endian EWKB with the SRID.
pub fn encode(geometry: &Geometry, srid: u32) -> Vec<u8> {
    let mut out = vec![1u8];
    let (kind, z) = match geometry {
        Geometry::Point { z, .. } => (1, z.is_some()),
        Geometry::LineString(_) => (2, false),
        Geometry::Polygon(_) => (3, false),
    };
    put_u32(&mut out, kind | SRID_FLAG | if z { Z_FLAG } else { 0 });
    put_u32(&mut out, srid);
    match geometry {
        Geometry::Point { p, z } => {
            put_f64(&mut out, p.x);
            put_f64(&mut out, p.y);
            if let Some(z) = z {
                put_f64(&mut out, *z);
            }
        }
        Geometry::LineString(pts) => put_points(&mut out, pts, false),
        Geometry::Polygon(rings) => {
            put_u32(&mut out, rings.len() as u32);
            for ring in rings {
                put_points(&mut out, ring, true);
            }
        }
    }
    out
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
    little: bool,
}

impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], String> {
        let end = self.at + N;
        let chunk = self
            .bytes
            .get(self.at..end)
            .ok_or("EWKB beklenenden kısa")?;
        self.at = end;
        chunk
            .try_into()
            .map_err(|_| "EWKB beklenenden kısa".to_string())
    }
    fn u32(&mut self) -> Result<u32, String> {
        let b = self.take::<4>()?;
        Ok(if self.little {
            u32::from_le_bytes(b)
        } else {
            u32::from_be_bytes(b)
        })
    }
    fn f64(&mut self) -> Result<f64, String> {
        let b = self.take::<8>()?;
        Ok(if self.little {
            f64::from_le_bytes(b)
        } else {
            f64::from_be_bytes(b)
        })
    }
    fn point(&mut self, z: bool) -> Result<(Vec2, Option<f64>), String> {
        let p = Vec2::new(self.f64()?, self.f64()?);
        Ok((p, if z { Some(self.f64()?) } else { None }))
    }
    fn points(&mut self) -> Result<Vec<Vec2>, String> {
        let n = self.u32()? as usize;
        if n > (self.bytes.len() - self.at) / 16 {
            return Err("EWKB nokta sayısı veriden büyük".into());
        }
        (0..n)
            .map(|_| Ok(Vec2::new(self.f64()?, self.f64()?)))
            .collect()
    }
}

/// Reads EWKB (either byte order); returns the geometry and its SRID (0 when absent).
pub fn decode(bytes: &[u8]) -> Result<(Geometry, u32), String> {
    let mut r = Reader {
        bytes,
        at: 1,
        little: *bytes.first().ok_or("EWKB boş")? == 1,
    };
    let head = r.u32()?;
    let srid = if head & SRID_FLAG != 0 { r.u32()? } else { 0 };
    let z = head & Z_FLAG != 0;
    let geometry = match head & 0x0fff_ffff {
        1 => {
            let (p, z) = r.point(z)?;
            Geometry::Point { p, z }
        }
        2 if !z => Geometry::LineString(r.points()?),
        3 if !z => {
            let n = r.u32()? as usize;
            let mut rings = Vec::with_capacity(n.min(1024));
            for _ in 0..n {
                let mut ring = r.points()?;
                if ring.len() < 2 || ring.first() != ring.last() {
                    return Err("EWKB halkası kapalı değil".into());
                }
                ring.pop();
                rings.push(ring);
            }
            Geometry::Polygon(rings)
        }
        other => {
            return Err(format!(
                "EWKB türü desteklenmiyor: {other}{}",
                if z { " (Z)" } else { "" }
            ));
        }
    };
    if r.at != bytes.len() {
        return Err("EWKB sonunda fazladan bayt var".into());
    }
    Ok((geometry, srid))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_postgis_bytes_for_a_point() {
        // SELECT ST_AsEWKB('SRID=4326;POINT(1 2)'::geometry)
        let hex: String = encode(
            &Geometry::Point {
                p: Vec2::new(1.0, 2.0),
                z: None,
            },
            4326,
        )
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect();
        assert_eq!(hex, "0101000020E6100000000000000000F03F0000000000000040");
    }

    #[test]
    fn round_trips_every_bit() {
        let tm = |x: f64, y: f64| Vec2::new(486_512.34 + 1e-9 / 3.0 + x, 4_420_210.123_456_789 + y);
        for g in [
            Geometry::Point {
                p: tm(0.0, 0.0),
                z: Some(-0.0),
            },
            Geometry::Point {
                p: Vec2::new(-0.0, 1e-300),
                z: None,
            },
            Geometry::LineString(vec![tm(0.0, 0.0), tm(0.1, 0.2), tm(1.0 / 3.0, 0.0)]),
            Geometry::Polygon(vec![
                vec![tm(0.0, 0.0), tm(10.0, 0.0), tm(10.0, 10.0)],
                vec![tm(1.0, 1.0), tm(2.0, 1.0), tm(2.0, 2.0)],
            ]),
        ] {
            let (back, srid) = decode(&encode(&g, 5256)).unwrap();
            assert_eq!(srid, 5256);
            let bits = |g: &Geometry| format!("{:?}", encode(g, 0));
            assert_eq!(bits(&back), bits(&g));
        }
    }

    #[test]
    fn refuses_truncated_or_unknown_input() {
        let bytes = encode(
            &Geometry::LineString(vec![Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0)]),
            5256,
        );
        assert!(decode(&bytes[..bytes.len() - 1]).is_err());
        assert!(decode(&[]).is_err());
        let mut multipoint = bytes.clone();
        multipoint[1] = 4;
        assert!(decode(&multipoint).unwrap_err().contains("desteklenmiyor"));
        let mut long = bytes;
        long.push(0);
        assert!(decode(&long).is_err());
    }
}
