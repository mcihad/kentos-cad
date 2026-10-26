//! An OGC API collection's queryables (OGC API Features Part 3, docs/adr/0655 §3): the JSON Schema of the properties a
//! service can filter on, read as typed layer fields, so the layer a take makes has the service's types and titles
//! instead of types guessed from values. A field takes the schema's kind only when every value taken fits it; a
//! property the schema does not know keeps its guessed field.

use kentos_contracts::fields::{LayerField, LayerFieldKind, check_value};

/// The queryables' address of a collection whose items are at `items_url` (`…/collections/{id}/items`).
pub fn url(items_url: &str) -> String {
    let base = items_url.split(['?', '#']).next().unwrap_or(items_url);
    let base = base.trim_end_matches('/');
    let base = base.strip_suffix("/items").unwrap_or(base);
    format!("{base}/queryables?f=json")
}

/// The schema's properties as fields, in its order: `integer` an integer, `number` a decimal, `boolean` true/false, a
/// string of format `date` a date and `date-time` a date and time, any other string text; its `title` the alias. A
/// geometry (`x-ogc-role: primary-geometry`, a `geometry-…` format, a GeoJSON `$ref`) and an unknown type are left out.
pub fn fields(body: &str) -> Vec<LayerField> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    let Some(props) = v["properties"].as_object() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (name, p) in props {
        let geometry = p["x-ogc-role"] == "primary-geometry"
            || p["format"]
                .as_str()
                .is_some_and(|f| f.starts_with("geometry"))
            || p["$ref"].as_str().is_some_and(|r| r.contains("geojson"));
        if geometry || name.is_empty() {
            continue;
        }
        let ty = match &p["type"] {
            serde_json::Value::String(t) => Some(t.as_str()),
            // `["string", "null"]`: the kind that is not null.
            serde_json::Value::Array(ts) => {
                ts.iter().filter_map(|t| t.as_str()).find(|t| *t != "null")
            }
            _ => None,
        };
        let kind = match (ty, p["format"].as_str()) {
            (Some("integer"), _) => LayerFieldKind::Integer,
            (Some("number"), _) => LayerFieldKind::Decimal,
            (Some("boolean"), _) => LayerFieldKind::Boolean,
            (Some("string"), Some("date")) => LayerFieldKind::Date,
            (Some("string"), Some("date-time")) => LayerFieldKind::DateTime,
            (Some("string"), _) => LayerFieldKind::Text,
            _ => continue,
        };
        let mut f = LayerField::new(name.clone(), kind);
        f.alias = p["title"]
            .as_str()
            .map(str::trim)
            .filter(|t| !t.is_empty() && *t != name)
            .map(str::to_owned);
        out.push(f);
    }
    out
}

/// `guessed` (the fields made from the values) with the schema's kind and title where the schema names the field and
/// every value of `rows` fits that kind; an empty value always fits.
pub fn typed<'a>(
    guessed: Vec<LayerField>,
    schema: &[LayerField],
    rows: impl IntoIterator<Item = &'a [(String, String)]> + Clone,
) -> Vec<LayerField> {
    guessed
        .into_iter()
        .map(|g| {
            let Some(s) = schema.iter().find(|s| s.name == g.name) else {
                return g;
            };
            let fits = rows.clone().into_iter().all(|row| {
                row.iter()
                    .filter(|(k, v)| *k == g.name && !v.is_empty())
                    .all(|(_, v)| check_value(s, v).is_ok())
            });
            if fits {
                s.clone()
            } else {
                LayerField {
                    alias: s.alias.clone(),
                    ..g
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCHEMA: &str = r#"{"type":"object","properties":{
        "geometry":{"x-ogc-role":"primary-geometry","format":"geometry-polygon"},
        "ada":{"type":"integer","title":"Ada no"},
        "alan":{"type":"number"},
        "tescil":{"type":"string","format":"date"},
        "guncel":{"type":["string","null"],"format":"date-time"},
        "malik":{"type":"string","title":"malik"},
        "aktif":{"type":"boolean"},
        "nesne":{"type":"object"}}}"#;

    #[test]
    fn the_address_is_beside_the_items() {
        assert_eq!(
            url("https://s/ogc/collections/parsel/items?limit=10"),
            "https://s/ogc/collections/parsel/queryables?f=json"
        );
        assert_eq!(
            url("https://s/collections/a/"),
            "https://s/collections/a/queryables?f=json"
        );
    }

    #[test]
    fn the_schema_s_properties_are_typed_fields() {
        let f = fields(SCHEMA);
        let kinds: Vec<(&str, LayerFieldKind)> =
            f.iter().map(|f| (f.name.as_str(), f.kind)).collect();
        assert_eq!(
            kinds,
            vec![
                ("ada", LayerFieldKind::Integer),
                ("aktif", LayerFieldKind::Boolean),
                ("alan", LayerFieldKind::Decimal),
                ("guncel", LayerFieldKind::DateTime),
                ("malik", LayerFieldKind::Text),
                ("tescil", LayerFieldKind::Date),
            ]
        );
        assert_eq!(f[0].alias.as_deref(), Some("Ada no"));
        assert_eq!(f[4].alias, None, "a title that is the name");
        assert!(fields("değil").is_empty());
        assert!(fields("{}").is_empty());
    }

    #[test]
    fn a_kind_is_taken_only_when_every_value_fits() {
        let rows: Vec<Vec<(String, String)>> = vec![
            vec![("ada".into(), "101".into()), ("alan".into(), "12,5".into())],
            vec![("ada".into(), "".into()), ("alan".into(), "x".into())],
        ];
        let guessed = vec![
            LayerField::new("ada", LayerFieldKind::Text),
            LayerField::new("alan", LayerFieldKind::Text),
            LayerField::new("yok", LayerFieldKind::Text),
        ];
        let t = typed(guessed, &fields(SCHEMA), rows.iter().map(Vec::as_slice));
        assert_eq!(t[0].kind, LayerFieldKind::Integer);
        assert_eq!(t[0].alias.as_deref(), Some("Ada no"));
        assert_eq!(t[1].kind, LayerFieldKind::Text, "“x” is not a number");
        assert_eq!(t[2].kind, LayerFieldKind::Text, "not in the schema");
    }
}
