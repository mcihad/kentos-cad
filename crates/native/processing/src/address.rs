//! Adres arama's shared parts (docs/adr/0229 §10–§13): where addresses are
//! looked for and what a found address is, for İşlemler's tools and the
//! desktop's Adres tab alike. A source is the project's address data
//! (`locator:<id>`) or one of the device's online providers
//! (`provider:<id>`); the host gives the providers and sends their requests
//! ([`AddressHost`]: the key added, paced, cached), a host without one (a
//! test, the headless server) gives none and an online source is refused.
//! A found address is a [`Match`] in the project's system: the locator's
//! candidates and places as they are, an online provider's moved from WGS 84
//! with the project's datum choices ([`Plane`], docs/adr/0168).

use std::collections::BTreeMap;

use kentos_commands::AddressLocator;
use kentos_domain::Document;
use kentos_geometry_core::crs::{Choice, Datum, System, transform_in};
use kentos_geometry_core::ops::geocode::{Found, Parsed, Place, Status};
use kentos_geometry_core::vec2::Vec2;
use kentos_project::systems::{choices, own};
use kentos_services::geocode::Provider;
use kentos_services::geocode::read::OnlineCandidate;
use kentos_services::geocode::request::GeoRequest;
use kentos_services::geocode::score::{kind_word, score};

/// A source's id's prefixes.
pub const LOCATOR: &str = "locator:";
pub const PROVIDER: &str = "provider:";

/// The device's online providers, as a run asks them.
pub trait AddressHost: Send + Sync {
    /// The device's providers, the ready ones first (none usable until turned on).
    fn providers(&self) -> Vec<Provider>;
    /// Sends a provider's request and waits for its answer's body: the key
    /// added, the provider's pace kept, the device's cache asked first and
    /// written after. `stop` is asked while it waits.
    fn ask(
        &self,
        provider: &Provider,
        request: &GeoRequest,
        stop: &dyn Fn() -> bool,
    ) -> Result<Vec<u8>, String>;
}

/// A source offered in a list.
#[derive(Clone, Debug, PartialEq)]
pub struct SourceOption {
    pub id: String,
    pub name: String,
    /// Why it cannot be chosen (off, terms not accepted, no Google basemap); none when it can.
    pub why: Option<String>,
}

/// The project's address data as sources.
pub fn project_sources(doc: &Document) -> Vec<SourceOption> {
    doc.settings()
        .address_locators
        .iter()
        .map(|l| SourceOption {
            id: format!("{LOCATOR}{}", l.id),
            name: format!("Projenin adres verisi: {}", l.name),
            why: None,
        })
        .collect()
}

/// Online providers as sources, those that cannot be asked with why.
pub fn provider_sources(providers: &[Provider]) -> Vec<SourceOption> {
    providers
        .iter()
        .map(|p| SourceOption {
            id: format!("{PROVIDER}{}", p.id),
            name: p.name.clone(),
            why: if !p.usable() && p.accepted != Some(p.terms) {
                Some("koşullar onaylanmadı".into())
            } else if !p.enabled {
                Some("açılmadı".into())
            } else {
                None
            },
        })
        .collect()
}

/// A run's source.
pub enum Source<'d> {
    Local(&'d AddressLocator),
    Online(Box<Provider>),
}

/// The source of `id`, or why a run cannot use it.
pub fn resolve<'d>(
    id: &str,
    doc: &'d Document,
    host: Option<&dyn AddressHost>,
) -> Result<Source<'d>, String> {
    if let Some(l) = id.strip_prefix(LOCATOR) {
        return doc
            .settings()
            .address_locators
            .iter()
            .find(|o| o.id == l)
            .map(Source::Local)
            .ok_or_else(|| {
                format!("“{l}” adres verisi projede yok; Adres verisi penceresinden tanımlayın.")
            });
    }
    let Some(p) = id.strip_prefix(PROVIDER) else {
        return Err("Adres kaynağı seçin: projenin adres verisi ya da açık bir sağlayıcı.".into());
    };
    let Some(host) = host else {
        return Err(
            "Bu ortamda çevrimiçi adres sağlayıcısı yok; projenin adres verisini seçin.".into(),
        );
    };
    let provider = host
        .providers()
        .into_iter()
        .find(|o| o.id == p)
        .ok_or_else(|| format!("“{p}” sağlayıcısı bu cihazda yok."))?;
    if !provider.usable() {
        return Err(format!(
            "{} açık değil ya da koşulları onaylanmadı; Adres sağlayıcıları penceresinden açın.",
            provider.name
        ));
    }
    Ok(Source::Online(Box::new(provider)))
}

/// Why an online provider is not asked in a project without a system.
pub const NO_SYSTEM: &str = "Projenin koordinat sistemi yok; çevrimiçi sağlayıcılar WGS 84 konum verir. Proje ayarları'nda sistem seçin ya da projenin adres verisini kullanın.";

/// The project's system and its datum choices, for an online provider's places; none for a local project.
pub struct Plane {
    system: System,
    choices: Vec<Choice>,
}

const WGS84: System = System::Geographic {
    datum: Datum::Wgs84,
};

impl Plane {
    pub fn of(doc: &Document) -> Option<Plane> {
        let s = doc.settings();
        Some(Plane {
            system: own(s)?.system?,
            choices: choices(s),
        })
    }

    /// A WGS 84 place in the project's system.
    pub fn from_wgs84(&self, lon: f64, lat: f64) -> Option<Vec2> {
        transform_in(&WGS84, &self.system, Vec2::new(lon, lat), &self.choices)
            .ok()
            .map(|t| t.point)
    }

    /// A project's place in WGS 84 (longitude, latitude).
    pub fn to_wgs84(&self, p: Vec2) -> Option<(f64, f64)> {
        transform_in(&self.system, &WGS84, p, &self.choices)
            .ok()
            .map(|t| (t.point.x, t.point.y))
    }
}

/// A provider's own score as an attribute's text.
pub fn provider_score(s: Option<f64>) -> String {
    s.map(|x| {
        let t = format!("{x:.3}");
        t.trim_end_matches('0').trim_end_matches('.').to_owned()
    })
    .unwrap_or_default()
}

/// The fields a found address names beside its words, in this order (§11).
pub const PLACE_FIELDS: [&str; 6] = ["Mahalle", "Yol", "Kapı no", "İlçe", "İl", "Posta kodu"];

/// A found address: a row of Adres's table and what a point written at it takes.
#[derive(Clone, Debug, PartialEq)]
pub struct Match {
    /// The address as one line (§9).
    pub label: String,
    /// Kod, Kapı, Kapı (aradeğer), Yol, Mahalle; a provider's: Kapı, Yol, Yer adı, Bölge, Diğer; a place's: Kapı, Yol, Bölge.
    pub kind: String,
    /// Its score (an address looked for); none for a place's address.
    pub score: Option<u32>,
    /// How far a place's address is from the place, metres; none for a region (or an address looked for).
    pub distance: Option<f64>,
    /// Where it is, in the project's system.
    pub at: Vec2,
    /// [`PLACE_FIELDS`]' values.
    pub fields: [String; 6],
    /// The provider's own score, as text; empty for the project's data.
    pub provider_score: String,
}

impl Match {
    pub fn neighbourhood(&self) -> &str {
        &self.fields[0]
    }

    pub fn district(&self) -> &str {
        &self.fields[3]
    }
}

fn fields_of(t: &kentos_geometry_core::ops::geocode::locator::Texts) -> [String; 6] {
    [
        t.neighbourhood.clone(),
        t.street.clone(),
        t.door.clone(),
        t.district.clone(),
        t.province.clone(),
        t.postcode.clone(),
    ]
}

/// The project's address data's candidates, in their order.
pub fn local_matches(found: &Found) -> Vec<Match> {
    found
        .candidates
        .iter()
        .map(|c| Match {
            label: c.label.clone(),
            kind: c.kind.word().into(),
            score: Some(c.score),
            distance: None,
            at: c.at,
            fields: fields_of(&c.texts),
            provider_score: String::new(),
        })
        .collect()
}

/// A place's address in the project's data.
pub fn local_place(p: &Place) -> Match {
    Match {
        label: p.label.clone(),
        kind: p.kind.word().into(),
        score: None,
        distance: p.distance,
        at: p.at,
        fields: fields_of(&p.texts),
        provider_score: String::new(),
    }
}

fn online_fields(c: &OnlineCandidate) -> [String; 6] {
    [
        c.neighbourhood.clone(),
        c.street.clone(),
        c.door.clone(),
        c.district.clone(),
        c.province.clone(),
        c.postcode.clone(),
    ]
}

/// An online candidate scored by the locator's rules, at its place in the project's system; none when its place
/// does not move there.
pub fn online_match(q: &Parsed, c: &OnlineCandidate, plane: &Plane) -> Option<Match> {
    let (points, kind) = score(q, c);
    Some(Match {
        label: c.label.clone(),
        kind: kind_word(kind).into(),
        score: Some(points),
        distance: None,
        at: plane.from_wgs84(c.lon, c.lat)?,
        fields: online_fields(c),
        provider_score: provider_score(c.provider_score),
    })
}

/// A provider's candidates scored, the best first (the provider's order among equal scores).
pub fn online_matches(q: &Parsed, candidates: &[OnlineCandidate], plane: &Plane) -> Vec<Match> {
    let mut out: Vec<Match> = candidates
        .iter()
        .filter_map(|c| online_match(q, c, plane))
        .collect();
    out.sort_by_key(|m| std::cmp::Reverse(m.score));
    out
}

/// A provider's address of a place: its kind the provider's word (else Adres), its distance from the place.
pub fn online_place(
    c: &OnlineCandidate,
    plane: &Plane,
    from: Vec2,
    per_metre: f64,
) -> Option<Match> {
    let at = plane.from_wgs84(c.lon, c.lat)?;
    Some(Match {
        label: c.label.clone(),
        kind: if c.kind.is_empty() {
            "Adres".into()
        } else {
            c.kind.clone()
        },
        score: None,
        distance: Some(
            kentos_geometry_core::jsmath::js_hypot(at.x - from.x, at.y - from.y) / per_metre,
        ),
        at,
        fields: online_fields(c),
        provider_score: provider_score(c.provider_score),
    })
}

/// What a point written at a found address says of it (§11).
pub struct Matched<'a> {
    pub status: Status,
    pub found: &'a Match,
    /// The source's name: the address data's or the provider's.
    pub source: &'a str,
    /// The provider's attribution; empty for the project's data.
    pub attribution: &'a str,
    /// The address as it was given.
    pub input: &'a str,
}

/// A found address's attributes over `base` (a row's cells, a point's own): Durum, Puan, Eşleşme türü, Eşleşen
/// adres, the fields that are not empty, Kaynak, Girdi adresi; a provider's Atıf and Sağlayıcı puanı (§11).
pub fn match_attributes(
    base: &BTreeMap<String, String>,
    m: &Matched<'_>,
) -> BTreeMap<String, String> {
    let mut attrs = base.clone();
    attrs.insert("Durum".into(), m.status.word().into());
    if let Some(s) = m.found.score {
        attrs.insert("Puan".into(), s.to_string());
    }
    attrs.insert("Eşleşme türü".into(), m.found.kind.clone());
    attrs.insert("Eşleşen adres".into(), m.found.label.clone());
    for (name, value) in PLACE_FIELDS.iter().zip(&m.found.fields) {
        if !value.is_empty() {
            attrs.insert((*name).into(), value.clone());
        }
    }
    attrs.insert("Kaynak".into(), m.source.to_owned());
    attrs.insert("Girdi adresi".into(), m.input.to_owned());
    if !m.attribution.is_empty() {
        attrs.insert("Atıf".into(), m.attribution.to_owned());
        if !m.found.provider_score.is_empty() {
            attrs.insert("Sağlayıcı puanı".into(), m.found.provider_score.clone());
        }
    }
    attrs
}
