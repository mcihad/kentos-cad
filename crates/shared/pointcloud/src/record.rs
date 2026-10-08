//! Point data records (ASPRS LAS 1.4 R15 §2.6): the layouts of formats 0–10,
//! their fields read and written in place, and the move of any record to
//! formats 6, 7 and 8 the COPC index holds (docs/adr/0207 §4, LAS 1.4 R15
//! Annex A's legacy rules).

use crate::bytes::{f64_at, i16_at, i32_at, put_f64, put_i16, put_i32, put_u16, u16_at};

/// The most bytes a record may take (docs/adr/0207 §2).
pub const MAX_RECORD: u16 = 1024;

/// A format's own bytes, extra bytes not counted.
pub const fn standard_len(format: u8) -> usize {
    match format {
        0 => 20,
        1 => 28,
        2 => 26,
        3 => 34,
        4 => 57,
        5 => 63,
        6 => 30,
        7 => 36,
        8 => 38,
        9 => 59,
        10 => 67,
        _ => usize::MAX,
    }
}

/// Where a record's fields lie.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    pub format: u8,
    /// The whole record, extra bytes included.
    pub len: usize,
    /// Byte of the GPS time, when the format has one.
    pub gps: Option<usize>,
    /// Byte of red (then green, blue), when the format has colours.
    pub rgb: Option<usize>,
    /// Byte of near infrared, when the format has it.
    pub nir: Option<usize>,
}

impl Layout {
    pub fn new(format: u8, len: usize) -> Layout {
        let (gps, rgb, nir) = match format {
            0 => (None, None, None),
            1 | 4 => (Some(20), None, None),
            2 => (None, Some(20), None),
            3 | 5 => (Some(20), Some(28), None),
            6 | 9 => (Some(22), None, None),
            7 => (Some(22), Some(30), None),
            8 | 10 => (Some(22), Some(30), Some(36)),
            _ => (None, None, None),
        };
        Layout {
            format,
            len,
            gps,
            rgb,
            nir,
        }
    }

    /// Extra bytes after the format's own.
    pub fn extra(&self) -> usize {
        self.len.saturating_sub(standard_len(self.format))
    }

    /// Whether the format is 6 to 10 (LAS 1.4's).
    pub fn wide(&self) -> bool {
        self.format >= 6
    }

    #[inline]
    pub fn x(&self, r: &[u8]) -> i32 {
        i32_at(r, 0)
    }

    #[inline]
    pub fn y(&self, r: &[u8]) -> i32 {
        i32_at(r, 4)
    }

    #[inline]
    pub fn z(&self, r: &[u8]) -> i32 {
        i32_at(r, 8)
    }

    #[inline]
    pub fn intensity(&self, r: &[u8]) -> u16 {
        u16_at(r, 12)
    }

    /// Return number and number of returns.
    #[inline]
    pub fn returns(&self, r: &[u8]) -> (u8, u8) {
        let b = r[14];
        if self.wide() {
            (b & 0x0F, b >> 4)
        } else {
            (b & 0x07, (b >> 3) & 0x07)
        }
    }

    /// The class (0–255; 0–31 in formats 0–5).
    #[inline]
    pub fn class(&self, r: &[u8]) -> u8 {
        if self.wide() { r[16] } else { r[15] & 0x1F }
    }

    /// Sets the class; formats 0–5 keep their flag bits and take the low five bits.
    #[inline]
    pub fn set_class(&self, r: &mut [u8], c: u8) {
        if self.wide() {
            r[16] = c;
        } else {
            r[15] = (r[15] & 0xE0) | (c & 0x1F);
        }
    }

    /// Synthetic, key-point, withheld and overlap flags in bits 0–3.
    #[inline]
    pub fn flags(&self, r: &[u8]) -> u8 {
        if self.wide() {
            r[15] & 0x0F
        } else {
            (r[15] >> 5) & 0x07
        }
    }

    /// User data: byte 17 in every format.
    #[inline]
    pub fn user_data(&self, r: &[u8]) -> u8 {
        r[17]
    }

    /// The scan angle in 0.006° steps (formats 0–5's whole degrees converted).
    #[inline]
    pub fn scan_angle(&self, r: &[u8]) -> i16 {
        if self.wide() {
            i16_at(r, 18)
        } else {
            rank_to_angle(r[16] as i8)
        }
    }

    #[inline]
    pub fn source_id(&self, r: &[u8]) -> u16 {
        if self.wide() {
            u16_at(r, 20)
        } else {
            u16_at(r, 18)
        }
    }

    #[inline]
    pub fn gps_time(&self, r: &[u8]) -> f64 {
        self.gps.map_or(0.0, |at| f64_at(r, at))
    }

    #[inline]
    pub fn rgb(&self, r: &[u8]) -> Option<[u16; 3]> {
        self.rgb
            .map(|at| [u16_at(r, at), u16_at(r, at + 2), u16_at(r, at + 4)])
    }

    #[inline]
    pub fn nir(&self, r: &[u8]) -> Option<u16> {
        self.nir.map(|at| u16_at(r, at))
    }

    /// The extra bytes.
    #[inline]
    pub fn extras<'a>(&self, r: &'a [u8]) -> &'a [u8] {
        &r[standard_len(self.format).min(r.len())..]
    }

    /// Sets the coordinates' integers.
    #[inline]
    pub fn set_xyz(&self, r: &mut [u8], x: i32, y: i32, z: i32) {
        put_i32(r, 0, x);
        put_i32(r, 4, y);
        put_i32(r, 8, z);
    }
}

/// Formats 0–5's scan angle rank (whole degrees, −90 to 90) as 1.4's 0.006°
/// steps: rank × 1000 / 6, the half away from zero.
pub fn rank_to_angle(rank: i8) -> i16 {
    let n = i32::from(rank) * 1000;
    let (q, m) = (n / 6, n % 6);
    let q = if m.abs() >= 3 { q + m.signum() } else { q };
    q.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
}

/// The format of 6, 7 and 8 a record of `format` moves to: 7 with colours, 8 with near infrared.
pub fn wide_format(format: u8) -> u8 {
    match format {
        2 | 3 | 5 | 7 => 7,
        8 | 10 => 8,
        _ => 6,
    }
}

/// A record of `from` written as `to` (6, 7 or 8) into `out` (`to`'s record
/// with the same extra bytes): LAS 1.4 R15's legacy rules (docs/adr/0207 §4).
pub fn widen(from: &Layout, r: &[u8], to: &Layout, out: &mut [u8]) {
    out[..to.len].fill(0);
    out[0..12].copy_from_slice(&r[0..12]);
    put_u16(out, 12, from.intensity(r));
    let (ret, n) = from.returns(r);
    out[14] = (ret & 0x0F) | ((n & 0x0F) << 4);
    // Flags (synthetic, key-point, withheld, overlap), scanner channel 0, scan direction and edge.
    let dir_edge = if from.wide() {
        r[15] & 0xC0
    } else {
        r[14] & 0xC0
    };
    out[15] = from.flags(r) | dir_edge;
    out[16] = from.class(r);
    out[17] = from.user_data(r);
    put_i16(out, 18, from.scan_angle(r));
    put_u16(out, 20, from.source_id(r));
    put_f64(out, 22, from.gps_time(r));
    if let (Some(at), Some(c)) = (to.rgb, from.rgb(r)) {
        put_u16(out, at, c[0]);
        put_u16(out, at + 2, c[1]);
        put_u16(out, at + 4, c[2]);
    }
    if let (Some(at), Some(v)) = (to.nir, from.nir(r)) {
        put_u16(out, at, v);
    }
    let extra = from.extras(r);
    let start = standard_len(to.format);
    let n = extra.len().min(to.len.saturating_sub(start));
    out[start..start + n].copy_from_slice(&extra[..n]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rank_moves_by_the_half_away_from_zero() {
        assert_eq!(rank_to_angle(0), 0);
        assert_eq!(rank_to_angle(1), 167);
        assert_eq!(rank_to_angle(-1), -167);
        assert_eq!(rank_to_angle(3), 500);
        assert_eq!(rank_to_angle(90), 15000);
        assert_eq!(rank_to_angle(-90), -15000);
        assert_eq!(rank_to_angle(127), 21167);
    }

    #[test]
    fn a_legacy_record_widens() {
        let from = Layout::new(3, 34 + 2);
        let mut r = vec![0u8; 36];
        put_i32(&mut r, 0, 10);
        put_i32(&mut r, 4, -20);
        put_i32(&mut r, 8, 30);
        put_u16(&mut r, 12, 400);
        r[14] = 2 | (3 << 3) | 0x80; // return 2 of 3, edge of flight line
        r[15] = 6 | 0x20; // building, synthetic
        r[16] = (-12i8) as u8;
        r[17] = 9;
        put_u16(&mut r, 18, 77);
        put_f64(&mut r, 20, 123.5);
        put_u16(&mut r, 28, 1000);
        put_u16(&mut r, 30, 2000);
        put_u16(&mut r, 32, 3000);
        r[34] = 0xAB;
        r[35] = 0xCD;
        let to = Layout::new(7, 36 + 2);
        let mut out = vec![0xFFu8; 38];
        widen(&from, &r, &to, &mut out);
        assert_eq!(to.x(&out), 10);
        assert_eq!(to.y(&out), -20);
        assert_eq!(to.z(&out), 30);
        assert_eq!(to.intensity(&out), 400);
        assert_eq!(to.returns(&out), (2, 3));
        assert_eq!(to.class(&out), 6);
        assert_eq!(to.flags(&out), 1);
        assert_eq!(out[15] & 0xC0, 0x80);
        assert_eq!(to.user_data(&out), 9);
        assert_eq!(to.scan_angle(&out), -2000);
        assert_eq!(to.source_id(&out), 77);
        assert_eq!(to.gps_time(&out), 123.5);
        assert_eq!(to.rgb(&out), Some([1000, 2000, 3000]));
        assert_eq!(to.extras(&out), &[0xAB, 0xCD]);
    }
}
