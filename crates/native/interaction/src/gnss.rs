//! The GNSS import's points (docs/adr/0169 §6; the web's
//! `model/gnssImport.ts`): a GNSS file's positions of the chosen kinds moved
//! from WGS 84 into the project's system the way Koordinat dönüştür moves
//! them (docs/adr/0167, the project's datum choices taken), named, with
//! their fix and heights as attributes and their ellipsoidal height, when
//! the file gives one, as their elevation: a height whose datum the file
//! does not say is never taken for it. The shared cases are
//! fixtures/gnss/v1/import.json (scripts/fixtures/gnss_import_cases.py, with
//! PROJ).

use std::collections::BTreeMap;

use kentos_contracts::{Entity, EntityBase, GnssPoint, LineError, PointEntity, Vec2 as Wire};
use kentos_geometry_core::crs::{Choice, Datum, System, Unreached, format_dd, transform_in};

use crate::Vec2;
use crate::format::fixed;
use crate::second::accuracy_text;

/// What the import takes: the kinds of points, and how the unnamed are
/// named (the prefix, then a number from `start`).
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    pub kinds: Vec<String>,
    pub prefix: String,
    pub start: u32,
}

/// A point as the import writes it, and its line in the file.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub line: u32,
    pub name: String,
    pub p: Vec2,
    pub z: Option<f64>,
    pub attrs: BTreeMap<String, String>,
}

/// The points to write, and those the project's system does not reach,
/// with why.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    pub placed: Vec<Placed>,
    pub skipped: Vec<LineError>,
}

/// What is said of a point the project's system does not reach; `{line}`,
/// `{name}` and `{why}` are filled in.
pub const SKIPPED: &str =
    "Satır {line}: {name} noktası projenin sistemine çevrilemedi ({why}); eklenmedi.";

/// A point's source as its Kaynak attribute names it.
pub fn source(kind: &str) -> &'static str {
    match kind {
        "wpt" => "GPX yol noktası",
        "rtept" => "GPX rota noktası",
        "trkpt" => "GPX iz noktası",
        "gga" => "NMEA GGA",
        _ => "GNSS",
    }
}

/// Why a point is not reached, in a few words.
fn why(u: Unreached) -> &'static str {
    match u {
        Unreached::Outside => "sistemin ulaştığı yerin dışında",
        Unreached::NoLink => "datumlardan birinin WGS 84'e dönüşümü yok",
        Unreached::NoGrid => "datum dönüşümünün ızgarası bu cihazda yok",
        Unreached::OutsideGrid => "datum dönüşümünün ızgarasının dışında",
    }
}

/// The points of `options.kinds` in `to` (named `system`), in the file's order.
pub fn place(
    points: &[GnssPoint],
    options: &Options,
    to: &System,
    system: &str,
    choices: &[Choice],
) -> Plan {
    let wgs84 = System::Geographic {
        datum: Datum::Wgs84,
    };
    let mut plan = Plan::default();
    let mut number = options.start;
    for g in points.iter().filter(|g| options.kinds.contains(&g.kind)) {
        let moved = match transform_in(&wgs84, to, Vec2::new(g.lon, g.lat), choices) {
            Ok(moved) => moved,
            Err(u) => {
                let name = g.name.as_deref().unwrap_or("Adsız");
                let message = SKIPPED
                    .replace("{line}", &g.line.to_string())
                    .replace("{why}", why(u))
                    .replace("{name}", name);
                plan.skipped.push(LineError {
                    line: g.line,
                    message,
                });
                continue;
            }
        };
        let name = match &g.name {
            Some(n) if !n.is_empty() => n.clone(),
            _ => {
                let n = format!("{}{number}", options.prefix);
                number = number.saturating_add(1);
                n
            }
        };
        let mut attrs = BTreeMap::from([
            ("Ad".to_owned(), name.clone()),
            ("Tür".to_owned(), "GNSS noktası".to_owned()),
            ("Kaynak".to_owned(), source(&g.kind).to_owned()),
        ]);
        let mut put = |key: &str, value: Option<String>| {
            if let Some(v) = value {
                attrs.insert(key.to_owned(), v);
            }
        };
        put("Çözüm", g.fix.clone());
        put("Uydu", g.satellites.map(|s| s.to_string()));
        put("HDOP", g.hdop.map(|h| fixed(h, 1)));
        put("Zaman", g.time.clone());
        put("Enlem", Some(format_dd(g.lat, true, 9)));
        put("Boylam", Some(format_dd(g.lon, false, 9)));
        put(
            "Elipsoit yüksekliği (m)",
            g.ellipsoidal.map(|h| fixed(h, 3)),
        );
        put("Yükseklik (dosyada, m)", g.height.map(|h| fixed(h, 3)));
        put("Geoit ayrımı (m)", g.geoid.map(|h| fixed(h, 3)));
        put(
            "Dönüşüm",
            Some(format!("WGS 84 → {system}: {}", accuracy_text(&moved))),
        );
        plan.placed.push(Placed {
            line: g.line,
            name,
            p: moved.point,
            z: g.ellipsoidal,
            attrs,
        });
    }
    plan
}

/// The placed points as the import writes them: on the source layer `""`,
/// which the window sends to its target layer (`exchange::apply`), their
/// names as labels.
pub fn entities(placed: &[Placed]) -> Vec<Entity> {
    placed
        .iter()
        .map(|pt| {
            Entity::Point(PointEntity {
                base: EntityBase {
                    id: 0,
                    layer_id: String::new(),
                    color: None,
                    attrs: pt.attrs.clone(),
                    label: Some(pt.name.clone()),
                    symbol: None,
                    line_weight: None,
                    label_pins: Vec::new(),
                },
                p: Wire {
                    x: pt.p.x,
                    y: pt.p.y,
                },
                z: pt.z,
                parts: None,
            })
        })
        .collect()
}
