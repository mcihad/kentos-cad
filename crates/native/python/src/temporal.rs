//! Zaman for Python (`kentos.temporal`, docs/adr/0210 §11): the geometry
//! core's time rules (`kentos_geometry_core::time`) as the apps use them: a
//! value read and written, a moment shown, a slider's steps and positions,
//! and the times of a temporal layer's objects read from an open drawing as
//! the apps' time slider reads them (§4). Moments cross as milliseconds since
//! 1970 (`f64`, open ends ±∞); `kentos.temporal` makes `datetime`s of them.

use kentos_contracts::{LayerNodeType, LayerTime};
use kentos_domain::Slot;
use kentos_geometry_core::time::{self, Mode, Read, Rule, Step, Time, Unit, Window};
use kentos_headless::HeadlessError;
use pyo3::prelude::*;

use super::{PySession, host};

fn refused(code: &'static str, message: impl Into<String>) -> PyErr {
    host(HeadlessError::new(code, message.into()))
}

/// The first and the last moment the calendar holds (years 1–9999).
fn bounds() -> (f64, f64) {
    const DAY: i64 = 86_400_000;
    (
        (time::days_from_civil(1, 1, 1) * DAY) as f64,
        (time::days_from_civil(10_000, 1, 1) * DAY - 1) as f64,
    )
}

/// A moment Python sent: a whole number of milliseconds in the years 1–9999.
fn moment(t: f64) -> PyResult<f64> {
    let (lo, hi) = bounds();
    if t.is_finite() && t.fract() == 0.0 && (lo..=hi).contains(&t) {
        Ok(t)
    } else {
        Err(refused(
            "invalid_input",
            format!("{t} bir an değil: 1–9999 yıllarında, 1970'ten bu yana tam milisaniye olmalı."),
        ))
    }
}

fn unit(name: &str) -> PyResult<Unit> {
    Unit::from_name(name).ok_or_else(|| {
        refused(
            "invalid_input",
            format!("“{name}” bir zaman birimi değil: second, minute, hour, day, week, month ya da year olmalı."),
        )
    })
}

fn step(n: i64, name: &str) -> PyResult<Step> {
    Step::new(n, unit(name)?).ok_or_else(|| {
        refused(
            "invalid_input",
            format!("Adım 1 ile 999 birim arasında olmalı; {n} yazıldı."),
        )
    })
}

fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Range => "range",
        Mode::Instant => "instant",
        Mode::Cumulative => "cumulative",
    }
}

fn mode_of(name: &str) -> PyResult<Mode> {
    match name {
        "range" => Ok(Mode::Range),
        "instant" => Ok(Mode::Instant),
        "cumulative" => Ok(Mode::Cumulative),
        _ => Err(refused(
            "invalid_input",
            format!("“{name}” bir zaman türü değil: range, instant ya da cumulative olmalı."),
        )),
    }
}

/// A window: the moment `a`, or `[a, b)`.
fn window(a: f64, b: Option<f64>) -> PyResult<Window> {
    let a = moment(a)?;
    match b {
        None => Ok(Window::Instant(a)),
        Some(b) => {
            let b = moment(b)?;
            if b <= a {
                return Err(refused(
                    "invalid_input",
                    "Aralığın sonu başından sonra olmalı.",
                ));
            }
            Ok(Window::Range(a, b))
        }
    }
}

/// An object's time as it crosses: `(start, end, mode)`, open ends ±∞.
type Crossed = Option<(f64, f64, &'static str)>;

fn crossed(t: Option<Time>) -> Crossed {
    t.map(|t| (t.s, t.e, mode_name(t.mode)))
}

/// What a layer's values give: `(timed, timeless, unreadable)` and the extent.
type Summary = ((usize, usize, usize), Option<(f64, f64)>);

fn summary(s: time::Summary) -> Summary {
    ((s.timed, s.timeless, s.unreadable), s.extent)
}

/// A value read: `(0, nan)` empty, `(1, moment)`, `(2, nan)` unreadable (§3).
#[pyfunction]
pub fn time_read(text: &str) -> (u8, f64) {
    match time::read(text) {
        Read::Empty => (0, f64::NAN),
        Read::Moment(t) => (1, t),
        Read::Unreadable => (2, f64::NAN),
    }
}

/// A moment as a value is written: `YYYY-AA-GG` at midnight (always when `date_only`), else with its time.
#[pyfunction]
pub fn time_write(t: f64, date_only: bool) -> PyResult<String> {
    Ok(time::write(moment(t)?, date_only))
}

/// A moment as the apps show it for a step of `unit`: `GG.AA.YYYY`, with the time under a day.
#[pyfunction]
pub fn time_show(t: f64, unit_name: &str) -> PyResult<String> {
    Ok(time::show(moment(t)?, unit(unit_name)?))
}

/// A moment rounded down to `unit` (a week from Monday).
#[pyfunction]
pub fn time_floor(t: f64, unit_name: &str) -> PyResult<f64> {
    Ok(time::floor_to(moment(t)?, unit(unit_name)?))
}

/// The step the slider opens with over `[start, end]` (§5): `(n, unit)`.
#[pyfunction]
pub fn time_auto_step(start: f64, end: f64) -> PyResult<(i64, &'static str)> {
    let s = time::auto_step((moment(start)?, moment(end)?));
    Ok((s.n, s.unit.name()))
}

/// The slider's anchor and its last position's index over `[start, end]` with the step; None past 100 000 positions.
#[pyfunction]
pub fn time_positions(
    start: f64,
    end: f64,
    n: i64,
    unit_name: &str,
) -> PyResult<Option<(f64, i64)>> {
    Ok(time::positions(
        (moment(start)?, moment(end)?),
        step(n, unit_name)?,
    ))
}

/// The `k`-th position from `anchor`, before it when negative (months and years by the calendar).
#[pyfunction]
pub fn time_position(anchor: f64, n: i64, unit_name: &str, k: i64) -> PyResult<f64> {
    let most = time::MAX_POSITIONS + 1;
    if !(-most..=most).contains(&k) {
        return Err(refused(
            "invalid_input",
            format!("Konum -{most} ile {most} arasında olmalı; {k} yazıldı."),
        ));
    }
    let t = time::position(moment(anchor)?, step(n, unit_name)?, k);
    moment(t)
}

/// An object's time from its start and end values on a layer of the rule (§4); None timeless.
#[pyfunction]
#[pyo3(signature = (ranged, cumulative, start = None, end = None))]
pub fn time_of(ranged: bool, cumulative: bool, start: Option<&str>, end: Option<&str>) -> Crossed {
    let read = |v: Option<&str>| v.map_or(Read::Empty, time::read);
    let end = if ranged { read(end) } else { Read::Empty };
    crossed(time::object_time(
        Rule { ranged, cumulative },
        read(start),
        end,
    ))
}

/// Whether a time `[s, e)` of `mode` shows in the window: the moment `a`, or `[a, b)` (§4's table).
#[pyfunction]
#[pyo3(signature = (s, e, mode, a, b = None))]
pub fn time_shows(s: f64, e: f64, mode: &str, a: f64, b: Option<f64>) -> PyResult<bool> {
    let t = Time {
        s,
        e,
        mode: mode_of(mode)?,
    };
    Ok(time::shows(&t, &window(a, b)?))
}

/// The times of a layer's values `(start, end)` (§4) and what they give.
#[pyfunction]
pub fn time_values(
    ranged: bool,
    cumulative: bool,
    values: Vec<(Option<String>, Option<String>)>,
) -> (Vec<Crossed>, Summary) {
    let (times, s) = time::layer_times(
        Rule { ranged, cumulative },
        values.iter().map(|(a, b)| (a.as_deref(), b.as_deref())),
    );
    (times.into_iter().map(crossed).collect(), summary(s))
}

/// A temporal layer of the drawing, its rule; else why not, in the apps' words.
fn temporal(session: &PySession, layer: &str) -> PyResult<LayerTime> {
    let layers = session.inner.document().layers();
    let Some(node) = layers.get(layer) else {
        return Err(refused(
            "layer_not_found",
            format!("“{layer}” kimlikli katman yok."),
        ));
    };
    if node.kind == LayerNodeType::Group {
        return Err(refused(
            "not_a_layer",
            format!("“{}” bir grup; zaman katmanındır.", node.name),
        ));
    }
    node.time.clone().ok_or_else(|| {
        refused(
            "not_temporal",
            format!(
                "“{}” katmanı zamansal değil: başlangıç alanını kentos.temporal.set_time ya da Zaman ayarları verir.",
                node.name
            ),
        )
    })
}

/// The times of temporal layer `layer`'s objects in the drawing's order, as the
/// apps' slider reads them: their ids, their times and what they give.
#[pyfunction]
pub fn temporal_layer(
    session: PyRef<'_, PySession>,
    layer: &str,
) -> PyResult<(Vec<String>, Vec<Crossed>, Summary)> {
    let rule = temporal(&session, layer)?;
    let doc = session.inner.document();
    let list: Vec<_> = doc.by_layer(layer).collect();
    let (times, s) = time::layer_times(
        Rule {
            ranged: rule.end.is_some(),
            cumulative: rule.cumulative,
        },
        list.iter().map(|e| {
            let attrs = &e.base().attrs;
            (
                attrs.get(&rule.start).map(String::as_str),
                rule.end
                    .as_ref()
                    .and_then(|k| attrs.get(k))
                    .map(String::as_str),
            )
        }),
    );
    let uids = list
        .iter()
        .map(|e| {
            doc.uid(Slot(e.base().id))
                .map_or_else(String::new, |u| u.to_string())
        })
        .collect();
    Ok((uids, times.into_iter().map(crossed).collect(), summary(s)))
}

fn temporal_ids(session: &PySession) -> Vec<String> {
    session
        .inner
        .document()
        .layers()
        .leaves()
        .into_iter()
        .filter(|l| l.time.is_some() && l.service.is_none())
        .map(|l| l.id.clone())
        .collect()
}

/// The temporal layers of the drawing (layers with a time), in the tree's order.
#[pyfunction]
pub fn temporal_layers(session: PyRef<'_, PySession>) -> Vec<String> {
    temporal_ids(&session)
}

/// The ids of temporal layers' objects shown in the window, in the drawing's
/// order (§4): the moment `a`, or `[a, b)`; timeless objects show always.
/// `layers` None: every temporal layer.
#[pyfunction]
#[pyo3(signature = (session, a, b = None, layers = None))]
pub fn temporal_shown(
    session: PyRef<'_, PySession>,
    a: f64,
    b: Option<f64>,
    layers: Option<Vec<String>>,
) -> PyResult<Vec<String>> {
    let w = window(a, b)?;
    let ids = match layers {
        Some(ids) => ids,
        None => temporal_ids(&session),
    };
    let mut shown = std::collections::HashSet::new();
    for id in &ids {
        let rule = temporal(&session, id)?;
        let doc = session.inner.document();
        let core = Rule {
            ranged: rule.end.is_some(),
            cumulative: rule.cumulative,
        };
        for e in doc.by_layer(id) {
            let attrs = &e.base().attrs;
            let read = |k: Option<&String>| {
                k.and_then(|k| attrs.get(k))
                    .map_or(Read::Empty, |v| time::read(v))
            };
            let end = if core.ranged {
                read(rule.end.as_ref())
            } else {
                Read::Empty
            };
            let t = time::object_time(core, read(Some(&rule.start)), end);
            if t.is_none_or(|t| time::shows(&t, &w)) {
                shown.insert(e.base().id);
            }
        }
    }
    let doc = session.inner.document();
    Ok(doc
        .entities()
        .filter(|e| shown.contains(&e.base().id))
        .filter_map(|e| doc.uid(Slot(e.base().id)))
        .map(|u| u.to_string())
        .collect())
}
