//! `cad.time.query` (docs/adr/0537): which objects of a temporal layer show for a valid time and a day known. The
//! valid time is read by the layer time's rules (`time::object_time`, `time::shows`, ADR 0210 §4); the day known by
//! the versions' system time (`time::recorded_interval`, `time::known`). Read only.

use kentos_contracts::{CommandResult, LayerNodeType, TimeQueried, TimeQuery};
use kentos_domain::Document;
use kentos_geometry_core::time::{self, Read, Rule, Window};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(checks::error(code, message, Some(path.into())))
}

/// A moment the input gives; why not.
fn moment(text: &str, path: &str) -> Result<f64, Stop> {
    match time::read(text) {
        Read::Moment(t) => Ok(t),
        _ => Err(refuse(
            codes::INVALID_MOMENT,
            format!(
                "“{text}” bir an değil: bir tarih (2024-06-01 ya da 01.06.2024) ya da tarih ve saat (2024-06-01T14:30, bölgesiyle 2024-06-01T14:30+03:00) yazın."
            ),
            path,
        )),
    }
}

/// A moment written back, in UTC.
fn utc(t: f64) -> String {
    let w = time::write(t, false);
    if w.contains('T') { format!("{w}Z") } else { w }
}

/// What `cad.time.query` answers; why not.
pub(crate) fn query_in(doc: &Document, input: &TimeQuery) -> Result<TimeQueried, Stop> {
    let id = &input.layer;
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
            format!(
                "“{}” bir katman grubu; zaman katmanın ayarıdır. Grubun içinden bir katman seçin.",
                node.name
            ),
            "layer",
        ));
    }
    let Some(setting) = node.time.clone() else {
        return Err(refuse(
            codes::NOT_TEMPORAL,
            format!(
                "“{}” katmanının zaman ayarı yok; önce Zaman ayarları ile başlangıç alanını seçin.",
                node.name
            ),
            "layer",
        ));
    };
    let window = match (&input.at, &input.from, &input.to) {
        (Some(_), Some(_), _) | (Some(_), _, Some(_)) => {
            return Err(refuse(
                codes::INVALID_PERIOD,
                "Geçerlilik zamanı ya bir an (at) ya bir aralıktır (from ve to); ikisi birden verilmez.".into(),
                "at",
            ));
        }
        (Some(a), None, None) => Some(Window::Instant(moment(a, "at")?)),
        (None, Some(f), Some(t)) => {
            let (a, b) = (moment(f, "from")?, moment(t, "to")?);
            if b <= a {
                return Err(refuse(
                    codes::INVALID_PERIOD,
                    "Aralığın bitişi başlangıcından sonra olmalı; [from, to) bitişi içermez."
                        .into(),
                    "to",
                ));
            }
            Some(Window::Range(a, b))
        }
        (None, Some(_), None) | (None, None, Some(_)) => {
            return Err(refuse(
                codes::INVALID_PERIOD,
                "Aralığın başlangıcı (from) ve bitişi (to) birlikte verilir.".into(),
                if input.from.is_some() { "to" } else { "from" },
            ));
        }
        (None, None, None) => None,
    };
    let known_at = match &input.known_at {
        Some(text) => {
            if !setting.recorded() {
                return Err(refuse(
                    codes::NOT_RECORDED,
                    format!(
                        "“{}” katmanı sürümlerinin kayıt zamanını tutmuyor; Zaman ayarları'nda kayıt başlangıcı alanını seçin.",
                        node.name
                    ),
                    "knownAt",
                ));
            }
            Some(moment(text, "knownAt")?)
        }
        None => None,
    };
    let rule = Rule {
        ranged: setting.ranged(),
        cumulative: setting.cumulative,
    };
    let mut out = TimeQueried {
        layer: id.clone(),
        revision: doc.revision().to_string(),
        known_at: known_at.map(utc),
        ..TimeQueried::default()
    };
    match window {
        Some(Window::Instant(a)) => out.at = Some(utc(a)),
        Some(Window::Range(a, b)) => {
            out.from = Some(utc(a));
            out.to = Some(utc(b));
        }
        None => {}
    }
    let read_attr = |attrs: &std::collections::BTreeMap<String, String>, key: Option<&str>| {
        key.and_then(|k| attrs.get(k))
            .map_or(Read::Empty, |v| time::read(v))
    };
    for (uid, e) in doc.by_layer_with_uids(id) {
        let attrs = &e.base().attrs;
        let start = read_attr(attrs, Some(setting.start.as_str()));
        let end = if rule.ranged {
            read_attr(attrs, setting.end.as_deref())
        } else {
            Read::Empty
        };
        out.unreadable += u32::from(start == Read::Unreadable) + u32::from(end == Read::Unreadable);
        let valid = time::object_time(rule, start, end);
        if valid.is_none() {
            out.timeless += 1;
        }
        let mut shows = match (&valid, &window) {
            (Some(t), Some(w)) => time::shows(t, w),
            _ => true,
        };
        if !shows {
            out.not_valid += 1;
        }
        if setting.recorded() {
            let rs = read_attr(attrs, setting.recorded_start.as_deref());
            let re = read_attr(attrs, setting.recorded_end.as_deref());
            out.unreadable += u32::from(rs == Read::Unreadable) + u32::from(re == Read::Unreadable);
            let recorded = time::recorded_interval(rs, re);
            if recorded.is_none() {
                out.unrecorded += 1;
            }
            if let Some(at) = known_at
                && !time::known(recorded, at)
            {
                if shows {
                    out.not_known += 1;
                }
                shows = false;
            }
        }
        if shows {
            out.shown.push(uid.to_string());
        }
    }
    Ok(out)
}

pub fn validate(cx: &ExecutionContext<'_>, input: &TimeQuery) -> CommandResult<()> {
    match query_in(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &TimeQuery) -> CommandResult<TimeQueried> {
    match query_in(cx.doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// Read-only: the same answer as the plan.
pub fn execute(cx: &mut ExecutionContext<'_>, input: TimeQuery) -> CommandResult<TimeQueried> {
    plan(cx, &input)
}
