//! Renkli kabartma's colour table (docs/adr/0231 §5): a ramp of ADR 0204's
//! spread over two values, or the user's lines `value #RRGGBB[AA]` (as
//! gdaldem color-relief's text); a value is coloured linearly between its
//! two lines, or by the nearer line, its ends held.

use libm::floor;

/// How a value between two lines is coloured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interp {
    Linear,
    Nearest,
}

/// A colour table: values ascending, one RGBA each.
#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    values: Vec<f64>,
    colors: Vec<[u8; 4]>,
}

fn hex(s: &str) -> Option<[u8; 4]> {
    let s = s.strip_prefix('#')?;
    let n = s.len();
    if (n != 6 && n != 8) || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |k: usize| u8::from_str_radix(s.get(k..k + 2)?, 16).ok();
    Some([
        byte(0)?,
        byte(2)?,
        byte(4)?,
        if n == 8 { byte(6)? } else { 255 },
    ])
}

impl Table {
    /// The user's lines: `value #RRGGBB` or `value #RRGGBBAA`, one a line
    /// (empty lines skipped); ascending by value, the first of equal values
    /// kept; at least two.
    pub fn parse(text: &str) -> Result<Table, String> {
        let mut rows: Vec<(f64, [u8; 4])> = Vec::new();
        // Rows by line or by `;` (a one-line field writes “100 #2E7D32; 500 #FFF59D”).
        for (k, line) in text.split(['\n', ';']).enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let mut parts = line.split_whitespace();
            let (Some(v), Some(c), None) = (parts.next(), parts.next(), parts.next()) else {
                return Err(format!(
                    "Renk tablosunun {}. satırı “değer #RRGGBB” değil: “{line}”.",
                    k + 1
                ));
            };
            let value: f64 = v
                .parse()
                .ok()
                .filter(|x: &f64| x.is_finite())
                .ok_or_else(|| {
                    format!(
                        "Renk tablosunun {}. satırındaki değer sayı değil: “{v}”.",
                        k + 1
                    )
                })?;
            let color = hex(c).ok_or_else(|| {
                format!(
                    "Renk tablosunun {}. satırındaki renk #RRGGBB ya da #RRGGBBAA değil: “{c}”.",
                    k + 1
                )
            })?;
            rows.push((value, color));
        }
        // Ascending; of equal values the first written stays (a stable sort keeps their order).
        rows.sort_by(|a, b| a.0.total_cmp(&b.0));
        rows.dedup_by(|b, a| a.0 == b.0);
        if rows.len() < 2 {
            return Err("Renk tablosunda en az iki satır olmalı: “değer #RRGGBB”.".into());
        }
        Ok(Table {
            values: rows.iter().map(|r| r.0).collect(),
            colors: rows.iter().map(|r| r.1).collect(),
        })
    }

    /// A ramp's stops spread evenly from `lo` to `hi` (reversed when
    /// `invert`); all one colour, the first, when `hi` is not above `lo`.
    pub fn ramp(stops: &[[u8; 3]], invert: bool, lo: f64, hi: f64) -> Table {
        let mut colors: Vec<[u8; 4]> = stops.iter().map(|c| [c[0], c[1], c[2], 255]).collect();
        if invert {
            colors.reverse();
        }
        if hi.partial_cmp(&lo) != Some(std::cmp::Ordering::Greater) || colors.len() < 2 {
            let first = colors.first().copied().unwrap_or([0, 0, 0, 255]);
            return Table {
                values: vec![lo, lo],
                colors: vec![first, first],
            };
        }
        let n = colors.len() - 1;
        let values = (0..=n)
            .map(|k| lo + (hi - lo) * k as f64 / n as f64)
            .collect();
        Table { values, colors }
    }

    /// The colour of `v` (not NaN).
    #[inline]
    pub fn color(&self, v: f64, interp: Interp) -> [u8; 4] {
        let last = self.values.len() - 1;
        if v <= self.values[0] {
            return self.colors[0];
        }
        if v >= self.values[last] {
            return self.colors[last];
        }
        // The line at or below `v`: values[k] ≤ v < values[k + 1].
        let k = self.values.partition_point(|&x| x <= v) - 1;
        let (a, b) = (self.values[k], self.values[k + 1]);
        let (ca, cb) = (self.colors[k], self.colors[k + 1]);
        match interp {
            Interp::Nearest => {
                if v - a <= b - v {
                    ca
                } else {
                    cb
                }
            }
            Interp::Linear => {
                let t = (v - a) / (b - a);
                let mut out = [0u8; 4];
                for (c, o) in out.iter_mut().enumerate() {
                    let x = f64::from(ca[c]) + t * (f64::from(cb[c]) - f64::from(ca[c]));
                    *o = floor(x + 0.5).clamp(0.0, 255.0) as u8;
                }
                out
            }
        }
    }
}
