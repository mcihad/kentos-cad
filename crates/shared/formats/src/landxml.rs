//! LandXML surfaces (plan/road.md Y2.5, docs/adr/0900 §14): what a LandXML 1.x file says of its TIN surfaces, the
//! exchange format of Civil 3D, OpenRoads and Netcad. A surface's `Definition` holds its points (`Pnts/P`, each
//! “northing easting elevation”) and faces (`Faces/F`, three point ids, `i="1"` an ignored face); its `SourceData` the
//! breaklines (`Breaklines/Breakline/PntList3D`) and boundaries (`Boundaries/Boundary`, `bndType` outer or void, its
//! `PntList2D` or `PntList3D`). Numbers are in the file's linear unit (`Units/Metric|Imperial@linearUnit`), turned into
//! metres here; a place is (east, north) as KentOS writes it. Names are matched without their namespace.

use roxmltree::Node;

use crate::xml::{self, Unparsed};

/// A point of a surface: its id in the file and its place (metres).
#[derive(Clone, Debug, PartialEq)]
pub struct LxPoint {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// A face: its three points (indices into [`LxSurface::points`]), and whether the file ignores it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LxFace {
    pub points: [usize; 3],
    pub ignored: bool,
}

/// A boundary: a void, or the outer one, its places (metres, without heights).
#[derive(Clone, Debug, PartialEq)]
pub struct LxBoundary {
    pub void: bool,
    pub points: Vec<[f64; 2]>,
}

/// A surface of the file.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LxSurface {
    pub name: String,
    pub points: Vec<LxPoint>,
    pub faces: Vec<LxFace>,
    pub breaklines: Vec<Vec<[f64; 3]>>,
    pub boundaries: Vec<LxBoundary>,
    /// Faces naming a point the file does not have, left out.
    pub broken_faces: usize,
    /// Points whose id the file gives twice, the later left out.
    pub twice: usize,
}

/// What a LandXML file says of its surfaces.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LandXml {
    pub surfaces: Vec<LxSurface>,
    /// The metres of the file's linear unit.
    pub metres: f64,
    /// The unit's name as the file writes it (`meter`, `USSurveyFoot` …); empty when it says none (metres taken).
    pub unit: String,
}

/// Why a file is no LandXML.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LxError {
    NotXml,
    TooDeep,
    /// Its root is no `LandXML`.
    NotLandXml,
    /// A number or a list of numbers that does not read.
    BadNumber,
}

impl LxError {
    /// In the user's words.
    pub fn words(self) -> &'static str {
        match self {
            LxError::NotXml => "XML olarak okunamadı",
            LxError::TooDeep => "XML çok derin iç içe",
            LxError::NotLandXml => "kökü LandXML değil",
            LxError::BadNumber => "bir koordinat sayı olarak okunamadı",
        }
    }
}

fn named<'a, 'b>(n: Node<'a, 'b>, name: &str) -> impl Iterator<Item = Node<'a, 'b>> {
    n.children().filter(move |c| c.is_element() && c.tag_name().name() == name)
}

fn first<'a, 'b>(n: Node<'a, 'b>, name: &str) -> Option<Node<'a, 'b>> {
    named(n, name).next()
}

fn numbers(text: &str) -> Result<Vec<f64>, LxError> {
    text.split_whitespace()
        .map(|w| w.replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite()).ok_or(LxError::BadNumber))
        .collect()
}

/// The metres of a linear unit's name.
fn metres_of(unit: &str) -> f64 {
    match unit {
        "foot" | "Foot" | "feet" => 0.3048,
        "USSurveyFoot" | "USFoot" => 1200.0 / 3937.0,
        "kilometer" => 1000.0,
        "centimeter" => 0.01,
        "millimeter" => 0.001,
        _ => 1.0,
    }
}

/// Reads a file's surfaces.
pub fn read(bytes: &[u8]) -> Result<LandXml, LxError> {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim_start_matches('\u{feff}');
    let doc = xml::document(text).map_err(|e| match e {
        Unparsed::TooDeep => LxError::TooDeep,
        _ => LxError::NotXml,
    })?;
    let root = doc.root_element();
    if root.tag_name().name() != "LandXML" {
        return Err(LxError::NotLandXml);
    }
    let unit = first(root, "Units")
        .and_then(|u| u.children().find(|c| c.is_element()))
        .and_then(|m| m.attribute("linearUnit"))
        .unwrap_or("")
        .to_owned();
    let k = metres_of(&unit);
    let mut out = LandXml {
        surfaces: Vec::new(),
        metres: k,
        unit,
    };
    for surfaces in named(root, "Surfaces") {
        for s in named(surfaces, "Surface") {
            out.surfaces.push(surface(s, k)?);
        }
    }
    Ok(out)
}

fn surface(s: Node<'_, '_>, k: f64) -> Result<LxSurface, LxError> {
    let mut out = LxSurface {
        name: s.attribute("name").unwrap_or("").to_owned(),
        ..LxSurface::default()
    };
    let mut at = std::collections::HashMap::new();
    if let Some(def) = first(s, "Definition") {
        if let Some(pnts) = first(def, "Pnts") {
            for p in named(pnts, "P") {
                let v = numbers(p.text().unwrap_or(""))?;
                let [n, e, z] = v[..] else {
                    return Err(LxError::BadNumber);
                };
                let id = p.attribute("id").unwrap_or("").to_owned();
                if at.contains_key(&id) {
                    out.twice += 1;
                    continue;
                }
                at.insert(id.clone(), out.points.len());
                out.points.push(LxPoint { id, x: e * k, y: n * k, z: z * k });
            }
        }
        if let Some(faces) = first(def, "Faces") {
            for f in named(faces, "F") {
                let ids: Vec<&str> = f.text().unwrap_or("").split_whitespace().collect();
                let found: Option<Vec<usize>> = ids.iter().map(|i| at.get(*i).copied()).collect();
                match found.as_deref() {
                    Some(&[a, b, c]) => out.faces.push(LxFace {
                        points: [a, b, c],
                        ignored: f.attribute("i") == Some("1"),
                    }),
                    _ => out.broken_faces += 1,
                }
            }
        }
    }
    if let Some(source) = first(s, "SourceData") {
        if let Some(lines) = first(source, "Breaklines") {
            for b in named(lines, "Breakline") {
                let Some(list) = first(b, "PntList3D") else { continue };
                let v = numbers(list.text().unwrap_or(""))?;
                if v.len() % 3 != 0 {
                    return Err(LxError::BadNumber);
                }
                let pts: Vec<[f64; 3]> = v.chunks(3).map(|c| [c[1] * k, c[0] * k, c[2] * k]).collect();
                if pts.len() >= 2 {
                    out.breaklines.push(pts);
                }
            }
        }
        if let Some(bounds) = first(source, "Boundaries") {
            for b in named(bounds, "Boundary") {
                let void = b.attribute("bndType").is_some_and(|t| t.eq_ignore_ascii_case("void"));
                let (list, step) = match (first(b, "PntList2D"), first(b, "PntList3D")) {
                    (Some(l), _) => (l, 2),
                    (None, Some(l)) => (l, 3),
                    _ => continue,
                };
                let v = numbers(list.text().unwrap_or(""))?;
                if v.len() % step != 0 {
                    return Err(LxError::BadNumber);
                }
                let mut pts: Vec<[f64; 2]> = v.chunks(step).map(|c| [c[1] * k, c[0] * k]).collect();
                // A ring written closed ends where it starts.
                if pts.len() > 1 && pts.first() == pts.last() {
                    pts.pop();
                }
                if pts.len() >= 3 {
                    out.boundaries.push(LxBoundary { void, points: pts });
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_surface_reads_northing_first_in_its_unit() {
        let text = r#"<?xml version="1.0"?>
<LandXML xmlns="http://www.landxml.org/schema/LandXML-1.2" version="1.2">
  <Units><Imperial linearUnit="foot"/></Units>
  <Surfaces><Surface name="EG"><Definition surfType="TIN">
    <Pnts><P id="1">10 20 3</P><P id="2">10 30 4</P><P id="3">20 20 5</P><P id="2">0 0 0</P></Pnts>
    <Faces><F>1 2 3</F><F i="1">1 3 2</F><F>1 2 9</F></Faces>
  </Definition>
  <SourceData><Breaklines><Breakline><PntList3D>10 20 3 20 20 5</PntList3D></Breakline></Breaklines>
  <Boundaries><Boundary bndType="outer"><PntList2D>10 20 10 30 20 20 10 20</PntList2D></Boundary></Boundaries></SourceData>
  </Surface></Surfaces></LandXML>"#;
        let x = read(text.as_bytes()).expect("reads");
        assert_eq!((x.unit.as_str(), x.metres), ("foot", 0.3048));
        let s = &x.surfaces[0];
        assert_eq!(s.name, "EG");
        assert_eq!(s.points.len(), 3);
        assert_eq!((s.points[0].x, s.points[0].y), (20.0 * 0.3048, 10.0 * 0.3048));
        assert_eq!((s.twice, s.broken_faces), (1, 1));
        assert_eq!(s.faces.iter().filter(|f| f.ignored).count(), 1);
        assert_eq!(s.breaklines[0][1], [20.0 * 0.3048, 20.0 * 0.3048, 5.0 * 0.3048]);
        assert_eq!(s.boundaries[0].points.len(), 3, "a closed ring's last place goes");
        assert_eq!(read(b"<a/>"), Err(LxError::NotLandXml));
        assert_eq!(read(b"<LandXML><Surfaces><Surface><Definition><Pnts><P id=\"1\">x 1 2</P></Pnts></Definition></Surface></Surfaces></LandXML>"), Err(LxError::BadNumber));
    }
}
