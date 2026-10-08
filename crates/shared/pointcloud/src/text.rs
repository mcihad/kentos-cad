//! Text clouds (docs/adr/0207 §2): XYZ, PTS, TXT and CSV, read in two passes.
//!
//! The first pass ([`Scan`]) finds the columns (by the first data line's
//! count: 3 X Y Z, 4 with intensity, 6 with R G B, 7 with intensity and R G B,
//! PTS's order), the bounds and the most decimals the coordinates are written
//! with; the scale is 10⁻ᵈ (d at most 6) and the offset each axis's whole
//! least value, so every coordinate becomes its integer exactly, by its
//! decimal text, never through a float. Lines before the first data line that
//! are not numbers are a header (a PTS's single count too); later bad lines
//! are counted and said. The second pass ([`Parse`]) writes records of format
//! 6 (7 with colours): return 1 of 1, class 0.

use crate::bytes::{put_i32, put_u16};
use crate::record::standard_len;
use crate::{PcError, Result};

/// The most letters a line may have.
pub const MAX_LINE: usize = 4096;
/// The most decimals a coordinate's scale keeps.
pub const MAX_DECIMALS: u32 = 6;

/// What the columns hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Columns {
    Xyz,
    XyzI,
    XyzRgb,
    XyzIRgb,
}

impl Columns {
    fn of(n: usize) -> Option<Columns> {
        match n {
            3 => Some(Columns::Xyz),
            4 | 5 => Some(Columns::XyzI),
            6 => Some(Columns::XyzRgb),
            n if n >= 7 => Some(Columns::XyzIRgb),
            _ => None,
        }
    }

    pub fn intensity(self) -> bool {
        matches!(self, Columns::XyzI | Columns::XyzIRgb)
    }

    pub fn rgb(self) -> bool {
        matches!(self, Columns::XyzRgb | Columns::XyzIRgb)
    }

    /// The record format: 7 with colours, else 6.
    pub fn format(self) -> u8 {
        if self.rgb() { 7 } else { 6 }
    }

    fn rgb_at(self) -> usize {
        if self == Columns::XyzIRgb { 4 } else { 3 }
    }
}

/// A decimal number as written: sign, digits and how many are after the point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Decimal {
    neg: bool,
    digits: u128,
    places: u32,
}

/// A coordinate's text as a decimal (no exponent, at most 30 digits); none when it is not one.
fn decimal(s: &str) -> Option<Decimal> {
    let b = s.as_bytes();
    let (neg, body) = match b.first()? {
        b'-' => (true, &b[1..]),
        b'+' => (false, &b[1..]),
        _ => (false, b),
    };
    if body.is_empty() || body.len() > 31 {
        return None;
    }
    let (mut digits, mut places, mut seen_point, mut any) = (0u128, 0u32, false, false);
    for &c in body {
        match c {
            b'0'..=b'9' => {
                digits = digits * 10 + u128::from(c - b'0');
                any = true;
                if seen_point {
                    places += 1;
                }
            }
            b'.' if !seen_point => seen_point = true,
            _ => return None,
        }
    }
    any.then_some(Decimal {
        neg,
        digits,
        places,
    })
}

impl Decimal {
    fn value(self) -> f64 {
        let v = self.digits as f64 / 10f64.powi(self.places as i32);
        if self.neg { -v } else { v }
    }

    /// Its value in steps of 10⁻ᵈ, the half away from zero.
    fn steps(self, d: u32) -> i128 {
        let v = if d >= self.places {
            (self.digits as i128) * 10i128.pow(d - self.places)
        } else {
            let div = 10i128.pow(self.places - d);
            let (q, r) = ((self.digits as i128) / div, (self.digits as i128) % div);
            if 2 * r >= div { q + 1 } else { q }
        };
        if self.neg { -v } else { v }
    }
}

/// Splits a line into its fields: spaces, tabs, commas and semicolons separate.
fn fields(line: &str) -> impl Iterator<Item = &str> {
    line.split(|c: char| c.is_whitespace() || c == ',' || c == ';')
        .filter(|s| !s.is_empty())
}

/// What the first pass found.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub columns: Columns,
    pub count: u64,
    /// `[x₁, y₁, z₁, x₂, y₂, z₂]`.
    pub bounds: [f64; 6],
    pub decimals: u32,
    pub scale: [f64; 3],
    pub offset: [f64; 3],
    /// Intensities written as fractions (0–1): times 65535.
    pub fraction_intensity: bool,
    /// Colours written as 0–255: times 257.
    pub byte_colours: bool,
    /// Lines that were not points.
    pub skipped: u64,
    /// The first of them (its number and why), for the window.
    pub first_skip: Option<(u64, String)>,
    /// The decimals were cut so that the coordinates fit the records' 32 bits.
    pub coarsened: bool,
}

/// Lines out of bytes handed over in pieces.
#[derive(Clone, Debug, Default)]
struct Lines {
    partial: Vec<u8>,
    number: u64,
}

impl Lines {
    /// Calls `f` with each whole line of `bytes` (and what was left from before).
    fn feed(&mut self, bytes: &[u8], mut f: impl FnMut(u64, &str) -> Result<()>) -> Result<()> {
        let mut start = 0;
        for (i, &c) in bytes.iter().enumerate() {
            if c == b'\n' {
                self.partial.extend_from_slice(&bytes[start..i]);
                self.number += 1;
                let line = std::mem::take(&mut self.partial);
                f(self.number, decode(&line)?)?;
                start = i + 1;
            }
        }
        self.partial.extend_from_slice(&bytes[start..]);
        if self.partial.len() > MAX_LINE {
            return Err(PcError::new(format!(
                "{}. satır {MAX_LINE} harften uzun; metin bulutu değil gibi.",
                self.number + 1
            )));
        }
        Ok(())
    }

    fn finish(&mut self, f: impl FnMut(u64, &str) -> Result<()>) -> Result<()> {
        if self.partial.is_empty() {
            return Ok(());
        }
        let mut f = f;
        self.number += 1;
        let line = std::mem::take(&mut self.partial);
        f(self.number, decode(&line)?)
    }
}

fn decode(line: &[u8]) -> Result<&str> {
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    let line = line.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(line);
    std::str::from_utf8(line).map_err(|_| PcError::new("Metin bulutu UTF-8 değil."))
}

/// The first pass.
#[derive(Clone, Debug, Default)]
pub struct Scan {
    lines: Lines,
    columns: Option<Columns>,
    count: u64,
    least: [Option<Decimal>; 3],
    most: [Option<Decimal>; 3],
    decimals: u32,
    max_intensity: f64,
    max_colour: f64,
    skipped: u64,
    first_skip: Option<(u64, String)>,
}

fn less(a: Decimal, b: Decimal) -> bool {
    a.value() < b.value()
}

impl Scan {
    pub fn new() -> Scan {
        Scan::default()
    }

    pub fn feed(&mut self, bytes: &[u8]) -> Result<()> {
        let mut lines = std::mem::take(&mut self.lines);
        let r = lines.feed(bytes, |n, l| self.line(n, l));
        self.lines = lines;
        r
    }

    fn skip(&mut self, n: u64, why: &str) {
        self.skipped += 1;
        if self.first_skip.is_none() {
            self.first_skip = Some((n, why.to_owned()));
        }
    }

    fn line(&mut self, n: u64, line: &str) -> Result<()> {
        let f: Vec<&str> = fields(line).collect();
        if f.is_empty() {
            return Ok(());
        }
        let xyz: Vec<Option<Decimal>> = f.iter().take(3).map(|s| decimal(s)).collect();
        let numbers = f.len() >= 3 && xyz.iter().all(Option::is_some);
        let Some(columns) = self.columns else {
            if numbers {
                self.columns = Columns::of(f.len());
            } else {
                // A header, or a PTS's count: before the first point.
                return Ok(());
            }
            return self.point(n, &f);
        };
        if !numbers
            || f.len()
                < match columns {
                    Columns::Xyz => 3,
                    Columns::XyzI => 4,
                    Columns::XyzRgb => 6,
                    Columns::XyzIRgb => 7,
                }
        {
            self.skip(n, "sayı değil ya da sütunu eksik");
            return Ok(());
        }
        self.point(n, &f)
    }

    fn point(&mut self, n: u64, f: &[&str]) -> Result<()> {
        let columns = self.columns.unwrap_or(Columns::Xyz);
        let mut v = [Decimal {
            neg: false,
            digits: 0,
            places: 0,
        }; 3];
        for k in 0..3 {
            match decimal(f[k]) {
                Some(d) => v[k] = d,
                None => {
                    self.skip(n, "koordinat sayı değil");
                    return Ok(());
                }
            }
        }
        let extra = |s: &str| s.parse::<f64>().ok().filter(|x| x.is_finite());
        if columns.intensity() {
            match extra(f[3]) {
                Some(i) => self.max_intensity = self.max_intensity.max(i),
                None => {
                    self.skip(n, "yoğunluk sayı değil");
                    return Ok(());
                }
            }
        }
        if columns.rgb() {
            let at = columns.rgb_at();
            for s in &f[at..at + 3] {
                match extra(s) {
                    Some(c) => self.max_colour = self.max_colour.max(c),
                    None => {
                        self.skip(n, "renk sayı değil");
                        return Ok(());
                    }
                }
            }
        }
        for (k, &d) in v.iter().take(3).enumerate() {
            self.decimals = self.decimals.max(d.places);
            if self.least[k].is_none_or(|m| less(d, m)) {
                self.least[k] = Some(d);
            }
            if self.most[k].is_none_or(|m| less(m, d)) {
                self.most[k] = Some(d);
            }
        }
        self.count += 1;
        Ok(())
    }

    /// What the file holds; why not when it holds no point.
    pub fn finish(mut self) -> Result<Plan> {
        let mut lines = std::mem::take(&mut self.lines);
        lines.finish(|n, l| self.line(n, l))?;
        let (Some(columns), true) = (self.columns, self.count > 0) else {
            return Err(PcError::new(
                "Dosyada nokta yok: en az üç sayılık (X Y Z) satır bulunmadı.",
            ));
        };
        let mut d = self.decimals.min(MAX_DECIMALS);
        let mut coarsened = self.decimals > MAX_DECIMALS;
        let least: Vec<Decimal> = self.least.iter().map(|v| v.unwrap_or(ZERO)).collect();
        let most: Vec<Decimal> = self.most.iter().map(|v| v.unwrap_or(ZERO)).collect();
        // The offset: each axis's least value's whole part, toward minus infinity.
        let offset: Vec<i128> = least
            .iter()
            .map(|m| {
                let (a, p) = (m.digits as i128, 10i128.pow(m.places));
                if m.neg { -((a + p - 1) / p) } else { a / p }
            })
            .collect();
        loop {
            let fits = (0..3).all(|k| {
                let span = most[k].steps(d) - offset[k] * 10i128.pow(d);
                span <= i128::from(i32::MAX) && least[k].steps(d) - offset[k] * 10i128.pow(d) >= 0
            });
            if fits || d == 0 {
                break;
            }
            d -= 1;
            coarsened = true;
        }
        let scale = 10f64.powi(-(d as i32));
        Ok(Plan {
            columns,
            count: self.count,
            bounds: [
                least[0].value(),
                least[1].value(),
                least[2].value(),
                most[0].value(),
                most[1].value(),
                most[2].value(),
            ],
            decimals: d,
            scale: [scale; 3],
            offset: [offset[0] as f64, offset[1] as f64, offset[2] as f64],
            fraction_intensity: columns.intensity() && self.max_intensity <= 1.0,
            byte_colours: columns.rgb() && self.max_colour <= 255.0,
            skipped: self.skipped,
            first_skip: self.first_skip,
            coarsened,
        })
    }
}

const ZERO: Decimal = Decimal {
    neg: false,
    digits: 0,
    places: 0,
};

/// The second pass: records of the plan's format.
#[derive(Clone, Debug)]
pub struct Parse {
    plan: Plan,
    lines: Lines,
    started: bool,
    record: usize,
}

impl Parse {
    pub fn new(plan: Plan) -> Parse {
        let record = standard_len(plan.columns.format());
        Parse {
            plan,
            lines: Lines::default(),
            started: false,
            record,
        }
    }

    pub fn record_len(&self) -> usize {
        self.record
    }

    /// The records of the whole lines in `bytes`, appended to `out`.
    pub fn feed(&mut self, bytes: &[u8], out: &mut Vec<u8>) -> Result<()> {
        let mut lines = std::mem::take(&mut self.lines);
        let r = lines.feed(bytes, |_, l| {
            self.line(l, out);
            Ok(())
        });
        self.lines = lines;
        r
    }

    pub fn finish(&mut self, out: &mut Vec<u8>) -> Result<()> {
        let mut lines = std::mem::take(&mut self.lines);
        lines.finish(|_, l| {
            self.line(l, out);
            Ok(())
        })
    }

    fn line(&mut self, line: &str, out: &mut Vec<u8>) {
        let f: Vec<&str> = fields(line).collect();
        let columns = self.plan.columns;
        let need = match columns {
            Columns::Xyz => 3,
            Columns::XyzI => 4,
            Columns::XyzRgb => 6,
            Columns::XyzIRgb => 7,
        };
        if f.len() < 3 {
            return;
        }
        let xyz: Option<Vec<Decimal>> = f.iter().take(3).map(|s| decimal(s)).collect();
        let Some(xyz) = xyz else {
            return;
        };
        if !self.started {
            self.started = true;
        }
        if f.len() < need {
            return;
        }
        let extra = |s: &str| s.parse::<f64>().ok().filter(|x| x.is_finite());
        let intensity = if columns.intensity() {
            match extra(f[3]) {
                Some(i) => Some(i),
                None => return,
            }
        } else {
            None
        };
        let rgb = if columns.rgb() {
            let at = columns.rgb_at();
            let c: Option<Vec<f64>> = f[at..at + 3].iter().map(|s| extra(s)).collect();
            match c {
                Some(c) => Some(c),
                None => return,
            }
        } else {
            None
        };
        let d = self.plan.decimals;
        let mut r = vec![0u8; self.record];
        let mut ints = [0i32; 3];
        for k in 0..3 {
            let v = xyz[k].steps(d) - (self.plan.offset[k] as i128) * 10i128.pow(d);
            ints[k] = v.clamp(i128::from(i32::MIN), i128::from(i32::MAX)) as i32;
        }
        put_i32(&mut r, 0, ints[0]);
        put_i32(&mut r, 4, ints[1]);
        put_i32(&mut r, 8, ints[2]);
        if let Some(i) = intensity {
            let v = if self.plan.fraction_intensity {
                i * 65535.0
            } else {
                i
            };
            put_u16(&mut r, 12, (v + 0.5).floor().clamp(0.0, 65535.0) as u16);
        }
        r[14] = 1 | (1 << 4);
        if let Some(c) = rgb {
            for (k, v) in c.iter().enumerate() {
                let v = if self.plan.byte_colours {
                    v * 257.0
                } else {
                    *v
                };
                put_u16(
                    &mut r,
                    30 + 2 * k,
                    (v + 0.5).floor().clamp(0.0, 65535.0) as u16,
                );
            }
        }
        out.extend_from_slice(&r);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimals_become_their_integers_exactly() {
        let d = decimal("500013.380").unwrap();
        assert_eq!(d.steps(3), 500_013_380);
        assert_eq!(d.steps(1), 5_000_134);
        assert_eq!(decimal("-0.25").unwrap().steps(1), -3);
        assert_eq!(decimal("-0.24").unwrap().steps(1), -2);
        assert!(decimal("1e5").is_none());
        assert!(decimal("x").is_none());
    }

    #[test]
    fn a_pts_file_reads_by_its_columns() {
        let text = "3\n1.5 2.25 3 100 255 0 10\n-1 0.5 2 200 0 128 255\n0 0 0 0 1 2 3\n";
        let mut s = Scan::new();
        s.feed(text.as_bytes()).unwrap();
        let p = s.finish().unwrap();
        assert_eq!(p.columns, Columns::XyzIRgb);
        assert_eq!(p.count, 3);
        assert_eq!(p.decimals, 2);
        assert_eq!(p.offset, [-1.0, 0.0, 0.0]);
        assert_eq!(p.bounds, [-1.0, 0.0, 0.0, 1.5, 2.25, 3.0]);
        assert!(p.byte_colours && !p.fraction_intensity);
        let mut parse = Parse::new(p);
        let mut out = Vec::new();
        parse.feed(text.as_bytes(), &mut out).unwrap();
        parse.finish(&mut out).unwrap();
        assert_eq!(out.len(), 3 * 36);
        let l = crate::record::Layout::new(7, 36);
        let r = &out[..36];
        assert_eq!((l.x(r), l.y(r), l.z(r)), (250, 225, 300));
        assert_eq!(l.intensity(r), 100);
        assert_eq!(l.rgb(r), Some([255 * 257, 0, 10 * 257]));
        assert_eq!(l.returns(r), (1, 1));
    }

    #[test]
    fn headers_and_bad_lines() {
        let text = "X;Y;Z\n1;2;3\noops\n4;5;6\n";
        let mut s = Scan::new();
        s.feed(text.as_bytes()).unwrap();
        let p = s.finish().unwrap();
        assert_eq!(p.count, 2);
        assert_eq!(p.skipped, 1);
        assert_eq!(p.first_skip.as_ref().map(|s| s.0), Some(3));
        let mut empty = Scan::new();
        empty.feed(b"just words\n").unwrap();
        assert!(empty.finish().is_err());
    }
}
