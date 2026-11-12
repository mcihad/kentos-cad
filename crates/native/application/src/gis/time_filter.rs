//! A question's time (docs/adr/0537, 0595): the valid time at an instant or over a period and the day known, read as
//! `cad.time.query` reads them, checked on a record's attributes by the layer time's rules (`time::object_time`,
//! `time::shows`, `time::recorded_interval`, `time::known`), and the condition a source is sent so that it hands over
//! fewer records. The condition keeps at least every record the rules keep: a date column's first ten characters are
//! compared with the moments a day wider on each side (a zone moves a moment less than a day), and a value that does
//! not begin with an ISO date (empty, `01.06.2024`, a number) is kept and checked here.

use std::collections::BTreeMap;

use kentos_commands::{LayerTime, QueryTime};
use kentos_expression::pushdown::Dialect;
use kentos_geometry_core::time::{self, Read, Rule, Window};

use crate::checks::{self, Stop};
use crate::codes;

/// A day, in milliseconds (the moments' unit).
const DAY: f64 = 86_400_000.0;

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(checks::error(code, message, Some(path.into())))
}

/// A moment the input gives; why not.
pub(crate) fn moment(text: &str, path: &str) -> Result<f64, Stop> {
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
pub(crate) fn utc(t: f64) -> String {
    let w = time::write(t, false);
    if w.contains('T') { format!("{w}Z") } else { w }
}

/// The valid time an input asks: an instant, a period [from, to), or none; `prefix` before the fields' paths.
pub(crate) fn window(
    at: Option<&str>,
    from: Option<&str>,
    to: Option<&str>,
    prefix: &str,
) -> Result<Option<Window>, Stop> {
    let path = |p: &str| format!("{prefix}{p}");
    match (at, from, to) {
        (Some(_), Some(_), _) | (Some(_), _, Some(_)) => Err(refuse(
            codes::INVALID_PERIOD,
            "Geçerlilik zamanı ya bir an (at) ya bir aralıktır (from ve to); ikisi birden verilmez.".into(),
            &path("at"),
        )),
        (Some(a), None, None) => Ok(Some(Window::Instant(moment(a, &path("at"))?))),
        (None, Some(f), Some(t)) => {
            let (a, b) = (moment(f, &path("from"))?, moment(t, &path("to"))?);
            if b <= a {
                return Err(refuse(
                    codes::INVALID_PERIOD,
                    "Aralığın bitişi başlangıcından sonra olmalı; [from, to) bitişi içermez.".into(),
                    &path("to"),
                ));
            }
            Ok(Some(Window::Range(a, b)))
        }
        (None, Some(_), None) | (None, None, Some(_)) => Err(refuse(
            codes::INVALID_PERIOD,
            "Aralığın başlangıcı (from) ve bitişi (to) birlikte verilir.".into(),
            &path(if from.is_some() { "to" } else { "from" }),
        )),
        (None, None, None) => Ok(None),
    }
}

/// A question's time made ready: the fields, the rule, the valid time and the day known.
#[derive(Clone, Debug)]
pub(crate) struct TimeFilter {
    pub fields: LayerTime,
    rule: Rule,
    pub window: Option<Window>,
    pub known_at: Option<f64>,
}

/// One column's side of the condition: its value no later (`Le`) or no earlier (`Ge`) than a day.
#[derive(Clone, Copy)]
enum Side {
    Le,
    Ge,
}

impl TimeFilter {
    /// The question `q` asks with the fields it names, else `own` (a temporal layer's); why not. `layer`: the name a
    /// refusal says.
    pub(crate) fn read(q: &QueryTime, own: Option<&LayerTime>, layer: &str) -> Result<Self, Stop> {
        let fields = match (&q.fields, own) {
            (Some(f), _) => {
                if let Some(why) = f.problem() {
                    return Err(refuse(
                        codes::INVALID_INPUT,
                        format!("Zaman alanları: {why}."),
                        "time.fields",
                    ));
                }
                f.clone()
            }
            (None, Some(f)) => f.clone(),
            (None, None) => {
                return Err(refuse(
                    codes::NOT_TEMPORAL,
                    format!(
                        "{layer} zaman ayarı yok; zaman alanlarını (time.fields: start, isteğe bağlı end, recordedStart) verin."
                    ),
                    "time.fields",
                ));
            }
        };
        let window = window(q.at.as_deref(), q.from.as_deref(), q.to.as_deref(), "time.")?;
        let known_at = match &q.known_at {
            Some(text) => {
                if !fields.recorded() {
                    return Err(refuse(
                        codes::NOT_RECORDED,
                        "Zaman alanları sürümlerin kayıt zamanını tutmuyor; bilinen günle sormak için kayıt başlangıcı alanını (recordedStart) verin.".into(),
                        "time.knownAt",
                    ));
                }
                Some(moment(text, "time.knownAt")?)
            }
            None => None,
        };
        if window.is_none() && known_at.is_none() {
            return Err(refuse(
                codes::INVALID_PERIOD,
                "Zaman sorusunda bir an (at), bir aralık (from, to) ya da bilinen gün (knownAt) verin.".into(),
                "time",
            ));
        }
        Ok(TimeFilter {
            rule: Rule {
                ranged: fields.ranged(),
                cumulative: fields.cumulative,
            },
            fields,
            window,
            known_at,
        })
    }

    /// The fields it reads.
    pub(crate) fn columns(&self) -> Vec<(&'static str, &str)> {
        let f = &self.fields;
        let mut out = vec![("time.fields.start", f.start.as_str())];
        if let Some(e) = f.end.as_deref() {
            out.push(("time.fields.end", e));
        }
        if self.known_at.is_some() {
            if let Some(r) = f.recorded_start.as_deref() {
                out.push(("time.fields.recordedStart", r));
            }
            if let Some(r) = f.recorded_end.as_deref() {
                out.push(("time.fields.recordedEnd", r));
            }
        }
        out
    }

    /// Whether a record with these attributes is of the valid time and known that day.
    pub(crate) fn passes(&self, attrs: &BTreeMap<String, String>) -> bool {
        let read = |key: Option<&str>| {
            key.and_then(|k| attrs.get(k))
                .map_or(Read::Empty, |v| time::read(v))
        };
        let start = read(Some(self.fields.start.as_str()));
        let end = if self.rule.ranged {
            read(self.fields.end.as_deref())
        } else {
            Read::Empty
        };
        // A timeless record shows at any valid time (docs/adr/0210 §4).
        let valid = match (time::object_time(self.rule, start, end), &self.window) {
            (Some(t), Some(w)) => time::shows(&t, w),
            _ => true,
        };
        if !valid {
            return false;
        }
        match self.known_at {
            None => true,
            Some(at) => {
                let recorded = time::recorded_interval(
                    read(self.fields.recorded_start.as_deref()),
                    read(self.fields.recorded_end.as_deref()),
                );
                time::known(recorded, at)
            }
        }
    }

    /// The condition a source is sent and its values (`YYYY-MM-DD` texts); `name` writes a field's column as the SQL
    /// names it (none: not a column of the table), `first` is a Postgres placeholder's first number.
    pub(crate) fn sql(
        &self,
        dialect: Dialect,
        name: &dyn Fn(&str) -> Option<String>,
        first: usize,
    ) -> (String, Vec<String>) {
        let mut parts: Vec<(String, Side, f64)> = Vec::new();
        let mut add = |field: Option<&str>, side: Side, t: f64| {
            if let Some(c) = field.and_then(name) {
                parts.push((c, side, t));
            }
        };
        if let Some(w) = &self.window {
            let (lo, hi) = match *w {
                Window::Instant(a) => (a, a),
                Window::Range(a, b) => (a, b),
            };
            add(Some(self.fields.start.as_str()), Side::Le, hi + DAY);
            if !self.rule.cumulative {
                if self.rule.ranged {
                    add(self.fields.end.as_deref(), Side::Ge, lo - DAY);
                } else {
                    add(Some(self.fields.start.as_str()), Side::Ge, lo - DAY);
                }
            }
        }
        if let Some(k) = self.known_at {
            add(self.fields.recorded_start.as_deref(), Side::Le, k + DAY);
            add(self.fields.recorded_end.as_deref(), Side::Ge, k - DAY);
        }
        let mut sql = Vec::new();
        let mut values = Vec::new();
        for (k, (c, side, t)) in parts.into_iter().enumerate() {
            let op = match side {
                Side::Le => "<=",
                Side::Ge => ">=",
            };
            sql.push(match dialect {
                Dialect::Sqlite => format!(
                    "({c} IS NULL OR {c} NOT GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]*' OR substr({c}, 1, 10) {op} ?)"
                ),
                Dialect::Postgres => format!(
                    "({c} IS NULL OR CAST({c} AS text) !~ '^[0-9]{{4}}-[0-9]{{2}}-[0-9]{{2}}' OR left(CAST({c} AS text), 10) {op} ${})",
                    first + k
                ),
            });
            values.push(time::write(t, true));
        }
        (sql.join(" AND "), values)
    }
}
