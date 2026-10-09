//! Komşuluk istatistiği (docs/adr/0233 §12): a statistic of each cell's
//! window: a rectangle (odd sides), a circle (cells whose centres are within
//! r cells) or a ring (r₁ < distance ≤ r₂). Cells past the raster are not in
//! the window; cells without a value are left out (Değersizleri yok say),
//! or make the cell empty.
//!
//! - Sums (Toplam, Ortalama, Standart sapma) slide in double-double: a
//!   rectangle's down its columns (begun anew every 32 rows) and then along
//!   its rows (anew every 256 columns), a circle's or a ring's along the
//!   rows of each window row. Each cell's sums are the same whatever the
//!   threads: the restarts are at fixed rows and columns.
//! - En küçük and En büyük by van Herk and Gil-Werman's running blocks
//!   (along the rows, then down the columns; a circle a window row at a time).
//! - Ortanca, Çoğunluk, Azınlık and Çeşit from the window's values.

use crate::dd::Dd;
use crate::par;
use crate::stats::{Stat, order_stat, std_of};

/// Rows begun anew when sums slide down the columns.
pub const ROW_RESTART: u32 = 32;
/// Columns begun anew when sums slide along a row.
pub const COLUMN_RESTART: i64 = 256;

/// A window's shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// Half widths: (2·hx + 1) × (2·hy + 1) cells.
    Rect { hx: u32, hy: u32 },
    /// Cells with di² + dj² ≤ r².
    Circle { r: u32 },
    /// Cells with r₁² < di² + dj² ≤ r₂².
    Ring { inner: u32, outer: u32 },
}

/// A window: each row's offset and its spans of column offsets (inclusive).
#[derive(Clone, Debug, PartialEq)]
pub struct Window {
    pub shape: Shape,
    rows: Vec<(i64, Vec<(i64, i64)>)>,
    halo: (u32, u32),
}

/// The largest h with h² + dy² ≤ r² (none when dy² > r²).
fn half(r: u32, dy: i64) -> Option<i64> {
    let (r2, d2) = (i64::from(r) * i64::from(r), dy * dy);
    if d2 > r2 {
        return None;
    }
    let mut h = libm::sqrt((r2 - d2) as f64) as i64;
    while h * h + d2 > r2 {
        h -= 1;
    }
    while (h + 1) * (h + 1) + d2 <= r2 {
        h += 1;
    }
    Some(h)
}

impl Window {
    /// A rectangle `width` × `height` cells (odd, 1–255).
    pub fn rect(width: u32, height: u32) -> Result<Window, String> {
        let ok = |s: u32| s % 2 == 1 && (1..=255).contains(&s);
        if !ok(width) || !ok(height) {
            return Err(
                "Dikdörtgenin genişliği ve yüksekliği 1 ile 255 arasında tek sayı olmalı.".into(),
            );
        }
        let (hx, hy) = (width / 2, height / 2);
        let span = vec![(-i64::from(hx), i64::from(hx))];
        Ok(Window {
            shape: Shape::Rect { hx, hy },
            rows: (-i64::from(hy)..=i64::from(hy))
                .map(|dy| (dy, span.clone()))
                .collect(),
            halo: (hx, hy),
        })
    }

    /// A circle of radius `r` cells (1–127).
    pub fn circle(r: u32) -> Result<Window, String> {
        if !(1..=127).contains(&r) {
            return Err("Dairenin yarıçapı 1 ile 127 hücre arasında olmalı.".into());
        }
        let rows = (-i64::from(r)..=i64::from(r))
            .filter_map(|dy| half(r, dy).map(|h| (dy, vec![(-h, h)])))
            .collect();
        Ok(Window {
            shape: Shape::Circle { r },
            rows,
            halo: (r, r),
        })
    }

    /// A ring between radii `inner` < `outer` cells (outer at most 127).
    pub fn ring(inner: u32, outer: u32) -> Result<Window, String> {
        if !(1..=127).contains(&outer) || inner >= outer {
            return Err("Halkanın iç yarıçapı dış yarıçapından küçük, dış yarıçapı 1 ile 127 hücre arasında olmalı.".into());
        }
        let mut rows = Vec::new();
        for dy in -i64::from(outer)..=i64::from(outer) {
            let Some(h2) = half(outer, dy) else {
                continue;
            };
            let spans = match half(inner, dy) {
                Some(h1) if h1 < h2 => vec![(-h2, -h1 - 1), (h1 + 1, h2)],
                Some(_) => continue,
                None => vec![(-h2, h2)],
            };
            rows.push((dy, spans));
        }
        Ok(Window {
            shape: Shape::Ring { inner, outer },
            rows,
            halo: (outer, outer),
        })
    }

    /// Cells it reaches each side, and above and below.
    pub fn halo(&self) -> (u32, u32) {
        self.halo
    }

    /// Its cells' offsets (dx, dy), row by row.
    pub fn offsets(&self) -> Vec<(i64, i64)> {
        let mut out = Vec::new();
        for (dy, spans) in &self.rows {
            for &(lo, hi) in spans {
                out.extend((lo..=hi).map(|dx| (dx, *dy)));
            }
        }
        out
    }
}

/// The raster's cells round a block: columns `x0..x0 + w`, rows `y0..y0 + h`
/// (the raster's own cells, nothing past its edges), NaN where no value.
#[derive(Clone, Copy, Debug)]
pub struct Region<'v> {
    pub x0: i64,
    pub y0: i64,
    pub w: usize,
    pub h: usize,
    pub v: &'v [f64],
}

impl Region<'_> {
    /// The cell (x, y): none past the region (past the raster), NaN without a value.
    #[inline]
    fn get(&self, x: i64, y: i64) -> Option<f64> {
        let (dx, dy) = (x - self.x0, y - self.y0);
        if dx < 0 || dy < 0 || dx >= self.w as i64 || dy >= self.h as i64 {
            return None;
        }
        Some(self.v[dy as usize * self.w + dx as usize])
    }
}

/// A window's sums: of the values and their squares, how many values and how many cells without one.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Sums {
    s1: Dd,
    s2: Dd,
    n: u32,
    m: u32,
}

impl Sums {
    #[inline]
    fn add(&mut self, v: Option<f64>) {
        match v {
            None => {}
            Some(x) if x.is_nan() => self.m += 1,
            Some(x) => {
                self.n += 1;
                self.s1 = self.s1.add_f64(x);
                self.s2 = self.s2 + Dd::square(x);
            }
        }
    }

    #[inline]
    fn sub(&mut self, v: Option<f64>) {
        match v {
            None => {}
            Some(x) if x.is_nan() => self.m -= 1,
            Some(x) => {
                self.n -= 1;
                self.s1 = self.s1.add_f64(-x);
                self.s2 = self.s2 - Dd::square(x);
            }
        }
    }

    #[inline]
    fn plus(&mut self, o: &Sums) {
        self.s1 = self.s1 + o.s1;
        self.s2 = self.s2 + o.s2;
        self.n += o.n;
        self.m += o.m;
    }

    #[inline]
    fn minus(&mut self, o: &Sums) {
        self.s1 = self.s1 - o.s1;
        self.s2 = self.s2 - o.s2;
        self.n -= o.n;
        self.m -= o.m;
    }
}

/// A statistic and how cells without a value count.
#[derive(Clone, Debug)]
pub struct Focal {
    pub window: Window,
    pub stat: Stat,
    /// Değersizleri yok say.
    pub ignore: bool,
}

impl Focal {
    pub fn new(window: Window, stat: Stat, ignore: bool) -> Result<Focal, String> {
        if matches!(stat, Stat::Count | Stat::Area) {
            return Err("Komşuluk istatistiği bu istatistiği vermez.".into());
        }
        Ok(Focal {
            window,
            stat,
            ignore,
        })
    }

    fn of_sums(&self, s: &Sums) -> f64 {
        if s.n == 0 || (!self.ignore && s.m > 0) {
            return f64::NAN;
        }
        match self.stat {
            Stat::Sum => s.s1.value() + 0.0,
            Stat::Mean => s.s1.div_f64(f64::from(s.n)).value() + 0.0,
            Stat::Std => std_of(u64::from(s.n), s.s1, s.s2).unwrap_or(f64::NAN),
            _ => f64::NAN,
        }
    }

    /// The result's rows `j0..j0 + n`, columns `c0..c1`, from `region` (the
    /// raster's cells round them; the raster `size` cells), rows on `threads`.
    pub fn block(
        &self,
        region: &Region<'_>,
        size: (u32, u32),
        (c0, c1, j0, n): (u32, u32, u32, u32),
        threads: usize,
        out: &mut [f64],
    ) {
        let bw = (c1 - c0) as usize;
        match (self.stat, self.window.shape) {
            (Stat::Sum | Stat::Mean | Stat::Std, Shape::Rect { hx, hy }) => {
                self.rect_sums(region, size, (c0, c1, j0, n), (hx, hy), threads, out);
            }
            (Stat::Sum | Stat::Mean | Stat::Std, _) => {
                par::rows(threads, out, bw, &|first, chunk: &mut [f64]| {
                    for (k, row) in chunk.chunks_mut(bw).enumerate() {
                        self.span_sums_row(region, size, (c0, c1), j0 + (first + k) as u32, row);
                    }
                });
            }
            (Stat::Min | Stat::Max | Stat::Range, _) => {
                self.extremes(region, size, (c0, c1, j0, n), threads, out);
            }
            _ => {
                let offsets = self.window.offsets();
                par::rows(threads, out, bw, &|first, chunk: &mut [f64]| {
                    let mut values = Vec::with_capacity(offsets.len());
                    for (k, row) in chunk.chunks_mut(bw).enumerate() {
                        let j = i64::from(j0) + (first + k) as i64;
                        for (i, o) in row.iter_mut().enumerate() {
                            let x = i64::from(c0) + i as i64;
                            values.clear();
                            let mut missing = false;
                            for &(dx, dy) in &offsets {
                                match region.get(x + dx, j + dy) {
                                    None => {}
                                    Some(v) if v.is_nan() => missing = true,
                                    Some(v) => values.push(v),
                                }
                            }
                            *o = if missing && !self.ignore {
                                f64::NAN
                            } else {
                                order_stat(&mut values, self.stat).unwrap_or(f64::NAN)
                            };
                        }
                    }
                });
            }
        }
    }

    /// A rectangle's sums: down the columns, then along the rows.
    fn rect_sums(
        &self,
        region: &Region<'_>,
        size: (u32, u32),
        (c0, c1, j0, n): (u32, u32, u32, u32),
        (hx, hy): (u32, u32),
        threads: usize,
        out: &mut [f64],
    ) {
        let bw = (c1 - c0) as usize;
        let (hx, hy) = (i64::from(hx), i64::from(hy));
        let width = i64::from(size.0);
        // The columns the rows' windows reach.
        let xa = (i64::from(c0) - hx).max(0);
        let xb = (i64::from(c1) + hx).min(width);
        // Groups of rows begun anew at the fixed restarts.
        let mut groups: Vec<(u32, u32)> = Vec::new();
        let mut j = j0;
        while j < j0 + n {
            let end = ((j / ROW_RESTART + 1) * ROW_RESTART).min(j0 + n);
            groups.push((j, end));
            j = end;
        }
        let rows: Vec<Vec<f64>> = par::map(threads, &groups, &|&(ja, jb)| {
            let mut cols = vec![Sums::default(); (xb - xa).max(0) as usize];
            let mut values = vec![0.0; (jb - ja) as usize * bw];
            for jj in ja..jb {
                let jy = i64::from(jj);
                if jj == ja {
                    for (k, c) in cols.iter_mut().enumerate() {
                        let x = xa + k as i64;
                        for y in jy - hy..=jy + hy {
                            c.add(region.get(x, y));
                        }
                    }
                } else {
                    for (k, c) in cols.iter_mut().enumerate() {
                        let x = xa + k as i64;
                        c.add(region.get(x, jy + hy));
                        c.sub(region.get(x, jy - hy - 1));
                    }
                }
                let row = &mut values[(jj - ja) as usize * bw..(jj - ja + 1) as usize * bw];
                let col = |x: i64| -> Option<&Sums> {
                    (x >= xa && x < xb).then(|| &cols[(x - xa) as usize])
                };
                let mut s = Sums::default();
                for (i, o) in row.iter_mut().enumerate() {
                    let x = i64::from(c0) + i as i64;
                    if i == 0 || x % COLUMN_RESTART == 0 {
                        s = Sums::default();
                        for xx in x - hx..=x + hx {
                            if let Some(c) = col(xx) {
                                s.plus(c);
                            }
                        }
                    } else {
                        if let Some(c) = col(x + hx) {
                            s.plus(c);
                        }
                        if let Some(c) = col(x - hx - 1) {
                            s.minus(c);
                        }
                    }
                    *o = self.of_sums(&s);
                }
            }
            values
        });
        let mut at = 0;
        for r in rows {
            out[at..at + r.len()].copy_from_slice(&r);
            at += r.len();
        }
    }

    /// A circle's or a ring's sums on row `j`: each window row's spans slide along it.
    fn span_sums_row(
        &self,
        region: &Region<'_>,
        size: (u32, u32),
        (c0, c1): (u32, u32),
        j: u32,
        out: &mut [f64],
    ) {
        let bw = (c1 - c0) as usize;
        let mut sums = vec![Sums::default(); bw];
        let height = i64::from(size.1);
        for (dy, spans) in &self.window.rows {
            let y = i64::from(j) + dy;
            if y < 0 || y >= height {
                continue;
            }
            for &(lo, hi) in spans {
                let mut s = Sums::default();
                for (i, total) in sums.iter_mut().enumerate() {
                    let x = i64::from(c0) + i as i64;
                    if i == 0 || x % COLUMN_RESTART == 0 {
                        s = Sums::default();
                        for xx in x + lo..=x + hi {
                            s.add(region.get(xx, y));
                        }
                    } else {
                        s.add(region.get(x + hi, y));
                        s.sub(region.get(x + lo - 1, y));
                    }
                    total.plus(&s);
                }
            }
        }
        for (o, s) in out.iter_mut().zip(&sums) {
            *o = self.of_sums(s);
        }
    }

    /// En küçük, En büyük and Aralık: running blocks along each window row's
    /// spans (a rectangle: along the rows, then down the columns), and the
    /// windows' counts for the cells without a value.
    fn extremes(
        &self,
        region: &Region<'_>,
        size: (u32, u32),
        (c0, c1, j0, n): (u32, u32, u32, u32),
        threads: usize,
        out: &mut [f64],
    ) {
        let bw = (c1 - c0) as usize;
        let height = i64::from(size.1);
        // A window with a cell without a value empties the cell unless they are left out.
        let lack = if self.ignore {
            Vec::new()
        } else {
            self.lacking(region, size, (c0, c1, j0, n), threads)
        };
        let lacks = |k: usize| lack.get(k).copied().unwrap_or(false);
        match self.window.shape {
            Shape::Rect { hx, hy } => {
                let (hx, hy) = (i64::from(hx), i64::from(hy));
                // The rows the windows reach, their running extremes along the row.
                let ya = (i64::from(j0) - hy).max(0);
                let yb = (i64::from(j0 + n) + hy).min(height);
                let rows_n = (yb - ya).max(0) as usize;
                let mut lo_row = vec![f64::INFINITY; rows_n * bw];
                let mut hi_row = vec![f64::NEG_INFINITY; rows_n * bw];
                {
                    let mut both: Vec<(&mut [f64], &mut [f64])> =
                        lo_row.chunks_mut(bw).zip(hi_row.chunks_mut(bw)).collect();
                    let items: Vec<usize> = (0..rows_n).collect();
                    let done: Vec<(Vec<f64>, Vec<f64>)> = par::map(threads, &items, &|&k| {
                        let y = ya + k as i64;
                        let line: Vec<f64> = (i64::from(c0) - hx..i64::from(c1) + hx)
                            .map(|x| region.get(x, y).unwrap_or(f64::NAN))
                            .collect();
                        (
                            running(&line, (2 * hx + 1) as usize, true),
                            running(&line, (2 * hx + 1) as usize, false),
                        )
                    });
                    for ((lo, hi), (a, b)) in both.iter_mut().zip(done) {
                        lo.copy_from_slice(&a[..bw]);
                        hi.copy_from_slice(&b[..bw]);
                    }
                }
                // Down the columns.
                let cols: Vec<usize> = (0..bw).collect();
                let k = (2 * hy + 1) as usize;
                let done: Vec<(Vec<f64>, Vec<f64>)> = par::map(threads, &cols, &|&i| {
                    // The column's rows from j0 − hy to j0 + n + hy, those past the raster ±∞.
                    let pick = |rows: &[f64], init: f64| -> Vec<f64> {
                        (i64::from(j0) - hy..i64::from(j0 + n) + hy)
                            .map(|y| {
                                if y >= ya && y < yb {
                                    rows[(y - ya) as usize * bw + i]
                                } else {
                                    init
                                }
                            })
                            .collect()
                    };
                    let lo = running(&pick(&lo_row, f64::INFINITY), k, true);
                    let hi = running(&pick(&hi_row, f64::NEG_INFINITY), k, false);
                    (lo[..n as usize].to_vec(), hi[..n as usize].to_vec())
                });
                for (i, (lo, hi)) in done.iter().enumerate() {
                    for r in 0..n as usize {
                        out[r * bw + i] = self.extreme(lo[r], hi[r], lacks(r * bw + i));
                    }
                }
            }
            _ => {
                let (want_lo, want_hi) = (self.stat != Stat::Max, self.stat != Stat::Min);
                par::rows(threads, out, bw, &|first, chunk: &mut [f64]| {
                    // A thread's buffers, kept from row to row.
                    let mut lo = vec![f64::INFINITY; bw];
                    let mut hi = vec![f64::NEG_INFINITY; bw];
                    let (mut g, mut h) = (Vec::new(), Vec::new());
                    for (k, row) in chunk.chunks_mut(bw).enumerate() {
                        let j = i64::from(j0) + (first + k) as i64;
                        lo.fill(f64::INFINITY);
                        hi.fill(f64::NEG_INFINITY);
                        for (dy, spans) in &self.window.rows {
                            let y = j + dy;
                            if y < 0 || y >= height {
                                continue;
                            }
                            for &(a, b) in spans {
                                let x0 = i64::from(c0) + a;
                                let len = bw + (b - a) as usize;
                                let at =
                                    |i: usize| region.get(x0 + i as i64, y).unwrap_or(f64::NAN);
                                let w = (b - a + 1) as usize;
                                if want_lo {
                                    running_fold(&at, len, w, true, (&mut g, &mut h), &mut lo);
                                }
                                if want_hi {
                                    running_fold(&at, len, w, false, (&mut g, &mut h), &mut hi);
                                }
                            }
                        }
                        let at = (first + k) * bw;
                        for (i, o) in row.iter_mut().enumerate() {
                            // A window with no value has ±∞ on both sides; one side is enough to tell.
                            let (l, u) = (
                                if want_lo { lo[i] } else { hi[i] },
                                if want_hi { hi[i] } else { lo[i] },
                            );
                            *o = self.extreme(l, u, lacks(at + i));
                        }
                    }
                });
            }
        }
    }

    /// The window's least `lo` and largest `hi` (±∞ when it has no value).
    fn extreme(&self, lo: f64, hi: f64, lacking: bool) -> f64 {
        if lo == f64::INFINITY || hi == f64::NEG_INFINITY || lacking {
            return f64::NAN;
        }
        match self.stat {
            Stat::Min => lo + 0.0,
            Stat::Max => hi + 0.0,
            _ => hi - lo,
        }
    }

    /// Whether each window holds a cell of the raster without a value.
    fn lacking(
        &self,
        region: &Region<'_>,
        size: (u32, u32),
        (c0, c1, j0, n): (u32, u32, u32, u32),
        threads: usize,
    ) -> Vec<bool> {
        let bw = (c1 - c0) as usize;
        let mut out = vec![false; bw * n as usize];
        let width = i64::from(size.0);
        par::rows(threads, &mut out, bw, &|first, chunk: &mut [bool]| {
            for (k, row) in chunk.chunks_mut(bw).enumerate() {
                let j = i64::from(j0) + (first + k) as i64;
                // Each window row's cells without a value, as a running count along the row.
                let mut lacking = vec![0u32; bw];
                for (dy, spans) in &self.window.rows {
                    let y = j + dy;
                    for &(lo, hi) in spans {
                        let mut c = 0u32;
                        for (i, l) in lacking.iter_mut().enumerate() {
                            let x = i64::from(c0) + i as i64;
                            let empty = |xx: i64| {
                                xx >= 0 && xx < width && region.get(xx, y).is_some_and(f64::is_nan)
                            };
                            if i == 0 {
                                c = (x + lo..=x + hi).filter(|&xx| empty(xx)).count() as u32;
                            } else {
                                c += u32::from(empty(x + hi));
                                c -= u32::from(empty(x + lo - 1));
                            }
                            *l += c;
                        }
                    }
                }
                for (o, l) in row.iter_mut().zip(lacking) {
                    *o = l > 0;
                }
            }
        });
        out
    }
}

/// The least (`least`) or largest of each run of `k` values of `line`
/// (van Herk and Gil-Werman: within blocks of k, from the left and from the
/// right); NaN counts as no value (+∞ for the least, −∞ for the largest).
/// `line.len() − k + 1` results, the first for `line[0..k]`.
pub fn running(line: &[f64], k: usize, least: bool) -> Vec<f64> {
    if k == 0 || line.len() < k {
        return Vec::new();
    }
    let none = if least {
        f64::INFINITY
    } else {
        f64::NEG_INFINITY
    };
    let mut out = vec![none; line.len() - k + 1];
    let (mut g, mut h) = (Vec::new(), Vec::new());
    running_fold(
        &|i| line[i],
        line.len(),
        k,
        least,
        (&mut g, &mut h),
        &mut out,
    );
    out
}

/// Each run of `k` of the values `at(0..len)` folded into `acc` (its least
/// or largest kept), `g` and `h` the caller's buffers.
pub fn running_fold(
    at: &dyn Fn(usize) -> f64,
    len: usize,
    k: usize,
    least: bool,
    (g, h): (&mut Vec<f64>, &mut Vec<f64>),
    acc: &mut [f64],
) {
    if k == 0 || len < k {
        return;
    }
    let none = if least {
        f64::INFINITY
    } else {
        f64::NEG_INFINITY
    };
    let pick = |a: f64, b: f64| if least { a.min(b) } else { a.max(b) };
    g.clear();
    g.resize(len, none);
    h.clear();
    h.resize(len, none);
    for start in (0..len).step_by(k) {
        let end = (start + k).min(len);
        let mut a = none;
        for (i, slot) in g.iter_mut().enumerate().take(end).skip(start) {
            let x = at(i);
            a = pick(a, if x.is_nan() { none } else { x });
            *slot = a;
        }
        let mut a = none;
        for i in (start..end).rev() {
            let x = at(i);
            a = pick(a, if x.is_nan() { none } else { x });
            h[i] = a;
        }
    }
    for (i, o) in acc.iter_mut().enumerate().take(len - k + 1) {
        *o = pick(*o, pick(h[i], g[i + k - 1]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The statistic of each cell by the window's cells, one by one: the reference.
    fn plain(f: &Focal, v: &[f64], w: u32, h: u32) -> Vec<f64> {
        let offsets = f.window.offsets();
        let mut out = Vec::new();
        for j in 0..i64::from(h) {
            for i in 0..i64::from(w) {
                let mut vals = Vec::new();
                let mut missing = false;
                for &(dx, dy) in &offsets {
                    let (x, y) = (i + dx, j + dy);
                    if x < 0 || y < 0 || x >= i64::from(w) || y >= i64::from(h) {
                        continue;
                    }
                    let x = v[(y * i64::from(w) + x) as usize];
                    if x.is_nan() {
                        missing = true;
                    } else {
                        vals.push(x);
                    }
                }
                let r = if vals.is_empty() || (missing && !f.ignore) {
                    f64::NAN
                } else {
                    match f.stat {
                        Stat::Sum => vals.iter().sum(),
                        Stat::Mean => vals.iter().sum::<f64>() / vals.len() as f64,
                        Stat::Min => vals.iter().copied().fold(f64::INFINITY, f64::min),
                        Stat::Max => vals.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                        Stat::Range => {
                            vals.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                                - vals.iter().copied().fold(f64::INFINITY, f64::min)
                        }
                        Stat::Std => {
                            let n = vals.len() as f64;
                            if vals.len() < 2 {
                                f64::NAN
                            } else {
                                let m = vals.iter().sum::<f64>() / n;
                                (vals.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (n - 1.0))
                                    .sqrt()
                            }
                        }
                        s => order_stat(&mut vals, s).unwrap_or(f64::NAN),
                    }
                };
                out.push(r);
            }
        }
        out
    }

    fn field(w: u32, h: u32) -> Vec<f64> {
        (0..w * h)
            .map(|k| {
                let (i, j) = (f64::from(k % w), f64::from(k / w));
                if k % 17 == 5 {
                    f64::NAN
                } else {
                    ((i * 7.0 + j * 13.0) % 11.0) + 0.25 * j
                }
            })
            .collect()
    }

    fn run(f: &Focal, v: &[f64], w: u32, h: u32, threads: usize) -> Vec<f64> {
        let region = Region {
            x0: 0,
            y0: 0,
            w: w as usize,
            h: h as usize,
            v,
        };
        let mut out = vec![0.0; (w * h) as usize];
        f.block(&region, (w, h), (0, w, 0, h), threads, &mut out);
        out
    }

    fn close(a: f64, b: f64) -> bool {
        (a.is_nan() && b.is_nan()) || (a - b).abs() <= 1e-9 * (1.0 + b.abs())
    }

    #[test]
    fn every_window_and_statistic_matches_the_cells_one_by_one() {
        let (w, h) = (37, 29);
        let v = field(w, h);
        let windows = [
            Window::rect(3, 3).unwrap(),
            Window::rect(5, 1).unwrap(),
            Window::rect(1, 7).unwrap(),
            Window::circle(3).unwrap(),
            Window::ring(1, 3).unwrap(),
        ];
        let stats = [
            Stat::Sum,
            Stat::Mean,
            Stat::Std,
            Stat::Min,
            Stat::Max,
            Stat::Range,
            Stat::Median,
            Stat::Majority,
            Stat::Minority,
            Stat::Variety,
        ];
        for win in &windows {
            for &stat in &stats {
                for ignore in [true, false] {
                    let f = Focal::new(win.clone(), stat, ignore).unwrap();
                    let want = plain(&f, &v, w, h);
                    let got = run(&f, &v, w, h, 3);
                    for (k, (a, b)) in got.iter().zip(&want).enumerate() {
                        assert!(
                            close(*a, *b),
                            "{:?} {stat:?} {ignore}: cell {k}: {a} vs {b}",
                            win.shape
                        );
                    }
                    // The same bits on one thread.
                    let one = run(&f, &v, w, h, 1);
                    assert!(
                        got.iter()
                            .zip(&one)
                            .all(|(a, b)| a.to_bits() == b.to_bits())
                    );
                }
            }
        }
    }

    #[test]
    fn windows_have_their_cells() {
        assert_eq!(Window::rect(3, 5).unwrap().offsets().len(), 15);
        // r = 2: 13 cells (di² + dj² ≤ 4).
        assert_eq!(Window::circle(2).unwrap().offsets().len(), 13);
        // 1 < d² ≤ 4: 13 − 5 = 8 cells... d² = 1 cells (4) and the centre out.
        assert_eq!(Window::ring(1, 2).unwrap().offsets().len(), 8);
        assert!(Window::rect(2, 3).is_err());
        assert!(Window::ring(3, 3).is_err());
    }

    #[test]
    fn running_extremes() {
        let line = [3.0, 1.0, f64::NAN, 4.0, 1.0, 5.0, 9.0, 2.0];
        assert_eq!(running(&line, 3, true), vec![1.0, 1.0, 1.0, 1.0, 1.0, 2.0]);
        assert_eq!(running(&line, 3, false), vec![3.0, 4.0, 4.0, 5.0, 9.0, 9.0]);
        assert_eq!(running(&line, 1, true)[2], f64::INFINITY);
    }
}
