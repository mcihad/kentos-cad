//! The coordinate system a cloud names (docs/adr/0207 §2): LAS's WKT VLR
//! (`LASF_Projection` 2112, in a VLR or an EVLR) or its GeoTIFF keys (34735,
//! the key directory). The horizontal system's EPSG code is what the rule of
//! systems compares (a compound system's horizontal part); a system the file
//! names without a code KentOS can tell is shown by its name and counts as
//! not known.

use kentos_geometry_core::crs::text::read_text;
use kentos_geometry_core::crs::wkt::{self, Item, Node};
use serde::Serialize;

use crate::las::Vlr;

/// GeoKey ids read (GeoTIFF 1.1).
const GEOGRAPHIC_TYPE: u16 = 2048;
const PROJECTED_CS_TYPE: u16 = 3072;
/// A user-defined system: KentOS does not read its parameters.
const USER_DEFINED: u16 = 32767;

/// What a cloud says of its system.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudCrs {
    /// The horizontal system's EPSG code, when the file names one KentOS can tell.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epsg: Option<u32>,
    /// The system's name as the file writes it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub geographic: bool,
    /// The WKT the file holds (written back by the index and the writers).
    #[serde(skip)]
    pub wkt: Option<String>,
    /// The GeoTIFF key VLRs the file holds (34735, 34736, 34737), written back as they are.
    #[serde(skip)]
    pub geokeys: Vec<Vlr>,
}

impl CloudCrs {
    /// Whether the file names any system at all.
    pub fn named(&self) -> bool {
        self.epsg.is_some() || self.name.is_some()
    }
}

/// The system the VLRs and EVLRs name; WKT before GeoTIFF keys.
pub fn of(vlrs: &[Vlr]) -> CloudCrs {
    let geokeys: Vec<Vlr> = vlrs
        .iter()
        .filter(|v| v.user == "LASF_Projection" && matches!(v.record, 34735..=34737))
        .cloned()
        .collect();
    let wkt_text = vlrs
        .iter()
        .find(|v| v.is("LASF_Projection", 2112))
        .map(|v| text_of(&v.data))
        .filter(|t| !t.trim().is_empty());
    if let Some(text) = wkt_text {
        let mut c = from_wkt(&text);
        c.wkt = Some(text);
        c.geokeys = geokeys;
        return c;
    }
    let mut c = geokeys
        .iter()
        .find(|v| v.record == 34735)
        .map(|v| from_geokeys(&v.data))
        .unwrap_or_default();
    c.geokeys = geokeys;
    c
}

/// A VLR's text up to its first zero.
fn text_of(data: &[u8]) -> String {
    let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    String::from_utf8_lossy(&data[..end]).into_owned()
}

/// A horizontal system's node: the compound's projected or geographic part, else the root.
fn horizontal(root: &Node) -> &Node {
    const COMPOUND: [&str; 2] = ["COMPD_CS", "COMPOUNDCRS"];
    const PLANE: [&str; 6] = [
        "PROJCS",
        "PROJCRS",
        "GEOGCS",
        "GEOGCRS",
        "GEODCRS",
        "BASEGEOGCRS",
    ];
    if COMPOUND.iter().any(|c| root.name.eq_ignore_ascii_case(c)) {
        root.child(&PLANE).unwrap_or(root)
    } else {
        root
    }
}

/// The EPSG code a node's own AUTHORITY (WKT 1) or ID (WKT 2) gives.
fn authority(node: &Node) -> Option<u32> {
    node.items.iter().find_map(|i| match i {
        Item::Node(n)
            if n.name.eq_ignore_ascii_case("AUTHORITY") || n.name.eq_ignore_ascii_case("ID") =>
        {
            let is_epsg = n
                .items
                .first()
                .is_some_and(|f| matches!(f, Item::Str(s) if s.eq_ignore_ascii_case("EPSG")));
            if !is_epsg {
                return None;
            }
            n.items.get(1).and_then(|v| match v {
                Item::Num(x) if *x >= 1.0 && *x < 1.0e9 && x.fract() == 0.0 => Some(*x as u32),
                Item::Str(s) => s.trim().parse().ok(),
                _ => None,
            })
        }
        _ => None,
    })
}

/// A system from a WKT text: the horizontal part's code, else the registry's system it is.
pub fn from_wkt(text: &str) -> CloudCrs {
    let Some(root) = wkt::parse(text) else {
        return CloudCrs::default();
    };
    let h = horizontal(&root);
    let geographic = ["GEOGCS", "GEOGCRS", "GEODCRS"]
        .iter()
        .any(|n| h.name.eq_ignore_ascii_case(n));
    let name = h.text().map(str::to_owned);
    let epsg = authority(h).or_else(|| {
        read_text(text)
            .ok()
            .and_then(|r| r.registry.filter(|(_, exact)| *exact).map(|(s, _)| s))
    });
    CloudCrs {
        epsg,
        name,
        geographic,
        wkt: None,
        geokeys: Vec::new(),
    }
}

/// A system from GeoTIFF's key directory (u16s: version, revision, minor, count, then
/// id, location, count, value per key).
pub fn from_geokeys(data: &[u8]) -> CloudCrs {
    let shorts: Vec<u16> = data
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let key = |id: u16| -> Option<u16> {
        let n = usize::from(*shorts.get(3)?);
        (0..n).find_map(|i| {
            let e = shorts.get(4 + 4 * i..8 + 4 * i)?;
            (e[0] == id && e[1] == 0 && e[2] == 1).then_some(e[3])
        })
    };
    let usable = |v: u16| v != 0 && v != USER_DEFINED;
    if let Some(p) = key(PROJECTED_CS_TYPE).filter(|&v| usable(v)) {
        return CloudCrs {
            epsg: Some(u32::from(p)),
            ..CloudCrs::default()
        };
    }
    if let Some(g) = key(GEOGRAPHIC_TYPE).filter(|&v| usable(v)) {
        return CloudCrs {
            epsg: Some(u32::from(g)),
            geographic: true,
            ..CloudCrs::default()
        };
    }
    let user_defined =
        key(PROJECTED_CS_TYPE) == Some(USER_DEFINED) || key(GEOGRAPHIC_TYPE) == Some(USER_DEFINED);
    CloudCrs {
        name: user_defined.then(|| "Kullanıcı tanımlı sistem (GeoTIFF)".to_owned()),
        ..CloudCrs::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wkt_codes_and_compound_systems() {
        let wkt1 = r#"PROJCS["TUREF / TM30",GEOGCS["TUREF",DATUM["Turkish_National_Reference_Frame",SPHEROID["GRS 1980",6378137,298.257222101]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",30],PARAMETER["scale_factor",1],PARAMETER["false_easting",500000],PARAMETER["false_northing",0],UNIT["metre",1],AUTHORITY["EPSG","5254"]]"#;
        let c = from_wkt(wkt1);
        assert_eq!(c.epsg, Some(5254));
        assert_eq!(c.name.as_deref(), Some("TUREF / TM30"));
        let compound = format!(
            r#"COMPD_CS["TUREF / TM30 + height",{wkt1},VERT_CS["height",VERT_DATUM["x",2005],UNIT["metre",1]]]"#
        );
        assert_eq!(from_wkt(&compound).epsg, Some(5254));
        let wkt2 = r#"PROJCRS["WGS 84 / UTM zone 35N",BASEGEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563]]],CONVERSION["UTM zone 35N",METHOD["Transverse Mercator"]],ID["EPSG",32635]]"#;
        assert_eq!(from_wkt(wkt2).epsg, Some(32635));
        assert_eq!(from_wkt("not a wkt"), CloudCrs::default());
    }

    #[test]
    fn geokeys() {
        let keys = |entries: &[[u16; 4]]| -> Vec<u8> {
            let mut s = vec![1u16, 1, 0, entries.len() as u16];
            for e in entries {
                s.extend_from_slice(e);
            }
            s.iter().flat_map(|v| v.to_le_bytes()).collect()
        };
        let c = from_geokeys(&keys(&[[1024, 0, 1, 1], [3072, 0, 1, 5254]]));
        assert_eq!(c.epsg, Some(5254));
        let g = from_geokeys(&keys(&[[1024, 0, 1, 2], [2048, 0, 1, 4326]]));
        assert_eq!((g.epsg, g.geographic), (Some(4326), true));
        let u = from_geokeys(&keys(&[[3072, 0, 1, 32767]]));
        assert_eq!(u.epsg, None);
        assert!(u.named());
        assert!(!from_geokeys(&[]).named());
    }
}
