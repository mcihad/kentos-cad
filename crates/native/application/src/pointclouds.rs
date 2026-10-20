//! A point cloud's source metadata and its points by a lasting id (docs/adr/0536): `cad.pointclouds.describe` reads
//! each source's header (and, with `classes`, every record), `cad.pointclouds.query` the point nearest a place in plan
//! at the source's full resolution. Both read the files themselves with the point cloud core (a file where the cloud
//! names it, looked for as the project's other links are; a library asset); an address is not reached here.

use std::collections::BTreeMap;

use kentos_commands::{
    CloudDescribed, CloudExtra, CloudFormat, CloudMismatch, CloudPoint, CloudSource,
    CloudSourceDescribed, CloudSummary, CommandResult, CommandWarning, Entity, POINTCLOUD_MOST_COUNTED, PointCloudEntity,
    PointCloudsDescribe, PointCloudsDescribed, PointCloudsQueried, PointCloudsQuery,
};
use kentos_domain::Document;
use kentos_pointcloud::Step;
use kentos_pointcloud::copc::Key;
use kentos_pointcloud::las::{Header, Vlr};
use kentos_pointcloud::ops::query::Facts;
use kentos_pointcloud::source::{Cloud, Opening, Run};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::source_bytes::{Bytes, bytes_for};

/// A source's bytes: its library asset, or its file found as the project's links are; why not.
pub(crate) fn bytes_of(
    doc: &Document,
    drawing: Option<&str>,
    s: &CloudSource,
) -> Result<Bytes, String> {
    if s.asset.is_none() && s.file.is_none() && s.url.is_none() {
        return Err("bulutun kaynağı yok".into());
    }
    bytes_for(
        doc,
        drawing,
        s.asset.as_deref(),
        s.file.as_deref(),
        s.url.as_deref(),
        "adresteki bulut burada okunmaz; masaüstünün Nokta bulutu penceresi adresten okur",
    )
}

/// The cloud of `bytes`: its header, VLRs, chunks or COPC hierarchy.
pub(crate) fn open(bytes: &Bytes) -> Result<Cloud, String> {
    let mut o = Opening::new(bytes.size());
    loop {
        match o.step().map_err(|e| e.0)? {
            Step::Done(c) => return Ok(c),
            Step::Need(n) => o.put(n.offset, bytes.read(n.offset, n.len)?),
        }
    }
}

/// A run's records.
pub(crate) fn records(cloud: &Cloud, bytes: &Bytes, run: &Run, out: &mut Vec<u8>) -> Result<(), String> {
    let raw = bytes.read(run.need.offset, run.need.len)?;
    out.clear();
    cloud.records(run, &raw, out).map_err(|e| e.0)
}

/// The dimensions of a point format, in the LAS specification's order, its names camel cased.
fn dimensions(format: u8) -> Vec<String> {
    let legacy = [
        "x",
        "y",
        "z",
        "intensity",
        "returnNumber",
        "numberOfReturns",
        "scanDirectionFlag",
        "edgeOfFlightLine",
        "classification",
        "scanAngleRank",
        "userData",
        "pointSourceId",
    ];
    let wide = [
        "x",
        "y",
        "z",
        "intensity",
        "returnNumber",
        "numberOfReturns",
        "classificationFlags",
        "scannerChannel",
        "scanDirectionFlag",
        "edgeOfFlightLine",
        "classification",
        "userData",
        "scanAngle",
        "pointSourceId",
        "gpsTime",
    ];
    let mut out: Vec<&str> = if format >= 6 {
        wide.to_vec()
    } else {
        legacy.to_vec()
    };
    if matches!(format, 1 | 3 | 4 | 5) {
        out.push("gpsTime");
    }
    if matches!(format, 2 | 3 | 5 | 7 | 8 | 10) {
        out.extend(["red", "green", "blue"]);
    }
    if matches!(format, 8 | 10) {
        out.push("nir");
    }
    if matches!(format, 4 | 5 | 9 | 10) {
        out.extend([
            "wavePacketDescriptorIndex",
            "byteOffsetToWaveformData",
            "waveformPacketSize",
            "returnPointWaveformLocation",
            "xT",
            "yT",
            "zT",
        ]);
    }
    out.into_iter().map(str::to_owned).collect()
}

/// The extra bytes' dimensions (LASF_Spec 4's 192-byte descriptors).
fn extras(cloud: &Cloud) -> Vec<CloudExtra> {
    let Some(v) = cloud
        .vlrs
        .iter()
        .chain(&cloud.evlrs)
        .find(|v| v.user == "LASF_Spec" && v.record == 4)
    else {
        return Vec::new();
    };
    let base = |t: u8| -> Option<(&'static str, u32)> {
        Some(match t {
            1 => ("u8", 1),
            2 => ("i8", 1),
            3 => ("u16", 2),
            4 => ("i16", 2),
            5 => ("u32", 4),
            6 => ("i32", 4),
            7 => ("u64", 8),
            8 => ("i64", 8),
            9 => ("f32", 4),
            10 => ("f64", 8),
            _ => return None,
        })
    };
    v.data
        .chunks_exact(192)
        .map(|d| {
            let (t, options) = (d[2], d[3]);
            let name = String::from_utf8_lossy(&d[4..36])
                .trim_end_matches('\0')
                .to_owned();
            let (kind, size) = match t {
                0 => ("bytes".to_owned(), u32::from(options)),
                1..=10 => base(t).map_or(("bytes".to_owned(), 0), |(k, n)| (k.to_owned(), n)),
                11..=20 => base(t - 10)
                    .map_or(("bytes".to_owned(), 0), |(k, n)| (format!("{k}[2]"), 2 * n)),
                _ => base(t - 20)
                    .map_or(("bytes".to_owned(), 0), |(k, n)| (format!("{k}[3]"), 3 * n)),
            };
            CloudExtra { name, kind, size }
        })
        .collect()
}

/// The vertical system the file names: a WKT's `VERT_CS`/`VERTCRS` name, else a GeoTIFF key 4096's `EPSG:n`.
fn vertical(cloud: &Cloud) -> Option<String> {
    if let Some(w) = &cloud.crs.wkt {
        for tag in ["VERT_CS[\"", "VERTCRS[\"", "VERTICALCRS[\""] {
            if let Some(i) = w.find(tag) {
                let rest = &w[i + tag.len()..];
                if let Some(end) = rest.find('"') {
                    return Some(rest[..end].to_owned());
                }
            }
        }
    }
    let dir: &Vlr = cloud.crs.geokeys.iter().find(|v| v.record == 34735)?;
    let words: Vec<u16> = dir
        .data
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();
    let keys = usize::from(*words.get(3)?);
    for k in 0..keys {
        let e = words.get(4 + 4 * k..8 + 4 * k)?;
        if e[0] == 4096 && e[1] == 0 && e[3] != 0 && e[3] != 32767 {
            return Some(format!("EPSG:{}", e[3]));
        }
    }
    None
}

/// The header's creation day as `YYYY-MM-DD`.
fn created(h: &Header) -> Option<String> {
    if h.year == 0 || h.day == 0 {
        return None;
    }
    let leap = (h.year.is_multiple_of(4) && !h.year.is_multiple_of(100)) || h.year.is_multiple_of(400);
    let months = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut day = h.day;
    for (m, n) in months.iter().enumerate() {
        if day <= *n {
            return Some(format!("{:04}-{:02}-{:02}", h.year, m + 1, day));
        }
        day -= n;
    }
    None
}

/// The header's GUID, bytes as a GUID's little-endian fields.
fn guid(g: &[u8; 16]) -> Option<String> {
    if g.iter().all(|b| *b == 0) {
        return None;
    }
    let d1 = u32::from_le_bytes([g[0], g[1], g[2], g[3]]);
    let d2 = u16::from_le_bytes([g[4], g[5]]);
    let d3 = u16::from_le_bytes([g[6], g[7]]);
    Some(format!(
        "{d1:08x}-{d2:04x}-{d3:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        g[8], g[9], g[10], g[11], g[12], g[13], g[14], g[15]
    ))
}

fn text_of(s: &str) -> Option<String> {
    let t = s.trim_end_matches('\0').trim();
    (!t.is_empty()).then(|| t.to_owned())
}

/// One source described, its classes counted when asked; why it could not be read.
fn describe_source(
    doc: &Document,
    drawing: Option<&str>,
    s: &CloudSource,
    classes: bool,
) -> CloudSourceDescribed {
    let mut out = CloudSourceDescribed {
        file: s.file.clone(),
        url: s.url.clone(),
        asset: s.asset.clone(),
        count: s.count,
        bounds: s.bounds,
        ..CloudSourceDescribed::default()
    };
    if s.format == CloudFormat::Xyz {
        // A text cloud has no header: its count and bounds are the cloud's record.
        out.kind = "text".into();
        out.read = true;
        out.dimensions = ["x", "y", "z"].map(str::to_owned).to_vec();
        return out;
    }
    let read = (|| -> Result<(Cloud, Bytes), String> {
        let bytes = bytes_of(doc, drawing, s)?;
        Ok((open(&bytes)?, bytes))
    })();
    let (cloud, bytes) = match read {
        Ok(x) => x,
        Err(why) => {
            out.kind = match s.format {
                CloudFormat::Las => "las",
                CloudFormat::Laz => "laz",
                CloudFormat::Copc => "copc",
                CloudFormat::Xyz => "text",
            }
            .into();
            out.why = Some(why);
            return out;
        }
    };
    let h = &cloud.head;
    out.read = true;
    out.kind = cloud.kind.name().into();
    out.version = Some(format!("{}.{}", h.major, h.minor));
    out.format = Some(h.format);
    out.record_len = Some(h.record_len);
    out.count = h.count;
    out.bounds = cloud.bounds();
    out.scale = Some(h.scale);
    out.offset = Some(h.offset);
    out.epsg = cloud.crs.epsg;
    out.crs_name = cloud.crs.name.clone();
    out.vertical = vertical(&cloud);
    out.created = created(h);
    out.gps_time = cloud.layout.gps.map(|_| {
        if h.global_encoding & 1 == 1 {
            "standard"
        } else {
            "week"
        }
        .to_owned()
    });
    out.system = text_of(&h.system);
    out.software = text_of(&h.software);
    out.source_id = Some(h.source_id);
    out.guid = guid(&h.guid);
    out.dimensions = dimensions(h.format & 0x3f);
    out.extra = extras(&cloud);
    let last = h
        .by_return
        .iter()
        .rposition(|n| *n > 0)
        .map_or(0, |i| i + 1);
    out.by_return = h.by_return[..last].to_vec();
    out.nodes = cloud.hierarchy.as_ref().map(|hi| hi.nodes.len() as u32);
    if classes {
        let mut counts: BTreeMap<String, u64> = BTreeMap::new();
        let mut recs = Vec::new();
        let len = cloud.layout.len;
        for run in cloud.runs() {
            if let Err(why) = records(&cloud, &bytes, &run, &mut recs) {
                out.why = Some(format!("noktaları okunamadı ({why})"));
                return out;
            }
            for r in recs.chunks_exact(len) {
                *counts.entry(cloud.layout.class(r).to_string()).or_default() += 1;
            }
        }
        out.classes = Some(counts);
    }
    out
}

/// A source described as `cad.pointclouds.describe` says it, classes not counted (Öznitelikler's rows).
pub fn source_described(
    doc: &Document,
    drawing: Option<&str>,
    s: &CloudSource,
) -> CloudSourceDescribed {
    describe_source(doc, drawing, s, false)
}

/// A cloud's sources summed up as `cad.pointclouds.describe` says it (docs/adr/0552): none for fewer than two
/// sources (Öznitelikler's rows).
pub fn summary_described(
    doc: &Document,
    drawing: Option<&str>,
    sources: &[CloudSource],
) -> Option<CloudSummary> {
    let described: Vec<CloudSourceDescribed> = sources
        .iter()
        .map(|s| describe_source(doc, drawing, s, false))
        .collect();
    summary_of(sources, &described)
}

/// The point clouds named (each once, in the input's order), or every one of the drawing in its order.
pub(crate) fn clouds<'a>(
    doc: &'a Document,
    uids: &'a [String],
) -> Result<Vec<(String, &'a PointCloudEntity)>, Stop> {
    if uids.is_empty() {
        return Ok(doc
            .entities_with_uids()
            .filter_map(|(u, e)| match e {
                Entity::PointCloud(c) => Some((u.to_string(), c)),
                _ => None,
            })
            .collect());
    }
    let named = checks::named(doc, uids)?;
    named
        .into_iter()
        .map(|(at, _, e, uid)| match e {
            Entity::PointCloud(c) => Ok((uid.clone(), c)),
            _ => Err(Stop::failed(checks::error(
                codes::NOT_A_POINTCLOUD,
                "Verilen nesne bir nokta bulutu değil; bir nokta bulutunun kimliğini verin."
                    .to_owned(),
                Some(format!("uids[{at}]")),
            ))),
        })
        .collect()
}

pub(crate) fn uids_checked(uids: &[String]) -> Result<(), Stop> {
    if uids.is_empty() {
        return Ok(());
    }
    checks::uids(uids, "")
}

/// What `cad.pointclouds.describe` says; why not.
pub(crate) fn describe_in(
    doc: &Document,
    input: &PointCloudsDescribe,
) -> Result<(PointCloudsDescribed, Vec<CommandWarning>), Stop> {
    uids_checked(&input.uids)?;
    let chosen = clouds(doc, &input.uids)?;
    if input.classes {
        let total: u64 = chosen.iter().map(|(_, c)| c.cloud.count).sum();
        if total > POINTCLOUD_MOST_COUNTED {
            return Err(Stop::failed(checks::error(
                codes::TOO_MANY_CLOUD_POINTS,
                format!(
                    "Sınıflar sayılırken {total} nokta okunurdu; bir çağrı en çok {POINTCLOUD_MOST_COUNTED} nokta sayar. Bulutları ayrı ayrı sorun."
                ),
                Some("classes".into()),
            )));
        }
    }
    let drawing = input.drawing.as_deref();
    let mut warnings = Vec::new();
    let mut out = Vec::new();
    for (uid, c) in chosen {
        let sources: Vec<CloudSourceDescribed> = c
            .cloud
            .sources
            .iter()
            .map(|s| describe_source(doc, drawing, s, input.classes))
            .collect();
        for (i, s) in sources.iter().enumerate() {
            if let Some(why) = &s.why {
                warnings.push(CommandWarning {
                    code: codes::CLOUD_SOURCE_UNREADABLE.into(),
                    message: format!("Bulutun {}. kaynağı okunamadı: {why}.", i + 1),
                    path: Some(format!("clouds[{}].sources[{i}]", out.len())),
                });
            }
        }
        out.push(CloudDescribed {
            uid,
            layer: c.base.layer_id.clone(),
            srid: c.cloud.srid,
            count: c.cloud.count,
            summary: summary_of(&c.cloud.sources, &sources),
            sources,
        });
    }
    Ok((
        PointCloudsDescribed {
            clouds: out,
            revision: doc.revision().to_string(),
        },
        warnings,
    ))
}

/// The sources of a cloud of several files summed up (docs/adr/0552): their points and box together, the system they
/// share, and where they disagree: system, point format, GPS time, scale, extra dimensions; a header that disagrees
/// with the cloud's record of it (points, box); two files whose boxes cover the same ground (at most 50 pairs).
fn summary_of(recorded: &[CloudSource], sources: &[CloudSourceDescribed]) -> Option<CloudSummary> {
    if sources.len() < 2 {
        return None;
    }
    let mut bounds = sources[0].bounds;
    for s in &sources[1..] {
        for k in 0..3 {
            bounds[k] = bounds[k].min(s.bounds[k]);
            bounds[k + 3] = bounds[k + 3].max(s.bounds[k + 3]);
        }
    }
    // The sources that have a header to compare (a text cloud has none).
    let headed: Vec<usize> = (0..sources.len())
        .filter(|&i| sources[i].read && sources[i].kind != "text")
        .collect();
    let mut mismatches = Vec::new();
    let mut epsg = None;
    if !headed.is_empty() {
        let first = sources[headed[0]].epsg;
        if headed.iter().all(|&i| sources[i].epsg == first) {
            epsg = first;
        }
    }
    if headed.len() >= 2 {
        let mut differ = |kind: &str, what: &str, values: Vec<String>| {
            if values.iter().any(|v| *v != values[0]) {
                let said: Vec<String> = headed
                    .iter()
                    .zip(&values)
                    .map(|(i, v)| format!("{}. kaynak {v}", i + 1))
                    .collect();
                mismatches.push(CloudMismatch {
                    kind: kind.to_owned(),
                    sources: headed.iter().map(|&i| i as u32).collect(),
                    message: format!("Kaynakların {what} aynı değil: {}.", said.join(", ")),
                });
            }
        };
        let none = || "yok".to_owned();
        differ(
            "epsg",
            "yatay sistemi",
            headed
                .iter()
                .map(|&i| sources[i].epsg.map_or_else(none, |e| format!("EPSG:{e}")))
                .collect(),
        );
        differ(
            "vertical",
            "düşey sistemi",
            headed
                .iter()
                .map(|&i| sources[i].vertical.clone().unwrap_or_else(none))
                .collect(),
        );
        differ(
            "point_format",
            "nokta biçimi",
            headed
                .iter()
                .map(|&i| sources[i].format.map_or_else(none, |f| f.to_string()))
                .collect(),
        );
        differ(
            "gps_time",
            "GPS zamanı",
            headed
                .iter()
                .map(|&i| sources[i].gps_time.clone().unwrap_or_else(none))
                .collect(),
        );
        differ(
            "scale",
            "ölçeği",
            headed
                .iter()
                .map(|&i| {
                    sources[i].scale.map_or_else(none, |s| {
                        format!("{} {} {}", s[0], s[1], s[2])
                    })
                })
                .collect(),
        );
        differ(
            "extra",
            "ek boyutları",
            headed
                .iter()
                .map(|&i| {
                    let names: Vec<&str> = sources[i].extra.iter().map(|e| e.name.as_str()).collect();
                    if names.is_empty() {
                        none()
                    } else {
                        names.join("/")
                    }
                })
                .collect(),
        );
    }
    let count_off: Vec<usize> = headed
        .iter()
        .copied()
        .filter(|&i| recorded.get(i).is_some_and(|r| r.count != sources[i].count))
        .collect();
    if !count_off.is_empty() {
        let said: Vec<String> = count_off
            .iter()
            .map(|&i| {
                format!(
                    "{}. kaynak (kayıtta {}, başlıkta {})",
                    i + 1,
                    recorded[i].count,
                    sources[i].count
                )
            })
            .collect();
        mismatches.push(CloudMismatch {
            kind: "count".into(),
            sources: count_off.iter().map(|&i| i as u32).collect(),
            message: format!(
                "Bulutun kaydındaki nokta sayısı kaynağın başlığındakinden farklı: {}.",
                said.join(", ")
            ),
        });
    }
    let bounds_off: Vec<usize> = headed
        .iter()
        .copied()
        .filter(|&i| recorded.get(i).is_some_and(|r| r.bounds != sources[i].bounds))
        .collect();
    if !bounds_off.is_empty() {
        let said: Vec<String> = bounds_off.iter().map(|i| format!("{}. kaynak", i + 1)).collect();
        mismatches.push(CloudMismatch {
            kind: "bounds".into(),
            sources: bounds_off.iter().map(|&i| i as u32).collect(),
            message: format!(
                "Bulutun kaydındaki kutu kaynağın başlığındakinden farklı: {}.",
                said.join(", ")
            ),
        });
    }
    let mut pairs = 0;
    'pairs: for (a, &i) in headed.iter().enumerate() {
        for &j in &headed[a + 1..] {
            let (p, q) = (sources[i].bounds, sources[j].bounds);
            if p[0].max(q[0]) < p[3].min(q[3]) && p[1].max(q[1]) < p[4].min(q[4]) {
                mismatches.push(CloudMismatch {
                    kind: "overlap".into(),
                    sources: vec![i as u32, j as u32],
                    message: format!(
                        "{}. ve {}. kaynağın kutuları aynı yeri kaplıyor.",
                        i + 1,
                        j + 1
                    ),
                });
                pairs += 1;
                if pairs == 50 {
                    break 'pairs;
                }
            }
        }
    }
    Some(CloudSummary {
        sources: sources.len() as u32,
        readable: sources.iter().filter(|s| s.read).count() as u32,
        count: sources.iter().map(|s| s.count).sum(),
        bounds,
        epsg,
        mismatches,
    })
}

/// The point nearest `input.at` in plan within its reach; why not.
pub(crate) fn query_in(
    doc: &Document,
    input: &PointCloudsQuery,
) -> Result<(PointCloudsQueried, Vec<CommandWarning>), Stop> {
    checks::point(input.at, "Aranan yerin", "at")?;
    checks::finite(
        input.radius,
        "Erim",
        "Sıfırdan büyük bir uzaklık verin.",
        "radius",
    )?;
    if input.radius <= 0.0 {
        return Err(Stop::failed(checks::error(
            codes::INVALID_REACH,
            "Aramanın erimi sıfırdan büyük olmalı; noktanın en çok ne kadar uzakta aranacağını verin.".into(),
            Some("radius".into()),
        )));
    }
    uids_checked(&input.uids)?;
    let mut chosen = clouds(doc, &input.uids)?;
    if input.uids.is_empty() {
        chosen.retain(|(_, c)| doc.layers().is_visible(&c.base.layer_id));
    }
    let (at, reach) = ([input.at.x, input.at.y], input.radius);
    let r2 = reach * reach;
    let drawing = input.drawing.as_deref();
    let mut warnings = Vec::new();
    let mut best: Option<(f64, CloudPoint)> = None;
    let mut read = 0u64;
    let mut recs = Vec::new();
    for (k, (uid, c)) in chosen.iter().enumerate() {
        for (si, s) in c.cloud.sources.iter().enumerate() {
            let b = s.bounds;
            if at[0] + reach < b[0]
                || at[0] - reach > b[3]
                || at[1] + reach < b[1]
                || at[1] - reach > b[4]
            {
                continue;
            }
            if s.format == CloudFormat::Xyz {
                warnings.push(CommandWarning {
                    code: codes::CLOUD_SOURCE_UNREADABLE.into(),
                    message: format!(
                        "Bulutun {}. kaynağı metin bulutu; noktası burada aranmaz.",
                        si + 1
                    ),
                    path: Some(format!("clouds[{k}].sources[{si}]")),
                });
                continue;
            }
            let opened = (|| -> Result<(Cloud, Bytes), String> {
                let bytes = bytes_of(doc, drawing, s)?;
                Ok((open(&bytes)?, bytes))
            })();
            let (cloud, bytes) = match opened {
                Ok(x) => x,
                Err(why) => {
                    warnings.push(CommandWarning {
                        code: codes::CLOUD_SOURCE_UNREADABLE.into(),
                        message: format!("Bulutun {}. kaynağı okunamadı: {why}.", si + 1),
                        path: Some(format!("clouds[{k}].sources[{si}]")),
                    });
                    continue;
                }
            };
            let (layout, scale, offset) = (cloud.layout, cloud.head.scale, cloud.head.offset);
            for run in cloud.runs() {
                // A COPC node whose cube is off the search square is not read.
                if let (Some(key), Some(info)) = (run.node, cloud.info.as_ref())
                    && !near(key, info, at, reach)
                {
                    continue;
                }
                if let Err(why) = records(&cloud, &bytes, &run, &mut recs) {
                    warnings.push(CommandWarning {
                        code: codes::CLOUD_SOURCE_UNREADABLE.into(),
                        message: format!(
                            "Bulutun {}. kaynağının noktaları okunamadı: {why}.",
                            si + 1
                        ),
                        path: Some(format!("clouds[{k}].sources[{si}]")),
                    });
                    break;
                }
                for (i, r) in recs.chunks_exact(layout.len).enumerate() {
                    read += 1;
                    let dx = f64::from(layout.x(r)) * scale[0] + offset[0] - at[0];
                    let dy = f64::from(layout.y(r)) * scale[1] + offset[1] - at[1];
                    let d = dx * dx + dy * dy;
                    if d > r2 || best.as_ref().is_some_and(|(bd, _)| d >= *bd) {
                        continue;
                    }
                    let f = Facts::of(&layout, r, scale, offset);
                    best = Some((
                        d,
                        CloudPoint {
                            cloud: uid.clone(),
                            source: si as u32,
                            index: run.first + i as u64,
                            x: f.xyz[0],
                            y: f.xyz[1],
                            z: f.xyz[2],
                            distance: d.sqrt(),
                            class: f.class,
                            intensity: f.intensity,
                            return_number: f.returns.0,
                            number_of_returns: f.returns.1,
                            point_source_id: f.source_id,
                            gps_time: f.gps_time,
                            rgb: f.rgb,
                        },
                    ));
                }
            }
        }
    }
    Ok((
        PointCloudsQueried {
            point: best.map(|(_, p)| p),
            read,
            revision: doc.revision().to_string(),
        },
        warnings,
    ))
}

/// Whether a COPC node's cube meets the square of `reach` round `at`, in plan.
fn near(key: Key, info: &kentos_pointcloud::copc::Info, at: [f64; 2], reach: f64) -> bool {
    let (low, side) = key.cube(info);
    !(at[0] + reach < low[0]
        || at[0] - reach > low[0] + side
        || at[1] + reach < low[1]
        || at[1] - reach > low[1] + side)
}

pub(crate) fn answer<T>(r: Result<(T, Vec<CommandWarning>), Stop>) -> CommandResult<T> {
    match r {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

pub mod describe {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &PointCloudsDescribe) -> CommandResult<()> {
        match uids_checked(&input.uids).and_then(|()| clouds(cx.doc, &input.uids).map(|_| ())) {
            Ok(()) => CommandResult::Completed {
                output: (),
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &PointCloudsDescribe,
    ) -> CommandResult<PointCloudsDescribed> {
        answer(describe_in(cx.doc, input))
    }

    /// Read-only: the same answer as the plan.
    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: PointCloudsDescribe,
    ) -> CommandResult<PointCloudsDescribed> {
        plan(cx, &input)
    }
}

pub mod query {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &PointCloudsQuery) -> CommandResult<()> {
        let checked = checks::point(input.at, "Aranan yerin", "at")
            .and_then(|()| uids_checked(&input.uids))
            .and_then(|()| clouds(cx.doc, &input.uids).map(|_| ()));
        match checked {
            Ok(()) => CommandResult::Completed {
                output: (),
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &PointCloudsQuery,
    ) -> CommandResult<PointCloudsQueried> {
        answer(query_in(cx.doc, input))
    }

    /// Read-only: the same answer as the plan.
    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: PointCloudsQuery,
    ) -> CommandResult<PointCloudsQueried> {
        plan(cx, &input)
    }
}
