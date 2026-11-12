//! `cad.time.query` (docs/adr/0537): which objects of a temporal layer show for a valid time and a day known. The
//! valid time is read by the layer time's rules (`time::object_time`, `time::shows`, ADR 0210 §4); the day known by
//! the versions' system time (`time::recorded_interval`, `time::known`). Read only.
//!
//! The state asked (docs/adr/0646 §1) is the drawing as it is, the drawing some undo steps back or a drawing file
//! (`drawing_compare::other_state`); the answer gives the layer's digest in it (`sources::layer_digest`), and a
//! question that gives it back as `expectedStamp` is answered only of the same data (`state_changed`).

use kentos_commands::{CommandResult, LayerNodeType, TimeQueried, TimeQuery};
use kentos_domain::Document;
use kentos_geometry_core::time::{self, Read, Rule, Window};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(checks::error(code, message, Some(path.into())))
}

use crate::time_filter::{moment, utc};

/// The state's name when none is asked.
const NOW: &str = "Şimdiki hâl";

/// What `cad.time.query` answers; why not.
pub(crate) fn query_in(now: &Document, input: &TimeQuery) -> Result<TimeQueried, Stop> {
    let past;
    let (doc, state) = match &input.revision {
        Some(against) => {
            past = crate::drawing_compare::other_state(now, against, "revision")?;
            (&past.0, past.1.clone())
        }
        None => (now, NOW.to_owned()),
    };
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
    let window = crate::time_filter::window(
        input.at.as_deref(),
        input.from.as_deref(),
        input.to.as_deref(),
        "",
    )?;
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
    let stamp = crate::sources::layer_digest(doc, id);
    if let Some(expected) = &input.expected_stamp
        && *expected != stamp
    {
        return Err(Stop::conflict(checks::error(
            codes::STATE_CHANGED,
            format!(
                "“{}” katmanı sorulan durumda değişmiş: içeriğinin özeti beklenenle aynı değil. Soruyu bu durumla yeniden sorun ya da sabitlenmiş durumun dosyasını verin.",
                node.name
            ),
            Some("expectedStamp".into()),
        )));
    }
    let mut out = TimeQueried {
        layer: id.clone(),
        state,
        stamp,
        revision: now.revision().to_string(),
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
