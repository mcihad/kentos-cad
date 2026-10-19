//! The assets of a STAC item as rasters (docs/adr/0528 §4–§6): each asset a single-band raster read by address, its
//! grid from the catalog (`proj:shape`, `proj:transform`, `proj:epsg`), its data type, nodata, scale and offset
//! from `raster:bands`, in a group of its own with one layer per asset; the quality (QA) band and its mask rule;
//! and the preparation record that says what was done (what was searched, which item and assets, the band
//! mapping, scale and offset, the mask, the acquisition time). Nothing is opened: a COG's header is read when the
//! raster is drawn. The command (`kentos-native-application`) writes what this plans.
//!
//! Refusals come in the order of `StacPrepare`'s doc: every asset is checked for each reason before the next
//! reason is looked at.

use std::collections::BTreeMap;

use kentos_contracts::{
    RasterFields, RasterRender, RasterSample, RasterStretch, STAC_MOST_ASSETS, STAC_RECORD_FORMAT,
    STAC_RECORD_VERSION, StacAssetInfo, StacMask, StacPreparation, StacPrepare, StacPreparedBand,
    StacPreparedQa, StacReflectance, StacSearchRecord, codes, stac_attrs,
};

use super::{Problem, endpoint, href};
use crate::query::number;

/// A warning: its code, words and the input field it is about.
pub type Warning = (&'static str, String, String);

/// One raster to add: the layer's name, the raster's fields and its attributes.
#[derive(Clone, Debug, PartialEq)]
pub struct RasterPlan {
    pub layer: String,
    pub fields: RasterFields,
    pub attrs: BTreeMap<String, String>,
}

/// What a preparation writes and answers.
#[derive(Clone, Debug, PartialEq)]
pub struct Prepared {
    /// The group's name.
    pub group: String,
    /// The data bands' rasters, in the order asked; then the quality band's.
    pub bands: Vec<RasterPlan>,
    pub qa: Option<RasterPlan>,
    /// The record, without layer and uid (the command fills them when it writes).
    pub preparation: StacPreparation,
    pub warnings: Vec<Warning>,
}

/// The longest group name.
const GROUP_MOST: usize = 120;

/// A quality asset's mask rule when none is given, by the asset's key in any case (Planetary Computer writes `SCL`,
/// Earth Search `scl`): Sentinel-2's scene classification (no data, saturated or defective, cloud shadow, medium
/// and high probability cloud, thin cirrus) and Landsat's QA_PIXEL (fill, dilated cloud, cirrus, cloud, cloud
/// shadow).
pub fn default_mask(asset_key: &str) -> Option<StacMask> {
    match asset_key.to_ascii_lowercase().as_str() {
        "scl" => Some(StacMask::Values {
            values: vec![0, 1, 3, 8, 9, 10],
        }),
        "qa_pixel" => Some(StacMask::Bits {
            bits: vec![0, 1, 2, 3, 4],
        }),
        _ => None,
    }
}

/// A mask rule as the record keeps it: sorted, once each; none when it holds nothing a QA band can hold.
pub fn normal_mask(mask: &StacMask) -> Option<StacMask> {
    match mask {
        StacMask::Values { values } => {
            let mut v = values.clone();
            v.sort_unstable();
            v.dedup();
            (!v.is_empty() && v.iter().all(|x| *x <= 65_535))
                .then_some(StacMask::Values { values: v })
        }
        StacMask::Bits { bits } => {
            let mut v = bits.clone();
            v.sort_unstable();
            v.dedup();
            (!v.is_empty() && v.iter().all(|x| *x < 32)).then_some(StacMask::Bits { bits: v })
        }
    }
}

/// A mask rule as text: `values:0,1,3` or `bits:0,1,2`.
pub fn mask_text(mask: &StacMask) -> String {
    let join = |v: &[u32]| v.iter().map(u32::to_string).collect::<Vec<_>>().join(",");
    match mask {
        StacMask::Values { values } => format!("values:{}", join(values)),
        StacMask::Bits { bits } => format!("bits:{}", join(bits)),
    }
}

/// A STAC `data_type` as a raster's sample.
pub fn sample_of(data_type: &str) -> Option<RasterSample> {
    Some(match data_type {
        "uint8" => RasterSample::U8,
        "int8" => RasterSample::I8,
        "uint16" => RasterSample::U16,
        "int16" => RasterSample::I16,
        "uint32" => RasterSample::U32,
        "int32" => RasterSample::I32,
        "float32" => RasterSample::F32,
        "float64" => RasterSample::F64,
        _ => return None,
    })
}

/// Everything about an asset a raster needs, read and checked.
struct Grid {
    sample: RasterSample,
    nodata: Option<f64>,
    scale: Option<f64>,
    offset: Option<f64>,
    width: u32,
    height: u32,
    affine: [f64; 6],
    epsg: u32,
}

fn find<'a>(input: &'a StacPrepare, key: &str) -> Option<&'a StacAssetInfo> {
    input.item.assets.iter().find(|a| a.key == key)
}

fn system_text(srid: u32) -> String {
    if srid == 0 {
        "yerel".to_owned()
    } else {
        format!("EPSG:{srid}")
    }
}

/// The asset's grid, once its sample and bands are known good.
fn grid_of(a: &StacAssetInfo, at: &str) -> Result<Grid, Problem> {
    let (Some(shape), Some(t), Some(epsg)) = (a.shape, a.transform, a.epsg) else {
        let mut missing = Vec::new();
        if a.shape.is_none() {
            missing.push("proj:shape");
        }
        if a.transform.is_none() {
            missing.push("proj:transform");
        }
        if a.epsg.is_none() {
            missing.push("proj:epsg");
        }
        return Err(Problem::new(
            codes::STAC_NO_GRID,
            format!(
                "“{}” varlığının ızgarası bilinmiyor: {} yok (varlıkta ya da öğede).",
                a.key,
                missing.join(", ")
            ),
            at,
        ));
    };
    let rb = a.raster_bands.first();
    let sample = rb
        .and_then(|b| b.data_type.as_deref())
        .and_then(sample_of)
        .ok_or_else(|| {
            let shown = rb.and_then(|b| b.data_type.clone()).unwrap_or_else(|| "yok".into());
            Problem::new(
                codes::STAC_UNSUPPORTED_SAMPLE,
                format!(
                    "“{}” varlığının veri türü ({shown}) rasterde desteklenmiyor: uint8, int8, uint16, int16, uint32, int32, float32 ve float64 desteklenir.",
                    a.key
                ),
                at,
            )
        })?;
    // x = a·column + b·row + c, y = d·column + e·row + f  →  a raster's [x₀, a, b, y₀, c, d].
    let affine = [t[2], t[0], t[1], t[5], t[3], t[4]];
    Ok(Grid {
        sample,
        nodata: rb.and_then(|b| b.nodata),
        scale: rb.and_then(|b| b.scale),
        offset: rb.and_then(|b| b.offset),
        width: shape[1],
        height: shape[0],
        affine,
        epsg,
    })
}

fn fields(a: &StacAssetInfo, g: &Grid, qa: bool, at: &str) -> Result<RasterFields, Problem> {
    let mut style = kentos_formats::raster::style::default_style(1, g.sample, false);
    style.render = RasterRender::Gray;
    style.ramp = None;
    style.stretch = match (g.sample, qa) {
        (RasterSample::U8, _) => RasterStretch::None,
        (_, true) => RasterStretch::MinMax,
        _ => RasterStretch::Percent,
    };
    style.nodata = g.nodata;
    let f = RasterFields {
        affine: g.affine,
        width: g.width,
        height: g.height,
        bands: 1,
        sample: g.sample,
        asset: None,
        file: None,
        url: Some(a.href.clone()),
        srid: g.epsg,
        style,
        opacity: None,
        dataset: None,
        georef: None,
    };
    match f.problem() {
        None => Ok(f),
        Some(why) => Err(Problem::new(
            codes::STAC_NO_GRID,
            format!("“{}” varlığı bir raster olamaz: {why}", a.key),
            at,
        )),
    }
}

fn record_text(r: &StacSearchRecord) -> String {
    let mut parts = Vec::new();
    if let Some(b) = r.bbox {
        parts.push(format!(
            "bbox={},{},{},{}",
            number(b[0]),
            number(b[1]),
            number(b[2]),
            number(b[3])
        ));
    }
    if let Some(d) = &r.datetime {
        parts.push(format!("datetime={d}"));
    }
    if !r.collections.is_empty() {
        parts.push(format!("collections={}", r.collections.join(",")));
    }
    if let Some(c) = r.max_cloud_cover {
        parts.push(format!("maxCloudCover={}", number(c)));
    }
    parts.join("; ")
}

/// The group's name, the given one checked.
fn group_of(input: &StacPrepare) -> Result<String, Problem> {
    match &input.group {
        None => Ok(format!("STAC: {}", input.item.id)),
        Some(g) => {
            let t = g.trim();
            if t.is_empty() || t.chars().count() > GROUP_MOST || t.chars().any(char::is_control) {
                return Err(Problem::new(
                    codes::INVALID_GROUP_NAME,
                    format!(
                        "Grup adı boş olmamalı, en çok {GROUP_MOST} harf olmalı ve denetim karakteri içermemeli."
                    ),
                    "group",
                ));
            }
            Ok(t.to_owned())
        }
    }
}

/// What preparing `input` in a project of system `project_srid` writes, or the first thing wrong with it.
pub fn plan(input: &StacPrepare, project_srid: u32) -> Result<Prepared, Problem> {
    let item = &input.item;
    // The item itself.
    if item.id.trim().is_empty() || item.id.chars().any(char::is_control) {
        return Err(Problem::new(
            codes::INVALID_STAC_ITEM,
            "Öğenin kimliği boş olmamalı ve denetim karakteri içermemeli.",
            "item.id",
        ));
    }
    if endpoint::normalize(&item.catalog).is_err() {
        return Err(Problem::new(
            codes::INVALID_STAC_ITEM,
            "Öğenin kataloğu (catalog) bir http ya da https adresi olmalı.",
            "item.catalog",
        ));
    }
    let mut seen: Vec<&str> = Vec::new();
    for (i, a) in item.assets.iter().enumerate() {
        if a.key.trim().is_empty()
            || a.key.chars().any(char::is_control)
            || seen.contains(&a.key.as_str())
        {
            return Err(Problem::new(
                codes::INVALID_STAC_ITEM,
                format!(
                    "Varlık anahtarı boş, tekrarlı ya da denetim karakteri içeriyor (“{}”).",
                    a.key
                ),
                &format!("item.assets[{i}].key"),
            ));
        }
        seen.push(&a.key);
    }
    // The assets asked for, each once, in the order given.
    let mut keys: Vec<(&str, String)> = Vec::new();
    for (i, k) in input.assets.iter().enumerate() {
        if !keys.iter().any(|(x, _)| *x == k.as_str()) {
            keys.push((k.as_str(), format!("assets[{i}]")));
        }
    }
    if keys.is_empty() {
        return Err(Problem::new(
            codes::STAC_ASSETS_EMPTY,
            "Eklenecek varlık verilmedi.",
            "assets",
        ));
    }
    if keys.len() > STAC_MOST_ASSETS {
        return Err(Problem::new(
            codes::STAC_TOO_MANY_ASSETS,
            format!(
                "Bir hazırlama en çok {STAC_MOST_ASSETS} varlık ekler; {} verildi.",
                keys.len()
            ),
            "assets",
        ));
    }
    // Data bands first, then the quality band: (asset, is_qa, path).
    let mut wanted: Vec<(&StacAssetInfo, bool, String)> = Vec::new();
    for (k, at) in &keys {
        let a = find(input, k).ok_or_else(|| {
            Problem::new(
                codes::STAC_ASSET_NOT_FOUND,
                format!(
                    "Öğede “{k}” anahtarlı bir varlık yok (var olanlar: {}).",
                    listed(input)
                ),
                at,
            )
        })?;
        wanted.push((a, false, at.clone()));
    }
    if let Some(q) = &input.qa {
        let a = find(input, &q.asset).ok_or_else(|| {
            Problem::new(
                codes::STAC_ASSET_NOT_FOUND,
                format!(
                    "Öğede “{}” anahtarlı bir kalite varlığı yok (var olanlar: {}).",
                    q.asset,
                    listed(input)
                ),
                "qa.asset",
            )
        })?;
        if keys.iter().any(|(k, _)| *k == q.asset.as_str()) {
            return Err(Problem::new(
                codes::STAC_QA_IS_DATA,
                format!(
                    "“{}” hem veri bandı hem kalite bandı olarak verilmiş.",
                    q.asset
                ),
                "qa.asset",
            ));
        }
        wanted.push((a, true, "qa.asset".into()));
    }
    for (a, _, at) in &wanted {
        if !href::is_http(&a.href) {
            return Err(Problem::new(
                codes::STAC_HREF_NOT_HTTP,
                format!(
                    "“{}” varlığının adresi (“{}”) http ya da https değil; raster yalnız http(s) adresinden okunur. Kataloğun http adresi veren bir kopyasını kullanın ya da dosyayı indirin.",
                    a.key, a.href
                ),
                at,
            ));
        }
    }
    for (a, _, at) in &wanted {
        if a.shape.is_none() || a.transform.is_none() || a.epsg.is_none() {
            grid_of(a, at)?;
        }
    }
    for (a, _, at) in &wanted {
        if a.raster_bands.len() > 1 || a.bands.len() > 1 {
            return Err(Problem::new(
                codes::STAC_MULTIBAND,
                format!(
                    "“{}” varlığının birden çok bandı var; her biri tek bantlı varlıklar eklenir (kırmızı, yakın kızılötesi …).",
                    a.key
                ),
                at,
            ));
        }
    }
    let mut grids: Vec<Grid> = Vec::new();
    for (a, _, at) in &wanted {
        grids.push(grid_of(a, at)?);
    }
    // The quality band's rule.
    let mask = match &input.qa {
        None => None,
        Some(q) => {
            let (given, default) = match &q.mask {
                Some(m) => (
                    Some(normal_mask(m).ok_or_else(|| {
                        Problem::new(
                            codes::STAC_QA_UNKNOWN,
                            "Maske kuralı geçersiz: boş olmamalı; sınıf değerleri 0–65535, bit numaraları 0–31.",
                            "qa.mask",
                        )
                    })?),
                    false,
                ),
                None => (default_mask(&q.asset), true),
            };
            let m = given.ok_or_else(|| {
                Problem::new(
                    codes::STAC_QA_UNKNOWN,
                    format!(
                        "“{}” kalite varlığı için maske kuralı bilinmiyor: Sentinel-2'nin scl'si ve Landsat'ın qa_pixel'i dışında bir varlık için mask verin.",
                        q.asset
                    ),
                    "qa.mask",
                )
            })?;
            Some((m, default))
        }
    };
    for ((a, _, at), g) in wanted.iter().zip(&grids) {
        if g.epsg != project_srid {
            return Err(Problem::new(
                codes::CRS_MISMATCH,
                format!(
                    "“{}” varlığının sistemi {}, projeninki {}; raster çevrilmeden eklenmez (dönüşüm yok).",
                    a.key,
                    system_text(g.epsg),
                    system_text(project_srid)
                ),
                at,
            ));
        }
    }
    let group = group_of(input)?;
    // The rasters.
    let common = |a: &StacAssetInfo, role: &str| -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert(stac_attrs::CATALOG.into(), item.catalog.clone());
        if let Some(c) = &item.collection {
            m.insert(stac_attrs::COLLECTION.into(), c.clone());
        }
        m.insert(stac_attrs::ITEM.into(), item.id.clone());
        m.insert(stac_attrs::ASSET.into(), a.key.clone());
        m.insert(stac_attrs::HREF.into(), a.href.clone());
        m.insert(stac_attrs::ROLE.into(), role.into());
        if let Some(d) = &item.datetime {
            m.insert(stac_attrs::DATETIME.into(), d.clone());
        }
        if let Some(c) = item.cloud_cover {
            m.insert(stac_attrs::CLOUD_COVER.into(), number(c));
        }
        if let Some(p) = &item.platform {
            m.insert(stac_attrs::PLATFORM.into(), p.clone());
        }
        if let Some(l) = item.license.as_ref().or(item.license_href.as_ref()) {
            m.insert(stac_attrs::LICENSE.into(), l.clone());
        }
        if let Some(s) = &input.search {
            let text = record_text(s);
            if !text.is_empty() {
                m.insert(stac_attrs::SEARCH.into(), text);
            }
        }
        m
    };
    let mut bands = Vec::new();
    let mut prepared_bands = Vec::new();
    let mut qa_plan = None;
    let mut prepared_qa = None;
    for ((a, is_qa, at), g) in wanted.iter().zip(&grids) {
        let f = fields(a, g, *is_qa, at)?;
        let mut attrs = common(a, if *is_qa { "qa" } else { "data" });
        if let Some(n) = g.nodata {
            attrs.insert(stac_attrs::NODATA.into(), number(n));
        }
        if *is_qa {
            let (m, default) = mask
                .clone()
                .unwrap_or((StacMask::Values { values: vec![] }, false));
            attrs.insert(stac_attrs::QA_MASK.into(), mask_text(&m));
            qa_plan = Some(RasterPlan {
                layer: a.key.clone(),
                fields: f,
                attrs,
            });
            prepared_qa = Some(StacPreparedQa {
                asset: a.key.clone(),
                href: a.href.clone(),
                signed: a.signed,
                sample: g.sample,
                nodata: g.nodata,
                mask: m,
                default_mask: default,
                width: g.width,
                height: g.height,
                affine: g.affine,
                layer: None,
                uid: None,
            });
        } else {
            let (scale, offset) = (g.scale.unwrap_or(1.0), g.offset.unwrap_or(0.0));
            let common_name = a.bands.first().and_then(|b| b.common_name.clone());
            if let Some(c) = &common_name {
                attrs.insert(stac_attrs::BAND.into(), c.clone());
            }
            attrs.insert(stac_attrs::SCALE.into(), number(scale));
            attrs.insert(stac_attrs::OFFSET.into(), number(offset));
            bands.push(RasterPlan {
                layer: a.key.clone(),
                fields: f,
                attrs,
            });
            prepared_bands.push(StacPreparedBand {
                asset: a.key.clone(),
                common_name,
                href: a.href.clone(),
                signed: a.signed,
                sample: g.sample,
                nodata: g.nodata,
                scale,
                offset,
                width: g.width,
                height: g.height,
                affine: g.affine,
                epsg: g.epsg,
                layer: None,
                uid: None,
            });
        }
    }
    // Warnings.
    let mut warnings: Vec<Warning> = Vec::new();
    let signed: Vec<&str> = wanted
        .iter()
        .filter(|(a, _, _)| a.signed)
        .map(|(a, _, _)| a.key.as_str())
        .collect();
    if !signed.is_empty() {
        warnings.push((
            codes::STAC_SIGNED_HREF_STRIPPED,
            format!(
                "{} varlığının adresindeki imza (geçici erişim anahtarı) projeye yazılmadı; dosyayı okumak için imza gerekiyorsa raster açılmaz.",
                signed.join(", ")
            ),
            "assets".into(),
        ));
    }
    if item.datetime.is_none() || item.cloud_cover.is_none() {
        warnings.push((
            codes::STAC_ITEM_INCOMPLETE,
            "Öğenin çekim zamanı ya da bulut oranı yok; kayıt bunları boş taşır.".into(),
            "item".into(),
        ));
    }
    if let Some(first) = prepared_bands.first() {
        let same_grid = |w: u32, h: u32, af: &[f64; 6]| {
            w == first.width && h == first.height && *af == first.affine
        };
        if !prepared_bands
            .iter()
            .all(|b| same_grid(b.width, b.height, &b.affine))
        {
            warnings.push((
                codes::STAC_GRID_DIFFERS,
                "Bantların ızgarası (piksel boyu ya da yeri) aynı değil; birlikte işlemeden önce ortak ızgaraya örneklenmeli.".into(),
                "assets".into(),
            ));
        }
        if !prepared_bands
            .iter()
            .all(|b| b.scale == first.scale && b.offset == first.offset)
        {
            warnings.push((
                codes::STAC_SCALE_DIFFERS,
                "Bantların ölçeği ya da ötelemesi aynı değil; Spektral indis tek ölçek ve öteleme alır, bantlar ayrı ayrı yansımaya çevrilmeli.".into(),
                "assets".into(),
            ));
        }
        if let Some(q) = &prepared_qa
            && !same_grid(q.width, q.height, &q.affine)
        {
            warnings.push((
                codes::STAC_QA_GRID_DIFFERS,
                format!(
                    "Kalite bandının ({}) ızgarası veri bantlarınınkinden başka; maske uygulanmadan önce örneklenmeli.",
                    q.asset
                ),
                "qa.asset".into(),
            ));
        }
    }
    // Reflectance: every data band is one and they share a scale and an offset.
    let reflectance = prepared_bands.first().and_then(|first| {
        let all = wanted
            .iter()
            .filter(|(_, qa, _)| !*qa)
            .all(|(a, _, _)| a.roles.iter().any(|r| r == "reflectance"));
        let same = prepared_bands
            .iter()
            .all(|b| b.scale == first.scale && b.offset == first.offset);
        (all && same).then_some(StacReflectance {
            scale: first.scale,
            offset: first.offset,
        })
    });
    let preparation = StacPreparation {
        format: STAC_RECORD_FORMAT.into(),
        version: STAC_RECORD_VERSION,
        search: input.search.clone(),
        catalog: item.catalog.clone(),
        collection: item.collection.clone(),
        item: item.id.clone(),
        datetime: item.datetime.clone(),
        cloud_cover: item.cloud_cover,
        platform: item.platform.clone(),
        license: item.license.clone(),
        license_href: item.license_href.clone(),
        bands: prepared_bands,
        qa: prepared_qa,
        reflectance,
    };
    Ok(Prepared {
        group,
        bands,
        qa: qa_plan,
        preparation,
        warnings,
    })
}

fn listed(input: &StacPrepare) -> String {
    let keys: Vec<&str> = input.item.assets.iter().map(|a| a.key.as_str()).collect();
    if keys.is_empty() {
        "hiçbiri".into()
    } else {
        keys.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_contracts::{StacBand, StacItemInfo, StacQa, StacRasterBand};

    fn asset(key: &str, common: &str, scale: f64, offset: f64) -> StacAssetInfo {
        StacAssetInfo {
            key: key.into(),
            href: format!("https://b.example/{key}.tif"),
            signed: false,
            title: None,
            media_type: None,
            roles: vec!["data".into(), "reflectance".into()],
            bands: vec![StacBand {
                name: None,
                common_name: Some(common.into()),
                center_wavelength: None,
            }],
            raster_bands: vec![StacRasterBand {
                data_type: Some("uint16".into()),
                nodata: Some(0.0),
                scale: Some(scale),
                offset: Some(offset),
                spatial_resolution: None,
                unit: None,
            }],
            epsg: Some(32637),
            shape: Some([100, 200]),
            transform: Some([10.0, 0.0, 300_000.0, 0.0, -10.0, 4_400_040.0]),
        }
    }

    fn scl() -> StacAssetInfo {
        StacAssetInfo {
            key: "scl".into(),
            roles: vec!["data".into()],
            bands: vec![],
            raster_bands: vec![StacRasterBand {
                data_type: Some("uint8".into()),
                nodata: Some(0.0),
                scale: None,
                offset: None,
                spatial_resolution: None,
                unit: None,
            }],
            shape: Some([50, 100]),
            transform: Some([20.0, 0.0, 300_000.0, 0.0, -20.0, 4_400_040.0]),
            ..asset("scl", "x", 1.0, 0.0)
        }
    }

    fn input() -> StacPrepare {
        StacPrepare {
            item: StacItemInfo {
                catalog: "https://x.example/v1".into(),
                collection: Some("sentinel-2-l2a".into()),
                id: "S2A_37SCD_20240713_0_L2A".into(),
                datetime: Some("2024-07-13T08:29:15.388000Z".into()),
                end_datetime: None,
                bbox: None,
                cloud_cover: Some(3.13),
                platform: Some("sentinel-2a".into()),
                instruments: vec![],
                gsd: None,
                epsg: Some(32637),
                license: None,
                license_href: None,
                self_href: None,
                assets: vec![
                    asset("red", "red", 0.0001, -0.1),
                    asset("nir", "nir", 0.0001, -0.1),
                    scl(),
                ],
            },
            assets: vec!["red".into(), "nir".into()],
            qa: Some(StacQa {
                asset: "scl".into(),
                mask: None,
            }),
            search: None,
            group: None,
            expected_revision: None,
        }
    }

    #[test]
    fn two_bands_and_the_scene_classification() {
        let p = plan(&input(), 32637).unwrap();
        assert_eq!(p.group, "STAC: S2A_37SCD_20240713_0_L2A");
        assert_eq!(p.bands.len(), 2);
        let red = &p.bands[0];
        assert_eq!(red.layer, "red");
        assert_eq!(
            red.fields.affine,
            [300_000.0, 10.0, 0.0, 4_400_040.0, 0.0, -10.0]
        );
        assert_eq!((red.fields.width, red.fields.height), (200, 100));
        assert_eq!(red.fields.url.as_deref(), Some("https://b.example/red.tif"));
        assert_eq!(red.fields.sample, RasterSample::U16);
        assert_eq!(red.attrs["stac.scale"], "0.0001");
        assert_eq!(red.attrs["stac.offset"], "-0.1");
        assert_eq!(red.attrs["stac.band"], "red");
        let qa = p.qa.unwrap();
        assert_eq!(qa.attrs["stac.qa_mask"], "values:0,1,3,8,9,10");
        assert_eq!(qa.attrs["stac.role"], "qa");
        let r = p.preparation.reflectance.unwrap();
        assert_eq!((r.scale, r.offset), (0.0001, -0.1));
        let got: Vec<&str> = p.warnings.iter().map(|w| w.0).collect();
        assert_eq!(got, [codes::STAC_QA_GRID_DIFFERS]);
        assert!(p.preparation.qa.unwrap().default_mask);
    }

    #[test]
    fn refusals_come_in_their_order() {
        let mut i = input();
        i.assets = vec![];
        assert_eq!(plan(&i, 32637).unwrap_err().code, codes::STAC_ASSETS_EMPTY);
        i.assets = vec!["yok".into()];
        assert_eq!(
            plan(&i, 32637).unwrap_err().code,
            codes::STAC_ASSET_NOT_FOUND
        );
        i.assets = vec!["red".into(), "scl".into()];
        assert_eq!(plan(&i, 32637).unwrap_err().code, codes::STAC_QA_IS_DATA);
        i.assets = vec!["red".into()];
        i.item.assets[0].href = "s3://b/red.tif".into();
        assert_eq!(plan(&i, 32637).unwrap_err().code, codes::STAC_HREF_NOT_HTTP);
        i.item.assets[0].href = "https://b/red.tif".into();
        i.item.assets[0].transform = None;
        assert_eq!(plan(&i, 32637).unwrap_err().code, codes::STAC_NO_GRID);
        i.item.assets[0].transform = Some([10.0, 0.0, 0.0, 0.0, -10.0, 0.0]);
        i.item.assets[0].raster_bands[0].data_type = Some("cint16".into());
        assert_eq!(
            plan(&i, 32637).unwrap_err().code,
            codes::STAC_UNSUPPORTED_SAMPLE
        );
        i.item.assets[0].raster_bands[0].data_type = Some("uint16".into());
        i.item.assets[0].bands.push(StacBand {
            name: None,
            common_name: None,
            center_wavelength: None,
        });
        assert_eq!(plan(&i, 32637).unwrap_err().code, codes::STAC_MULTIBAND);
        i.item.assets[0].bands.pop();
        assert_eq!(plan(&i, 5254).unwrap_err().code, codes::CRS_MISMATCH);
        i.qa = Some(StacQa {
            asset: "nir".into(),
            mask: None,
        });
        assert_eq!(plan(&i, 32637).unwrap_err().code, codes::STAC_QA_UNKNOWN);
        i.qa = None;
        i.group = Some("  ".into());
        assert_eq!(plan(&i, 32637).unwrap_err().code, codes::INVALID_GROUP_NAME);
    }

    #[test]
    fn signed_addresses_and_differing_scales_are_warned() {
        let mut i = input();
        i.qa = None;
        i.item.assets[0].signed = true;
        i.item.assets[1].raster_bands[0].offset = Some(0.0);
        let p = plan(&i, 32637).unwrap();
        let got: Vec<&str> = p.warnings.iter().map(|w| w.0).collect();
        assert_eq!(
            got,
            [codes::STAC_SIGNED_HREF_STRIPPED, codes::STAC_SCALE_DIFFERS]
        );
        assert!(p.preparation.reflectance.is_none());
    }

    #[test]
    fn masks_are_normal() {
        assert_eq!(
            normal_mask(&StacMask::Values {
                values: vec![9, 3, 9]
            }),
            Some(StacMask::Values { values: vec![3, 9] })
        );
        assert_eq!(normal_mask(&StacMask::Bits { bits: vec![32] }), None);
        assert_eq!(normal_mask(&StacMask::Values { values: vec![] }), None);
        assert_eq!(mask_text(&StacMask::Bits { bits: vec![0, 4] }), "bits:0,4");
    }
}
