//! Capabilities documents (docs/adr/0208 §5–§9): what a service says it
//! has, read into plain structures the windows list and the requests use.
//! XML is read by local names (any namespace prefix); a document that is not
//! one of these, or a service's exception report, is said in words. Every
//! box comes out as east and north (longitude and latitude), whatever order
//! the document wrote it in.

pub mod arcgis;
pub mod ogc;
pub mod tilejson;
pub mod wfs;
pub mod wms;
pub mod wmts;

use roxmltree::{Document, Node};

/// The most bytes of XML a capabilities document may be (some ArcGIS and
/// GeoServer documents run to tens of MB).
pub const MAX_XML: usize = 64 * 1024 * 1024;
/// The most nested elements read.
const MAX_DEPTH: u32 = 256;

/// The XML document of `text`, or why it is none.
pub(crate) fn xml(text: &str) -> Result<Document<'_>, String> {
    if text.len() > MAX_XML {
        return Err(format!(
            "Yanıt {} MB; en çok {} MB okunur.",
            text.len() / (1024 * 1024),
            MAX_XML / (1024 * 1024)
        ));
    }
    let opts = roxmltree::ParsingOptions {
        allow_dtd: true,
        nodes_limit: 20_000_000,
    };
    let doc = Document::parse_with_options(text, opts).map_err(|e| {
        let head: String = text.trim_start().chars().take(60).collect();
        if head.starts_with('{') {
            "Yanıt XML değil, JSON: adres bir OGC API ya da ArcGIS servisi olabilir.".to_owned()
        } else if head.to_ascii_lowercase().starts_with("<!doctype html")
            || head.to_ascii_lowercase().starts_with("<html")
        {
            "Yanıt bir web sayfası, yetenek belgesi değil: adresi denetleyin.".to_owned()
        } else {
            format!("Yanıt okunamadı (XML değil): {e}.")
        }
    })?;
    if depth(doc.root_element(), 0) > MAX_DEPTH {
        return Err(format!("Belge {MAX_DEPTH} düzeyden derin; okunmadı."));
    }
    if let Some(words) = exception(doc.root_element()) {
        return Err(words);
    }
    Ok(doc)
}

fn depth(n: Node<'_, '_>, d: u32) -> u32 {
    if d > MAX_DEPTH {
        return d;
    }
    n.children()
        .filter(Node::is_element)
        .map(|c| depth(c, d + 1))
        .max()
        .unwrap_or(d)
}

/// The words of a service's exception report (`ServiceExceptionReport`,
/// `ExceptionReport`), if the document is one.
pub(crate) fn exception(root: Node<'_, '_>) -> Option<String> {
    let name = root.tag_name().name();
    if !name.ends_with("ExceptionReport") {
        return None;
    }
    let words: Vec<String> = root
        .descendants()
        .filter(|n| {
            let t = n.tag_name().name();
            t == "ServiceException" || t == "ExceptionText"
        })
        .filter_map(|n| text_of(n))
        .collect();
    Some(if words.is_empty() {
        "Servis bir hata bildirdi.".to_owned()
    } else {
        format!("Servis bir hata bildirdi: {}", words.join(" "))
    })
}

/// The first child element of `n` named `name` (local name).
pub(crate) fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children()
        .find(|c| c.is_element() && c.tag_name().name() == name)
}

/// Every child element of `n` named `name`.
pub(crate) fn children<'a, 'i: 'a>(
    n: Node<'a, 'i>,
    name: &'a str,
) -> impl Iterator<Item = Node<'a, 'i>> + 'a {
    n.children()
        .filter(move |c| c.is_element() && c.tag_name().name() == name)
}

/// The trimmed text of `n`; none when empty.
pub(crate) fn text_of(n: Node<'_, '_>) -> Option<String> {
    let t: String = n
        .descendants()
        .filter(Node::is_text)
        .filter_map(|t| t.text())
        .collect();
    let t = t.trim();
    (!t.is_empty()).then(|| t.to_owned())
}

/// The text of `n`'s child `name`.
pub(crate) fn child_text(n: Node<'_, '_>, name: &str) -> Option<String> {
    child(n, name).and_then(text_of)
}

/// The attribute `name` of `n` (local name, any namespace).
pub(crate) fn attr<'a>(n: Node<'a, '_>, name: &str) -> Option<&'a str> {
    n.attributes().find(|a| a.name() == name).map(|a| a.value())
}

/// The `xlink:href` of `n`'s first `OnlineResource` (itself or a child).
pub(crate) fn href(n: Node<'_, '_>) -> Option<String> {
    let r = if n.tag_name().name() == "OnlineResource" {
        Some(n)
    } else {
        n.descendants()
            .find(|c| c.is_element() && c.tag_name().name() == "OnlineResource")
    }?;
    attr(r, "href")
        .map(|h| h.trim().to_owned())
        .filter(|h| !h.is_empty())
}

/// Two numbers of a `"a b"` text (an OWS corner).
pub(crate) fn pair(text: &str) -> Option<(f64, f64)> {
    let mut it = text.split_whitespace().map(str::parse::<f64>);
    let a = it.next()?.ok()?;
    let b = it.next()?.ok()?;
    (a.is_finite() && b.is_finite()).then_some((a, b))
}

/// A number attribute.
pub(crate) fn num_attr(n: Node<'_, '_>, name: &str) -> Option<f64> {
    attr(n, name)
        .and_then(|v| v.trim().parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

/// An OWS `WGS84BoundingBox` child of `n`: `[west, south, east, north]`.
pub(crate) fn wgs84_box(n: Node<'_, '_>) -> Option<[f64; 4]> {
    let b = child(n, "WGS84BoundingBox")?;
    let (w, s) = pair(&child_text(b, "LowerCorner")?)?;
    let (e, no) = pair(&child_text(b, "UpperCorner")?)?;
    Some([w, s, e, no])
}

/// Fixes the ordering of a box read: west ≤ east, south ≤ north.
pub(crate) fn ordered(b: [f64; 4]) -> [f64; 4] {
    [
        if b[0] <= b[2] { b[0] } else { b[2] },
        if b[1] <= b[3] { b[1] } else { b[3] },
        if b[0] <= b[2] { b[2] } else { b[0] },
        if b[1] <= b[3] { b[3] } else { b[1] },
    ]
}

/// A JSON document, or why it is none (an HTML page, an XML exception).
pub(crate) fn json(text: &str) -> Result<serde_json::Value, String> {
    serde_json::from_str(text).map_err(|_| {
        let head: String = text
            .trim_start()
            .chars()
            .take(60)
            .collect::<String>()
            .to_ascii_lowercase();
        if head.starts_with('<') {
            if head.contains("exception") {
                xml(text)
                    .err()
                    .unwrap_or_else(|| "Servis bir hata bildirdi.".to_owned())
            } else if head.starts_with("<!doctype html") || head.starts_with("<html") {
                "Yanıt bir web sayfası, JSON değil: adresi denetleyin.".to_owned()
            } else {
                "Yanıt JSON değil, XML: adres bir OGC servisi (WMS, WMTS, WFS) olabilir.".to_owned()
            }
        } else {
            "Yanıt okunamadı (JSON değil).".to_owned()
        }
    })
}

/// A metres-a-unit factor for an OGC scale denominator (0.28 mm pixels):
/// 1 for a system in metres, 2πR / 360 for one in degrees (R = 6 378 137 m,
/// WMTS 1.0 Annex E).
pub(crate) fn metres_per_unit(srid: u32) -> f64 {
    if crate::crs::degrees(srid) || srid == 4326 {
        2.0 * std::f64::consts::PI * 6_378_137.0 / 360.0
    } else {
        1.0
    }
}
