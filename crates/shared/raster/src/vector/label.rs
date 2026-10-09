//! A raster's regions (docs/adr/0234 §4): cells of exactly equal value
//! joined by their sides (or by their corners too), labelled row after row
//! as the strips come, by union–find over provisional labels; then
//! numbered 1, 2, … in the order of their first cell, row by row. Only the
//! previous row's values are kept; every cell's label stays (4 bytes a cell).

/// Provisional labels a run may make before it is refused (§2).
pub const MOST_PROVISIONAL: usize = 8_000_000;

/// The regions of a raster: each cell's region (0: no value, else 1..=n)
/// and each region's value (`values[k - 1]`, its first cell's).
#[derive(Clone, Debug, PartialEq)]
pub struct Regions {
    pub width: u32,
    pub height: u32,
    pub labels: Vec<u32>,
    pub values: Vec<f64>,
}

/// Rows going into regions.
#[derive(Debug)]
pub struct Labeler {
    width: usize,
    height: usize,
    eight: bool,
    labels: Vec<u32>,
    /// Union–find over the provisional labels (index 0 unused): a set's
    /// root is its least label, the one its first cell made.
    parent: Vec<u32>,
    value: Vec<f64>,
    prev: Vec<f64>,
    row: usize,
}

fn find(parent: &mut [u32], mut x: u32) -> u32 {
    let mut root = x;
    while parent[root as usize] != root {
        root = parent[root as usize];
    }
    while parent[x as usize] != root {
        let next = parent[x as usize];
        parent[x as usize] = root;
        x = next;
    }
    root
}

/// Joins the sets of `a` and `b`; the root is the lesser label.
fn union(parent: &mut [u32], a: u32, b: u32) -> u32 {
    let (ra, rb) = (find(parent, a), find(parent, b));
    let (lo, hi) = if ra < rb { (ra, rb) } else { (rb, ra) };
    parent[hi as usize] = lo;
    lo
}

impl Labeler {
    /// A raster of `width` × `height` cells; `eight`: corners join cells too.
    pub fn new(width: u32, height: u32, eight: bool) -> Result<Labeler, String> {
        let cells = u64::from(width) * u64::from(height);
        if cells > super::MOST_CELLS {
            return Err(too_big(width, height));
        }
        Ok(Labeler {
            width: width as usize,
            height: height as usize,
            eight,
            labels: vec![0; cells as usize],
            parent: vec![0],
            value: vec![f64::NAN],
            prev: vec![f64::NAN; width as usize],
            row: 0,
        })
    }

    /// The next row's values (NaN: no value), all `width` of them.
    pub fn row(&mut self, vals: &[f64]) -> Result<(), String> {
        let (w, j) = (self.width, self.row);
        if j >= self.height || vals.len() != w {
            return Err("Rasterin satırları beklenenden fazla.".into());
        }
        let base = j * w;
        for i in 0..w {
            let v = vals[i];
            if v.is_nan() {
                continue;
            }
            let mut l = 0u32;
            if i > 0 && vals[i - 1] == v {
                l = self.labels[base + i - 1];
            }
            if j > 0 {
                let up = base - w + i;
                let mut join = |l: &mut u32, other: u32| {
                    *l = if *l == 0 {
                        other
                    } else {
                        union(&mut self.parent, *l, other)
                    };
                };
                if self.prev[i] == v {
                    join(&mut l, self.labels[up]);
                }
                if self.eight {
                    if i > 0 && self.prev[i - 1] == v {
                        join(&mut l, self.labels[up - 1]);
                    }
                    if i + 1 < w && self.prev[i + 1] == v {
                        join(&mut l, self.labels[up + 1]);
                    }
                }
            }
            if l == 0 {
                if self.parent.len() > MOST_PROVISIONAL {
                    return Err(too_many());
                }
                l = self.parent.len() as u32;
                self.parent.push(l);
                self.value.push(v);
            }
            self.labels[base + i] = l;
        }
        self.prev.copy_from_slice(vals);
        self.row += 1;
        Ok(())
    }

    /// The regions, numbered by their first cells; refused past `most`.
    pub fn finish(mut self, most: usize) -> Result<Regions, String> {
        if self.row != self.height {
            return Err("Rasterin satırları eksik geldi.".into());
        }
        let mut number = vec![0u32; self.parent.len()];
        let mut values = Vec::new();
        for k in 0..self.labels.len() {
            let l = self.labels[k];
            if l == 0 {
                continue;
            }
            let root = find(&mut self.parent, l);
            if number[root as usize] == 0 {
                if values.len() >= most {
                    return Err(too_many());
                }
                values.push(self.value[root as usize]);
                number[root as usize] = values.len() as u32;
            }
            self.labels[k] = number[root as usize];
        }
        Ok(Regions {
            width: self.width as u32,
            height: self.height as u32,
            labels: self.labels,
            values,
        })
    }
}

fn too_big(width: u32, height: u32) -> String {
    format!(
        "Raster vektörleştirme için çok büyük ({width} × {height}): en çok 2²⁶ hücre (8192 × 8192). Önce Maskeyle kırp ile bölün ya da Yeniden örnekle ile hücreleri büyütün."
    )
}

fn too_many() -> String {
    format!(
        "Rasterden {} alandan fazlası çıkıyor: değerler çok dağınık. Önce Yeniden sınıflandır ile sınıflara ayırın ya da Yeniden örnekle ile hücreleri büyütün.",
        super::MOST_FEATURES
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regions(rows: &[&[f64]], eight: bool) -> Regions {
        let w = rows[0].len() as u32;
        let mut l = Labeler::new(w, rows.len() as u32, eight).unwrap();
        for r in rows {
            l.row(r).unwrap();
        }
        l.finish(1000).unwrap()
    }

    #[test]
    fn regions_by_sides_and_by_corners() {
        let n = f64::NAN;
        // A U whose arms meet the row below only at a corner.
        let rows: [&[f64]; 3] = [&[1.0, 2.0, 1.0], &[1.0, 1.0, 1.0], &[2.0, n, 2.0]];
        let four = regions(&rows, false);
        assert_eq!(four.labels, vec![1, 2, 1, 1, 1, 1, 3, 0, 4]);
        assert_eq!(four.values, vec![1.0, 2.0, 2.0, 2.0]);
        // By corners the 2s in the top row meet nothing more, the bottom ones stay apart (NaN between).
        let eight = regions(&rows, true);
        assert_eq!(eight.labels, vec![1, 2, 1, 1, 1, 1, 3, 0, 4]);
        // A diagonal pair: two regions by sides, one by corners.
        let diag: [&[f64]; 2] = [&[5.0, 0.0], &[0.0, 5.0]];
        assert_eq!(regions(&diag, false).labels, vec![1, 2, 3, 4]);
        assert_eq!(regions(&diag, true).labels, vec![1, 2, 2, 1]);
    }

    #[test]
    fn a_region_merged_late_keeps_its_first_cells_number() {
        // Two arms that join only in the last row: one region, numbered by its first cell.
        let rows: [&[f64]; 3] = [&[7.0, 0.0, 7.0], &[7.0, 0.0, 7.0], &[7.0, 7.0, 7.0]];
        let r = regions(&rows, false);
        assert_eq!(r.labels, vec![1, 2, 1, 1, 2, 1, 1, 1, 1]);
        assert_eq!(r.values, vec![7.0, 0.0]);
    }

    #[test]
    fn too_many_regions_are_refused() {
        let mut l = Labeler::new(4, 1, false).unwrap();
        l.row(&[1.0, 2.0, 3.0, 4.0]).unwrap();
        assert!(l.finish(3).is_err());
    }
}
