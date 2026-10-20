//! FlatGeobuf (docs/adr/0544; plan/gis.md CBS-05): one layer's features in one file, as
//! <https://flatgeobuf.org> (v3.x) and GDAL's FlatGeobuf driver write and read them. The file is the magic
//! `fgb\x03fgb\x00`, the header (a size-prefixed FlatBuffer: name, envelope, geometry type, Z and M, columns, feature
//! count, index node size, system), the packed Hilbert R-tree of the features' boxes (40 bytes a node), then the
//! features (each a size-prefixed FlatBuffer: geometry and properties). The FlatBuffers here are our own: a writer that
//! lays a table out before its children (every offset points forward, as the format wants) and a reader that checks
//! each offset, length and alignment before it reads, so a broken file is refused, never read past its end.

use crate::{BlobError, Column, ColumnKind, Coord, Curve, Geometry, Value};

/// The file's first eight bytes (version 3).
pub const MAGIC: [u8; 8] = *b"fgb\x03fgb\x00";
/// The index node size this writer uses (the format's default).
pub const NODE_SIZE: u16 = 16;
/// The most bytes a header or a feature may take (a broken size is refused before anything is allocated).
const MAX_BUFFER: usize = 1 << 30;
/// The deepest a geometry's parts may nest.
const MAX_DEPTH: usize = 32;

// ── The format's codes ─────────────────────────────────────────────────

mod gt {
    pub const UNKNOWN: u8 = 0;
    pub const POINT: u8 = 1;
    pub const LINE_STRING: u8 = 2;
    pub const POLYGON: u8 = 3;
    pub const MULTI_POINT: u8 = 4;
    pub const MULTI_LINE_STRING: u8 = 5;
    pub const MULTI_POLYGON: u8 = 6;
    pub const COLLECTION: u8 = 7;
    pub const CIRCULAR_STRING: u8 = 8;
    pub const COMPOUND_CURVE: u8 = 9;
    pub const CURVE_POLYGON: u8 = 10;
    pub const MULTI_CURVE: u8 = 11;
    pub const MULTI_SURFACE: u8 = 12;
}

/// The column types of FlatGeobuf.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ColumnType {
    Byte = 0,
    UByte = 1,
    Bool = 2,
    Short = 3,
    UShort = 4,
    Int = 5,
    UInt = 6,
    Long = 7,
    ULong = 8,
    Float = 9,
    Double = 10,
    String = 11,
    Json = 12,
    DateTime = 13,
    Binary = 14,
}

impl ColumnType {
    fn of(code: u8) -> Option<ColumnType> {
        use ColumnType::*;
        Some(match code {
            0 => Byte,
            1 => UByte,
            2 => Bool,
            3 => Short,
            4 => UShort,
            5 => Int,
            6 => UInt,
            7 => Long,
            8 => ULong,
            9 => Float,
            10 => Double,
            11 => String,
            12 => Json,
            13 => DateTime,
            14 => Binary,
            _ => return None,
        })
    }

    /// The type a column of this kind is written as (a date as a date-time column holding the day alone).
    pub fn for_kind(kind: &ColumnKind) -> ColumnType {
        match kind {
            ColumnKind::Boolean => ColumnType::Bool,
            ColumnKind::Integer => ColumnType::Long,
            ColumnKind::Real => ColumnType::Double,
            ColumnKind::Blob(_) => ColumnType::Binary,
            ColumnKind::Date | ColumnKind::DateTime => ColumnType::DateTime,
            ColumnKind::Text(_) | ColumnKind::Other(_) => ColumnType::String,
        }
    }

    /// GDAL's word for it (as `ogrinfo` says a FlatGeobuf column).
    pub fn word(self) -> &'static str {
        use ColumnType::*;
        match self {
            Byte | UByte | Short | UShort | Int | UInt => "Integer",
            Bool => "Integer(Boolean)",
            Long | ULong => "Integer64",
            Float | Double => "Real",
            String | Json => "String",
            DateTime => "DateTime",
            Binary => "Binary",
        }
    }
}

/// A column as the file declares it.
#[derive(Clone, Debug, PartialEq)]
pub struct FgbColumn {
    pub name: String,
    pub kind: ColumnType,
    /// Its width (a text column's most characters); none when the file does not say.
    pub width: Option<u32>,
}

/// A layer to write: its name, system (EPSG), columns and features (each its geometry and its values, one a column).
pub struct Layer<'a> {
    pub name: &'a str,
    pub srid: Option<u32>,
    pub columns: &'a [FgbColumn],
    pub features: &'a [(Option<Geometry>, Vec<Value>)],
}

/// A feature read: its geometry (none for a record; an error when it is not read) and its values, a column each.
pub type Feature = (Option<Result<Geometry, BlobError>>, Vec<Value>);

/// A file read: the layer's name, system and columns, and its features in file order.
#[derive(Clone, Debug, PartialEq)]
pub struct Read {
    pub name: String,
    pub srid: Option<u32>,
    /// The system's organisation and code as the file names it, when it is not EPSG.
    pub other_crs: Option<String>,
    pub columns: Vec<Column>,
    pub declared: Vec<FgbColumn>,
    pub features: Vec<Feature>,
    pub indexed: bool,
    pub has_z: bool,
    pub has_m: bool,
}

// ── Writing FlatBuffers forward ────────────────────────────────────────

enum Item {
    U8(u8),
    U16(u16),
    I32(i32),
    U64(u64),
    Ref(Node),
}

impl Item {
    fn size(&self) -> usize {
        match self {
            Item::U8(_) => 1,
            Item::U16(_) => 2,
            Item::I32(_) | Item::Ref(_) => 4,
            Item::U64(_) => 8,
        }
    }
}

enum Node {
    Table(Vec<(u16, Item)>),
    Str(String),
    Bytes(Vec<u8>),
    F64s(Vec<f64>),
    U32s(Vec<u32>),
    Tables(Vec<Node>),
}

struct Out {
    b: Vec<u8>,
}

impl Out {
    fn pad_to(&mut self, align: usize, after: usize) {
        while !(self.b.len() + after).is_multiple_of(align) {
            self.b.push(0);
        }
    }

    fn patch(&mut self, at: usize, to: usize) {
        let v = (to - at) as u32;
        self.b[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }

    /// Writes a node and its children after it; where it starts (a table's start, a vector's or string's length).
    fn node(&mut self, node: Node) -> usize {
        match node {
            Node::Str(s) => {
                self.pad_to(4, 0);
                let at = self.b.len();
                self.b.extend((s.len() as u32).to_le_bytes());
                self.b.extend(s.as_bytes());
                self.b.push(0);
                at
            }
            Node::Bytes(v) => {
                self.pad_to(4, 0);
                let at = self.b.len();
                self.b.extend((v.len() as u32).to_le_bytes());
                self.b.extend(v);
                at
            }
            Node::U32s(v) => {
                self.pad_to(4, 0);
                let at = self.b.len();
                self.b.extend((v.len() as u32).to_le_bytes());
                for x in v {
                    self.b.extend(x.to_le_bytes());
                }
                at
            }
            Node::F64s(v) => {
                self.pad_to(4, 0);
                self.pad_to(8, 4);
                let at = self.b.len();
                self.b.extend((v.len() as u32).to_le_bytes());
                for x in v {
                    self.b.extend(x.to_le_bytes());
                }
                at
            }
            Node::Tables(v) => {
                self.pad_to(4, 0);
                let at = self.b.len();
                self.b.extend((v.len() as u32).to_le_bytes());
                let slots: Vec<usize> = (0..v.len())
                    .map(|i| {
                        self.b.extend([0; 4]);
                        at + 4 + 4 * i
                    })
                    .collect();
                for (slot, t) in slots.into_iter().zip(v) {
                    let p = self.node(t);
                    self.patch(slot, p);
                }
                at
            }
            Node::Table(fields) => self.table(fields),
        }
    }

    /// A table: its vtable just before it, its fields by size (8, 4, 2, 1 bytes) so each is aligned, its children
    /// after it.
    fn table(&mut self, mut fields: Vec<(u16, Item)>) -> usize {
        fields.sort_by_key(|(slot, item)| (std::cmp::Reverse(item.size()), *slot));
        let slots = fields.iter().map(|(s, _)| *s as usize + 1).max().unwrap_or(0);
        let vtable = 4 + 2 * slots;
        // The inline layout: the vtable offset, then the fields, each at a multiple of its size.
        let mut at = 4usize;
        let mut offsets = Vec::with_capacity(fields.len());
        for (_, item) in &fields {
            let size = item.size();
            at = at.div_ceil(size) * size;
            offsets.push(at);
            at += size;
        }
        let inline = at;
        // The table starts on a multiple of 8, the vtable just before it.
        self.pad_to(2, 0);
        self.pad_to(8, vtable);
        let vt = self.b.len();
        let mut entries = vec![0u16; slots];
        for ((slot, _), off) in fields.iter().zip(&offsets) {
            entries[*slot as usize] = *off as u16;
        }
        self.b.extend((vtable as u16).to_le_bytes());
        self.b.extend((inline as u16).to_le_bytes());
        for e in entries {
            self.b.extend(e.to_le_bytes());
        }
        let start = self.b.len();
        debug_assert_eq!(start - vt, vtable);
        self.b.extend(((start - vt) as i32).to_le_bytes());
        self.b.resize(start + inline, 0);
        let mut refs = Vec::new();
        for ((_, item), off) in fields.into_iter().zip(offsets) {
            let p = start + off;
            match item {
                Item::U8(v) => self.b[p] = v,
                Item::U16(v) => self.b[p..p + 2].copy_from_slice(&v.to_le_bytes()),
                Item::I32(v) => self.b[p..p + 4].copy_from_slice(&v.to_le_bytes()),
                Item::U64(v) => self.b[p..p + 8].copy_from_slice(&v.to_le_bytes()),
                Item::Ref(n) => refs.push((p, n)),
            }
        }
        for (p, n) in refs {
            let to = self.node(n);
            self.patch(p, to);
        }
        start
    }
}

/// A root table as one FlatBuffer with its size before it. As FlatBuffers' own `finish_size_prefixed`, values are
/// aligned from the size's first byte and the whole is a multiple of 8, so in the file every buffer starts on 8.
fn size_prefixed(root: Node) -> Vec<u8> {
    let mut out = Out { b: vec![0; 8] };
    let at = out.node(root);
    out.patch(4, at);
    out.pad_to(8, 0);
    let size = (out.b.len() - 4) as u32;
    out.b[..4].copy_from_slice(&size.to_le_bytes());
    out.b
}

// ── Geometry out ───────────────────────────────────────────────────────

struct Coords {
    xy: Vec<f64>,
    z: Vec<f64>,
    m: Vec<f64>,
}

impl Coords {
    fn new() -> Coords {
        Coords {
            xy: Vec::new(),
            z: Vec::new(),
            m: Vec::new(),
        }
    }

    fn push(&mut self, c: &Coord) {
        self.xy.push(c.x);
        self.xy.push(c.y);
        self.z.push(c.z.unwrap_or(f64::NAN));
        self.m.push(c.m.unwrap_or(f64::NAN));
    }

    fn points(&self) -> u32 {
        (self.xy.len() / 2) as u32
    }

    fn table(self, kind: u8, ends: Option<Vec<u32>>, has: (bool, bool)) -> Vec<(u16, Item)> {
        let mut t = vec![(6, Item::U8(kind))];
        if let Some(e) = ends {
            t.push((0, Item::Ref(Node::U32s(e))));
        }
        if !self.xy.is_empty() {
            t.push((1, Item::Ref(Node::F64s(self.xy))));
            if has.0 {
                t.push((2, Item::Ref(Node::F64s(self.z))));
            }
            if has.1 {
                t.push((3, Item::Ref(Node::F64s(self.m))));
            }
        }
        t
    }
}

fn linear(c: &Curve) -> Option<&[Coord]> {
    match c {
        Curve::Line(p) => Some(p),
        _ => None,
    }
}

/// Simple rings or lines as one coordinate run with their ends (none for one part, as GDAL writes it).
fn run(parts: &[&[Coord]], kind: u8, has: (bool, bool)) -> Node {
    let mut c = Coords::new();
    let mut ends = Vec::with_capacity(parts.len());
    for p in parts {
        p.iter().for_each(|x| c.push(x));
        ends.push(c.points());
    }
    let ends = (parts.len() > 1).then_some(ends);
    Node::Table(c.table(kind, ends, has))
}

fn curve_node(c: &Curve, has: (bool, bool)) -> Node {
    match c {
        Curve::Line(p) => run(&[p], gt::LINE_STRING, has),
        Curve::Arc(p) => run(&[p], gt::CIRCULAR_STRING, has),
        Curve::Compound(parts) => {
            let parts = parts.iter().map(|p| curve_node(p, has)).collect();
            Node::Table(vec![
                (6, Item::U8(gt::COMPOUND_CURVE)),
                (7, Item::Ref(Node::Tables(parts))),
            ])
        }
    }
}

fn polygon_node(rings: &[Curve], has: (bool, bool)) -> Node {
    match rings.iter().map(linear).collect::<Option<Vec<_>>>() {
        Some(lines) => run(&lines, gt::POLYGON, has),
        None => Node::Table(vec![
            (6, Item::U8(gt::CURVE_POLYGON)),
            (
                7,
                Item::Ref(Node::Tables(rings.iter().map(|r| curve_node(r, has)).collect())),
            ),
        ]),
    }
}

fn parts(kind: u8, parts: Vec<Node>) -> Node {
    Node::Table(vec![(6, Item::U8(kind)), (7, Item::Ref(Node::Tables(parts)))])
}

fn geometry_node(g: &Geometry, has: (bool, bool)) -> Node {
    match g {
        Geometry::Point(Some(c)) => run(&[std::slice::from_ref(c)], gt::POINT, has),
        Geometry::Point(None) => Node::Table(vec![(6, Item::U8(gt::POINT))]),
        Geometry::MultiPoint(p) => run(&[p], gt::MULTI_POINT, has),
        Geometry::Curve(c) => curve_node(c, has),
        Geometry::Polygon(rings) => polygon_node(rings, has),
        Geometry::MultiCurve(cs) => match cs.iter().map(linear).collect::<Option<Vec<_>>>() {
            Some(lines) => run(&lines, gt::MULTI_LINE_STRING, has),
            None => parts(gt::MULTI_CURVE, cs.iter().map(|c| curve_node(c, has)).collect()),
        },
        Geometry::MultiPolygon(polys) => {
            let kind = if polys.iter().flatten().all(|r| linear(r).is_some()) {
                gt::MULTI_POLYGON
            } else {
                gt::MULTI_SURFACE
            };
            parts(kind, polys.iter().map(|p| polygon_node(p, has)).collect())
        }
        Geometry::Collection(gs) => parts(
            gt::COLLECTION,
            gs.iter().map(|g| geometry_node(g, has)).collect(),
        ),
    }
}

/// The geometry's type code at the top (as the header names a layer of one type).
fn top_type(g: &Geometry) -> u8 {
    match geometry_node(g, (false, false)) {
        Node::Table(fields) => fields
            .iter()
            .find_map(|(s, i)| match (s, i) {
                (6, Item::U8(k)) => Some(*k),
                _ => None,
            })
            .unwrap_or(gt::UNKNOWN),
        _ => gt::UNKNOWN,
    }
}

fn each_coord(g: &Geometry, f: &mut dyn FnMut(&Coord)) {
    fn curve(c: &Curve, f: &mut dyn FnMut(&Coord)) {
        match c {
            Curve::Line(p) | Curve::Arc(p) => p.iter().for_each(&mut *f),
            Curve::Compound(ps) => ps.iter().for_each(|p| curve(p, f)),
        }
    }
    match g {
        Geometry::Point(p) => p.iter().for_each(&mut *f),
        Geometry::MultiPoint(p) => p.iter().for_each(&mut *f),
        Geometry::Curve(c) => curve(c, f),
        Geometry::Polygon(rs) | Geometry::MultiCurve(rs) => rs.iter().for_each(|c| curve(c, f)),
        Geometry::MultiPolygon(ps) => ps.iter().flatten().for_each(|c| curve(c, f)),
        Geometry::Collection(gs) => gs.iter().for_each(|g| each_coord(g, f)),
    }
}

/// A geometry's box (min x, min y, max x, max y); none when it has no position.
pub fn bounds(g: &Geometry) -> Option<[f64; 4]> {
    let mut b: Option<[f64; 4]> = None;
    each_coord(g, &mut |c| {
        let e = b.get_or_insert([c.x, c.y, c.x, c.y]);
        e[0] = e[0].min(c.x);
        e[1] = e[1].min(c.y);
        e[2] = e[2].max(c.x);
        e[3] = e[3].max(c.y);
    });
    b
}

// ── Properties out ─────────────────────────────────────────────────────

fn property(out: &mut Vec<u8>, i: u16, kind: ColumnType, v: &Value) {
    let text = |out: &mut Vec<u8>, t: &[u8]| {
        out.extend((t.len() as u32).to_le_bytes());
        out.extend(t);
    };
    let int = match v {
        Value::Integer(n) => Some(*n),
        Value::Real(r) if r.fract() == 0.0 && r.abs() < 9.2e18 => Some(*r as i64),
        Value::Text(t) => t.trim().parse().ok(),
        _ => None,
    };
    let real = match v {
        Value::Integer(n) => Some(*n as f64),
        Value::Real(r) => Some(*r),
        Value::Text(t) => t.trim().parse().ok(),
        _ => None,
    };
    let start = out.len();
    out.extend(i.to_le_bytes());
    let written = match (kind, v) {
        (_, Value::Null) => false,
        (ColumnType::Bool, _) => match int {
            Some(n) => {
                out.push(u8::from(n != 0));
                true
            }
            None => false,
        },
        (ColumnType::Byte | ColumnType::UByte, _) => int.is_some_and(|n| {
            out.push(n as u8);
            true
        }),
        (ColumnType::Short | ColumnType::UShort, _) => int.is_some_and(|n| {
            out.extend((n as i16).to_le_bytes());
            true
        }),
        (ColumnType::Int | ColumnType::UInt, _) => int.is_some_and(|n| {
            out.extend((n as i32).to_le_bytes());
            true
        }),
        (ColumnType::Long | ColumnType::ULong, _) => int.is_some_and(|n| {
            out.extend(n.to_le_bytes());
            true
        }),
        (ColumnType::Float, _) => real.is_some_and(|r| {
            out.extend((r as f32).to_le_bytes());
            true
        }),
        (ColumnType::Double, _) => real.is_some_and(|r| {
            out.extend(r.to_le_bytes());
            true
        }),
        (ColumnType::Binary, Value::Blob(b)) => {
            text(out, b);
            true
        }
        (_, Value::Blob(_)) => false,
        (_, Value::Text(t)) => {
            text(out, t.as_bytes());
            true
        }
        (_, Value::Integer(n)) => {
            text(out, n.to_string().as_bytes());
            true
        }
        (_, Value::Real(r)) => {
            text(out, format!("{r}").as_bytes());
            true
        }
    };
    if !written {
        out.truncate(start);
    }
}

// ── The index ──────────────────────────────────────────────────────────

/// Each level's node range, the leaves (level 0) last in the array, the root first.
fn level_bounds(items: usize, node_size: usize) -> Vec<std::ops::Range<usize>> {
    let mut counts = vec![items];
    let mut n = items;
    loop {
        n = n.div_ceil(node_size);
        counts.push(n);
        if n <= 1 {
            break;
        }
    }
    let total: usize = counts.iter().sum();
    let mut end = total;
    counts
        .iter()
        .map(|c| {
            let r = end - c..end;
            end -= c;
            r
        })
        .collect()
}

/// The index's size in bytes for `items` features.
pub fn index_size(items: u64, node_size: u16) -> Option<u64> {
    if items == 0 || node_size == 0 {
        return Some(0);
    }
    let node_size = u64::from(node_size.max(2));
    let mut n = items;
    let mut nodes = n;
    loop {
        n = n.div_ceil(node_size);
        nodes = nodes.checked_add(n)?;
        if n <= 1 {
            break;
        }
    }
    nodes.checked_mul(40)
}

/// The Hilbert code of a cell of the 2¹⁶ × 2¹⁶ grid (as FlatGeobuf orders its features).
pub fn hilbert(x: u32, y: u32) -> u32 {
    let mut a = x ^ y;
    let mut b = 0xFFFF ^ a;
    let mut c = 0xFFFF ^ (x | y);
    let mut d = x & (y ^ 0xFFFF);
    let mut aa = a | (b >> 1);
    let mut bb = (a >> 1) ^ a;
    let mut cc = ((c >> 1) ^ (b & (d >> 1))) ^ c;
    let mut dd = ((a & (c >> 1)) ^ (d >> 1)) ^ d;
    a = aa;
    b = bb;
    c = cc;
    d = dd;
    aa = (a & (a >> 2)) ^ (b & (b >> 2));
    bb = (a & (b >> 2)) ^ (b & ((a ^ b) >> 2));
    cc ^= (a & (c >> 2)) ^ (b & (d >> 2));
    dd ^= (b & (c >> 2)) ^ ((a ^ b) & (d >> 2));
    a = aa;
    b = bb;
    c = cc;
    d = dd;
    aa = (a & (a >> 4)) ^ (b & (b >> 4));
    bb = (a & (b >> 4)) ^ (b & ((a ^ b) >> 4));
    cc ^= (a & (c >> 4)) ^ (b & (d >> 4));
    dd ^= (b & (c >> 4)) ^ ((a ^ b) & (d >> 4));
    a = aa;
    b = bb;
    c = cc;
    d = dd;
    cc ^= (a & (c >> 8)) ^ (b & (d >> 8));
    dd ^= (b & (c >> 8)) ^ ((a ^ b) & (d >> 8));
    a = cc ^ (cc >> 1);
    b = dd ^ (dd >> 1);
    let mut i0 = x ^ y;
    let mut i1 = b | (0xFFFF ^ (i0 | a));
    for (shift, mask) in [(8, 0x00FF_00FF), (4, 0x0F0F_0F0F), (2, 0x3333_3333), (1, 0x5555_5555)] {
        i0 = (i0 | (i0 << shift)) & mask;
        i1 = (i1 | (i1 << shift)) & mask;
    }
    (i1 << 1) | i0
}

// ── Writing a file ─────────────────────────────────────────────────────

/// The layer as one FlatGeobuf file. Its features are written in Hilbert order of their boxes' centres with the
/// packed R-tree before them; a layer with a feature that has no position has no index (node size 0), as GDAL writes.
pub fn write(layer: &Layer<'_>) -> Vec<u8> {
    let feats = layer.features;
    let boxes: Vec<Option<[f64; 4]>> = feats
        .iter()
        .map(|(g, _)| g.as_ref().and_then(bounds))
        .collect();
    let has_z = feats.iter().any(|(g, _)| {
        g.as_ref().is_some_and(|g| {
            let mut z = false;
            each_coord(g, &mut |c| z |= c.z.is_some());
            z
        })
    });
    let has_m = feats.iter().any(|(g, _)| {
        g.as_ref().is_some_and(|g| {
            let mut m = false;
            each_coord(g, &mut |c| m |= c.m.is_some());
            m
        })
    });
    let types: std::collections::BTreeSet<u8> =
        feats.iter().filter_map(|(g, _)| g.as_ref().map(top_type)).collect();
    let header_type = match (types.len(), types.first()) {
        (1, Some(t)) => *t,
        _ => gt::UNKNOWN,
    };
    let envelope = boxes.iter().flatten().fold(None::<[f64; 4]>, |acc, b| {
        Some(match acc {
            None => *b,
            Some(a) => [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])],
        })
    });
    let indexed = !feats.is_empty() && boxes.iter().all(Option::is_some);
    // The order: by the Hilbert code of the box's centre, high first (as FlatGeobuf's own writer).
    let mut order: Vec<usize> = (0..feats.len()).collect();
    if let (true, Some(e)) = (indexed, envelope) {
        let (w, h) = (e[2] - e[0], e[3] - e[1]);
        let code = |b: &[f64; 4]| {
            let cell = |v: f64, lo: f64, span: f64| {
                if span > 0.0 {
                    (65535.0 * ((v - lo) / span)).floor() as u32
                } else {
                    0
                }
            };
            hilbert(
                cell((b[0] + b[2]) / 2.0, e[0], w),
                cell((b[1] + b[3]) / 2.0, e[1], h),
            )
        };
        let codes: Vec<u32> = boxes.iter().map(|b| b.as_ref().map_or(0, code)).collect();
        order.sort_by(|a, b| codes[*b].cmp(&codes[*a]).then(a.cmp(b)));
    }
    // The features, each a size-prefixed buffer.
    let mut features = Vec::new();
    let mut offsets = Vec::with_capacity(feats.len());
    for &i in &order {
        let (g, values) = &feats[i];
        offsets.push(features.len() as u64);
        let mut fields = Vec::new();
        if let Some(g) = g {
            fields.push((0, Item::Ref(geometry_node(g, (has_z, has_m)))));
        }
        let mut props = Vec::new();
        for (k, (c, v)) in layer.columns.iter().zip(values).enumerate() {
            property(&mut props, k as u16, c.kind, v);
        }
        if !props.is_empty() {
            fields.push((1, Item::Ref(Node::Bytes(props))));
        }
        features.extend(size_prefixed(Node::Table(fields)));
    }
    // The header.
    let columns: Vec<Node> = layer
        .columns
        .iter()
        .map(|c| {
            let mut t = vec![
                (0, Item::Ref(Node::Str(c.name.clone()))),
                (1, Item::U8(c.kind as u8)),
            ];
            if let Some(w) = c.width {
                t.push((4, Item::I32(w as i32)));
            }
            Node::Table(t)
        })
        .collect();
    let mut header = vec![
        (0, Item::Ref(Node::Str(layer.name.to_owned()))),
        (2, Item::U8(header_type)),
        (8, Item::U64(feats.len() as u64)),
        (9, Item::U16(if indexed { NODE_SIZE } else { 0 })),
    ];
    if let Some(e) = envelope {
        header.push((1, Item::Ref(Node::F64s(e.to_vec()))));
    }
    if has_z {
        header.push((3, Item::U8(1)));
    }
    if has_m {
        header.push((4, Item::U8(1)));
    }
    if !columns.is_empty() {
        header.push((7, Item::Ref(Node::Tables(columns))));
    }
    if let Some(s) = layer.srid {
        header.push((
            10,
            Item::Ref(Node::Table(vec![
                (0, Item::Ref(Node::Str("EPSG".into()))),
                (1, Item::I32(s as i32)),
            ])),
        ));
    }
    let mut file = MAGIC.to_vec();
    file.extend(size_prefixed(Node::Table(header)));
    if indexed {
        let node_size = NODE_SIZE as usize;
        let levels = level_bounds(feats.len(), node_size);
        let total = levels.first().map_or(0, |r| r.end);
        let mut nodes = vec![([f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY], 0u64); total];
        for (k, &i) in order.iter().enumerate() {
            let b = boxes[i].unwrap_or([0.0; 4]);
            nodes[levels[0].start + k] = (b, offsets[k]);
        }
        for l in 0..levels.len() - 1 {
            let (mut pos, end) = (levels[l].start, levels[l].end);
            let mut parent = levels[l + 1].start;
            while pos < end {
                let mut node = (
                    [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY],
                    pos as u64,
                );
                for _ in 0..node_size {
                    if pos >= end {
                        break;
                    }
                    let b = nodes[pos].0;
                    node.0[0] = node.0[0].min(b[0]);
                    node.0[1] = node.0[1].min(b[1]);
                    node.0[2] = node.0[2].max(b[2]);
                    node.0[3] = node.0[3].max(b[3]);
                    pos += 1;
                }
                nodes[parent] = node;
                parent += 1;
            }
        }
        for (b, off) in nodes {
            for v in b {
                file.extend(v.to_le_bytes());
            }
            file.extend(off.to_le_bytes());
        }
    }
    file.extend(features);
    file
}

// ── Reading FlatBuffers, checked ───────────────────────────────────────

#[derive(Clone, Copy)]
struct Buf<'a> {
    b: &'a [u8],
}

#[derive(Clone, Copy)]
struct Table<'a> {
    buf: Buf<'a>,
    at: usize,
    vt: usize,
    vt_len: usize,
}

type R<T> = Result<T, String>;

fn bad(what: &str) -> String {
    format!("FlatBuffer bozuk: {what}")
}

impl<'a> Buf<'a> {
    fn u8(&self, at: usize) -> R<u8> {
        self.b.get(at).copied().ok_or_else(|| bad("dosyanın sonu"))
    }

    /// `N` bytes at `at`. Alignment is not asked: writers align from the size prefix, the reader holds the buffer
    /// after it, and the values are copied out.
    fn take<const N: usize>(&self, at: usize, _align: usize) -> R<[u8; N]> {
        self.b
            .get(at..at.checked_add(N).ok_or_else(|| bad("taşma"))?)
            .and_then(|s| s.try_into().ok())
            .ok_or_else(|| bad("dosyanın sonu"))
    }

    fn u16(&self, at: usize) -> R<u16> {
        Ok(u16::from_le_bytes(self.take(at, 2)?))
    }

    fn u32(&self, at: usize) -> R<u32> {
        Ok(u32::from_le_bytes(self.take(at, 4)?))
    }

    fn i32(&self, at: usize) -> R<i32> {
        Ok(i32::from_le_bytes(self.take(at, 4)?))
    }

    fn u64(&self, at: usize) -> R<u64> {
        Ok(u64::from_le_bytes(self.take(at, 8)?))
    }

    fn f64(&self, at: usize) -> R<f64> {
        Ok(f64::from_le_bytes(self.take(at, 8)?))
    }

    /// Where the offset at `at` points.
    fn follow(&self, at: usize) -> R<usize> {
        let to = at
            .checked_add(self.u32(at)? as usize)
            .ok_or_else(|| bad("taşma"))?;
        if to >= self.b.len() {
            return Err(bad("dosyanın dışına gösteren başvuru"));
        }
        Ok(to)
    }

    fn table(&self, at: usize) -> R<Table<'a>> {
        let soff = self.i32(at)? as i64;
        let vt = (at as i64)
            .checked_sub(soff)
            .filter(|v| *v >= 0)
            .ok_or_else(|| bad("vtable"))? as usize;
        let vt_len = self.u16(vt)? as usize;
        let inline = self.u16(vt + 2)? as usize;
        if vt_len < 4 || !vt_len.is_multiple_of(2) || at + inline > self.b.len() {
            return Err(bad("vtable"));
        }
        Ok(Table {
            buf: *self,
            at,
            vt,
            vt_len,
        })
    }

    /// A vector's length and where its first element is.
    fn vector(&self, at: usize, elem: usize) -> R<(usize, usize)> {
        let n = self.u32(at)? as usize;
        let first = at + 4;
        let end = n
            .checked_mul(elem)
            .and_then(|s| first.checked_add(s))
            .ok_or_else(|| bad("taşma"))?;
        if end > self.b.len() {
            return Err(bad("dizinin sonu dosyanın dışında"));
        }
        Ok((n, first))
    }
}

impl<'a> Table<'a> {
    fn field(&self, slot: usize) -> R<Option<usize>> {
        let e = 4 + 2 * slot;
        if e + 2 > self.vt_len {
            return Ok(None);
        }
        let off = self.buf.u16(self.vt + e)? as usize;
        Ok((off != 0).then_some(self.at + off))
    }

    fn u8(&self, slot: usize, default: u8) -> R<u8> {
        self.field(slot)?.map_or(Ok(default), |p| self.buf.u8(p))
    }

    fn u16(&self, slot: usize, default: u16) -> R<u16> {
        self.field(slot)?.map_or(Ok(default), |p| self.buf.u16(p))
    }

    fn i32(&self, slot: usize, default: i32) -> R<i32> {
        self.field(slot)?.map_or(Ok(default), |p| self.buf.i32(p))
    }

    fn u64(&self, slot: usize, default: u64) -> R<u64> {
        self.field(slot)?.map_or(Ok(default), |p| self.buf.u64(p))
    }

    fn string(&self, slot: usize) -> R<Option<String>> {
        let Some(p) = self.field(slot)? else {
            return Ok(None);
        };
        let at = self.buf.follow(p)?;
        let (n, first) = self.buf.vector(at, 1)?;
        let bytes = &self.buf.b[first..first + n];
        Ok(Some(String::from_utf8_lossy(bytes).into_owned()))
    }

    fn bytes(&self, slot: usize) -> R<&'a [u8]> {
        let Some(p) = self.field(slot)? else {
            return Ok(&[]);
        };
        let at = self.buf.follow(p)?;
        let (n, first) = self.buf.vector(at, 1)?;
        Ok(&self.buf.b[first..first + n])
    }

    fn f64s(&self, slot: usize) -> R<Vec<f64>> {
        let Some(p) = self.field(slot)? else {
            return Ok(Vec::new());
        };
        let at = self.buf.follow(p)?;
        let (n, first) = self.buf.vector(at, 8)?;
        (0..n).map(|i| self.buf.f64(first + 8 * i)).collect()
    }

    fn u32s(&self, slot: usize) -> R<Option<Vec<u32>>> {
        let Some(p) = self.field(slot)? else {
            return Ok(None);
        };
        let at = self.buf.follow(p)?;
        let (n, first) = self.buf.vector(at, 4)?;
        (0..n).map(|i| self.buf.u32(first + 4 * i)).collect::<R<_>>().map(Some)
    }

    fn table(&self, slot: usize) -> R<Option<Table<'a>>> {
        let Some(p) = self.field(slot)? else {
            return Ok(None);
        };
        let at = self.buf.follow(p)?;
        self.buf.table(at).map(Some)
    }

    fn tables(&self, slot: usize) -> R<Vec<Table<'a>>> {
        let Some(p) = self.field(slot)? else {
            return Ok(Vec::new());
        };
        let at = self.buf.follow(p)?;
        let (n, first) = self.buf.vector(at, 4)?;
        (0..n)
            .map(|i| {
                let e = first + 4 * i;
                let to = self.buf.follow(e)?;
                self.buf.table(to)
            })
            .collect()
    }
}

/// The root table of a buffer (the size prefix taken off).
fn root(b: &[u8]) -> R<Table<'_>> {
    let buf = Buf { b };
    let at = buf.follow(0)?;
    buf.table(at)
}

// ── Geometry in ────────────────────────────────────────────────────────

fn coords(t: &Table<'_>, has: (bool, bool)) -> R<Vec<Coord>> {
    let xy = t.f64s(1)?;
    if !xy.len().is_multiple_of(2) {
        return Err(bad("xy dizisinin uzunluğu tek"));
    }
    let n = xy.len() / 2;
    let z = if has.0 { t.f64s(2)? } else { Vec::new() };
    let m = if has.1 { t.f64s(3)? } else { Vec::new() };
    let pick = |v: &[f64], i: usize| v.get(i).copied().filter(|x| !x.is_nan());
    Ok((0..n)
        .map(|i| Coord {
            x: xy[2 * i],
            y: xy[2 * i + 1],
            z: pick(&z, i),
            m: pick(&m, i),
        })
        .collect())
}

fn split(t: &Table<'_>, has: (bool, bool)) -> R<Vec<Vec<Coord>>> {
    let all = coords(t, has)?;
    let Some(ends) = t.u32s(0)? else {
        return Ok(if all.is_empty() { Vec::new() } else { vec![all] });
    };
    let mut out = Vec::with_capacity(ends.len());
    let mut from = 0usize;
    for e in ends {
        let e = e as usize;
        if e < from || e > all.len() {
            return Err(bad("parçaların sonları"));
        }
        out.push(all[from..e].to_vec());
        from = e;
    }
    Ok(out)
}

fn curve_of(t: &Table<'_>, kind: u8, has: (bool, bool), depth: usize) -> R<Curve> {
    if depth > MAX_DEPTH {
        return Err(bad("çok derin geometri"));
    }
    match kind {
        gt::LINE_STRING => Ok(Curve::Line(coords(t, has)?)),
        gt::CIRCULAR_STRING => Ok(Curve::Arc(coords(t, has)?)),
        gt::COMPOUND_CURVE => t
            .tables(7)?
            .iter()
            .map(|p| curve_of(p, p.u8(6, gt::UNKNOWN)?, has, depth + 1))
            .collect::<R<_>>()
            .map(Curve::Compound),
        k => Err(format!("{k} türü bir eğri değil")),
    }
}

fn rings_of(t: &Table<'_>, kind: u8, has: (bool, bool), depth: usize) -> R<Vec<Curve>> {
    match kind {
        gt::POLYGON => Ok(split(t, has)?.into_iter().map(Curve::Line).collect()),
        gt::CURVE_POLYGON => t
            .tables(7)?
            .iter()
            .map(|p| curve_of(p, p.u8(6, gt::UNKNOWN)?, has, depth + 1))
            .collect(),
        k => Err(format!("{k} türü bir alan değil")),
    }
}

fn geometry_of(t: &Table<'_>, kind: u8, has: (bool, bool), depth: usize) -> R<Geometry> {
    if depth > MAX_DEPTH {
        return Err(bad("çok derin geometri"));
    }
    let own = t.u8(6, gt::UNKNOWN)?;
    let kind = if own == gt::UNKNOWN { kind } else { own };
    Ok(match kind {
        gt::POINT => Geometry::Point(coords(t, has)?.into_iter().next()),
        gt::MULTI_POINT => Geometry::MultiPoint(coords(t, has)?),
        gt::LINE_STRING | gt::CIRCULAR_STRING | gt::COMPOUND_CURVE => {
            Geometry::Curve(curve_of(t, kind, has, depth)?)
        }
        gt::POLYGON | gt::CURVE_POLYGON => Geometry::Polygon(rings_of(t, kind, has, depth)?),
        gt::MULTI_LINE_STRING => {
            Geometry::MultiCurve(split(t, has)?.into_iter().map(Curve::Line).collect())
        }
        gt::MULTI_CURVE => Geometry::MultiCurve(
            t.tables(7)?
                .iter()
                .map(|p| curve_of(p, p.u8(6, gt::UNKNOWN)?, has, depth + 1))
                .collect::<R<_>>()?,
        ),
        gt::MULTI_POLYGON | gt::MULTI_SURFACE => Geometry::MultiPolygon(
            t.tables(7)?
                .iter()
                .map(|p| {
                    let k = match p.u8(6, gt::UNKNOWN)? {
                        gt::UNKNOWN => gt::POLYGON,
                        k => k,
                    };
                    rings_of(p, k, has, depth + 1)
                })
                .collect::<R<_>>()?,
        ),
        gt::COLLECTION => Geometry::Collection(
            t.tables(7)?
                .iter()
                .map(|p| geometry_of(p, gt::UNKNOWN, has, depth + 1))
                .collect::<R<_>>()?,
        ),
        gt::UNKNOWN => return Err("geometrinin türü yok".into()),
        k => return Err(format!("FlatGeobuf geometri türü {k} okunmuyor (çokyüzlü yüzey, TIN, üçgen)")),
    })
}

// ── Properties in ──────────────────────────────────────────────────────

fn properties(bytes: &[u8], columns: &[FgbColumn]) -> R<Vec<Value>> {
    let mut values = vec![Value::Null; columns.len()];
    let mut at = 0usize;
    let get = |at: usize, n: usize| bytes.get(at..at + n).ok_or_else(|| bad("özelliklerin sonu"));
    while at < bytes.len() {
        let i = u16::from_le_bytes(get(at, 2)?.try_into().map_err(|_| bad("özellik"))?) as usize;
        at += 2;
        let c = columns.get(i).ok_or_else(|| bad("olmayan sütunun değeri"))?;
        let fixed = |n: usize| -> R<&[u8]> { get(at, n) };
        let (v, n) = match c.kind {
            ColumnType::Bool => (Value::Integer(i64::from(fixed(1)?[0] != 0)), 1),
            ColumnType::Byte => (Value::Integer(i64::from(fixed(1)?[0] as i8)), 1),
            ColumnType::UByte => (Value::Integer(i64::from(fixed(1)?[0])), 1),
            ColumnType::Short => (
                Value::Integer(i64::from(i16::from_le_bytes(fixed(2)?.try_into().unwrap_or_default()))),
                2,
            ),
            ColumnType::UShort => (
                Value::Integer(i64::from(u16::from_le_bytes(fixed(2)?.try_into().unwrap_or_default()))),
                2,
            ),
            ColumnType::Int => (
                Value::Integer(i64::from(i32::from_le_bytes(fixed(4)?.try_into().unwrap_or_default()))),
                4,
            ),
            ColumnType::UInt => (
                Value::Integer(i64::from(u32::from_le_bytes(fixed(4)?.try_into().unwrap_or_default()))),
                4,
            ),
            ColumnType::Long => (
                Value::Integer(i64::from_le_bytes(fixed(8)?.try_into().unwrap_or_default())),
                8,
            ),
            ColumnType::ULong => {
                let u = u64::from_le_bytes(fixed(8)?.try_into().unwrap_or_default());
                (
                    i64::try_from(u).map_or_else(|_| Value::Text(u.to_string()), Value::Integer),
                    8,
                )
            }
            ColumnType::Float => (
                Value::Real(f64::from(f32::from_le_bytes(fixed(4)?.try_into().unwrap_or_default()))),
                4,
            ),
            ColumnType::Double => (
                Value::Real(f64::from_le_bytes(fixed(8)?.try_into().unwrap_or_default())),
                8,
            ),
            ColumnType::String | ColumnType::Json | ColumnType::DateTime | ColumnType::Binary => {
                let len = u32::from_le_bytes(fixed(4)?.try_into().unwrap_or_default()) as usize;
                let body = get(at + 4, len)?;
                let v = if c.kind == ColumnType::Binary {
                    Value::Blob(body.to_vec())
                } else {
                    Value::Text(String::from_utf8_lossy(body).into_owned())
                };
                (v, 4 + len)
            }
        };
        values[i] = v;
        at += n;
    }
    Ok(values)
}

fn date_only(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 10 && b[4] == b'-' && b[7] == b'-' && b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

/// A table of a FlatGeobuf column as the import reads it: a date-time column holding only days is a date.
fn column_of(c: &FgbColumn, values: &[&Value]) -> Column {
    let kind = match c.kind {
        ColumnType::Bool => ColumnKind::Boolean,
        ColumnType::Byte
        | ColumnType::UByte
        | ColumnType::Short
        | ColumnType::UShort
        | ColumnType::Int
        | ColumnType::UInt
        | ColumnType::Long => ColumnKind::Integer,
        ColumnType::ULong => {
            if values.iter().any(|v| matches!(v, Value::Text(_))) {
                ColumnKind::Text(None)
            } else {
                ColumnKind::Integer
            }
        }
        ColumnType::Float | ColumnType::Double => ColumnKind::Real,
        ColumnType::String | ColumnType::Json => ColumnKind::Text(c.width.filter(|w| *w > 0)),
        ColumnType::Binary => ColumnKind::Blob(None),
        ColumnType::DateTime => {
            let days = values.iter().all(|v| match v {
                Value::Text(t) => date_only(t),
                _ => true,
            });
            if days && values.iter().any(|v| !matches!(v, Value::Null)) {
                ColumnKind::Date
            } else {
                ColumnKind::DateTime
            }
        }
    };
    Column {
        name: c.name.clone(),
        decl: String::new(),
        kind,
        not_null: false,
        default: None,
    }
}

fn prefixed(file: &[u8], at: usize, what: &str) -> R<(usize, usize)> {
    let size = file
        .get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
        .ok_or_else(|| format!("{what} yarım"))?;
    if size > MAX_BUFFER || at + 4 + size > file.len() {
        return Err(format!("{what} dosyanın dışına taşıyor"));
    }
    Ok((at + 4, size))
}

/// The file's layer, or why it cannot be read. A feature whose geometry is not read keeps its values (its geometry
/// says why).
pub fn read(file: &[u8]) -> R<Read> {
    if file.len() < 12 || &file[..3] != b"fgb" || file[3] != 3 || &file[4..7] != b"fgb" {
        return Err("FlatGeobuf dosyası değil (başı “fgb\\x03fgb” değil)".into());
    }
    let (h_at, h_len) = prefixed(file, 8, "başlık")?;
    let header = root(&file[h_at..h_at + h_len])?;
    let name = header.string(0)?.unwrap_or_default();
    let header_type = header.u8(2, gt::UNKNOWN)?;
    let has = (header.u8(3, 0)? != 0, header.u8(4, 0)? != 0);
    let declared: Vec<FgbColumn> = header
        .tables(7)?
        .iter()
        .map(|c| {
            Ok(FgbColumn {
                name: c.string(0)?.unwrap_or_default(),
                kind: ColumnType::of(c.u8(1, 0)?).ok_or_else(|| bad("bilinmeyen sütun türü"))?,
                width: u32::try_from(c.i32(4, -1)?).ok(),
            })
        })
        .collect::<R<_>>()?;
    let count = header.u64(8, 0)?;
    let node_size = header.u16(9, NODE_SIZE)?;
    let (srid, other_crs) = match header.table(10)? {
        None => (None, None),
        Some(c) => {
            let org = c.string(0)?.unwrap_or_default();
            let code = c.i32(1, 0)?;
            if code > 0 && (org.is_empty() || org.eq_ignore_ascii_case("EPSG")) {
                (Some(code as u32), None)
            } else if code > 0 || !org.is_empty() {
                (None, Some(format!("{org}:{code}")))
            } else {
                (None, c.string(4)?.map(|_| "WKT".to_owned()))
            }
        }
    };
    let mut at = h_at + h_len;
    let index = index_size(count, node_size).ok_or_else(|| "dizinin boyu taşıyor".to_owned())?;
    at = at
        .checked_add(index as usize)
        .filter(|a| *a <= file.len())
        .ok_or_else(|| "dizin dosyanın dışına taşıyor".to_owned())?;
    let mut features = Vec::new();
    while at < file.len() {
        let (f_at, f_len) = prefixed(file, at, "nesne")?;
        let f = root(&file[f_at..f_at + f_len])?;
        let geometry = f
            .table(0)?
            .map(|g| geometry_of(&g, header_type, has, 0).map_err(BlobError));
        let values = properties(f.bytes(1)?, &declared)?;
        features.push((geometry, values));
        at = f_at + f_len;
    }
    if count != 0 && features.len() as u64 != count {
        return Err(format!(
            "başlık {count} nesne söylüyor, dosyada {} nesne var",
            features.len()
        ));
    }
    let columns = declared
        .iter()
        .enumerate()
        .map(|(k, c)| {
            let vs: Vec<&Value> = features.iter().map(|(_, v)| &v[k]).collect();
            column_of(c, &vs)
        })
        .collect();
    Ok(Read {
        name,
        srid,
        other_crs,
        columns,
        declared,
        features,
        indexed: node_size > 0 && count > 0,
        has_z: has.0,
        has_m: has.1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(x: f64, y: f64) -> Coord {
        Coord::xy(x, y)
    }

    #[test]
    fn a_layer_goes_round() {
        let ring = vec![c(0.0, 0.0), c(10.0, 0.0), c(10.0, 10.0), c(0.0, 0.0)];
        let hole = vec![c(2.0, 2.0), c(3.0, 2.0), c(3.0, 3.0), c(2.0, 2.0)];
        let arc = Curve::Arc(vec![c(0.0, 0.0), c(1.0, 1.0), c(2.0, 0.0)]);
        let features = vec![
            (
                Some(Geometry::Polygon(vec![Curve::Line(ring.clone()), Curve::Line(hole)])),
                vec![Value::Text("Çağlayan".into()), Value::Integer(5), Value::Null],
            ),
            (
                Some(Geometry::Point(Some(Coord { x: 5.0, y: 6.0, z: Some(7.5), m: None }))),
                vec![Value::Null, Value::Integer(-3), Value::Real(1.25)],
            ),
            (
                Some(Geometry::Curve(Curve::Compound(vec![
                    arc,
                    Curve::Line(vec![c(2.0, 0.0), c(4.0, 0.0)]),
                ]))),
                vec![Value::Text("".into()), Value::Null, Value::Real(f64::MAX)],
            ),
            (
                Some(Geometry::MultiPolygon(vec![vec![Curve::Line(ring)]])),
                vec![Value::Null, Value::Null, Value::Null],
            ),
        ];
        let columns = vec![
            FgbColumn { name: "ad".into(), kind: ColumnType::String, width: Some(40) },
            FgbColumn { name: "no".into(), kind: ColumnType::Long, width: None },
            FgbColumn { name: "alan".into(), kind: ColumnType::Double, width: None },
        ];
        let bytes = write(&Layer { name: "parsel", srid: Some(5256), columns: &columns, features: &features });
        let back = read(&bytes).expect("reads");
        assert_eq!(back.name, "parsel");
        assert_eq!(back.srid, Some(5256));
        assert!(back.indexed && back.has_z);
        assert_eq!(back.declared, columns);
        assert_eq!(back.features.len(), 4);
        // Every feature comes back, in the file's (Hilbert) order.
        for (g, v) in &features {
            assert!(
                back.features.iter().any(|(bg, bv)| bg.as_ref().map(|r| r.as_ref().ok()) == Some(g.as_ref()) && bv == v),
                "{g:?} {v:?}"
            );
        }
    }

    #[test]
    fn a_feature_without_a_position_leaves_the_index_out() {
        let features = vec![(None, vec![Value::Integer(1)])];
        let columns = vec![FgbColumn { name: "n".into(), kind: ColumnType::Long, width: None }];
        let bytes = write(&Layer { name: "kayit", srid: None, columns: &columns, features: &features });
        let back = read(&bytes).expect("reads");
        assert!(!back.indexed);
        assert_eq!(back.features, vec![(None, vec![Value::Integer(1)])]);
    }

    #[test]
    fn a_broken_file_is_refused() {
        assert!(read(b"not a file at all").is_err());
        let features = vec![(Some(Geometry::Point(Some(c(1.0, 2.0)))), vec![])];
        let bytes = write(&Layer { name: "n", srid: None, columns: &[], features: &features });
        for cut in [9, 20, bytes.len() - 3] {
            assert!(read(&bytes[..cut]).is_err(), "cut at {cut}");
        }
        let mut flipped = bytes.clone();
        let at = 12;
        flipped[at] = 0xFF;
        flipped[at + 1] = 0xFF;
        assert!(read(&flipped).is_err());
    }

    #[test]
    fn the_index_size_is_the_formats() {
        // 1 feature: one leaf and the root. 17 at 16: 17 leaves, 2 nodes, 1 root.
        assert_eq!(index_size(1, 16), Some(80));
        assert_eq!(index_size(17, 16), Some(40 * 20));
        assert_eq!(index_size(0, 16), Some(0));
    }
}

#[cfg(test)]
mod gdal_files {
    use super::*;

    fn dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/fgb/v1")
    }

    /// GDAL's files read whole; ours written for GDAL to read (`KENTOS_WRITE_FGB=1`; fgb_exchange_cases.py checks
    /// them).
    #[test]
    fn gdals_files_read_and_ours_go_back() {
        let mut names: Vec<_> = std::fs::read_dir(dir())
            .expect("the fixtures")
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "fgb"))
            .collect();
        names.sort();
        assert!(names.len() >= 8, "{names:?}");
        let out = dir().join("geri");
        for p in names {
            let bytes = std::fs::read(&p).expect("read");
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            if name == "bozuk.fgb" || name == "kesik.fgb" {
                assert!(read(&bytes).is_err(), "{name} is refused");
                continue;
            }
            let r = read(&bytes).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            assert!(
                r.features.iter().all(|(g, _)| !matches!(g, Some(Err(_)))),
                "{}: {:?}",
                p.display(),
                r.features
            );
            if std::env::var_os("KENTOS_WRITE_FGB").is_some() {
                std::fs::create_dir_all(&out).expect("a folder");
                let features: Vec<(Option<Geometry>, Vec<Value>)> = r
                    .features
                    .iter()
                    .map(|(g, v)| (g.clone().and_then(Result::ok), v.clone()))
                    .collect();
                let again = write(&Layer {
                    name: &r.name,
                    srid: r.srid,
                    columns: &r.declared,
                    features: &features,
                });
                std::fs::write(out.join(p.file_name().expect("a name")), again).expect("written");
            }
        }
    }
}
