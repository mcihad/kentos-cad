//! Where Raster ekle puts a raster (docs/adr/0204 §1, §8), one rule for both
//! platforms (the web through its raster worker): a raster placed by its
//! file (GeoTIFF) or its world file goes where they say when its system is
//! the project's; one whose system is another's is refused (no reprojection;
//! docs/adr/0046); one whose system is not known goes there only when the
//! user says it is the project's; one placed by nothing goes unplaced, in
//! the middle of the view, its pixel a thousandth of the view's shorter
//! side, to be placed by Raster oturt.

use serde::Serialize;

use super::source::RasterInfo;

/// What the rule says of a raster and a project.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "rule")]
pub enum Rule {
    /// Its file's system is the project's: it goes where its file says.
    Same { srid: u32 },
    /// Its file names no system: it goes where its file says once the user
    /// says the system is the project's (`srid` 0 then).
    Unknown,
    /// Its file's system is another: refused.
    Other { srid: u32 },
    /// Nothing places it: it goes in the middle of the view, unplaced.
    Unplaced,
}

/// The rule for a raster the window read, in a project of `project_srid` (0: none).
pub fn rule(info: &RasterInfo, project_srid: u32) -> Rule {
    if info.affine.is_none() {
        return Rule::Unplaced;
    }
    match info.epsg {
        Some(srid) if srid == project_srid => Rule::Same { srid },
        Some(srid) => Rule::Other { srid },
        None => Rule::Unknown,
    }
}

/// An unplaced raster's affine: north up, centred on the view (`min_x`,
/// `min_y`, `max_x`, `max_y`), its pixel a thousandth of the view's shorter
/// side (1 when the view has no size).
pub fn unplaced(width: u32, height: u32, view: [f64; 4]) -> [f64; 6] {
    let [x0, y0, x1, y1] = view;
    let short = (x1 - x0).abs().min((y1 - y0).abs());
    let s = if short.is_finite() && short > 0.0 {
        short / 1000.0
    } else {
        1.0
    };
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let (cx, cy) = if cx.is_finite() && cy.is_finite() {
        (cx, cy)
    } else {
        (0.0, 0.0)
    };
    [
        cx - f64::from(width) * s / 2.0,
        s,
        0.0,
        cy + f64::from(height) * s / 2.0,
        0.0,
        -s,
    ]
}

/// Where the raster goes and its system as the object keeps it: the file's
/// affine and system when the rule takes it (`confirmed`: the user said an
/// unknown system is the project's), the unplaced one when nothing places
/// it; none when the rule refuses it (another system, or an unknown one
/// not confirmed).
pub fn placement(
    info: &RasterInfo,
    project_srid: u32,
    confirmed: bool,
    view: [f64; 4],
) -> Option<([f64; 6], u32)> {
    match rule(info, project_srid) {
        Rule::Same { srid } => info.affine.map(|a| (a, srid)),
        Rule::Unknown if confirmed => info.affine.map(|a| (a, 0)),
        Rule::Unknown | Rule::Other { .. } => None,
        Rule::Unplaced => Some((unplaced(info.width, info.height, view), 0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_contracts::RasterSample;

    fn info(affine: Option<[f64; 6]>, epsg: Option<u32>) -> RasterInfo {
        RasterInfo {
            width: 400,
            height: 200,
            bands: 1,
            sample: RasterSample::U8,
            color: "gray".into(),
            alpha: false,
            compression: String::new(),
            tiled: false,
            big: false,
            overviews: 0,
            levels: 2,
            affine,
            placed_by: if affine.is_some() { "world" } else { "none" }.into(),
            epsg,
            geographic: false,
            nodata: None,
            needs_pyramid: false,
        }
    }

    const A: [f64; 6] = [487000.0, 0.5, 0.0, 4420100.0, 0.0, -0.5];
    const VIEW: [f64; 4] = [100.0, 200.0, 500.0, 400.0];

    /// The project's system: where the file says; another: refused; none said:
    /// only once the user says it is the project's; nothing placing it: unplaced.
    #[test]
    fn the_rule_of_the_systems() {
        assert_eq!(
            rule(&info(Some(A), Some(5256)), 5256),
            Rule::Same { srid: 5256 }
        );
        assert_eq!(
            placement(&info(Some(A), Some(5256)), 5256, false, VIEW),
            Some((A, 5256))
        );
        assert_eq!(
            rule(&info(Some(A), Some(4326)), 5256),
            Rule::Other { srid: 4326 }
        );
        assert_eq!(
            placement(&info(Some(A), Some(4326)), 5256, true, VIEW),
            None
        );
        // A local project (0) takes no file that names a system.
        assert_eq!(
            rule(&info(Some(A), Some(5256)), 0),
            Rule::Other { srid: 5256 }
        );
        assert_eq!(rule(&info(Some(A), None), 5256), Rule::Unknown);
        assert_eq!(placement(&info(Some(A), None), 5256, false, VIEW), None);
        assert_eq!(
            placement(&info(Some(A), None), 5256, true, VIEW),
            Some((A, 0))
        );
        assert_eq!(rule(&info(None, None), 5256), Rule::Unplaced);
    }

    /// An unplaced raster: north up, in the middle of the view, its pixel a
    /// thousandth of the view's shorter side.
    #[test]
    fn an_unplaced_raster_sits_in_the_middle_of_the_view() {
        let a = unplaced(400, 200, VIEW);
        assert_eq!(
            a,
            [
                300.0 - 400.0 * 0.2 / 2.0,
                0.2,
                0.0,
                300.0 + 200.0 * 0.2 / 2.0,
                0.0,
                -0.2
            ]
        );
        assert_eq!(
            placement(&info(None, None), 5256, false, VIEW),
            Some((a, 0))
        );
        // A view without a size: a metre a pixel round the origin.
        assert_eq!(
            unplaced(2, 2, [0.0, 0.0, 0.0, 0.0]),
            [-1.0, 1.0, 0.0, 1.0, 0.0, -1.0]
        );
        let json = serde_json::to_string(&rule(&info(Some(A), Some(5256)), 5256)).expect("JSON");
        assert_eq!(json, r#"{"rule":"same","srid":5256}"#);
    }
}
