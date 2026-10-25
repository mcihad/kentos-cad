//! The style exchange's profile (docs/adr/0635 §8): which renderer and which symbol layer each style format carries
//! both ways, writes as something else, or loses (`catalog/v1/style-exchange.json`). Each format's crate checks the
//! profile against itself: every row's sample is written, read back and judged here (`check`), so the profile says
//! what the code does.

use serde_json::{Value, json};

use crate::sld::Kinds;

/// The profile, as the catalogue keeps it.
pub const PROFILE: &str = include_str!("../../../../catalog/v1/style-exchange.json");

/// What a format does with a renderer or a symbol layer.
pub const BOTH: &str = "both";
pub const WRITE: &str = "write";
pub const LOST: &str = "lost";

/// The profile's JSON.
pub fn profile() -> Value {
    serde_json::from_str(PROFILE).unwrap_or(Value::Null)
}

/// What the profile says `format` does with the renderer `kind`.
pub fn renderer_claim(format: &str, kind: &str) -> Option<String> {
    claim("renderers", format, kind)
}

/// What the profile says `format` does with the symbol layer `kind`.
pub fn layer_claim(format: &str, kind: &str) -> Option<String> {
    claim("layers", format, kind)
}

fn claim(list: &str, format: &str, kind: &str) -> Option<String> {
    profile()[list]
        .as_array()?
        .iter()
        .find(|r| r["kind"] == kind)?
        .get(format)?
        .as_str()
        .map(str::to_owned)
}

fn kinds_of(geometry: &str) -> Kinds {
    Kinds {
        point: matches!(geometry, "point" | "marker"),
        line: geometry == "line",
        area: matches!(geometry, "area" | "fill"),
    }
}

fn has_layer(v: &Value, kind: &str) -> bool {
    match v {
        Value::Object(o) => {
            (o.get("type").and_then(Value::as_str) == Some(kind)
                && (o.contains_key("id") || !o.contains_key("layers")))
                || o.values().any(|x| has_layer(x, kind))
        }
        Value::Array(a) => a.iter().any(|x| has_layer(x, kind)),
        _ => false,
    }
}

fn judged(same: bool, lost: &[String]) -> &'static str {
    if same {
        BOTH
    } else if lost.is_empty() {
        WRITE
    } else {
        LOST
    }
}

/// A row carried both ways with a loss has its note, and a note is of a loss.
fn noted(
    row: &Value,
    format: &str,
    got: &str,
    lost: &[String],
    what: &str,
    wrong: &mut Vec<String>,
) {
    let note = row["notes"].get(format).is_some();
    if got == BOTH && !lost.is_empty() && !note {
        wrong.push(format!("{what}: {format} için not yok ({lost:?})"));
    }
    if note && (got != BOTH || lost.is_empty()) {
        wrong.push(format!("{what}: {format} için not var ama kayıp yok"));
    }
}

/// A format's write and read back of a renderer for objects of some kinds: what writing lost and the renderer read
/// back, if any.
pub type Exchange<'a> = &'a dyn Fn(&Value, Kinds) -> (Vec<String>, Option<Value>);

/// Each row of the profile written by `exchange` (the renderer and its objects' kinds → what writing lost and the
/// renderer read back, if any) and judged; the rows whose claim for `format` is not what happened, in words.
pub fn check(format: &str, exchange: Exchange<'_>) -> Vec<String> {
    let p = profile();
    let mut wrong = Vec::new();
    for r in p["renderers"].as_array().into_iter().flatten() {
        let kind = r["kind"].as_str().unwrap_or("");
        let (lost, back) = exchange(&r["sample"], kinds_of(r["geometry"].as_str().unwrap_or("")));
        let same = back.as_ref().is_some_and(|b| b["type"] == kind);
        let got = judged(same, &lost);
        if r[format] != got {
            wrong.push(format!(
                "işleyici {kind}: profil {}, kod {got} ({lost:?})",
                r[format]
            ));
        }
        noted(
            r,
            format,
            got,
            &lost,
            &format!("işleyici {kind}"),
            &mut wrong,
        );
    }
    for l in p["layers"].as_array().into_iter().flatten() {
        let kind = l["kind"].as_str().unwrap_or("");
        let symbol = l["symbol"].as_str().unwrap_or("fill");
        let renderer = json!({ "type": "single",
            "symbols": { symbol: { "type": symbol, "layers": [l["sample"].clone()] } } });
        let (lost, back) = exchange(&renderer, kinds_of(symbol));
        let same = back.as_ref().is_some_and(|b| has_layer(b, kind));
        let got = judged(same, &lost);
        if l[format] != got {
            wrong.push(format!(
                "sembol katmanı {kind}: profil {}, kod {got} ({lost:?})",
                l[format]
            ));
        }
        noted(
            l,
            format,
            got,
            &lost,
            &format!("sembol katmanı {kind}"),
            &mut wrong,
        );
    }
    wrong
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sld_does_what_the_profile_says() {
        let wrong = check("sld", &|renderer, kinds| {
            let written = crate::sld::write("katman", renderer, &[], kinds, &|_| None);
            let back = crate::sld::read(&written.xml)
                .ok()
                .map(|r| r.renderer)
                .filter(|r| !r.is_null());
            (written.lost, back)
        });
        assert!(wrong.is_empty(), "{wrong:#?}");
    }
}
