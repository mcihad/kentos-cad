//! A PDF page as an external reference's source drawing (docs/adr/0351 §2; plan/cad.md `CAD-REF-17`): the page's
//! paths and texts (`kentos_formats::pdf`) as the drawing's objects, in paper metres from the page's lower left
//! corner, so the reference's loading, reload, kept local ids, layer group and packaging are the drawing's. Its
//! optional content groups are its layers (“PDF” for what is in none; a group off in the PDF's default view is a
//! hidden layer). A stroke is a polyline in its colour, width and dash; a fill an area; a text a text with its height
//! and turn. Black is the drawing's ink (it shows on a dark screen as on paper); a white fill is the paper and is left
//! out.

use kentos_commands::DocumentSnapshotV2;
use kentos_formats::pdf::{PdfLeft, PdfPage};
use serde_json::{Value, json};

/// Points to metres: 1 pt is 1/72 inch.
pub const PT: f64 = 0.0254 / 72.0;

fn color(c: [u8; 3]) -> Option<String> {
    (c != [0, 0, 0]).then(|| format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]))
}

fn point(p: [f64; 2]) -> Value {
    json!({ "x": p[0] * PT, "y": p[1] * PT })
}

/// The layer id of a page's layer `i`, or of what is in no group.
fn layer_id(i: Option<usize>) -> String {
    i.map_or_else(|| "pdf".to_owned(), |i| format!("pdf-{}", i + 1))
}

fn layer(id: &str, name: &str, visible: bool) -> Value {
    json!({ "id": id, "name": name, "type": "layer", "visible": visible, "locked": false, "expanded": true,
            "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.18 }, "children": [] })
}

fn with_look(mut e: Value, c: Option<String>, weight: Option<f64>, dashed: bool) -> Value {
    if let Some(c) = c {
        e["color"] = json!(c);
    }
    if let Some(w) = weight {
        e["lineWeight"] = json!(w);
    }
    if dashed {
        e["lineType"] = json!("dashed");
    }
    e
}

/// The page as a drawing named `name`, and what it left out.
pub fn drawing(page: &PdfPage, name: &str) -> Result<(DocumentSnapshotV2, PdfLeft), String> {
    let mut entities: Vec<Value> = Vec::new();
    let mut used_plain = false;
    for path in &page.paths {
        let lid = layer_id(path.layer);
        used_plain |= path.layer.is_none();
        if let Some(fill) = path.fill
            && fill != [255, 255, 255]
        {
            for (pts, _) in &path.parts {
                let mut ring: Vec<[f64; 2]> = pts.clone();
                if ring.len() > 1 && ring.first() == ring.last() {
                    ring.pop();
                }
                if ring.len() < 3 {
                    continue;
                }
                let e = json!({ "kind": "polygon", "layerId": lid, "attrs": {},
                                "pts": ring.iter().map(|p| point(*p)).collect::<Vec<_>>() });
                entities.push(with_look(e, color(fill), None, false));
            }
        }
        if let Some(stroke) = &path.stroke {
            // Its width on paper, millimetres; a hairline (0) is the thinnest.
            let weight = (stroke.width > 0.0).then(|| (stroke.width * 25.4 / 72.0 * 100.0).round() / 100.0);
            for (pts, closed) in &path.parts {
                let mut line: Vec<[f64; 2]> = pts.clone();
                let mut closed = *closed;
                if line.len() > 2 && line.first() == line.last() {
                    line.pop();
                    closed = true;
                }
                line.dedup();
                if line.len() < 2 {
                    continue;
                }
                let e = if line.len() == 2 && !closed {
                    json!({ "kind": "line", "layerId": lid, "attrs": {}, "a": point(line[0]), "b": point(line[1]) })
                } else {
                    let mut e = json!({ "kind": "polyline", "layerId": lid, "attrs": {},
                                        "pts": line.iter().map(|p| point(*p)).collect::<Vec<_>>() });
                    if closed {
                        e["closed"] = json!(true);
                    }
                    e
                };
                entities.push(with_look(e, color(stroke.color), weight.filter(|w| *w > 0.0), stroke.dashed));
            }
        }
    }
    for t in &page.texts {
        used_plain |= t.layer.is_none();
        let e = json!({ "kind": "text", "layerId": layer_id(t.layer), "attrs": {}, "p": point(t.p),
                        "text": t.text, "height": t.height * PT, "rotation": t.angle.to_degrees() });
        entities.push(with_look(e, color(t.color), None, false));
    }
    let mut layers = Vec::new();
    if used_plain || page.layers.is_empty() {
        layers.push(layer("pdf", "PDF", true));
    }
    for (i, l) in page.layers.iter().enumerate() {
        layers.push(layer(&layer_id(Some(i)), l, !page.hidden.get(i).copied().unwrap_or(false)));
    }
    let active = layers.first().and_then(|l| l["id"].as_str()).unwrap_or("pdf").to_owned();
    let v1 = json!({
        "format": "kentos.document", "version": 1, "name": name,
        "settings": { "srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                      "plotScale": 1, "workspace": "cad" },
        "origin": { "x": 0.0, "y": 0.0 }, "layers": layers, "activeLayer": active,
        "entities": entities.into_iter().enumerate().map(|(k, mut e)| { e["id"] = json!(k + 1); e }).collect::<Vec<_>>(),
        "styles": { "items": [], "categories": [] }
    });
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&v1.to_string()).map_err(|e| e.to_string())?;
    let doc = kentos_domain::Document::from_snapshot(snapshot)?;
    Ok((doc.to_snapshot_v2(), page.left.clone()))
}

/// What the page left out, in a warning's words; none when it took everything.
pub fn left_words(left: &PdfLeft) -> Option<String> {
    let mut parts = Vec::new();
    if left.images > 0 {
        parts.push(format!("{} resim", left.images));
    }
    if left.shadings > 0 {
        parts.push(format!("{} gölgeleme", left.shadings));
    }
    if left.unreadable > 0 {
        parts.push(format!("{} okunamayan harf", left.unreadable));
    }
    if left.truncated {
        parts.push("sayfanın sınırı aşan geri kalanı".to_owned());
    }
    (!parts.is_empty()).then(|| format!("PDF sayfasından altlığa alınmadı: {}.", parts.join(", ")))
}
