//! An XYZ file of positions (docs/adr/0545, plan/gis.md CBS-05): what `file.export.xyz` writes. One line per position,
//! `X Y Z` (east, north, elevation, in the project's system; the file names no system). The points' positions, or
//! every vertex of every object; an arc's middle point is no vertex and a closed ring's repeated last vertex is
//! written once. The geometry is the GeoPackage export's (`geopackage::geometry_of`), so a circle is its arc's end
//! points and an ellipse, a curve or a NURBS the vertices of the chords it is written as.

use std::collections::{BTreeMap, BTreeSet};

use kentos_commands::encoding::sha256_hex;
use kentos_commands::{Entity, ExportXyz, XyzExported, XyzSeparator, XyzVertices, XyzWithoutZ};
use kentos_domain::Document;
use kentos_geopackage::{Coord, Curve, Geometry};

use crate::geopackage;

/// What `file.export.xyz` makes before it writes: the file's bytes and the answer.
#[derive(Clone, Debug, PartialEq)]
pub struct Made {
    pub bytes: Vec<u8>,
    pub output: XyzExported,
}

fn same(a: &Coord, b: &Coord) -> bool {
    a.x == b.x && a.y == b.y
}

/// A curve's vertices: a line's, an arc's end points (not the middle ones, counted in `middles`), the pieces of a
/// compound end to end; a closed curve's repeated last vertex once.
fn curve_vertices(c: &Curve, out: &mut Vec<Coord>, middles: &mut u32) {
    let mut mine: Vec<Coord> = Vec::new();
    pieces(c, &mut mine, middles);
    if mine.len() > 2 && same(&mine[0], &mine[mine.len() - 1]) {
        mine.pop();
    }
    out.extend(mine);
}

fn pieces(c: &Curve, out: &mut Vec<Coord>, middles: &mut u32) {
    match c {
        Curve::Line(p) => push(out, p.iter()),
        Curve::Arc(p) => {
            *middles += p.len().saturating_sub(1) as u32 / 2;
            push(out, p.iter().step_by(2));
        }
        Curve::Compound(parts) => parts.iter().for_each(|p| pieces(p, out, middles)),
    }
}

/// Appends the positions, the first one left out when it is where the last one is (a piece starting where the
/// previous one ended).
fn push<'a>(out: &mut Vec<Coord>, pts: impl Iterator<Item = &'a Coord>) {
    for (i, p) in pts.enumerate() {
        if i == 0 && out.last().is_some_and(|l| same(l, p)) {
            continue;
        }
        out.push(*p);
    }
}

/// A geometry's positions for `vertices`, or the word of what it is when a position file has no line for it.
fn positions(g: &Geometry, vertices: XyzVertices, middles: &mut u32) -> Result<Vec<Coord>, &'static str> {
    let mut out = Vec::new();
    match (vertices, g) {
        (_, Geometry::Point(p)) => out.extend(p.iter().copied()),
        (_, Geometry::MultiPoint(p)) => out.extend(p.iter().copied()),
        (XyzVertices::Points, Geometry::Curve(_) | Geometry::MultiCurve(_)) => return Err("çizgi"),
        (XyzVertices::Points, Geometry::Polygon(_) | Geometry::MultiPolygon(_)) => return Err("alan"),
        (XyzVertices::Points, Geometry::Collection(_)) => return Err("başka nesne"),
        (XyzVertices::All, Geometry::Curve(c)) => curve_vertices(c, &mut out, middles),
        (XyzVertices::All, Geometry::MultiCurve(cs)) => {
            cs.iter().for_each(|c| curve_vertices(c, &mut out, middles))
        }
        (XyzVertices::All, Geometry::Polygon(rings)) => {
            rings.iter().for_each(|c| curve_vertices(c, &mut out, middles))
        }
        (XyzVertices::All, Geometry::MultiPolygon(polys)) => polys
            .iter()
            .flatten()
            .for_each(|c| curve_vertices(c, &mut out, middles)),
        (XyzVertices::All, Geometry::Collection(gs)) => {
            for g in gs {
                out.extend(positions(g, vertices, middles).unwrap_or_default());
            }
        }
    }
    Ok(out)
}

/// A number as the file writes it: the shortest text that reads back, or fixed places; never `-0`.
fn number(v: f64, decimals: Option<u8>) -> String {
    let s = match decimals {
        None => format!("{v}"),
        Some(n) => format!("{v:.*}", n as usize),
    };
    match s.strip_prefix('-') {
        Some(rest) if rest.chars().all(|c| c == '0' || c == '.') => rest.to_owned(),
        _ => s,
    }
}

/// The layers' positions as XYZ lines, or why none can be written: (code, message, path).
pub fn export(doc: &Document, input: &ExportXyz) -> Result<Made, (String, String, String)> {
    if input.decimals.is_some_and(|d| d > 15) {
        return Err((
            kentos_commands::codes::INVALID_INPUT.to_owned(),
            "Ondalık basamak sayısı 0 ile 15 arasında olmalı; 15'ten fazlası double'ın anlamlı basamaklarını aşar."
                .into(),
            "decimals".into(),
        ));
    }
    let ids = crate::shapefile::layer_ids(doc, &input.layers).map_err(|(i, id)| {
        (
            kentos_commands::codes::LAYER_NOT_FOUND.to_owned(),
            format!("“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."),
            format!("layers[{i}]"),
        )
    })?;
    let sep = match input.separator {
        XyzSeparator::Space => ' ',
        XyzSeparator::Comma => ',',
        XyzSeparator::Semicolon => ';',
        XyzSeparator::Tab => '\t',
    };
    let mut out = String::new();
    if input.header {
        out.push_str(&format!("X{sep}Y{sep}Z\n"));
    }
    let (mut points, mut objects, mut with_z) = (0u32, 0u32, 0u32);
    let (mut no_z, mut skipped_z, mut middles, mut chords) = (0u32, 0u32, 0u32, 0u32);
    let mut left: BTreeMap<String, u32> = BTreeMap::new();
    let mut lost: BTreeSet<String> = BTreeSet::new();
    for id in &ids {
        for e in doc.by_layer(id) {
            let entity: &Entity = e;
            let geometry = match geopackage::geometry_of(entity) {
                Ok(Some((g, chord))) => {
                    if chord && input.vertices == XyzVertices::All {
                        chords += 1;
                    }
                    g
                }
                Ok(None) => {
                    *left.entry("kayıt".to_owned()).or_default() += 1;
                    continue;
                }
                Err(word) => {
                    *left.entry(word.to_owned()).or_default() += 1;
                    continue;
                }
            };
            let pts = match positions(&geometry, input.vertices, &mut middles) {
                Ok(p) => p,
                Err(word) => {
                    *left.entry(word.to_owned()).or_default() += 1;
                    continue;
                }
            };
            let mut wrote = false;
            for c in pts {
                let z = match (c.z, input.without_z) {
                    (Some(z), _) => {
                        with_z += 1;
                        z
                    }
                    (None, XyzWithoutZ::Zero) => {
                        no_z += 1;
                        0.0
                    }
                    (None, XyzWithoutZ::Skip) => {
                        skipped_z += 1;
                        continue;
                    }
                };
                out.push_str(&number(c.x, input.decimals));
                out.push(sep);
                out.push_str(&number(c.y, input.decimals));
                out.push(sep);
                out.push_str(&number(z, input.decimals));
                out.push('\n');
                points += 1;
                wrote = true;
            }
            if wrote {
                objects += 1;
            }
        }
    }
    if points == 0 {
        return Err((
            kentos_commands::codes::NOTHING_TO_EXPORT.to_owned(),
            match input.vertices {
                XyzVertices::Points => {
                    "Katmanlarda XYZ'ye yazılacak nokta yok; çizgi ve alanların köşeleri için vertices: all verin."
                        .into()
                }
                XyzVertices::All => "Katmanlarda XYZ'ye yazılacak konum yok.".into(),
            },
            "layers".into(),
        ));
    }
    if !left.is_empty() {
        lost.insert(format!(
            "XYZ'de satırı olmayan nesneler yazılmadı: {}",
            left.iter()
                .map(|(k, n)| format!("{n} {k}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if no_z > 0 {
        lost.insert(format!(
            "{no_z} konumun kotu yok: üçüncü sütuna 0 yazıldı (çizimde konum kotsuz kalır)"
        ));
    }
    if skipped_z > 0 {
        lost.insert(format!("{skipped_z} kotsuz konum yazılmadı"));
    }
    if middles > 0 {
        lost.insert(format!("{middles} yayın orta noktası köşe olmadığından yazılmadı"));
    }
    if chords > 0 {
        lost.insert(format!(
            "{chords} eğri (elips, eğri, NURBS) kirişlerinin köşeleriyle yazıldı"
        ));
    }
    let srid = doc.settings().srid;
    let bytes = out.into_bytes();
    let output = XyzExported {
        path: input.path.clone(),
        points,
        objects,
        with_z,
        bytes: bytes.len() as u64,
        sha256: sha256_hex(&bytes),
        srid: (srid != 0).then_some(srid),
        left,
        lost: lost.into_iter().collect(),
    };
    Ok(Made { bytes, output })
}
