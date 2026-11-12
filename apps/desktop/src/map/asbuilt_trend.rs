//! Uygulama kontrolü › İzleme's trend and thresholds (docs/adr/4840; `cad.asbuilt.check`'s `trend` and `thresholds`):
//! Doğru uydurma, a line fitted through each mark's dated epochs at the confidence typed; Uyarı eşiği and Alarm eşiği,
//! each the horizontal and vertical movement (m, as the window's movement limit) and the horizontal and vertical speed
//! (m a year) beyond which a mark is at that level. Every value is the user's: an empty box is not checked, and no
//! regulation gives one for monitoring; their source is typed beside them and kept with a review (docs/adr/5220). Under
//! the series a choice shows its movements (docs/adr/4420), each mark's line and state, or one mark's chart, one at a
//! time.

use iced::widget::{container, row};
use iced::{Center, Element};
use kentos_commands::{AsbuiltLevel, AsbuiltState, AsbuiltThresholds, AsbuiltTrack, AsbuiltTrendAsk};
use kentos_ui::label;

use crate::app::Message;
use crate::exchange::words;
use crate::road_dialog::{col, num};
use crate::style::fields;

/// The form's values, as typed.
#[derive(Clone, Debug, Default)]
pub struct TrendForm {
    /// Doğru uydurma: the line asked.
    pub on: bool,
    /// The confidence in per cent (95).
    pub confidence: String,
    /// Uyarı and Alarm: horizontal and vertical movement (m), horizontal and vertical speed (m a year).
    pub warning: [String; 4],
    pub alarm: [String; 4],
    /// Who set the thresholds, by which document or decision, when (docs/adr/5220): kept with a review.
    pub source: String,
    /// The series' table shown.
    pub view: View,
    /// Grafik's mark (empty: the first in alarm, else the first) and part (docs/adr/5220).
    pub mark: String,
    pub part: crate::asbuilt_chart::Part,
}

/// A change of the form.
#[derive(Clone, Debug)]
pub enum Change {
    On(bool),
    Confidence(String),
    Warning(usize, String),
    Alarm(usize, String),
    Source(String),
    /// The table under the series, Grafik's mark and part: shown only, nothing checked again.
    View(View),
    Mark(String),
    Part(crate::asbuilt_chart::Part),
}

/// The series' views, one at a time: the movement at each epoch (docs/adr/4420), each mark's line and state, or one
/// mark's movement against time (docs/adr/5220).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum View {
    Series,
    #[default]
    Line,
    Chart,
}

impl std::fmt::Display for View {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            View::Series => "Hareketler",
            View::Line => "Doğru ve eşikler",
            View::Chart => "Grafik",
        })
    }
}

impl TrendForm {
    /// As the window opens: the line off, its confidence 95 % ready (the user's to change), no threshold.
    pub fn new() -> TrendForm {
        TrendForm { confidence: "95".into(), ..TrendForm::default() }
    }

    pub fn apply(&mut self, c: Change) {
        match c {
            Change::On(v) => self.on = v,
            Change::Confidence(v) => self.confidence = v,
            Change::Warning(k, v) => self.warning[k] = v,
            Change::Alarm(k, v) => self.alarm[k] = v,
            Change::Source(v) => self.source = v,
            Change::View(v) => self.view = v,
            Change::Mark(v) => self.mark = v,
            Change::Part(v) => self.part = v,
        }
    }
}

/// The limits' words, in the boxes' order.
const WORDS: [&str; 4] = ["yatay hareketi", "düşey hareketi", "yatay hızı", "düşey hızı"];

/// A box's number: empty is none, a comma is a point; other text is the field's error.
fn number(text: &str, what: &str) -> Result<Option<f64>, String> {
    let t = text.trim().replace(',', ".");
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>().ok().filter(|v| v.is_finite()).map(Some).ok_or_else(|| format!("{what} “{text}” bir sayı değil."))
}

/// A level from its four boxes (m, m a year).
fn level(boxes: &[String; 4], name: &str) -> Result<AsbuiltLevel, String> {
    let m = |k: usize| number(&boxes[k], &format!("{name} eşiğinin {}", WORDS[k]));
    Ok(AsbuiltLevel { horizontal: m(0)?, vertical: m(1)?, horizontal_rate: m(2)?, vertical_rate: m(3)? })
}

/// The command's `trend` and `thresholds` from the form (a monitoring job with a series only; the caller asks).
pub fn ask(f: &TrendForm) -> Result<(Option<AsbuiltTrendAsk>, Option<AsbuiltThresholds>), String> {
    let trend = if f.on {
        let c = number(&f.confidence, "Güven düzeyi")?.ok_or("Güven düzeyini yüzde olarak yazın (ör. 95).")?;
        Some(AsbuiltTrendAsk { confidence: c / 100.0 })
    } else {
        None
    };
    let any = f.warning.iter().chain(&f.alarm).any(|v| !v.trim().is_empty());
    let thresholds =
        if any { Some(AsbuiltThresholds { warning: level(&f.warning, "Uyarı")?, alarm: level(&f.alarm, "Alarm")? }) } else { None };
    Ok((trend, thresholds))
}

/// A number box of the window's width (the movement limits' 80 px).
fn number_box<'a>(value: &'a str, to: impl Fn(String) -> Message + 'a) -> Element<'a, Message> {
    container(fields::input(None, "—", value, false, false).on_input(to)).width(80).into()
}

/// The form's rows: Doğru uydurma, then each level's horizontal and vertical movement (m) and, after a caption, its
/// horizontal and vertical speed (m a year). Every row starts with its box; two values of one thing share a row.
pub fn rows<'a>(f: &'a TrendForm, on: fn(Change) -> Message) -> Vec<(&'static str, Element<'a, Message>)> {
    let line = row![
        words::check(f.on, "noktanın günlü dönemlerinden en küçük karelerle", Some(on(Change::On(!f.on)))),
        label::caption("güven %"),
        number_box(&f.confidence, move |v| on(Change::Confidence(v)))
    ]
    .spacing(8)
    .align_y(Center);
    let level = |boxes: &'a [String; 4], change: fn(usize, String) -> Change| -> Element<'a, Message> {
        let b = |k: usize| number_box(&boxes[k], move |v| on(change(k, v)));
        row![b(0), b(1), label::caption("hız (m/yıl)"), b(2), b(3)].spacing(8).align_y(Center).into()
    };
    vec![
        ("Doğru uydurma", line.into()),
        ("Uyarı eşiği yatay / düşey (m)", level(&f.warning, Change::Warning)),
        ("Alarm eşiği yatay / düşey (m)", level(&f.alarm, Change::Alarm)),
        (
            "Eşiklerin kaynağı",
            container(
                fields::input(None, "Kim, hangi belge ya da kararla, ne zaman", &f.source, false, false)
                    .on_input(move |v| on(Change::Source(v))),
            )
            .width(520)
            .into(),
        ),
    ]
}

/// Whether the answer has a line or a state to show: the choice of [`View`] is offered then.
pub fn shown(tracks: &[AsbuiltTrack]) -> bool {
    tracks.iter().any(|t| t.fit.is_some() || t.state.is_some())
}

/// The line table's lead: what its numbers are.
pub fn lead(tracks: &[AsbuiltTrack], confidence: &str) -> String {
    let mut said = String::from("Doğru ve eşikler:");
    if tracks.iter().any(|t| t.fit.is_some()) {
        said.push_str(&format!(
            " hızlar m/yıl, ± standart sapmaları; anlamlı hareket %{} güvenle Student t sınamasından.",
            confidence.trim()
        ));
    }
    if tracks.iter().any(|t| t.state.is_some()) {
        said.push_str(" Durum son hareketin ve hızın (doğrununki, yoksa ortalama) eşiklere göre.");
    }
    said
}

/// The line table: each mark's epochs in its line, its velocities east, north and up ± their σ (m a year), the
/// horizontal speed its state was judged by (the line's, else the mean), the components that move and its state.
pub fn table(tracks: &[AsbuiltTrack]) -> Element<'_, Message> {
    // Narrow enough for the window's 1000 px: “+0,0533 ±0,0067” and “Alarm (yatay, yatay hız)” fit their columns.
    let columns = vec![
        col("Nokta", 70.0),
        num("Dönem", 56.0),
        num("Doğu m/yıl", 118.0),
        num("Kuzey m/yıl", 118.0),
        num("Kot m/yıl", 118.0),
        num("Yatay hız m/yıl", 100.0),
        col("Anlamlı hareket", 110.0),
        col("Durum", 150.0),
    ];
    crate::road_dialog::sheet(columns, tracks.iter().map(cells).collect(), "İzlenen nokta yok.", 200.0)
}

/// A mark's row of [`table`].
pub fn cells(t: &AsbuiltTrack) -> Vec<String> {
    let comma = |s: String| s.replace('.', ",");
    // Signed as the movements are, a velocity under half the last digit unsigned (no “−0,0000”).
    let signed = |v: f64| if v.abs() < 0.00005 { "0,0000".to_owned() } else { comma(format!("{v:+.4}")) };
    let pm = |v: f64, s: f64| format!("{} ±{}", signed(v), comma(format!("{s:.4}")));
    let speed = t.fit.as_ref().map(|f| f.speed).or(t.rate_horizontal);
    let mut row = vec![t.name.clone()];
    match &t.fit {
        Some(f) => {
            let moving: Vec<&str> = f
                .moving
                .iter()
                .map(|c| match c.as_str() {
                    "east" => "doğu",
                    "north" => "kuzey",
                    _ => "kot",
                })
                .collect();
            row.extend([
                f.epochs.to_string(),
                pm(f.east, f.sigma_east),
                pm(f.north, f.sigma_north),
                f.up.zip(f.sigma_up).map(|(v, s)| pm(v, s)).unwrap_or_default(),
                speed.map(|v| comma(format!("{v:.4}"))).unwrap_or_default(),
                if moving.is_empty() { "yok".into() } else { moving.join(", ") },
            ]);
        }
        None => row.extend([
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            speed.map(|v| comma(format!("{v:.4}"))).unwrap_or_default(),
            String::new(),
        ]),
    }
    row.push(match t.state {
        Some(AsbuiltState::Alarm) => format!("Alarm ({})", beyond(&t.exceeded)),
        Some(AsbuiltState::Warning) => format!("Uyarı ({})", beyond(&t.exceeded)),
        Some(AsbuiltState::Normal) => "Olağan".into(),
        None => String::new(),
    });
    row
}

fn beyond(keys: &[String]) -> String {
    keys.iter()
        .map(|k| match k.as_str() {
            "horizontal" => "yatay",
            "vertical" => "düşey",
            "horizontalRate" => "yatay hız",
            _ => "düşey hız",
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The summary's line of the states (docs/adr/4840), when thresholds were asked.
pub fn line(tracks: &[AsbuiltTrack]) -> Option<String> {
    let count = |s: AsbuiltState| tracks.iter().filter(|t| t.state == Some(s)).count();
    if tracks.iter().all(|t| t.state.is_none()) {
        return None;
    }
    Some(format!(
        "Eşikler: {} nokta alarmda, {} uyarıda, {} olağan. Eşikler sizindir; izleme için yönetmelik değeri yoktur.",
        count(AsbuiltState::Alarm),
        count(AsbuiltState::Warning),
        count(AsbuiltState::Normal)
    ))
}
