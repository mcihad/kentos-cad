//! The drawing's heights made ellipsoidal for the scales a measured length passes (plan/map-cad.md MAP-02,
//! docs/adr/2900): the geoid's height above the ellipsoid (N) at points of the project's map, by what the project says
//! its heights are (Proje ayarları › Kotlar, docs/adr/0519): 0 where they are ellipsoidal, the project's geoid model's
//! N where they are orthometric. `cad.crs.factors` takes the drawing's heights through it (`elevations`), and
//! Kutupsal alım and `cad.survey.points` reduce each shot at its own line's height with it; one rule for the window,
//! Python and MCP. Nothing is guessed: a project that does not say what its heights are, an orthometric one without a
//! geoid model or a model whose grid is not on this device gets the reason, never a zero N.

use kentos_commands::{HeightKind, ProjectSettings};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::crs::geoid;
use kentos_geometry_core::survey::polar::{PolarInput, polar_survey};

use crate::crs_models::{self, Missing, ModelKind};

/// Why the drawing's heights cannot be made ellipsoidal.
#[derive(Clone, Debug, PartialEq)]
pub enum Unmade {
    /// The project does not say what its heights are.
    Unknown,
    /// Orthometric heights without a geoid model (or one named without its file's SHA-256).
    NoModel,
    /// The model's grid is not on this device, has changed or cannot be read: its words.
    Missing(String),
    /// The heights are not in metres (a vertical system in feet): the factors take metres.
    NotMetres(String),
    /// The project has no coordinate system to find a point's latitude and longitude with.
    NoSystem,
}

impl Unmade {
    /// What the windows and the refusals say.
    pub fn words(&self) -> String {
        match self {
            Unmade::Unknown => "Projenin kotlarının ne olduğu (elipsoit ya da ortometrik yükseklik) söylenmemiş: kotlardan elipsoit yüksekliği çıkarılmaz. Kotları Proje ayarları › Kotlar'da söyleyin.".into(),
            Unmade::NoModel => "Projenin kotları ortometrik ama geoit modeli (SHA-256'sıyla) yok: kotlar elipsoit yüksekliğine çevrilemez. Geoit modelini Proje ayarları › Kotlar'da seçin.".into(),
            Unmade::Missing(why) => why.clone(),
            Unmade::NotMetres(unit) => format!("Projenin kotları {unit} biriminde; çarpanlar metre ister. Kotları metre olan bir düşey sistemle verin."),
            Unmade::NoSystem => "Proje kendi koordinatlarında (yerel): noktanın enlemi ve boylamı, dolayısıyla geoit ayrımı bulunamaz.".into(),
        }
    }
}

/// The geoid's height above the ellipsoid at each point (none for a point beyond the system or outside the geoid's
/// grid), and the geoid model's name when the heights are orthometric; or why the drawing's heights cannot be made
/// ellipsoidal.
pub fn undulations(settings: &ProjectSettings, points: &[Vec2]) -> Result<(Vec<Option<f64>>, Option<String>), Unmade> {
    let Some(heights) = settings.heights.as_ref() else {
        return Err(Unmade::Unknown);
    };
    if heights.metres() != 1.0 {
        let unit = heights.vertical().map_or("başka bir", |v| v.unit);
        return Err(Unmade::NotMetres(unit.to_owned()));
    }
    match heights.kind {
        HeightKind::Unknown => Err(Unmade::Unknown),
        HeightKind::Ellipsoidal => Ok((vec![Some(0.0); points.len()], None)),
        HeightKind::Orthometric => {
            let Some((name, id)) = heights.geoid.as_ref().and_then(|g| Some((g.name.clone(), g.sha256.clone()?))) else {
                return Err(Unmade::NoModel);
            };
            crs_models::load(&id, ModelKind::Geoid, None).map_err(|why| {
                let reason = match why {
                    Missing::NotHere => "bu cihazın ızgara kitaplığında yok".to_owned(),
                    Missing::Changed => "kitaplıktaki dosya değişmiş (SHA-256'sı tutmuyor)".to_owned(),
                    Missing::Unread(r) => format!("okunamadı: {r}"),
                };
                Unmade::Missing(format!(
                    "“{name}” geoit modelinin ızgarası ({}…) {reason}. Izgarayı Proje ayarları › Izgaralar'a ekleyin; kotlar sıfır geoit ayrımıyla çevrilmez.",
                    &id[..id.len().min(12)]
                ))
            })?;
            let grid = geoid::get(&id).ok_or_else(|| Unmade::Missing(format!("“{name}” geoit modelinin ızgarası okunamadı.")))?;
            let system = kentos_project::systems::own(settings).and_then(|n| n.system).ok_or(Unmade::NoSystem)?;
            let ns = points
                .iter()
                .map(|&p| {
                    let (lat, lon) = system.geographic_of(p)?;
                    grid.at(lon, lat).map(|v| v[0])
                })
                .collect();
            Ok((ns, Some(name)))
        }
    }
}

/// What Kutupsal alım reduces its shots with (docs/adr/2900 §3): none where nothing is reduced at a line's own height
/// (no grid, no station height, no shot with a zenith angle); else the undulations at the station and at each shot's
/// point (the points found first by the grid's mean height: a few millimetres from the last, where N is the same to
/// far below a micrometre) with the geoid model's name, or why not, said in the window.
pub fn polar_undulations(settings: &ProjectSettings, input: &PolarInput) -> Option<Result<(Vec<f64>, Option<String>), String>> {
    if input.grid.is_none() || input.station_z.is_none() || input.shots.iter().all(|s| s.zenith.is_none()) {
        return None;
    }
    let first = PolarInput { undulations: None, ..input.clone() };
    let points = polar_survey(&first).ok()?;
    let mut at = vec![input.station];
    at.extend(points.iter().map(|p| p.p));
    Some(match undulations(settings, &at) {
        Err(why) => Err(why.words()),
        Ok((ns, model)) => match ns.iter().position(Option::is_none) {
            Some(0) => Err("Durulan nokta geoit modelinin ızgarasının dışında: çizgiler ortalama elipsoit yüksekliğinde indirildi.".into()),
            Some(k) => Err(format!("{k}. nokta geoit modelinin ızgarasının dışında: çizgiler ortalama elipsoit yüksekliğinde indirildi.")),
            None => Ok((ns.into_iter().flatten().collect(), model)),
        },
    })
}
