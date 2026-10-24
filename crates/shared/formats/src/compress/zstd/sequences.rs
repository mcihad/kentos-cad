//! zstd's sequences (RFC 8878 §3.1.1.3.2): literal lengths, match lengths and offsets decoded with three FSE tables
//! (predefined, one symbol, described, or the block before's), then carried out with the repeat offsets.

use super::bits::Backward;
use super::fse::{self, Table};

/// The predefined literal length distribution (accuracy 6).
pub const LL_DEFAULT: [i16; 36] = [
    4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1,
    -1, -1, -1, -1,
];
/// The predefined match length distribution (accuracy 6).
pub const ML_DEFAULT: [i16; 53] = [
    1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1, -1, -1,
];
/// The predefined offset code distribution (accuracy 5).
pub const OF_DEFAULT: [i16; 29] = [
    1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1,
];

/// A literal length code's baseline and extra bits.
fn literal_length(code: u8) -> Option<(u32, u32)> {
    const TAIL: [(u32, u32); 20] = [
        (16, 1),
        (18, 1),
        (20, 1),
        (22, 1),
        (24, 2),
        (28, 2),
        (32, 3),
        (40, 3),
        (48, 4),
        (64, 6),
        (128, 7),
        (256, 8),
        (512, 9),
        (1024, 10),
        (2048, 11),
        (4096, 12),
        (8192, 13),
        (16384, 14),
        (32768, 15),
        (65536, 16),
    ];
    match code {
        0..=15 => Some((u32::from(code), 0)),
        16..=35 => Some(TAIL[usize::from(code - 16)]),
        _ => None,
    }
}

/// A match length code's baseline and extra bits.
fn match_length(code: u8) -> Option<(u32, u32)> {
    const TAIL: [(u32, u32); 21] = [
        (35, 1),
        (37, 1),
        (39, 1),
        (41, 1),
        (43, 2),
        (47, 2),
        (51, 3),
        (59, 3),
        (67, 4),
        (83, 4),
        (99, 5),
        (131, 7),
        (259, 8),
        (515, 9),
        (1027, 10),
        (2051, 11),
        (4099, 12),
        (8195, 13),
        (16387, 14),
        (32771, 15),
        (65539, 16),
    ];
    match code {
        0..=31 => Some((u32::from(code) + 3, 0)),
        32..=52 => Some(TAIL[usize::from(code - 32)]),
        _ => None,
    }
}

/// The three tables of the block before (for the Repeat mode) and the repeat offsets: a frame's state.
pub struct Tables {
    pub ll: Option<Table>,
    pub of: Option<Table>,
    pub ml: Option<Table>,
    pub rep: [usize; 3],
}

impl Tables {
    pub fn new() -> Tables {
        Tables {
            ll: None,
            of: None,
            ml: None,
            rep: [1, 4, 8],
        }
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Ll,
    Of,
    Ml,
}

/// One table by its mode, read from `bytes` at `at` (moved past what it took).
fn table(
    mode: u8,
    kind: Kind,
    bytes: &[u8],
    at: &mut usize,
    previous: &Option<Table>,
) -> Result<Table, String> {
    let (default, default_log, max_log, max_symbol): (&[i16], u32, u32, usize) = match kind {
        Kind::Ll => (&LL_DEFAULT, 6, 9, 35),
        Kind::Of => (&OF_DEFAULT, 5, 8, 31),
        Kind::Ml => (&ML_DEFAULT, 6, 9, 52),
    };
    match mode {
        0 => fse::build(default, default_log),
        1 => {
            let symbol = *bytes.get(*at).ok_or("ZSTD dizi tablosu yarım")?;
            *at += 1;
            if usize::from(symbol) > max_symbol {
                return Err("ZSTD dizi simgesi aralık dışında".into());
            }
            Ok(Table::single(symbol))
        }
        2 => {
            let rest = bytes.get(*at..).ok_or("ZSTD dizi tablosu yarım")?;
            let (counts, log, used) = fse::read_counts(rest, max_log, max_symbol)?;
            *at += used;
            fse::build(&counts, log)
        }
        _ => previous
            .clone()
            .ok_or_else(|| "ZSTD önceki tabloyu yineliyor ama önceki tablo yok".to_owned()),
    }
}

/// The sequences of `bytes` (the block after its literals) carried out into `out` with `literals`; `frame_start`
/// bounds how far back a match may copy and `limit` how much may be written.
pub fn execute(
    bytes: &[u8],
    literals: &[u8],
    tables: &mut Tables,
    out: &mut Vec<u8>,
    frame_start: usize,
    limit: usize,
) -> Result<(), String> {
    let b0 = *bytes.first().ok_or("ZSTD dizi bölümü yok")?;
    let (count, mut at) = match b0 {
        0 => (0usize, 1usize),
        1..=127 => (usize::from(b0), 1),
        128..=254 => {
            let b1 = *bytes.get(1).ok_or("ZSTD dizi sayısı yarım")?;
            (((usize::from(b0) - 128) << 8) + usize::from(b1), 2)
        }
        255 => {
            let b = bytes.get(1..3).ok_or("ZSTD dizi sayısı yarım")?;
            (usize::from(b[0]) + (usize::from(b[1]) << 8) + 0x7f00, 3)
        }
    };
    if count == 0 {
        if out.len() + literals.len() > limit {
            return Err("ZSTD açılan veri sınırı aşıyor".into());
        }
        out.extend_from_slice(literals);
        return Ok(());
    }
    let modes = *bytes.get(at).ok_or("ZSTD dizi kipleri yok")?;
    at += 1;
    if modes & 3 != 0 {
        return Err("ZSTD dizi kiplerinin ayrılmış bitleri dolu".into());
    }
    let ll = table(modes >> 6, Kind::Ll, bytes, &mut at, &tables.ll)?;
    let of = table((modes >> 4) & 3, Kind::Of, bytes, &mut at, &tables.of)?;
    let ml = table((modes >> 2) & 3, Kind::Ml, bytes, &mut at, &tables.ml)?;
    let mut bits = Backward::new(bytes.get(at..).ok_or("ZSTD dizi akışı yok")?)?;
    let mut ll_state = bits.read(ll.log) as usize;
    let mut of_state = bits.read(of.log) as usize;
    let mut ml_state = bits.read(ml.log) as usize;
    let mut lit = 0usize;
    for n in 0..count {
        let ll_cell = *ll
            .cells
            .get(ll_state)
            .ok_or("ZSTD durumu tablonun dışında")?;
        let of_cell = *of
            .cells
            .get(of_state)
            .ok_or("ZSTD durumu tablonun dışında")?;
        let ml_cell = *ml
            .cells
            .get(ml_state)
            .ok_or("ZSTD durumu tablonun dışında")?;
        if of_cell.symbol > 31 {
            return Err("ZSTD uzaklık kodu aralık dışında".into());
        }
        let offset_value =
            (1usize << of_cell.symbol) + bits.read(u32::from(of_cell.symbol)) as usize;
        let (ml_base, ml_bits) =
            match_length(ml_cell.symbol).ok_or("ZSTD eşleşme kodu aralık dışında")?;
        let match_len = (ml_base + bits.read(ml_bits) as u32) as usize;
        let (ll_base, ll_bits) =
            literal_length(ll_cell.symbol).ok_or("ZSTD değişmez kodu aralık dışında")?;
        let literal_len = (ll_base + bits.read(ll_bits) as u32) as usize;
        if n + 1 < count {
            ll_state = usize::from(ll_cell.base) + bits.read(u32::from(ll_cell.bits)) as usize;
            ml_state = usize::from(ml_cell.base) + bits.read(u32::from(ml_cell.bits)) as usize;
            of_state = usize::from(of_cell.base) + bits.read(u32::from(of_cell.bits)) as usize;
        }
        let rep = &mut tables.rep;
        let offset = if offset_value > 3 {
            let o = offset_value - 3;
            *rep = [o, rep[0], rep[1]];
            o
        } else {
            let index = offset_value - 1 + usize::from(literal_len == 0);
            let o = match index {
                0 => rep[0],
                1 => rep[1],
                2 => rep[2],
                _ => rep[0]
                    .checked_sub(1)
                    .filter(|&o| o > 0)
                    .ok_or("ZSTD yinelenen uzaklık sıfıra iniyor")?,
            };
            match index {
                0 => {}
                1 => *rep = [rep[1], rep[0], rep[2]],
                _ => *rep = [o, rep[0], rep[1]],
            }
            o
        };
        let piece = literals
            .get(lit..lit + literal_len)
            .ok_or("ZSTD dizisi değişmezlerin dışına taşıyor")?;
        if out.len() + literal_len + match_len > limit {
            return Err("ZSTD açılan veri sınırı aşıyor".into());
        }
        out.extend_from_slice(piece);
        lit += literal_len;
        if offset > out.len() - frame_start {
            return Err(format!(
                "ZSTD eşleşmesinin uzaklığı ({offset}) çerçevenin dışında"
            ));
        }
        let start = out.len() - offset;
        if offset >= match_len {
            out.extend_from_within(start..start + match_len);
        } else {
            for k in 0..match_len {
                let v = out[start + k];
                out.push(v);
            }
        }
    }
    if !bits.finished() {
        return Err("ZSTD dizi akışı tam okunmadı".into());
    }
    let rest = &literals[lit..];
    if out.len() + rest.len() > limit {
        return Err("ZSTD açılan veri sınırı aşıyor".into());
    }
    out.extend_from_slice(rest);
    tables.ll = Some(ll);
    tables.of = Some(of);
    tables.ml = Some(ml);
    Ok(())
}
