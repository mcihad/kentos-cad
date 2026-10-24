//! zstd's Huffman-coded literals (RFC 8878 §4.2): the tree description (weights given directly or FSE-compressed, the
//! last weight implied), the decoding table and one or four streams.

use super::bits::Backward;
use super::fse;

#[derive(Clone, Debug)]
pub struct Huffman {
    max_bits: u32,
    /// (symbol, bits) by the next `max_bits` bits of the stream.
    cells: Vec<(u8, u8)>,
}

/// The tree a description gives, and the bytes the description took.
pub fn read_tree(bytes: &[u8]) -> Result<(Huffman, usize), String> {
    let header = *bytes.first().ok_or("ZSTD Huffman tanımı boş")?;
    let (mut weights, used) = if header < 128 {
        let size = usize::from(header);
        let data = bytes
            .get(1..1 + size)
            .ok_or("ZSTD Huffman ağırlıkları yarım")?;
        (fse_weights(data)?, 1 + size)
    } else {
        let n = usize::from(header - 127);
        let data = bytes
            .get(1..1 + n.div_ceil(2))
            .ok_or("ZSTD Huffman ağırlıkları yarım")?;
        let weights = (0..n)
            .map(|i| {
                let b = data[i / 2];
                if i % 2 == 0 { b >> 4 } else { b & 15 }
            })
            .collect();
        (weights, 1 + n.div_ceil(2))
    };
    if weights.len() > 255 {
        return Err("ZSTD Huffman ağacında çok fazla simge".into());
    }
    let mut total = 0u32;
    for &w in &weights {
        if w > 11 {
            return Err("ZSTD Huffman ağırlığı 11'i aşıyor".into());
        }
        if w > 0 {
            total += 1 << (w - 1);
        }
    }
    if total == 0 {
        return Err("ZSTD Huffman ağacı boş".into());
    }
    let max_bits = 32 - total.leading_zeros();
    if max_bits > 11 {
        return Err("ZSTD Huffman ağacı 11 bitten uzun".into());
    }
    let rest = (1u32 << max_bits) - total;
    if rest == 0 || !rest.is_power_of_two() {
        return Err("ZSTD Huffman ağırlıkları tamamlanmıyor".into());
    }
    weights.push((32 - rest.leading_zeros()) as u8);
    let mut starts = [0u32; 13];
    let mut counts = [0u32; 13];
    for &w in &weights {
        counts[usize::from(w)] += 1;
    }
    let mut pos = 0u32;
    for w in 1..=max_bits as usize {
        starts[w] = pos;
        pos += counts[w] << (w - 1);
    }
    if pos != 1 << max_bits {
        return Err("ZSTD Huffman tablosu tutmuyor".into());
    }
    let mut cells = vec![(0u8, 0u8); 1 << max_bits];
    for (symbol, &w) in weights.iter().enumerate() {
        if w == 0 {
            continue;
        }
        let w = usize::from(w);
        let len = 1u32 << (w - 1);
        let bits = (max_bits + 1 - w as u32) as u8;
        for k in starts[w]..starts[w] + len {
            cells[k as usize] = (symbol as u8, bits);
        }
        starts[w] += len;
    }
    Ok((Huffman { max_bits, cells }, used))
}

/// The weights an FSE-compressed description gives: two states over one table, taking turns.
fn fse_weights(data: &[u8]) -> Result<Vec<u8>, String> {
    let (counts, log, used) = fse::read_counts(data, 6, 255)?;
    let table = fse::build(&counts, log)?;
    let mut bits = Backward::new(data.get(used..).ok_or("ZSTD Huffman ağırlıkları yarım")?)?;
    let mut s1 = bits.read(log) as usize;
    let mut s2 = bits.read(log) as usize;
    let mut weights = Vec::new();
    loop {
        let c = cell(&table, s1)?;
        weights.push(c.symbol);
        s1 = usize::from(c.base) + bits.read(u32::from(c.bits)) as usize;
        if bits.overflowed() {
            weights.push(cell(&table, s2)?.symbol);
            break;
        }
        let c = cell(&table, s2)?;
        weights.push(c.symbol);
        s2 = usize::from(c.base) + bits.read(u32::from(c.bits)) as usize;
        if bits.overflowed() {
            weights.push(cell(&table, s1)?.symbol);
            break;
        }
        if weights.len() > 255 {
            return Err("ZSTD Huffman ağırlıkları bitmiyor".into());
        }
    }
    Ok(weights)
}

fn cell(table: &fse::Table, state: usize) -> Result<fse::Cell, String> {
    table
        .cells
        .get(state)
        .copied()
        .ok_or_else(|| "ZSTD durumu tablonun dışında".to_owned())
}

impl Huffman {
    /// `size` literals from one stream, or from four after their jump table.
    pub fn decode(&self, data: &[u8], four: bool, size: usize) -> Result<Vec<u8>, String> {
        let mut out = Vec::with_capacity(size);
        if !four {
            self.stream(data, size, &mut out)?;
            return Ok(out);
        }
        let jump = data
            .get(..6)
            .ok_or("ZSTD dört akışın sıçrama tablosu yarım")?;
        let lens = [
            usize::from(u16::from_le_bytes([jump[0], jump[1]])),
            usize::from(u16::from_le_bytes([jump[2], jump[3]])),
            usize::from(u16::from_le_bytes([jump[4], jump[5]])),
        ];
        let first = lens[0] + lens[1] + lens[2];
        if 6 + first > data.len() {
            return Err("ZSTD akışları bloğun dışına taşıyor".into());
        }
        let each = size.div_ceil(4);
        if each * 3 > size {
            return Err("ZSTD değişmezleri dört akışa sığmıyor".into());
        }
        let mut at = 6;
        for &len in &lens {
            self.stream(&data[at..at + len], each, &mut out)?;
            at += len;
        }
        self.stream(&data[at..], size - 3 * each, &mut out)?;
        Ok(out)
    }

    fn stream(&self, data: &[u8], count: usize, out: &mut Vec<u8>) -> Result<(), String> {
        if count == 0 && data.is_empty() {
            return Ok(());
        }
        let mut bits = Backward::new(data)?;
        for _ in 0..count {
            let (symbol, n) = self.cells[bits.peek(self.max_bits) as usize];
            out.push(symbol);
            bits.skip(u32::from(n));
        }
        if !bits.finished() {
            return Err("ZSTD Huffman akışı tam okunmadı".into());
        }
        Ok(())
    }
}
