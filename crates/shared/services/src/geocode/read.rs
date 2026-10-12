//! A provider's answers read (docs/adr/0229 §10): each answer's candidates
//! with their place (WGS 84), text, street, door, area names, neighbourhood,
//! district, province, postcode, name, the provider's kind and score. An
//! answer that says it failed is an error in its words; a candidate without
//! a place is skipped and counted; at most 50 are read. Area names come in a
//! kind's order; the neighbourhood is the first of its fields that says
//! MAHALLE or KÖY, else its first.

use serde_json::Value;

use super::providers::ProviderKind;
use kentos_geometry_core::ops::geocode::locator::Locator;

/// The most candidates read from one answer.
pub const READ_LIMIT: usize = 50;

/// A candidate as a provider gave it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OnlineCandidate {
    pub lon: f64,
    pub lat: f64,
    pub label: String,
    pub street: String,
    pub door: String,
    /// Its area names, in its kind's order.
    pub areas: Vec<String>,
    pub neighbourhood: String,
    pub district: String,
    pub province: String,
    pub postcode: String,
    pub name: String,
    pub kind: String,
    pub provider_score: Option<f64>,
}

/// An answer's candidates and how many were skipped.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Read {
    pub candidates: Vec<OnlineCandidate>,
    pub skipped: usize,
}

fn s(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(t)) => t.trim().to_owned(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(t) => t.trim().parse::<f64>().ok(),
        _ => None,
    }
    .filter(|x| x.is_finite())
}

/// The first non-empty of `keys` in `v`.
fn first(v: &Value, keys: &[&str]) -> String {
    keys.iter()
        .map(|k| s(v, k))
        .find(|t| !t.is_empty())
        .unwrap_or_default()
}

/// The non-empty values of `keys`, in order, each once.
fn all(v: &Value, keys: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for k in keys {
        let t = s(v, k);
        if !t.is_empty() && !out.contains(&t) {
            out.push(t);
        }
    }
    out
}

/// The neighbourhood among `keys`: the first that says MAHALLE or KÖY, else the first.
fn hood(v: &Value, keys: &[&str]) -> String {
    let values: Vec<String> = keys
        .iter()
        .map(|k| s(v, k))
        .filter(|t| !t.is_empty())
        .collect();
    values
        .iter()
        .find(|t| Locator::tagged_hood(t))
        .or(values.first())
        .cloned()
        .unwrap_or_default()
}

fn place(lon: Option<f64>, lat: Option<f64>) -> Option<(f64, f64)> {
    let (lon, lat) = (lon?, lat?);
    ((-180.0..=180.0).contains(&lon) && (-90.0..=90.0).contains(&lat)).then_some((lon, lat))
}

const NOMINATIM_STREETS: &[&str] = &[
    "road",
    "pedestrian",
    "footway",
    "cycleway",
    "path",
    "square",
];
const NOMINATIM_HOODS: &[&str] = &["suburb", "neighbourhood", "quarter", "village", "hamlet"];
const NOMINATIM_DISTRICTS: &[&str] = &[
    "town",
    "city_district",
    "district",
    "county",
    "municipality",
];
const NOMINATIM_PROVINCES: &[&str] = &["province", "state", "city"];
const NOMINATIM_AREAS: &[&str] = &[
    "suburb",
    "neighbourhood",
    "quarter",
    "village",
    "hamlet",
    "town",
    "city_district",
    "district",
    "county",
    "municipality",
    "city",
    "province",
    "state",
];

fn nominatim(v: &Value) -> Option<OnlineCandidate> {
    let (lon, lat) = place(v.get("lon").and_then(num), v.get("lat").and_then(num))?;
    let a = v.get("address").cloned().unwrap_or(Value::Null);
    Some(OnlineCandidate {
        lon,
        lat,
        label: s(v, "display_name"),
        street: first(&a, NOMINATIM_STREETS),
        door: s(&a, "house_number"),
        areas: all(&a, NOMINATIM_AREAS),
        neighbourhood: hood(&a, NOMINATIM_HOODS),
        district: first(&a, NOMINATIM_DISTRICTS),
        province: first(&a, NOMINATIM_PROVINCES),
        postcode: s(&a, "postcode"),
        name: s(v, "name"),
        kind: first(v, &["addresstype", "type"]),
        provider_score: v.get("importance").and_then(num),
    })
}

fn geojson_point(f: &Value) -> Option<(f64, f64)> {
    let c = f.get("geometry")?.get("coordinates")?.as_array()?;
    place(c.first().and_then(num), c.get(1).and_then(num))
}

fn photon(f: &Value) -> Option<OnlineCandidate> {
    let (lon, lat) = geojson_point(f)?;
    let p = f.get("properties").cloned().unwrap_or(Value::Null);
    let kind = s(&p, "type");
    let street = match s(&p, "street") {
        t if t.is_empty() && kind == "street" => s(&p, "name"),
        t => t,
    };
    let state = s(&p, "state");
    let city = s(&p, "city");
    let district = if !city.is_empty() && city != state {
        city
    } else {
        s(&p, "county")
    };
    let mut label: Vec<String> = Vec::new();
    for t in [
        s(&p, "name"),
        format!("{} {}", street, s(&p, "housenumber"))
            .trim()
            .to_owned(),
        s(&p, "district"),
        s(&p, "city"),
        s(&p, "state"),
        s(&p, "postcode"),
    ] {
        if !t.is_empty() && !label.contains(&t) {
            label.push(t);
        }
    }
    Some(OnlineCandidate {
        lon,
        lat,
        label: label.join(", "),
        street,
        door: s(&p, "housenumber"),
        areas: all(&p, &["district", "locality", "city", "county", "state"]),
        neighbourhood: hood(&p, &["district", "locality"]),
        district,
        province: state,
        postcode: s(&p, "postcode"),
        name: s(&p, "name"),
        kind,
        provider_score: None,
    })
}

fn pelias(f: &Value) -> Option<OnlineCandidate> {
    let (lon, lat) = geojson_point(f)?;
    let p = f.get("properties").cloned().unwrap_or(Value::Null);
    Some(OnlineCandidate {
        lon,
        lat,
        label: s(&p, "label"),
        street: s(&p, "street"),
        door: s(&p, "housenumber"),
        areas: all(
            &p,
            &[
                "neighbourhood",
                "borough",
                "locality",
                "localadmin",
                "county",
                "region",
            ],
        ),
        neighbourhood: hood(&p, &["neighbourhood", "borough"]),
        district: first(&p, &["county", "localadmin", "locality"]),
        province: s(&p, "region"),
        postcode: s(&p, "postalcode"),
        name: s(&p, "name"),
        kind: s(&p, "layer"),
        provider_score: p.get("confidence").and_then(num),
    })
}

fn arcgis_street(a: &Value) -> String {
    ["StPreType", "StName", "StType"]
        .iter()
        .map(|k| s(a, k))
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn arcgis(c: &Value) -> Option<OnlineCandidate> {
    let loc = c.get("location")?;
    let (lon, lat) = place(loc.get("x").and_then(num), loc.get("y").and_then(num))?;
    let a = c.get("attributes").cloned().unwrap_or(Value::Null);
    let label = match first(&a, &["LongLabel", "Match_addr"]) {
        t if t.is_empty() => s(c, "address"),
        t => t,
    };
    Some(OnlineCandidate {
        lon,
        lat,
        label,
        street: arcgis_street(&a),
        door: s(&a, "AddNum"),
        areas: all(&a, &["Nbrhd", "District", "City", "Subregion", "Region"]),
        neighbourhood: hood(&a, &["Nbrhd"]),
        district: first(&a, &["City", "District", "Subregion"]),
        province: s(&a, "Region"),
        postcode: s(&a, "Postal"),
        name: s(&a, "PlaceName"),
        kind: s(&a, "Addr_type"),
        provider_score: c.get("score").and_then(num),
    })
}

fn arcgis_reverse(v: &Value) -> Option<OnlineCandidate> {
    let loc = v.get("location")?;
    let (lon, lat) = place(loc.get("x").and_then(num), loc.get("y").and_then(num))?;
    let a = v.get("address").cloned().unwrap_or(Value::Null);
    let door = s(&a, "AddNum");
    let address = s(&a, "Address");
    let street = if door.is_empty() {
        address
    } else {
        let words: Vec<&str> = address.split_whitespace().collect();
        let kept: Vec<&str> = match (words.first(), words.last()) {
            (Some(f), _) if *f == door => words[1..].to_vec(),
            (_, Some(l)) if *l == door => words[..words.len() - 1].to_vec(),
            _ => words,
        };
        kept.join(" ")
    };
    Some(OnlineCandidate {
        lon,
        lat,
        label: first(&a, &["LongLabel", "Match_addr"]),
        street,
        door,
        areas: all(
            &a,
            &["Neighborhood", "District", "City", "Subregion", "Region"],
        ),
        neighbourhood: hood(&a, &["Neighborhood"]),
        district: first(&a, &["City", "District", "Subregion"]),
        province: s(&a, "Region"),
        postcode: s(&a, "Postal"),
        name: s(&a, "PlaceName"),
        kind: s(&a, "Addr_type"),
        provider_score: None,
    })
}

/// A Google result's component of `kind`, its long name.
fn component(r: &Value, kind: &str) -> String {
    r.get("address_components")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|c| {
            c.get("types")
                .and_then(Value::as_array)
                .is_some_and(|t| t.iter().any(|x| x.as_str() == Some(kind)))
        })
        .map(|c| s(c, "long_name"))
        .unwrap_or_default()
}

fn google(r: &Value) -> Option<OnlineCandidate> {
    let l = r.get("geometry")?.get("location")?;
    let (lon, lat) = place(l.get("lng").and_then(num), l.get("lat").and_then(num))?;
    let hoods: Vec<String> = [
        "neighborhood",
        "sublocality_level_1",
        "administrative_area_level_4",
    ]
    .iter()
    .map(|k| component(r, k))
    .filter(|t| !t.is_empty())
    .collect();
    let neighbourhood = hoods
        .iter()
        .find(|t| Locator::tagged_hood(t))
        .or(hoods.first())
        .cloned()
        .unwrap_or_default();
    let mut areas: Vec<String> = Vec::new();
    for k in [
        "neighborhood",
        "sublocality_level_2",
        "sublocality_level_1",
        "sublocality",
        "administrative_area_level_4",
        "administrative_area_level_3",
        "locality",
        "administrative_area_level_2",
        "administrative_area_level_1",
    ] {
        let t = component(r, k);
        if !t.is_empty() && !areas.contains(&t) {
            areas.push(t);
        }
    }
    let types = r.get("types").and_then(Value::as_array);
    Some(OnlineCandidate {
        lon,
        lat,
        label: s(r, "formatted_address"),
        street: component(r, "route"),
        door: component(r, "street_number"),
        areas,
        neighbourhood,
        district: component(r, "administrative_area_level_2"),
        province: component(r, "administrative_area_level_1"),
        postcode: component(r, "postal_code"),
        name: String::new(),
        kind: types
            .and_then(|t| t.first())
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned(),
        provider_score: None,
    })
}

/// Why an answer says it failed, in its words; none when it does not.
fn failure(kind: ProviderKind, v: &Value) -> Option<String> {
    match kind {
        ProviderKind::Arcgis => v.get("error").map(|e| {
            let msg = s(e, "message");
            format!(
                "ArcGIS cevap vermedi ({}): {}",
                s(e, "code"),
                if msg.is_empty() {
                    "nedeni yazılmamış".into()
                } else {
                    msg
                }
            )
        }),
        ProviderKind::Google => {
            let status = s(v, "status");
            (!status.is_empty() && status != "OK" && status != "ZERO_RESULTS").then(|| {
                let msg = s(v, "error_message");
                if msg.is_empty() {
                    format!("Google cevap vermedi: {status}.")
                } else {
                    format!("Google cevap vermedi ({status}): {msg}")
                }
            })
        }
        _ => v.get("error").map(|e| match e {
            Value::String(t) => format!("Sağlayıcı cevap vermedi: {t}"),
            other => format!("Sağlayıcı cevap vermedi: {}", s(other, "message")),
        }),
    }
}

fn parse_body(body: &[u8]) -> Result<Value, String> {
    serde_json::from_slice(body)
        .map_err(|e| format!("Sağlayıcının cevabı okunamadı (JSON değil): {e}."))
}

/// How one kind's answer item is read.
type ItemReader = fn(&Value) -> Option<OnlineCandidate>;

/// A search's answer.
pub fn forward(kind: ProviderKind, body: &[u8]) -> Result<Read, String> {
    let v = parse_body(body)?;
    if let Some(why) = failure(kind, &v) {
        return Err(why);
    }
    let (items, read): (Vec<&Value>, ItemReader) = match kind {
        ProviderKind::Nominatim => (
            v.as_array().map(|a| a.iter().collect()).unwrap_or_default(),
            nominatim,
        ),
        ProviderKind::Photon | ProviderKind::Pelias => (
            v.get("features")
                .and_then(Value::as_array)
                .map(|a| a.iter().collect())
                .unwrap_or_default(),
            if kind == ProviderKind::Photon {
                photon
            } else {
                pelias
            },
        ),
        ProviderKind::Arcgis => (
            v.get("candidates")
                .and_then(Value::as_array)
                .map(|a| a.iter().collect())
                .unwrap_or_default(),
            arcgis,
        ),
        ProviderKind::Google => (
            v.get("results")
                .and_then(Value::as_array)
                .map(|a| a.iter().collect())
                .unwrap_or_default(),
            google,
        ),
    };
    let mut out = Read::default();
    for item in items.into_iter().take(READ_LIMIT) {
        match read(item) {
            Some(c) => out.candidates.push(c),
            None => out.skipped += 1,
        }
    }
    Ok(out)
}

/// A place's answer: its first candidate, if any.
pub fn reverse(kind: ProviderKind, body: &[u8]) -> Result<Option<OnlineCandidate>, String> {
    let v = parse_body(body)?;
    if let Some(why) = failure(kind, &v) {
        // Nominatim says a place without an address as an error.
        if kind == ProviderKind::Nominatim && s(&v, "error") == "Unable to geocode" {
            return Ok(None);
        }
        return Err(why);
    }
    Ok(match kind {
        ProviderKind::Nominatim => nominatim(&v),
        ProviderKind::Arcgis => arcgis_reverse(&v),
        _ => forward(kind, &serde_json::to_vec(&v).unwrap_or_default())?
            .candidates
            .into_iter()
            .next(),
    })
}
