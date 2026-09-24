//! Objects packed as numbers (docs/adr/0008: points cross as `Float64Array`):
//! how the page sends whole drawings to the store. A JSON array of 80 000
//! parcels is 26 MB and builds a tree of millions of small values before a
//! single object is read; the packed form is one run of numbers and a short
//! list of strings (layer ids, texts), read straight into shapes.
//! `apps/web/src/wasm/pack.ts` writes it. Every number is a float64, so ids and
//! coordinates arrive bit for bit (−0 and NaN included).
//!
//! Per object: `id, layer, label, kind`, then the kind's fields:
//!
//! | kind | fields |
//! |---|---|
//! | 0 point | x, y, hasZ, z |
//! | 1 line | a.x, a.y, b.x, b.y |
//! | 2 polyline, 3 polygon | path, holes |
//! | 4 circle | c.x, c.y, r |
//! | 5 arc | c.x, c.y, r, a0, a1 |
//! | 6 ellipse | c.x, c.y, major.x, major.y, ratio, t0, t1 |
//! | 7 xline, 8 ray | p.x, p.y, dir.x, dir.y |
//! | 9 spline | points, closed |
//! | 10 text | p.x, p.y, height, rotation, text |
//! | 11 dimension | a.x, a.y, b.x, b.y, offset, height, text?, style?, hasAngle, angle, hasC, c.x, c.y |
//! | 12 hatch | points, hatch holes, pattern type, angle, spacing |
//!
//! `layer`, `text` and `style` are indices into the strings (−1: none);
//! `label` is 1 for a non-empty label. `points` is a count n and 2n
//! coordinates; a `path` is points and bulges (a count, −1 for none, then
//! the values); `holes` is a count (−1 for none) of paths; `hatch holes` a
//! count (−1 for none) of point lists.

use super::Store;
use crate::entity::{HatchPattern, Shape};
use crate::geom::arrangement::Ring;
use crate::vec2::Vec2;

/// A path's points, bulges and holes.
type Path = (Vec<Vec2>, Option<Vec<f64>>, Option<Vec<Ring>>);

struct Reader<'a> {
    nums: &'a [f64],
    at: usize,
    strings: &'a [String],
}

impl Reader<'_> {
    fn num(&mut self) -> Result<f64, String> {
        let v = *self
            .nums
            .get(self.at)
            .ok_or_else(|| format!("paket {}. sayıda bitti", self.at))?;
        self.at += 1;
        Ok(v)
    }

    /// A count or an index: a whole number, −1 for "none".
    fn int(&mut self) -> Result<Option<usize>, String> {
        let v = self.num()?;
        if v == -1.0 {
            return Ok(None);
        }
        // No count or index can exceed what was sent.
        let limit = self.nums.len().max(self.strings.len()) as f64;
        if v >= 0.0 && v.fract() == 0.0 && v <= limit {
            Ok(Some(v as usize))
        } else {
            Err(format!(
                "paketin {}. sayısı geçersiz bir sayaç ({v})",
                self.at
            ))
        }
    }

    fn count(&mut self) -> Result<usize, String> {
        self.int()?
            .ok_or_else(|| format!("paketin {}. sayısı sayaç olmalı", self.at))
    }

    fn flag(&mut self) -> Result<bool, String> {
        Ok(self.num()? != 0.0)
    }

    fn pt(&mut self) -> Result<Vec2, String> {
        Ok(Vec2::new(self.num()?, self.num()?))
    }

    fn points(&mut self) -> Result<Vec<Vec2>, String> {
        let n = self.count()?;
        (0..n).map(|_| self.pt()).collect()
    }

    fn values(&mut self) -> Result<Option<Vec<f64>>, String> {
        match self.int()? {
            None => Ok(None),
            Some(n) => (0..n)
                .map(|_| self.num())
                .collect::<Result<Vec<_>, _>>()
                .map(Some),
        }
    }

    fn string(&mut self) -> Result<Option<String>, String> {
        match self.int()? {
            None => Ok(None),
            Some(i) => self.strings.get(i).cloned().map(Some).ok_or_else(|| {
                format!(
                    "paketin {}. sayısı olmayan bir metni gösteriyor ({i})",
                    self.at
                )
            }),
        }
    }

    fn path(&mut self) -> Result<Path, String> {
        let pts = self.points()?;
        let bulges = self.values()?;
        let holes = match self.int()? {
            None => None,
            Some(n) => Some(
                (0..n)
                    .map(|_| {
                        Ok(Ring {
                            pts: self.points()?,
                            bulges: self.values()?,
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()?,
            ),
        };
        Ok((pts, bulges, holes))
    }

    fn shape(&mut self, kind: f64) -> Result<Shape, String> {
        if !(kind >= 0.0 && kind <= 12.0 && kind.fract() == 0.0) {
            return Err(format!(
                "paketin {}. sayısı bilinmeyen bir nesne türü ({kind})",
                self.at
            ));
        }
        Ok(match kind as i64 {
            0 => {
                let p = self.pt()?;
                let has_z = self.flag()?;
                let z = self.num()?;
                Shape::Point {
                    p,
                    z: has_z.then_some(z),
                }
            }
            1 => Shape::Line {
                a: self.pt()?,
                b: self.pt()?,
            },
            2 | 3 => {
                let (pts, bulges, holes) = self.path()?;
                if kind == 2.0 {
                    Shape::Polyline { pts, bulges, holes }
                } else {
                    Shape::Polygon { pts, bulges, holes }
                }
            }
            4 => Shape::Circle {
                c: self.pt()?,
                r: self.num()?,
            },
            5 => Shape::Arc {
                c: self.pt()?,
                r: self.num()?,
                a0: self.num()?,
                a1: self.num()?,
            },
            6 => Shape::Ellipse {
                c: self.pt()?,
                major: self.pt()?,
                ratio: self.num()?,
                t0: self.num()?,
                t1: self.num()?,
            },
            7 => Shape::Xline {
                p: self.pt()?,
                dir: self.pt()?,
            },
            8 => Shape::Ray {
                p: self.pt()?,
                dir: self.pt()?,
            },
            9 => Shape::Spline {
                pts: self.points()?,
                closed: self.flag()?,
            },
            10 => Shape::Text {
                p: self.pt()?,
                height: self.num()?,
                rotation: self.num()?,
                text: self.string()?.unwrap_or_default(),
            },
            11 => {
                let a = self.pt()?;
                let b = self.pt()?;
                let offset = self.num()?;
                let height = self.num()?;
                let text = self.string()?;
                let style = self.string()?;
                let has_angle = self.flag()?;
                let angle = self.num()?;
                let has_c = self.flag()?;
                let c = self.pt()?;
                Shape::Dimension {
                    a,
                    b,
                    offset,
                    height,
                    text,
                    style,
                    angle: has_angle.then_some(angle),
                    c: has_c.then_some(c),
                }
            }
            12 => {
                let ring = self.points()?;
                let holes = match self.int()? {
                    None => None,
                    Some(n) => Some(
                        (0..n)
                            .map(|_| self.points())
                            .collect::<Result<Vec<_>, _>>()?,
                    ),
                };
                Shape::Hatch {
                    ring,
                    holes,
                    pattern: HatchPattern {
                        kind: self.string()?.unwrap_or_default(),
                        angle: self.num()?,
                        spacing: self.num()?,
                    },
                }
            }
            _ => {
                return Err(format!(
                    "paketin {}. sayısı bilinmeyen bir nesne türü ({kind})",
                    self.at
                ));
            }
        })
    }
}

impl Store {
    /// Adds or replaces packed objects in order (see the module's table);
    /// `strings` holds the layer ids and texts the numbers point at.
    pub fn put_packed(&mut self, nums: &[f64], strings: &[String]) -> Result<usize, String> {
        let mut r = Reader {
            nums,
            at: 0,
            strings,
        };
        let mut n = 0;
        while r.at < nums.len() {
            let id = r.num()?;
            let layer = r.string()?.unwrap_or_default();
            let label = r.flag()?;
            let kind = r.num()?;
            let shape = r.shape(kind)?;
            self.put(id, &layer, label, shape);
            n += 1;
        }
        self.maybe_rebuild();
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_every_kind_like_the_json() {
        let strings: Vec<String> = ["a", "Ada 104", "linear", "lines"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let nums = [
            // A polygon with bulges and a hole on layer "a", labelled.
            1.0,
            0.0,
            1.0,
            3.0,
            4.0,
            0.0,
            0.0,
            10.0,
            0.0,
            10.0,
            10.0,
            0.0,
            10.0,
            4.0,
            0.5,
            0.0,
            0.0,
            0.0,
            1.0,
            3.0,
            4.0,
            4.0,
            6.0,
            4.0,
            5.0,
            6.0,
            -1.0,
            // A text.
            2.0,
            0.0,
            0.0,
            10.0,
            1.0,
            2.0,
            3.0,
            45.0,
            1.0,
            // A linear dimension with an angle and no centre.
            3.0,
            0.0,
            0.0,
            11.0,
            0.0,
            0.0,
            10.0,
            0.0,
            2.0,
            0.5,
            -1.0,
            2.0,
            1.0,
            90.0,
            0.0,
            f64::NAN,
            f64::NAN,
            // A hatch with one hole.
            4.0,
            0.0,
            0.0,
            12.0,
            3.0,
            0.0,
            0.0,
            4.0,
            0.0,
            4.0,
            4.0,
            1.0,
            3.0,
            1.0,
            1.0,
            2.0,
            1.0,
            2.0,
            2.0,
            3.0,
            45.0,
            1.0,
        ];
        let mut s = Store::new();
        assert_eq!(s.put_packed(&nums, &strings), Ok(4));
        let mut j = Store::new();
        j.put_json(
            r#"[{"id":1,"layerId":"a","label":"7","kind":"polygon","pts":[{"x":0,"y":0},{"x":10,"y":0},{"x":10,"y":10},{"x":0,"y":10}],"bulges":[0.5,0,0,0],"holes":[{"pts":[{"x":4,"y":4},{"x":6,"y":4},{"x":5,"y":6}]}]},
                {"id":2,"layerId":"a","kind":"text","p":{"x":1,"y":2},"height":3,"rotation":45,"text":"Ada 104"},
                {"id":3,"layerId":"a","kind":"dimension","a":{"x":0,"y":0},"b":{"x":10,"y":0},"offset":2,"height":0.5,"style":"linear","angle":90},
                {"id":4,"layerId":"a","kind":"hatch","ring":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4}],"holes":[[{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":2}]],"pattern":{"type":"lines","angle":45,"spacing":1}}]"#,
        )
        .unwrap();
        for id in [1.0, 2.0, 3.0, 4.0] {
            let (a, b) = (s.get(id).unwrap(), j.get(id).unwrap());
            assert_eq!(a.shape, b.shape, "{id}");
            assert_eq!(a.bounds, b.bounds);
            assert_eq!((a.label, a.layer), (b.label, b.layer));
        }
    }

    #[test]
    fn a_short_or_bad_pack_is_an_error_not_a_panic() {
        let strings = vec!["a".to_string()];
        let mut s = Store::new();
        assert!(s.put_packed(&[1.0, 0.0, 0.0, 1.0, 0.0], &strings).is_err());
        assert!(
            s.put_packed(&[1.0, 5.0, 0.0, 4.0, 0.0, 0.0, 1.0], &strings)
                .is_err()
        );
        assert!(s.put_packed(&[1.0, 0.0, 0.0, 99.0], &strings).is_err());
        assert!(
            s.put_packed(&[1.0, 0.0, 0.0, 2.0, 1e300], &strings)
                .is_err()
        );
        assert!(s.put_packed(&[1.0, 0.0, 0.0, 2.0, 0.5], &strings).is_err());
        assert!(
            s.put_packed(&[1.0, 0.0, 0.0, f64::NAN, 1.0, 2.0], &strings)
                .is_err()
        );
        assert_eq!(s.put_packed(&[], &strings), Ok(0));
    }
}
