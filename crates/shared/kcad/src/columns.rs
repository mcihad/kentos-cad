//! A drawing's objects as typed columns (docs/adr/0030, TODOS.md FILE-15):
//! how the browser hands a drawing between its page and its formats worker.
//! A JSON text of a large drawing, or a structured copy of every object,
//! took seconds and held the page; these are six typed arrays the two sides
//! pass without copying (transferable buffers), written and read by one loop
//! on each side. The page's side is `apps/web/src/io/columns.ts`, which
//! follows the same layout; the fixtures hold the two to each other. The
//! desktop compares a written drawing with the one read back through the
//! same packing (`first_difference`), every float bit for bit.
//!
//! Everything but the objects (name, settings, layers, styles …) stays the
//! contract's JSON: it is small, and the style engine's parts are JSON anyway.
//!
//! Six streams, each read front to back:
//!
//! - `kinds`: one byte per object, its index in [`KINDS`];
//! - `uids`: sixteen bytes per object, its persistent id;
//! - `ints` (u32): first the number of layer ids in the layer table, then per
//!   object its layer (table index), flags, attribute count and the kind's
//!   own counts;
//! - `floats` (f64, bit for bit: −0 stays −0): per object the kind's numbers;
//! - `text` (UTF-16 code units, as JavaScript holds its strings) and
//!   `text_lengths` (code units of each text): first the layer table (layer
//!   ids in the order objects first use them), then per object its colour,
//!   label and symbol when it has them, its attributes as key, value pairs
//!   in key order (UTF-8 bytes, as the contract's map), then the kind's texts.
//!
//! | Kind | ints | floats | texts |
//! |---|---|---|---|
//! | every object | layer, flags, attributes | | colour?, label?, symbol?, key and value per attribute |
//! | point | | p, z? | |
//! | line | | a, b | |
//! | polyline, polygon | n; m if bulges; h if holes, then per hole: hole flags, k, j if bulges | pts (2n), bulges (m), per hole: pts (2k), bulges (j) | |
//! | circle | | c, r | |
//! | arc | | c, r, a0, a1 | |
//! | ellipse | | c, major, ratio, t0, t1 | |
//! | spline | n, closed (0 or 1) | pts (2n) | |
//! | xline, ray | | p, dir | |
//! | text | | p, height, rotation | text |
//! | dimension | style if any | a, b, offset, height, angle if any, c if any | text if any |
//! | hatch | n, pattern type; h if holes, then k per hole | ring (2n), pattern angle, spacing, per hole: pts (2k) | |
//!
//! A point is two floats, x then y. Flags: 1 colour, 2 label, 4 symbol; a
//! kind's optional fields from bit 8 up, in the order the table names them
//! (point: z; polyline and polygon: bulges, holes; dimension: text, style,
//! angle, c; hatch: holes). A hole's flags: 1 bulges. Dimension styles and
//! hatch pattern types are numbered in the contract's order.

use std::collections::{BTreeMap, HashMap};

use kentos_contracts::{
    ArcEntity, CircleEntity, ConstructionEntity, DimensionEntity, DimensionStyle, EllipseEntity,
    Entity, EntityBase, EntityId, HatchEntity, HatchPattern, HatchPatternType, LineEntity,
    PathEntity, PointEntity, RingGeometry, SplineEntity, TextEntity, Vec2,
};

use crate::error::{Code, KcadError};

/// The kinds, numbered as `kinds` holds them.
pub const KINDS: [&str; 13] = [
    "point",
    "line",
    "polyline",
    "polygon",
    "circle",
    "arc",
    "ellipse",
    "spline",
    "xline",
    "ray",
    "text",
    "dimension",
    "hatch",
];

const COLOR: u32 = 1;
const LABEL: u32 = 2;
const SYMBOL: u32 = 4;
/// A kind's optional fields, in the order the module's table names them.
const OPT: [u32; 4] = [1 << 8, 1 << 9, 1 << 10, 1 << 11];
const HOLE_BULGES: u32 = 1;

const DIMENSION_STYLES: [DimensionStyle; 5] = [
    DimensionStyle::Aligned,
    DimensionStyle::Linear,
    DimensionStyle::Angular,
    DimensionStyle::Radius,
    DimensionStyle::Diameter,
];
const HATCH_PATTERNS: [HatchPatternType; 3] = [
    HatchPatternType::Solid,
    HatchPatternType::Lines,
    HatchPatternType::Cross,
];

/// A drawing's objects as typed columns (see the module comment).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Columns {
    pub kinds: Vec<u8>,
    pub uids: Vec<u8>,
    pub ints: Vec<u32>,
    pub floats: Vec<f64>,
    pub text: Vec<u16>,
    pub text_lengths: Vec<u32>,
}

impl Columns {
    /// How many objects they hold.
    pub fn len(&self) -> usize {
        self.kinds.len()
    }

    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }

    fn clear(&mut self) {
        self.kinds.clear();
        self.uids.clear();
        self.ints.clear();
        self.floats.clear();
        self.text.clear();
        self.text_lengths.clear();
    }
}

fn kind_index(entity: &Entity) -> u8 {
    match entity {
        Entity::Point(_) => 0,
        Entity::Line(_) => 1,
        Entity::Polyline(_) => 2,
        Entity::Polygon(_) => 3,
        Entity::Circle(_) => 4,
        Entity::Arc(_) => 5,
        Entity::Ellipse(_) => 6,
        Entity::Spline(_) => 7,
        Entity::Xline(_) => 8,
        Entity::Ray(_) => 9,
        Entity::Text(_) => 10,
        Entity::Dimension(_) => 11,
        Entity::Hatch(_) => 12,
    }
}

/// A count as the columns hold it. No list of a drawing comes near 2³² items
/// (the file allows 2²⁴); one that did would not pass the encoder anyway.
fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

// ── Writing the columns ─────────────────────────────────────────────────

/// Appends objects to columns; one field at a time, in the module table's order.
struct Packer {
    out: Columns,
}

impl Packer {
    fn int(&mut self, v: u32) {
        self.out.ints.push(v);
    }

    fn float(&mut self, x: f64) {
        self.out.floats.push(x);
    }

    fn point(&mut self, p: &Vec2) {
        let Vec2 { x, y } = *p;
        self.out.floats.extend([x, y]);
    }

    /// A list of points: its length, then its coordinates.
    fn points(&mut self, list: &[Vec2]) {
        self.int(count(list.len()));
        self.out.floats.reserve(list.len() * 2);
        for p in list {
            self.point(p);
        }
    }

    /// A list of numbers (bulges): its length, then the numbers.
    fn numbers(&mut self, list: &[f64]) {
        self.int(count(list.len()));
        self.out.floats.extend_from_slice(list);
    }

    fn text(&mut self, s: &str) {
        let start = self.out.text.len();
        self.out.text.extend(s.encode_utf16());
        let units = self.out.text.len() - start;
        self.out.text_lengths.push(count(units));
    }

    /// One object, its layer already a table index. Out of line: see `Encoder::object`.
    #[inline(never)]
    fn object(&mut self, entity: &Entity, uid: &EntityId, layer: u32) {
        self.out.kinds.push(kind_index(entity));
        self.out.uids.extend_from_slice(&uid.0);
        // Every field is named, so a field added to the contract cannot be left out unnoticed.
        let EntityBase {
            id: _slot,
            layer_id: _,
            color,
            attrs,
            label,
            symbol,
        } = entity.base();
        let flags_at = self.out.ints.len() + 1;
        self.out.ints.extend([layer, 0, count(attrs.len())]);
        let mut flags = 0;
        for (bit, value) in [(COLOR, color), (LABEL, label), (SYMBOL, symbol)] {
            if let Some(v) = value {
                flags |= bit;
                self.text(v);
            }
        }
        for (key, value) in attrs {
            self.text(key);
            self.text(value);
        }
        flags |= self.geometry(entity);
        self.out.ints[flags_at] = flags;
    }

    /// The kind's own fields; returns their flags.
    fn geometry(&mut self, entity: &Entity) -> u32 {
        let mut flags = 0;
        match entity {
            Entity::Point(PointEntity { base: _, p, z }) => {
                self.point(p);
                if let Some(z) = z {
                    flags |= OPT[0];
                    self.float(*z);
                }
            }
            Entity::Line(LineEntity { base: _, a, b }) => {
                self.point(a);
                self.point(b);
            }
            Entity::Polyline(path) | Entity::Polygon(path) => {
                let PathEntity {
                    base: _,
                    pts,
                    bulges,
                    holes,
                } = path;
                self.points(pts);
                if let Some(b) = bulges {
                    flags |= OPT[0];
                    self.numbers(b);
                }
                if let Some(holes) = holes {
                    flags |= OPT[1];
                    self.int(count(holes.len()));
                    for RingGeometry { pts, bulges } in holes {
                        self.int(if bulges.is_some() { HOLE_BULGES } else { 0 });
                        self.points(pts);
                        if let Some(b) = bulges {
                            self.numbers(b);
                        }
                    }
                }
            }
            Entity::Circle(CircleEntity { base: _, c, r }) => {
                self.point(c);
                self.float(*r);
            }
            Entity::Arc(ArcEntity {
                base: _,
                c,
                r,
                a0,
                a1,
            }) => {
                self.point(c);
                self.out.floats.extend([*r, *a0, *a1]);
            }
            Entity::Ellipse(EllipseEntity {
                base: _,
                c,
                major,
                ratio,
                t0,
                t1,
            }) => {
                self.point(c);
                self.point(major);
                self.out.floats.extend([*ratio, *t0, *t1]);
            }
            Entity::Spline(SplineEntity {
                base: _,
                pts,
                closed,
            }) => {
                self.points(pts);
                self.int(u32::from(*closed));
            }
            Entity::Xline(line) | Entity::Ray(line) => {
                let ConstructionEntity { base: _, p, dir } = line;
                self.point(p);
                self.point(dir);
            }
            Entity::Text(TextEntity {
                base: _,
                p,
                text,
                height,
                rotation,
            }) => {
                self.point(p);
                self.out.floats.extend([*height, *rotation]);
                self.text(text);
            }
            Entity::Dimension(DimensionEntity {
                base: _,
                a,
                b,
                offset,
                height,
                text,
                style,
                angle,
                c,
            }) => {
                self.point(a);
                self.point(b);
                self.out.floats.extend([*offset, *height]);
                if let Some(t) = text {
                    flags |= OPT[0];
                    self.text(t);
                }
                if let Some(s) = style {
                    flags |= OPT[1];
                    let at = DIMENSION_STYLES.iter().position(|x| x == s).unwrap_or(0);
                    self.int(count(at));
                }
                if let Some(x) = angle {
                    flags |= OPT[2];
                    self.float(*x);
                }
                if let Some(c) = c {
                    flags |= OPT[3];
                    self.point(c);
                }
            }
            Entity::Hatch(HatchEntity {
                base: _,
                ring,
                holes,
                pattern,
            }) => {
                let HatchPattern {
                    kind,
                    angle,
                    spacing,
                } = pattern;
                self.points(ring);
                let at = HATCH_PATTERNS.iter().position(|x| x == kind).unwrap_or(0);
                self.int(count(at));
                self.out.floats.extend([*angle, *spacing]);
                if let Some(holes) = holes {
                    flags |= OPT[0];
                    self.int(count(holes.len()));
                    for hole in holes {
                        self.points(hole);
                    }
                }
            }
        }
        flags
    }
}

/// Objects as columns, taking them: each is dropped once written, so a large
/// drawing is in memory about once, not twice. The layer table lists the
/// layer ids in the order the objects first use them.
pub fn pack(entities: Vec<Entity>, uids: Vec<EntityId>) -> Columns {
    let mut table: HashMap<String, u32> = HashMap::new();
    let mut order: Vec<&str> = Vec::new();
    for e in &entities {
        let id = e.base().layer_id.as_str();
        if !table.contains_key(id) {
            table.insert(id.to_owned(), count(order.len()));
            order.push(id);
        }
    }
    let mut packer = Packer {
        out: Columns::default(),
    };
    packer.int(count(order.len()));
    for id in &order {
        packer.text(id);
    }
    packer.out.kinds.reserve(entities.len());
    packer.out.uids.reserve(entities.len() * 16);
    for (entity, uid) in entities.into_iter().zip(uids) {
        let layer = table.get(&entity.base().layer_id).copied().unwrap_or(0);
        packer.object(&entity, &uid, layer);
    }
    packer.out
}

// ── Reading the columns ─────────────────────────────────────────────────

/// Reads the streams front to back; any read past an end is a broken set of columns.
struct Cursor<'c> {
    cols: &'c Columns,
    int: usize,
    float: usize,
    unit: usize,
    text: usize,
}

fn broken(what: &str) -> KcadError {
    KcadError::new(
        Code::BadColumns,
        format!(
            "Çizim biçim modülüne bozuk aktarıldı ({what}); dosya yazılmadı, eski dosyaya dokunulmadı. Bu bir yazılım hatasıdır: sayfayı yenileyip yeniden deneyin ve durumu bildirin."
        ),
    )
}

impl<'c> Cursor<'c> {
    fn new(cols: &'c Columns) -> Self {
        Self {
            cols,
            int: 0,
            float: 0,
            unit: 0,
            text: 0,
        }
    }

    fn int(&mut self) -> Result<u32, KcadError> {
        let v = *self
            .cols
            .ints
            .get(self.int)
            .ok_or_else(|| broken("tam sayılar erken bitti"))?;
        self.int += 1;
        Ok(v)
    }

    fn usize(&mut self) -> Result<usize, KcadError> {
        Ok(self.int()? as usize)
    }

    fn float(&mut self) -> Result<f64, KcadError> {
        let v = *self
            .cols
            .floats
            .get(self.float)
            .ok_or_else(|| broken("sayılar erken bitti"))?;
        self.float += 1;
        Ok(v)
    }

    fn point(&mut self) -> Result<Vec2, KcadError> {
        Ok(Vec2 {
            x: self.float()?,
            y: self.float()?,
        })
    }

    fn points(&mut self) -> Result<Vec<Vec2>, KcadError> {
        let n = self.usize()?;
        if n.saturating_mul(2) > self.cols.floats.len() - self.float {
            return Err(broken("nokta listesi sayılardan uzun"));
        }
        (0..n).map(|_| self.point()).collect()
    }

    fn numbers(&mut self) -> Result<Vec<f64>, KcadError> {
        let n = self.usize()?;
        let end = self
            .float
            .checked_add(n)
            .filter(|&end| end <= self.cols.floats.len())
            .ok_or_else(|| broken("sayı listesi sayılardan uzun"))?;
        let out = self.cols.floats[self.float..end].to_vec();
        self.float = end;
        Ok(out)
    }

    /// The next text; `where_` names it when it is not valid Unicode (a lone surrogate).
    fn text(&mut self, where_: impl FnOnce() -> String) -> Result<String, KcadError> {
        let units = *self
            .cols
            .text_lengths
            .get(self.text)
            .ok_or_else(|| broken("metinler erken bitti"))? as usize;
        let end = self
            .unit
            .checked_add(units)
            .filter(|&end| end <= self.cols.text.len())
            .ok_or_else(|| broken("metin uzunlukları metinden uzun"))?;
        let s = String::from_utf16(&self.cols.text[self.unit..end]).map_err(|_| {
            KcadError::unwritable(
                Code::InvalidUtf8,
                &where_(),
                "metinde eşi olmayan bir UTF-16 vekili var; KCAD 2 yalnız geçerli Unicode metin yazar",
            )
        })?;
        self.unit = end;
        self.text += 1;
        Ok(s)
    }

    /// The layer table at the head of the columns.
    fn table(&mut self) -> Result<Vec<String>, KcadError> {
        let n = self.usize()?;
        if n > self.cols.text_lengths.len() {
            return Err(broken("katman tablosu metinlerden uzun"));
        }
        (0..n)
            .map(|i| self.text(|| format!("katman {}", i + 1)))
            .collect()
    }

    fn at_end(&self) -> bool {
        self.int == self.cols.ints.len()
            && self.float == self.cols.floats.len()
            && self.unit == self.cols.text.len()
            && self.text == self.cols.text_lengths.len()
    }
}

/// The objects the columns hold, as the contract has them, with their
/// persistent ids; the slots are 1, 2, 3 … as a file reader gives them.
/// Columns that do not hold together are refused (`Code::BadColumns`); a
/// text that is not valid Unicode is refused with its place.
pub fn unpack(cols: &Columns) -> Result<(Vec<Entity>, Vec<EntityId>), KcadError> {
    let n = cols.kinds.len();
    if cols.uids.len() != n * 16 {
        return Err(broken("kimlikler nesne sayısıyla uyuşmuyor"));
    }
    let mut c = Cursor::new(cols);
    let table = c.table()?;
    let mut entities = Vec::with_capacity(n);
    let mut uids = Vec::with_capacity(n);
    for (i, &k) in cols.kinds.iter().enumerate() {
        entities.push(object(&mut c, &table, i, k)?);
        let mut id = [0u8; 16];
        id.copy_from_slice(&cols.uids[i * 16..i * 16 + 16]);
        uids.push(EntityId(id));
    }
    if !c.at_end() {
        return Err(broken("nesnelerden sonra fazladan değer var"));
    }
    Ok((entities, uids))
}

/// The `i`th object, of kind `k`. Out of line: see `Encoder::object`.
#[inline(never)]
fn object(c: &mut Cursor<'_>, table: &[String], i: usize, k: u8) -> Result<Entity, KcadError> {
    let kind = *KINDS
        .get(usize::from(k))
        .ok_or_else(|| broken(&format!("{}. nesnenin türü {k}", i + 1)))?;
    let place = |field: &str| format!("entities/{i} ({kind}) › {field}");
    let layer = table
        .get(c.usize()?)
        .ok_or_else(|| broken(&format!("{}. nesnenin katmanı tabloda yok", i + 1)))?
        .clone();
    let flags = c.int()?;
    let attr_count = c.usize()?;
    if attr_count > c.cols.text_lengths.len() {
        return Err(broken("öznitelik sayısı metinlerden fazla"));
    }
    let color = (flags & COLOR != 0)
        .then(|| c.text(|| place("color")))
        .transpose()?;
    let label = (flags & LABEL != 0)
        .then(|| c.text(|| place("label")))
        .transpose()?;
    let symbol = (flags & SYMBOL != 0)
        .then(|| c.text(|| place("symbol")))
        .transpose()?;
    let mut attrs = BTreeMap::new();
    for _ in 0..attr_count {
        let key = c.text(|| place("attrs"))?;
        let value = c.text(|| place(&format!("attrs/{key}")))?;
        attrs.insert(key, value);
    }
    let base = EntityBase {
        id: count(i + 1),
        layer_id: layer,
        color,
        attrs,
        label,
        symbol,
    };
    let known = COLOR | LABEL | SYMBOL | allowed(k);
    if flags & !known != 0 {
        return Err(broken(&format!(
            "{}. nesnenin bayrakları {flags:#x}",
            i + 1
        )));
    }
    geometry(c, k, base, flags, &place)
}

/// The optional-field flags a kind may have.
fn allowed(kind: u8) -> u32 {
    match kind {
        0 => OPT[0],
        2 | 3 => OPT[0] | OPT[1],
        11 => OPT[0] | OPT[1] | OPT[2] | OPT[3],
        12 => OPT[0],
        _ => 0,
    }
}

fn geometry(
    c: &mut Cursor<'_>,
    kind: u8,
    base: EntityBase,
    flags: u32,
    place: &dyn Fn(&str) -> String,
) -> Result<Entity, KcadError> {
    let has = |bit: usize| flags & OPT[bit] != 0;
    Ok(match kind {
        0 => Entity::Point(PointEntity {
            base,
            p: c.point()?,
            z: if has(0) { Some(c.float()?) } else { None },
        }),
        1 => Entity::Line(LineEntity {
            base,
            a: c.point()?,
            b: c.point()?,
        }),
        2 | 3 => {
            let pts = c.points()?;
            let bulges = if has(0) { Some(c.numbers()?) } else { None };
            let holes = if has(1) {
                let h = c.usize()?;
                if h > c.cols.ints.len() {
                    return Err(broken("delik sayısı tam sayılardan fazla"));
                }
                let mut rings = Vec::with_capacity(h);
                for _ in 0..h {
                    let hole = c.int()?;
                    if hole & !HOLE_BULGES != 0 {
                        return Err(broken(&format!("deliğin bayrakları {hole:#x}")));
                    }
                    rings.push(RingGeometry {
                        pts: c.points()?,
                        bulges: if hole & HOLE_BULGES != 0 {
                            Some(c.numbers()?)
                        } else {
                            None
                        },
                    });
                }
                Some(rings)
            } else {
                None
            };
            let path = PathEntity {
                base,
                pts,
                bulges,
                holes,
            };
            if kind == 3 {
                Entity::Polygon(path)
            } else {
                Entity::Polyline(path)
            }
        }
        4 => Entity::Circle(CircleEntity {
            base,
            c: c.point()?,
            r: c.float()?,
        }),
        5 => Entity::Arc(ArcEntity {
            base,
            c: c.point()?,
            r: c.float()?,
            a0: c.float()?,
            a1: c.float()?,
        }),
        6 => Entity::Ellipse(EllipseEntity {
            base,
            c: c.point()?,
            major: c.point()?,
            ratio: c.float()?,
            t0: c.float()?,
            t1: c.float()?,
        }),
        7 => {
            let pts = c.points()?;
            let closed = match c.int()? {
                0 => false,
                1 => true,
                v => return Err(broken(&format!("eğrinin kapalılığı {v}"))),
            };
            Entity::Spline(SplineEntity { base, pts, closed })
        }
        8 | 9 => {
            let line = ConstructionEntity {
                base,
                p: c.point()?,
                dir: c.point()?,
            };
            if kind == 8 {
                Entity::Xline(line)
            } else {
                Entity::Ray(line)
            }
        }
        10 => Entity::Text(TextEntity {
            base,
            p: c.point()?,
            height: c.float()?,
            rotation: c.float()?,
            text: c.text(|| place("text"))?,
        }),
        11 => {
            let (a, b) = (c.point()?, c.point()?);
            let (offset, height) = (c.float()?, c.float()?);
            let text = if has(0) {
                Some(c.text(|| place("text"))?)
            } else {
                None
            };
            let style = if has(1) {
                let v = c.usize()?;
                Some(
                    *DIMENSION_STYLES
                        .get(v)
                        .ok_or_else(|| broken(&format!("ölçü türü {v}")))?,
                )
            } else {
                None
            };
            let angle = if has(2) { Some(c.float()?) } else { None };
            let corner = if has(3) { Some(c.point()?) } else { None };
            Entity::Dimension(DimensionEntity {
                base,
                a,
                b,
                offset,
                height,
                text,
                style,
                angle,
                c: corner,
            })
        }
        _ => {
            let ring = c.points()?;
            let v = c.usize()?;
            let kind = *HATCH_PATTERNS
                .get(v)
                .ok_or_else(|| broken(&format!("tarama deseni {v}")))?;
            let (angle, spacing) = (c.float()?, c.float()?);
            let holes = if has(0) {
                let h = c.usize()?;
                if h > c.cols.ints.len() {
                    return Err(broken("delik sayısı tam sayılardan fazla"));
                }
                Some((0..h).map(|_| c.points()).collect::<Result<Vec<_>, _>>()?)
            } else {
                None
            };
            Entity::Hatch(HatchEntity {
                base,
                ring,
                holes,
                pattern: HatchPattern {
                    kind,
                    angle,
                    spacing,
                },
            })
        }
    })
}

// ── Comparing ───────────────────────────────────────────────────────────

/// Whether two slices of floats are the same bit for bit (−0 is not 0).
/// Out of line: called once per object (see `Encoder::object`).
#[inline(never)]
fn same_bits(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
}

/// Where the objects `cols` hold differ from `entities` with `uids` (the
/// drawing a written file reads back to), or `None`: every field of every
/// object, every float bit for bit; the slots are not compared (a file does
/// not keep them). One object at a time, so no second copy of a large
/// drawing is made.
pub fn differs(cols: &Columns, entities: &[Entity], uids: &[EntityId]) -> Option<String> {
    if entities.len() != cols.len() || uids.len() != cols.len() {
        return Some(format!(
            "nesne sayısı: {} gönderildi, {} geri okundu",
            cols.len(),
            entities.len()
        ));
    }
    let mut c = Cursor::new(cols);
    let Ok(table) = c.table() else {
        return Some("katman tablosu".to_owned());
    };
    let index: HashMap<&str, u32> = table
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), count(i)))
        .collect();
    let mut scratch = Packer {
        out: Columns::default(),
    };
    for (i, (entity, uid)) in entities.iter().zip(uids).enumerate() {
        let Some(&layer) = index.get(entity.base().layer_id.as_str()) else {
            return Some(format!("nesne {}: katman", i + 1));
        };
        scratch.out.clear();
        scratch.object(entity, uid, layer);
        let s = &scratch.out;
        let (ints, floats) = (c.int + s.ints.len(), c.float + s.floats.len());
        let (units, texts) = (c.unit + s.text.len(), c.text + s.text_lengths.len());
        let same = cols.kinds[i] == s.kinds[0]
            && cols.uids[i * 16..i * 16 + 16] == s.uids[..]
            && cols.ints.get(c.int..ints) == Some(&s.ints[..])
            && cols
                .floats
                .get(c.float..floats)
                .is_some_and(|f| same_bits(f, &s.floats))
            && cols.text.get(c.unit..units) == Some(&s.text[..])
            && cols.text_lengths.get(c.text..texts) == Some(&s.text_lengths[..]);
        if !same {
            return Some(format!("nesne {} ({})", i + 1, entity.kind()));
        }
        (c.int, c.float, c.unit, c.text) = (ints, floats, units, texts);
    }
    (!c.at_end()).then(|| "nesnelerden sonra fazladan değer".to_owned())
}

/// The first object where two drawings' objects differ (every field, every
/// float bit for bit; not the slots), or `None`. Both sides are packed one
/// object at a time.
pub fn first_difference(
    a: &[Entity],
    a_uids: &[EntityId],
    b: &[Entity],
    b_uids: &[EntityId],
) -> Option<usize> {
    if a.len() != b.len() || a_uids.len() != a.len() || b_uids.len() != b.len() {
        return Some(a.len().min(b.len()));
    }
    let mut x = Packer {
        out: Columns::default(),
    };
    let mut y = Packer {
        out: Columns::default(),
    };
    (0..a.len()).find(|&i| {
        x.out.clear();
        y.out.clear();
        x.object(&a[i], &a_uids[i], 0);
        y.object(&b[i], &b_uids[i], 0);
        let (p, q) = (&x.out, &y.out);
        a[i].base().layer_id != b[i].base().layer_id
            || p.kinds != q.kinds
            || p.uids != q.uids
            || p.ints != q.ints
            || !same_bits(&p.floats, &q.floats)
            || p.text != q.text
            || p.text_lengths != q.text_lengths
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(layer: &str) -> EntityBase {
        EntityBase {
            id: 7,
            layer_id: layer.into(),
            color: None,
            attrs: BTreeMap::new(),
            label: None,
            symbol: None,
        }
    }

    fn id(n: u8) -> EntityId {
        let mut b = [0u8; 16];
        b[15] = n;
        b[6] = 0x70;
        EntityId(b)
    }

    #[test]
    fn a_point_is_laid_out_as_the_table_says() {
        let mut b = base("0");
        b.label = Some("P1".into());
        b.attrs.insert("Ad".into(), "Nokta".into());
        let p = Entity::Point(PointEntity {
            base: b,
            p: Vec2 { x: -0.0, y: 2.5 },
            z: Some(12.0),
        });
        let cols = pack(vec![p], vec![id(1)]);
        assert_eq!(cols.kinds, [0]);
        assert_eq!(cols.ints, [1, 0, LABEL | OPT[0], 1]);
        assert_eq!(cols.floats[0].to_bits(), (-0.0f64).to_bits());
        assert_eq!(&cols.floats[1..], [2.5, 12.0]);
        let texts: Vec<String> = {
            let mut at = 0;
            cols.text_lengths
                .iter()
                .map(|&n| {
                    let s = String::from_utf16(&cols.text[at..at + n as usize]).unwrap_or_default();
                    at += n as usize;
                    s
                })
                .collect()
        };
        assert_eq!(texts, ["0", "P1", "Ad", "Nokta"]);
    }

    #[test]
    fn unpacking_gives_back_the_objects_and_refuses_what_does_not_hold_together() {
        let entities = vec![
            Entity::Line(LineEntity {
                base: base("a"),
                a: Vec2 { x: 1.0, y: 2.0 },
                b: Vec2 { x: 3.0, y: 4.0 },
            }),
            Entity::Hatch(HatchEntity {
                base: base("b"),
                ring: vec![Vec2 { x: 0.0, y: 0.0 }; 3],
                holes: Some(vec![vec![Vec2 { x: 1.0, y: 1.0 }; 3]]),
                pattern: HatchPattern {
                    kind: HatchPatternType::Cross,
                    angle: 45.0,
                    spacing: 2.0,
                },
            }),
        ];
        let cols = pack(entities.clone(), vec![id(1), id(2)]);
        let (back, uids) = unpack(&cols).expect("unpacks");
        assert_eq!(uids, [id(1), id(2)]);
        assert_eq!(first_difference(&entities, &uids, &back, &uids), None);
        assert_eq!(differs(&cols, &back, &uids), None);
        // Slots are the reader's: 1, 2 …
        assert_eq!(back[1].base().id, 2);

        let mut short = cols.clone();
        short.floats.pop();
        assert_eq!(
            unpack(&short).map(|_| ()).unwrap_err().code,
            Code::BadColumns
        );
        let mut extra = cols.clone();
        extra.ints.push(0);
        assert_eq!(
            unpack(&extra).map(|_| ()).unwrap_err().code,
            Code::BadColumns
        );
        let mut kind = cols.clone();
        kind.kinds[0] = 13;
        assert_eq!(
            unpack(&kind).map(|_| ()).unwrap_err().code,
            Code::BadColumns
        );
        let mut lone = cols;
        lone.text[0] = 0xd800;
        let e = unpack(&lone).map(|_| ()).unwrap_err();
        assert_eq!(e.code, Code::InvalidUtf8);
    }

    #[test]
    fn a_changed_bit_or_field_is_a_difference() {
        let line = |x: f64| {
            Entity::Line(LineEntity {
                base: base("a"),
                a: Vec2 { x, y: 0.0 },
                b: Vec2 { x: 1.0, y: 1.0 },
            })
        };
        let cols = pack(vec![line(0.0)], vec![id(1)]);
        assert_eq!(differs(&cols, &[line(0.0)], &[id(1)]), None);
        assert!(differs(&cols, &[line(-0.0)], &[id(1)]).is_some());
        assert!(differs(&cols, &[line(0.0)], &[id(2)]).is_some());
        assert_eq!(
            first_difference(&[line(0.0)], &[id(1)], &[line(-0.0)], &[id(1)]),
            Some(0)
        );
        let mut labelled = line(0.0);
        if let Entity::Line(l) = &mut labelled {
            l.base.label = Some("x".into());
        }
        assert!(differs(&cols, &[labelled], &[id(1)]).is_some());
    }
}
