//! Parquet's value encodings (docs/adr/0628 §1): PLAIN, the dictionary's indices, RLE booleans, DELTA_BINARY_PACKED,
//! DELTA_LENGTH_BYTE_ARRAY, DELTA_BYTE_ARRAY and BYTE_STREAM_SPLIT, decoded into typed columns without a value
//! allocated each; PLAIN written. Every length is checked against the page.

use super::meta::Physical;
use super::rle;

/// Byte arrays one after another: value `i` is `data[ends[i - 1]..ends[i]]`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bytes {
    pub data: Vec<u8>,
    pub ends: Vec<usize>,
}

impl Bytes {
    pub fn len(&self) -> usize {
        self.ends.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ends.is_empty()
    }

    pub fn get(&self, i: usize) -> &[u8] {
        let start = if i == 0 { 0 } else { self.ends[i - 1] };
        &self.data[start..self.ends[i]]
    }

    pub fn push(&mut self, v: &[u8]) {
        self.data.extend_from_slice(v);
        self.ends.push(self.data.len());
    }
}

/// A column's values (nulls left out), by physical type.
#[derive(Clone, Debug, PartialEq)]
pub enum Values {
    Bool(Vec<bool>),
    I32(Vec<i32>),
    I64(Vec<i64>),
    I96(Vec<[u8; 12]>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    /// BYTE_ARRAY and FIXED_LEN_BYTE_ARRAY.
    Bytes(Bytes),
}

impl Values {
    pub fn empty(p: Physical) -> Values {
        match p {
            Physical::Boolean => Values::Bool(Vec::new()),
            Physical::Int32 => Values::I32(Vec::new()),
            Physical::Int64 => Values::I64(Vec::new()),
            Physical::Int96 => Values::I96(Vec::new()),
            Physical::Float => Values::F32(Vec::new()),
            Physical::Double => Values::F64(Vec::new()),
            Physical::ByteArray | Physical::FixedLenByteArray => Values::Bytes(Bytes::default()),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Values::Bool(v) => v.len(),
            Values::I32(v) => v.len(),
            Values::I64(v) => v.len(),
            Values::I96(v) => v.len(),
            Values::F32(v) => v.len(),
            Values::F64(v) => v.len(),
            Values::Bytes(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Appends `dict`'s values at `indices`.
    pub fn take(&mut self, dict: &Values, indices: &[u32]) -> Result<(), String> {
        let n = dict.len();
        if let Some(&bad) = indices.iter().find(|&&i| i as usize >= n) {
            return Err(format!(
                "Parquet sözlük dizini ({bad}) sözlüğün dışında ({n} değer)"
            ));
        }
        match (self, dict) {
            (Values::Bool(o), Values::Bool(d)) => o.extend(indices.iter().map(|&i| d[i as usize])),
            (Values::I32(o), Values::I32(d)) => o.extend(indices.iter().map(|&i| d[i as usize])),
            (Values::I64(o), Values::I64(d)) => o.extend(indices.iter().map(|&i| d[i as usize])),
            (Values::I96(o), Values::I96(d)) => o.extend(indices.iter().map(|&i| d[i as usize])),
            (Values::F32(o), Values::F32(d)) => o.extend(indices.iter().map(|&i| d[i as usize])),
            (Values::F64(o), Values::F64(d)) => o.extend(indices.iter().map(|&i| d[i as usize])),
            (Values::Bytes(o), Values::Bytes(d)) => {
                for &i in indices {
                    o.push(d.get(i as usize));
                }
            }
            _ => return Err("Parquet sözlüğünün türü sütununkiyle aynı değil".into()),
        }
        Ok(())
    }
}

fn fixed<const N: usize>(
    bytes: &[u8],
    n: usize,
) -> Result<impl Iterator<Item = [u8; N]> + '_, String> {
    let size = n.checked_mul(N).ok_or("Parquet sayfası bozuk")?;
    let b = bytes
        .get(..size)
        .ok_or("Parquet sayfasının değerleri yarım")?;
    Ok(b.chunks_exact(N).map(|c| {
        let mut a = [0u8; N];
        a.copy_from_slice(c);
        a
    }))
}

/// `n` PLAIN values appended to `out`; the bytes they took.
pub fn plain(
    p: Physical,
    type_length: usize,
    bytes: &[u8],
    n: usize,
    out: &mut Values,
) -> Result<usize, String> {
    match out {
        Values::Bool(o) => {
            let size = n.div_ceil(8);
            let b = bytes
                .get(..size)
                .ok_or("Parquet sayfasının değerleri yarım")?;
            o.extend((0..n).map(|k| (b[k / 8] >> (k % 8)) & 1 == 1));
            Ok(size)
        }
        Values::I32(o) => {
            o.extend(fixed::<4>(bytes, n)?.map(i32::from_le_bytes));
            Ok(4 * n)
        }
        Values::I64(o) => {
            o.extend(fixed::<8>(bytes, n)?.map(i64::from_le_bytes));
            Ok(8 * n)
        }
        Values::I96(o) => {
            o.extend(fixed::<12>(bytes, n)?);
            Ok(12 * n)
        }
        Values::F32(o) => {
            o.extend(fixed::<4>(bytes, n)?.map(f32::from_le_bytes));
            Ok(4 * n)
        }
        Values::F64(o) => {
            o.extend(fixed::<8>(bytes, n)?.map(f64::from_le_bytes));
            Ok(8 * n)
        }
        Values::Bytes(o) => {
            if p == Physical::FixedLenByteArray {
                let size = n.checked_mul(type_length).ok_or("Parquet sayfası bozuk")?;
                let b = bytes
                    .get(..size)
                    .ok_or("Parquet sayfasının değerleri yarım")?;
                o.data.reserve(size);
                for k in 0..n {
                    o.push(&b[k * type_length..(k + 1) * type_length]);
                }
                return Ok(size);
            }
            let mut at = 0usize;
            for _ in 0..n {
                let l = bytes
                    .get(at..at + 4)
                    .ok_or("Parquet sayfasının değerleri yarım")?;
                let len = u32::from_le_bytes([l[0], l[1], l[2], l[3]]) as usize;
                at += 4;
                let v = bytes
                    .get(at..at.checked_add(len).ok_or("Parquet sayfası bozuk")?)
                    .ok_or("Parquet sayfasının değerleri yarım")?;
                o.push(v);
                at += len;
            }
            Ok(at)
        }
    }
}

/// The dictionary indices of a data page: their width in a byte, then the hybrid.
pub fn indices(bytes: &[u8], n: usize) -> Result<Vec<u32>, String> {
    let width = u32::from(*bytes.first().ok_or("Parquet sözlük dizinleri yarım")?);
    let mut out = Vec::with_capacity(n);
    if n > 0 {
        rle::read(&bytes[1..], width, n, &mut out)?;
    }
    Ok(out)
}

/// Booleans in the RLE encoding: a four-byte length, then the hybrid of width 1.
pub fn rle_bools(bytes: &[u8], n: usize, out: &mut Vec<bool>) -> Result<(), String> {
    let mut v = Vec::with_capacity(n);
    rle::read_prefixed(bytes, 1, n, &mut v)?;
    out.extend(v.into_iter().map(|b| b == 1));
    Ok(())
}

fn uleb(bytes: &[u8], at: &mut usize) -> Result<u64, String> {
    let mut v = 0u64;
    for shift in (0..64).step_by(7) {
        let b = *bytes.get(*at).ok_or("Parquet delta akışı yarım")?;
        *at += 1;
        v |= u64::from(b & 0x7f) << shift;
        if b & 0x80 == 0 {
            return Ok(v);
        }
    }
    Err("Parquet delta akışında bozuk sayı".into())
}

fn zigzag(v: u64) -> i64 {
    (v >> 1) as i64 ^ -((v & 1) as i64)
}

/// DELTA_BINARY_PACKED of at most `max` values: the values (in 64 bits, wrapping) and the bytes the stream took, up to
/// the end of its last miniblock as written.
pub fn delta_binary(bytes: &[u8], max: usize) -> Result<(Vec<i64>, usize), String> {
    let mut at = 0usize;
    let block = usize::try_from(uleb(bytes, &mut at)?).map_err(|_| "Parquet delta bloğu bozuk")?;
    let minis = usize::try_from(uleb(bytes, &mut at)?).map_err(|_| "Parquet delta bloğu bozuk")?;
    let total = usize::try_from(uleb(bytes, &mut at)?).map_err(|_| "Parquet delta bloğu bozuk")?;
    let first = zigzag(uleb(bytes, &mut at)?);
    if block == 0
        || minis == 0
        || block % 128 != 0
        || block % minis != 0
        || (block / minis) % 32 != 0
    {
        return Err(format!(
            "Parquet delta bloğu ({block} değer, {minis} küçük blok) kurala uymuyor"
        ));
    }
    if total > max {
        return Err(format!(
            "Parquet delta akışı {total} değer söylüyor, sayfada {max} var"
        ));
    }
    let mini = block / minis;
    let mut out = Vec::with_capacity(total);
    if total == 0 {
        return Ok((out, at));
    }
    out.push(first);
    let mut last = first;
    let mut unpacked = Vec::with_capacity(mini);
    while out.len() < total {
        let min_delta = zigzag(uleb(bytes, &mut at)?);
        let widths = bytes
            .get(at..at + minis)
            .ok_or("Parquet delta bloğu yarım")?
            .to_vec();
        at += minis;
        for &w in &widths {
            if out.len() >= total {
                break;
            }
            let w = u32::from(w);
            if w > 64 {
                return Err("Parquet delta genişliği 64'ü aşıyor".into());
            }
            let size = mini * w as usize / 8;
            let body = bytes.get(at..).ok_or("Parquet delta bloğu yarım")?;
            let take = (total - out.len()).min(mini);
            if body.len() < (take * w as usize).div_ceil(8) {
                return Err("Parquet delta küçük bloğu yarım".into());
            }
            unpacked.clear();
            if w <= 32 {
                rle::unpack(&body[..size.min(body.len())], w, take, &mut unpacked);
                for &d in &unpacked {
                    last = last.wrapping_add(min_delta).wrapping_add(i64::from(d));
                    out.push(last);
                }
            } else {
                for k in 0..take {
                    let mut v = 0u64;
                    for j in 0..w as usize {
                        let bit = k * w as usize + j;
                        v |= u64::from((body[bit / 8] >> (bit % 8)) & 1) << j;
                    }
                    last = last.wrapping_add(min_delta).wrapping_add(v as i64);
                    out.push(last);
                }
            }
            at += size;
        }
    }
    Ok((out, at.min(bytes.len())))
}

/// DELTA_LENGTH_BYTE_ARRAY of at most `max` values: the lengths, then the bytes one after another; the bytes it took.
pub fn delta_length(bytes: &[u8], max: usize, out: &mut Bytes) -> Result<usize, String> {
    let (lengths, mut at) = delta_binary(bytes, max)?;
    for l in lengths {
        let l = usize::try_from(l).map_err(|_| "Parquet uzunluğu eksi")?;
        let v = bytes
            .get(at..at.checked_add(l).ok_or("Parquet sayfası bozuk")?)
            .ok_or("Parquet sayfasının değerleri yarım")?;
        out.push(v);
        at += l;
    }
    Ok(at)
}

/// DELTA_BYTE_ARRAY of at most `max` values: prefix lengths, then the suffixes as DELTA_LENGTH_BYTE_ARRAY; each value
/// the previous one's prefix and its suffix.
pub fn delta_bytes(bytes: &[u8], max: usize, out: &mut Bytes) -> Result<usize, String> {
    let (prefixes, at) = delta_binary(bytes, max)?;
    let mut suffixes = Bytes::default();
    let used = delta_length(&bytes[at..], max, &mut suffixes)?;
    if suffixes.len() != prefixes.len() {
        return Err("Parquet DELTA_BYTE_ARRAY önek ve sonek sayıları tutmuyor".into());
    }
    let mut previous: Vec<u8> = Vec::new();
    for (k, &p) in prefixes.iter().enumerate() {
        let p = usize::try_from(p).map_err(|_| "Parquet öneki eksi")?;
        if p > previous.len() {
            return Err("Parquet öneki önceki değerden uzun".into());
        }
        previous.truncate(p);
        previous.extend_from_slice(suffixes.get(k));
        out.push(&previous);
    }
    Ok(at + used)
}

/// BYTE_STREAM_SPLIT of `n` values `width` bytes wide: the bytes of value `i` gathered from each stream.
pub fn byte_stream_split(bytes: &[u8], width: usize, n: usize) -> Result<Vec<u8>, String> {
    let size = n.checked_mul(width).ok_or("Parquet sayfası bozuk")?;
    let b = bytes
        .get(..size)
        .ok_or("Parquet sayfasının değerleri yarım")?;
    let mut out = vec![0u8; size];
    for k in 0..width {
        let stream = &b[k * n..(k + 1) * n];
        for (i, &v) in stream.iter().enumerate() {
            out[i * width + k] = v;
        }
    }
    Ok(out)
}

/// Writes `values` PLAIN.
pub fn write_plain(values: &Values, out: &mut Vec<u8>) {
    match values {
        Values::Bool(v) => {
            let mut b = vec![0u8; v.len().div_ceil(8)];
            for (k, &x) in v.iter().enumerate() {
                if x {
                    b[k / 8] |= 1 << (k % 8);
                }
            }
            out.extend_from_slice(&b);
        }
        Values::I32(v) => v
            .iter()
            .for_each(|x| out.extend_from_slice(&x.to_le_bytes())),
        Values::I64(v) => v
            .iter()
            .for_each(|x| out.extend_from_slice(&x.to_le_bytes())),
        Values::I96(v) => v.iter().for_each(|x| out.extend_from_slice(x)),
        Values::F32(v) => v
            .iter()
            .for_each(|x| out.extend_from_slice(&x.to_le_bytes())),
        Values::F64(v) => v
            .iter()
            .for_each(|x| out.extend_from_slice(&x.to_le_bytes())),
        Values::Bytes(v) => {
            for k in 0..v.len() {
                let s = v.get(k);
                out.extend_from_slice(&(s.len() as u32).to_le_bytes());
                out.extend_from_slice(s);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_specs_delta_example_reads() {
        // parquet-format Encodings.md: 1, 2, 3, 4, 5 → header 128 4 5 2, min delta 1 (zigzag 2), widths 0 0 0 0.
        let bytes = [0x80, 0x01, 0x04, 0x05, 0x02, 0x02, 0, 0, 0, 0];
        let (v, used) = delta_binary(&bytes, 5).unwrap();
        assert!(delta_binary(&bytes, 4).is_err());
        assert_eq!(v, [1, 2, 3, 4, 5]);
        assert_eq!(used, bytes.len());
    }

    #[test]
    fn byte_stream_split_gathers_each_value() {
        let values = [1.5f32, -2.25, 1e30];
        let raw: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        let mut split = vec![0u8; raw.len()];
        for (i, v) in raw.chunks(4).enumerate() {
            for k in 0..4 {
                split[k * 3 + i] = v[k];
            }
        }
        assert_eq!(byte_stream_split(&split, 4, 3).unwrap(), raw);
    }
}
