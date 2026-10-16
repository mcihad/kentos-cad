//! GPX 1.1 out (docs/adr/0222 §5): the objects moved from the project's
//! system into WGS 84 latitude and longitude (as Koordinat dönüştür moves a
//! point, the project's datum choices taken). Points are waypoints (`wpt`;
//! each point of a multi-point object one), a text a waypoint named by its
//! text; lines are tracks (`trk`, a segment per path) or, asked to, routes
//! (`rte`, one per path); an area's outline and holes are a closed track's
//! segments whichever lines become (GPX has no areas). Curves, hatches and
//! inserts are what GeoJSON writes for them (`export_geom`). A waypoint's or a
//! line's name is the object's label, else its Ad attribute; its description
//! its Açıklama attribute and its type its Kod attribute; other attributes
//! are not written (GPX holds them only in a program's own extensions). A
//! vertex's elevation is its `ele`, as it is. Latitudes and longitudes take
//! ten decimals (about 0.01 mm). The schema's order is kept: waypoints, then
//! routes, then tracks.

use std::fmt::Write as _;

use kentos_contracts::{Entity, ExportReport, GpxWriteInput, Vec2};
use kentos_geometry_core::crs::{Choice, System, Unreached, transform_in};
use serde::{Deserialize, Deserializer};

use crate::blocks::Placing;
use crate::dxf::{Definitions, Objects};
use crate::export_geom::{Geometry, Words, geometry};
use crate::gis::Ring;
use crate::kml::write::{DEGREE_DECIMALS, WGS84, escape, say_unreached, tally};
use crate::num::{decimals, plain};
use crate::report::Report;

/// GPX's words for what it cannot hold.
pub(crate) const GPX: Words = Words {
    ring_turn: "halkanın yönü çevrildi (dış sınır saat yönünün tersine, delikler saat yönünde); köşeler aynı",
    curve: "GPX'te eğri yok: uygulamanın kendi örneklemesiyle (turda 72 adım) çizgi olarak yazıldı",
    path_holes: "açık çizginin delikleri GPX'te gösterilemez; yazılmadı",
    arc_area: "GPX'te eğri yok: yayları uygulamanın kendi örneklemesiyle (turda 72 adım) yazıldı",
    hatch: "deseni GPX'te gösterilemez; sınırı kapalı iz olarak yazıldı",
    text: "bloğun içindeki yazı GPX'te gösterilemez; yazılmadı",
    dimension: "GPX'te ölçü nesnesi yok; yazılmadı",
    table: "GPX'te tablo nesnesi yok; yazılmadı",
    image: "GPX'te resim nesnesi yok; yazılmadı",
    cloud: "GPX'te nokta bulutu yok; yazılmadı",
    record: "GPX'e geometrisiz kayıt yazılmaz; yazılmadı",
    raster: "GPX'te raster yok; yazılmadı",
    leader: "kırık çizgisi çizgi olarak yazıldı; ok başı ve not GPX'te gösterilemez",
    block_empty: "bloğunda GPX'e yazılabilecek nesne yok; yazılmadı",
    block: "yerleştirmeler bloklarının nesneleriyle, açılmış olarak yazıldı; GPX'te blok yoktur",
    infinite: "uçsuz doğru ve ışın GPX'te gösterilemez; yazılmadı",
    measure: "GPX köşenin ölçüsünü (M) taşımaz; iz yazıldı, ölçüleri yazılmadı",
    mixed_z: None,
};

/// What a text becomes.
const TEXT: &str = "yazının metni yol noktasının (wpt) adı olarak yazıldı";

/// What an area becomes.
const AREA: &str = "GPX'te alan yok: sınırı ve delikleri kapalı iz (trk) parçaları olarak yazıldı";

/// A waypoint, route or track's name, description and type.
struct Said<'a> {
    name: Option<&'a str>,
    desc: Option<&'a str>,
    kind: Option<&'a str>,
}

fn said(e: &Entity) -> Said<'_> {
    let b = e.base();
    let attr = |k: &str| b.attrs.get(k).map(String::as_str).filter(|v| !v.is_empty());
    let name = match e {
        Entity::Text(t) => Some(t.text.as_str()).filter(|t| !t.is_empty()),
        _ => b
            .label
            .as_deref()
            .filter(|l| !l.is_empty())
            .or_else(|| attr("Ad")),
    };
    Said {
        name,
        desc: attr("Açıklama"),
        kind: attr("Kod"),
    }
}

fn element(tag: &str, text: Option<&str>, out: &mut String) {
    if let Some(t) = text {
        let _ = write!(out, "<{tag}>");
        escape(t, out);
        let _ = write!(out, "</{tag}>");
    }
}

/// A point's element (`wpt`, `rtept`, `trkpt`) with its elevation; a waypoint also named and described.
fn point(tag: &str, p: Vec2, z: Option<f64>, s: Option<&Said>, out: &mut String) {
    let _ = write!(
        out,
        "<{tag} lat=\"{}\" lon=\"{}\"",
        decimals(p.y, DEGREE_DECIMALS),
        decimals(p.x, DEGREE_DECIMALS)
    );
    let named = s.is_some_and(|s| s.name.is_some() || s.desc.is_some() || s.kind.is_some());
    if z.is_none() && !named {
        out.push_str("/>");
        return;
    }
    out.push('>');
    if let Some(z) = z {
        let _ = write!(out, "<ele>{}</ele>", plain(z));
    }
    if let Some(s) = s {
        element("name", s.name, out);
        element("desc", s.desc, out);
        element("type", s.kind, out);
    }
    let _ = write!(out, "</{tag}>");
}

/// A path's points as `tag` elements.
fn run(tag: &str, r: &Ring, out: &mut String) {
    for (i, p) in r.pts.iter().enumerate() {
        let z = r.zs.as_ref().and_then(|zs| zs.get(i).copied().flatten());
        point(tag, *p, z, None, out);
    }
}

/// The three lists GPX keeps apart, as written.
#[derive(Default)]
struct Lists {
    waypoints: String,
    routes: String,
    tracks: String,
}

/// What an object adds: its points as waypoints, its paths as routes or a
/// track, its areas' rings as a track; whether it had an elevation and an area.
fn add(
    g: &Geometry,
    s: &Said,
    routes: bool,
    lists: &mut Lists,
    heights: &mut bool,
    area: &mut bool,
) {
    let mut paths: Vec<&Ring> = Vec::new();
    let mut rings: Vec<&Ring> = Vec::new();
    fn gather<'g>(
        g: &'g Geometry,
        points: &mut Vec<(Vec2, Option<f64>)>,
        paths: &mut Vec<&'g Ring>,
        rings: &mut Vec<&'g Ring>,
    ) {
        match g {
            Geometry::Point(p, z) => points.push((*p, *z)),
            Geometry::Points(ps) => points.extend(ps.iter().copied()),
            Geometry::Line(r) => paths.push(r),
            Geometry::Lines(rs) => paths.extend(rs),
            Geometry::Area(rs) => rings.extend(rs),
            Geometry::Areas(parts) => rings.extend(parts.iter().flatten()),
            Geometry::Collection(members) => {
                for m in members {
                    gather(m, points, paths, rings);
                }
            }
        }
    }
    let mut points = Vec::new();
    gather(g, &mut points, &mut paths, &mut rings);
    let has_z = |r: &&Ring| r.zs.iter().flatten().any(Option::is_some);
    *heights |= points.iter().any(|(_, z)| z.is_some())
        || paths.iter().any(has_z)
        || rings.iter().any(has_z);
    *area |= !rings.is_empty();
    for (p, z) in points {
        point("wpt", p, z, Some(s), &mut lists.waypoints);
        lists.waypoints.push('\n');
    }
    let head = |tag: &str, out: &mut String| {
        let _ = write!(out, "<{tag}>");
        element("name", s.name, out);
        element("desc", s.desc, out);
        element("type", s.kind, out);
    };
    if routes {
        for r in &paths {
            head("rte", &mut lists.routes);
            run("rtept", r, &mut lists.routes);
            lists.routes.push_str("</rte>\n");
        }
    }
    let segments: Vec<&Ring> = if routes {
        rings
    } else {
        paths.into_iter().chain(rings).collect()
    };
    if !segments.is_empty() {
        head("trk", &mut lists.tracks);
        for r in segments {
            lists.tracks.push_str("<trkseg>");
            run("trkpt", r, &mut lists.tracks);
            lists.tracks.push_str("</trkseg>");
        }
        lists.tracks.push_str("</trk>\n");
    }
}

/// Writes the objects as GPX 1.1, moved from `from` into WGS 84 with `choices`.
pub fn write(input: &GpxWriteInput, from: &System, choices: &[Choice]) -> (Vec<u8>, ExportReport) {
    let mut rep = Report::default();
    let blocks = Placing::new(&input.blocks);
    let mut lists = Lists::default();
    let mut unreached: Vec<(Unreached, u32)> = Vec::new();
    let (mut heights, mut areas, mut others) = (0u32, 0u32, 0u32);
    for e in &input.entities {
        let mut g = match e {
            Entity::Text(t) => {
                rep.note("Yazı", TEXT, 0);
                Geometry::Point(t.p, None)
            }
            _ => match geometry(e, &blocks, &mut rep, &GPX) {
                Some(g) => g,
                None => continue,
            },
        };
        let mut why = None;
        let moved = g.map(&mut |p| match transform_in(
            from,
            &WGS84,
            kentos_geometry_core::Vec2::new(p.x, p.y),
            choices,
        ) {
            Ok(t) => Some(Vec2 {
                x: t.point.x,
                y: t.point.y,
            }),
            Err(u) => {
                why = Some(u);
                None
            }
        });
        if !moved {
            if let Some(u) = why {
                tally(&mut unreached, u);
            }
            continue;
        }
        let s = said(e);
        let (mut z, mut area) = (false, false);
        add(&g, &s, input.routes, &mut lists, &mut z, &mut area);
        heights += u32::from(z);
        areas += u32::from(area);
        let kept = ["Ad", "Açıklama", "Kod"];
        others += u32::from(e.base().attrs.keys().any(|k| !kept.contains(&k.as_str())));
        rep.count(e.kind());
    }
    say_unreached(&mut rep, unreached);
    rep.note_n("Alan", AREA, 0, areas);
    rep.note_n(
        "Kot",
        "köşelerin kotları yükseklik (ele) olarak, olduğu gibi yazıldı",
        0,
        heights,
    );
    rep.note_n(
        "Öznitelik",
        "GPX'te yalnız ad (etiket ya da Ad), açıklama (Açıklama) ve tür (Kod) yazılır; öbür öznitelikler yazılmadı",
        0,
        others,
    );
    let mut out = String::with_capacity(
        200 + lists.waypoints.len() + lists.routes.len() + lists.tracks.len(),
    );
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<gpx version=\"1.1\" creator=\"KentOS CAD\" xmlns=\"http://www.topografix.com/GPX/1/1\">\n<metadata>");
    element(
        "name",
        Some(input.name.as_str()).filter(|n| !n.is_empty()),
        &mut out,
    );
    out.push_str("</metadata>\n");
    out.push_str(&lists.waypoints);
    out.push_str(&lists.routes);
    out.push_str(&lists.tracks);
    out.push_str("</gpx>\n");
    (out.into_bytes(), rep.export())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Input {
    entities: Objects,
    name: String,
    #[serde(default)]
    blocks: Definitions,
    #[serde(default)]
    routes: bool,
}

/// A `GpxWriteInput` read with the DXF writer's visitor for the objects.
pub struct WriteInput(pub GpxWriteInput);

impl<'de> Deserialize<'de> for WriteInput {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<WriteInput, D::Error> {
        let i = Input::deserialize(d)?;
        Ok(WriteInput(GpxWriteInput {
            entities: i.entities.0,
            name: i.name,
            blocks: i.blocks.0,
            routes: i.routes,
        }))
    }
}
