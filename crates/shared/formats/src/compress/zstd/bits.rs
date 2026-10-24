//! zstd's bit streams (RFC 8878 §4.1): FSE and Huffman streams are read backwards from their end, whose last byte's
//! highest set bit marks where the bits begin; table descriptions are read forwards, least significant bit first.

/// A stream read from its end: `pos` bits are left; reading past the beginning gives zeros and leaves `pos` negative.
pub struct Backward<'a> {
    bytes: &'a [u8],
    pos: isize,
}

impl<'a> Backward<'a> {
    pub fn new(bytes: &'a [u8]) -> Result<Backward<'a>, String> {
        let last = *bytes.last().ok_or("ZSTD bit akışı boş")?;
        if last == 0 {
            return Err("ZSTD bit akışının son baytı sıfır".into());
        }
        let pos = bytes.len() as isize * 8 - (last.leading_zeros() as isize + 1);
        Ok(Backward { bytes, pos })
    }

    /// The next `n` bits (at most 56) without taking them, the first read the most significant.
    pub fn peek(&self, n: u32) -> u64 {
        if n == 0 {
            return 0;
        }
        let end = self.pos;
        let start = end - n as isize;
        if end <= 0 {
            return 0;
        }
        let (lo, missing) = if start < 0 {
            (0usize, (-start) as u32)
        } else {
            (start as usize, 0)
        };
        let count = (end as usize - lo) as u32;
        let byte = lo / 8;
        let shift = (lo % 8) as u32;
        let word = match self.bytes.get(byte..byte + 8) {
            Some(b) => u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]),
            None => {
                let mut w = 0u64;
                for k in 0..8 {
                    if let Some(&b) = self.bytes.get(byte + k) {
                        w |= u64::from(b) << (8 * k);
                    }
                }
                w
            }
        };
        let value = (word >> shift) & ((1u64 << count) - 1);
        value << missing
    }

    pub fn skip(&mut self, n: u32) {
        self.pos -= n as isize;
    }

    pub fn read(&mut self, n: u32) -> u64 {
        let v = self.peek(n);
        self.skip(n);
        v
    }

    /// Whether more bits were read than the stream holds.
    pub fn overflowed(&self) -> bool {
        self.pos < 0
    }

    /// Whether every bit was read, no more.
    pub fn finished(&self) -> bool {
        self.pos == 0
    }
}

/// A stream read from its start, least significant bit first; reading past the end gives zeros.
pub struct Forward<'a> {
    bytes: &'a [u8],
    pub pos: usize,
}

impl<'a> Forward<'a> {
    pub fn new(bytes: &'a [u8]) -> Forward<'a> {
        Forward { bytes, pos: 0 }
    }

    pub fn peek(&self, n: u32) -> u32 {
        let mut value = 0u32;
        for k in 0..n as usize {
            let p = self.pos + k;
            let bit = self.bytes.get(p / 8).map_or(0, |b| (b >> (p % 8)) & 1);
            value |= u32::from(bit) << k;
        }
        value
    }

    pub fn read(&mut self, n: u32) -> u32 {
        let v = self.peek(n);
        self.pos += n as usize;
        v
    }

    /// The whole bytes read so far.
    pub fn bytes_used(&self) -> usize {
        self.pos.div_ceil(8)
    }

    pub fn overrun(&self) -> bool {
        self.pos > self.bytes.len() * 8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_backward_stream_reads_its_last_bits_first() {
        // Written forwards: 0b101 (3 bits) then 0b01 (2 bits), then the end mark: byte 0b0010_1101.
        let bytes = [0b0010_1101u8];
        let mut b = Backward::new(&bytes).unwrap();
        assert_eq!(b.read(2), 0b01);
        assert_eq!(b.read(3), 0b101);
        assert!(b.finished());
        assert_eq!(b.read(1), 0);
        assert!(b.overflowed());
    }
}
