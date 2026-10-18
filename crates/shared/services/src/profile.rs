//! What an OGC API Features service says it can do (docs/adr/0513): its conformance page's classes read into the
//! parts KentOS asks of it. A filter is sent as CQL2 text (`filter`, `filter-lang=cql2-text`), so it is sent only to a
//! service that declares Part 3's features filter and CQL2's text encoding; another service would ignore the unknown
//! parameters and give every object, which a take must not pass off as the filtered ones. The class's version is not
//! compared (1.0 and its drafts name the same class); Part 4 (create, replace, delete) is read but is a draft standard
//! and KentOS does not write through it.

use serde::Serialize;

/// The conformance classes KentOS reads, by family (the segment after `/spec/`) and class (after `/conf/`).
const CLASSES: [(&str, &str, Part); 13] = [
    ("ogcapi-features-1", "core", Part::Core),
    ("ogcapi-features-1", "geojson", Part::Geojson),
    ("ogcapi-features-2", "crs", Part::Crs),
    ("ogcapi-features-3", "filter", Part::Filter),
    ("ogcapi-features-3", "features-filter", Part::FeaturesFilter),
    ("ogcapi-features-3", "queryables", Part::Queryables),
    ("cql2", "cql2-text", Part::Cql2Text),
    ("cql2", "cql2-json", Part::Cql2Json),
    ("cql2", "basic-cql2", Part::BasicCql2),
    ("cql2", "basic-spatial-functions", Part::Spatial),
    ("cql2", "basic-spatial-operators", Part::Spatial),
    ("cql2", "temporal-functions", Part::Temporal),
    ("ogcapi-features-4", "create-replace-delete", Part::Write),
];

#[derive(Clone, Copy)]
enum Part {
    Core,
    Geojson,
    Crs,
    Filter,
    FeaturesFilter,
    Queryables,
    Cql2Text,
    Cql2Json,
    BasicCql2,
    Spatial,
    Temporal,
    Write,
}

/// A service's profile: what its conformance page declares, or that it could not be read.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedProfile {
    /// The conformance page was read; when not, nothing below is known.
    pub read: bool,
    /// Part 1: core and GeoJSON.
    pub core: bool,
    pub geojson: bool,
    /// Part 2: objects asked in a system other than WGS 84.
    pub crs: bool,
    /// Part 3: filtering, the features' filter, queryables.
    pub filter: bool,
    pub features_filter: bool,
    pub queryables: bool,
    /// CQL2: its text and JSON encodings, the basic language, spatial and temporal functions.
    pub cql2_text: bool,
    pub cql2_json: bool,
    pub basic_cql2: bool,
    pub spatial: bool,
    pub temporal: bool,
    /// Part 4 (create, replace, delete): declared; a draft standard, not used.
    pub write: bool,
    /// Every class the page listed, as written.
    pub classes: Vec<String>,
}

impl FeedProfile {
    /// The profile of a conformance page (`{"conformsTo": [...]}`); none when it is not one.
    pub fn of_page(text: &str) -> Option<FeedProfile> {
        let v: serde_json::Value = serde_json::from_str(text).ok()?;
        let list = v["conformsTo"].as_array()?;
        let classes: Vec<String> = list
            .iter()
            .filter_map(|c| c.as_str().map(str::to_owned))
            .collect();
        Some(FeedProfile::of(&classes))
    }

    /// The profile of the classes a page listed.
    pub fn of(classes: &[String]) -> FeedProfile {
        let mut p = FeedProfile {
            read: true,
            classes: classes.to_vec(),
            ..FeedProfile::default()
        };
        for uri in classes {
            let Some((family, class)) = split(uri) else {
                continue;
            };
            for (f, c, part) in CLASSES {
                if f == family && c == class {
                    *p.flag(part) = true;
                }
            }
        }
        p
    }

    fn flag(&mut self, part: Part) -> &mut bool {
        match part {
            Part::Core => &mut self.core,
            Part::Geojson => &mut self.geojson,
            Part::Crs => &mut self.crs,
            Part::Filter => &mut self.filter,
            Part::FeaturesFilter => &mut self.features_filter,
            Part::Queryables => &mut self.queryables,
            Part::Cql2Text => &mut self.cql2_text,
            Part::Cql2Json => &mut self.cql2_json,
            Part::BasicCql2 => &mut self.basic_cql2,
            Part::Spatial => &mut self.spatial,
            Part::Temporal => &mut self.temporal,
            Part::Write => &mut self.write,
        }
    }

    /// Whether a filter in CQL2 text may be sent: Part 3's features filter and CQL2's text encoding.
    pub fn filters(&self) -> bool {
        self.read && self.features_filter && self.cql2_text
    }

    /// Why a filter may not be sent, in the windows' words; none when it may.
    pub fn filter_problem(&self) -> Option<String> {
        if self.filters() {
            return None;
        }
        Some(if !self.read {
            "Servisin uygunluk sayfası (conformance) okunamadı; süzgeci desteklediği bilinmiyor. Süzgeç desteklenmezse \
             servis onu yok sayıp bütün nesneleri verir: süzgeci kaldırın ya da alanı kutuyla sınırlayın."
                .into()
        } else {
            let mut missing = Vec::new();
            if !self.features_filter {
                missing.push("OGC API Features Bölüm 3'ün nesne süzgeci");
            }
            if !self.cql2_text {
                missing.push("CQL2 metni");
            }
            format!(
                "Servis süzgeci desteklediğini bildirmiyor (eksik: {}). Süzgeç desteklenmezse servis onu yok sayıp bütün \
                 nesneleri verir: süzgeci kaldırın ya da alanı kutuyla sınırlayın.",
                missing.join(", ")
            )
        })
    }

    /// The profile in one line of the windows' words: the parts declared.
    pub fn words(&self) -> String {
        if !self.read {
            return "Uygunluk sayfası okunamadı".into();
        }
        let mut parts = Vec::new();
        if self.core {
            parts.push("Bölüm 1");
        }
        if self.crs {
            parts.push("Bölüm 2 (sistem)");
        }
        if self.features_filter {
            parts.push("Bölüm 3 (süzgeç)");
        }
        if self.cql2_text || self.cql2_json {
            parts.push(match (self.cql2_text, self.cql2_json) {
                (true, true) => "CQL2 metin ve JSON",
                (true, false) => "CQL2 metin",
                _ => "CQL2 JSON",
            });
        }
        if self.write {
            parts.push("Bölüm 4 taslağı (yazma; kullanılmaz)");
        }
        if parts.is_empty() {
            "Bildirilen bölüm yok".into()
        } else {
            parts.join(", ")
        }
    }
}

/// A class URI's family and class: `http://www.opengis.net/spec/<family>/<version>/conf/<class>`.
fn split(uri: &str) -> Option<(&str, &str)> {
    let rest = uri.trim().split_once("/spec/")?.1;
    let (family, rest) = rest.split_once('/')?;
    let class = rest.split_once("/conf/")?.1.trim_end_matches('/');
    Some((family, class))
}

/// The conformance page of an OGC API address: the landing page's `conformance` link when known, else beside the
/// `collections` path (an address of the collections or of one collection), else under the address itself.
pub fn conformance_url(url: &str, link: Option<&str>) -> String {
    if let Some(l) = link {
        return crate::query::with_params(l, &[("f", "json")]);
    }
    // The address's own query (a key a service asks in it) goes along; `f` is replaced.
    let query = url.find('?').map_or("", |i| &url[i..]);
    let base = crate::query::base(url).trim_end_matches('/');
    let root = match base.find("/collections") {
        Some(i) => &base[..i],
        None => base,
    };
    crate::query::with_params(&format!("{root}/conformance{query}"), &[("f", "json")])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_class_is_read_by_its_family_and_name_whatever_its_version() {
        let p = FeedProfile::of(&[
            "http://www.opengis.net/spec/ogcapi-features-1/1.0/conf/core".into(),
            "http://www.opengis.net/spec/ogcapi-features-3/1.0.0-draft.1/conf/features-filter"
                .into(),
            "http://www.opengis.net/spec/cql2/1.0/conf/cql2-text".into(),
        ]);
        assert!(p.core && p.features_filter && p.cql2_text && p.filters());
        assert!(!p.crs && !p.write);
        assert_eq!(p.words(), "Bölüm 1, Bölüm 3 (süzgeç), CQL2 metin");
    }

    #[test]
    fn the_conformance_page_is_beside_the_collections() {
        assert_eq!(
            conformance_url("https://x.org/ogc/collections/parsel?f=json", None),
            "https://x.org/ogc/conformance?f=json"
        );
        assert_eq!(
            conformance_url("https://x.org/ogc/", None),
            "https://x.org/ogc/conformance?f=json"
        );
        assert_eq!(
            conformance_url("https://x.org/ogc/collections?key=abc&f=json", None),
            "https://x.org/ogc/conformance?key=abc&f=json"
        );
        assert_eq!(
            conformance_url("https://x.org/ogc", Some("https://x.org/ogc/conf")),
            "https://x.org/ogc/conf?f=json"
        );
    }
}
