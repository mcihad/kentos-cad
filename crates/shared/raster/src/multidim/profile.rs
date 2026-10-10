//! Kesit (docs/adr/0243 §8): a raster's values along lines. Each line is
//! walked from its start (its arcs and curves their own; the route walker
//! of Km yaz, `tools::point_calc::Walk`): a point every Adım from the start
//! and one at the end. A point's value is its cell's (a mesh's raster: the
//! mesh's interpolation), in the slice the raster shows. The table has a row
//! a point; the lines drawn with the values as their vertices' elevations
//! break where a point has none.

use kentos_geometry_core::display::fixed;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::tools::point_calc::Walk;

use super::points::{Make, PointValues, PointsJob};
use super::{Finished, Piece, Table, grouped, value_text};
use crate::inputs::Input;

/// The most points a run reads.
pub const MOST_POINTS: usize = 1_000_000;

/// Why a step is refused.
pub const BAD_STEP: &str = "Adım sıfırdan büyük bir uzunluk olmalı.";

/// A point along a line: the line's number (from 1), how far along it is, where.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Station {
    pub line: u32,
    pub s: f64,
    pub x: f64,
    pub y: f64,
}

/// The lines of objects as Kesit walks them: a multi-part line part by part.
pub fn walked_lines(objects: Vec<Shape>) -> Vec<Shape> {
    let mut out = Vec::new();
    for s in objects {
        match s {
            Shape::Polyline {
                pts, bulges, parts, ..
            } => {
                out.push(Shape::Polyline {
                    pts,
                    bulges,
                    holes: None,
                    parts: None,
                });
                for p in parts.unwrap_or_default() {
                    out.push(Shape::Polyline {
                        pts: p.pts,
                        bulges: p.bulges,
                        holes: None,
                        parts: None,
                    });
                }
            }
            other => out.push(other),
        }
    }
    out
}

/// The points along `lines` every `step` from each start, and at each end.
pub fn stations(lines: &[Shape], step: f64) -> Result<(Vec<Station>, usize), String> {
    if !(step > 0.0 && step.is_finite()) {
        return Err(BAD_STEP.into());
    }
    let mut out = Vec::new();
    let mut walked = 0;
    for (k, shape) in lines.iter().enumerate() {
        let Some(walk) = Walk::new(shape, false) else {
            continue;
        };
        walked += 1;
        let line = k as u32 + 1;
        let length = walk.length();
        let at = |s: f64, out: &mut Vec<Station>| {
            let (p, _) = walk.frame(s);
            out.push(Station {
                line,
                s,
                x: p.x,
                y: p.y,
            });
        };
        // The multiples i·step short of the end, then the end.
        let mut i = 0u64;
        loop {
            let s = i as f64 * step;
            if i > 0 && s >= length - 1e-9 {
                break;
            }
            if out.len() >= MOST_POINTS {
                return Err(format!(
                    "Bu adımla {} noktadan çok olur; adımı büyütün.",
                    grouped(MOST_POINTS as u64)
                ));
            }
            at(s.min(length), &mut out);
            i += 1;
            if length <= 1e-9 {
                break;
            }
        }
        if length > 1e-9 {
            at(length, &mut out);
        }
    }
    Ok((out, walked))
}

/// What Kesit's values make: its points, the axes' names (east's first), the lines walked.
pub struct Made {
    pub stations: Vec<Station>,
    pub axes: [String; 2],
    pub lines: usize,
}

impl Made {
    /// The table, the lines with values and the summary.
    pub fn finish(self, values: &[f64]) -> Finished {
        let value = |k: usize| values.get(k).copied().unwrap_or(f64::NAN);
        let rows: Vec<Vec<String>> = self
            .stations
            .iter()
            .enumerate()
            .map(|(k, st)| {
                vec![
                    st.line.to_string(),
                    fixed(st.s, 3),
                    fixed(st.x, 3),
                    fixed(st.y, 3),
                    value_text(value(k)),
                ]
            })
            .collect();
        let mut pieces: Vec<Piece> = Vec::new();
        let mut run: Vec<[f64; 3]> = Vec::new();
        let mut line = 0;
        let close = |run: &mut Vec<[f64; 3]>, line: u32, pieces: &mut Vec<Piece>| {
            if run.len() >= 2 {
                pieces.push(Piece {
                    line,
                    points: std::mem::take(run),
                });
            } else {
                run.clear();
            }
        };
        for (k, st) in self.stations.iter().enumerate() {
            if st.line != line {
                close(&mut run, line, &mut pieces);
                line = st.line;
            }
            let v = value(k);
            if v.is_finite() {
                run.push([st.x, st.y, v]);
            } else {
                close(&mut run, line, &mut pieces);
            }
        }
        close(&mut run, line, &mut pieces);
        let empty = (0..self.stations.len())
            .filter(|&k| !value(k).is_finite())
            .count();
        let mut summary = format!(
            "{} çizgi, {} nokta",
            grouped(self.lines as u64),
            grouped(self.stations.len() as u64)
        );
        if empty > 0 {
            summary.push_str(&format!(" ({} değersiz)", grouped(empty as u64)));
        }
        summary.push('.');
        let mut warnings = Vec::new();
        if empty == self.stations.len() && !self.stations.is_empty() {
            warnings.push(
                "Çizgilerin hiçbir noktasında değer yok: çizgiler rasterin dışında olabilir."
                    .into(),
            );
        }
        Finished {
            table: Some(Table {
                columns: vec![
                    "Çizgi".into(),
                    "Uzaklık (m)".into(),
                    self.axes[0].clone(),
                    self.axes[1].clone(),
                    "Değer".into(),
                ],
                rows,
            }),
            pieces,
            summary,
            warnings,
        }
    }
}

/// Kesit over `input`'s band `band` (from 0) along the objects `lines`
/// (a multi-part line part by part), a point every `step` metres.
pub fn job(
    input: Input,
    lines: &[Shape],
    step: f64,
    band: usize,
    axes: [String; 2],
) -> Result<PointsJob, String> {
    let lines = walked_lines(lines.to_vec());
    let lines = lines.as_slice();
    if input.reader.mesh_levels().is_none() && band >= input.values() as usize {
        return Err(format!(
            "Rasterin {} bandı var; {}. bant yok.",
            input.values(),
            band + 1
        ));
    }
    let (stations, walked) = stations(lines, step)?;
    if walked == 0 {
        return Err("Çizgi seçin: Kesit çizgi ya da çoklu çizgi ister.".into());
    }
    let points = stations.iter().map(|s| [s.x, s.y]).collect();
    Ok(PointsJob {
        values: PointValues::new(input, points, vec![band]),
        make: Make::Profile(Made {
            stations,
            axes,
            lines: walked,
        }),
    })
}
