//! WMS capabilities, 1.1.1 and 1.3.0 (docs/adr/0208 §5): the service's
//! title, its GetMap and GetFeatureInfo addresses and formats, and its
//! layers depth first, each with what it inherits (WMS 1.3.0 §7.2.4.8:
//! styles and systems added to the parent's, the boxes, the scale range and
//! the attribution replacing them).

use serde::Serialize;

use super::{attr, child, child_text, children, href, num_attr, ordered, text_of, xml};
use crate::crs::{crs_name, to_east_north};

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WmsCapabilities {
    pub version: String,
    pub title: String,
    #[serde(rename = "abstract", skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// GetMap's image formats.
    pub formats: Vec<String>,
    /// GetFeatureInfo's formats; empty when the service has none.
    pub info_formats: Vec<String>,
    /// GetMap's address (absent: the one the capabilities were asked at).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get_map: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get_feature_info: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_height: Option<u32>,
    pub layers: Vec<WmsLayer>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WmsStyle {
    pub name: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legend: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WmsLayer {
    /// None for a layer that only groups others (it cannot be asked for).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub title: String,
    #[serde(rename = "abstract", skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// How deep in the tree, the root 0.
    pub depth: u32,
    pub queryable: bool,
    pub opaque: bool,
    /// The systems it may be asked in (EPSG codes; CRS84 as 4326).
    pub srids: Vec<u32>,
    /// `[west, south, east, north]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wgs84: Option<[f64; 4]>,
    /// Its boxes in its systems, east and north.
    pub boxes: Vec<(u32, [f64; 4])>,
    pub styles: Vec<WmsStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_scale: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_scale: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribution: Option<String>,
}

/// What a layer passes to the layers under it.
#[derive(Clone, Default)]
struct Inherited {
    srids: Vec<u32>,
    styles: Vec<WmsStyle>,
    wgs84: Option<[f64; 4]>,
    boxes: Vec<(u32, [f64; 4])>,
    min_scale: Option<f64>,
    max_scale: Option<f64>,
    attribution: Option<String>,
    queryable: bool,
    opaque: bool,
}

/// Reads a WMS capabilities document.
pub fn read(text: &str) -> Result<WmsCapabilities, String> {
    let doc = xml(text)?;
    let root = doc.root_element();
    let name = root.tag_name().name();
    if name != "WMS_Capabilities" && name != "WMT_MS_Capabilities" {
        return Err(format!(
            "Belge bir WMS yetenek belgesi değil (kökü {name})."
        ));
    }
    let version = attr(root, "version").unwrap_or("1.3.0").to_owned();
    let modern = !version.starts_with("1.1");
    let service = child(root, "Service");
    let title = service
        .and_then(|s| child_text(s, "Title"))
        .unwrap_or_default();
    let summary = service.and_then(|s| child_text(s, "Abstract"));
    let num = |n: Option<roxmltree::Node<'_, '_>>, k: &str| {
        n.and_then(|s| child_text(s, k))
            .and_then(|t| t.parse::<u32>().ok())
    };
    let capability =
        child(root, "Capability").ok_or_else(|| "WMS belgesinde Capability yok.".to_owned())?;
    let request = child(capability, "Request");
    let op = |name: &str| request.and_then(|r| child(r, name));
    let formats = |o: Option<roxmltree::Node<'_, '_>>| {
        o.map(|o| {
            children(o, "Format")
                .filter_map(text_of)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
    };
    let get_url = |o: Option<roxmltree::Node<'_, '_>>| {
        o.and_then(|o| {
            o.descendants()
                .find(|d| d.is_element() && d.tag_name().name() == "Get")
                .and_then(href)
        })
    };
    let mut layers = Vec::new();
    if let Some(top) = child(capability, "Layer") {
        walk(top, 0, &Inherited::default(), modern, &mut layers);
    }
    Ok(WmsCapabilities {
        version,
        title,
        summary,
        formats: formats(op("GetMap")),
        info_formats: formats(op("GetFeatureInfo")),
        get_map: get_url(op("GetMap")),
        get_feature_info: get_url(op("GetFeatureInfo")),
        max_width: num(service, "MaxWidth"),
        max_height: num(service, "MaxHeight"),
        layers,
    })
}

fn walk(
    n: roxmltree::Node<'_, '_>,
    depth: u32,
    parent: &Inherited,
    modern: bool,
    out: &mut Vec<WmsLayer>,
) {
    let mut own = parent.clone();
    for key in ["CRS", "SRS"] {
        for c in children(n, key) {
            // 1.1.1 may list several in one element, space separated.
            for word in text_of(c).unwrap_or_default().split_whitespace() {
                if let Some(name) = crs_name(word)
                    && !own.srids.contains(&name.srid)
                {
                    own.srids.push(name.srid);
                }
            }
        }
    }
    if let Some(b) = child(n, "EX_GeographicBoundingBox") {
        let v = |k: &str| child_text(b, k).and_then(|t| t.parse::<f64>().ok());
        if let (Some(w), Some(e), Some(s), Some(no)) = (
            v("westBoundLongitude"),
            v("eastBoundLongitude"),
            v("southBoundLatitude"),
            v("northBoundLatitude"),
        ) {
            own.wgs84 = Some(ordered([w, s, e, no]));
        }
    } else if let Some(b) = child(n, "LatLonBoundingBox")
        && let (Some(w), Some(s), Some(e), Some(no)) = (
            num_attr(b, "minx"),
            num_attr(b, "miny"),
            num_attr(b, "maxx"),
            num_attr(b, "maxy"),
        )
    {
        own.wgs84 = Some(ordered([w, s, e, no]));
    }
    let boxes: Vec<(u32, [f64; 4])> = children(n, "BoundingBox")
        .filter_map(|b| {
            let name = crs_name(attr(b, "CRS").or_else(|| attr(b, "SRS"))?)?;
            let (a1, b1, a2, b2) = (
                num_attr(b, "minx")?,
                num_attr(b, "miny")?,
                num_attr(b, "maxx")?,
                num_attr(b, "maxy")?,
            );
            // 1.3.0 writes the box in the system's axis order; 1.1.1 east first.
            let (x1, y1, x2, y2) = if modern {
                let (x1, y1) = to_east_north(name, a1, b1);
                let (x2, y2) = to_east_north(name, a2, b2);
                (x1, y1, x2, y2)
            } else {
                (a1, b1, a2, b2)
            };
            Some((name.srid, ordered([x1, y1, x2, y2])))
        })
        .collect();
    if !boxes.is_empty() {
        own.boxes = boxes;
    }
    for s in children(n, "Style") {
        let Some(name) = child_text(s, "Name") else {
            continue;
        };
        if own.styles.iter().any(|o| o.name == name) {
            continue;
        }
        own.styles.push(WmsStyle {
            title: child_text(s, "Title").unwrap_or_else(|| name.clone()),
            legend: child(s, "LegendURL").and_then(href),
            name,
        });
    }
    let scale = |k: &str| child_text(n, k).and_then(|t| t.parse::<f64>().ok());
    if let Some(v) = scale("MinScaleDenominator") {
        own.min_scale = Some(v);
    }
    if let Some(v) = scale("MaxScaleDenominator") {
        own.max_scale = Some(v);
    }
    if let Some(a) = child(n, "Attribution").and_then(|a| child_text(a, "Title")) {
        own.attribution = Some(a);
    }
    let flag =
        |k: &str, was: bool| attr(n, k).map_or(was, |v| v == "1" || v.eq_ignore_ascii_case("true"));
    own.queryable = flag("queryable", parent.queryable);
    own.opaque = flag("opaque", parent.opaque);
    out.push(WmsLayer {
        name: child_text(n, "Name"),
        title: child_text(n, "Title").unwrap_or_default(),
        summary: child_text(n, "Abstract"),
        depth,
        queryable: own.queryable,
        opaque: own.opaque,
        srids: own.srids.clone(),
        wgs84: own.wgs84,
        boxes: own.boxes.clone(),
        styles: own.styles.clone(),
        min_scale: own.min_scale,
        max_scale: own.max_scale,
        attribution: own.attribution.clone(),
    });
    for c in children(n, "Layer") {
        walk(c, depth + 1, &own, modern, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const V13: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<WMS_Capabilities version="1.3.0" xmlns="http://www.opengis.net/wms" xmlns:xlink="http://www.w3.org/1999/xlink">
  <Service><Name>WMS</Name><Title>Belediye KBS</Title><Abstract>İmar ve kadastro</Abstract><MaxWidth>4096</MaxWidth></Service>
  <Capability>
    <Request>
      <GetMap><Format>image/png</Format><Format>image/jpeg</Format>
        <DCPType><HTTP><Get><OnlineResource xlink:href="https://kbs.ornek.bel.tr/geoserver/wms?"/></Get></HTTP></DCPType></GetMap>
      <GetFeatureInfo><Format>text/plain</Format><Format>application/json</Format>
        <DCPType><HTTP><Get><OnlineResource xlink:href="https://kbs.ornek.bel.tr/geoserver/wms?"/></Get></HTTP></DCPType></GetFeatureInfo>
    </Request>
    <Layer>
      <Title>Katmanlar</Title>
      <CRS>EPSG:4326</CRS><CRS>EPSG:5254</CRS>
      <EX_GeographicBoundingBox><westBoundLongitude>29</westBoundLongitude><eastBoundLongitude>31</eastBoundLongitude><southBoundLatitude>40</southBoundLatitude><northBoundLatitude>41</northBoundLatitude></EX_GeographicBoundingBox>
      <BoundingBox CRS="EPSG:5254" minx="4400000" miny="400000" maxx="4500000" maxy="600000"/>
      <Style><Name>default</Name><Title>Varsayılan</Title></Style>
      <Layer queryable="1">
        <Name>imar:plan</Name><Title>İmar planı</Title>
        <CRS>EPSG:3857</CRS>
        <Style><Name>renkli</Name><Title>Renkli</Title><LegendURL><OnlineResource xlink:href="https://x/legend.png"/></LegendURL></Style>
        <MinScaleDenominator>500</MinScaleDenominator>
      </Layer>
    </Layer>
  </Capability>
</WMS_Capabilities>"#;

    #[test]
    fn a_130_document_with_inheritance_and_the_systems_axis_order() {
        let c = read(V13).expect("reads");
        assert_eq!(c.version, "1.3.0");
        assert_eq!(c.title, "Belediye KBS");
        assert_eq!(c.formats, vec!["image/png", "image/jpeg"]);
        assert_eq!(c.info_formats, vec!["text/plain", "application/json"]);
        assert_eq!(
            c.get_map.as_deref(),
            Some("https://kbs.ornek.bel.tr/geoserver/wms?")
        );
        assert_eq!(c.max_width, Some(4096));
        assert_eq!(c.layers.len(), 2);
        let top = &c.layers[0];
        assert_eq!((top.name.as_deref(), top.depth), (None, 0));
        // EPSG:5254 is north first: the box comes out east first.
        assert_eq!(
            top.boxes,
            vec![(5254, [400_000.0, 4_400_000.0, 600_000.0, 4_500_000.0])]
        );
        let plan = &c.layers[1];
        assert_eq!(plan.name.as_deref(), Some("imar:plan"));
        assert!(plan.queryable);
        assert_eq!(plan.srids, vec![4326, 5254, 3857]);
        assert_eq!(plan.wgs84, Some([29.0, 40.0, 31.0, 41.0]));
        assert_eq!(
            plan.styles
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>(),
            vec!["default", "renkli"]
        );
        assert_eq!(
            plan.styles[1].legend.as_deref(),
            Some("https://x/legend.png")
        );
        assert_eq!(plan.min_scale, Some(500.0));
    }

    #[test]
    fn a_111_document_and_errors() {
        let v11 = r#"<?xml version="1.0"?>
<!DOCTYPE WMT_MS_Capabilities SYSTEM "http://schemas.opengis.net/wms/1.1.1/WMS_MS_Capabilities.dtd">
<WMT_MS_Capabilities version="1.1.1"><Service><Title>Eski</Title></Service><Capability>
<Request><GetMap><Format>image/png</Format></GetMap></Request>
<Layer><Title>Kök</Title><SRS>EPSG:4326 EPSG:32636</SRS><LatLonBoundingBox minx="26" miny="36" maxx="45" maxy="42"/>
<BoundingBox SRS="EPSG:4326" minx="26" miny="36" maxx="45" maxy="42"/>
<Layer><Name>il</Name><Title>İller</Title></Layer></Layer></Capability></WMT_MS_Capabilities>"#;
        let c = read(v11).expect("reads");
        assert_eq!(c.version, "1.1.1");
        assert_eq!(c.layers[1].srids, vec![4326, 32636]);
        // 1.1.1 boxes are east first even in EPSG:4326.
        assert_eq!(c.layers[1].boxes, vec![(4326, [26.0, 36.0, 45.0, 42.0])]);
        assert!(read(r#"<ServiceExceptionReport><ServiceException code="x">Katman yok</ServiceException></ServiceExceptionReport>"#)
            .unwrap_err()
            .contains("Katman yok"));
        assert!(
            read("<html><body>404</body></html>")
                .unwrap_err()
                .contains("WMS")
        );
        assert!(read("{\"a\":1}").unwrap_err().contains("JSON"));
    }
}
