//! The GNSS import's points (docs/adr/0169 §6; the web's
//! `model/gnssImport.ts`): a GNSS file's positions of the chosen kinds moved
//! from WGS 84 into the project's system the way Koordinat dönüştür moves
//! them (docs/adr/0167, the project's datum choices taken), named, with
//! their fix and heights as attributes and their ellipsoidal height, when
//! the file gives one, as their elevation: a height whose datum the file
//! does not say is never taken for it. Asked to, its routes, tracks and
//! NMEA log become polylines of those points (docs/adr/0222 §6), whatever
//! kinds of points are taken. The shared cases are
//! fixtures/gnss/v1/import.json (scripts/fixtures/gnss_import_cases.py, with
//! PROJ).

use std::collections::BTreeMap;

use kentos_contracts::{
    AreaPart, Entity, EntityBase, GnssLine, GnssPoint, LineError, PathEntity, PointEntity,
    Vec2 as Wire,
};
use kentos_geometry_core::crs::{Choice, Datum, System, Unreached, format_dd, transform_in};

use crate::Vec2;
use crate::format::fixed;
use crate::second::accuracy_text;

/// What the import takes: the kinds of points, how the unnamed are named
/// (the prefix, then a number from `start`), and whether the routes and
/// tracks come in as lines.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    pub kinds: Vec<String>,
    pub prefix: String,
    pub start: u32,
    pub lines: bool,
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

/// A route's, track's or log's line as the import writes it: its parts'
/// vertices (each with its point's ellipsoidal height), its name, its line
/// in the file and its attributes.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedLine {
    pub line: u32,
    pub name: Option<String>,
    pub parts: Vec<Vec<(Vec2, Option<f64>)>>,
    pub attrs: BTreeMap<String, String>,
}

/// The points and lines to write, and those the project's system does not
/// reach (or a line without a part of two points), with why.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    pub placed: Vec<Placed>,
    pub skipped: Vec<LineError>,
    pub lines: Vec<PlacedLine>,
    pub lines_skipped: Vec<LineError>,
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

/// What is said of a line without a part of two points, and of one the
/// project's system does not reach; `{line}`, `{name}` and `{why}` are filled in.
pub const THIN: &str = "Satır {line}: {name} çizgisinin iki noktalı parçası yok; eklenmedi.";
pub const LINE_SKIPPED: &str =
    "Satır {line}: {name} çizgisi projenin sistemine çevrilemedi ({why}); eklenmedi.";

/// A line's Tür, Kaynak and the name of one without its own.
fn line_kind(kind: &str) -> (&'static str, &'static str, &'static str) {
    match kind {
        "rte" => ("GNSS rotası", "GPX rotası", "Adsız rota"),
        "nmea" => ("GNSS izi", "NMEA kaydı", "NMEA kaydı"),
        _ => ("GNSS izi", "GPX izi", "Adsız iz"),
    }
}

/// The file's lines in `to` (named `system`), in the file's order: each part
/// of two points or more its points moved as the points are.
pub fn place_lines(
    points: &[GnssPoint],
    lines: &[GnssLine],
    to: &System,
    system: &str,
    choices: &[Choice],
) -> (Vec<PlacedLine>, Vec<LineError>) {
    let wgs84 = System::Geographic {
        datum: Datum::Wgs84,
    };
    let (mut placed, mut skipped) = (Vec::new(), Vec::new());
    let mut way: Option<String> = None;
    'lines: for l in lines {
        let (kind, source, unnamed) = line_kind(&l.kind);
        let name = l.name.as_deref().filter(|n| !n.is_empty());
        let shown = name.unwrap_or(unnamed);
        let say = |text: &str, why: &str| LineError {
            line: l.line,
            message: text
                .replace("{line}", &l.line.to_string())
                .replace("{why}", why)
                .replace("{name}", shown),
        };
        let parts: Vec<&Vec<u32>> = l.parts.iter().filter(|p| p.len() >= 2).collect();
        if parts.is_empty() {
            skipped.push(say(THIN, ""));
            continue;
        }
        let mut out = Vec::with_capacity(parts.len());
        for part in parts {
            let mut run = Vec::with_capacity(part.len());
            for &i in part {
                let Some(g) = points.get(i as usize) else {
                    continue;
                };
                match transform_in(&wgs84, to, Vec2::new(g.lon, g.lat), choices) {
                    Ok(moved) => {
                        way.get_or_insert_with(|| {
                            format!("WGS 84 → {system}: {}", accuracy_text(&moved))
                        });
                        run.push((moved.point, g.ellipsoidal));
                    }
                    Err(u) => {
                        skipped.push(say(LINE_SKIPPED, why(u)));
                        continue 'lines;
                    }
                }
            }
            out.push(run);
        }
        let mut attrs = BTreeMap::new();
        if let Some(n) = name {
            attrs.insert("Ad".to_owned(), n.to_owned());
        }
        attrs.insert("Tür".to_owned(), kind.to_owned());
        attrs.insert("Kaynak".to_owned(), source.to_owned());
        attrs.insert(
            "Nokta".to_owned(),
            out.iter().map(Vec::len).sum::<usize>().to_string(),
        );
        attrs.insert("Dönüşüm".to_owned(), way.clone().unwrap_or_default());
        placed.push(PlacedLine {
            line: l.line,
            name: name.map(str::to_owned),
            parts: out,
            attrs,
        });
    }
    (placed, skipped)
}

/// The points of `options.kinds` in `to` (named `system`), in the file's
/// order; and, asked to, the file's lines.
pub fn place_all(
    points: &[GnssPoint],
    lines: &[GnssLine],
    options: &Options,
    to: &System,
    system: &str,
    choices: &[Choice],
) -> Plan {
    let mut plan = place(points, options, to, system, choices);
    if options.lines {
        (plan.lines, plan.lines_skipped) = place_lines(points, lines, to, system, choices);
    }
    plan
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

/// The placed lines as the import writes them: polylines on the source
/// layer `""`, a part after the first a part of the polyline, their names as
/// labels.
pub fn line_entities(lines: &[PlacedLine]) -> Vec<Entity> {
    let run = |r: &[(Vec2, Option<f64>)]| -> (Vec<Wire>, Option<Vec<Option<f64>>>) {
        let pts = r.iter().map(|(p, _)| Wire { x: p.x, y: p.y }).collect();
        let zs: Vec<Option<f64>> = r.iter().map(|(_, z)| *z).collect();
        (pts, zs.iter().any(Option::is_some).then_some(zs))
    };
    lines
        .iter()
        .filter_map(|l| {
            let (first, rest) = l.parts.split_first()?;
            let (pts, zs) = run(first);
            let parts: Vec<AreaPart> = rest
                .iter()
                .map(|r| {
                    let (pts, zs) = run(r);
                    AreaPart {
                        ms: None,
                        pts,
                        bulges: None,
                        holes: None,
                        zs,
                    }
                })
                .collect();
            Some(Entity::Polyline(PathEntity {
                ms: None,
                base: EntityBase {
                    id: 0,
                    layer_id: String::new(),
                    color: None,
                    attrs: l.attrs.clone(),
                    label: l.name.clone(),
                    symbol: None,
                    line_weight: None,
                    label_pins: Vec::new(),
                    attachments: Vec::new(),
                },
                pts,
                bulges: None,
                holes: None,
                zs,
                parts: (!parts.is_empty()).then_some(parts),
                closed: false,
                widths: None,
            }))
        })
        .collect()
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
                    attachments: Vec::new(),
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
