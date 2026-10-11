//! KML and KMZ out (docs/adr/0222 §4): the objects moved from the project's
//! system into WGS 84 longitude and latitude (the projections, the datum
//! shift and the project's datum choices, as Koordinat dönüştür moves a
//! point), a Folder per layer (its groups as Folders around it, outermost
//! first) and a Placemark per object: its geometry as GeoJSON writes it
//! (`export_geom`: curves sampled 72 steps a turn, hatches as their areas,
//! rings counter-clockwise outside and clockwise in holes, an insert its
//! block's objects in a MultiGeometry), a text a Point named by its text; its
//! name its label (else its Ad attribute), its attributes ExtendedData, its
//! colour and line weight a shared Style. A geometry whose every vertex has an
//! elevation is written with it, `absolute`; any other on the ground.
//! Longitudes and latitudes take ten decimals (a hundredth of a millimetre);
//! elevations are written as they are (the shortest decimal of the value). A KMZ is the KML as `doc.kml` in a zip archive.

use std::collections::HashMap;
use std::fmt::Write as _;

use kentos_contracts::{Entity, ExportReport, KmlLayer, KmlWriteInput, Vec2};
use kentos_geometry_core::crs::{Choice, Datum, System, Unreached, transform_in};
use serde::{Deserialize, Deserializer};

use super::style::{MM_PER_PIXEL, to_kml};
use crate::blocks::Placing;
use crate::dxf::{Definitions, Objects};
use crate::export_geom::{Geometry, Words, geometry};
use crate::gis::Ring;
use crate::num::{decimals, plain};
use crate::report::Report;
use crate::zip;

/// KML's words for what it cannot hold.
pub(crate) const KML: Words = Words {
    ring_turn: "KML'nin kuralına çevrildi (dış sınır saat yönünün tersine, delikler saat yönünde); köşeler aynı",
    curve: "KML'de eğri yok: uygulamanın kendi örneklemesiyle (turda 72 adım) çizgi olarak yazıldı",
    path_holes: "açık çizginin delikleri KML'de gösterilemez; yazılmadı",
    arc_area: "KML'de eğri yok: yayları uygulamanın kendi örneklemesiyle (turda 72 adım) yazıldı",
    hatch: "deseni KML'de gösterilemez; sınırı alan (Polygon) olarak yazıldı",
    text: "bloğun içindeki yazı KML'de gösterilemez; yazılmadı",
    dimension: "KML'de ölçü nesnesi yok; yazılmadı",
    table: "KML'de tablo nesnesi yok; yazılmadı",
    image: "KML'de resim nesnesi yok; yazılmadı",
    cloud: "KML'de nokta bulutu yok; yazılmadı",
    raster: "KML'de raster yok; yazılmadı",
    leader: "kırık çizgisi çizgi (LineString) olarak yazıldı; ok başı ve not KML'de gösterilemez",
    block_empty: "bloğunda KML'ye yazılabilecek nesne yok; yazılmadı",
    block: "yerleştirmeler bloklarının nesneleriyle, açılmış olarak (MultiGeometry) yazıldı; KML'de blok yoktur",
    infinite: "uçsuz doğru ve ışın KML'de gösterilemez; yazılmadı",
    measure: "KML köşenin ölçüsünü (M) taşımaz; çizgi yazıldı, ölçüleri yazılmadı",
    mixed_z: None,
};

/// What a text becomes.
const TEXT: &str = "yazının metni nokta (Point) yer iminin adı olarak yazıldı";

/// WGS 84 longitude and latitude.
pub(crate) const WGS84: System = System::Geographic {
    datum: Datum::Wgs84,
};

/// Decimals of a longitude or latitude (1e-10° is about 0.01 mm).
pub(crate) const DEGREE_DECIMALS: usize = 10;

/// Text in an XML element or attribute.
pub(crate) fn escape(t: &str, out: &mut String) {
    for c in t.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            // XML 1.0 has no other control characters.
            c if (c as u32) < 0x20 && !matches!(c, '\t' | '\n' | '\r') => {}
            c => out.push(c),
        }
    }
}

/// The elevations of a run when every vertex has one.
fn all_heights(r: &Ring) -> Option<Vec<f64>> {
    let zs = r.zs.as_ref().filter(|zs| zs.len() == r.pts.len())?;
    zs.iter().copied().collect()
}

/// Whether every vertex of the geometry has an elevation.
fn three_d(g: &Geometry) -> bool {
    match g {
        Geometry::Point(_, z) => z.is_some(),
        Geometry::Points(ps) => ps.iter().all(|(_, z)| z.is_some()),
        Geometry::Line(r) => all_heights(r).is_some(),
        Geometry::Lines(rs) | Geometry::Area(rs) => rs.iter().all(|r| all_heights(r).is_some()),
        Geometry::Areas(parts) => parts.iter().flatten().all(|r| all_heights(r).is_some()),
        Geometry::Collection(members) => members.iter().all(three_d),
    }
}

fn has_heights(g: &Geometry) -> bool {
    let some = |r: &Ring| r.zs.iter().flatten().any(Option::is_some);
    match g {
        Geometry::Point(_, z) => z.is_some(),
        Geometry::Points(ps) => ps.iter().any(|(_, z)| z.is_some()),
        Geometry::Line(r) => some(r),
        Geometry::Lines(rs) | Geometry::Area(rs) => rs.iter().any(some),
        Geometry::Areas(parts) => parts.iter().flatten().any(some),
        Geometry::Collection(members) => members.iter().any(has_heights),
    }
}

fn position(p: Vec2, z: Option<f64>, out: &mut String) {
    out.push_str(&decimals(p.x, DEGREE_DECIMALS));
    out.push(',');
    out.push_str(&decimals(p.y, DEGREE_DECIMALS));
    if let Some(z) = z {
        out.push(',');
        out.push_str(&plain(z));
    }
}

fn coordinates(r: &Ring, z: bool, out: &mut String) {
    out.push_str("<coordinates>");
    let heights = if z { all_heights(r) } else { None };
    for (i, p) in r.pts.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        position(*p, heights.as_ref().map(|h| h[i]), out);
    }
    out.push_str("</coordinates>");
}

fn mode(z: bool, out: &mut String) {
    if z {
        out.push_str("<altitudeMode>absolute</altitudeMode>");
    }
}

fn polygon(rings: &[Ring], z: bool, out: &mut String) {
    out.push_str("<Polygon>");
    mode(z, out);
    for (i, r) in rings.iter().enumerate() {
        out.push_str(if i == 0 {
            "<outerBoundaryIs><LinearRing>"
        } else {
            "<innerBoundaryIs><LinearRing>"
        });
        coordinates(r, z, out);
        out.push_str(if i == 0 {
            "</LinearRing></outerBoundaryIs>"
        } else {
            "</LinearRing></innerBoundaryIs>"
        });
    }
    out.push_str("</Polygon>");
}

fn write_geometry(g: &Geometry, z: bool, out: &mut String) {
    match g {
        Geometry::Point(p, h) => {
            out.push_str("<Point>");
            mode(z, out);
            out.push_str("<coordinates>");
            position(*p, if z { *h } else { None }, out);
            out.push_str("</coordinates></Point>");
        }
        Geometry::Points(ps) => {
            out.push_str("<MultiGeometry>");
            for (p, h) in ps {
                write_geometry(&Geometry::Point(*p, *h), z, out);
            }
            out.push_str("</MultiGeometry>");
        }
        Geometry::Line(r) => {
            out.push_str("<LineString>");
            mode(z, out);
            coordinates(r, z, out);
            out.push_str("</LineString>");
        }
        Geometry::Lines(rs) => {
            out.push_str("<MultiGeometry>");
            for r in rs {
                write_geometry(&Geometry::Line(r.clone()), z, out);
            }
            out.push_str("</MultiGeometry>");
        }
        Geometry::Area(rings) => polygon(rings, z, out),
        Geometry::Areas(parts) => {
            out.push_str("<MultiGeometry>");
            for rings in parts {
                polygon(rings, z, out);
            }
            out.push_str("</MultiGeometry>");
        }
        Geometry::Collection(members) => {
            out.push_str("<MultiGeometry>");
            for m in members {
                write_geometry(m, z, out);
            }
            out.push_str("</MultiGeometry>");
        }
    }
}

/// What a Placemark's style is: its colour, its width (px) and what it styles.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct StyleKey {
    color: String,
    /// Hundredths of a pixel.
    width: u32,
    area: bool,
    point: bool,
}

fn kind_of(g: &Geometry) -> (bool, bool) {
    match g {
        Geometry::Point(..) | Geometry::Points(_) => (false, true),
        Geometry::Area(_) | Geometry::Areas(_) => (true, false),
        Geometry::Collection(members) => members
            .iter()
            .map(kind_of)
            .fold((false, false), |(a, p), (b, q)| (a || b, p || q)),
        _ => (false, false),
    }
}

/// The Placemark's name: a text's text; any other object's label, else its Ad attribute.
fn name_of(e: &Entity) -> Option<&str> {
    if let Entity::Text(t) = e {
        return Some(t.text.as_str()).filter(|t| !t.is_empty());
    }
    let b = e.base();
    b.label.as_deref().filter(|l| !l.is_empty()).or_else(|| {
        b.attrs
            .get("Ad")
            .map(String::as_str)
            .filter(|a| !a.is_empty())
    })
}

/// An object as written: its geometry's KML and its Style's place.
struct Placed<'a> {
    entity: &'a Entity,
    body: String,
    style: usize,
}

/// One more of `u` among the objects left out.
pub(crate) fn tally(list: &mut Vec<(Unreached, u32)>, u: Unreached) {
    match list.iter_mut().find(|(w, _)| *w == u) {
        Some((_, n)) => *n += 1,
        None => list.push((u, 1)),
    }
}

/// The objects one of whose vertices WGS 84 does not reach, said by why.
pub(crate) fn say_unreached(rep: &mut Report, mut list: Vec<(Unreached, u32)>) {
    list.sort_by_key(|(u, _)| u.as_str());
    for (u, n) in list {
        rep.skip_n(
            "Çevrilemeyen nesne",
            &format!(
                "bir köşesi WGS 84'e çevrilemedi ({}); yazılmadı",
                crate::reproject::why(u)
            ),
            0,
            n,
        );
    }
}

/// Writes the objects as KML (or KMZ), moved from `from` into WGS 84 with `choices`.
pub fn write(input: &KmlWriteInput, from: &System, choices: &[Choice]) -> (Vec<u8>, ExportReport) {
    let mut rep = Report::default();
    let blocks = Placing::new(&input.blocks);
    let layers: HashMap<&str, &KmlLayer> =
        input.layers.iter().map(|l| (l.id.as_str(), l)).collect();
    // The layers' objects as written, in the input's order; the layers in theirs, then any not listed.
    let mut per: HashMap<&str, Vec<Placed>> = HashMap::new();
    let mut order: Vec<&str> = input.layers.iter().map(|l| l.id.as_str()).collect();
    let mut styles: Vec<StyleKey> = Vec::new();
    let mut unreached: Vec<(Unreached, u32)> = Vec::new();
    let (mut mixed, mut heights) = (0u32, 0u32);
    for e in &input.entities {
        let mut g = match e {
            Entity::Text(t) => {
                rep.note("Yazı", TEXT, 0);
                Geometry::Point(t.p, None)
            }
            _ => match geometry(e, &blocks, &mut rep, &KML) {
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
        let z = three_d(&g);
        if z {
            heights += 1;
        } else if has_heights(&g) {
            mixed += 1;
        }
        let mut body = String::new();
        write_geometry(&g, z, &mut body);
        let b = e.base();
        let layer = layers.get(b.layer_id.as_str());
        let color = b
            .color
            .as_deref()
            .and_then(|c| to_kml(c, 0xff))
            .or_else(|| layer.and_then(|l| to_kml(&l.color, 0xff)))
            .unwrap_or_else(|| "ff000000".to_owned());
        let weight = b
            .line_weight
            .or_else(|| layer.and_then(|l| l.line_weight))
            .unwrap_or(0.25);
        // Pixels to two decimals: each standard weight reads back as itself (style::line_weight);
        // the thinnest line (0) is one pixel.
        let width = if weight > 0.0 {
            ((weight / MM_PER_PIXEL) * 100.0).round().max(1.0) as u32
        } else {
            100
        };
        let (area, point) = kind_of(&g);
        let key = StyleKey {
            color,
            width,
            area,
            point,
        };
        let style = match styles.iter().position(|s| *s == key) {
            Some(i) => i,
            None => {
                styles.push(key);
                styles.len() - 1
            }
        };
        let id = b.layer_id.as_str();
        if !layers.contains_key(id) && !per.contains_key(id) {
            order.push(id);
        }
        per.entry(id).or_default().push(Placed {
            entity: e,
            body,
            style,
        });
        rep.count(e.kind());
    }
    say_unreached(&mut rep, unreached);
    rep.note_n(
        "Kot",
        "kotları deniz seviyesinden yükseklik (absolute) olarak, olduğu gibi yazıldı",
        0,
        heights,
    );
    rep.note_n(
        "Kotsuz köşe",
        "bir kısım köşesi kotsuz nesne yere oturtularak, kotsuz yazıldı",
        0,
        mixed,
    );
    let mut out = String::with_capacity(256 + input.entities.len() * 200);
    out.push_str(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<kml xmlns=\"http://www.opengis.net/kml/2.2\">\n<Document>\n<name>",
    );
    escape(&input.name, &mut out);
    out.push_str("</name>\n");
    for (i, s) in styles.iter().enumerate() {
        let _ = write!(out, "<Style id=\"s{}\">", i + 1);
        if s.point {
            let _ = write!(out, "<IconStyle><color>{}</color></IconStyle>", s.color);
        }
        let _ = write!(
            out,
            "<LineStyle><color>{}</color><width>{}</width></LineStyle>",
            s.color,
            decimals(f64::from(s.width) / 100.0, 2)
        );
        if s.area {
            // The fill two fifths opaque, so what lies under shows.
            let _ = write!(
                out,
                "<PolyStyle><color>66{}</color><fill>1</fill><outline>1</outline></PolyStyle>",
                &s.color[2..]
            );
        }
        out.push_str("</Style>\n");
    }
    // The layers in order, their groups as Folders around them.
    let mut open: Vec<&str> = Vec::new();
    for id in order {
        let Some(list) = per.get(id) else { continue };
        let layer = layers.get(id);
        let path: &[String] = layer.map_or(&[], |l| l.path.as_slice());
        let common = open
            .iter()
            .zip(path)
            .take_while(|(a, b)| **a == b.as_str())
            .count();
        while open.len() > common {
            open.pop();
            out.push_str("</Folder>\n");
        }
        for g in &path[common..] {
            out.push_str("<Folder><name>");
            escape(g, &mut out);
            out.push_str("</name>\n");
            open.push(g);
        }
        out.push_str("<Folder><name>");
        escape(layer.map_or(id, |l| l.name.as_str()), &mut out);
        out.push_str("</name>\n");
        for p in list {
            out.push_str("<Placemark>");
            if let Some(n) = name_of(p.entity) {
                out.push_str("<name>");
                escape(n, &mut out);
                out.push_str("</name>");
            }
            let _ = write!(out, "<styleUrl>#s{}</styleUrl>", p.style + 1);
            let attrs = &p.entity.base().attrs;
            if !attrs.is_empty() {
                out.push_str("<ExtendedData>");
                for (k, v) in attrs {
                    out.push_str("<Data name=\"");
                    escape(k, &mut out);
                    out.push_str("\"><value>");
                    escape(v, &mut out);
                    out.push_str("</value></Data>");
                }
                out.push_str("</ExtendedData>");
            }
            out.push_str(&p.body);
            out.push_str("</Placemark>\n");
        }
        out.push_str("</Folder>\n");
    }
    while open.pop().is_some() {
        out.push_str("</Folder>\n");
    }
    out.push_str("</Document>\n</kml>\n");
    if !input.kmz {
        return (out.into_bytes(), rep.export());
    }
    match zip::write(&[("doc.kml", out.as_bytes())]) {
        Ok(bytes) => (bytes, rep.export()),
        Err(e) => {
            rep.skip("KMZ", &e, 0);
            (Vec::new(), rep.export())
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Input {
    entities: Objects,
    layers: Vec<KmlLayer>,
    name: String,
    #[serde(default)]
    blocks: Definitions,
    #[serde(default)]
    kmz: bool,
}

/// A `KmlWriteInput` read with the DXF writer's visitor for the objects.
pub struct WriteInput(pub KmlWriteInput);

impl<'de> Deserialize<'de> for WriteInput {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<WriteInput, D::Error> {
        let i = Input::deserialize(d)?;
        Ok(WriteInput(KmlWriteInput {
            entities: i.entities.0,
            layers: i.layers,
            name: i.name,
            blocks: i.blocks.0,
            kmz: i.kmz,
        }))
    }
}
