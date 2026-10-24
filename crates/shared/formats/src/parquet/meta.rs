//! Parquet's metadata (docs/adr/0628 §1; parquet.thrift of format 2.11): the footer's `FileMetaData` with its schema
//! elements, row groups and column chunks, and the header before every page. Read with the compact protocol, fields
//! not used passed over; written with the fields KentOS's files need.

use super::thrift::{In, Kind, Out};

/// A column's physical type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Physical {
    Boolean,
    Int32,
    Int64,
    Int96,
    Float,
    Double,
    ByteArray,
    FixedLenByteArray,
}

impl Physical {
    fn of(v: i32) -> Result<Physical, String> {
        Ok(match v {
            0 => Physical::Boolean,
            1 => Physical::Int32,
            2 => Physical::Int64,
            3 => Physical::Int96,
            4 => Physical::Float,
            5 => Physical::Double,
            6 => Physical::ByteArray,
            7 => Physical::FixedLenByteArray,
            _ => return Err(format!("Parquet fiziksel türü {v} tanınmıyor")),
        })
    }

    fn code(self) -> i32 {
        match self {
            Physical::Boolean => 0,
            Physical::Int32 => 1,
            Physical::Int64 => 2,
            Physical::Int96 => 3,
            Physical::Float => 4,
            Physical::Double => 5,
            Physical::ByteArray => 6,
            Physical::FixedLenByteArray => 7,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Physical::Boolean => "BOOLEAN",
            Physical::Int32 => "INT32",
            Physical::Int64 => "INT64",
            Physical::Int96 => "INT96",
            Physical::Float => "FLOAT",
            Physical::Double => "DOUBLE",
            Physical::ByteArray => "BYTE_ARRAY",
            Physical::FixedLenByteArray => "FIXED_LEN_BYTE_ARRAY",
        }
    }
}

/// A field's repetition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Repetition {
    Required,
    Optional,
    Repeated,
}

/// The unit of a time or a timestamp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeUnit {
    Millis,
    Micros,
    Nanos,
}

/// A logical type (`LogicalType`), or the converted type of older writers read into one.
#[derive(Clone, Debug, PartialEq)]
pub enum Logical {
    String,
    Map,
    List,
    Enum,
    Decimal {
        scale: i32,
        precision: i32,
    },
    Date,
    Time {
        utc: bool,
        unit: TimeUnit,
    },
    Timestamp {
        utc: bool,
        unit: TimeUnit,
    },
    Integer {
        bits: u8,
        signed: bool,
    },
    Unknown,
    Json,
    Bson,
    Uuid,
    Float16,
    Variant,
    /// Parquet 2.11's geometry: WKB with its system (`None`: OGC:CRS84).
    Geometry {
        crs: Option<String>,
    },
    Geography {
        crs: Option<String>,
    },
    Interval,
}

/// One element of the flattened schema (`SchemaElement`): a group when it has children, a column otherwise.
#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub name: String,
    pub physical: Option<Physical>,
    pub type_length: Option<i32>,
    pub repetition: Option<Repetition>,
    pub children: Option<i32>,
    pub logical: Option<Logical>,
}

/// A column chunk's statistics (only what KentOS writes and checks).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Statistics {
    pub null_count: Option<i64>,
    pub min_value: Option<Vec<u8>>,
    pub max_value: Option<Vec<u8>>,
}

/// `ColumnMetaData`.
#[derive(Clone, Debug, PartialEq)]
pub struct ColumnMeta {
    pub physical: Physical,
    pub encodings: Vec<i32>,
    pub path: Vec<String>,
    pub codec: i32,
    pub num_values: i64,
    pub uncompressed: i64,
    pub compressed: i64,
    pub data_page_offset: i64,
    pub dictionary_page_offset: Option<i64>,
    pub statistics: Option<Statistics>,
}

/// `RowGroup`.
#[derive(Clone, Debug, PartialEq)]
pub struct RowGroup {
    pub columns: Vec<ColumnMeta>,
    pub total_byte_size: i64,
    pub num_rows: i64,
}

/// `FileMetaData`.
#[derive(Clone, Debug, PartialEq)]
pub struct FileMeta {
    pub version: i32,
    pub schema: Vec<Element>,
    pub num_rows: i64,
    pub row_groups: Vec<RowGroup>,
    pub key_value: Vec<(String, Option<String>)>,
    pub created_by: Option<String>,
}

impl FileMeta {
    /// The value of a key of the file's metadata (`geo` for GeoParquet).
    pub fn value(&self, key: &str) -> Option<&str> {
        self.key_value
            .iter()
            .find(|(k, _)| k == key)
            .and_then(|(_, v)| v.as_deref())
    }
}

fn unit(r: &mut In<'_>) -> Result<TimeUnit, String> {
    let mut u = None;
    r.fields(|r, id, k| {
        u = match id {
            1 => Some(TimeUnit::Millis),
            2 => Some(TimeUnit::Micros),
            3 => Some(TimeUnit::Nanos),
            _ => u,
        };
        r.skip(k)
    })?;
    u.ok_or_else(|| "Parquet zaman biriminde birim yok".to_owned())
}

fn time_like(r: &mut In<'_>) -> Result<(bool, TimeUnit), String> {
    let (mut utc, mut u) = (false, None);
    r.fields(|r, id, k| {
        match (id, k) {
            (1, _) => utc = r.bool(k)?,
            (2, Kind::Struct) => u = Some(unit(r)?),
            _ => r.skip(k)?,
        }
        Ok(())
    })?;
    Ok((utc, u.ok_or("Parquet zaman türünde birim yok")?))
}

fn crs_of(r: &mut In<'_>) -> Result<Option<String>, String> {
    let mut crs = None;
    r.fields(|r, id, k| {
        match id {
            1 => crs = Some(r.string(k)?),
            _ => r.skip(k)?,
        }
        Ok(())
    })?;
    Ok(crs)
}

fn decimal(r: &mut In<'_>) -> Result<Logical, String> {
    let (mut scale, mut precision) = (0, 0);
    r.fields(|r, id, k| {
        match id {
            1 => scale = r.i32(k)?,
            2 => precision = r.i32(k)?,
            _ => r.skip(k)?,
        }
        Ok(())
    })?;
    Ok(Logical::Decimal { scale, precision })
}

fn integer(r: &mut In<'_>) -> Result<Logical, String> {
    let (mut bits, mut signed) = (64u8, true);
    r.fields(|r, id, k| {
        match id {
            1 => bits = u8::try_from(r.i32(k)?).map_err(|_| "Parquet tam sayı genişliği bozuk")?,
            2 => signed = r.bool(k)?,
            _ => r.skip(k)?,
        }
        Ok(())
    })?;
    Ok(Logical::Integer { bits, signed })
}

fn logical(r: &mut In<'_>) -> Result<Option<Logical>, String> {
    let mut out = None;
    r.fields(|r, id, k| {
        if k != Kind::Struct {
            return r.skip(k);
        }
        let l = match id {
            5 => decimal(r)?,
            7 => {
                let (utc, unit) = time_like(r)?;
                Logical::Time { utc, unit }
            }
            8 => {
                let (utc, unit) = time_like(r)?;
                Logical::Timestamp { utc, unit }
            }
            10 => integer(r)?,
            17 => Logical::Geometry { crs: crs_of(r)? },
            18 => Logical::Geography { crs: crs_of(r)? },
            _ => {
                r.skip(k)?;
                match id {
                    1 => Logical::String,
                    2 => Logical::Map,
                    3 => Logical::List,
                    4 => Logical::Enum,
                    6 => Logical::Date,
                    11 => Logical::Unknown,
                    12 => Logical::Json,
                    13 => Logical::Bson,
                    14 => Logical::Uuid,
                    15 => Logical::Float16,
                    16 => Logical::Variant,
                    _ => return Ok(()),
                }
            }
        };
        out = Some(l);
        Ok(())
    })?;
    Ok(out)
}

/// An older writer's converted type as a logical type.
fn converted(code: i32, scale: i32, precision: i32) -> Option<Logical> {
    Some(match code {
        0 => Logical::String,
        1 | 2 => Logical::Map,
        3 => Logical::List,
        4 => Logical::Enum,
        5 => Logical::Decimal { scale, precision },
        6 => Logical::Date,
        7 => Logical::Time {
            utc: true,
            unit: TimeUnit::Millis,
        },
        8 => Logical::Time {
            utc: true,
            unit: TimeUnit::Micros,
        },
        9 => Logical::Timestamp {
            utc: true,
            unit: TimeUnit::Millis,
        },
        10 => Logical::Timestamp {
            utc: true,
            unit: TimeUnit::Micros,
        },
        11 => Logical::Integer {
            bits: 8,
            signed: false,
        },
        12 => Logical::Integer {
            bits: 16,
            signed: false,
        },
        13 => Logical::Integer {
            bits: 32,
            signed: false,
        },
        14 => Logical::Integer {
            bits: 64,
            signed: false,
        },
        15 => Logical::Integer {
            bits: 8,
            signed: true,
        },
        16 => Logical::Integer {
            bits: 16,
            signed: true,
        },
        17 => Logical::Integer {
            bits: 32,
            signed: true,
        },
        18 => Logical::Integer {
            bits: 64,
            signed: true,
        },
        19 => Logical::Json,
        20 => Logical::Bson,
        21 => Logical::Interval,
        _ => return None,
    })
}

fn element(r: &mut In<'_>) -> Result<Element, String> {
    let mut e = Element {
        name: String::new(),
        physical: None,
        type_length: None,
        repetition: None,
        children: None,
        logical: None,
    };
    let (mut conv, mut scale, mut precision) = (None, 0, 0);
    r.fields(|r, id, k| {
        match id {
            1 => e.physical = Some(Physical::of(r.i32(k)?)?),
            2 => e.type_length = Some(r.i32(k)?),
            3 => {
                e.repetition = Some(match r.i32(k)? {
                    0 => Repetition::Required,
                    1 => Repetition::Optional,
                    2 => Repetition::Repeated,
                    v => return Err(format!("Parquet tekrar türü {v} tanınmıyor")),
                })
            }
            4 => e.name = r.string(k)?,
            5 => e.children = Some(r.i32(k)?),
            6 => conv = Some(r.i32(k)?),
            7 => scale = r.i32(k)?,
            8 => precision = r.i32(k)?,
            10 if k == Kind::Struct => e.logical = logical(r)?,
            _ => r.skip(k)?,
        }
        Ok(())
    })?;
    if e.logical.is_none()
        && let Some(c) = conv
    {
        e.logical = converted(c, scale, precision);
    }
    Ok(e)
}

fn statistics(r: &mut In<'_>) -> Result<Statistics, String> {
    let mut s = Statistics::default();
    let (mut old_min, mut old_max) = (None, None);
    r.fields(|r, id, k| {
        match id {
            1 => old_max = Some(r.binary(k)?.to_vec()),
            2 => old_min = Some(r.binary(k)?.to_vec()),
            3 => s.null_count = Some(r.i64(k)?),
            5 => s.max_value = Some(r.binary(k)?.to_vec()),
            6 => s.min_value = Some(r.binary(k)?.to_vec()),
            _ => r.skip(k)?,
        }
        Ok(())
    })?;
    s.min_value = s.min_value.or(old_min);
    s.max_value = s.max_value.or(old_max);
    Ok(s)
}

fn column_meta(r: &mut In<'_>) -> Result<ColumnMeta, String> {
    let mut m = ColumnMeta {
        physical: Physical::Boolean,
        encodings: Vec::new(),
        path: Vec::new(),
        codec: 0,
        num_values: 0,
        uncompressed: 0,
        compressed: 0,
        data_page_offset: 0,
        dictionary_page_offset: None,
        statistics: None,
    };
    let mut typed = false;
    r.fields(|r, id, k| {
        match id {
            1 => {
                m.physical = Physical::of(r.i32(k)?)?;
                typed = true;
            }
            2 => {
                let (_, n) = r.list(k)?;
                for _ in 0..n {
                    m.encodings.push(r.i32(Kind::I32)?);
                }
            }
            3 => {
                let (_, n) = r.list(k)?;
                for _ in 0..n {
                    m.path.push(r.string(Kind::Binary)?);
                }
            }
            4 => m.codec = r.i32(k)?,
            5 => m.num_values = r.i64(k)?,
            6 => m.uncompressed = r.i64(k)?,
            7 => m.compressed = r.i64(k)?,
            9 => m.data_page_offset = r.i64(k)?,
            11 => m.dictionary_page_offset = Some(r.i64(k)?),
            12 if k == Kind::Struct => m.statistics = Some(statistics(r)?),
            _ => r.skip(k)?,
        }
        Ok(())
    })?;
    if !typed {
        return Err("Parquet sütun parçasının türü yok".into());
    }
    Ok(m)
}

fn row_group(r: &mut In<'_>) -> Result<RowGroup, String> {
    let mut g = RowGroup {
        columns: Vec::new(),
        total_byte_size: 0,
        num_rows: 0,
    };
    r.fields(|r, id, k| {
        match id {
            1 => {
                let (_, n) = r.list(k)?;
                for _ in 0..n {
                    let mut meta = None;
                    let mut external = None;
                    r.fields(|r, id, k| {
                        match id {
                            1 => external = Some(r.string(k)?),
                            3 if k == Kind::Struct => meta = Some(column_meta(r)?),
                            8 | 9 => return Err("şifreli Parquet sütunları açılmaz".into()),
                            _ => r.skip(k)?,
                        }
                        Ok(())
                    })?;
                    if let Some(f) = external {
                        return Err(format!(
                            "sütun başka bir dosyada ({f}); yalnız tek dosyalık Parquet okunur"
                        ));
                    }
                    g.columns
                        .push(meta.ok_or("Parquet sütun parçasının üst verisi yok")?);
                }
            }
            2 => g.total_byte_size = r.i64(k)?,
            3 => g.num_rows = r.i64(k)?,
            _ => r.skip(k)?,
        }
        Ok(())
    })?;
    Ok(g)
}

/// Reads the footer's `FileMetaData`.
pub fn read_file_meta(bytes: &[u8]) -> Result<FileMeta, String> {
    let mut r = In::new(bytes);
    let mut m = FileMeta {
        version: 0,
        schema: Vec::new(),
        num_rows: 0,
        row_groups: Vec::new(),
        key_value: Vec::new(),
        created_by: None,
    };
    r.fields(|r, id, k| {
        match id {
            1 => m.version = r.i32(k)?,
            2 => {
                let (_, n) = r.list(k)?;
                for _ in 0..n {
                    m.schema.push(element(r)?);
                }
            }
            3 => m.num_rows = r.i64(k)?,
            4 => {
                let (_, n) = r.list(k)?;
                for _ in 0..n {
                    m.row_groups.push(row_group(r)?);
                }
            }
            5 => {
                let (_, n) = r.list(k)?;
                for _ in 0..n {
                    let (mut key, mut value) = (String::new(), None);
                    r.fields(|r, id, k| {
                        match id {
                            1 => key = r.string(k)?,
                            2 => value = Some(r.string(k)?),
                            _ => r.skip(k)?,
                        }
                        Ok(())
                    })?;
                    m.key_value.push((key, value));
                }
            }
            6 => m.created_by = Some(r.string(k)?),
            8 => return Err("şifreli Parquet dosyası açılmaz".into()),
            _ => r.skip(k)?,
        }
        Ok(())
    })?;
    if m.schema.is_empty() {
        return Err("Parquet şeması boş".into());
    }
    Ok(m)
}

/// A page's kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageKind {
    Data,
    Index,
    Dictionary,
    DataV2,
}

/// `PageHeader` with the parts of its kind's header KentOS uses.
#[derive(Clone, Debug, PartialEq)]
pub struct PageHeader {
    pub kind: PageKind,
    pub uncompressed: i32,
    pub compressed: i32,
    pub num_values: i32,
    pub encoding: i32,
    pub def_encoding: i32,
    pub rep_encoding: i32,
    /// v2: the levels' lengths, not compressed, before the values.
    pub def_length: i32,
    pub rep_length: i32,
    /// v2: whether the values are compressed (true when not said).
    pub values_compressed: bool,
    pub num_nulls: i32,
    pub num_rows: i32,
}

/// Reads a page header; the number of bytes it took.
pub fn read_page_header(bytes: &[u8]) -> Result<(PageHeader, usize), String> {
    let mut r = In::new(bytes);
    let mut h = PageHeader {
        kind: PageKind::Index,
        uncompressed: 0,
        compressed: 0,
        num_values: 0,
        encoding: 0,
        def_encoding: 3,
        rep_encoding: 3,
        def_length: 0,
        rep_length: 0,
        values_compressed: true,
        num_nulls: 0,
        num_rows: 0,
    };
    let mut kind = None;
    r.fields(|r, id, k| {
        match id {
            1 => {
                kind = Some(match r.i32(k)? {
                    0 => PageKind::Data,
                    1 => PageKind::Index,
                    2 => PageKind::Dictionary,
                    3 => PageKind::DataV2,
                    v => return Err(format!("Parquet sayfa türü {v} tanınmıyor")),
                })
            }
            2 => h.uncompressed = r.i32(k)?,
            3 => h.compressed = r.i32(k)?,
            5 if k == Kind::Struct => r.fields(|r, id, k| {
                match id {
                    1 => h.num_values = r.i32(k)?,
                    2 => h.encoding = r.i32(k)?,
                    3 => h.def_encoding = r.i32(k)?,
                    4 => h.rep_encoding = r.i32(k)?,
                    _ => r.skip(k)?,
                }
                Ok(())
            })?,
            7 if k == Kind::Struct => r.fields(|r, id, k| {
                match id {
                    1 => h.num_values = r.i32(k)?,
                    2 => h.encoding = r.i32(k)?,
                    _ => r.skip(k)?,
                }
                Ok(())
            })?,
            8 if k == Kind::Struct => r.fields(|r, id, k| {
                match id {
                    1 => h.num_values = r.i32(k)?,
                    2 => h.num_nulls = r.i32(k)?,
                    3 => h.num_rows = r.i32(k)?,
                    4 => h.encoding = r.i32(k)?,
                    5 => h.def_length = r.i32(k)?,
                    6 => h.rep_length = r.i32(k)?,
                    7 => h.values_compressed = r.bool(k)?,
                    _ => r.skip(k)?,
                }
                Ok(())
            })?,
            _ => r.skip(k)?,
        }
        Ok(())
    })?;
    h.kind = kind.ok_or("Parquet sayfa başlığında tür yok")?;
    if h.uncompressed < 0
        || h.compressed < 0
        || h.num_values < 0
        || h.def_length < 0
        || h.rep_length < 0
    {
        return Err("Parquet sayfa başlığında eksi boy".into());
    }
    Ok((h, r.at))
}

/// Writes a data page (v1) header: PLAIN values, RLE levels.
pub fn write_data_page_header(
    out: &mut Vec<u8>,
    uncompressed: usize,
    compressed: usize,
    values: usize,
) {
    let mut o = Out::new();
    o.i32(1, 0);
    o.i32(2, uncompressed as i32);
    o.i32(3, compressed as i32);
    o.begin(5);
    o.i32(1, values as i32);
    o.i32(2, 0);
    o.i32(3, 3);
    o.i32(4, 3);
    o.end();
    o.end();
    out.extend_from_slice(&o.b);
}

fn write_logical(o: &mut Out, l: &Logical) {
    o.begin(10);
    let empty = |o: &mut Out, id: i16| {
        o.begin(id);
        o.end();
    };
    let time_unit = |o: &mut Out, u: TimeUnit| {
        o.begin(2);
        let id = match u {
            TimeUnit::Millis => 1,
            TimeUnit::Micros => 2,
            TimeUnit::Nanos => 3,
        };
        o.begin(id);
        o.end();
        o.end();
    };
    match l {
        Logical::String => empty(o, 1),
        Logical::Date => empty(o, 6),
        Logical::Json => empty(o, 12),
        Logical::Uuid => empty(o, 14),
        Logical::Time { utc, unit } => {
            o.begin(7);
            o.bool(1, *utc);
            time_unit(o, *unit);
            o.end();
        }
        Logical::Timestamp { utc, unit } => {
            o.begin(8);
            o.bool(1, *utc);
            time_unit(o, *unit);
            o.end();
        }
        Logical::Integer { bits, signed } => {
            o.begin(10);
            o.field_byte(1, *bits as i8);
            o.bool(2, *signed);
            o.end();
        }
        _ => {}
    }
    o.end();
}

/// The converted type older readers want beside a logical type.
fn converted_of(l: &Logical) -> Option<i32> {
    Some(match l {
        Logical::String => 0,
        Logical::Date => 6,
        Logical::Json => 19,
        Logical::Time {
            unit: TimeUnit::Millis,
            ..
        } => 7,
        Logical::Time {
            unit: TimeUnit::Micros,
            ..
        } => 8,
        Logical::Timestamp {
            unit: TimeUnit::Millis,
            utc: true,
        } => 9,
        Logical::Timestamp {
            unit: TimeUnit::Micros,
            utc: true,
        } => 10,
        Logical::Integer {
            bits: 64,
            signed: true,
        } => 18,
        _ => return None,
    })
}

/// Writes the footer's `FileMetaData`.
pub fn write_file_meta(m: &FileMeta) -> Vec<u8> {
    let mut o = Out::new();
    o.i32(1, m.version);
    o.list(2, 12, m.schema.len());
    for e in &m.schema {
        o.elem_begin();
        if let Some(p) = e.physical {
            o.i32(1, p.code());
        }
        if let Some(n) = e.type_length {
            o.i32(2, n);
        }
        if let Some(r) = e.repetition {
            o.i32(
                3,
                match r {
                    Repetition::Required => 0,
                    Repetition::Optional => 1,
                    Repetition::Repeated => 2,
                },
            );
        }
        o.binary(4, e.name.as_bytes());
        if let Some(n) = e.children {
            o.i32(5, n);
        }
        if let Some(l) = &e.logical {
            if let Some(c) = converted_of(l) {
                o.i32(6, c);
            }
            write_logical(&mut o, l);
        }
        o.end();
    }
    o.i64(3, m.num_rows);
    o.list(4, 12, m.row_groups.len());
    for g in &m.row_groups {
        o.elem_begin();
        o.list(1, 12, g.columns.len());
        for c in &g.columns {
            o.elem_begin();
            o.i64(2, c.data_page_offset);
            o.begin(3);
            o.i32(1, c.physical.code());
            o.list(2, 5, c.encodings.len());
            for e in &c.encodings {
                o.elem_i32(*e);
            }
            o.list(3, 8, c.path.len());
            for p in &c.path {
                o.elem_binary(p.as_bytes());
            }
            o.i32(4, c.codec);
            o.i64(5, c.num_values);
            o.i64(6, c.uncompressed);
            o.i64(7, c.compressed);
            o.i64(9, c.data_page_offset);
            if let Some(s) = &c.statistics {
                o.begin(12);
                if let Some(n) = s.null_count {
                    o.i64(3, n);
                }
                if let Some(v) = &s.max_value {
                    o.binary(5, v);
                }
                if let Some(v) = &s.min_value {
                    o.binary(6, v);
                }
                o.end();
            }
            o.end();
            o.end();
        }
        o.i64(2, g.total_byte_size);
        o.i64(3, g.num_rows);
        o.end();
    }
    if !m.key_value.is_empty() {
        o.list(5, 12, m.key_value.len());
        for (k, v) in &m.key_value {
            o.elem_begin();
            o.binary(1, k.as_bytes());
            if let Some(v) = v {
                o.binary(2, v.as_bytes());
            }
            o.end();
        }
    }
    if let Some(c) = &m.created_by {
        o.binary(6, c.as_bytes());
    }
    // Every column ordered by its type (min_value and max_value mean what they say).
    let columns = m.schema.iter().filter(|e| e.children.is_none()).count();
    o.list(7, 12, columns);
    for _ in 0..columns {
        o.elem_begin();
        o.begin(1);
        o.end();
        o.end();
    }
    o.end();
    o.b
}
