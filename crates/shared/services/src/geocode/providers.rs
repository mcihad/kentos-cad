//! The providers (docs/adr/0229 §10, §16): the ready ones of
//! `fixtures/geocode/v1/providers.json` and the kinds of a server of one's
//! own, a device's settings of them (`adres-saglayicilari.json`, format
//! `kentos.address-providers` 1: which are on, the terms accepted, the
//! country, ArcGIS's storing, a server's address and pace), and what a
//! provider is with them ([`Provider`]): a provider is usable only on, with
//! the terms of its card accepted.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

pub const FORMAT: &str = "kentos.address-providers";
pub const VERSION: u32 = 1;

/// The most servers of one's own a device keeps.
pub const SERVER_LIMIT: usize = 16;

/// What answers a provider's requests.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderKind {
    Nominatim,
    Photon,
    Pelias,
    Arcgis,
    Google,
}

/// Whether a provider's results may be written into a drawing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StoreRule {
    Allowed,
    /// Only with ArcGIS's “Sonuçları sakla” (`forStorage=true`).
    WithOption,
    Never,
}

/// Whether a provider asks a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KeyRule {
    None,
    Optional,
    Required,
}

/// A ready provider, as the table writes it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    pub kind: ProviderKind,
    pub name: String,
    pub url: String,
    pub key: KeyRule,
    #[serde(default)]
    pub key_param: Option<String>,
    pub rate: f64,
    pub parallel: u32,
    /// The most unique addresses a batch sends; none: no limit; 0: no batch.
    pub batch: Option<u32>,
    pub store: StoreRule,
    pub cache_days: u32,
    pub terms: u32,
    pub attribution: String,
    pub terms_url: String,
    pub privacy_url: String,
    /// Shown only beside a visible Google basemap (Google's terms 6.2).
    #[serde(default)]
    pub google_map: bool,
    pub lines: Vec<String>,
}

/// A kind a server of one's own may be.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerKind {
    pub kind: ProviderKind,
    pub name: String,
    pub key: KeyRule,
    #[serde(default)]
    pub key_param: Option<String>,
}

/// What a server of one's own is unless set.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Servers {
    pub rate: f64,
    pub parallel: u32,
    pub rates: (f64, f64),
    pub parallels: (u32, u32),
    pub store: StoreRule,
    pub cache_days: u32,
    pub terms: u32,
    pub kinds: Vec<ServerKind>,
    /// The card's lines; `{host}` is the server's.
    pub lines: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Table {
    pub presets: Vec<Preset>,
    pub servers: Servers,
}

/// The table, read once.
pub fn table() -> &'static Table {
    static TABLE: OnceLock<Table> = OnceLock::new();
    TABLE.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../../fixtures/geocode/v1/providers.json"
        ))
        .unwrap_or(Table {
            presets: Vec::new(),
            servers: Servers {
                rate: 1.0,
                parallel: 1,
                rates: (0.1, 50.0),
                parallels: (1, 8),
                store: StoreRule::Allowed,
                cache_days: 90,
                terms: 1,
                kinds: Vec::new(),
                lines: Vec::new(),
            },
        })
    })
}

/// A device's setting of a provider (a ready one by its id, or a server of one's own).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSetting {
    pub id: String,
    /// A server's kind, name and address; none for a ready one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<ProviderKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default)]
    pub enabled: bool,
    /// The card's version accepted, and when (RFC 3339).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_at: Option<String>,
    /// ISO 3166-1 alpha-2; none: no country asked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// ArcGIS's “Sonuçları sakla”.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub store: bool,
    /// A server's requests a second and at once.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parallel: Option<u32>,
}

/// The device's file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Providers {
    pub format: String,
    pub version: u32,
    #[serde(default)]
    pub providers: Vec<ProviderSetting>,
}

impl Default for Providers {
    fn default() -> Self {
        Providers {
            format: FORMAT.into(),
            version: VERSION,
            providers: Vec::new(),
        }
    }
}

fn http_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
        .is_some_and(|rest| !rest.is_empty() && !rest.starts_with('/'))
        && url.len() <= 4096
        && !url.chars().any(char::is_control)
}

impl ProviderSetting {
    /// What is wrong with the setting, in the window's words; none when it holds.
    pub fn problem(&self) -> Option<String> {
        let t = table();
        let preset = t.presets.iter().find(|p| p.id == self.id);
        match preset {
            Some(_) => {
                if self.kind.is_some() || self.name.is_some() || self.url.is_some() {
                    return Some(format!(
                        "“{}” hazır bir sağlayıcıdır; türü, adı ve adresi değiştirilmez.",
                        self.id
                    ));
                }
                if self.rate.is_some() || self.parallel.is_some() {
                    return Some(format!("“{}” sağlayıcısının hızı değiştirilmez.", self.id));
                }
            }
            None => {
                let ok_id = self.id.starts_with("sunucu-")
                    && self.id.len() <= 40
                    && self
                        .id
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
                if !ok_id {
                    return Some(format!(
                        "Sağlayıcının kimliği “{}”; hazırlardan biri ya da sunucu-N olmalı.",
                        self.id
                    ));
                }
                let Some(kind) = self.kind else {
                    return Some("Sunucunun türü yok.".into());
                };
                if !t.servers.kinds.iter().any(|k| k.kind == kind) {
                    return Some(
                        "Google sunucusu eklenmez; hazır Google sağlayıcısını kullanın.".into(),
                    );
                }
                let name = self.name.as_deref().unwrap_or("");
                if name.trim().is_empty() || name.trim() != name || name.chars().count() > 80 {
                    return Some(
                        "Sunucunun adı boş, kırpılmamış ya da 80 karakterden uzun.".into(),
                    );
                }
                if !self.url.as_deref().is_some_and(http_url) {
                    return Some(format!(
                        "“{name}” sunucusunun adresi http ya da https ile başlamalı."
                    ));
                }
                if let Some(r) = self.rate
                    && !(r.is_finite() && r >= t.servers.rates.0 && r <= t.servers.rates.1)
                {
                    return Some(format!(
                        "“{name}” sunucusunun hızı saniyede {} ile {} istek arasında olmalı.",
                        t.servers.rates.0, t.servers.rates.1
                    ));
                }
                if let Some(p) = self.parallel
                    && !(t.servers.parallels.0..=t.servers.parallels.1).contains(&p)
                {
                    return Some(format!(
                        "“{name}” sunucusuna aynı anda {} ile {} istek gider.",
                        t.servers.parallels.0, t.servers.parallels.1
                    ));
                }
                if self.store {
                    return Some("“Sonuçları sakla” yalnız ArcGIS World'ün ayarıdır.".into());
                }
            }
        }
        if let Some(c) = &self.country
            && !(c.len() == 2 && c.bytes().all(|b| b.is_ascii_uppercase()))
        {
            return Some(format!("Ülke kodu “{c}”; iki büyük harf olmalı (TR)."));
        }
        if self.store && preset.is_some_and(|p| p.store != StoreRule::WithOption) {
            return Some("“Sonuçları sakla” yalnız ArcGIS World'ün ayarıdır.".into());
        }
        if self.enabled && self.accepted.is_none() {
            return Some("Koşulları onaylanmamış sağlayıcı açılamaz.".into());
        }
        None
    }
}

impl Providers {
    /// The file as a device keeps it: its settings that hold, each id once, at most 16 servers.
    pub fn sanitized(self) -> Providers {
        let mut kept: Vec<ProviderSetting> = Vec::new();
        let mut servers = 0;
        for p in self.providers {
            if p.problem().is_some() || kept.iter().any(|o| o.id == p.id) {
                continue;
            }
            if p.kind.is_some() {
                if servers == SERVER_LIMIT {
                    continue;
                }
                servers += 1;
            }
            kept.push(p);
        }
        Providers {
            format: FORMAT.into(),
            version: VERSION,
            providers: kept,
        }
    }

    /// The id a new server takes: the first free `sunucu-N`.
    pub fn next_server_id(&self) -> String {
        (1..)
            .map(|n| format!("sunucu-{n}"))
            .find(|id| !self.providers.iter().any(|p| p.id == *id))
            .unwrap_or_else(|| "sunucu".into())
    }

    /// Every provider the device has: the ready ones in the table's order, then its servers in its order.
    pub fn resolved(&self) -> Vec<Provider> {
        let t = table();
        let own = |id: &str| self.providers.iter().find(|p| p.id == id);
        let mut out: Vec<Provider> = t
            .presets
            .iter()
            .map(|p| Provider::of_preset(p, own(&p.id)))
            .collect();
        out.extend(
            self.providers
                .iter()
                .filter(|s| s.kind.is_some())
                .filter_map(Provider::of_server),
        );
        out
    }
}

/// A provider as its requests and its card take it.
#[derive(Clone, Debug, PartialEq)]
pub struct Provider {
    pub id: String,
    pub kind: ProviderKind,
    pub name: String,
    pub url: String,
    pub key: KeyRule,
    pub key_param: Option<String>,
    pub rate: f64,
    pub parallel: u32,
    pub batch: Option<u32>,
    pub store: StoreRule,
    pub cache_days: u32,
    /// The card's version the user must accept, and the version accepted.
    pub terms: u32,
    pub accepted: Option<u32>,
    pub enabled: bool,
    pub country: Option<String>,
    /// ArcGIS's “Sonuçları sakla”.
    pub storing: bool,
    pub attribution: String,
    pub terms_url: String,
    pub privacy_url: String,
    pub google_map: bool,
    pub lines: Vec<String>,
    pub server: bool,
}

impl Provider {
    fn of_preset(p: &Preset, own: Option<&ProviderSetting>) -> Provider {
        Provider {
            id: p.id.clone(),
            kind: p.kind,
            name: p.name.clone(),
            url: p.url.clone(),
            key: p.key,
            key_param: p.key_param.clone(),
            rate: p.rate,
            parallel: p.parallel,
            batch: p.batch,
            store: p.store,
            cache_days: p.cache_days,
            terms: p.terms,
            accepted: own.and_then(|o| o.accepted),
            enabled: own.is_some_and(|o| o.enabled),
            country: own.map_or(Some("TR".to_owned()), |o| o.country.clone()),
            storing: own.is_some_and(|o| o.store),
            attribution: p.attribution.clone(),
            terms_url: p.terms_url.clone(),
            privacy_url: p.privacy_url.clone(),
            google_map: p.google_map,
            lines: p.lines.clone(),
            server: false,
        }
    }

    fn of_server(s: &ProviderSetting) -> Option<Provider> {
        let t = table();
        let kind = s.kind?;
        let k = t.servers.kinds.iter().find(|k| k.kind == kind)?;
        let url = s.url.clone().unwrap_or_default();
        let host = host_of(&url);
        Some(Provider {
            id: s.id.clone(),
            kind,
            name: s.name.clone().unwrap_or_default(),
            url: url.trim_end_matches('/').to_owned(),
            key: k.key,
            key_param: k.key_param.clone(),
            rate: s.rate.unwrap_or(t.servers.rate),
            parallel: s.parallel.unwrap_or(t.servers.parallel),
            batch: None,
            store: t.servers.store,
            cache_days: t.servers.cache_days,
            terms: t.servers.terms,
            accepted: s.accepted,
            enabled: s.enabled,
            country: s.country.clone(),
            storing: false,
            attribution: host.clone(),
            terms_url: String::new(),
            privacy_url: String::new(),
            google_map: false,
            lines: t
                .servers
                .lines
                .iter()
                .map(|l| l.replace("{host}", &host))
                .collect(),
            server: true,
        })
    }

    /// Whether it may be asked: on, its current card accepted.
    pub fn usable(&self) -> bool {
        self.enabled && self.accepted == Some(self.terms)
    }

    /// Whether its results may be written into a drawing.
    pub fn may_store(&self) -> bool {
        match self.store {
            StoreRule::Allowed => true,
            StoreRule::WithOption => self.storing,
            StoreRule::Never => false,
        }
    }

    /// Whether its answers may be kept on the device, and how long (days).
    pub fn cache_days(&self) -> u32 {
        if self.may_store() { self.cache_days } else { 0 }
    }

    /// Why a batch of `unique` addresses may not be sent to it; none when it may.
    pub fn batch_refusal(&self, unique: usize) -> Option<String> {
        if !self.may_store() {
            return Some(match self.store {
                StoreRule::WithOption => format!(
                    "{}'in sonuçları saklanmak için forStorage=true ister; Adres sağlayıcıları'nda “Sonuçları sakla”yı açın.",
                    self.name
                ),
                _ => format!(
                    "{} sonuçların saklanmasına izin vermez; toplu iş başka bir kaynakla yapılır.",
                    self.name
                ),
            });
        }
        match self.batch {
            Some(0) => Some(format!("{} toplu işte kullanılmaz.", self.name)),
            Some(max) if unique > max as usize => Some(format!(
                "{} adres var; {} bir çalıştırmada en çok {max} adres alır (kullanım koşulları). Adresleri bölün ya da kurumun sunucusunu kullanın.",
                unique, self.name
            )),
            _ => None,
        }
    }

    /// The least time between two requests' starts, milliseconds.
    pub fn interval_ms(&self) -> u64 {
        if self.rate > 0.0 {
            (1000.0 / self.rate).ceil() as u64
        } else {
            1000
        }
    }

    /// Its server's host, as the card and the messages name it.
    pub fn host(&self) -> String {
        host_of(&self.url)
    }
}

/// An address's host (its authority without a user and a port), lower case.
pub fn host_of(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let authority = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    authority
        .split(':')
        .next()
        .unwrap_or(authority)
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_reads_and_a_preset_is_off_until_accepted() {
        let t = table();
        assert_eq!(t.presets.len(), 4);
        let mut file = Providers::default();
        let off = file.resolved();
        assert!(off.iter().all(|p| !p.usable()));
        file.providers.push(ProviderSetting {
            id: "nominatim-osm".into(),
            enabled: true,
            accepted: Some(1),
            accepted_at: Some("2026-10-12T18:00:00Z".into()),
            country: Some("TR".into()),
            ..ProviderSetting::default()
        });
        let on = file.resolved();
        assert!(on[0].usable());
        assert_eq!(on[0].interval_ms(), 1000);
        assert!(on[0].batch_refusal(1001).is_some());
        assert!(on[0].batch_refusal(1000).is_none());
        let google = on.iter().find(|p| p.id == "google").expect("google");
        assert!(!google.may_store() && google.batch_refusal(1).is_some());
    }

    #[test]
    fn a_server_holds_and_its_card_names_its_host() {
        let s = ProviderSetting {
            id: "sunucu-1".into(),
            kind: Some(ProviderKind::Nominatim),
            name: Some("Belediye".into()),
            url: Some("https://adres.ornek.bel.tr/".into()),
            enabled: true,
            accepted: Some(1),
            rate: Some(5.0),
            ..ProviderSetting::default()
        };
        assert_eq!(s.problem(), None);
        let file = Providers {
            providers: vec![s.clone()],
            ..Providers::default()
        };
        let p = file.resolved().pop().expect("the server");
        assert!(p.server && p.usable() && p.may_store());
        assert_eq!(p.url, "https://adres.ornek.bel.tr");
        assert!(p.lines[1].contains("adres.ornek.bel.tr"));
        let mut bad = s;
        bad.kind = Some(ProviderKind::Google);
        assert!(bad.problem().is_some());
    }
}
