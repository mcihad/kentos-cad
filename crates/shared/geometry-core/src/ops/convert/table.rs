//! Semt ve uzunluktan çizgi and Tablodan elips (docs/adr/0218 §5, §7):
//! a line from a place along a bearing, an ellipse from its centre, semi-axes
//! and the bearing of its major axis. Bearings are a surveyor's: from north,
//! clockwise, in grads or degrees. Tablodan nokta (docs/adr/0223 §5): a point
//! from its east and north, moved from the table's system into the project's.

use crate::crs::{Choice, System, Unreached, parse_angle, transform_in};
use crate::entity::Shape;
use crate::jsmath::{cos, sin};
use crate::ops::cogo::read_number;
use crate::tools::point_text::js_trim;
use crate::vec2::Vec2;

/// Why a row gives nothing (§5–§7; docs/adr/0223 §5): a cell (by its place
/// in the row) empty, no number (docs/adr/0180's rule: a sign, digits and one
/// decimal point or comma; in a geographic system degrees, minutes and
/// seconds too), not more than zero where it must be, a latitude past ±90 or
/// a longitude past ±180; a line's two ends in one place; a point the move
/// into the project's system does not reach.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowFault {
    Empty(usize),
    NotNumber(usize),
    NotPositive(usize),
    SamePlace,
    OutOfRange(usize),
    Unreached(Unreached),
}

/// A row's cells as numbers; an empty cell is `None` where `optional`.
fn numbers<const N: usize>(
    cells: &[String],
    optional: [bool; N],
) -> Result<[Option<f64>; N], RowFault> {
    let mut out = [None; N];
    for k in 0..N {
        let text = cells.get(k).map_or("", String::as_str);
        if js_trim(text).is_empty() {
            if optional[k] {
                continue;
            }
            return Err(RowFault::Empty(k));
        }
        out[k] = Some(read_number(text).ok_or(RowFault::NotNumber(k))?);
    }
    Ok(out)
}

/// Semt ve uzunluktan çizgi's rows (§5): east, north, bearing and length
/// (more than zero), each a line.
pub fn bearing_rows(rows: &[Vec<String>], unit: &str) -> Vec<Result<[Vec2; 2], RowFault>> {
    rows.iter()
        .map(|r| {
            let [e, n, b, l] = numbers(r, [false; 4])?.map(|v| v.unwrap_or(0.0));
            if !(l > 0.0) {
                return Err(RowFault::NotPositive(3));
            }
            Ok(bearing_line(e, n, b, l, unit))
        })
        .collect()
}

/// XY'den çizgi's rows (§6): the start's east and north, the end's, each a
/// line unless its ends are one place.
pub fn xy_rows(rows: &[Vec<String>]) -> Vec<Result<[Vec2; 2], RowFault>> {
    rows.iter()
        .map(|r| {
            let [x0, y0, x1, y1] = numbers(r, [false; 4])?.map(|v| v.unwrap_or(0.0));
            let (a, b) = (Vec2::new(x0, y0), Vec2::new(x1, y1));
            if a == b {
                return Err(RowFault::SamePlace);
            }
            Ok([a, b])
        })
        .collect()
}

/// Tablodan elips's rows (§7): east, north, the two semi-axes (more than
/// zero) and the bearing (an empty cell north), each a whole ellipse.
pub fn ellipse_rows(rows: &[Vec<String>], unit: &str) -> Vec<Result<Shape, RowFault>> {
    rows.iter()
        .map(|r| {
            let [e, n, a, b, t] = numbers(r, [false, false, false, false, true])?;
            let (a, b) = (a.unwrap_or(0.0), b.unwrap_or(0.0));
            if !(a > 0.0) {
                return Err(RowFault::NotPositive(2));
            }
            if !(b > 0.0) {
                return Err(RowFault::NotPositive(3));
            }
            Ok(table_ellipse(
                e.unwrap_or(0.0),
                n.unwrap_or(0.0),
                a,
                b,
                t.unwrap_or(0.0),
                unit,
            ))
        })
        .collect()
}

/// A cell of a geographic system's coordinate: decimal degrees (a point or a
/// comma, docs/adr/0180's rule), else degrees, minutes and seconds
/// (`crs::parse_angle`: spaces or ° ′ ″, a hemisphere letter).
fn angle(text: &str) -> Option<f64> {
    read_number(text).or_else(|| parse_angle(text))
}

/// Tablodan nokta's rows (docs/adr/0223 §5): east and north (in a geographic
/// system longitude within ±180 and latitude within ±90, `angle`'s way), and
/// an elevation (an empty cell none), each a point. With `from` and `to` both
/// given and not the same, the point is moved from `from` into `to` with the
/// project's datum choices (`transform_in`); its elevation stays as it is.
/// Which system the cells are in: `from`, else `to` (none: a plane's numbers).
pub fn point_rows(
    rows: &[Vec<String>],
    from: Option<&System>,
    to: Option<&System>,
    choices: &[Choice],
) -> Vec<Result<(Vec2, Option<f64>), RowFault>> {
    let geographic = matches!(from.or(to), Some(System::Geographic { .. }));
    let way = match (from, to) {
        (Some(f), Some(t)) if f != t => Some((f, t)),
        _ => None,
    };
    rows.iter()
        .map(|r| {
            let cell = |k: usize| r.get(k).map_or("", String::as_str);
            let coordinate = |k: usize| -> Result<f64, RowFault> {
                let text = cell(k);
                if js_trim(text).is_empty() {
                    return Err(RowFault::Empty(k));
                }
                let v = if geographic {
                    angle(text)
                } else {
                    read_number(text)
                };
                let v = v.ok_or(RowFault::NotNumber(k))?;
                let most = if k == 0 { 180.0 } else { 90.0 };
                if geographic && !(v.abs() <= most) {
                    return Err(RowFault::OutOfRange(k));
                }
                Ok(v)
            };
            let (e, n) = (coordinate(0)?, coordinate(1)?);
            let z = if js_trim(cell(2)).is_empty() {
                None
            } else {
                Some(read_number(cell(2)).ok_or(RowFault::NotNumber(2))?)
            };
            let p = Vec2::new(e, n);
            let p = match way {
                Some((f, t)) => {
                    transform_in(f, t, p, choices)
                        .map_err(RowFault::Unreached)?
                        .point
                }
                None => p,
            };
            Ok((p, z))
        })
        .collect()
}

/// A bearing in radians from its value in `unit` (`deg` degrees, otherwise grads).
pub fn radians(value: f64, unit: &str) -> f64 {
    match unit {
        "deg" => value * std::f64::consts::PI / 180.0,
        _ => value * std::f64::consts::PI / 200.0,
    }
}

/// Semt ve uzunluktan çizgi (§5): from (east, north) along `bearing`
/// `length` long: its end (east + length·sin θ, north + length·cos θ).
pub fn bearing_line(east: f64, north: f64, bearing: f64, length: f64, unit: &str) -> [Vec2; 2] {
    let t = radians(bearing, unit);
    [
        Vec2::new(east, north),
        Vec2::new(east + length * sin(t), north + length * cos(t)),
    ]
}

/// Tablodan elips (§7): a whole ellipse centred at (east, north), its
/// semi-axes `a` and `b`, the larger along `bearing`; when `b` is larger the
/// two change places and the direction turns a right angle.
pub fn table_ellipse(east: f64, north: f64, a: f64, b: f64, bearing: f64, unit: &str) -> Shape {
    let mut t = radians(bearing, unit);
    let (major, minor) = if b > a {
        t += std::f64::consts::FRAC_PI_2;
        (b, a)
    } else {
        (a, b)
    };
    Shape::Ellipse {
        c: Vec2::new(east, north),
        major: Vec2::new(major * sin(t), major * cos(t)),
        ratio: minor / major,
        // A whole ellipse, as the drawing's Elips writes it.
        t0: 0.0,
        t1: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crs::Datum;

    fn row(cells: &[&str]) -> Vec<String> {
        cells.iter().map(|c| (*c).to_owned()).collect()
    }

    #[test]
    fn point_rows_read_plane_and_geographic_cells() {
        let plane = point_rows(
            &[
                row(&["452345,125", "4412345.5", "812.40"]),
                row(&["1", "2"]),
                row(&["", "2"]),
                row(&["1", "iki"]),
                row(&["1", "2", "x"]),
            ],
            None,
            None,
            &[],
        );
        assert_eq!(
            plane[0],
            Ok((Vec2::new(452_345.125, 4_412_345.5), Some(812.4)))
        );
        assert_eq!(plane[1], Ok((Vec2::new(1.0, 2.0), None)));
        assert_eq!(plane[2], Err(RowFault::Empty(0)));
        assert_eq!(plane[3], Err(RowFault::NotNumber(1)));
        assert_eq!(plane[4], Err(RowFault::NotNumber(2)));
        let wgs84 = System::Geographic {
            datum: Datum::Wgs84,
        };
        let geo = point_rows(
            &[
                row(&["32°51'54\"", "39 55 12.5 K"]),
                row(&["32,865", "-39.92"]),
                row(&["181", "39"]),
                row(&["32", "90.5"]),
            ],
            Some(&wgs84),
            None,
            &[],
        );
        let (p, _) = geo[0].expect("a point");
        assert!((p.x - (32.0 + 51.0 / 60.0 + 54.0 / 3600.0)).abs() < 1e-12);
        assert!((p.y - (39.0 + 55.0 / 60.0 + 12.5 / 3600.0)).abs() < 1e-12);
        assert_eq!(geo[1], Ok((Vec2::new(32.865, -39.92), None)));
        assert_eq!(geo[2], Err(RowFault::OutOfRange(0)));
        assert_eq!(geo[3], Err(RowFault::OutOfRange(1)));
    }

    #[test]
    fn point_rows_move_into_the_projects_system() {
        let wgs84 = System::Geographic {
            datum: Datum::Wgs84,
        };
        let tm33 = System::Tm {
            datum: Datum::Turef,
            latitude_of_origin: None,
            central_meridian: 33.0,
            scale_factor: 1.0,
            false_easting: 500_000.0,
            false_northing: 0.0,
        };
        let got = point_rows(
            &[row(&["32.865", "39.92", "900"])],
            Some(&wgs84),
            Some(&tm33),
            &[],
        );
        let want = transform_in(&wgs84, &tm33, Vec2::new(32.865, 39.92), &[])
            .expect("reached")
            .point;
        assert_eq!(got[0], Ok((want, Some(900.0))));
        let far = point_rows(&[row(&["150", "39"])], Some(&wgs84), Some(&tm33), &[]);
        assert!(matches!(far[0], Err(RowFault::Unreached(_)) | Ok(_)));
    }

    #[test]
    fn a_bearing_of_100_grads_is_east() {
        let [a, b] = bearing_line(10.0, 20.0, 100.0, 5.0, "grad");
        assert_eq!(a, Vec2::new(10.0, 20.0));
        assert!((b.x - 15.0).abs() < 1e-14 && (b.y - 20.0).abs() < 1e-14);
        let [_, c] = bearing_line(0.0, 0.0, 180.0, 2.0, "deg");
        assert!(c.x.abs() < 1e-15 && (c.y + 2.0).abs() < 1e-15);
    }

    #[test]
    fn the_larger_semi_axis_is_the_major_one() {
        let Shape::Ellipse { major, ratio, .. } = table_ellipse(0.0, 0.0, 3.0, 5.0, 0.0, "grad")
        else {
            panic!("an ellipse");
        };
        // 5 along east (north turned a right angle), 3 / 5.
        assert!((major.x - 5.0).abs() < 1e-14 && major.y.abs() < 1e-14);
        assert_eq!(ratio, 0.6);
    }
}
