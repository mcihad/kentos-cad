//! Raster hesaplayıcı's expressions (docs/adr/0233 §3): KentOS's expression
//! language over the cells of a grid. Each input raster is named after its
//! layer (`DEM`, a second one on the layer `DEM (2)`), a band with `@`
//! (`[Ortofoto@3]`; the bare name is band 1); they are the schema's number
//! fields. `$y` and `$x` are a cell's centre (east, north), `$alan` its
//! area. A row is evaluated by the column engine, 256 cells a batch.

use kentos_expression::exec::Slot;
use kentos_expression::host::{
    Builtin, FieldDef, FieldSource, FieldType, Geometry, Objects, Schema,
};
use kentos_expression::rows::{As, NUMBER};
use kentos_expression::{Expr, compile_with};

use crate::inputs::point_of;

/// How cells without a value meet the expression.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Empty {
    /// A cell any raster the expression reads has no value at stays empty.
    Propagate,
    /// The expression sees `boş` and decides.
    Expression,
}

/// A compiled expression and the bands its fields stand for.
pub struct Calc {
    expr: Expr,
    /// Each field of the expression (its order): the input and the band (from 0).
    refs: Vec<(usize, usize)>,
    empty: Empty,
}

impl std::fmt::Debug for Calc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Calc").field("refs", &self.refs).finish()
    }
}

/// The band a field names among the inputs `names` (each with its value
/// bands): the bare name band 1, `name@b` band b.
fn band_of(field: &str, names: &[(String, u32)]) -> Option<(usize, usize)> {
    if let Some(k) = names.iter().position(|(n, _)| n == field) {
        return Some((k, 0));
    }
    let (name, b) = field.rsplit_once('@')?;
    let b: usize = b.parse().ok()?;
    let k = names.iter().position(|(n, _)| n == name)?;
    (b >= 1 && b <= names[k].1 as usize).then_some((k, b - 1))
}

const NO_RASTER: &str = "İfade bir raster anmalı (örnek: [DEM] * 2).";

/// A field that names no raster's band, said with the rasters' names.
fn no_band<'a>(field: &str, names: impl Iterator<Item = &'a str>) -> String {
    let list: Vec<String> = names.map(|n| format!("[{n}]")).collect();
    format!(
        "“{field}” adında raster bandı yok. Rasterler: {}; bant @ ile: [Ad@2].",
        list.join(", ")
    )
}

/// The inputs `source` names, each once in the order it names them (the
/// first is the one whose grid the result takes), by their names alone:
/// before the rasters are opened, their bands unknown. A field is a
/// raster's name, or its name, `@` and a band number; one that is no
/// raster's is refused as `Calc::new` refuses it.
pub fn named(source: &str, names: &[&str]) -> Result<Vec<usize>, String> {
    let expr = kentos_expression::compile(source).map_err(|e| e.text())?;
    let mut read: Vec<usize> = Vec::new();
    for f in &expr.fields {
        let k = names.iter().position(|n| n == f).or_else(|| {
            let (name, b) = f.rsplit_once('@')?;
            b.parse::<usize>().ok()?;
            names.iter().position(|n| *n == name)
        });
        match k {
            Some(k) if !read.contains(&k) => read.push(k),
            Some(_) => {}
            None => return Err(no_band(f, names.iter().copied())),
        }
    }
    if read.is_empty() {
        return Err(NO_RASTER.into());
    }
    Ok(read)
}

impl Calc {
    /// `source` over the inputs `names` (their names and value bands).
    pub fn new(source: &str, names: &[(String, u32)], empty: Empty) -> Result<Calc, String> {
        let mut fields = Vec::new();
        for (name, bands) in names {
            let def = |n: String, d: String| FieldDef {
                name: n,
                ty: FieldType::Number,
                source: FieldSource::User,
                description: d,
            };
            fields.push(def(name.clone(), format!("{name} rasterinin 1. bandı")));
            for b in 1..=*bands {
                fields.push(def(
                    format!("{name}@{b}"),
                    format!("{name} rasterinin {b}. bandı"),
                ));
            }
        }
        let schema = Schema {
            fields,
            variables: Vec::new(),
            world: false,
        };
        let expr = compile_with(source, &schema).map_err(|e| e.text())?;
        let mut refs = Vec::with_capacity(expr.fields.len());
        for f in &expr.fields {
            match band_of(f, names) {
                Some(r) => refs.push(r),
                None => return Err(no_band(f, names.iter().map(|(n, _)| n.as_str()))),
            }
        }
        if refs.is_empty() {
            return Err(NO_RASTER.into());
        }
        Ok(Calc { expr, refs, empty })
    }

    /// The input whose grid the result takes: the first the expression names.
    pub fn grid_input(&self) -> usize {
        self.refs[0].0
    }

    /// Each field's input and band (from 0), in the expression's order.
    pub fn refs(&self) -> &[(usize, usize)] {
        &self.refs
    }

    /// The inputs the expression reads, each once, in the order it names them.
    pub fn reads(&self) -> Vec<usize> {
        let mut out: Vec<usize> = Vec::new();
        for &(k, _) in &self.refs {
            if !out.contains(&k) {
                out.push(k);
            }
        }
        out
    }

    /// Row `j`'s values from column `i0` on (`out.len()` cells) on a grid of
    /// `affine` whose cells are `area` m²: `cols[f]` the cells' values of
    /// field f's band (NaN where none). Not a finite number: NaN.
    pub fn row(
        &self,
        cols: &[&[f64]],
        affine: &[f64; 6],
        (i0, j): (u32, u32),
        area: f64,
        out: &mut [f64],
    ) {
        let n = out.len();
        let objects = Row {
            calc: self,
            cols,
            affine,
            i0,
            j,
            area,
            n,
        };
        let column = self.expr.evaluate_objects(&objects, As::Number);
        for (k, o) in out.iter_mut().enumerate() {
            let v = column.numbers.get(k).copied().unwrap_or(f64::NAN);
            *o = if column.kinds.get(k) == Some(&NUMBER) && v.is_finite() {
                v + 0.0
            } else {
                f64::NAN
            };
        }
        if self.empty == Empty::Propagate {
            for c in cols {
                for (o, &v) in out.iter_mut().zip(c.iter()) {
                    if v.is_nan() {
                        *o = f64::NAN;
                    }
                }
            }
        }
    }
}

/// A row's cells as the engine's objects.
struct Row<'r> {
    calc: &'r Calc,
    cols: &'r [&'r [f64]],
    affine: &'r [f64; 6],
    i0: u32,
    j: u32,
    area: f64,
    n: usize,
}

impl<'a> Objects<'a> for Row<'_> {
    fn len(&self) -> usize {
        self.n
    }

    fn field(&self, name: &str, _ty: FieldType, start: usize, mut slot: Slot<'_, 'a>) {
        match self.calc.expr.fields.iter().position(|f| f == name) {
            Some(f) => {
                let col = self.cols[f];
                slot.numbers(|k| {
                    let v = col.get(start + k).copied().unwrap_or(f64::NAN);
                    (!v.is_nan(), v)
                });
            }
            None => slot.numbers(|_| (false, 0.0)),
        }
    }

    fn geometry(&self, what: Geometry, start: usize, mut slot: Slot<'_, 'a>) {
        let centre = |k: usize| {
            point_of(
                self.affine,
                f64::from(self.i0) + (start + k) as f64 + 0.5,
                f64::from(self.j) + 0.5,
            )
        };
        match what {
            Geometry::AnchorY | Geometry::CentroidY => slot.numbers(|k| (true, centre(k).0)),
            Geometry::AnchorX | Geometry::CentroidX => slot.numbers(|k| (true, centre(k).1)),
            Geometry::Area => slot.numbers(|_| (true, self.area)),
            _ => slot.numbers(|_| (false, 0.0)),
        }
    }

    fn builtin(&self, _what: Builtin, _start: usize, mut slot: Slot<'_, 'a>) {
        for k in 0..slot.len() {
            slot.text(k, None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names() -> Vec<(String, u32)> {
        vec![
            ("DEM".into(), 1),
            ("Ortofoto".into(), 3),
            ("DEM (2)".into(), 1),
        ]
    }

    #[test]
    fn names_and_bands() {
        let c = Calc::new("[Ortofoto@3] - [DEM] + DEM", &names(), Empty::Propagate).unwrap();
        assert_eq!(c.refs(), &[(1, 2), (0, 0)]);
        assert_eq!(c.grid_input(), 1);
        assert_eq!(c.reads(), vec![1, 0]);
        assert!(
            Calc::new("[Yok] + 1", &names(), Empty::Propagate)
                .unwrap_err()
                .contains("“Yok”")
        );
        assert!(Calc::new("[Ortofoto@4]", &names(), Empty::Propagate).is_err());
        assert!(
            Calc::new("$x + 1", &names(), Empty::Propagate)
                .unwrap_err()
                .contains("raster anmalı")
        );
    }

    #[test]
    fn a_row_is_worked_out() {
        let affine = [100.0, 2.0, 0.0, 50.0, 0.0, -2.0];
        let a = [1.0, 2.0, f64::NAN, 4.0];
        let b = [10.0, 20.0, 30.0, f64::NAN];
        let mut out = [0.0; 4];
        let c = Calc::new("[DEM] * 2 + [DEM (2)]", &names(), Empty::Propagate).unwrap();
        c.row(&[&a, &b], &affine, (0, 0), 4.0, &mut out);
        assert_eq!(out[..2], [12.0, 24.0]);
        assert!(out[2].is_nan() && out[3].is_nan());
        // The expression decides: an empty DEM counts as 0.
        let c = Calc::new(
            "durum eğer [DEM] boş ise 0 yoksa [DEM] son",
            &names(),
            Empty::Expression,
        )
        .unwrap();
        c.row(&[&a], &affine, (0, 0), 4.0, &mut out);
        assert_eq!(out, [1.0, 2.0, 0.0, 4.0]);
        // Conditions are 1 and 0; the centre and the area.
        let c = Calc::new("[DEM] > 1.5", &names(), Empty::Propagate).unwrap();
        c.row(&[&a], &affine, (0, 0), 4.0, &mut out);
        assert_eq!(out[..2], [0.0, 1.0]);
        let c = Calc::new(
            "[DEM] * 0 + $y + $x / 1000 + $alan",
            &names(),
            Empty::Propagate,
        )
        .unwrap();
        c.row(&[&[0.0, 0.0, 0.0, 0.0]], &affine, (3, 1), 4.0, &mut out);
        // Cell (3, 1): centre x = 100 + 2·3.5 = 107, y = 50 − 2·1.5 = 47.
        assert_eq!(out[0], 107.0 + 0.047 + 4.0);
    }
}
