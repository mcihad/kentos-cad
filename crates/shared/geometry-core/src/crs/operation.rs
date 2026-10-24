//! A transformation as a record and with heights (docs/adr/0519 §2): the steps a point takes from one system to
//! another (the projections' inverses and forwards, a local system's plane, the datum shift's Helmert, null and
//! grid steps with their EPSG codes or the project's names and stated accuracies), a PROJ pipeline that runs the
//! same, the grids it reads, the area of use; and the point moved with an ellipsoidal height, which rides through
//! the Earth-centred steps as PROJ's 3D pipelines carry it. Heights of another kind are not this module's: they
//! stay out of the datum shift (the pipeline keeps them with `push`/`pop`), and the vertical rules are the
//! command's. The reference is PROJ itself: `scripts/fixtures/crs_vertical_cases.py` runs each pipeline through
//! pyproj and compares it with the EPSG operation PROJ picks.

use super::datum::{self, Path, Step};
use super::{System, Unreached};
use crate::vec2::Vec2;

/// What a step is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepKind {
    Conversion,
    Helmert,
    Null,
    Grid,
    /// A geoid model: an orthometric height made ellipsoidal, or back (docs/adr/0631 §1).
    Geoid,
    /// A velocity grid: the point moved to another epoch (docs/adr/0631 §2).
    Velocity,
}

/// A grid a step reads: its name as the caller gives it and the file PROJ reads it from.
#[derive(Clone, Debug, PartialEq)]
pub struct GridStep {
    pub name: String,
    pub file: String,
}

/// What rides a transformation besides its horizontal way (docs/adr/0631): the source's orthometric heights made
/// ellipsoidal by its geoid, the point moved along a velocity grid by `years`, the target's heights made orthometric
/// by its geoid.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Extras {
    pub from_geoid: Option<GridStep>,
    pub to_geoid: Option<GridStep>,
    pub velocity: Option<(GridStep, f64)>,
}

/// A step: what it is, its name, whether it runs reversed, its stated accuracy (m).
#[derive(Clone, Debug, PartialEq)]
pub struct OpStep {
    pub kind: StepKind,
    pub name: String,
    pub reverse: bool,
    pub accuracy: Option<f64>,
}

/// The EPSG operations' area of use (all four of the registry's ways are EPSG's Türkiye ones).
pub const TURKIYE: (&str, [f64; 4]) = (
    "Türkiye (Turkey) - onshore and offshore.",
    [25.62, 34.42, 44.83, 43.45],
);

/// A transformation as a record.
#[derive(Clone, Debug, PartialEq)]
pub struct Operation {
    /// The datum steps' names joined by “ + ”; empty when the datum stays.
    pub id: String,
    pub steps: Vec<OpStep>,
    /// A PROJ pipeline that runs the same (`+proj=pipeline +step …`), x east or longitude first, degrees in and out
    /// on a geographic system.
    pub pipeline: String,
    /// The ids (SHA-256) of the grids it reads, in their order.
    pub grids: Vec<String>,
    pub area: Option<(&'static str, [f64; 4])>,
    /// The datum steps' summed accuracy (m), to the millimetre; none when a step's is not known.
    pub accuracy: Option<f64>,
    pub unofficial: bool,
}

fn n(v: f64) -> String {
    format!("{v}")
}

fn ellipsoid((a, rf): (f64, f64)) -> String {
    format!("+a={} +rf={}", n(a), n(rf))
}

fn tm_params(s: &System) -> String {
    let System::Tm {
        datum,
        latitude_of_origin,
        central_meridian,
        scale_factor,
        false_easting,
        false_northing,
    } = s
    else {
        return String::new();
    };
    format!(
        "+proj=tmerc +lat_0={} +lon_0={} +k={} +x_0={} +y_0={} {}",
        n(latitude_of_origin.unwrap_or(0.0)),
        n(*central_meridian),
        n(*scale_factor),
        n(*false_easting),
        n(*false_northing),
        ellipsoid(datum.ellipsoid())
    )
}

const WEBMERC: &str = "+proj=webmerc +a=6378137 +rf=298.257223563";

fn affine(plane: &super::Plane) -> String {
    let [a, b, c, d, e, f] = plane.coefficients();
    format!(
        "+proj=affine +xoff={} +yoff={} +s11={} +s12={} +s21={} +s22={}",
        n(c),
        n(f),
        n(a),
        n(b),
        n(d),
        n(e)
    )
}

/// The steps from the system's coordinates to latitude and longitude in radians.
fn unproject(s: &System, pipe: &mut Vec<String>, steps: &mut Vec<OpStep>) {
    let conversion = |name: String, reverse: bool| OpStep {
        kind: StepKind::Conversion,
        name,
        reverse,
        accuracy: Some(0.0),
    };
    match s {
        System::Geographic { .. } => pipe.push("+proj=unitconvert +xy_in=deg +xy_out=rad".into()),
        System::Tm {
            central_meridian, ..
        } => {
            pipe.push(format!("+inv {}", tm_params(s)));
            steps.push(conversion(format!("TM {}°", n(*central_meridian)), true));
        }
        System::Mercator {} => {
            pipe.push(format!("+inv {WEBMERC}"));
            steps.push(conversion("Pseudo-Mercator".into(), true));
        }
        System::Local { base, plane } => {
            pipe.push(affine(plane));
            steps.push(conversion("Yerel düzlem".into(), false));
            unproject(base, pipe, steps);
        }
    }
}

/// The steps from latitude and longitude in radians to the system's coordinates.
fn project(s: &System, pipe: &mut Vec<String>, steps: &mut Vec<OpStep>) {
    let conversion = |name: String, reverse: bool| OpStep {
        kind: StepKind::Conversion,
        name,
        reverse,
        accuracy: Some(0.0),
    };
    match s {
        System::Geographic { .. } => pipe.push("+proj=unitconvert +xy_in=rad +xy_out=deg".into()),
        System::Tm {
            central_meridian, ..
        } => {
            pipe.push(tm_params(s));
            steps.push(conversion(format!("TM {}°", n(*central_meridian)), false));
        }
        System::Mercator {} => {
            pipe.push(WEBMERC.into());
            steps.push(conversion("Pseudo-Mercator".into(), false));
        }
        System::Local { base, plane } => {
            project(base, pipe, steps);
            pipe.push(format!("+inv {}", affine(plane)));
            steps.push(conversion("Yerel düzlem".into(), true));
        }
    }
}

pub(crate) fn way(
    from: &System,
    to: &System,
    choices: &[datum::Choice],
) -> Result<Path, Unreached> {
    datum::path(&from.datum(), &to.datum(), choices).map_err(|gap| match gap {
        datum::Gap::NoLink => Unreached::NoLink,
        datum::Gap::NoGrid => Unreached::NoGrid,
    })
}

/// The record of the way from `from` to `to` with the project's datum choices. `keep_height` keeps the third
/// coordinate out of the datum shift (`push`/`pop`): heights that are not ellipsoidal.
pub fn operation(
    from: &System,
    to: &System,
    choices: &[datum::Choice],
    keep_height: bool,
) -> Result<Operation, Unreached> {
    operation_with(from, to, choices, keep_height, &Extras::default())
}

/// [`operation`] with geoid models and a velocity grid (docs/adr/0631): PROJ's `vgridshift` (multiplier 1 from the
/// source's orthometric heights, −1 to the target's) and `deformation` (`+dt` years, in Earth-centred coordinates
/// of the source's ellipsoid) at the latitude and longitude stage, the velocity before the datum shift.
pub fn operation_with(
    from: &System,
    to: &System,
    choices: &[datum::Choice],
    keep_height: bool,
    extras: &Extras,
) -> Result<Operation, Unreached> {
    let path = way(from, to, choices)?;
    let mut pipe = Vec::new();
    let mut steps = Vec::new();
    unproject(from, &mut pipe, &mut steps);
    let shifts = !path.steps.is_empty() || extras.velocity.is_some();
    if shifts && keep_height {
        pipe.push("+proj=push +v_3".into());
        // The 2D way shifts the datum at height 0, as the ADRs' 2D transforms do; the height waits aside.
        pipe.push("+proj=set +v_3=0".into());
    }
    let unknown = |kind, name: String| OpStep {
        kind,
        name,
        reverse: false,
        accuracy: None,
    };
    if let Some(g) = &extras.from_geoid {
        pipe.push(format!("+proj=vgridshift +grids={} +multiplier=1", g.file));
        steps.push(unknown(StepKind::Geoid, g.name.clone()));
    }
    if let Some((g, years)) = &extras.velocity {
        let e = ellipsoid(from.datum().ellipsoid());
        pipe.push(format!("+proj=cart {e}"));
        pipe.push(format!(
            "+proj=deformation +grids={} +dt={} {e}",
            g.file,
            n(*years)
        ));
        pipe.push(format!("+inv +proj=cart {e}"));
        steps.push(unknown(StepKind::Velocity, g.name.clone()));
    }
    let mut ellipsoid_now = from.datum().ellipsoid();
    let mut geographic = true;
    let mut grids = Vec::new();
    for ((step, lands), (name, accuracy)) in path.steps.iter().zip(&path.names) {
        let (kind, reverse) = match step {
            Step::Helmert(h, reverse) => {
                if geographic {
                    pipe.push(format!("+proj=cart {}", ellipsoid(ellipsoid_now)));
                    geographic = false;
                }
                let convention = match h.convention {
                    super::Convention::PositionVector => "position_vector",
                    super::Convention::CoordinateFrame => "coordinate_frame",
                };
                pipe.push(format!(
                    "{}+proj=helmert +x={} +y={} +z={} +rx={} +ry={} +rz={} +s={} +convention={convention}",
                    if *reverse { "+inv " } else { "" },
                    n(h.translation[0]),
                    n(h.translation[1]),
                    n(h.translation[2]),
                    n(h.rotation[0]),
                    n(h.rotation[1]),
                    n(h.rotation[2]),
                    n(h.scale)
                ));
                (StepKind::Helmert, *reverse)
            }
            Step::Null => {
                if !geographic {
                    pipe.push(format!("+inv +proj=cart {}", ellipsoid(ellipsoid_now)));
                    geographic = true;
                }
                (StepKind::Null, false)
            }
            Step::Grid(_, reverse, id) => {
                if !geographic {
                    pipe.push(format!("+inv +proj=cart {}", ellipsoid(ellipsoid_now)));
                    geographic = true;
                }
                pipe.push(format!(
                    "{}+proj=hgridshift +grids={id}.gsb",
                    if *reverse { "+inv " } else { "" }
                ));
                grids.push(id.clone());
                (StepKind::Grid, *reverse)
            }
        };
        steps.push(OpStep {
            kind,
            name: name.clone(),
            reverse,
            accuracy: *accuracy,
        });
        ellipsoid_now = *lands;
    }
    if !geographic {
        pipe.push(format!("+inv +proj=cart {}", ellipsoid(ellipsoid_now)));
    }
    if shifts && keep_height {
        pipe.push("+proj=pop +v_3".into());
    }
    if let Some(g) = &extras.to_geoid {
        pipe.push(format!("+proj=vgridshift +grids={} +multiplier=-1", g.file));
        steps.push(unknown(StepKind::Geoid, g.name.clone()));
    }
    project(to, &mut pipe, &mut steps);
    let epsg = path.names.iter().any(|(name, _)| name.starts_with("EPSG:"));
    let mut pipeline = String::from("+proj=pipeline");
    for p in pipe {
        pipeline.push_str(" +step ");
        pipeline.push_str(&p);
    }
    Ok(Operation {
        id: path.via.join(" + "),
        steps,
        pipeline,
        grids,
        area: epsg.then_some(TURKIYE),
        // A velocity grid's accuracy is not known: neither is the way's.
        accuracy: path.accuracy.filter(|_| extras.velocity.is_none()),
        unofficial: path.unofficial,
    })
}

/// `p` of `from` at the ellipsoidal height `height` (m) in `to`: the point and its ellipsoidal height there.
pub fn transform_height_in(
    from: &System,
    to: &System,
    p: Vec2,
    height: f64,
    choices: &[datum::Choice],
) -> Result<(Vec2, f64), Unreached> {
    let path = way(from, to, choices)?;
    let (lat, lon) = from.unproject(p).ok_or(Unreached::Outside)?;
    let (lat, lon, h) = path
        .run_height(from.datum().ellipsoid(), lat, lon, height)
        .ok_or(Unreached::OutsideGrid)?;
    Ok((to.project(lat, lon).ok_or(Unreached::Outside)?, h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crs::Datum;

    fn tm(datum: Datum, cm: f64) -> System {
        System::Tm {
            datum,
            latitude_of_origin: None,
            central_meridian: cm,
            scale_factor: 1.0,
            false_easting: 500_000.0,
            false_northing: 0.0,
        }
    }

    #[test]
    fn a_projection_alone_is_one_inverse_and_one_forward() {
        let op =
            operation(&tm(Datum::Turef, 30.0), &tm(Datum::Turef, 33.0), &[], true).expect("a way");
        assert_eq!(op.id, "");
        assert_eq!(op.accuracy, Some(0.0));
        assert_eq!(op.steps.len(), 2);
        assert_eq!(
            op.pipeline,
            "+proj=pipeline +step +inv +proj=tmerc +lat_0=0 +lon_0=30 +k=1 +x_0=500000 +y_0=0 +a=6378137 \
             +rf=298.257222101 +step +proj=tmerc +lat_0=0 +lon_0=33 +k=1 +x_0=500000 +y_0=0 +a=6378137 \
             +rf=298.257222101"
        );
        assert_eq!(op.area, None);
    }

    #[test]
    fn ed50_to_turef_is_two_helmerts_and_their_area() {
        let op =
            operation(&tm(Datum::Ed50, 30.0), &tm(Datum::Turef, 30.0), &[], false).expect("a way");
        assert_eq!(op.id, "EPSG:1783 + EPSG:5260");
        assert_eq!(op.accuracy, Some(2.1));
        assert!(op.unofficial);
        assert_eq!(op.area, Some(TURKIYE));
        let kinds: Vec<_> = op.steps.iter().map(|s| (s.kind, s.reverse)).collect();
        assert_eq!(
            kinds,
            [
                (StepKind::Conversion, true),
                (StepKind::Helmert, false),
                (StepKind::Helmert, true),
                (StepKind::Conversion, false)
            ]
        );
        assert!(!op.pipeline.contains("push"));
        assert!(op.pipeline.contains("+inv +proj=helmert +x=0.023"));
    }

    #[test]
    fn an_ellipsoidal_height_rides_the_shift() {
        let ed50 = System::Geographic { datum: Datum::Ed50 };
        let wgs = System::Geographic {
            datum: Datum::Wgs84,
        };
        let (p, h) = transform_height_in(&ed50, &wgs, Vec2::new(32.8597, 39.9334), 1000.0, &[])
            .expect("a point");
        // PROJ: ED50 to WGS 84 (30) in 3D gives 1040.3406805936 m (scripts/fixtures/crs_vertical_cases.py).
        assert!((h - 1_040.340_680_593_6).abs() < 1e-6, "{h}");
        assert!((p.x - 32.859_363_516_103).abs() < 1e-11, "{p:?}");
        // The null step leaves it.
        let turef = System::Geographic {
            datum: Datum::Turef,
        };
        let (_, h) =
            transform_height_in(&turef, &wgs, Vec2::new(32.0, 39.0), 812.5, &[]).expect("a point");
        assert_eq!(h, 812.5);
    }
}
