//! Schema 32's map services (docs/specs/kcad-v2.md §6.4, §6.5,
//! docs/adr/0208 §2): a layer node's `service` and `feed`, the settings'
//! `connections`. Each is read field by field and then checked whole by the
//! contract's rules (`ServiceLayer::problem`, `FeatureFeed::problem`,
//! `connections_problem`), as the writers check it; a flag written false and
//! an empty list are refused (a writer leaves them out).

use kentos_contracts::{
    AuthKind, FeatureFeed, FeedKind, ServiceConnection, ServiceKind, ServiceLayer, ServiceParam,
    TileGrid, TileMatrix, connections_problem,
};

use super::{list, map, named, required, text, unknown};
use crate::cbor::Reader;
use crate::error::{Code, KcadError};

/// A flag: only `true` is written.
fn only_true(r: &mut Reader<'_>, words: &str) -> Result<bool, KcadError> {
    let at = r.position();
    if !r.bool()? {
        return Err(r.fail_at(Code::BadValue, at, words));
    }
    Ok(true)
}

/// A list of texts that is not empty.
fn texts(r: &mut Reader<'_>, words: &str) -> Result<Vec<String>, KcadError> {
    let at = r.position();
    let all = list(r, |r, _| text(r))?;
    if all.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, words));
    }
    Ok(all)
}

fn small(r: &mut Reader<'_>) -> Result<u32, KcadError> {
    r.uint(u64::from(u32::MAX)).map(|n| n as u32)
}

fn tile_grid(r: &mut Reader<'_>) -> Result<TileGrid, KcadError> {
    let (mut srid, mut matrices) = (None, None);
    map(r, |r, key| {
        match key {
            "srid" => srid = Some(small(r)?),
            "matrices" => matrices = Some(list(r, |r, _| tile_matrix(r))?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(TileGrid {
        srid: required(r, srid, "srid")?,
        matrices: required(r, matrices, "matrices")?,
    })
}

fn tile_matrix(r: &mut Reader<'_>) -> Result<TileMatrix, KcadError> {
    let (mut id, mut x0, mut y0, mut resolution) = (None, None, None, None);
    let (mut tile_width, mut tile_height, mut matrix_width, mut matrix_height) =
        (None, None, None, None);
    map(r, |r, key| {
        match key {
            "id" => id = Some(text(r)?),
            "x0" => x0 = Some(r.float()?),
            "y0" => y0 = Some(r.float()?),
            "resolution" => resolution = Some(r.float()?),
            "tileWidth" => tile_width = Some(small(r)?),
            "tileHeight" => tile_height = Some(small(r)?),
            "matrixWidth" => matrix_width = Some(r.uint(u64::MAX)?),
            "matrixHeight" => matrix_height = Some(r.uint(u64::MAX)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(TileMatrix {
        id: required(r, id, "id")?,
        resolution: required(r, resolution, "resolution")?,
        x0: required(r, x0, "x0")?,
        y0: required(r, y0, "y0")?,
        tile_width: required(r, tile_width, "tileWidth")?,
        tile_height: required(r, tile_height, "tileHeight")?,
        matrix_width: required(r, matrix_width, "matrixWidth")?,
        matrix_height: required(r, matrix_height, "matrixHeight")?,
    })
}

/// A layer's map service, checked whole.
pub(super) fn service_layer(r: &mut Reader<'_>) -> Result<ServiceLayer, KcadError> {
    let at = r.position();
    let (mut kind, mut url) = (None, None);
    let mut s = ServiceLayer {
        kind: ServiceKind::Xyz,
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
    };
    let kinds: Vec<(&str, ServiceKind)> = ServiceKind::ALL.iter().map(|k| (k.name(), *k)).collect();
    map(r, |r, key| {
        match key {
            "kind" => kind = Some(named(r, &kinds)?),
            "url" => url = Some(text(r)?),
            "layers" => s.layers = texts(r, "boş katman listesi yazılmaz")?,
            "subdomains" => s.subdomains = texts(r, "boş alt alan listesi yazılmaz")?,
            "params" => {
                let at = r.position();
                s.params = list(r, |r, _| {
                    let (mut name, mut value) = (None, None);
                    map(r, |r, key| {
                        match key {
                            "name" => name = Some(text(r)?),
                            "value" => value = Some(text(r)?),
                            _ => return Err(unknown(r)),
                        }
                        Ok(())
                    })?;
                    Ok(ServiceParam {
                        name: required(r, name, "name")?,
                        value: required(r, value, "value")?,
                    })
                })?;
                if s.params.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "boş parametre listesi yazılmaz"));
                }
            }
            "style" => s.style = Some(text(r)?),
            "format" => s.format = Some(text(r)?),
            "template" => s.template = Some(text(r)?),
            "version" => s.version = Some(text(r)?),
            "attribution" => s.attribution = Some(text(r)?),
            "connection" => s.connection = Some(text(r)?),
            "preset" => s.preset = Some(text(r)?),
            "matrixSet" => s.matrix_set = Some(text(r)?),
            "srid" => s.srid = Some(small(r)?),
            "tileSize" => s.tile_size = Some(small(r)?),
            "minZoom" => s.min_zoom = Some(small(r)?),
            "maxZoom" => s.max_zoom = Some(small(r)?),
            "grid" => s.grid = Some(tile_grid(r)?),
            "yFlip" => {
                s.y_flip = only_true(
                    r,
                    "yFlip false yazılmaz; satırlar üstten sayılıyorsa alan yoktur",
                )?
            }
            "transparent" => {
                s.transparent =
                    only_true(r, "transparent false yazılmaz; saydam değilse alan yoktur")?
            }
            "dynamic" => {
                s.dynamic = only_true(
                    r,
                    "dynamic false yazılmaz; karolarla isteniyorsa alan yoktur",
                )?
            }
            "opacity" => s.opacity = Some(r.float()?),
            "bbox" => s.bbox = Some(four(r, "servisin kapsamı")?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    s.kind = required(r, kind, "kind")?;
    s.url = required(r, url, "url")?;
    match s.problem() {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(s),
    }
}

/// Four numbers (a box): `what` says which when they are not four.
fn four(r: &mut Reader<'_>, what: &str) -> Result<[f64; 4], KcadError> {
    let at = r.position();
    let v = list(r, |r, _| r.float())?;
    <[f64; 4]>::try_from(v.as_slice()).map_err(|_| {
        r.fail_at(
            Code::BadValue,
            at,
            &format!("{what} 4 sayı olmalı, {} var", v.len()),
        )
    })
}

/// Where a layer's objects came from, checked whole.
pub(super) fn feature_feed(r: &mut Reader<'_>) -> Result<FeatureFeed, KcadError> {
    let at = r.position();
    let (mut kind, mut url) = (None, None);
    let mut d = FeatureFeed {
        kind: FeedKind::Geojson,
        url: String::new(),
        name: None,
        srid: None,
        filter: None,
        bbox: None,
        limit: None,
        version: None,
        key: None,
        connection: None,
        fetched: None,
    };
    let kinds: Vec<(&str, FeedKind)> = FeedKind::ALL.iter().map(|k| (k.name(), *k)).collect();
    map(r, |r, key| {
        match key {
            "kind" => kind = Some(named(r, &kinds)?),
            "url" => url = Some(text(r)?),
            "name" => d.name = Some(text(r)?),
            "filter" => d.filter = Some(text(r)?),
            "version" => d.version = Some(text(r)?),
            "key" => d.key = Some(text(r)?),
            "connection" => d.connection = Some(text(r)?),
            "fetched" => d.fetched = Some(text(r)?),
            "srid" => d.srid = Some(small(r)?),
            "limit" => d.limit = Some(r.uint(u64::MAX)?),
            "bbox" => d.bbox = Some(four(r, "istenen alan")?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    d.kind = required(r, kind, "kind")?;
    d.url = required(r, url, "url")?;
    match d.problem() {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(d),
    }
}

/// The project's connections, checked whole; an empty list is not written.
pub(super) fn connections(r: &mut Reader<'_>) -> Result<Vec<ServiceConnection>, KcadError> {
    let at = r.position();
    let auths: Vec<(&str, AuthKind)> = AuthKind::ALL.iter().map(|k| (k.name(), *k)).collect();
    let all = list(r, |r, _| {
        let (mut id, mut name, mut origin, mut auth) = (None, None, None, None);
        let mut c = ServiceConnection {
            id: String::new(),
            name: String::new(),
            origin: String::new(),
            auth: AuthKind::None,
            names: Vec::new(),
            token_url: None,
            scope: None,
        };
        map(r, |r, key| {
            match key {
                "id" => id = Some(text(r)?),
                "name" => name = Some(text(r)?),
                "origin" => origin = Some(text(r)?),
                "auth" => auth = Some(named(r, &auths)?),
                "names" => c.names = texts(r, "boş ad listesi yazılmaz")?,
                "tokenUrl" => c.token_url = Some(text(r)?),
                "scope" => c.scope = Some(text(r)?),
                _ => return Err(unknown(r)),
            }
            Ok(())
        })?;
        c.id = required(r, id, "id")?;
        c.name = required(r, name, "name")?;
        c.origin = required(r, origin, "origin")?;
        c.auth = required(r, auth, "auth")?;
        Ok(c)
    })?;
    if all.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, "boş bağlantı listesi yazılmaz"));
    }
    match connections_problem(&all) {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(all),
    }
}
