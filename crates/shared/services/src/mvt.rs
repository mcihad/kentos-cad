//! Mapbox vector tiles, specification 2.1 (docs/adr/0208 §9): a tile's
//! layers, each its name, extent, keys and values, and its features with
//! their ids, tags and geometry (points, lines, polygons of rings: an
//! exterior ring has a positive area in tile coordinates, Y down, and starts
//! a polygon; a negative one is a hole of the polygon before). A gzipped or
//! zlib tile is inflated first. A protobuf that does not hold is said in
//! words, never a panic.

/// The most bytes an inflated tile may have.
pub const MAX_TILE: usize = 64 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Text(String),
    Number(f64),
    Bool(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeomType {
    Unknown,
    Point,
    Line,
    Polygon,
}

/// A point in tile coordinates (0..extent, Y down).
pub type P = [i32; 2];

#[derive(Clone, Debug, PartialEq)]
pub enum Geometry {
    Points(Vec<P>),
    Lines(Vec<Vec<P>>),
    /// Each polygon its exterior ring first, then its holes.
    Polygons(Vec<Vec<Vec<P>>>),
    None,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Feature {
    pub id: Option<u64>,
    pub kind: GeomType,
    /// Pairs of indices into the layer's keys and values.
    pub tags: Vec<(u32, u32)>,
    pub geometry: Geometry,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub name: String,
    pub extent: u32,
    pub keys: Vec<String>,
    pub values: Vec<Value>,
    pub features: Vec<Feature>,
}

impl Layer {
    /// A feature's property by its key.
    pub fn get<'a>(&'a self, f: &Feature, key: &str) -> Option<&'a Value> {
        f.tags.iter().find_map(|&(k, v)| {
            (self.keys.get(k as usize).map(String::as_str) == Some(key))
                .then(|| self.values.get(v as usize))
                .flatten()
        })
    }
}

/// A protobuf reader over bytes.
struct Pb<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Pb<'a> {
    fn new(b: &'a [u8]) -> Pb<'a> {
        Pb { b, at: 0 }
    }

    fn done(&self) -> bool {
        self.at >= self.b.len()
    }

    fn varint(&mut self) -> Result<u64, String> {
        let mut out = 0u64;
        for shift in (0..64).step_by(7) {
            let byte = *self
                .b
                .get(self.at)
                .ok_or("karo kısa kesilmiş (sayı yarım)")?;
            self.at += 1;
            out |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Ok(out);
            }
        }
        Err("karoda 10 bayttan uzun bir sayı var".into())
    }

    fn bytes(&mut self) -> Result<&'a [u8], String> {
        let n = usize::try_from(self.varint()?).map_err(|_| "karoda çok uzun bir alan var")?;
        let end = self
            .at
            .checked_add(n)
            .filter(|&e| e <= self.b.len())
            .ok_or("karo kısa kesilmiş (alan yarım)")?;
        let out = &self.b[self.at..end];
        self.at = end;
        Ok(out)
    }

    fn fixed(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self
            .at
            .checked_add(n)
            .filter(|&e| e <= self.b.len())
            .ok_or("karo kısa kesilmiş")?;
        let out = &self.b[self.at..end];
        self.at = end;
        Ok(out)
    }

    /// Skips a field of wire type `wire`.
    fn skip(&mut self, wire: u64) -> Result<(), String> {
        match wire {
            0 => {
                self.varint()?;
            }
            1 => {
                self.fixed(8)?;
            }
            2 => {
                self.bytes()?;
            }
            5 => {
                self.fixed(4)?;
            }
            _ => return Err(format!("karoda bilinmeyen alan türü ({wire})")),
        }
        Ok(())
    }

    /// The next key: field number and wire type.
    fn key(&mut self) -> Result<(u64, u64), String> {
        let k = self.varint()?;
        Ok((k >> 3, k & 7))
    }
}

fn packed(b: &[u8]) -> Result<Vec<u32>, String> {
    let mut p = Pb::new(b);
    let mut out = Vec::with_capacity(b.len());
    while !p.done() {
        out.push(u32::try_from(p.varint()?).map_err(|_| "karoda 32 biti aşan bir komut var")?);
    }
    Ok(out)
}

fn zigzag(n: u32) -> i32 {
    ((n >> 1) as i32) ^ -((n & 1) as i32)
}

/// A gzipped or zlib tile's bytes inflated; others as they are.
pub fn inflate(bytes: &[u8]) -> Result<std::borrow::Cow<'_, [u8]>, String> {
    use miniz_oxide::inflate::{decompress_to_vec_with_limit, decompress_to_vec_zlib_with_limit};
    if bytes.len() >= 18 && bytes[0] == 0x1f && bytes[1] == 0x8b {
        // RFC 1952: a 10-byte header, then optional fields by its flags.
        let flags = bytes[3];
        let mut at = 10usize;
        if flags & 4 != 0 {
            let n = usize::from(bytes.get(at).copied().unwrap_or(0))
                | (usize::from(bytes.get(at + 1).copied().unwrap_or(0)) << 8);
            at += 2 + n;
        }
        for bit in [8u8, 16] {
            if flags & bit != 0 {
                while at < bytes.len() && bytes[at] != 0 {
                    at += 1;
                }
                at += 1;
            }
        }
        if flags & 2 != 0 {
            at += 2;
        }
        let body = bytes
            .get(at..bytes.len().saturating_sub(8))
            .ok_or("gzip'li karo kısa")?;
        return decompress_to_vec_with_limit(body, MAX_TILE)
            .map(std::borrow::Cow::Owned)
            .map_err(|e| format!("gzip'li karo açılamadı ({e:?})"));
    }
    if bytes.len() >= 2 && bytes[0] == 0x78 && matches!(bytes[1], 0x01 | 0x5e | 0x9c | 0xda) {
        return decompress_to_vec_zlib_with_limit(bytes, MAX_TILE)
            .map(std::borrow::Cow::Owned)
            .map_err(|e| format!("zlib'li karo açılamadı ({e:?})"));
    }
    Ok(std::borrow::Cow::Borrowed(bytes))
}

/// Reads a tile (inflating it first when it is compressed).
pub fn read(bytes: &[u8]) -> Result<Vec<Layer>, String> {
    let data = inflate(bytes)?;
    let mut p = Pb::new(&data);
    let mut layers = Vec::new();
    while !p.done() {
        let (field, wire) = p.key()?;
        if field == 3 && wire == 2 {
            layers.push(layer(p.bytes()?)?);
        } else {
            p.skip(wire)?;
        }
    }
    Ok(layers)
}

fn layer(b: &[u8]) -> Result<Layer, String> {
    let mut p = Pb::new(b);
    let mut l = Layer {
        name: String::new(),
        extent: 4096,
        keys: Vec::new(),
        values: Vec::new(),
        features: Vec::new(),
    };
    let mut raw_features = Vec::new();
    while !p.done() {
        let (field, wire) = p.key()?;
        match (field, wire) {
            (1, 2) => l.name = String::from_utf8_lossy(p.bytes()?).into_owned(),
            (2, 2) => raw_features.push(p.bytes()?),
            (3, 2) => l
                .keys
                .push(String::from_utf8_lossy(p.bytes()?).into_owned()),
            (4, 2) => l.values.push(value(p.bytes()?)?),
            (5, 0) => l.extent = u32::try_from(p.varint()?).unwrap_or(4096).max(1),
            (_, w) => p.skip(w)?,
        }
    }
    l.features = raw_features
        .into_iter()
        .map(feature)
        .collect::<Result<Vec<_>, String>>()?;
    Ok(l)
}

fn value(b: &[u8]) -> Result<Value, String> {
    let mut p = Pb::new(b);
    let mut v = Value::Text(String::new());
    while !p.done() {
        let (field, wire) = p.key()?;
        v = match (field, wire) {
            (1, 2) => Value::Text(String::from_utf8_lossy(p.bytes()?).into_owned()),
            (2, 5) => {
                let f = p.fixed(4)?;
                Value::Number(f64::from(f32::from_le_bytes([f[0], f[1], f[2], f[3]])))
            }
            (3, 1) => {
                let f = p.fixed(8)?;
                let mut a = [0u8; 8];
                a.copy_from_slice(f);
                Value::Number(f64::from_le_bytes(a))
            }
            (4, 0) => Value::Number(p.varint()? as i64 as f64),
            (5, 0) => Value::Number(p.varint()? as f64),
            (6, 0) => {
                let n = p.varint()?;
                Value::Number(((n >> 1) as i64 ^ -((n & 1) as i64)) as f64)
            }
            (7, 0) => Value::Bool(p.varint()? != 0),
            (_, w) => {
                p.skip(w)?;
                continue;
            }
        };
    }
    Ok(v)
}

fn feature(b: &[u8]) -> Result<Feature, String> {
    let mut p = Pb::new(b);
    let (mut id, mut kind, mut tags, mut geometry) =
        (None, GeomType::Unknown, Vec::new(), Vec::new());
    while !p.done() {
        let (field, wire) = p.key()?;
        match (field, wire) {
            (1, 0) => id = Some(p.varint()?),
            (2, 2) => {
                let t = packed(p.bytes()?)?;
                tags = t.chunks_exact(2).map(|c| (c[0], c[1])).collect();
            }
            (3, 0) => {
                kind = match p.varint()? {
                    1 => GeomType::Point,
                    2 => GeomType::Line,
                    3 => GeomType::Polygon,
                    _ => GeomType::Unknown,
                }
            }
            (4, 2) => geometry = packed(p.bytes()?)?,
            (_, w) => p.skip(w)?,
        }
    }
    Ok(Feature {
        id,
        kind,
        tags,
        geometry: decode_geometry(kind, &geometry),
    })
}

/// The commands' paths: each MoveTo starts one, LineTo continues it,
/// ClosePath ends a ring (its first point not repeated).
fn paths(cmds: &[u32]) -> Vec<Vec<P>> {
    let mut out: Vec<Vec<P>> = Vec::new();
    let (mut x, mut y) = (0i32, 0i32);
    let mut i = 0;
    while i < cmds.len() {
        let c = cmds[i];
        i += 1;
        let (id, count) = (c & 7, (c >> 3) as usize);
        match id {
            1 | 2 => {
                for k in 0..count {
                    let (Some(&dx), Some(&dy)) = (cmds.get(i), cmds.get(i + 1)) else {
                        i = cmds.len();
                        break;
                    };
                    i += 2;
                    x = x.wrapping_add(zigzag(dx));
                    y = y.wrapping_add(zigzag(dy));
                    if (id == 1 && k == 0) || out.is_empty() {
                        out.push(Vec::new());
                    }
                    if let Some(path) = out.last_mut() {
                        path.push([x, y]);
                    }
                }
            }
            7 => {}
            _ => break,
        }
    }
    out
}

fn decode_geometry(kind: GeomType, cmds: &[u32]) -> Geometry {
    match kind {
        GeomType::Point => Geometry::Points(paths(cmds).into_iter().flatten().collect()),
        GeomType::Line => {
            Geometry::Lines(paths(cmds).into_iter().filter(|p| p.len() >= 2).collect())
        }
        GeomType::Polygon => {
            let mut polys: Vec<Vec<Vec<P>>> = Vec::new();
            for ring in paths(cmds).into_iter().filter(|r| r.len() >= 3) {
                let mut area = 0i64;
                for k in 0..ring.len() {
                    let a = ring[k];
                    let b = ring[(k + 1) % ring.len()];
                    area += i64::from(a[0]) * i64::from(b[1]) - i64::from(b[0]) * i64::from(a[1]);
                }
                if area > 0 || polys.is_empty() {
                    polys.push(vec![ring]);
                } else if area < 0
                    && let Some(last) = polys.last_mut()
                {
                    last.push(ring);
                }
            }
            Geometry::Polygons(polys)
        }
        GeomType::Unknown => Geometry::None,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A protobuf writer for the tests' tiles.
    #[derive(Default)]
    pub(crate) struct W(pub Vec<u8>);

    impl W {
        pub fn varint(&mut self, mut n: u64) {
            loop {
                let b = (n & 0x7f) as u8;
                n >>= 7;
                if n == 0 {
                    self.0.push(b);
                    return;
                }
                self.0.push(b | 0x80);
            }
        }
        pub fn key(&mut self, field: u64, wire: u64) {
            self.varint(field << 3 | wire);
        }
        pub fn bytes(&mut self, field: u64, b: &[u8]) {
            self.key(field, 2);
            self.varint(b.len() as u64);
            self.0.extend_from_slice(b);
        }
        pub fn packed(&mut self, field: u64, v: &[u32]) {
            let mut inner = W::default();
            for x in v {
                inner.varint(u64::from(*x));
            }
            self.bytes(field, &inner.0);
        }
    }

    pub(crate) fn zz(n: i32) -> u32 {
        ((n << 1) ^ (n >> 31)) as u32
    }

    /// A layer `name` with one feature of `kind` and `geometry`, tagged `class=value`.
    pub(crate) fn tile(name: &str, kind: u64, geometry: &[u32], class: &str) -> Vec<u8> {
        let mut f = W::default();
        f.key(1, 0);
        f.varint(7);
        f.packed(2, &[0, 0]);
        f.key(3, 0);
        f.varint(kind);
        f.packed(4, geometry);
        let mut v = W::default();
        v.bytes(1, class.as_bytes());
        let mut l = W::default();
        l.key(15, 0);
        l.varint(2);
        l.bytes(1, name.as_bytes());
        l.bytes(2, &f.0);
        l.bytes(3, b"class");
        l.bytes(4, &v.0);
        l.key(5, 0);
        l.varint(4096);
        let mut t = W::default();
        t.bytes(3, &l.0);
        t.0
    }

    #[test]
    fn the_specifications_examples() {
        // §4.3.5.1: a point at (25, 17).
        let t = read(&tile("p", 1, &[9, 50, 34], "a")).unwrap();
        assert_eq!(t[0].features[0].geometry, Geometry::Points(vec![[25, 17]]));
        assert_eq!(t[0].features[0].id, Some(7));
        assert_eq!(
            t[0].get(&t[0].features[0], "class"),
            Some(&Value::Text("a".into()))
        );
        // §4.3.5.4: two lines.
        let lines = [9, 4, 4, 18, 0, 16, 16, 0, 9, 17, 17, 10, 4, 8];
        let t = read(&tile("l", 2, &lines, "b")).unwrap();
        assert_eq!(
            t[0].features[0].geometry,
            Geometry::Lines(vec![vec![[2, 2], [2, 10], [10, 10]], vec![[1, 1], [3, 5]]])
        );
        // §4.3.5.7: a multipolygon, the second with a hole.
        let polys = [
            9, 0, 0, 26, 20, 0, 0, 20, 19, 0, 15, 9, 22, 2, 26, 18, 0, 0, 18, 17, 0, 15, 9, 4, 13,
            26, 0, 8, 8, 0, 0, 7, 15,
        ];
        let t = read(&tile("a", 3, &polys, "c")).unwrap();
        let Geometry::Polygons(p) = &t[0].features[0].geometry else {
            panic!("polygons");
        };
        assert_eq!(p.len(), 2);
        assert_eq!(p[0], vec![vec![[0, 0], [10, 0], [10, 10], [0, 10]]]);
        assert_eq!(p[1].len(), 2);
        assert_eq!(p[1][1], vec![[13, 13], [13, 17], [17, 17], [17, 13]]);
        assert_eq!(zz(-1), 1);
    }

    #[test]
    fn broken_tiles_are_said_not_panicked_on() {
        let full = tile("p", 1, &[9, 50, 34], "a");
        for cut in 0..full.len() {
            let _ = read(&full[..cut]);
        }
        assert!(read(&[0x1a, 0xff]).is_err());
        // A gzipped tile reads as the plain one.
        let gz = {
            let deflated = miniz_oxide::deflate::compress_to_vec(&full, 6);
            let mut g = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 255];
            g.extend_from_slice(&deflated);
            g.extend_from_slice(&[0; 8]);
            g
        };
        assert_eq!(read(&gz).unwrap(), read(&full).unwrap());
    }
}
