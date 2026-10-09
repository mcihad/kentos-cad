//! Servisten veri al (docs/adr/0208 §10), the same in both apps: Bağlan
//! reads what a data service has (a WFS's feature types, an OGC API's
//! collections, an ArcGIS service's layers, or a GeoJSON address as one),
//! the choice makes the feed the layer remembers, and taking asks for its
//! objects page by page (WFS's `COUNT` and `STARTINDEX`, OGC API's `next`
//! link, ArcGIS's `resultOffset`), each page read into the contract's
//! objects in the system they were asked in. The host moves them into the
//! project's system, writes them, and takes them again with Yenile.

use kentos_contracts::{FeatureFeed, FeedKind, ImportResult, ReportItem};
use serde::{Deserialize, Serialize};

use crate::caps::arcgis::{self, ArcgisService};
use crate::caps::ogc::{self, Collection};
use crate::caps::wfs::{self, WfsCapabilities};
use crate::features;
use crate::query::{join, with_params};
use crate::request::{self, Request};

/// A new data layer's colours, by how many layers the drawing has: its line
/// and a light fill of it (the theme's `fg` would vanish on a light or a
/// dark basemap).
pub fn layer_colors(layers: usize) -> (&'static str, &'static str) {
    const PALETTE: [(&str, &str); 6] = [
        ("#E5484D", "#E5484D26"),
        ("#3E63DD", "#3E63DD26"),
        ("#30A46C", "#30A46C26"),
        ("#F76B15", "#F76B1526"),
        ("#8E4EC6", "#8E4EC626"),
        ("#12A594", "#12A59426"),
    ];
    PALETTE[layers % PALETTE.len()]
}

/// What a take brought, as both apps say it once the objects are written:
/// how many, what the service said it matched, what the readers left out
/// (a feature whose geometry came null is told apart: a service that cannot
/// show an object in the system asked gives it so), what found no place in
/// the project's system, and whether the most objects stopped it. `srid`
/// is the system the objects were asked in.
pub fn taken_words(
    taken: usize,
    matched: Option<u64>,
    skipped: &[ReportItem],
    dropped: usize,
    capped: bool,
    srid: u32,
) -> String {
    let null = kentos_formats::geojson::NULL_GEOMETRY;
    let mut s = format!("{taken} nesne alındı.");
    if let Some(m) = matched.filter(|m| *m > taken as u64) {
        s.push_str(&format!(" Servis {m} nesne eşleştiğini söyledi."));
    }
    let empty: u32 = skipped
        .iter()
        .filter(|i| i.what == null)
        .map(|i| i.count)
        .sum();
    if empty > 0 {
        s.push_str(&format!(
            " {empty} nesne servisten geometrisi boş geldi ve alınmadı."
        ));
        if srid != 4326 {
            s.push_str(&format!(
                " Geometrisi boş gelen nesne, servisin istenen sistemde (EPSG:{srid}) gösteremediği nesne olabilir: İstenen sistem'i WGS 84 (EPSG:4326) yapıp yeniden alabilirsiniz."
            ));
        }
    }
    for i in skipped.iter().filter(|i| i.what != null) {
        s.push_str(&format!(
            " {} nesne alınmadı: {} ({}).",
            i.count, i.what, i.reason
        ));
    }
    if dropped > 0 {
        s.push_str(&format!(
            " {dropped} nesne projenin sisteminde yer bulamadığı için alınmadı."
        ));
    }
    if capped {
        s.push_str(" En çok nesne sınırına ulaşıldı; servisin başka nesneleri de olabilir.");
    }
    s
}

/// The most objects a take may bring.
pub const MOST: u32 = 500_000;
/// The default for “En çok nesne”.
pub const DEFAULT_MOST: u32 = 50_000;
/// A page's size, when the service does not say less.
const PAGE: u64 = 2_000;

/// One kind of objects a data service offers.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedItem {
    /// What the request names it by (a WFS type's name, a collection's id, an ArcGIS layer's id).
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// The systems its objects may be asked in.
    pub srids: Vec<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wgs84: Option<[f64; 4]>,
    /// ArcGIS's geometry type, when said.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedOffer {
    pub kind: FeedKind,
    pub title: String,
    pub items: Vec<FeedItem>,
}

/// What the window chose.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FeedChoice {
    pub item: String,
    /// The system asked: absent, the project's when offered, else WGS 84.
    pub srid: Option<u32>,
    /// A CQL filter (WFS, OGC API) or an ArcGIS `where`.
    pub filter: Option<String>,
    /// The area, in the project's system (the feed keeps it for Yenile).
    pub bbox: Option<[f64; 4]>,
    pub limit: Option<u64>,
    /// The attribute that matches objects on Yenile.
    pub key: Option<String>,
    pub connection: Option<String>,
}

#[derive(Clone, Debug)]
enum Stage {
    Wfs,
    /// An OGC API page: a landing page (its data link followed) or its collections.
    Ogc,
    /// An ArcGIS service's description, or a layer's.
    Arcgis,
    Done,
}

/// Bağlan for one data service.
#[derive(Clone, Debug)]
pub struct FeedConnecting {
    kind: FeedKind,
    url: String,
    stage: Stage,
    wfs: Option<WfsCapabilities>,
    collections: Vec<Collection>,
    arcgis: Option<ArcgisService>,
    /// An ArcGIS layer's own address read: its description.
    arcgis_layer: Option<arcgis::ArcgisLayerInfo>,
    asked: Option<String>,
    offer: Option<FeedOffer>,
}

impl FeedConnecting {
    /// Bağlan: the first request, or none for a GeoJSON address (its offer is ready).
    pub fn start(kind: FeedKind, url: &str) -> Result<(FeedConnecting, Option<Request>), String> {
        let url = url.trim().to_owned();
        if let Some(p) = kentos_contracts::service::http_problem("Verinin adresi", &url) {
            return Err(p);
        }
        let mut c = FeedConnecting {
            kind,
            url: url.clone(),
            stage: Stage::Done,
            wfs: None,
            collections: Vec::new(),
            arcgis: None,
            arcgis_layer: None,
            asked: None,
            offer: None,
        };
        let first = match kind {
            FeedKind::Geojson => {
                c.offer = Some(FeedOffer {
                    kind,
                    title: kentos_contracts::origin_of(&url).unwrap_or_default(),
                    items: vec![FeedItem {
                        id: String::new(),
                        title: "GeoJSON".into(),
                        summary: None,
                        srids: vec![4326],
                        wgs84: None,
                        geometry: None,
                    }],
                });
                return Ok((c, None));
            }
            FeedKind::Wfs => {
                c.stage = Stage::Wfs;
                if url.to_ascii_lowercase().contains("request=getcapabilities") {
                    url
                } else {
                    request::wfs::capabilities(&url)
                }
            }
            FeedKind::OgcFeatures => {
                c.stage = Stage::Ogc;
                with_params(&url, &[("f", "json")])
            }
            FeedKind::Arcgis => {
                c.stage = Stage::Arcgis;
                request::arcgis::info(&url)
            }
        };
        c.asked = Some(first.clone());
        Ok((c, Some(Request::get(first))))
    }

    /// The answer to the last request: the next request, or none when the offer is ready.
    pub fn answer(&mut self, body: &str) -> Result<Option<Request>, String> {
        let asked = self.asked.take().unwrap_or_else(|| self.url.clone());
        match self.stage {
            Stage::Wfs => {
                let caps = wfs::read(body)?;
                if caps.types.is_empty() {
                    return Err("WFS servisi hiç nesne türü sunmuyor.".into());
                }
                self.offer = Some(FeedOffer {
                    kind: self.kind,
                    title: caps.title.clone(),
                    items: caps
                        .types
                        .iter()
                        .map(|t| FeedItem {
                            id: t.name.clone(),
                            title: if t.title.is_empty() {
                                t.name.clone()
                            } else {
                                t.title.clone()
                            },
                            summary: t.summary.clone(),
                            srids: t.srids.clone(),
                            wgs84: t.wgs84,
                            geometry: None,
                        })
                        .collect(),
                });
                self.wfs = Some(caps);
                self.stage = Stage::Done;
                Ok(None)
            }
            Stage::Ogc => {
                let v: serde_json::Value = serde_json::from_str(body).map_err(|_| {
                    "Yanıt JSON değil: adres bir OGC API Features sayfası olmalı.".to_owned()
                })?;
                if v["collections"].is_array() {
                    self.collections = ogc::collections(body, &asked)?
                        .into_iter()
                        .filter(|c| c.items.is_some())
                        .collect();
                    if self.collections.is_empty() {
                        return Err("Koleksiyonların hiçbirinin nesnesi (items) yok.".into());
                    }
                    self.offer = Some(FeedOffer {
                        kind: self.kind,
                        title: kentos_contracts::origin_of(&self.url).unwrap_or_default(),
                        items: self
                            .collections
                            .iter()
                            .map(|c| FeedItem {
                                id: c.id.clone(),
                                title: c.title.clone(),
                                summary: c.description.clone(),
                                srids: if c.srids.is_empty() {
                                    vec![4326]
                                } else {
                                    c.srids.clone()
                                },
                                wgs84: c.wgs84,
                                geometry: None,
                            })
                            .collect(),
                    });
                    self.stage = Stage::Done;
                    return Ok(None);
                }
                // One collection's own page: its items.
                if v["id"].is_string() && v["links"].is_array() && !v["collections"].is_array() {
                    let wrapped = format!(r#"{{"collections":[{body}]}}"#);
                    if let Ok(list) = ogc::collections(&wrapped, &asked)
                        && list.iter().any(|c| c.items.is_some())
                    {
                        self.collections = list;
                        self.offer = Some(FeedOffer {
                            kind: self.kind,
                            title: self.collections[0].title.clone(),
                            items: self
                                .collections
                                .iter()
                                .map(|c| FeedItem {
                                    id: c.id.clone(),
                                    title: c.title.clone(),
                                    summary: c.description.clone(),
                                    srids: if c.srids.is_empty() {
                                        vec![4326]
                                    } else {
                                        c.srids.clone()
                                    },
                                    wgs84: c.wgs84,
                                    geometry: None,
                                })
                                .collect(),
                        });
                        self.stage = Stage::Done;
                        return Ok(None);
                    }
                }
                let page = ogc::landing(body, &asked)?;
                let next = ogc::link(&page.links, &["data"])
                    .map(|l| l.href.clone())
                    .unwrap_or_else(|| join(&self.url, "collections"));
                let url = with_params(&next, &[("f", "json")]);
                self.asked = Some(url.clone());
                Ok(Some(Request::get(url)))
            }
            Stage::Arcgis => {
                let v: serde_json::Value = serde_json::from_str(body).map_err(|_| {
                    "Yanıt JSON değil: adres bir ArcGIS REST servisi olmalı.".to_owned()
                })?;
                if v["type"].as_str().is_some_and(|t| t.contains("Layer"))
                    || v["geometryType"].is_string()
                {
                    let info = arcgis::layer(body)?;
                    let id = self
                        .url
                        .trim_end_matches('/')
                        .rsplit('/')
                        .next()
                        .unwrap_or("")
                        .to_owned();
                    self.offer = Some(FeedOffer {
                        kind: self.kind,
                        title: info.name.clone(),
                        items: vec![FeedItem {
                            id,
                            title: info.name.clone(),
                            summary: info.geometry.clone(),
                            srids: Vec::new(),
                            wgs84: None,
                            geometry: info.geometry.clone(),
                        }],
                    });
                    self.arcgis_layer = Some(info);
                } else {
                    let s = arcgis::service(body)?;
                    let items: Vec<FeedItem> = s
                        .layers
                        .iter()
                        .filter(|l| l.geometry.is_some())
                        .map(|l| FeedItem {
                            id: l.id.to_string(),
                            title: l.name.clone(),
                            summary: l.geometry.clone(),
                            srids: s.srid.into_iter().collect(),
                            wgs84: None,
                            geometry: l.geometry.clone(),
                        })
                        .collect();
                    if items.is_empty() {
                        return Err("Servisin nesne katmanı yok.".into());
                    }
                    self.offer = Some(FeedOffer {
                        kind: self.kind,
                        title: s.title.clone(),
                        items,
                    });
                    self.arcgis = Some(s);
                }
                self.stage = Stage::Done;
                Ok(None)
            }
            Stage::Done => Ok(None),
        }
    }

    pub fn offer(&self) -> Option<&FeedOffer> {
        self.offer.as_ref()
    }

    /// Whether a WFS gives its objects as GeoJSON (else they come as GML).
    pub fn geojson(&self) -> bool {
        self.wfs
            .as_ref()
            .and_then(|c| wfs::output_format(&c.formats))
            .is_some_and(|f| f.to_ascii_lowercase().contains("json"))
    }

    fn item(&self, id: &str) -> Option<&FeedItem> {
        self.offer.as_ref()?.items.iter().find(|i| i.id == id)
    }

    /// Whether item `item`'s objects may be asked in `srid`: a system it
    /// offers; an ArcGIS layer (the server projects) also in the project's
    /// and in WGS 84.
    fn listed(&self, item: &FeedItem, srid: u32, project: u32) -> bool {
        srid != 0
            && (item.srids.contains(&srid)
                || (self.kind == FeedKind::Arcgis && (srid == project || srid == 4326)))
    }

    /// The system the objects of item `id` are asked in for the window's
    /// `choice`: the choice when listed, else the project's when listed,
    /// else WGS 84; none for a GeoJSON address (always WGS 84) or an unknown item.
    pub fn asked_srid(&self, id: &str, choice: Option<u32>, project: u32) -> Option<u32> {
        if self.kind == FeedKind::Geojson {
            return None;
        }
        let item = self.item(id)?;
        Some(
            choice
                .filter(|s| self.listed(item, *s, project))
                .or_else(|| self.listed(item, project, project).then_some(project))
                .unwrap_or(4326),
        )
    }

    /// The systems the window lists for item `id`, in order: for an ArcGIS
    /// layer the project's first, then the item's own, then WGS 84; for
    /// another kind the item's own; the one asked when none is chosen is
    /// always among them, so the list shows what will be asked.
    pub fn systems(&self, id: &str, project: u32) -> Vec<u32> {
        let Some(item) = self.item(id) else {
            return Vec::new();
        };
        let mut out: Vec<u32> = Vec::new();
        let mut put = |s: u32| {
            if s != 0 && !out.contains(&s) {
                out.push(s);
            }
        };
        if self.kind == FeedKind::Arcgis {
            put(project);
        }
        for s in &item.srids {
            put(*s);
        }
        if self.kind == FeedKind::Arcgis {
            put(4326);
        }
        if let Some(s) = self.asked_srid(id, None, project) {
            put(s);
        }
        out
    }

    /// The feed the choice makes, or why it makes none. `project` is the project's system.
    pub fn feed(&self, choice: &FeedChoice, project: u32) -> Result<FeatureFeed, String> {
        let offer = self
            .offer
            .as_ref()
            .ok_or("Önce Bağlan ile servisi okuyun.")?;
        let item = offer
            .items
            .iter()
            .find(|i| i.id == choice.item)
            .ok_or("Listeden bir tür seçin.")?;
        let srid = self.asked_srid(&item.id, choice.srid, project);
        let some = |t: &Option<String>| t.clone().filter(|v| !v.trim().is_empty());
        let (url, name, version) = match self.kind {
            FeedKind::Wfs => {
                let caps = self.wfs.as_ref().ok_or("Önce Bağlan ile servisi okuyun.")?;
                (
                    caps.get_feature
                        .clone()
                        .unwrap_or_else(|| crate::query::base(&self.url).to_owned()),
                    Some(item.id.clone()),
                    Some(caps.version.clone()),
                )
            }
            FeedKind::OgcFeatures => {
                let c = self
                    .collections
                    .iter()
                    .find(|c| c.id == item.id)
                    .ok_or("Koleksiyon bulunamadı.")?;
                (
                    c.items.clone().ok_or("Koleksiyonun nesne adresi yok.")?,
                    Some(item.id.clone()),
                    None,
                )
            }
            FeedKind::Arcgis => {
                let layer_url = if self.arcgis_layer.is_some() {
                    self.url.trim_end_matches('/').to_owned()
                } else {
                    join(&self.url, &item.id)
                };
                (layer_url, Some(item.id.clone()), None)
            }
            FeedKind::Geojson => (self.url.clone(), None, None),
        };
        let feed = FeatureFeed {
            kind: self.kind,
            url,
            name,
            srid,
            filter: some(&choice.filter),
            bbox: choice.bbox,
            limit: choice.limit.filter(|l| *l > 0),
            version,
            key: some(&choice.key),
            connection: choice.connection.clone(),
            fetched: None,
        };
        if let Some(p) = feed.problem() {
            return Err(p);
        }
        Ok(feed)
    }
}

/// Taking a feed's objects, page by page.
#[derive(Clone, Debug)]
pub struct Taking {
    feed: FeatureFeed,
    /// The system the objects come in.
    pub srid: u32,
    /// The area in that system.
    area: Option<[f64; 4]>,
    /// The most objects to take.
    most: u64,
    /// Taken so far.
    pub taken: u64,
    /// What the service said it matched, when it said.
    pub matched: Option<u64>,
    /// What the pages' readers left out, summed by what and why.
    pub skipped: Vec<ReportItem>,
    /// WFS: GeoJSON asked for (else GML).
    geojson: bool,
    page: u64,
    offset: u64,
    /// The next request's address.
    next: Option<String>,
}

impl Taking {
    /// The first page's request for `feed`; `area` in the system the objects
    /// are asked in (the feed's), `geojson` whether a WFS offers GeoJSON.
    pub fn start(
        feed: &FeatureFeed,
        area: Option<[f64; 4]>,
        most: u32,
        geojson: bool,
    ) -> (Taking, Request) {
        let srid = feed.srid.unwrap_or(4326);
        let most = u64::from(most.clamp(1, MOST)).min(feed.limit.unwrap_or(u64::MAX));
        let t = Taking {
            feed: feed.clone(),
            srid,
            area,
            most,
            taken: 0,
            matched: None,
            skipped: Vec::new(),
            geojson,
            page: PAGE.min(most),
            offset: 0,
            next: None,
        };
        let url = t.page_url();
        (t, Request::get(url))
    }

    fn page_url(&self) -> String {
        let f = &self.feed;
        match f.kind {
            FeedKind::Wfs => request::wfs::get_feature(&request::wfs::GetFeature {
                base: &f.url,
                version: f.version.as_deref().unwrap_or("2.0.0"),
                type_name: f.name.as_deref().unwrap_or(""),
                srid: self.srid,
                bbox: self.area,
                filter: f.filter.as_deref(),
                count: Some(self.page),
                start: Some(self.offset),
                output_format: self.geojson.then_some("application/json"),
                short_srs: self.geojson,
            }),
            FeedKind::OgcFeatures => self.next.clone().unwrap_or_else(|| {
                let crs84 = self.srid == 4326;
                request::features::items(
                    &f.url,
                    self.area,
                    (!crs84).then_some(self.srid),
                    (!crs84).then_some(self.srid),
                    self.page,
                    f.filter.as_deref(),
                )
            }),
            FeedKind::Arcgis => request::arcgis::query(
                &f.url,
                f.filter.as_deref(),
                self.area,
                self.srid,
                self.offset,
                self.page,
            ),
            FeedKind::Geojson => f.url.clone(),
        }
    }

    /// A page's answer: its objects on layer `layer` (in [`Taking::srid`]),
    /// and the next page's request when there is one.
    pub fn answer(
        &mut self,
        body: &str,
        layer: &str,
    ) -> Result<(ImportResult, Option<Request>), String> {
        let left = self.most.saturating_sub(self.taken);
        let cap = u32::try_from(left).unwrap_or(u32::MAX);
        let (mut result, count, more) = match self.feed.kind {
            FeedKind::Wfs => {
                let (text, matched) = if self.geojson {
                    let matched = serde_json::from_str::<serde_json::Value>(body)
                        .ok()
                        .and_then(|v| {
                            v["numberMatched"]
                                .as_u64()
                                .or_else(|| v["totalFeatures"].as_u64())
                        });
                    (body.to_owned(), matched)
                } else {
                    crate::gml::to_geojson(body)?
                };
                if matched.is_some() {
                    self.matched = matched;
                }
                let n = features::feature_count(&text) as u64;
                let r = features::read_geojson(&text, layer, cap)?;
                let more = n >= self.page && self.matched.is_none_or(|m| self.taken + n < m);
                (r, n, more)
            }
            FeedKind::OgcFeatures => {
                let n = features::feature_count(body) as u64;
                if let Some(m) = serde_json::from_str::<serde_json::Value>(body)
                    .ok()
                    .and_then(|v| v["numberMatched"].as_u64())
                {
                    self.matched = Some(m);
                }
                let base = self.next.clone().unwrap_or_else(|| self.feed.url.clone());
                self.next = features::next_link(body, &base);
                let r = features::read_geojson(body, layer, cap)?;
                (r, n, self.next.is_some())
            }
            FeedKind::Arcgis => {
                let n = features::feature_count(body) as u64;
                let more = features::arcgis_more(body);
                (features::read_geojson(body, layer, cap)?, n, more)
            }
            FeedKind::Geojson => {
                let r = features::read_geojson(body, layer, cap)?;
                (r, 0, false)
            }
        };
        // A CQL filter takes the place of the box: the box is the host's to keep.
        if self.feed.filter.is_some()
            && self.feed.kind == FeedKind::Wfs
            && let Some(area) = self.area
        {
            features::within(&mut result, area);
        }
        for i in &result.report.skipped {
            match self
                .skipped
                .iter_mut()
                .find(|k| k.what == i.what && k.reason == i.reason)
            {
                Some(k) => k.count += i.count,
                // A page's lines are not the take's: none kept.
                None => self.skipped.push(ReportItem {
                    lines: Vec::new(),
                    ..i.clone()
                }),
            }
        }
        self.taken += result.entities.len() as u64;
        self.offset += count;
        let go_on = more && count > 0 && self.taken < self.most;
        let next = go_on.then(|| Request::get(self.page_url()));
        Ok((result, next))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAYER: &str = r#"{"type":"Feature Layer","name":"Ülkeler","geometryType":"esriGeometryPolygon","maxRecordCount":2000,"supportedQueryFormats":"JSON, geoJSON, PBF"}"#;

    /// Bağlan on an ArcGIS layer's own address.
    fn arcgis_layer() -> FeedConnecting {
        let (mut c, first) = FeedConnecting::start(
            FeedKind::Arcgis,
            "https://ornek.org/arcgis/rest/services/Ulkeler/FeatureServer/0",
        )
        .expect("starts");
        assert!(first.is_some_and(|r| r.url.contains("f=json")));
        assert!(c.answer(LAYER).expect("reads").is_none());
        c
    }

    #[test]
    fn an_arcgis_layer_is_asked_in_the_projects_system_and_the_list_shows_it() {
        let c = arcgis_layer();
        let id = c.offer().expect("an offer").items[0].id.clone();
        assert_eq!(id, "0");
        assert_eq!(c.systems(&id, 5255), vec![5255, 4326]);
        assert_eq!(c.asked_srid(&id, None, 5255), Some(5255));
        assert_eq!(c.asked_srid(&id, Some(4326), 5255), Some(4326));
        // A system the list does not have is not asked for.
        assert_eq!(c.asked_srid(&id, Some(3857), 5255), Some(5255));
        // A project without a system: WGS 84.
        assert_eq!(c.systems(&id, 0), vec![4326]);
        assert_eq!(c.asked_srid(&id, None, 0), Some(4326));
        let choice = FeedChoice {
            item: id.clone(),
            srid: Some(4326),
            ..FeedChoice::default()
        };
        assert_eq!(c.feed(&choice, 5255).expect("a feed").srid, Some(4326));
        assert_eq!(c.asked_srid("9", None, 5255), None);
    }

    #[test]
    fn a_geojson_address_has_no_system_to_choose() {
        let (c, first) = FeedConnecting::start(FeedKind::Geojson, "https://ornek.org/veri.geojson")
            .expect("starts");
        assert!(first.is_none());
        let id = c.offer().expect("an offer").items[0].id.clone();
        assert_eq!(c.asked_srid(&id, Some(5255), 5255), None);
        let choice = FeedChoice {
            item: id,
            ..FeedChoice::default()
        };
        assert_eq!(c.feed(&choice, 5255).expect("a feed").srid, None);
    }

    #[test]
    fn the_pages_left_outs_are_summed_and_said() {
        let c = arcgis_layer();
        let choice = FeedChoice {
            item: "0".into(),
            ..FeedChoice::default()
        };
        let feed = c.feed(&choice, 5255).expect("a feed");
        let (mut t, first) = Taking::start(&feed, None, 1000, false);
        assert!(first.url.contains("outSR=5255"), "{}", first.url);
        let page = r#"{"type":"FeatureCollection","exceededTransferLimit":true,"features":[
            {"type":"Feature","properties":{"ULKE":"Türkiye"},"geometry":{"type":"Point","coordinates":[32.85,39.92]}},
            {"type":"Feature","properties":{"ULKE":"Arjantin"},"geometry":null}]}"#;
        let (r, next) = t.answer(page, "").expect("reads");
        assert_eq!(r.entities.len(), 1);
        assert!(next.expect("a second page").url.contains("resultOffset=2"));
        let last = r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{},"geometry":null}]}"#;
        let (r, next) = t.answer(last, "").expect("reads");
        assert!(r.entities.is_empty() && next.is_none());
        assert_eq!(t.skipped.len(), 1);
        assert_eq!(
            (t.skipped[0].what.as_str(), t.skipped[0].count),
            (kentos_formats::geojson::NULL_GEOMETRY, 2)
        );
        assert!(t.skipped[0].lines.is_empty());
        assert_eq!(
            taken_words(1, None, &t.skipped, 0, false, t.srid),
            "1 nesne alındı. 2 nesne servisten geometrisi boş geldi ve alınmadı. Geometrisi boş gelen nesne, servisin istenen sistemde (EPSG:5255) gösteremediği nesne olabilir: İstenen sistem'i WGS 84 (EPSG:4326) yapıp yeniden alabilirsiniz."
        );
        assert_eq!(
            taken_words(1, None, &t.skipped, 0, false, 4326),
            "1 nesne alındı. 2 nesne servisten geometrisi boş geldi ve alınmadı."
        );
    }

    #[test]
    fn the_words_say_what_matched_what_was_left_out_and_what_stopped_it() {
        let other = ReportItem {
            what: "Geometri türü “Circle”".into(),
            count: 3,
            reason: "desteklenmiyor; alınmadı".into(),
            lines: vec![4],
        };
        assert_eq!(
            taken_words(10, Some(25), &[other], 2, true, 4326),
            "10 nesne alındı. Servis 25 nesne eşleştiğini söyledi. 3 nesne alınmadı: Geometri türü “Circle” (desteklenmiyor; alınmadı). 2 nesne projenin sisteminde yer bulamadığı için alınmadı. En çok nesne sınırına ulaşıldı; servisin başka nesneleri de olabilir."
        );
        assert_eq!(
            taken_words(5, Some(5), &[], 0, false, 4326),
            "5 nesne alındı."
        );
    }
}
