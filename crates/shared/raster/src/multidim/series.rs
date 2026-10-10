//! Zaman serisi (docs/adr/0243 §9): values at points over a NetCDF dataset's
//! time steps (the formats core's `Cube::series`: only the values the points
//! need are read; the other slice dimensions at the raster's values), or
//! over a raster's bands. The table has a row a step and a column a point.

use kentos_formats::multidim::cube::dim_labels;
use kentos_formats::multidim::series::Series;

use super::points::{Make, PointValues, PointsJob};
use super::{Finished, Table, grouped, value_text};
use crate::inputs::Input;

/// A point the series is read at: its name (`ad`), where.
#[derive(Clone, Debug, PartialEq)]
pub struct NamedPoint {
    pub name: Option<String>,
    pub x: f64,
    pub y: f64,
}

/// The points of objects: a point's, and every point of a multi-point
/// object, each with the object's name (`ad`); other objects none.
pub fn named_points(
    objects: Vec<(Option<String>, kentos_geometry_core::entity::Shape)>,
) -> Vec<NamedPoint> {
    use kentos_geometry_core::entity::Shape;
    let mut out = Vec::new();
    for (name, s) in objects {
        if let Shape::Point { p, parts, .. } = s {
            out.push(NamedPoint {
                name: name.clone(),
                x: p.x,
                y: p.y,
            });
            for q in parts.unwrap_or_default() {
                out.push(NamedPoint {
                    name: name.clone(),
                    x: q.p.x,
                    y: q.p.y,
                });
            }
        }
    }
    out
}

/// Why a raster has no series.
pub const NO_SERIES: &str = "Raster tek bantlı ve zaman boyutu yok: Zaman serisi zaman boyutlu veri seti ya da çok bantlı raster ister.";

/// The points' columns: their names, else “Nokta k” (k their place from 1);
/// a name met again numbered (“K (2)”).
pub fn names(points: &[NamedPoint]) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(points.len());
    for (k, p) in points.iter().enumerate() {
        let base = match p.name.as_deref().map(str::trim) {
            Some(n) if !n.is_empty() => n.to_owned(),
            _ => format!("Nokta {}", k + 1),
        };
        let mut name = base.clone();
        let mut i = 2;
        while out.contains(&name) {
            name = format!("{base} ({i})");
            i += 1;
        }
        out.push(name);
    }
    out
}

fn summary(
    points: usize,
    steps: usize,
    values: impl Iterator<Item = f64>,
) -> (String, Vec<String>) {
    let mut any = false;
    for v in values {
        if v.is_finite() {
            any = true;
            break;
        }
    }
    let mut warnings = Vec::new();
    if !any && points > 0 && steps > 0 {
        warnings
            .push("Noktaların hiçbirinde değer yok: noktalar rasterin dışında olabilir.".into());
    }
    (
        format!(
            "{} nokta, {} adım.",
            grouped(points as u64),
            grouped(steps as u64)
        ),
        warnings,
    )
}

/// What a band series' values make.
pub struct BandsMade {
    pub names: Vec<String>,
}

impl BandsMade {
    /// The table: a row a band.
    pub fn finish(self, values: &[f64], bands: usize) -> Finished {
        let n = self.names.len();
        let at = |p: usize, b: usize| values.get(p * bands + b).copied().unwrap_or(f64::NAN);
        let rows = (0..bands)
            .map(|b| {
                let mut row = vec![(b + 1).to_string()];
                row.extend((0..n).map(|p| value_text(at(p, b))));
                row
            })
            .collect();
        let (summary, warnings) = summary(n, bands, values.iter().copied());
        let mut columns = vec!["Bant".to_owned()];
        columns.extend(self.names);
        Finished {
            table: Some(Table { columns, rows }),
            pieces: Vec::new(),
            summary,
            warnings,
        }
    }
}

/// Zaman serisi over `input`'s value bands at `points`.
pub fn bands_job(input: Input, points: &[NamedPoint]) -> Result<PointsJob, String> {
    if points.is_empty() {
        return Err("Nokta seçin: Zaman serisi noktalar ister.".into());
    }
    let bands = input.values() as usize;
    if bands < 2 || input.reader.mesh_levels().is_some() {
        return Err(NO_SERIES.into());
    }
    Ok(PointsJob {
        values: PointValues::new(
            input,
            points.iter().map(|p| [p.x, p.y]).collect(),
            (0..bands).collect(),
        ),
        make: Make::Bands(BandsMade {
            names: names(points),
        }),
    })
}

/// Zaman serisi over a NetCDF dataset's time steps.
pub struct TimeJob {
    series: Series,
    names: Vec<String>,
    asked: bool,
    values: Option<Vec<Vec<f64>>>,
}

impl TimeJob {
    pub fn new(series: Series, points: &[NamedPoint]) -> Result<TimeJob, String> {
        if points.is_empty() {
            return Err("Nokta seçin: Zaman serisi noktalar ister.".into());
        }
        Ok(TimeJob {
            series,
            names: names(points),
            asked: false,
            values: None,
        })
    }

    /// Every run the points need, at once.
    pub fn needs(&mut self) -> Vec<(u64, u64)> {
        if self.asked {
            return Vec::new();
        }
        self.asked = true;
        self.series.runs.iter().map(|r| (r.offset, r.len)).collect()
    }

    pub fn put(&mut self, k: usize, bytes: Vec<u8>) -> Result<(), String> {
        let r = self
            .series
            .runs
            .get(k)
            .cloned()
            .ok_or("Böyle bir parça istenmedi.")?;
        self.series.put(r.offset, bytes);
        Ok(())
    }

    pub fn step(&mut self) -> Result<(), String> {
        if self.asked && self.values.is_none() {
            self.values = Some(self.series.values().map_err(|e| e.0)?);
        }
        Ok(())
    }

    pub fn done(&self) -> bool {
        self.values.is_some()
    }

    pub fn share(&self) -> f64 {
        if self.values.is_some() { 1.0 } else { 0.0 }
    }

    /// The table: a row a time step.
    pub fn finish(self) -> Result<Finished, String> {
        let values = self.values.ok_or("Zaman serisi okunmadı.")?;
        let labels = dim_labels(&self.series.times, true, None);
        let n = self.names.len();
        let rows = values
            .iter()
            .enumerate()
            .map(|(s, row)| {
                let mut out = vec![
                    (s + 1).to_string(),
                    labels.get(s).cloned().unwrap_or_default(),
                ];
                out.extend(row.iter().map(|&v| value_text(v)));
                out
            })
            .collect();
        let (summary, warnings) = summary(n, values.len(), values.iter().flatten().copied());
        let mut columns = vec!["Adım".to_owned(), "Zaman".to_owned()];
        columns.extend(self.names);
        Ok(Finished {
            table: Some(Table { columns, rows }),
            pieces: Vec::new(),
            summary,
            warnings,
        })
    }
}
