//! The ready basemaps (docs/adr/0208 §1): `fixtures/services/v1/presets.json`,
//! which both platforms read, each a service layer and, for one that asks
//! for proof, the connection it needs (its secrets are the user's).

use std::sync::OnceLock;

use kentos_contracts::{ServiceConnection, ServiceLayer};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    pub group: String,
    pub name: String,
    pub icon: String,
    pub service: ServiceLayer,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection: Option<ServiceConnection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribution_url: Option<String>,
    /// The most requests at once to its host (OpenStreetMap's 2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per_host: Option<u32>,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Catalog {
    pub groups: Vec<Group>,
    pub presets: Vec<Preset>,
}

/// The catalog, read once.
pub fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../fixtures/services/v1/presets.json"
        ))
        .unwrap_or(Catalog {
            groups: Vec::new(),
            presets: Vec::new(),
        })
    })
}

/// A preset by its id.
pub fn preset(id: &str) -> Option<&'static Preset> {
    catalog().presets.iter().find(|p| p.id == id)
}

/// The service layer a preset adds: its own, with `preset` naming it and
/// its connection's id when it has one.
pub fn layer_of(p: &Preset) -> ServiceLayer {
    let mut s = p.service.clone();
    s.preset = Some(p.id.clone());
    if let Some(c) = &p.connection {
        s.connection = Some(c.id.clone());
    }
    s
}

/// A preset's command in both apps: `basemap.` and its id in camel case
/// (`osm-standard` → `basemap.osmStandard`; the web's `basemapCommand`).
pub fn command_of(id: &str) -> String {
    let mut out = String::from("basemap.");
    let mut upper = false;
    for c in id.chars() {
        if c == '-' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// The preset a `basemap.` command shows.
pub fn by_command(command: &str) -> Option<&'static Preset> {
    catalog()
        .presets
        .iter()
        .find(|p| command_of(&p.id) == command)
}

/// The most requests at once a host takes: the preset's word, else 6.
pub fn per_host(service: &ServiceLayer) -> u32 {
    service
        .preset
        .as_deref()
        .and_then(preset)
        .and_then(|p| p.per_host)
        .unwrap_or(6)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_makes_a_layer_the_rules_take() {
        let c = catalog();
        assert!(c.presets.len() >= 16);
        for p in &c.presets {
            assert!(c.groups.iter().any(|g| g.id == p.group), "{}: group", p.id);
            let layer = layer_of(p);
            assert_eq!(layer.problem(), None, "{}", p.id);
            if let Some(conn) = &p.connection {
                assert_eq!(conn.problem(), None, "{}", p.id);
                assert!(
                    layer.url.is_empty() || crate::auth::same_origin(conn, &layer.url),
                    "{}: its address is the connection's origin",
                    p.id
                );
            }
        }
        assert_eq!(per_host(&layer_of(preset("osm-standard").unwrap())), 2);
        assert_eq!(command_of("osm-standard"), "basemap.osmStandard");
        assert_eq!(command_of("cyclosm"), "basemap.cyclosm");
        assert_eq!(
            by_command("basemap.googleSatellite").map(|p| p.id.as_str()),
            Some("google-satellite")
        );
        assert_eq!(per_host(&layer_of(preset("esri-imagery").unwrap())), 6);
    }
}
