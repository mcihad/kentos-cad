//! An extension tool's examples (docs/adr/8057 §48, PY-TOOL-23): each a few synthetic objects (`features`: a layer's
//! name, a GeoJSON shape, attributes), the tool's parameters (`params`) and what its run should give (`expect`: an
//! output's value, a features output's count and attributes some written object has, words of the run's summary).
//! The host runs them on a drawing of their own, never the user's; here only their form is read.

use serde_json::Value;

/// The most examples a tool's descriptor gives.
pub const MOST_EXAMPLES: usize = 10;

/// The most objects an example gives.
pub const MOST_EXAMPLE_FEATURES: usize = 500;

/// A descriptor's examples as given, their form checked; none when it gives none. Why not, with the example's place.
pub(super) fn examples_of(value: Option<&Value>) -> Result<Vec<Value>, String> {
    let Some(value) = value.filter(|v| !v.is_null()) else {
        return Ok(Vec::new());
    };
    let list = value
        .as_array()
        .ok_or_else(|| "examples bir örnek listesi ([…]) olmalı.".to_owned())?;
    if list.len() > MOST_EXAMPLES {
        return Err(format!(
            "En çok {MOST_EXAMPLES} örnek olur; {} örnek var.",
            list.len()
        ));
    }
    for (i, example) in list.iter().enumerate() {
        let at = format!("examples[{i}]");
        let o = example
            .as_object()
            .ok_or_else(|| format!("{at}: bir nesne ({{…}}) olmalı."))?;
        let title = o.get("title").and_then(Value::as_str).map(str::trim);
        if !title.is_some_and(|t| !t.is_empty() && t.chars().count() <= 80) {
            return Err(format!(
                "{at}.title: örneğin adı (en çok 80 karakter) gerekli."
            ));
        }
        let features = o
            .get("features")
            .and_then(Value::as_array)
            .filter(|f| !f.is_empty() && f.len() <= MOST_EXAMPLE_FEATURES)
            .ok_or_else(|| {
                format!(
                    "{at}.features: 1 ile {MOST_EXAMPLE_FEATURES} arasında sentetik nesne olmalı."
                )
            })?;
        for (j, f) in features.iter().enumerate() {
            let layer = f.get("layer").and_then(Value::as_str).map(str::trim);
            if !layer.is_some_and(|l| !l.is_empty() && l.chars().count() <= 80) {
                return Err(format!("{at}.features[{j}].layer: katmanın adı gerekli."));
            }
            if !f.get("geometry").is_some_and(Value::is_object) {
                return Err(format!(
                    "{at}.features[{j}].geometry: bir GeoJSON şekli olmalı."
                ));
            }
            if f.get("attrs").is_some_and(|a| !a.is_object()) {
                return Err(format!(
                    "{at}.features[{j}].attrs: öznitelikler bir nesne olmalı."
                ));
            }
        }
        if o.get("params").is_some_and(|p| !p.is_object()) {
            return Err(format!("{at}.params: parametreler bir nesne olmalı."));
        }
        if let Some(expect) = o.get("expect") {
            let e = expect
                .as_object()
                .ok_or_else(|| format!("{at}.expect: beklenen bir nesne olmalı."))?;
            if e.get("outputs").is_some_and(|x| !x.is_object()) {
                return Err(format!(
                    "{at}.expect.outputs: çıktıların beklenenleri bir nesne olmalı."
                ));
            }
            if e.get("summary").is_some_and(|x| !x.is_string()) {
                return Err(format!(
                    "{at}.expect.summary: özette geçmesi beklenen bir söz olmalı."
                ));
            }
        }
    }
    Ok(list.clone())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// None is no example; a whole one reads; a missing title, an object without a shape and too many are said with
    /// their place.
    #[test]
    fn an_example_is_read_with_its_form_checked() {
        assert_eq!(examples_of(None), Ok(Vec::new()));
        let one = json!({ "title": "Bir parsel", "features": [
            { "layer": "parsel", "geometry": { "type": "Point", "coordinates": [0, 0] }, "attrs": { "ada": "1" } }
        ], "params": { "taks_siniri": 0.4 }, "expect": { "outputs": { "asan": 1 }, "summary": "aşıyor" } });
        assert_eq!(examples_of(Some(&json!([one.clone()]))).unwrap().len(), 1);
        let mut untitled = one.clone();
        untitled["title"] = json!(" ");
        assert!(
            examples_of(Some(&json!([untitled])))
                .unwrap_err()
                .starts_with("examples[0].title")
        );
        let mut shapeless = one.clone();
        shapeless["features"][0]["geometry"] = json!("yok");
        assert!(
            examples_of(Some(&json!([shapeless])))
                .unwrap_err()
                .starts_with("examples[0].features[0].geometry")
        );
        let many: Vec<serde_json::Value> = (0..11).map(|_| one.clone()).collect();
        assert!(
            examples_of(Some(&json!(many)))
                .unwrap_err()
                .starts_with("En çok 10 örnek")
        );
    }
}
