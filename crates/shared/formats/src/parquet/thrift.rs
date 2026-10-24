//! Thrift's compact protocol (docs/adr/0628 §1), the encoding of Parquet's metadata: field headers with id deltas,
//! zigzag varints, binaries, lists and nested structs. Reading checks every length against what is left and nesting
//! against a depth limit, so a hostile footer is an error, never a panic or an allocation past the file.

/// A value's type as the compact protocol writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    True,
    False,
    Byte,
    I16,
    I32,
    I64,
    Double,
    Binary,
    List,
    Set,
    Map,
    Struct,
    Uuid,
}

impl Kind {
    fn of(code: u8) -> Result<Kind, String> {
        Ok(match code {
            1 => Kind::True,
            2 => Kind::False,
            3 => Kind::Byte,
            4 => Kind::I16,
            5 => Kind::I32,
            6 => Kind::I64,
            7 => Kind::Double,
            8 => Kind::Binary,
            9 => Kind::List,
            10 => Kind::Set,
            11 => Kind::Map,
            12 => Kind::Struct,
            13 => Kind::Uuid,
            _ => return Err(format!("Thrift türü {code} tanınmıyor")),
        })
    }
}

const DEPTH: u32 = 64;

/// A reader over compact-protocol bytes.
pub struct In<'a> {
    b: &'a [u8],
    pub at: usize,
    depth: u32,
}

impl<'a> In<'a> {
    pub fn new(b: &'a [u8]) -> In<'a> {
        In { b, at: 0, depth: 0 }
    }

    fn byte(&mut self) -> Result<u8, String> {
        let v = *self.b.get(self.at).ok_or("Parquet üst verisi yarım")?;
        self.at += 1;
        Ok(v)
    }

    fn varint(&mut self) -> Result<u64, String> {
        let mut v = 0u64;
        for shift in (0..70).step_by(7) {
            let b = self.byte()?;
            v |= u64::from(b & 0x7f) << shift.min(63);
            if b & 0x80 == 0 {
                return Ok(v);
            }
        }
        Err("Parquet üst verisinde bozuk sayı".into())
    }

    fn zigzag(&mut self) -> Result<i64, String> {
        let v = self.varint()?;
        Ok((v >> 1) as i64 ^ -((v & 1) as i64))
    }

    /// A struct's fields: `f` is given each field's id and type and must read or skip its value.
    pub fn fields(
        &mut self,
        mut f: impl FnMut(&mut Self, i16, Kind) -> Result<(), String>,
    ) -> Result<(), String> {
        self.depth += 1;
        if self.depth > DEPTH {
            return Err("Parquet üst verisi çok derin".into());
        }
        let mut last: i16 = 0;
        loop {
            let b = self.byte()?;
            if b == 0 {
                break;
            }
            let kind = Kind::of(b & 0x0f)?;
            let delta = i16::from(b >> 4);
            let id = if delta != 0 {
                last.checked_add(delta)
                    .ok_or("Parquet üst verisinde bozuk alan")?
            } else {
                i16::try_from(self.zigzag()?).map_err(|_| "Parquet üst verisinde bozuk alan")?
            };
            last = id;
            f(self, id, kind)?;
        }
        self.depth -= 1;
        Ok(())
    }

    pub fn bool(&mut self, kind: Kind) -> Result<bool, String> {
        match kind {
            Kind::True => Ok(true),
            Kind::False => Ok(false),
            _ => self.mismatch(kind),
        }
    }

    pub fn i32(&mut self, kind: Kind) -> Result<i32, String> {
        match kind {
            Kind::Byte => Ok(i32::from(self.byte()? as i8)),
            Kind::I16 | Kind::I32 => {
                i32::try_from(self.zigzag()?).map_err(|_| "Parquet üst verisinde taşan sayı".into())
            }
            _ => self.mismatch(kind),
        }
    }

    pub fn i64(&mut self, kind: Kind) -> Result<i64, String> {
        match kind {
            Kind::Byte => Ok(i64::from(self.byte()? as i8)),
            Kind::I16 | Kind::I32 | Kind::I64 => self.zigzag(),
            _ => self.mismatch(kind),
        }
    }

    pub fn double(&mut self, kind: Kind) -> Result<f64, String> {
        if kind != Kind::Double {
            return self.mismatch(kind);
        }
        let b = self.take(8)?;
        Ok(f64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let s = self
            .b
            .get(self.at..self.at.checked_add(n).ok_or("Parquet üst verisi yarım")?)
            .ok_or("Parquet üst verisi yarım")?;
        self.at += n;
        Ok(s)
    }

    pub fn binary(&mut self, kind: Kind) -> Result<&'a [u8], String> {
        if kind != Kind::Binary {
            return self.mismatch(kind);
        }
        let n = usize::try_from(self.varint()?).map_err(|_| "Parquet üst verisi yarım")?;
        self.take(n)
    }

    pub fn string(&mut self, kind: Kind) -> Result<String, String> {
        Ok(String::from_utf8_lossy(self.binary(kind)?).into_owned())
    }

    /// A list's element type and size; each element is then read by the caller.
    pub fn list(&mut self, kind: Kind) -> Result<(Kind, usize), String> {
        if kind != Kind::List && kind != Kind::Set {
            return self.mismatch(kind);
        }
        let b = self.byte()?;
        let elem = Kind::of(b & 0x0f)?;
        let n = if b >> 4 == 15 {
            usize::try_from(self.varint()?).map_err(|_| "Parquet listesi çok uzun")?
        } else {
            usize::from(b >> 4)
        };
        // Every element takes a byte at least.
        if n > self.b.len() - self.at {
            return Err("Parquet üst verisinde liste dosyadan uzun".into());
        }
        Ok((elem, n))
    }

    /// A list element of `elem` type read as a boolean (lists write booleans as a byte: 1 true).
    pub fn elem_bool(&mut self) -> Result<bool, String> {
        Ok(self.byte()? == 1)
    }

    fn mismatch<T>(&self, kind: Kind) -> Result<T, String> {
        Err(format!("Parquet üst verisinde beklenmeyen {kind:?} türü"))
    }

    /// Passes over a value of `kind`.
    pub fn skip(&mut self, kind: Kind) -> Result<(), String> {
        match kind {
            Kind::True | Kind::False => Ok(()),
            Kind::Byte => self.byte().map(|_| ()),
            Kind::I16 | Kind::I32 | Kind::I64 => self.varint().map(|_| ()),
            Kind::Double => self.take(8).map(|_| ()),
            Kind::Uuid => self.take(16).map(|_| ()),
            Kind::Binary => self.binary(kind).map(|_| ()),
            Kind::List | Kind::Set => {
                let (elem, n) = self.list(kind)?;
                self.depth += 1;
                if self.depth > DEPTH {
                    return Err("Parquet üst verisi çok derin".into());
                }
                for _ in 0..n {
                    if matches!(elem, Kind::True | Kind::False) {
                        self.byte()?;
                    } else {
                        self.skip(elem)?;
                    }
                }
                self.depth -= 1;
                Ok(())
            }
            Kind::Map => {
                let n = usize::try_from(self.varint()?).map_err(|_| "Parquet eşlemesi çok uzun")?;
                if n == 0 {
                    return Ok(());
                }
                if n > self.b.len() - self.at {
                    return Err("Parquet üst verisinde eşleme dosyadan uzun".into());
                }
                let b = self.byte()?;
                let (k, v) = (Kind::of(b >> 4)?, Kind::of(b & 0x0f)?);
                self.depth += 1;
                if self.depth > DEPTH {
                    return Err("Parquet üst verisi çok derin".into());
                }
                for _ in 0..n {
                    for t in [k, v] {
                        if matches!(t, Kind::True | Kind::False) {
                            self.byte()?;
                        } else {
                            self.skip(t)?;
                        }
                    }
                }
                self.depth -= 1;
                Ok(())
            }
            Kind::Struct => self.fields(|r, _, k| r.skip(k)),
        }
    }
}

/// A writer of compact-protocol bytes.
#[derive(Default)]
pub struct Out {
    pub b: Vec<u8>,
    last: Vec<i16>,
}

impl Out {
    pub fn new() -> Out {
        Out {
            b: Vec::new(),
            last: vec![0],
        }
    }

    fn varint(&mut self, mut v: u64) {
        loop {
            let b = (v & 0x7f) as u8;
            v >>= 7;
            if v == 0 {
                self.b.push(b);
                return;
            }
            self.b.push(b | 0x80);
        }
    }

    fn zigzag(&mut self, v: i64) {
        self.varint(((v << 1) ^ (v >> 63)) as u64);
    }

    fn field(&mut self, id: i16, kind: u8) {
        let last = self.last.last().copied().unwrap_or(0);
        let delta = id - last;
        if (1..=15).contains(&delta) {
            self.b.push(((delta as u8) << 4) | kind);
        } else {
            self.b.push(kind);
            let wide = i64::from(id);
            self.varint(((wide << 1) ^ (wide >> 63)) as u64);
        }
        if let Some(l) = self.last.last_mut() {
            *l = id;
        }
    }

    pub fn bool(&mut self, id: i16, v: bool) {
        self.field(id, if v { 1 } else { 2 });
    }

    pub fn i32(&mut self, id: i16, v: i32) {
        self.field(id, 5);
        self.zigzag(i64::from(v));
    }

    pub fn i64(&mut self, id: i16, v: i64) {
        self.field(id, 6);
        self.zigzag(v);
    }

    pub fn field_byte(&mut self, id: i16, v: i8) {
        self.field(id, 3);
        self.b.push(v as u8);
    }

    pub fn binary(&mut self, id: i16, v: &[u8]) {
        self.field(id, 8);
        self.varint(v.len() as u64);
        self.b.extend_from_slice(v);
    }

    /// A struct field: its fields follow, then [`Out::end`].
    pub fn begin(&mut self, id: i16) {
        self.field(id, 12);
        self.last.push(0);
    }

    /// A list field of `n` elements of `kind` (5 i32, 8 binary, 12 struct).
    pub fn list(&mut self, id: i16, kind: u8, n: usize) {
        self.field(id, 9);
        if n < 15 {
            self.b.push(((n as u8) << 4) | kind);
        } else {
            self.b.push(0xf0 | kind);
            self.varint(n as u64);
        }
    }

    pub fn elem_i32(&mut self, v: i32) {
        self.zigzag(i64::from(v));
    }

    pub fn elem_binary(&mut self, v: &[u8]) {
        self.varint(v.len() as u64);
        self.b.extend_from_slice(v);
    }

    /// A struct element of a list: its fields follow, then [`Out::end`].
    pub fn elem_begin(&mut self) {
        self.last.push(0);
    }

    /// Ends the struct (or the top level).
    pub fn end(&mut self) {
        self.b.push(0);
        self.last.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn written_fields_read_back_and_unknown_ones_are_passed_over() {
        let mut o = Out::new();
        o.i32(1, -7);
        o.binary(4, b"parsel");
        o.begin(20);
        o.i64(1, 1 << 40);
        o.bool(2, true);
        o.end();
        o.list(21, 5, 20);
        for k in 0..20 {
            o.elem_i32(k * 1000);
        }
        o.i32(22, 3);
        o.end();
        let mut r = In::new(&o.b);
        let mut seen = Vec::new();
        r.fields(|r, id, kind| {
            match id {
                1 => seen.push(r.i32(kind)?.to_string()),
                4 => seen.push(r.string(kind)?),
                21 => {
                    let (_, n) = r.list(kind)?;
                    let mut sum = 0;
                    for _ in 0..n {
                        sum += r.i32(Kind::I32)?;
                    }
                    seen.push(sum.to_string());
                }
                22 => seen.push(r.i32(kind)?.to_string()),
                _ => r.skip(kind)?,
            }
            Ok(())
        })
        .unwrap();
        assert_eq!(seen, ["-7", "parsel", "190000", "3"]);
        assert_eq!(r.at, o.b.len());
        // Cut short anywhere, it is an error, never a panic.
        for n in 0..o.b.len() {
            let mut r = In::new(&o.b[..n]);
            assert!(r.fields(|r, _, k| r.skip(k)).is_err());
        }
    }
}
