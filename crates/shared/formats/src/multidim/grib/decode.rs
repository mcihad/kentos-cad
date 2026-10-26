//! A GRIB message's field decoded (docs/adr/0645 §4): GRIB1's simple packing (IBM reference value), GRIB2's simple
//! (5.0), complex (5.2) and complex with spatial differencing (5.3), PNG (5.41) and CCSDS (5.42) packing, with its
//! bitmap; the values laid row by row in the file's row order with the columns west to east (the scanning mode's
//! column direction and transposition undone).

use super::{Grid, Message, sec_int, sec_u};

/// Bits read MSB first.
struct Bits<'a> {
    b: &'a [u8],
    at: usize,
}

impl Bits<'_> {
    fn take(&mut self, n: u32) -> Result<u64, String> {
        if n == 0 {
            return Ok(0);
        }
        let mut v = 0u64;
        for _ in 0..n {
            let byte = *self
                .b
                .get(self.at / 8)
                .ok_or("GRIB veri bölümü erken bitti")?;
            v = (v << 1) | u64::from((byte >> (7 - self.at % 8)) & 1);
            self.at += 1;
        }
        Ok(v)
    }

    fn align(&mut self) {
        self.at = self.at.div_ceil(8) * 8;
    }
}

/// IBM single precision (GRIB1's reference value).
/// `base` to an integer power as ecCodes takes it (`codes_power`): multiplied, or divided for a negative power, one step
/// at a time, so a decimal scale rounds as theirs does.
pub(crate) fn power(base: f64, e: i32) -> f64 {
    let mut r = 1.0;
    for _ in 0..e.unsigned_abs() {
        if e < 0 { r /= base } else { r *= base }
    }
    r
}

fn ibm(b: [u8; 4]) -> f64 {
    let sign = if b[0] & 0x80 != 0 { -1.0 } else { 1.0 };
    let exp = i32::from(b[0] & 0x7F) - 64;
    let mant = f64::from(u32::from_be_bytes([0, b[1], b[2], b[3]])) / 16_777_216.0;
    sign * mant * power(16.0, exp)
}

/// The values of field `field` of a message (its bytes from the start).
pub fn field(bytes: &[u8], which: u32) -> Result<Vec<f64>, String> {
    let msg = super::read_message(bytes)?;
    let f = msg
        .fields
        .get(which as usize)
        .ok_or("GRIB iletisinde bu alan yok")?;
    let values = match msg.edition {
        1 => grib1(bytes, f)?,
        _ => grib2(bytes, f)?,
    };
    Ok(normalise(&f.grid, values))
}

/// The field's values in its scanning order made rows (file's row order) of columns west to east.
fn normalise(g: &Grid, v: Vec<f64>) -> Vec<f64> {
    let (ni, nj) = (g.ni as usize, g.nj as usize);
    if v.len() != ni * nj {
        return v;
    }
    let mut out = vec![f64::NAN; ni * nj];
    for j in 0..nj {
        for i in 0..ni {
            let src = if g.scan & 0x20 != 0 {
                i * nj + j
            } else {
                j * ni + i
            };
            let col = if g.scan & 0x80 != 0 { ni - 1 - i } else { i };
            out[j * ni + col] = v[src];
        }
    }
    out
}

/// The bitmap's bits (none: every point has a value).
fn place_bitmap(n: usize, bitmap: Option<&[u8]>, packed: Vec<f64>) -> Result<Vec<f64>, String> {
    let Some(bm) = bitmap else {
        let mut v = packed;
        v.resize(n, f64::NAN);
        return Ok(v);
    };
    let mut out = Vec::with_capacity(n);
    let mut it = packed.into_iter();
    for i in 0..n {
        let on = bm.get(i / 8).is_some_and(|b| b >> (7 - i % 8) & 1 == 1);
        out.push(if on {
            it.next().unwrap_or(f64::NAN)
        } else {
            f64::NAN
        });
    }
    Ok(out)
}

fn grib1(bytes: &[u8], f: &Message) -> Result<Vec<f64>, String> {
    let n = (f.grid.ni * f.grid.nj) as usize;
    let bds = bytes.get(f.data_at..).ok_or("GRIB1 veri bölümü yok")?;
    let len = sec_u(bds, 0, 3) as usize;
    let flags = *bds.get(3).ok_or("GRIB1 veri bölümü eksik")?;
    if flags & 0xC0 != 0 {
        return Err("GRIB1'in küresel harmonik ya da karmaşık paketlenmiş alanı okunmuyor".into());
    }
    let e = sec_int(bds, 4, 2) as i32;
    let r = ibm([bds[6], bds[7], bds[8], bds[9]]);
    let nbits = u32::from(*bds.get(10).ok_or("GRIB1 veri bölümü eksik")?);
    let d = f.decimal;
    let count = match &f.bitmap {
        Some(bm) => count_bits(bm, n),
        None => n,
    };
    let data = bds.get(11..len.min(bds.len())).unwrap_or(&[]);
    let mut b = Bits { b: data, at: 0 };
    let scale = power(2.0, e);
    let ten = power(10.0, -d);
    let mut packed = Vec::with_capacity(count);
    for _ in 0..count {
        let x = if nbits == 0 { 0 } else { b.take(nbits)? };
        packed.push((r + x as f64 * scale) * ten);
    }
    place_bitmap(n, f.bitmap.as_deref(), packed)
}

fn count_bits(bm: &[u8], n: usize) -> usize {
    (0..n)
        .filter(|&i| bm.get(i / 8).is_some_and(|b| b >> (7 - i % 8) & 1 == 1))
        .count()
}

fn grib2(bytes: &[u8], f: &Message) -> Result<Vec<f64>, String> {
    let n = (f.grid.ni * f.grid.nj) as usize;
    let s5 = bytes.get(f.repr_at..).ok_or("GRIB2 5. bölümü yok")?;
    let count = sec_u(s5, 5, 4) as usize;
    let template = sec_u(s5, 9, 2);
    let r = f32::from_bits(sec_u(s5, 11, 4) as u32) as f64;
    let e = sec_int(s5, 15, 2) as i32;
    let d = sec_int(s5, 17, 2) as i32;
    let nbits = u32::from(*s5.get(19).ok_or("GRIB2 5. bölümü eksik")?);
    let s7 = bytes.get(f.data_at..).ok_or("GRIB2 7. bölümü yok")?;
    let len7 = sec_u(s7, 0, 4) as usize;
    let data = s7.get(5..len7.min(s7.len())).unwrap_or(&[]);
    let scale = power(2.0, e);
    let ten = power(10.0, -d);
    let value = |x: f64| (r + x * scale) * ten;
    let packed: Vec<f64> = match template {
        0 => {
            let mut b = Bits { b: data, at: 0 };
            let mut out = Vec::with_capacity(count);
            for _ in 0..count {
                out.push(value(b.take(nbits)? as f64));
            }
            out
        }
        2 | 3 => complex(s5, data, count, nbits, template == 3)?
            .into_iter()
            .map(|x| x.map_or(f64::NAN, |x| value(x as f64)))
            .collect(),
        41 => {
            let png = crate::raster::png::decode(data).map_err(|e| e.0)?;
            (0..count.min(png.samples.len()))
                .map(|i| value(png.samples.get(i)))
                .collect()
        }
        42 => {
            let flags = u32::from(*s5.get(21).ok_or("GRIB2 5. bölümü eksik")?);
            let block = usize::from(*s5.get(22).ok_or("GRIB2 5. bölümü eksik")?);
            let rsi = sec_u(s5, 23, 2) as usize;
            let xs = if nbits == 0 {
                vec![0; count]
            } else {
                super::ccsds::decode(data, count, nbits, flags, block, rsi)?
            };
            xs.into_iter().map(|x| value(x as f64)).collect()
        }
        40 => {
            return Err("JPEG 2000 ile paketlenmiş GRIB2 alanı okunmuyor; `grib_set -r -s packingType=grid_simple girdi.grib2 çıktı.grib2` ile çevirin".into());
        }
        t => return Err(format!("GRIB2'nin 5.{t} veri gösterimi okunmuyor")),
    };
    place_bitmap(n, f.bitmap.as_deref(), packed)
}

/// Complex packing's values (none for a missing one), spatial differencing undone (5.3).
fn complex(
    s5: &[u8],
    data: &[u8],
    count: usize,
    nbits: u32,
    diff: bool,
) -> Result<Vec<Option<i64>>, String> {
    let at = |o: usize| *s5.get(o).unwrap_or(&0);
    let missing_mgmt = at(22);
    let ng = sec_u(s5, 31, 4) as usize;
    let ref_width = u64::from(at(35));
    let width_bits = u32::from(at(36));
    let ref_len = sec_u(s5, 37, 4);
    let len_inc = u64::from(at(41));
    let last_len = sec_u(s5, 42, 4);
    let len_bits = u32::from(at(46));
    let (order, extra) = if diff {
        (usize::from(at(47)), usize::from(at(48)))
    } else {
        (0, 0)
    };
    if ng > count.max(1) * 2 || nbits > 63 {
        return Err("GRIB2 karmaşık paketlemesi bozuk".into());
    }
    let mut b = Bits { b: data, at: 0 };
    let mut firsts = Vec::with_capacity(order);
    let mut min = 0i64;
    if diff && order > 0 {
        let signed = |v: u64, n: usize| -> i64 {
            let top = 1u64 << (8 * n - 1);
            if v & top != 0 {
                -((v & (top - 1)) as i64)
            } else {
                v as i64
            }
        };
        for _ in 0..order {
            firsts.push(signed(b.take(8 * extra as u32)?, extra));
        }
        min = signed(b.take(8 * extra as u32)?, extra);
    }
    let mut refs = Vec::with_capacity(ng);
    for _ in 0..ng {
        refs.push(b.take(nbits)?);
    }
    b.align();
    let mut widths = Vec::with_capacity(ng);
    for _ in 0..ng {
        widths.push(ref_width + b.take(width_bits)?);
    }
    b.align();
    let mut lens = Vec::with_capacity(ng);
    for g in 0..ng {
        let l = ref_len + len_inc * b.take(len_bits)?;
        lens.push(if g + 1 == ng { last_len } else { l });
    }
    b.align();
    let mut out: Vec<Option<i64>> = Vec::with_capacity(count);
    for g in 0..ng {
        let w = widths[g] as u32;
        if w > 63 {
            return Err("GRIB2 karmaşık paketlemesinde grup genişliği bozuk".into());
        }
        let all_ones = |v: u64, n: u32| n > 0 && v == (1u64 << n) - 1;
        for _ in 0..lens[g] {
            if out.len() >= count {
                break;
            }
            if w == 0 {
                let missing = missing_mgmt > 0 && all_ones(refs[g], nbits);
                out.push((!missing).then_some(refs[g] as i64));
            } else {
                let x = b.take(w)?;
                let missing = missing_mgmt > 0
                    && (all_ones(x, w) || (missing_mgmt == 2 && x == (1u64 << w) - 2));
                out.push((!missing).then_some((refs[g] + x) as i64));
            }
        }
    }
    out.resize(count, None);
    if diff && order > 0 {
        let mut k = 0usize;
        let mut prev: Vec<i64> = Vec::with_capacity(2);
        for v in out.iter_mut() {
            let Some(x) = v else { continue };
            let y = if k < order {
                firsts[k]
            } else if order == 1 {
                *x + min + prev[prev.len() - 1]
            } else {
                *x + min + 2 * prev[prev.len() - 1] - prev[prev.len() - 2]
            };
            *v = Some(y);
            prev.push(y);
            if prev.len() > 2 {
                prev.remove(0);
            }
            k += 1;
        }
    }
    Ok(out)
}
