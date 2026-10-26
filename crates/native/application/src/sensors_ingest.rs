//! `cad.sensors.ingest` (docs/adr/0646 §3–§4): observations of assets on a temporal layer, written by the duplicate
//! and late policies asked; `sensors_latest` says each asset's latest with its age. The observations come
//! given, from a SensorThings Observations file, or from a SensorThings address through the host's network
//! (`remote_host`), page by page. The shared cases are `fixtures/commands/v1/cad.sensors.{ingest,latest}.json`.

use std::collections::{BTreeMap, HashMap};

use kentos_commands::{
    CommandResult, CommandWarning, DuplicatePolicy, Entity, EntityBase, LatePolicy, LayerNodeType,
    LayerTime, MAX_OBSERVATIONS, MAX_SENSOR_PAGES, Observation, RecordEntity, SENSOR_PAGES,
    SensorRejected, SensorsIngest, SensorsIngestPlan, SensorsIngested,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::time::{self, Read};
use serde_json::Value;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::time_filter::{moment, utc};

/// The undo step's name.
pub const STEP: &str = "Sensör gözlemleri";

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(checks::error(code, message, Some(path.into())))
}

/// A sensor layer's time: its settings with an asset field; why not. `group`: the message a group gets.
pub(crate) fn sensor_time(
    doc: &Document,
    id: &str,
    group: &str,
) -> Result<(String, LayerTime), Stop> {
    let Some(node) = doc.layers().get(id) else {
        return Err(refuse(
            codes::LAYER_NOT_FOUND,
            format!("“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."),
            "layer",
        ));
    };
    if node.kind != LayerNodeType::Layer {
        return Err(refuse(
            codes::NOT_A_LAYER,
            format!("“{}” bir katman grubu; {group}", node.name),
            "layer",
        ));
    }
    let Some(t) = node.time.clone() else {
        return Err(refuse(
            codes::NOT_TEMPORAL,
            format!(
                "“{}” katmanının zaman ayarı yok; önce Zaman ayarları ile başlangıç alanını seçin.",
                node.name
            ),
            "layer",
        ));
    };
    if t.key.as_deref().is_none_or(|k| k.trim().is_empty()) {
        return Err(refuse(
            codes::NO_ASSET_FIELD,
            format!(
                "“{}” katmanının zaman ayarında kimlik alanı yok; gözlemler varlığa göre tutulur. Zaman ayarları'nda kimlik alanını seçin.",
                node.name
            ),
            "layer",
        ));
    }
    Ok((node.name.clone(), t))
}

pub(crate) fn value_field(name: &str) -> Result<(), Stop> {
    if name.trim().is_empty() {
        return Err(refuse(
            codes::INVALID_INPUT,
            "Değer alanının adını (valueField) verin.".into(),
            "valueField",
        ));
    }
    Ok(())
}

/// A result or a quality as written (ADR 0646 §4): a number its shortest text, a text as it is, a boolean
/// true/false, an object or a list its canonical JSON; null none.
fn text_of(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::Null => None,
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => Some(n.to_string()),
        Value::String(s) => Some(s.clone()),
        other => Some(kentos_commands::canonical::canonical_json(other)),
    }
}

/// The rows a SensorThings Observations answer holds and the next page's address; why not (`name` the file's or the
/// address's name in the message).
fn sta_rows(
    bytes: &[u8],
    name: &str,
    asset: &str,
) -> Result<(Vec<Observation>, Option<String>), String> {
    let unreadable =
        |why: &str| format!("{name} bir SensorThings gözlem listesi olarak okunamadı: {why}.");
    let data: Value = serde_json::from_slice(bytes).map_err(|_| unreadable("JSON değil"))?;
    let (items, next) = match &data {
        Value::Array(list) => (list, None),
        Value::Object(map) => match map.get("value") {
            Some(Value::Array(list)) => (
                list,
                map.get("@iot.nextLink")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            ),
            _ => return Err(unreadable("gözlemlerin dizisi (value) yok")),
        },
        _ => return Err(unreadable("gözlemlerin dizisi (value) yok")),
    };
    let rows = items
        .iter()
        .map(|o| {
            let at = o
                .get("phenomenonTime")
                .and_then(Value::as_str)
                .map(|t| t.split('/').next().unwrap_or("").to_owned())
                .unwrap_or_default();
            Observation {
                asset: asset.to_owned(),
                at,
                value: o.get("result").cloned(),
                quality: o.get("resultQuality").cloned(),
            }
        })
        .collect();
    Ok((rows, next))
}

/// The observations an input names, the pages read and whether one is left; why not.
fn source(input: &SensorsIngest) -> Result<(Vec<Observation>, u32, bool), Stop> {
    let given = [
        input.observations.is_some(),
        input.file.is_some(),
        input.url.is_some(),
    ];
    if given.iter().filter(|g| **g).count() != 1 {
        return Err(refuse(
            codes::INVALID_SOURCE,
            "Gözlemleri tek bir kaynaktan verin: observations, file ya da url.".into(),
            "observations",
        ));
    }
    if let Some(rows) = &input.observations {
        return Ok((rows.clone(), 0, false));
    }
    let asset = input.asset.as_deref().map(str::trim).unwrap_or("");
    if asset.is_empty() {
        return Err(refuse(
            codes::INVALID_INPUT,
            "Dosyanın ve adresin gözlemleri tek bir varlığındır; varlığı (asset) verin.".into(),
            "asset",
        ));
    }
    let pages = input.pages.unwrap_or(SENSOR_PAGES);
    if !(1..=MAX_SENSOR_PAGES).contains(&pages) {
        return Err(refuse(
            codes::INVALID_INPUT,
            format!("Sayfa sayısı (pages) 1 ile {MAX_SENSOR_PAGES} arasında olmalı."),
            "pages",
        ));
    }
    if let Some(path) = &input.file {
        let p = std::path::Path::new(path);
        if path.trim().is_empty() || !p.is_absolute() {
            return Err(refuse(
                codes::INVALID_FILE_PATH,
                format!(
                    "“{path}” mutlak bir dosya yolu değil; dosyanın tam yolunu verin (/klasör/ad.json)."
                ),
                "file",
            ));
        }
        let bytes = std::fs::read(p).map_err(|_| {
            refuse(
                codes::FILE_NOT_FOUND,
                format!("{path} okunamadı: dosya yok ya da açılamıyor."),
                "file",
            )
        })?;
        let name = p
            .file_name()
            .map_or_else(|| path.clone(), |n| n.to_string_lossy().into_owned());
        let (rows, next) = sta_rows(&bytes, &name, asset)
            .map_err(|m| refuse(codes::IMPORT_UNREADABLE, m, "file"))?;
        return Ok((rows, 1, next.is_some()));
    }
    let url = input.url.clone().unwrap_or_default();
    let Some(net) = crate::remote_host::remote() else {
        return Err(refuse(
            codes::SOURCE_UNREACHABLE,
            crate::remote_host::NO_NETWORK.into(),
            "url",
        ));
    };
    let mut rows = Vec::new();
    let mut next = Some(url);
    let mut read = 0;
    while let Some(at) = next.take() {
        if read == pages {
            next = Some(at);
            break;
        }
        let bytes = match net.object(&at) {
            Ok(Some(b)) => b,
            Ok(None) => {
                return Err(refuse(
                    codes::SOURCE_UNREACHABLE,
                    format!("“{at}” adresinde gözlem yok (sunucu bulamadı)."),
                    "url",
                ));
            }
            Err(why) => return Err(refuse(codes::SOURCE_UNREACHABLE, why, "url")),
        };
        let (page, link) = sta_rows(&bytes, &format!("“{at}”"), asset)
            .map_err(|m| refuse(codes::IMPORT_UNREADABLE, m, "url"))?;
        rows.extend(page);
        read += 1;
        next = link;
        if rows.len() > MAX_OBSERVATIONS {
            break;
        }
    }
    Ok((rows, read, next.is_some()))
}

/// The moment now, to the second.
pub(crate) fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs() as f64 * 1000.0)
}

/// What an ingest writes: the records to add, the records changed in place (slot, attributes), the replaced ids.
#[derive(Default)]
struct Ingest {
    adds: Vec<BTreeMap<String, String>>,
    sets: Vec<(Slot, BTreeMap<String, String>)>,
    replaced: Vec<String>,
    skipped: u32,
    late: u32,
    rejected: Vec<SensorRejected>,
    pages: u32,
    more: bool,
    received_at: String,
    warnings: Vec<CommandWarning>,
}

pub(crate) fn attr(attrs: &BTreeMap<String, String>, key: Option<&str>) -> Read {
    key.and_then(|k| attrs.get(k))
        .map_or(Read::Empty, |v| time::read(v))
}

fn ingest_in(doc: &Document, input: &SensorsIngest) -> Result<Ingest, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    let warnings = checks::layer_at(doc, &input.layer, "layer")?;
    let (_, t) = sensor_time(
        doc,
        &input.layer,
        "nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.",
    )?;
    value_field(&input.value_field)?;
    let received = match &input.received_at {
        Some(text) => moment(text, "receivedAt")?,
        None => now(),
    };
    let (rows, pages, more) = source(input)?;
    if rows.is_empty() || rows.len() > MAX_OBSERVATIONS {
        return Err(refuse(
            codes::INVALID_COUNT,
            format!("Gözlem sayısı 1 ile {MAX_OBSERVATIONS} arasında olmalı."),
            "observations",
        ));
    }
    let key = t.key.clone().unwrap_or_default();
    // The current versions by (asset, moment) and each asset's latest moment.
    let mut stored: HashMap<(String, u64), Vec<Slot>> = HashMap::new();
    let mut latest: HashMap<String, f64> = HashMap::new();
    for (uid, e) in doc.by_layer_with_uids(&input.layer) {
        let a = e.base().attrs.get(&key).map_or("", |v| v.trim());
        let Read::Moment(m) = attr(&e.base().attrs, Some(&t.start)) else {
            continue;
        };
        let ended = t
            .recorded_end
            .as_deref()
            .and_then(|f| e.base().attrs.get(f))
            .is_some_and(|v| !v.trim().is_empty());
        if a.is_empty() || ended {
            continue;
        }
        let Some(slot) = doc.slot_of(uid) else {
            continue;
        };
        stored
            .entry((a.to_owned(), m.to_bits()))
            .or_default()
            .push(slot);
        let l = latest.entry(a.to_owned()).or_insert(m);
        if m > *l {
            *l = m;
        }
    }
    let mut out = Ingest {
        pages,
        more,
        received_at: utc(received),
        warnings,
        ..Ingest::default()
    };
    let mut pending: HashMap<(String, u64), usize> = HashMap::new();
    let mut sets: Vec<(Slot, BTreeMap<String, String>)> = Vec::new();
    for (k, row) in rows.iter().enumerate() {
        let asset = row.asset.trim();
        let reject = |out: &mut Ingest, code: &str, message: String| {
            out.rejected.push(SensorRejected {
                index: k as u32,
                asset: row.asset.clone(),
                at: row.at.clone(),
                code: code.to_owned(),
                message,
            });
        };
        if asset.is_empty() {
            reject(
                &mut out,
                codes::INVALID_ASSET,
                "Gözlemin varlığı (asset) boş; sensörün ya da sayacın kimliğini verin.".into(),
            );
            continue;
        }
        let m = match time::read(&row.at) {
            Read::Moment(m) => m,
            Read::Empty => {
                reject(
                    &mut out,
                    codes::INVALID_MOMENT,
                    "Gözlemin anı boş; olay zamanını verin (at; SensorThings'te phenomenonTime)."
                        .into(),
                );
                continue;
            }
            Read::Unreadable => {
                let why = moment(&row.at, "at")
                    .err()
                    .map(|s| s.into_error().message)
                    .unwrap_or_default();
                reject(&mut out, codes::INVALID_MOMENT, why);
                continue;
            }
        };
        let id = (asset.to_owned(), m.to_bits());
        let duplicate = pending.contains_key(&id) || stored.contains_key(&id);
        if duplicate && input.duplicates == DuplicatePolicy::First {
            out.skipped += 1;
            continue;
        }
        let last = latest.get(asset).copied();
        let late = !duplicate && last.is_some_and(|l| m < l);
        if late && input.late == LatePolicy::Reject {
            reject(
                &mut out,
                codes::LATE_OBSERVATION,
                format!(
                    "{asset} için {} anında gözlem var; {} ondan önce (geç gelen gözlem).",
                    utc(last.unwrap_or(m)),
                    utc(m)
                ),
            );
            continue;
        }
        let mut given = BTreeMap::new();
        given.insert(key.clone(), asset.to_owned());
        given.insert(t.start.clone(), utc(m));
        if let Some(r) = &t.recorded_start {
            given.insert(r.clone(), utc(received));
        }
        if let Some(v) = text_of(row.value.as_ref()) {
            given.insert(input.value_field.clone(), v);
        }
        if let (Some(q), Some(f)) = (text_of(row.quality.as_ref()), &input.quality_field) {
            given.insert(f.clone(), q);
        }
        if late && let Some(f) = &input.late_field {
            given.insert(f.clone(), "evet".into());
        }
        let attrs = match checks::object_attrs(
            doc,
            &input.layer,
            Some(&given),
            &format!("observations.{k}"),
        ) {
            Ok(a) => a.unwrap_or_default(),
            Err(stop) => {
                let e = stop.into_error();
                reject(&mut out, &e.code, e.message);
                continue;
            }
        };
        out.late += u32::from(late);
        let l = latest.entry(asset.to_owned()).or_insert(m);
        if m > *l {
            *l = m;
        }
        if duplicate && input.duplicates == DuplicatePolicy::Last {
            if let Some(&at) = pending.get(&id) {
                out.adds[at] = attrs;
                out.skipped += 1;
                continue;
            }
            if let Some(slots) = stored.get(&id) {
                for &slot in slots {
                    let uid = doc.uid(slot).map(|u| u.to_string()).unwrap_or_default();
                    if !out.replaced.contains(&uid) {
                        out.replaced.push(uid);
                    }
                    let place = sets.iter().position(|(s, _)| *s == slot);
                    let mut old = match place {
                        Some(i) => sets[i].1.clone(),
                        None => doc
                            .get(slot)
                            .map(|e| e.base().attrs.clone())
                            .unwrap_or_default(),
                    };
                    if let Some(end) = &t.recorded_end {
                        old.insert(end.clone(), utc(received));
                    } else {
                        for name in
                            std::iter::once(&input.value_field).chain(input.quality_field.as_ref())
                        {
                            match attrs.get(name) {
                                Some(v) => old.insert(name.clone(), v.clone()),
                                None => old.remove(name),
                            };
                        }
                        if let Some(r) = &t.recorded_start
                            && let Some(v) = attrs.get(r)
                        {
                            old.insert(r.clone(), v.clone());
                        }
                        if let Some(f) = &input.late_field
                            && let Some(v) = attrs.get(f)
                        {
                            old.insert(f.clone(), v.clone());
                        }
                    }
                    match place {
                        Some(i) => sets[i].1 = old,
                        None => sets.push((slot, old)),
                    }
                }
                if t.recorded_end.is_none() {
                    continue;
                }
            }
        }
        pending.insert(id, out.adds.len());
        out.adds.push(attrs);
    }
    out.sets = sets;
    Ok(out)
}

pub fn validate(cx: &ExecutionContext<'_>, input: &SensorsIngest) -> CommandResult<()> {
    match ingest_in(cx.doc, input) {
        Ok(i) => CommandResult::Completed {
            output: (),
            warnings: i.warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &SensorsIngest) -> CommandResult<SensorsIngestPlan> {
    match ingest_in(cx.doc, input) {
        Ok(i) => CommandResult::Completed {
            output: SensorsIngestPlan {
                adds: i.adds.len() as u32,
                replaced: i.replaced,
                skipped: i.skipped,
                late: i.late,
                rejected: i.rejected,
                pages: i.pages,
                more: i.more,
                received_at: i.received_at,
                revision: cx.doc.revision().to_string(),
            },
            warnings: i.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Writes the observations as one undo step; nothing when nothing is to be written.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: SensorsIngest,
) -> CommandResult<SensorsIngested> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_SENSORS_INGEST,
        kentos_commands::CAD_SENSORS_INGEST_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: SensorsIngest,
) -> CommandResult<SensorsIngested> {
    let i = match ingest_in(cx.doc, &input) {
        Ok(i) => i,
        Err(stop) => return stop.into(),
    };
    let mut out = SensorsIngested {
        added: Vec::with_capacity(i.adds.len()),
        replaced: i.replaced,
        skipped: i.skipped,
        late: i.late,
        rejected: i.rejected,
        pages: i.pages,
        more: i.more,
        received_at: i.received_at,
        revision: cx.doc.revision().to_string(),
    };
    if !i.adds.is_empty() || !i.sets.is_empty() {
        cx.doc.name_step(STEP);
        for (slot, attrs) in i.sets {
            let Some(mut e) = cx.doc.get(slot).cloned() else {
                continue;
            };
            e.base_mut().attrs = attrs;
            cx.doc.update(slot, e);
        }
        for attrs in i.adds {
            let entity = Entity::Record(RecordEntity {
                base: EntityBase {
                    look: Default::default(),
                    id: 0,
                    layer_id: input.layer.clone(),
                    color: None,
                    attrs,
                    label: None,
                    symbol: None,
                    line_weight: None,
                    label_pins: Vec::new(),
                    attachments: Vec::new(),
                },
            });
            match checks::add(cx.doc, entity) {
                Ok((_, uid, _)) => out.added.push(uid),
                Err(stop) => return stop.into(),
            }
        }
        out.revision = cx.doc.revision().to_string();
    }
    CommandResult::Completed {
        output: out,
        warnings: i.warnings,
    }
}
