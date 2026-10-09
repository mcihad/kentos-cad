//! Harita servisi's Bağlan and Ekle (docs/adr/0208 §14), the same in both
//! apps' windows: the requests that read what a service has, one after
//! another (a WMS's or a WMTS's capabilities, an ArcGIS service's JSON, a
//! vector style or a TileJSON, an OGC API's landing page, its tile sets and
//! their matrix sets), the offer they make (the layers, styles, formats and
//! systems to pick from) and the service layer a choice from it makes.
//!
//! The host sends each request with the connection's proof and gives the
//! answer back ([`Connecting::answer`]); XYZ and Google read nothing.

use std::collections::VecDeque;

use kentos_contracts::{ServiceKind, ServiceLayer, ServiceParam, TileGrid, TileMatrix};
use serde::{Deserialize, Serialize};

use crate::attribution;
use crate::caps::arcgis::{self, ArcgisService};
use crate::caps::ogc::{self, MatrixSet, TileSet};
use crate::caps::tilejson;
use crate::caps::wms::{self, WmsCapabilities};
use crate::caps::wmts::{self, WmtsCapabilities};
use crate::query::with_params;
use crate::request::{self, Request};
use crate::source::is_vector_format;

/// One of the things a service offers: a WMS or WMTS layer, an OGC tile
/// set, an ArcGIS layer, a vector source.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    /// What the request names it by (a WMS or ArcGIS layer's name or id, a
    /// WMTS identifier, an OGC tile set's place in the list).
    pub id: String,
    pub title: String,
    /// How deep in the service's tree, the root 0.
    pub depth: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Whether it can be asked for (a WMS layer without a name only groups others).
    pub pickable: bool,
    /// Whether Servis bilgisi asks it.
    pub queryable: bool,
    pub styles: Vec<Named>,
    pub formats: Vec<String>,
    /// The systems it is offered in.
    pub srids: Vec<u32>,
    /// `[west, south, east, north]`, degrees.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wgs84: Option<[f64; 4]>,
    pub vector: bool,
}

/// A style: its name and its title.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Named {
    pub id: String,
    pub title: String,
}

/// What Bağlan found.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Offer {
    pub kind: ServiceKind,
    pub title: String,
    pub items: Vec<Item>,
    /// WMS's GetMap formats (WMTS's are each item's).
    pub formats: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// ArcGIS: the service draws any view (`export`); it has a tile cache.
    pub exports: bool,
    pub tiled: bool,
    /// How many items it has that can be picked.
    pub pickable: usize,
}

/// What the window chose.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Choice {
    /// The items' ids (WMS: one or more; the others: one).
    pub items: Vec<String>,
    pub style: Option<String>,
    pub format: Option<String>,
    pub srid: Option<u32>,
    pub transparent: bool,
    /// WMS and `export`: one picture of the view.
    pub dynamic: bool,
    pub opacity: Option<f64>,
    pub connection: Option<String>,
    /// Google's map type.
    pub map_type: Option<String>,
    pub min_zoom: Option<u32>,
    pub max_zoom: Option<u32>,
    /// XYZ.
    pub tile_size: Option<u32>,
    pub subdomains: Vec<String>,
    pub y_flip: bool,
    pub attribution: Option<String>,
}

/// Where Bağlan is.
#[derive(Clone, Debug)]
enum Stage {
    Wms,
    Wmts,
    Arcgis,
    Vector,
    /// An OGC API document of unknown kind: a landing page, collections, a list of tile sets or one.
    OgcAny,
    /// The tile sets' own documents and matrix sets still to read.
    OgcSets,
    Done,
}

/// Bağlan for one service: what it has read so far.
#[derive(Clone, Debug)]
pub struct Connecting {
    kind: ServiceKind,
    url: String,
    stage: Stage,
    wms: Option<WmsCapabilities>,
    wmts: Option<WmtsCapabilities>,
    arcgis: Option<ArcgisService>,
    /// A vector source: its template, levels, box, credits and title.
    vector: Option<VectorSource>,
    /// OGC API's tile sets, each with its matrix set when it is not Web Mercator's.
    sets: Vec<(TileSet, Option<MatrixSet>)>,
    /// What is still to be asked for an OGC set: (set, its own document) or (set, its matrix set).
    queue: VecDeque<(usize, bool, String)>,
    /// The request whose answer comes next.
    asked: Option<String>,
    offer: Option<Offer>,
}

#[derive(Clone, Debug)]
struct VectorSource {
    /// The style's address or a TileJSON's (the layer keeps it), or the template.
    url: String,
    title: String,
    min: u32,
    max: u32,
    bounds: Option<[f64; 4]>,
    attribution: Option<String>,
}

/// Why an address cannot be a service's, in the window's words.
fn address(url: &str) -> Result<String, String> {
    let u = url.trim();
    if let Some(p) = kentos_contracts::service::http_problem("Servisin adresi", u) {
        return Err(p);
    }
    Ok(u.to_owned())
}

impl Connecting {
    /// Bağlan: the first request, or none when the kind reads nothing (XYZ, Google, a vector template): its offer is ready.
    pub fn start(kind: ServiceKind, url: &str) -> Result<(Connecting, Option<Request>), String> {
        let mut c = Connecting {
            kind,
            url: String::new(),
            stage: Stage::Done,
            wms: None,
            wmts: None,
            arcgis: None,
            vector: None,
            sets: Vec::new(),
            queue: VecDeque::new(),
            asked: None,
            offer: None,
        };
        if kind == ServiceKind::Google {
            c.offer = Some(Offer {
                kind,
                title: "Google Map Tiles API".into(),
                items: ["roadmap", "satellite", "terrain", "hybrid"]
                    .iter()
                    .map(|t| item(t, map_type_title(t)))
                    .collect(),
                formats: Vec::new(),
                attribution: Some("Google Maps".into()),
                version: None,
                exports: false,
                tiled: true,
                pickable: 4,
            });
            return Ok((c, None));
        }
        c.url = address(url)?;
        let first = match kind {
            ServiceKind::Xyz => {
                let probe = ServiceLayer {
                    url: c.url.clone(),
                    ..empty(ServiceKind::Xyz)
                };
                if let Some(p) = probe.problem() {
                    return Err(p);
                }
                c.offer = Some(Offer {
                    kind,
                    title: kentos_contracts::origin_of(&c.url).unwrap_or_default(),
                    items: vec![item("xyz", "Karolar")],
                    formats: Vec::new(),
                    attribution: None,
                    version: None,
                    exports: false,
                    tiled: true,
                    pickable: 1,
                });
                return Ok((c, None));
            }
            ServiceKind::Vector if c.url.contains("{z}") => {
                c.vector = Some(VectorSource {
                    url: c.url.clone(),
                    title: kentos_contracts::origin_of(&c.url).unwrap_or_default(),
                    min: 0,
                    max: 14,
                    bounds: None,
                    attribution: None,
                });
                c.finish_vector();
                return Ok((c, None));
            }
            ServiceKind::Vector => {
                c.stage = Stage::Vector;
                c.url.clone()
            }
            ServiceKind::Wms => {
                c.stage = Stage::Wms;
                if c.url
                    .to_ascii_lowercase()
                    .contains("request=getcapabilities")
                {
                    c.url.clone()
                } else {
                    request::wms::capabilities(&c.url, Some("1.3.0"))
                }
            }
            ServiceKind::Wmts => {
                c.stage = Stage::Wmts;
                let lower = c.url.to_ascii_lowercase();
                if lower.ends_with(".xml") || lower.contains("request=getcapabilities") {
                    c.url.clone()
                } else {
                    request::wmts::capabilities(&c.url)
                }
            }
            ServiceKind::Arcgis => {
                c.stage = Stage::Arcgis;
                request::arcgis::info(&c.url)
            }
            ServiceKind::OgcTiles => {
                c.stage = Stage::OgcAny;
                with_params(&c.url, &[("f", "json")])
            }
            ServiceKind::Google => unreachable!("answered above"),
        };
        c.asked = Some(first.clone());
        Ok((c, Some(Request::get(first))))
    }

    /// The answer to the last request: the next request, or none when the offer is ready.
    pub fn answer(&mut self, body: &str) -> Result<Option<Request>, String> {
        let asked = self.asked.take().unwrap_or_else(|| self.url.clone());
        match self.stage {
            Stage::Wms => {
                let caps = wms::read(body)?;
                self.wms = Some(caps);
                self.finish_wms();
                Ok(None)
            }
            Stage::Wmts => {
                let caps = wmts::read(body)?;
                if caps.layers.is_empty() {
                    return Err("WMTS servisi hiç katman sunmuyor.".into());
                }
                self.wmts = Some(caps);
                self.finish_wmts();
                Ok(None)
            }
            Stage::Arcgis => {
                let s = arcgis::service(body)?;
                self.arcgis = Some(s);
                self.finish_arcgis();
                Ok(None)
            }
            Stage::Vector => {
                self.read_vector(body, &asked)?;
                self.finish_vector();
                Ok(None)
            }
            Stage::OgcAny => self.read_ogc(body, &asked),
            Stage::OgcSets => {
                let Some((i, own, _)) = self.queue.pop_front() else {
                    return self.finish_ogc();
                };
                if own {
                    let doc = ogc::tile_set_doc(body, &asked)?;
                    let set = &mut self.sets[i].0;
                    if set.template.is_none() {
                        set.template = doc.template;
                    }
                    if set.matrix_set_href.is_none() {
                        set.matrix_set_href = doc.matrix_set_href;
                    }
                    if set.matrix_set.is_empty() {
                        set.matrix_set = doc.matrix_set;
                    }
                    // Its matrix set, when it is not Web Mercator's and is linked.
                    if !ogc::is_web_mercator_quad(&set.matrix_set)
                        && let Some(href) = set.matrix_set_href.clone()
                    {
                        self.queue.push_front((i, false, href));
                    }
                } else {
                    self.sets[i].1 = Some(ogc::matrix_set(body)?);
                }
                self.next_ogc()
            }
            Stage::Done => Ok(None),
        }
    }

    /// What it found, once read.
    pub fn offer(&self) -> Option<&Offer> {
        self.offer.as_ref()
    }

    fn finish_wms(&mut self) {
        let Some(caps) = &self.wms else {
            return;
        };
        let items: Vec<Item> = caps
            .layers
            .iter()
            .map(|l| Item {
                id: l.name.clone().unwrap_or_default(),
                title: if l.title.is_empty() {
                    l.name.clone().unwrap_or_default()
                } else {
                    l.title.clone()
                },
                depth: l.depth,
                summary: l.summary.clone(),
                pickable: l.name.is_some(),
                queryable: l.queryable,
                styles: l
                    .styles
                    .iter()
                    .map(|s| Named {
                        id: s.name.clone(),
                        title: if s.title.is_empty() {
                            s.name.clone()
                        } else {
                            s.title.clone()
                        },
                    })
                    .collect(),
                formats: caps.formats.clone(),
                srids: l.srids.clone(),
                wgs84: l.wgs84,
                vector: false,
            })
            .collect();
        let pickable = items.iter().filter(|i| i.pickable).count();
        self.offer = Some(Offer {
            kind: ServiceKind::Wms,
            title: caps.title.clone(),
            formats: caps.formats.clone(),
            attribution: caps.layers.iter().find_map(|l| l.attribution.clone()),
            version: Some(caps.version.clone()),
            exports: true,
            tiled: false,
            pickable,
            items,
        });
        self.stage = Stage::Done;
    }

    fn finish_wmts(&mut self) {
        let Some(caps) = &self.wmts else {
            return;
        };
        let items: Vec<Item> = caps
            .layers
            .iter()
            .map(|l| {
                let srids: Vec<u32> = l
                    .links
                    .iter()
                    .filter_map(|k| caps.matrix_sets.iter().find(|m| m.id == k.set))
                    .filter_map(|m| m.srid)
                    .fold(Vec::new(), |mut v, s| {
                        if !v.contains(&s) {
                            v.push(s);
                        }
                        v
                    });
                Item {
                    id: l.id.clone(),
                    title: if l.title.is_empty() {
                        l.id.clone()
                    } else {
                        l.title.clone()
                    },
                    depth: 0,
                    summary: l.summary.clone(),
                    pickable: !srids.is_empty(),
                    queryable: false,
                    styles: l
                        .styles
                        .iter()
                        .map(|s| Named {
                            id: s.id.clone(),
                            title: if s.title.is_empty() {
                                s.id.clone()
                            } else {
                                s.title.clone()
                            },
                        })
                        .collect(),
                    formats: l.formats.clone(),
                    vector: !l.formats.is_empty() && l.formats.iter().all(|f| is_vector_format(f)),
                    srids,
                    wgs84: l.wgs84,
                }
            })
            .collect();
        let pickable = items.iter().filter(|i| i.pickable).count();
        self.offer = Some(Offer {
            kind: ServiceKind::Wmts,
            title: caps.title.clone(),
            formats: Vec::new(),
            attribution: None,
            version: Some("1.0.0".into()),
            exports: false,
            tiled: true,
            pickable,
            items,
        });
        self.stage = Stage::Done;
    }

    fn finish_arcgis(&mut self) {
        let Some(s) = &self.arcgis else {
            return;
        };
        let wgs84 = s.extent.and_then(|e| match s.srid {
            Some(4326) => Some(e),
            Some(3857) => Some(mercator_degrees(e)),
            _ => None,
        });
        let mut items = vec![Item {
            id: String::new(),
            title: if s.title.is_empty() {
                "Bütün katmanlar".into()
            } else {
                s.title.clone()
            },
            depth: 0,
            summary: None,
            pickable: true,
            queryable: true,
            styles: Vec::new(),
            formats: Vec::new(),
            srids: s.srid.into_iter().collect(),
            wgs84,
            vector: false,
        }];
        let depth_of = |id: u32| {
            let mut d = 1;
            let mut at = s.layers.iter().find(|l| l.id == id).and_then(|l| l.parent);
            while let Some(p) = at {
                d += 1;
                at = s.layers.iter().find(|l| l.id == p).and_then(|l| l.parent);
                if d > 32 {
                    break;
                }
            }
            d
        };
        for l in &s.layers {
            items.push(Item {
                id: l.id.to_string(),
                title: l.name.clone(),
                depth: depth_of(l.id),
                summary: l.geometry.clone(),
                // A cached service draws all its layers together: only the whole.
                pickable: s.exports,
                queryable: l.geometry.is_some(),
                styles: Vec::new(),
                formats: Vec::new(),
                srids: s.srid.into_iter().collect(),
                wgs84: None,
                vector: false,
            });
        }
        let pickable = items.iter().filter(|i| i.pickable).count();
        self.offer = Some(Offer {
            kind: ServiceKind::Arcgis,
            title: s.title.clone(),
            formats: Vec::new(),
            attribution: s.copyright.clone(),
            version: None,
            exports: s.exports,
            tiled: !s.tiles.is_empty(),
            pickable,
            items,
        });
        self.stage = Stage::Done;
    }

    fn read_vector(&mut self, body: &str, asked: &str) -> Result<(), String> {
        let v: serde_json::Value = serde_json::from_str(body).map_err(|_| {
            "Yanıt JSON değil: adres bir MapLibre stili ya da TileJSON olmalı.".to_owned()
        })?;
        if v["layers"].is_array() && v["sources"].is_object() {
            let st = crate::style::parse(body, asked)?;
            let main = st.main_vector().ok_or("Stilin vektör kaynağı yok.")?;
            self.vector = Some(VectorSource {
                url: self.url.clone(),
                title: v["name"].as_str().unwrap_or("").to_owned(),
                min: main.min_zoom,
                max: main.max_zoom,
                bounds: None,
                attribution: main.attribution.clone(),
            });
        } else {
            let tj = tilejson::read(body, asked)?;
            self.vector = Some(VectorSource {
                url: self.url.clone(),
                title: tj.name.clone(),
                min: tj.min_zoom,
                max: tj.max_zoom,
                bounds: tj.bounds,
                attribution: tj.attribution.clone(),
            });
        }
        Ok(())
    }

    fn finish_vector(&mut self) {
        let Some(v) = &self.vector else {
            return;
        };
        self.offer = Some(Offer {
            kind: ServiceKind::Vector,
            title: if v.title.is_empty() {
                kentos_contracts::origin_of(&v.url).unwrap_or_default()
            } else {
                v.title.clone()
            },
            items: vec![Item {
                id: "vector".into(),
                title: "Vektör karolar".into(),
                depth: 0,
                summary: None,
                pickable: true,
                queryable: false,
                styles: Vec::new(),
                formats: vec!["application/vnd.mapbox-vector-tile".into()],
                srids: vec![3857],
                wgs84: v.bounds,
                vector: true,
            }],
            formats: Vec::new(),
            attribution: v
                .attribution
                .as_deref()
                .map(|a| attribution::credit(a).text),
            version: None,
            exports: false,
            tiled: true,
            pickable: 1,
        });
        self.stage = Stage::Done;
    }

    /// An OGC API document: a landing page or a collection (its tiles followed), collections (the
    /// first with tiles followed), a list of tile sets or one.
    fn read_ogc(&mut self, body: &str, asked: &str) -> Result<Option<Request>, String> {
        let v: serde_json::Value = serde_json::from_str(body)
            .map_err(|_| "Yanıt JSON değil: adres bir OGC API sayfası olmalı.".to_owned())?;
        if v["tilesets"].is_array() {
            self.sets = ogc::tile_sets(body, asked)?
                .into_iter()
                .map(|s| (s, None))
                .collect();
            if self.sets.is_empty() {
                return Err("OGC API karo kümesi listesi boş.".into());
            }
            return self.queue_ogc();
        }
        if v["tileMatrixSetURI"].is_string()
            || v["tileMatrixSetId"].is_string()
            || v["dataType"].is_string()
        {
            let mut set = ogc::tile_set_doc(body, asked)?;
            if set.href.is_none() {
                set.href = Some(asked.to_owned());
            }
            self.sets = vec![(set, None)];
            return self.queue_ogc();
        }
        if v["collections"].is_array() {
            let list = ogc::collections(body, asked)?;
            let Some(tiles) = list.iter().find_map(|c| c.tiles.clone()) else {
                return Err("Koleksiyonların hiçbirinin karo kümesi yok.".into());
            };
            return Ok(Some(self.ask(with_params(&tiles, &[("f", "json")]))));
        }
        let page = ogc::landing(body, asked)?;
        let next = ogc::link(&page.links, &["tilesets-map", "tilesets-vector", "tiles"])
            .or_else(|| ogc::link(&page.links, &["data"]))
            .map(|l| l.href.clone())
            .ok_or("Sayfada karo kümesi ya da koleksiyon bağlantısı yok.")?;
        Ok(Some(self.ask(with_params(&next, &[("f", "json")]))))
    }

    fn ask(&mut self, url: String) -> Request {
        self.asked = Some(url.clone());
        Request::get(url)
    }

    fn queue_ogc(&mut self) -> Result<Option<Request>, String> {
        self.stage = Stage::OgcSets;
        self.queue.clear();
        for (i, (set, _)) in self.sets.iter().enumerate() {
            if set.template.is_none()
                && let Some(href) = &set.href
            {
                self.queue.push_back((i, true, href.clone()));
            } else if !ogc::is_web_mercator_quad(&set.matrix_set)
                && let Some(href) = &set.matrix_set_href
            {
                self.queue.push_back((i, false, href.clone()));
            }
        }
        self.next_ogc()
    }

    fn next_ogc(&mut self) -> Result<Option<Request>, String> {
        match self.queue.front() {
            Some((_, _, href)) => {
                let href = href.clone();
                Ok(Some(self.ask(with_params(&href, &[("f", "json")]))))
            }
            None => self.finish_ogc(),
        }
    }

    fn finish_ogc(&mut self) -> Result<Option<Request>, String> {
        let items: Vec<Item> = self
            .sets
            .iter()
            .enumerate()
            .map(|(i, (s, m))| {
                let srid = if ogc::is_web_mercator_quad(&s.matrix_set) {
                    Some(3857)
                } else {
                    m.as_ref().and_then(|m| m.srid)
                };
                Item {
                    id: i.to_string(),
                    title: if s.title.is_empty() {
                        s.matrix_set.rsplit('/').next().unwrap_or("").to_owned()
                    } else {
                        s.title.clone()
                    },
                    depth: 0,
                    summary: Some(if s.data_type == "vector" {
                        "Vektör karolar".into()
                    } else {
                        "Harita karoları".into()
                    }),
                    pickable: s.template.is_some() && srid.is_some(),
                    queryable: false,
                    styles: Vec::new(),
                    formats: Vec::new(),
                    srids: srid.into_iter().collect(),
                    wgs84: None,
                    vector: s.data_type == "vector",
                }
            })
            .collect();
        let pickable = items.iter().filter(|i| i.pickable).count();
        self.offer = Some(Offer {
            kind: ServiceKind::OgcTiles,
            title: kentos_contracts::origin_of(&self.url).unwrap_or_default(),
            items,
            formats: Vec::new(),
            attribution: None,
            version: None,
            exports: false,
            tiled: true,
            pickable,
        });
        self.stage = Stage::Done;
        Ok(None)
    }

    /// The service layer `choice` makes, or why it makes none. `project` is
    /// the project's system (the one asked first when offered).
    pub fn layer(&self, choice: &Choice, project: u32) -> Result<ServiceLayer, String> {
        let offer = self
            .offer
            .as_ref()
            .ok_or("Önce Bağlan ile servisi okuyun.")?;
        let picked: Vec<&Item> = choice
            .items
            .iter()
            .filter_map(|id| offer.items.iter().find(|i| &i.id == id && i.pickable))
            .collect();
        if picked.is_empty() {
            return Err("Listeden bir katman seçin.".into());
        }
        let mut s = ServiceLayer {
            url: self.url.clone(),
            opacity: choice.opacity.filter(|o| *o < 1.0),
            connection: choice.connection.clone(),
            attribution: choice
                .attribution
                .clone()
                .filter(|a| !a.trim().is_empty())
                .or(offer.attribution.clone()),
            min_zoom: choice.min_zoom,
            max_zoom: choice.max_zoom,
            bbox: union(picked.iter().filter_map(|i| i.wgs84)),
            ..empty(self.kind)
        };
        match self.kind {
            ServiceKind::Xyz => {
                s.tile_size = choice.tile_size.filter(|t| *t != 256);
                s.subdomains = choice.subdomains.clone();
                s.y_flip = choice.y_flip;
            }
            ServiceKind::Google => {
                s.url = String::new();
                s.style = Some(picked[0].id.clone());
                s.attribution = None;
            }
            ServiceKind::Vector => {
                let v = self
                    .vector
                    .as_ref()
                    .ok_or("Önce Bağlan ile servisi okuyun.")?;
                s.url = v.url.clone();
                s.min_zoom = choice.min_zoom.or((v.min > 0).then_some(v.min));
                s.max_zoom = choice
                    .max_zoom
                    .or((!v.url.contains("{z}")).then_some(v.max));
            }
            ServiceKind::Wms => {
                let caps = self.wms.as_ref().ok_or("Önce Bağlan ile servisi okuyun.")?;
                s.url = caps
                    .get_map
                    .clone()
                    .unwrap_or_else(|| crate::query::base(&self.url).to_owned());
                s.layers = picked.iter().map(|i| i.id.clone()).collect();
                s.version = Some(caps.version.clone());
                s.format = Some(
                    choice
                        .format
                        .clone()
                        .unwrap_or_else(|| best_format(&caps.formats, choice.transparent)),
                );
                // One style for one layer; several layers take their default styles.
                s.style = choice
                    .style
                    .clone()
                    .filter(|st| !st.is_empty() && picked.len() == 1);
                let offered: Vec<u32> = common(picked.iter().map(|i| i.srids.as_slice()));
                s.srid = Some(
                    choice
                        .srid
                        .filter(|c| offered.contains(c))
                        .or_else(|| suggested_srid(&offered, project))
                        .ok_or("Seçilen katmanlar KentOS'un bildiği bir sistemde sunulmuyor.")?,
                );
                s.transparent = choice.transparent;
                s.dynamic = choice.dynamic;
            }
            ServiceKind::Wmts => {
                let caps = self
                    .wmts
                    .as_ref()
                    .ok_or("Önce Bağlan ile servisi okuyun.")?;
                let id = &picked[0].id;
                let layer = caps
                    .layers
                    .iter()
                    .find(|l| &l.id == id)
                    .ok_or("Katman bulunamadı.")?;
                let sets: Vec<&wmts::WmtsMatrixSet> = layer
                    .links
                    .iter()
                    .filter_map(|k| caps.matrix_sets.iter().find(|m| m.id == k.set))
                    .filter(|m| m.srid.is_some_and(crate::crs::known) && !m.matrices.is_empty())
                    .collect();
                let srids: Vec<u32> = sets.iter().filter_map(|m| m.srid).collect();
                let want = choice
                    .srid
                    .filter(|c| srids.contains(c))
                    .or_else(|| suggested_srid(&srids, project))
                    .ok_or(
                        "Katmanın matris kümelerinin hiçbiri KentOS'un bildiği bir sistemde değil.",
                    )?;
                let set = sets
                    .iter()
                    .find(|m| m.srid == Some(want))
                    .ok_or("Matris kümesi bulunamadı.")?;
                s.layers = vec![id.clone()];
                s.matrix_set = Some(set.id.clone());
                s.grid = Some(TileGrid {
                    srid: want,
                    matrices: set.matrices.clone(),
                });
                let format = choice
                    .format
                    .clone()
                    .filter(|f| layer.formats.contains(f))
                    .unwrap_or_else(|| best_format(&layer.formats, true));
                s.style = Some(
                    choice
                        .style
                        .clone()
                        .filter(|st| layer.styles.iter().any(|x| &x.id == st))
                        .or_else(|| {
                            layer
                                .styles
                                .iter()
                                .find(|x| x.default)
                                .map(|x| x.id.clone())
                        })
                        .or_else(|| layer.styles.first().map(|x| x.id.clone()))
                        .unwrap_or_else(|| "default".into()),
                );
                s.template = layer
                    .templates
                    .iter()
                    .find(|(f, _)| *f == format)
                    .or_else(|| layer.templates.first())
                    .map(|(_, t)| t.clone());
                if s.template.is_none() {
                    s.url = caps
                        .get_tile
                        .clone()
                        .unwrap_or_else(|| crate::query::base(&self.url).to_owned());
                } else {
                    s.url = crate::query::base(&self.url).to_owned();
                }
                s.format = Some(format);
                s.params = layer
                    .dimensions
                    .iter()
                    .map(|(n, v)| ServiceParam {
                        name: n.clone(),
                        value: v.clone(),
                    })
                    .collect();
            }
            ServiceKind::OgcTiles => {
                let i: usize = picked[0]
                    .id
                    .parse()
                    .map_err(|_| "Karo kümesi bulunamadı.")?;
                let (set, matrix) = self.sets.get(i).ok_or("Karo kümesi bulunamadı.")?;
                let template = set.template.clone().ok_or("Karo kümesinin şablonu yok.")?;
                let grid = if ogc::is_web_mercator_quad(&set.matrix_set) {
                    web_mercator_grid(256, 24)
                } else {
                    let m = matrix
                        .as_ref()
                        .ok_or("Karo kümesinin matris kümesi okunamadı.")?;
                    TileGrid {
                        srid: m
                            .srid
                            .ok_or("Matris kümesinin sistemi KentOS'un kaydında yok.")?,
                        matrices: m.matrices.clone(),
                    }
                };
                s.url = set.href.clone().unwrap_or_else(|| self.url.clone());
                s.template = Some(template);
                s.format = Some(if set.data_type == "vector" {
                    "application/vnd.mapbox-vector-tile".into()
                } else {
                    "image/png".into()
                });
                s.grid = Some(grid);
            }
            ServiceKind::Arcgis => {
                let a = self
                    .arcgis
                    .as_ref()
                    .ok_or("Önce Bağlan ile servisi okuyun.")?;
                let whole = picked.iter().any(|i| i.id.is_empty());
                if !a.tiles.is_empty() && whole && !choice.dynamic {
                    s.grid = Some(TileGrid {
                        srid: a.srid.ok_or("Servisin sistemi KentOS'un kaydında yok.")?,
                        matrices: a.tiles.clone(),
                    });
                } else {
                    if !a.exports {
                        return Err(
                            "Servis yalnız karolarıyla çizer: bütün katmanları seçin.".into()
                        );
                    }
                    s.srid = Some(
                        choice
                            .srid
                            .or_else(|| suggested_srid(&[project, 3857], project))
                            .unwrap_or(3857),
                    );
                    s.layers = if whole {
                        Vec::new()
                    } else {
                        picked.iter().map(|i| i.id.clone()).collect()
                    };
                    s.transparent = choice.transparent;
                    s.dynamic = choice.dynamic;
                }
            }
        }
        if let Some(p) = s.problem() {
            return Err(p);
        }
        Ok(s)
    }
}

/// What Dene asks a service layer for to see that its connection works:
/// its capabilities, its style or TileJSON, an XYZ layer's first tile,
/// Google's session.
pub fn probe(s: &ServiceLayer) -> Option<Request> {
    let first_tile = |s: &ServiceLayer| {
        let src = crate::source::tiles(s, None)?;
        let t = kentos_geometry_core::geom::tiles::TileRef {
            level: src.min_level,
            col: 0,
            row: 0,
        };
        crate::source::tile_url(s, &src, t, false).map(Request::get)
    };
    match s.kind {
        ServiceKind::Google => Some(crate::google::create_session(
            &crate::google::SessionOptions {
                map_type: s.style.clone().unwrap_or_else(|| "roadmap".into()),
                language: "tr-TR".into(),
                region: "TR".into(),
                hidpi: false,
            },
        )),
        ServiceKind::Xyz => first_tile(s),
        ServiceKind::Vector if s.url.contains("{z}") => first_tile(s),
        ServiceKind::Vector => Some(Request::get(s.url.clone())),
        ServiceKind::Wms => Some(Request::get(request::wms::capabilities(
            &s.url,
            s.version.as_deref(),
        ))),
        ServiceKind::Wmts => Some(Request::get(request::wmts::capabilities(&s.url))),
        ServiceKind::OgcTiles => Some(Request::get(with_params(&s.url, &[("f", "json")]))),
        ServiceKind::Arcgis => Some(Request::get(request::arcgis::info(&s.url))),
    }
}

/// The system to ask in: the project's when offered (no transformation),
/// else Web Mercator, else WGS 84, else the first KentOS knows.
pub fn suggested_srid(offered: &[u32], project: u32) -> Option<u32> {
    if project != 0 && offered.contains(&project) {
        return Some(project);
    }
    [3857, 4326]
        .into_iter()
        .find(|s| offered.contains(s))
        .or_else(|| offered.iter().copied().find(|s| crate::crs::known(*s)))
}

/// The systems every list offers.
fn common<'a>(mut lists: impl Iterator<Item = &'a [u32]>) -> Vec<u32> {
    let Some(first) = lists.next() else {
        return Vec::new();
    };
    let mut out = first.to_vec();
    for l in lists {
        out.retain(|s| l.contains(s));
    }
    out
}

/// The format to ask for: PNG where it may be clear, JPEG for photographs, else the first.
fn best_format(formats: &[String], clear: bool) -> String {
    let has = |f: &str| formats.iter().find(|x| x.eq_ignore_ascii_case(f)).cloned();
    let order: &[&str] = if clear {
        &[
            "image/png",
            "image/png8",
            "image/png; mode=8bit",
            "image/jpeg",
        ]
    } else {
        &["image/jpeg", "image/png", "image/png8"]
    };
    order
        .iter()
        .find_map(|f| has(f))
        .or_else(|| formats.first().cloned())
        .unwrap_or_else(|| "image/png".into())
}

/// The box holding every box, within the world's degrees.
fn union(boxes: impl Iterator<Item = [f64; 4]>) -> Option<[f64; 4]> {
    boxes
        .reduce(|a, b| {
            [
                a[0].min(b[0]),
                a[1].min(b[1]),
                a[2].max(b[2]),
                a[3].max(b[3]),
            ]
        })
        .map(|[w, s, e, n]| {
            [
                w.clamp(-180.0, 180.0),
                s.clamp(-90.0, 90.0),
                e.clamp(-180.0, 180.0),
                n.clamp(-90.0, 90.0),
            ]
        })
        .filter(|b| b.iter().all(|v| v.is_finite()) && b[0] <= b[2] && b[1] <= b[3])
}

/// An extent in Web Mercator as degrees.
fn mercator_degrees(e: [f64; 4]) -> [f64; 4] {
    const R: f64 = 6_378_137.0;
    let lon = |x: f64| (x / R).to_degrees();
    let lat = |y: f64| (2.0 * (y / R).exp().atan() - std::f64::consts::FRAC_PI_2).to_degrees();
    [lon(e[0]), lat(e[1]), lon(e[2]), lat(e[3])]
}

fn item(id: &str, title: &str) -> Item {
    Item {
        id: id.into(),
        title: title.into(),
        depth: 0,
        summary: None,
        pickable: true,
        queryable: false,
        styles: Vec::new(),
        formats: Vec::new(),
        srids: vec![3857],
        wgs84: None,
        vector: false,
    }
}

fn map_type_title(t: &str) -> &'static str {
    match t {
        "roadmap" => "Yol",
        "satellite" => "Uydu",
        "terrain" => "Arazi",
        _ => "Karma (uydu ve yollar)",
    }
}

/// Web Mercator's square as OGC names it: its matrices 0 to `levels`.
fn web_mercator_grid(side: u32, levels: u32) -> TileGrid {
    const ORIGIN: f64 = 20_037_508.342_789_244;
    TileGrid {
        srid: 3857,
        matrices: (0..=levels)
            .map(|z| {
                let n = 1u64 << z;
                TileMatrix {
                    id: z.to_string(),
                    resolution: 2.0 * ORIGIN / (f64::from(side) * n as f64),
                    x0: -ORIGIN,
                    y0: ORIGIN,
                    tile_width: side,
                    tile_height: side,
                    matrix_width: n,
                    matrix_height: n,
                }
            })
            .collect(),
    }
}

/// A service layer of `kind` with nothing else.
pub fn empty(kind: ServiceKind) -> ServiceLayer {
    ServiceLayer {
        kind,
        url: String::new(),
        layers: Vec::new(),
        style: None,
        format: None,
        srid: None,
        grid: None,
        matrix_set: None,
        template: None,
        tile_size: None,
        min_zoom: None,
        max_zoom: None,
        subdomains: Vec::new(),
        y_flip: false,
        transparent: false,
        version: None,
        params: Vec::new(),
        dynamic: false,
        attribution: None,
        opacity: None,
        connection: None,
        preset: None,
        bbox: None,
    }
}
