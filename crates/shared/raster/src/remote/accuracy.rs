//! Doğruluk analizi (docs/adr/0242 §8): the class raster against reference
//! objects. A point gives the cell it falls in, an area the cells whose
//! centres it holds; each object's cells count for it (a cell under two
//! objects twice). The matrix's rows are the classified values, its columns
//! the reference values (both's union, rising); the producer's and user's
//! accuracies, the overall accuracy and Cohen's kappa come from the integer
//! counts, each a single division: κ = (n·Σd − Σrᵢcᵢ) / (n² − Σrᵢcᵢ).

use std::collections::BTreeMap;

use kentos_geometry_core::ops::statistics::read_number;

use super::{RemoteNotes, Table, count, fixed};

/// The largest class or reference value taken.
pub const MOST_VALUE: i64 = 2_147_483_647;

/// A reference text as a whole number (kentos.statistics/1); none when it is not one.
pub fn reference_of(text: &str) -> Option<i64> {
    let d = read_number(text)?;
    let p = 10i128.checked_pow(d.scale)?;
    if d.m % p != 0 {
        return None;
    }
    let v = d.m / p;
    (v.abs() <= i128::from(MOST_VALUE)).then_some(v as i64)
}

/// The matrix's counts by (classified, reference).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Confusion {
    pub pairs: BTreeMap<(i64, i64), u64>,
    /// Objects whose reference was not a whole number.
    pub unread: u64,
    /// Cells off the raster or without a value.
    pub off: u64,
}

impl Confusion {
    pub fn add(&mut self, classified: i64, reference: i64, n: u64) {
        *self.pairs.entry((classified, reference)).or_insert(0) += n;
    }

    pub fn join(&mut self, o: &Confusion) {
        for (&(c, r), &n) in &o.pairs {
            self.add(c, r, n);
        }
        self.unread += o.unread;
        self.off += o.off;
    }

    /// The notes: the table, the summary, the warnings, the overall accuracy and kappa.
    pub fn notes(&self) -> Result<RemoteNotes, String> {
        let n: u64 = self.pairs.values().sum();
        if n == 0 {
            return Err(
                "Değerlendirilecek hücre yok: referans nesneleri rasterin değerli hücrelerine düşmüyor."
                    .into(),
            );
        }
        let mut classes: Vec<i64> = self.pairs.keys().flat_map(|&(c, r)| [c, r]).collect();
        classes.sort_unstable();
        classes.dedup();
        let at = |c: i64, r: i64| self.pairs.get(&(c, r)).copied().unwrap_or(0);
        let rows_t: Vec<u64> = classes
            .iter()
            .map(|&c| classes.iter().map(|&r| at(c, r)).sum())
            .collect();
        let cols_t: Vec<u64> = classes
            .iter()
            .map(|&r| classes.iter().map(|&c| at(c, r)).sum())
            .collect();
        let pct = |part: u64, whole: u64| {
            if whole == 0 {
                "—".to_owned()
            } else {
                fixed((part * 100) as f64 / whole as f64, 2)
            }
        };
        let mut columns = vec!["Sınıflandırılan \\ Referans".to_owned()];
        columns.extend(classes.iter().map(i64::to_string));
        columns.push("Toplam".into());
        columns.push("Kullanıcı doğruluğu (%)".into());
        let mut table = Table {
            columns,
            rows: Vec::new(),
        };
        for (q, &c) in classes.iter().enumerate() {
            let mut row = vec![c.to_string()];
            row.extend(classes.iter().map(|&r| at(c, r).to_string()));
            row.push(rows_t[q].to_string());
            row.push(pct(at(c, c), rows_t[q]));
            table.rows.push(row);
        }
        let mut total = vec!["Toplam".to_owned()];
        total.extend(cols_t.iter().map(u64::to_string));
        total.push(n.to_string());
        total.push(String::new());
        table.rows.push(total);
        let mut producer = vec!["Üretici doğruluğu (%)".to_owned()];
        producer.extend(classes.iter().zip(&cols_t).map(|(&r, &t)| pct(at(r, r), t)));
        producer.push(String::new());
        producer.push(String::new());
        table.rows.push(producer);
        let diag: u64 = classes.iter().map(|&c| at(c, c)).sum();
        // Counts are at most 2³¹ cells: the products stay well within i128.
        let s: i128 = rows_t
            .iter()
            .zip(&cols_t)
            .map(|(&r, &c)| i128::from(r) * i128::from(c))
            .sum();
        let (ni, di) = (i128::from(n), i128::from(diag));
        let overall = (diag * 100) as f64 / n as f64;
        let kappa = if ni * ni == s {
            1.0
        } else {
            (ni * di - s) as f64 / (ni * ni - s) as f64
        };
        let mut warnings = Vec::new();
        if self.unread > 0 {
            warnings.push(format!(
                "{} nesnenin referansı tam sayı olarak okunamadı; alınmadı.",
                count(self.unread)
            ));
        }
        if self.off > 0 {
            warnings.push(format!(
                "{} hücre rasterin dışında ya da boş; alınmadı.",
                count(self.off)
            ));
        }
        Ok(RemoteNotes {
            table: Some(table),
            tail: format!(
                "Genel doğruluk %{}, kappa {} ({} hücre).",
                fixed(overall, 2),
                fixed(kappa, 4),
                count(n)
            ),
            warnings,
            overall: Some(overall),
            kappa: Some(kappa),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn references() {
        assert_eq!(reference_of("3"), Some(3));
        assert_eq!(reference_of(" 4,0 "), Some(4));
        assert_eq!(reference_of("2.5"), None);
        assert_eq!(reference_of("x"), None);
        assert_eq!(reference_of("-7"), Some(-7));
        assert_eq!(reference_of("99999999999"), None);
    }

    #[test]
    fn congaltons_example() {
        // Congalton & Green's matrix: 65 4 22 24 / 6 81 5 8 / 0 11 85 19 / 4 7 3 90.
        let m = [
            [65, 4, 22, 24],
            [6, 81, 5, 8],
            [0, 11, 85, 19],
            [4, 7, 3, 90],
        ];
        let mut c = Confusion::default();
        for (i, row) in m.iter().enumerate() {
            for (j, &n) in row.iter().enumerate() {
                c.add(i as i64 + 1, j as i64 + 1, n);
            }
        }
        let notes = c.notes().unwrap();
        // 321 / 434 and (434 · 321 − 46 814) / (434² − 46 814) = 92 500 / 141 542.
        assert_eq!(
            notes.tail,
            "Genel doğruluk %73.96, kappa 0.6535 (434 hücre)."
        );
        let t = notes.table.unwrap();
        assert_eq!(t.rows[0], ["1", "65", "4", "22", "24", "115", "56.52"]);
        assert_eq!(t.rows[5][1], "86.67");
    }
}
