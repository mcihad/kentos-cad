//! Each known point's own place in the project (plan/map-cad.md MAP-03, docs/adr/4120), for `map.survey.query`'s
//! `places`: a known point's published latitude, longitude and ellipsoidal height carried into the project the way an
//! accepted point's are (docs/adr/1340 §3). The epoch decision is that of the campaign of the first baseline on its
//! mark (the archive's order), else the project's; a point at another epoch goes there first by its own velocity when
//! it has one (docs/adr/2520, IERS Conventions 2010 §4.1), else along that campaign's velocity grid by the chain
//! itself, and says how far it moved east, north and up. Then the coordinate chain takes it into the project. Its
//! mark's accepted point, when it has one, is compared with it; a point the chain or the epoch cannot carry says why.
//! Reads only: nothing is written.

use kentos_commands::{KnownMoved, KnownMovedBy, MapSurvey, SurveyKnown, SurveyKnownPlace};
use kentos_domain::Document;
use kentos_geometry_core::survey::adjust::gnss;

use crate::map_survey::{decision, place};
use crate::map_survey_network::{RAD, by_velocity, ecef, ellipsoid_of, epoch_text, moved_to, velocity_of};

/// Every known point's place, in the archive's order.
pub(crate) fn known_places(doc: &Document, survey: &MapSurvey) -> Vec<SurveyKnownPlace> {
    survey.known.iter().map(|k| place_of(doc, survey, k)).collect()
}

/// One known point's place, or why it has none.
fn place_of(doc: &Document, survey: &MapSurvey, k: &SurveyKnown) -> SurveyKnownPlace {
    let campaign = survey
        .baselines
        .iter()
        .find(|b| b.from == k.mark || b.to == k.mark)
        .and_then(|b| survey.campaign(&b.campaign));
    let target = decision(doc, campaign);
    let point = survey.points.iter().find(|p| p.mark == k.mark);
    let mut row = SurveyKnownPlace {
        id: k.id.clone(),
        mark: k.mark.clone(),
        x: None,
        y: None,
        z: None,
        chain: None,
        epoch: None,
        moved: None,
        point: point.map(|p| p.id.clone()),
        dx: None,
        dy: None,
        dz: None,
        why: None,
    };
    let given = (k.lat, k.lon, k.height);
    // Its epoch first: by its own velocity, else along the campaign's velocity grid (docs/adr/2520).
    let (at, epoch) = match (k.epoch, target) {
        (Some(from), Some(to)) if from != to => {
            let Some(shape) = ellipsoid_of(k.srid) else {
                row.why = Some(format!(
                    "EPSG:{} kayıttaki coğrafi bir sistem değil; bilinen noktanın çerçevesini (4326 WGS 84, 5252 TUREF) verin.",
                    k.srid
                ));
                return row;
            };
            let (now, by) = match (k.velocity, velocity_of(campaign)) {
                (Some(v), _) => (by_velocity(given, shape, v, to - from), KnownMovedBy::Velocity),
                (None, Some(grid)) => match moved_to(doc, &grid, given, k.srid, from, to) {
                    Ok(now) => (now, KnownMovedBy::Grid),
                    Err(stop) => {
                        row.why = Some(stop.into_error().message);
                        return row;
                    }
                },
                (None, None) => {
                    row.why = Some(format!(
                        "“{}” bilinen noktası {} epokunda, projenin epok kararı {}: projeye taşımak için noktanın kendi hızı (yer merkezli VX, VY, VZ) ya da arazi ölçümünün hız ızgarası (Proje ayarları › Izgaralar) gerekir.",
                        k.id,
                        epoch_text(from),
                        epoch_text(to)
                    ));
                    return row;
                }
            };
            let (a, b) = (ecef(given, shape), ecef(now, shape));
            let r = gnss::enu_rows(given.0 / RAD, given.1 / RAD);
            let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let shift = [0, 1, 2].map(|i| r[i][0] * d[0] + r[i][1] * d[1] + r[i][2] * d[2]);
            row.moved = Some(KnownMoved { id: k.id.clone(), mark: k.mark.clone(), by, from, years: to - from, shift });
            (now, Some(to))
        }
        _ => (given, k.epoch),
    };
    match place(doc, campaign, at, k.srid, epoch, "places") {
        Ok(p) => {
            row.x = Some(p.x);
            row.y = Some(p.y);
            row.z = p.z;
            row.chain = Some(p.chain);
            row.epoch = p.target.or(epoch);
            if let Some(pt) = point {
                row.dx = Some(p.x - pt.result.x);
                row.dy = Some(p.y - pt.result.y);
                row.dz = p.z.zip(pt.result.z).map(|(a, b)| a - b);
            }
        }
        Err(stop) => row.why = Some(stop.into_error().message),
    }
    row
}
