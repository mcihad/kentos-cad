//! zstd's finite state entropy tables (RFC 8878 §4.1): the normalized counts of a table description and the decoding
//! table built from them.

use super::bits::Forward;

#[derive(Clone, Copy, Debug, Default)]
pub struct Cell {
    pub symbol: u8,
    pub bits: u8,
    pub base: u16,
}

#[derive(Clone, Debug)]
pub struct Table {
    pub log: u32,
    pub cells: Vec<Cell>,
}

impl Table {
    /// A table of one symbol read with no bits (RLE mode).
    pub fn single(symbol: u8) -> Table {
        Table {
            log: 0,
            cells: vec![Cell {
                symbol,
                bits: 0,
                base: 0,
            }],
        }
    }
}

/// A table description's normalized counts, its accuracy log and the bytes it took.
pub fn read_counts(
    bytes: &[u8],
    max_log: u32,
    max_symbol: usize,
) -> Result<(Vec<i16>, u32, usize), String> {
    let mut f = Forward::new(bytes);
    let log = f.read(4) + 5;
    if log > max_log {
        return Err(format!(
            "ZSTD tablosunun doğruluğu {log}, en çok {max_log} olabilir"
        ));
    }
    let mut remaining: i32 = (1 << log) + 1;
    let mut threshold: i32 = 1 << log;
    let mut nb_bits = log + 1;
    let mut counts: Vec<i16> = Vec::new();
    while remaining > 1 {
        if counts.len() > max_symbol {
            return Err("ZSTD tablosunda çok fazla simge".into());
        }
        let max = (2 * threshold - 1) - remaining;
        let low = f.peek(nb_bits - 1) as i32;
        let value = if low < max {
            f.read(nb_bits - 1);
            low
        } else {
            let v = f.read(nb_bits) as i32;
            if v >= threshold { v - max } else { v }
        };
        let count = value - 1;
        remaining -= count.abs();
        counts.push(count as i16);
        if count == 0 {
            loop {
                let repeat = f.read(2);
                counts.extend(std::iter::repeat_n(0, repeat as usize));
                if repeat != 3 {
                    break;
                }
                if counts.len() > max_symbol + 1 {
                    return Err("ZSTD tablosunda çok fazla simge".into());
                }
            }
        }
        while remaining < threshold && threshold > 1 {
            nb_bits -= 1;
            threshold >>= 1;
        }
        if f.overrun() {
            return Err("ZSTD tablo tanımı yarım".into());
        }
    }
    if remaining != 1 || counts.len() > max_symbol + 1 {
        return Err("ZSTD tablo tanımı tutarsız".into());
    }
    Ok((counts, log, f.bytes_used()))
}

/// The decoding table of `counts` at accuracy `log`.
pub fn build(counts: &[i16], log: u32) -> Result<Table, String> {
    let size = 1usize << log;
    let total: i64 = counts.iter().map(|&c| i64::from(c).abs()).sum();
    if total != size as i64 || counts.len() > 256 {
        return Err("ZSTD tablosunun sayıları tutmuyor".into());
    }
    let mut cells = vec![Cell::default(); size];
    let mut next: Vec<u32> = vec![0; counts.len()];
    let mut high = size as isize - 1;
    for (s, &c) in counts.iter().enumerate() {
        if c == -1 {
            cells[high as usize].symbol = s as u8;
            high -= 1;
            next[s] = 1;
        } else {
            next[s] = c.max(0) as u32;
        }
    }
    let step = (size >> 1) + (size >> 3) + 3;
    let mask = size - 1;
    let mut pos = 0usize;
    for (s, &c) in counts.iter().enumerate() {
        for _ in 0..c.max(0) {
            cells[pos].symbol = s as u8;
            pos = (pos + step) & mask;
            while pos as isize > high {
                pos = (pos + step) & mask;
            }
        }
    }
    if pos != 0 {
        return Err("ZSTD tablosu yerleşmedi".into());
    }
    for cell in &mut cells {
        let s = usize::from(cell.symbol);
        let state = next[s];
        next[s] += 1;
        if state == 0 {
            return Err("ZSTD tablosu tutarsız".into());
        }
        let bits = log - (31 - state.leading_zeros());
        cell.bits = bits as u8;
        cell.base = ((state << bits) - size as u32) as u16;
    }
    Ok(Table { log, cells })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_predefined_literal_lengths_build() {
        let t = build(&super::super::sequences::LL_DEFAULT, 6).unwrap();
        assert_eq!(t.cells.len(), 64);
        // Every state's next state stays inside the table.
        for c in &t.cells {
            assert!(usize::from(c.base) + (1usize << c.bits) <= 64);
        }
    }

    #[test]
    fn counts_that_do_not_add_up_are_refused() {
        assert!(build(&[3, 3], 2).is_err());
    }
}
