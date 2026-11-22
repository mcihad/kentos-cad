//! A data layer's rows as texts (docs/adr/9114): a row of a text table made the text it says, to be drawn or taken
//! into an edit; a text made its row's values, to be moved into a table or saved back. The mapping (which column holds
//! what) and the cells' rules are the contract's (`DataText`); this makes them objects and values.

use kentos_commands::{AnnotationKind, DataText, Entity, ProjectSettings, TextCell, TextEntity, TextParts};
use serde_json::{Map, Value as J, json};

/// The height a row without one takes: the project's text height at its drawing scale, as Yazı's.
pub fn default_height(settings: &ProjectSettings) -> f64 {
    kentos_commands::paper_height(settings.annotation_mm(AnnotationKind::Text), settings.plot_scale)
}

/// A cell of an object's attributes as its text.
fn cell(attrs: &Map<String, J>, column: &str) -> Option<String> {
    match attrs.get(column)? {
        J::String(s) => Some(s.clone()),
        J::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// A row of a text table as the text it says, at `(x, y)` of the project, an object of the layer `layer`: its values
/// kept as its attributes, as the drawing keeps an object's. None for a row without words.
pub fn object(
    text: &DataText,
    at: (f64, f64),
    layer: &str,
    id: u32,
    attrs: Map<String, J>,
    default_height: f64,
) -> Option<Entity> {
    let parts = text.read(|c| cell(&attrs, c), default_height)?;
    let mut e = json!({
        "kind": "text",
        "id": id,
        "layerId": layer,
        "attrs": attrs,
        "p": { "x": at.0, "y": at.1 },
        "text": parts.text,
        "height": parts.height,
        "rotation": parts.rotation,
    });
    if let Some(a) = parts.align {
        e["align"] = J::String(a.name().to_owned());
    }
    if let Some(w) = parts.width_factor {
        e["widthFactor"] = json!(w);
    }
    serde_json::from_value(e).ok()
}

/// Where a row's text stands: its point (a multipoint's first).
pub fn point_of(g: &kentos_data::Geometry) -> Option<(f64, f64)> {
    match g {
        kentos_data::Geometry::Point(Some(c)) => Some((c.x, c.y)),
        kentos_data::Geometry::MultiPoint(points) => points.first().map(|c| (c.x, c.y)),
        _ => None,
    }
}

/// A text's parts as its table keeps them.
pub fn parts_of(t: &TextEntity) -> TextParts {
    TextParts {
        text: t.text.clone(),
        height: t.height,
        rotation: t.rotation,
        align: t.align,
        width_factor: t.width_factor,
    }
}

/// Whether a text has what its table has no column for: a style or typeface of its own, a mask, a curve, scales, a link
/// to an object, paragraph formats (docs/adr/9114 §2).
pub fn details_left(t: &TextEntity) -> bool {
    t.face != Default::default()
        || t.paragraph != Default::default()
        || t.mask
        || t.path.is_some()
        || t.annotative.is_some()
        || t.label_of.is_some()
        || t.label_scale.is_some()
}

/// A text's cell as a source's value.
pub fn value_of(cell: &TextCell) -> kentos_data::Value {
    match cell {
        TextCell::Text(t) => kentos_data::Value::Text(t.clone()),
        TextCell::Number(n) => kentos_data::Value::Real(*n),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_is_drawn_as_its_text_and_its_text_writes_the_row_back() {
        let attrs: Map<String, J> = serde_json::from_value(json!({
            "metin": "Ada 101", "yukseklik": "2.5", "aci": "90", "hiza": "orta", "genislik": "0.8", "tur": "ada"
        }))
        .expect("attrs");
        let e = object(&DataText::written(), (600010.0, 4410020.0), "yazilar", 7, attrs, 2.0).expect("a text");
        let Entity::Text(t) = &e else { panic!("{e:?}") };
        assert_eq!((t.text.as_str(), t.height, t.rotation), ("Ada 101", 2.5, 90.0));
        assert_eq!(t.align, Some(kentos_commands::TextAlign::MiddleCenter));
        assert_eq!(t.width_factor, Some(0.8));
        assert_eq!((t.p.x, t.p.y), (600010.0, 4410020.0));
        assert_eq!(t.base.attrs.get("tur").map(String::as_str), Some("ada"), "its other values kept");
        let cells = DataText::written().cells(&parts_of(t));
        let values: Vec<(String, kentos_data::Value)> = cells.iter().map(|(c, v)| (c.clone(), value_of(v))).collect();
        assert_eq!(values[1], ("yukseklik".to_owned(), kentos_data::Value::Real(2.5)));
        assert_eq!(values[3], ("hiza".to_owned(), kentos_data::Value::Text("orta".into())));
        assert!(!details_left(t), "a plain text loses nothing");
    }

    #[test]
    fn a_row_without_words_is_no_text() {
        let attrs: Map<String, J> = serde_json::from_value(json!({ "metin": "" })).expect("attrs");
        assert!(object(&DataText::written(), (0.0, 0.0), "y", 1, attrs, 2.0).is_none());
    }
}
