//! `cad.sensors.latest` (docs/adr/0646 §5): each asset's latest observation on a sensor layer at a moment, among the
//! versions known then (or the current ones), with its age; a stale one is said so, so that a stopped stream's last
//! value is not shown as current. Read only.

use std::collections::{BTreeMap, HashMap};

use kentos_commands::{CommandResult, SensorLatest, SensorsLatest, SensorsLatestAnswer};
use kentos_domain::Document;
use kentos_geometry_core::text::natural::natural_cmp;
use kentos_geometry_core::time::{self, Read};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::sensors_ingest::{attr, now, sensor_time, value_field};
use crate::time_filter::{moment, utc};

/// An asset's best so far: (moment, receipt, place in the layer), its persistent id and attributes.
type Best = ((f64, f64, usize), String, BTreeMap<String, String>);

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(checks::error(code, message, Some(path.into())))
}

pub(crate) fn latest_in(
    doc: &Document,
    input: &SensorsLatest,
) -> Result<SensorsLatestAnswer, Stop> {
    let (name, t) = sensor_time(
        doc,
        &input.layer,
        "zaman katmanın ayarıdır. Grubun içinden bir katman seçin.",
    )?;
    value_field(&input.value_field)?;
    let at = match &input.at {
        Some(text) => moment(text, "at")?,
        None => now(),
    };
    let known_at = match &input.known_at {
        Some(text) => {
            if !t.recorded() {
                return Err(refuse(
                    codes::NOT_RECORDED,
                    format!(
                        "“{name}” katmanı sürümlerinin kayıt zamanını tutmuyor; Zaman ayarları'nda kayıt başlangıcı alanını seçin."
                    ),
                    "knownAt",
                ));
            }
            Some(moment(text, "knownAt")?)
        }
        None => None,
    };
    if input
        .stale_after
        .is_some_and(|s| !(s.is_finite() && s >= 0.0))
    {
        return Err(refuse(
            codes::INVALID_INPUT,
            "Eskime süresi (staleAfter) sıfır ya da pozitif bir saniye olmalı.".into(),
            "staleAfter",
        ));
    }
    let key = t.key.clone().unwrap_or_default();
    // Each asset's best: (moment, receipt, place), its id and attributes.
    let mut best: HashMap<String, Best> = HashMap::new();
    let mut unreadable = 0u32;
    for (place, (uid, e)) in doc.by_layer_with_uids(&input.layer).enumerate() {
        let attrs = &e.base().attrs;
        let asset = attrs.get(&key).map_or("", |v| v.trim());
        let Read::Moment(m) = attr(attrs, Some(&t.start)) else {
            unreadable += 1;
            continue;
        };
        if asset.is_empty() {
            unreadable += 1;
            continue;
        }
        let rs = attr(attrs, t.recorded_start.as_deref());
        if let Some(k) = known_at {
            let interval = time::recorded_interval(rs, attr(attrs, t.recorded_end.as_deref()));
            if !time::known(interval, k) {
                continue;
            }
        } else if t
            .recorded_end
            .as_deref()
            .and_then(|f| attrs.get(f))
            .is_some_and(|v| !v.trim().is_empty())
        {
            continue;
        }
        if m > at {
            continue;
        }
        let receipt = match rs {
            Read::Moment(r) => r,
            _ => f64::NEG_INFINITY,
        };
        let rank = (m, receipt, place);
        let better = best.get(asset).is_none_or(|(r, _, _)| {
            (rank.0, rank.1, rank.2).partial_cmp(&(r.0, r.1, r.2))
                == Some(std::cmp::Ordering::Greater)
        });
        if better {
            best.insert(asset.to_owned(), (rank, uid.to_string(), attrs.clone()));
        }
    }
    let mut assets: Vec<_> = best.into_iter().collect();
    assets.sort_by(|a, b| natural_cmp(&a.0, &b.0));
    let mut out = SensorsLatestAnswer {
        layer: input.layer.clone(),
        at: utc(at),
        known_at: known_at.map(utc),
        revision: doc.revision().to_string(),
        unreadable,
        ..SensorsLatestAnswer::default()
    };
    let shown = |attrs: &BTreeMap<String, String>, f: Option<&String>| {
        f.and_then(|f| attrs.get(f))
            .filter(|v| !v.is_empty())
            .cloned()
    };
    for (asset, ((m, _, _), uid, attrs)) in assets {
        let age = (at - m) / 1000.0;
        let stale = input.stale_after.is_some_and(|s| age > s);
        out.stale += u32::from(stale);
        out.assets.push(SensorLatest {
            asset,
            uid,
            observed_at: utc(m),
            value: shown(&attrs, Some(&input.value_field)),
            quality: shown(&attrs, input.quality_field.as_ref()),
            age_seconds: age,
            stale,
        });
    }
    for e in &input.expected_assets {
        if !out.assets.iter().any(|a| &a.asset == e) && !out.missing.contains(e) {
            out.missing.push(e.clone());
        }
    }
    Ok(out)
}

pub fn validate(cx: &ExecutionContext<'_>, input: &SensorsLatest) -> CommandResult<()> {
    match latest_in(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &SensorsLatest,
) -> CommandResult<SensorsLatestAnswer> {
    match latest_in(cx.doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// Read-only: the same answer as the plan.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: SensorsLatest,
) -> CommandResult<SensorsLatestAnswer> {
    plan(cx, &input)
}
