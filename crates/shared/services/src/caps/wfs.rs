//! WFS capabilities, 1.0, 1.1 and 2.0 (docs/adr/0208 §10): the feature
//! types (name, title, systems, longitude-latitude box, output formats),
//! GetFeature's address and formats, and whether the service pages
//! (`ImplementsResultPaging` in 2.0).

use serde::Serialize;

use super::{attr, child, child_text, children, num_attr, ordered, text_of, wgs84_box, xml};
use crate::crs::crs_name;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WfsCapabilities {
    pub version: String,
    pub title: String,
    pub types: Vec<WfsType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get_feature: Option<String>,
    /// GetFeature's output formats.
    pub formats: Vec<String>,
    /// Pages with `COUNT` and `STARTINDEX`.
    pub paging: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WfsType {
    pub name: String,
    pub title: String,
    #[serde(rename = "abstract", skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// The default system first, then the others KentOS reads.
    pub srids: Vec<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wgs84: Option<[f64; 4]>,
    pub formats: Vec<String>,
}

/// Reads a WFS capabilities document.
pub fn read(text: &str) -> Result<WfsCapabilities, String> {
    let doc = xml(text)?;
    let root = doc.root_element();
    if root.tag_name().name() != "WFS_Capabilities" {
        return Err(format!(
            "Belge bir WFS yetenek belgesi değil (kökü {}).",
            root.tag_name().name()
        ));
    }
    let version = attr(root, "version").unwrap_or("2.0.0").to_owned();
    let title = child(root, "ServiceIdentification")
        .and_then(|s| child_text(s, "Title"))
        .or_else(|| child(root, "Service").and_then(|s| child_text(s, "Title")))
        .unwrap_or_default();
    // 1.1 and 2.0: ows:OperationsMetadata; 1.0: Capability/Request/GetFeature.
    let mut get_feature = None;
    let mut formats = Vec::new();
    let mut paging = false;
    if let Some(m) = child(root, "OperationsMetadata") {
        if let Some(op) = children(m, "Operation").find(|o| attr(*o, "name") == Some("GetFeature"))
        {
            get_feature = op
                .descendants()
                .find(|d| d.is_element() && d.tag_name().name() == "Get")
                .and_then(|g| attr(g, "href").map(|h| h.trim().to_owned()));
            formats = op
                .descendants()
                .filter(|d| d.is_element() && d.tag_name().name() == "Parameter")
                .filter(|p| {
                    attr(*p, "name").is_some_and(|n| n.eq_ignore_ascii_case("outputFormat"))
                })
                .flat_map(|p| {
                    p.descendants()
                        .filter(|v| v.is_element() && v.tag_name().name() == "Value")
                        .filter_map(text_of)
                        .collect::<Vec<_>>()
                })
                .collect();
        }
        paging = m.descendants().any(|c| {
            c.is_element()
                && c.tag_name().name() == "Constraint"
                && attr(c, "name") == Some("ImplementsResultPaging")
                && c.descendants().any(|v| {
                    v.is_element() && text_of(v).is_some_and(|t| t.eq_ignore_ascii_case("true"))
                })
        });
    } else if let Some(g) = child(root, "Capability")
        .and_then(|c| child(c, "Request"))
        .and_then(|r| child(r, "GetFeature"))
    {
        get_feature = g
            .descendants()
            .find(|d| d.is_element() && d.tag_name().name() == "Get")
            .and_then(|d| attr(d, "onlineResource").map(|h| h.trim().to_owned()));
        formats = child(g, "ResultFormat")
            .map(|f| {
                f.children()
                    .filter(|c| c.is_element())
                    .map(|c| c.tag_name().name().to_owned())
                    .collect()
            })
            .unwrap_or_default();
    }
    let list = child(root, "FeatureTypeList");
    let types = list
        .map(|l| {
            children(l, "FeatureType")
                .filter_map(|t| {
                    let name = child_text(t, "Name")?;
                    let mut srids = Vec::new();
                    for key in ["DefaultCRS", "DefaultSRS", "SRS", "OtherCRS", "OtherSRS"] {
                        for c in children(t, key) {
                            if let Some(n) = text_of(c).and_then(|s| crs_name(&s))
                                && !srids.contains(&n.srid)
                            {
                                srids.push(n.srid);
                            }
                        }
                    }
                    let wgs84 = wgs84_box(t).or_else(|| {
                        child(t, "LatLongBoundingBox").and_then(|b| {
                            Some(ordered([
                                num_attr(b, "minx")?,
                                num_attr(b, "miny")?,
                                num_attr(b, "maxx")?,
                                num_attr(b, "maxy")?,
                            ]))
                        })
                    });
                    Some(WfsType {
                        title: child_text(t, "Title").unwrap_or_else(|| name.clone()),
                        summary: child_text(t, "Abstract"),
                        srids,
                        wgs84,
                        formats: child(t, "OutputFormats")
                            .map(|f| children(f, "Format").filter_map(text_of).collect())
                            .unwrap_or_default(),
                        name,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(WfsCapabilities {
        version,
        title,
        types,
        get_feature,
        formats,
        paging,
    })
}

/// The best output format of those offered: GeoJSON when there is one,
/// else GML 3.2, 3.1, 2 (none: the server's default, GML).
pub fn output_format(offered: &[String]) -> Option<String> {
    let lower: Vec<String> = offered.iter().map(|f| f.to_ascii_lowercase()).collect();
    let find = |want: &dyn Fn(&str) -> bool| {
        lower
            .iter()
            .position(|f| want(f))
            .map(|i| offered[i].clone())
    };
    find(&|f| {
        f == "application/json" || f == "application/geo+json" || f == "json" || f == "geojson"
    })
    .or_else(|| find(&|f| f.contains("json")))
    .or_else(|| find(&|f| f.contains("gml/3.2") || f.contains("gml32")))
    .or_else(|| find(&|f| f.contains("gml/3.1") || f.contains("gml3")))
    .or_else(|| find(&|f| f.contains("gml2") || f.contains("gml/2")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_20_document() {
        let doc = r#"<?xml version="1.0"?>
<wfs:WFS_Capabilities version="2.0.0" xmlns:wfs="http://www.opengis.net/wfs/2.0" xmlns:ows="http://www.opengis.net/ows/1.1" xmlns:xlink="http://www.w3.org/1999/xlink">
<ows:ServiceIdentification><ows:Title>Kadastro</ows:Title></ows:ServiceIdentification>
<ows:OperationsMetadata>
  <ows:Operation name="GetFeature"><ows:DCP><ows:HTTP><ows:Get xlink:href="https://x/wfs?"/></ows:HTTP></ows:DCP>
    <ows:Parameter name="outputFormat"><ows:AllowedValues><ows:Value>application/gml+xml; version=3.2</ows:Value><ows:Value>application/json</ows:Value></ows:AllowedValues></ows:Parameter>
  </ows:Operation>
  <ows:Constraint name="ImplementsResultPaging"><ows:NoValues/><ows:DefaultValue>TRUE</ows:DefaultValue></ows:Constraint>
</ows:OperationsMetadata>
<wfs:FeatureTypeList><wfs:FeatureType>
  <wfs:Name>kadastro:parsel</wfs:Name><wfs:Title>Parseller</wfs:Title>
  <wfs:DefaultCRS>urn:ogc:def:crs:EPSG::5254</wfs:DefaultCRS><wfs:OtherCRS>urn:ogc:def:crs:EPSG::4326</wfs:OtherCRS>
  <ows:WGS84BoundingBox><ows:LowerCorner>29 40</ows:LowerCorner><ows:UpperCorner>31 41</ows:UpperCorner></ows:WGS84BoundingBox>
</wfs:FeatureType></wfs:FeatureTypeList></wfs:WFS_Capabilities>"#;
        let c = read(doc).expect("reads");
        assert_eq!(
            (c.version.as_str(), c.title.as_str()),
            ("2.0.0", "Kadastro")
        );
        assert!(c.paging);
        assert_eq!(c.get_feature.as_deref(), Some("https://x/wfs?"));
        assert_eq!(
            output_format(&c.formats).as_deref(),
            Some("application/json")
        );
        let t = &c.types[0];
        assert_eq!(t.name, "kadastro:parsel");
        assert_eq!(t.srids, vec![5254, 4326]);
        assert_eq!(t.wgs84, Some([29.0, 40.0, 31.0, 41.0]));
    }

    #[test]
    fn a_10_document() {
        let doc = r#"<WFS_Capabilities version="1.0.0"><Service><Title>Eski</Title></Service>
<Capability><Request><GetFeature><ResultFormat><GML2/></ResultFormat><DCPType><HTTP><Get onlineResource="https://x/wfs?"/></HTTP></DCPType></GetFeature></Request></Capability>
<FeatureTypeList><FeatureType><Name>yol</Name><SRS>EPSG:32636</SRS><LatLongBoundingBox minx="30" miny="39" maxx="31" maxy="40"/></FeatureType></FeatureTypeList></WFS_Capabilities>"#;
        let c = read(doc).expect("reads");
        assert_eq!(c.formats, vec!["GML2"]);
        assert_eq!(c.get_feature.as_deref(), Some("https://x/wfs?"));
        assert_eq!(c.types[0].srids, vec![32636]);
        assert_eq!(c.types[0].wgs84, Some([30.0, 39.0, 31.0, 40.0]));
        assert!(!c.paging);
        assert_eq!(output_format(&c.formats).as_deref(), Some("GML2"));
    }
}
