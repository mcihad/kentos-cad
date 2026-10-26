//! CCSDS 121.0-B lossless decoding (Adaptive Entropy Coding, libaec's stream) as GRIB2's data representation 5.42
//! keeps a field (docs/adr/0645 §4): blocks of `block` samples, each with its option's id (zero blocks, second
//! extension, split samples of `k` low bits, or uncompressed); with preprocessing a reference sample opens every
//! reference interval and the rest are mapped differences from the sample before.

const ROS: u64 = 5;

struct Bits<'a> {
    b: &'a [u8],
    at: usize,
}

impl Bits<'_> {
    fn bit(&mut self) -> Result<u64, String> {
        let byte = *self.b.get(self.at / 8).ok_or("CCSDS akışı erken bitti")?;
        let v = (byte >> (7 - self.at % 8)) & 1;
        self.at += 1;
        Ok(u64::from(v))
    }

    fn take(&mut self, n: u32) -> Result<u64, String> {
        let mut v = 0u64;
        for _ in 0..n {
            v = (v << 1) | self.bit()?;
        }
        Ok(v)
    }

    /// A fundamental sequence code: the zeros before a one.
    fn fs(&mut self) -> Result<u64, String> {
        let mut n = 0u64;
        while self.bit()? == 0 {
            n += 1;
            if n > 1 << 20 {
                return Err("CCSDS akışı bozuk".into());
            }
        }
        Ok(n)
    }

    fn align(&mut self) {
        self.at = self.at.div_ceil(8) * 8;
    }
}

/// The second extension's table: each code's sum and the first value of its sum's row.
fn se_table() -> Vec<(u64, u64)> {
    let mut t = Vec::with_capacity(91);
    let mut k = 0u64;
    for i in 0..13u64 {
        let ms = k;
        for _ in 0..=i {
            t.push((i, ms));
            k += 1;
        }
    }
    t
}

/// `count` samples of `bits` bits decoded; `flags` libaec's (1 signed, 8 preprocessed, 16 restricted, 32 each
/// reference interval padded to a byte), `block` samples a block, `rsi` blocks a reference interval.
pub fn decode(
    data: &[u8],
    count: usize,
    bits: u32,
    flags: u32,
    block: usize,
    rsi: usize,
) -> Result<Vec<u64>, String> {
    if !(1..=32).contains(&bits) || !matches!(block, 8 | 16 | 32 | 64) || rsi == 0 {
        return Err("CCSDS parametreleri okunmuyor".into());
    }
    if flags & 1 != 0 {
        return Err("işaretli CCSDS örnekleri okunmuyor".into());
    }
    let pp = flags & 8 != 0;
    let restricted = flags & 16 != 0;
    let id_len: u32 = if bits > 16 {
        5
    } else if bits > 8 {
        4
    } else if restricted && bits <= 4 {
        if bits <= 2 { 1 } else { 2 }
    } else {
        3
    };
    let uncomp_id = (1u64 << id_len) - 1;
    let se = se_table();
    let mut r = Bits { b: data, at: 0 };
    let mut out: Vec<u64> = Vec::with_capacity(count);
    let xmax: u64 = (1u64 << bits) - 1;
    while out.len() < count {
        // One reference interval: residuals, then put back.
        let mut raw: Vec<u64> = Vec::with_capacity(rsi * block);
        let mut b = 0usize;
        while b < rsi && out.len() + raw.len() < count {
            let ref_here = pp && b == 0;
            let id = r.take(id_len)?;
            let mut samples: Vec<u64> = Vec::with_capacity(block);
            if id == 0 {
                let second = r.bit()? == 1;
                if ref_here {
                    samples.push(r.take(bits)?);
                }
                if second {
                    let mut i = samples.len();
                    while i < block {
                        let m = r.fs()? as usize;
                        let &(sum, ms) = se.get(m).ok_or("CCSDS ikinci uzantı kodu bozuk")?;
                        let d1 = m as u64 - ms;
                        if i.is_multiple_of(2) {
                            samples.push(sum - d1);
                            i += 1;
                        }
                        samples.push(d1);
                        i += 1;
                    }
                    samples.truncate(block);
                } else {
                    let mut zero = r.fs()? + 1;
                    if zero == ROS {
                        zero = ((rsi - b) as u64).min(64 - (b as u64 % 64));
                    } else if zero > ROS {
                        zero -= 1;
                    }
                    let n = zero as usize * block - samples.len();
                    samples.extend(std::iter::repeat_n(0, n));
                    raw.extend(samples);
                    b += zero as usize;
                    continue;
                }
            } else if id == uncomp_id {
                for _ in 0..block {
                    samples.push(r.take(bits)?);
                }
            } else {
                let k = (id - 1) as u32;
                if ref_here {
                    samples.push(r.take(bits)?);
                }
                let first = samples.len();
                let mut hi = Vec::with_capacity(block - first);
                for _ in first..block {
                    hi.push(r.fs()?);
                }
                for h in hi {
                    samples.push((h << k) | r.take(k)?);
                }
            }
            raw.extend(samples);
            b += 1;
        }
        if flags & 32 != 0 {
            r.align();
        }
        // Put the interval's samples back (the reference sample first; the rest differences mapped).
        let take = raw.len().min(count - out.len());
        if !pp {
            out.extend(&raw[..take]);
            continue;
        }
        let mut last: u64 = 0;
        for (i, &d) in raw[..take].iter().enumerate() {
            if i == 0 {
                last = d;
                out.push(d);
                continue;
            }
            let theta = last.min(xmax - last);
            let x = if d <= 2 * theta {
                if d % 2 == 0 {
                    last + d / 2
                } else {
                    last - d.div_ceil(2)
                }
            } else if theta == last {
                last + (d - theta)
            } else {
                last - (d - theta)
            };
            last = x & xmax;
            out.push(last);
        }
    }
    Ok(out)
}
