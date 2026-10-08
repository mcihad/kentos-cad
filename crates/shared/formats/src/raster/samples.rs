//! A block's or a tile's samples, kept in their own type so that an 8-bit
//! orthophoto takes a byte a sample and a float DEM four, read as `f64`.

use kentos_contracts::RasterSample;

/// Samples of one or more bands, pixel by pixel, bands interleaved.
#[derive(Clone, Debug, PartialEq)]
pub enum Samples {
    U8(Vec<u8>),
    I8(Vec<i8>),
    U16(Vec<u16>),
    I16(Vec<i16>),
    U32(Vec<u32>),
    I32(Vec<i32>),
    F32(Vec<f32>),
    F64(Vec<f64>),
}

impl Samples {
    /// `n` samples of `kind`, each `fill`.
    pub fn filled(kind: RasterSample, n: usize, fill: f64) -> Samples {
        match kind {
            RasterSample::U8 => Samples::U8(vec![saturate(fill, 0.0, 255.0) as u8; n]),
            RasterSample::I8 => Samples::I8(vec![saturate(fill, -128.0, 127.0) as i8; n]),
            RasterSample::U16 => Samples::U16(vec![saturate(fill, 0.0, 65535.0) as u16; n]),
            RasterSample::I16 => Samples::I16(vec![saturate(fill, -32768.0, 32767.0) as i16; n]),
            RasterSample::U32 => Samples::U32(vec![saturate(fill, 0.0, 4_294_967_295.0) as u32; n]),
            RasterSample::I32 => Samples::I32(vec![
                saturate(fill, -2_147_483_648.0, 2_147_483_647.0)
                    as i32;
                n
            ]),
            RasterSample::F32 => Samples::F32(vec![fill as f32; n]),
            RasterSample::F64 => Samples::F64(vec![fill; n]),
        }
    }

    pub fn kind(&self) -> RasterSample {
        match self {
            Samples::U8(_) => RasterSample::U8,
            Samples::I8(_) => RasterSample::I8,
            Samples::U16(_) => RasterSample::U16,
            Samples::I16(_) => RasterSample::I16,
            Samples::U32(_) => RasterSample::U32,
            Samples::I32(_) => RasterSample::I32,
            Samples::F32(_) => RasterSample::F32,
            Samples::F64(_) => RasterSample::F64,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Samples::U8(v) => v.len(),
            Samples::I8(v) => v.len(),
            Samples::U16(v) => v.len(),
            Samples::I16(v) => v.len(),
            Samples::U32(v) => v.len(),
            Samples::I32(v) => v.len(),
            Samples::F32(v) => v.len(),
            Samples::F64(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Bytes they take.
    pub fn bytes(&self) -> usize {
        self.len() * self.kind().bytes()
    }

    /// Sample `i` as a float (NaN past the end).
    #[inline]
    pub fn get(&self, i: usize) -> f64 {
        match self {
            Samples::U8(v) => v.get(i).map_or(f64::NAN, |&x| f64::from(x)),
            Samples::I8(v) => v.get(i).map_or(f64::NAN, |&x| f64::from(x)),
            Samples::U16(v) => v.get(i).map_or(f64::NAN, |&x| f64::from(x)),
            Samples::I16(v) => v.get(i).map_or(f64::NAN, |&x| f64::from(x)),
            Samples::U32(v) => v.get(i).map_or(f64::NAN, |&x| f64::from(x)),
            Samples::I32(v) => v.get(i).map_or(f64::NAN, |&x| f64::from(x)),
            Samples::F32(v) => v.get(i).map_or(f64::NAN, |&x| f64::from(x)),
            Samples::F64(v) => v.get(i).copied().unwrap_or(f64::NAN),
        }
    }

    /// Writes sample `i` from a float: integers rounded to nearest (halves
    /// away from zero) and held within their type, floats as they are.
    #[inline]
    pub fn set(&mut self, i: usize, value: f64) {
        match self {
            Samples::U8(v) => put(v, i, round_in(value, 0.0, 255.0) as u8),
            Samples::I8(v) => put(v, i, round_in(value, -128.0, 127.0) as i8),
            Samples::U16(v) => put(v, i, round_in(value, 0.0, 65535.0) as u16),
            Samples::I16(v) => put(v, i, round_in(value, -32768.0, 32767.0) as i16),
            Samples::U32(v) => put(v, i, round_in(value, 0.0, 4_294_967_295.0) as u32),
            Samples::I32(v) => put(
                v,
                i,
                round_in(value, -2_147_483_648.0, 2_147_483_647.0) as i32,
            ),
            Samples::F32(v) => put(v, i, value as f32),
            Samples::F64(v) => put(v, i, value),
        }
    }

    /// Copies `n` samples from `src` (at `from`) to `self` (at `to`), same type.
    pub fn copy_run(&mut self, to: usize, src: &Samples, from: usize, n: usize) {
        macro_rules! run {
            ($d:expr, $s:expr) => {
                if let (Some(d), Some(s)) = ($d.get_mut(to..to + n), $s.get(from..from + n)) {
                    d.copy_from_slice(s);
                }
            };
        }
        match (self, src) {
            (Samples::U8(d), Samples::U8(s)) => run!(d, s),
            (Samples::I8(d), Samples::I8(s)) => run!(d, s),
            (Samples::U16(d), Samples::U16(s)) => run!(d, s),
            (Samples::I16(d), Samples::I16(s)) => run!(d, s),
            (Samples::U32(d), Samples::U32(s)) => run!(d, s),
            (Samples::I32(d), Samples::I32(s)) => run!(d, s),
            (Samples::F32(d), Samples::F32(s)) => run!(d, s),
            (Samples::F64(d), Samples::F64(s)) => run!(d, s),
            (d, s) => {
                for k in 0..n {
                    let v = s.get(from + k);
                    d.set(to + k, v);
                }
            }
        }
    }

    /// Copies `n` samples within, from `from` to `to` (the runs may overlap).
    pub fn copy_within(&mut self, from: usize, to: usize, n: usize) {
        macro_rules! within {
            ($v:expr) => {
                if from + n <= $v.len() && to + n <= $v.len() {
                    $v.copy_within(from..from + n, to);
                }
            };
        }
        match self {
            Samples::U8(v) => within!(v),
            Samples::I8(v) => within!(v),
            Samples::U16(v) => within!(v),
            Samples::I16(v) => within!(v),
            Samples::U32(v) => within!(v),
            Samples::I32(v) => within!(v),
            Samples::F32(v) => within!(v),
            Samples::F64(v) => within!(v),
        }
    }

    /// The samples as the file stores them: `little` endian bytes.
    pub fn to_bytes(&self, little: bool) -> Vec<u8> {
        macro_rules! out {
            ($v:expr) => {
                $v.iter()
                    .flat_map(|x| {
                        if little {
                            x.to_le_bytes().to_vec()
                        } else {
                            x.to_be_bytes().to_vec()
                        }
                    })
                    .collect()
            };
        }
        match self {
            Samples::U8(v) => v.clone(),
            Samples::I8(v) => v.iter().map(|&x| x as u8).collect(),
            Samples::U16(v) => out!(v),
            Samples::I16(v) => out!(v),
            Samples::U32(v) => out!(v),
            Samples::I32(v) => out!(v),
            Samples::F32(v) => out!(v),
            Samples::F64(v) => out!(v),
        }
    }

    /// Samples of `kind` from bytes in the file's order, `little` endian.
    pub fn from_bytes(kind: RasterSample, bytes: &[u8], little: bool) -> Samples {
        macro_rules! read {
            ($t:ty, $n:expr, $variant:ident) => {
                Samples::$variant(
                    bytes
                        .chunks_exact($n)
                        .map(|c| {
                            let mut a = [0u8; $n];
                            a.copy_from_slice(c);
                            if little {
                                <$t>::from_le_bytes(a)
                            } else {
                                <$t>::from_be_bytes(a)
                            }
                        })
                        .collect(),
                )
            };
        }
        match kind {
            RasterSample::U8 => Samples::U8(bytes.to_vec()),
            RasterSample::I8 => Samples::I8(bytes.iter().map(|&b| b as i8).collect()),
            RasterSample::U16 => read!(u16, 2, U16),
            RasterSample::I16 => read!(i16, 2, I16),
            RasterSample::U32 => read!(u32, 4, U32),
            RasterSample::I32 => read!(i32, 4, I32),
            RasterSample::F32 => read!(f32, 4, F32),
            RasterSample::F64 => read!(f64, 8, F64),
        }
    }
}

/// `value` as a sample of `kind` holds it: what `set` and then `get` give.
pub fn stored(kind: RasterSample, value: f64) -> f64 {
    match kind {
        RasterSample::U8 => round_in(value, 0.0, 255.0),
        RasterSample::I8 => round_in(value, -128.0, 127.0),
        RasterSample::U16 => round_in(value, 0.0, 65535.0),
        RasterSample::I16 => round_in(value, -32768.0, 32767.0),
        RasterSample::U32 => round_in(value, 0.0, 4_294_967_295.0),
        RasterSample::I32 => round_in(value, -2_147_483_648.0, 2_147_483_647.0),
        RasterSample::F32 => f64::from(value as f32),
        RasterSample::F64 => value,
    }
}

#[inline]
fn put<T>(v: &mut [T], i: usize, x: T) {
    if let Some(slot) = v.get_mut(i) {
        *slot = x;
    }
}

/// `value` held within `lo` and `hi` (NaN: `lo`).
fn saturate(value: f64, lo: f64, hi: f64) -> f64 {
    if value.is_nan() || value < lo {
        lo
    } else if value > hi {
        hi
    } else {
        value
    }
}

/// `value` rounded to nearest, halves away from zero, held within `lo` and `hi`.
fn round_in(value: f64, lo: f64, hi: f64) -> f64 {
    let r = if value < 0.0 {
        -((-value + 0.5).floor())
    } else {
        (value + 0.5).floor()
    };
    saturate(r, lo, hi)
}
