//! Kayma sınırı (plan/map-cad.md MAP-01, docs/adr/5280): the limit Harita işi's shift table sets each changed object's
//! shift against: none, the user's own value, a detail's position, a point's place on the drawing at a scale, or a
//! production method's point position accuracy at a scale. The values, the methods and the scales come from KentOS's
//! position accuracy rules (`kentos_commands::position_accuracy`); a choice that does not hold (a scale the table has no
//! value for) is said under the row and the query runs without a limit. Within a limit is a measure, not a ruling.

use crate::map;
use iced::widget::{column, container, row};
use iced::{Center, Element, Fill};
use kentos_commands::{LimitBasis, ShiftLimit, ShiftLimitUsed};
use kentos_ui::label;

use crate::app::Message;
use crate::map_work_panel::Event;

#[derive(Clone, Debug)]
pub enum LimitEvent {
    Basis(&'static str),
    Metres(String),
    Scale(String),
    Method(String),
}

/// The bases as the row offers them.
const BASES: [crate::style::fields::Choice; 5] = [
    ("none", "Sınır yok"),
    ("own", "Kendi değerim"),
    ("detail", "Detay konumu"),
    ("drawing", "Çizim doğruluğu"),
    ("method", "Üretim yöntemi"),
];

/// What the row holds: the basis, the own value as typed, the scale and the method.
#[derive(Clone, Debug)]
pub struct LimitPick {
    pub basis: &'static str,
    pub metres: String,
    pub scale: u32,
    pub method: String,
}

impl Default for LimitPick {
    fn default() -> Self {
        LimitPick { basis: "none", metres: "0,05".into(), scale: 1000, method: "sayisal".into() }
    }
}

impl LimitPick {
    /// The query's limit, none for “Sınır yok”.
    pub fn input(&self) -> Option<ShiftLimit> {
        let basis = match self.basis {
            "own" => LimitBasis::Own,
            "detail" => LimitBasis::Detail,
            "drawing" => LimitBasis::Drawing,
            "method" => LimitBasis::Method,
            _ => return None,
        };
        Some(ShiftLimit {
            basis,
            metres: (basis == LimitBasis::Own).then(|| self.metres.trim().replace(',', ".").parse().unwrap_or(f64::NAN)),
            scale: matches!(basis, LimitBasis::Drawing | LimitBasis::Method).then_some(self.scale),
            method: (basis == LimitBasis::Method).then(|| self.method.clone()),
        })
    }

    /// The limit's value and source, or why it does not hold; none for “Sınır yok”.
    pub fn used(&self) -> Option<Result<ShiftLimitUsed, String>> {
        self.input().map(|l| kentos_commands::limit_metres(&l).map_err(|(_, why)| why))
    }

    /// The limit the query takes: one that holds.
    pub fn holding(&self) -> Option<ShiftLimit> {
        self.input().filter(|l| kentos_commands::limit_metres(l).is_ok())
    }

    /// A key of the choice, for the panel's kept read.
    pub fn key(&self) -> String {
        format!("{:?}", self.holding())
    }

    pub fn apply(&mut self, e: LimitEvent) {
        match e {
            LimitEvent::Basis(b) => self.basis = b,
            LimitEvent::Metres(m) => self.metres = m,
            LimitEvent::Scale(s) => {
                if let Ok(n) = s.parse() {
                    self.scale = n;
                }
            }
            LimitEvent::Method(m) => self.method = m,
        }
    }
}

fn msg(e: LimitEvent) -> Message {
    Message::Map(map::Event::MapWork(Event::Limit(e)))
}

/// A metre value as Turkish writes it.
pub fn metres_words(m: f64) -> String {
    format!("{m}").replace('.', ",")
}

/// The row: the basis, then what it takes (the value, the method, the scale), the value and its source under it.
pub fn view(pick: &LimitPick) -> Element<'_, Message> {
    let rules = kentos_commands::position_accuracy();
    let mut line = row![
        container(label::caption("Kayma sınırı")).width(84),
        container(crate::style::fields::select(&BASES, pick.basis, |b| msg(LimitEvent::Basis(b)))).width(Fill),
    ]
    .spacing(4)
    .align_y(Center);
    let scales: Vec<(String, String)> = rules
        .as_ref()
        .map(|r| r.table.scales.iter().map(|s| (s.to_string(), format!("1/{s}"))).collect())
        .unwrap_or_default();
    let mut second = None;
    match pick.basis {
        "own" => {
            line = line.push(
                container(
                    crate::style::fields::input(None, "0,05", &pick.metres, false, pick.input().is_some_and(|l| kentos_commands::limit_metres(&l).is_err()))
                        .on_input(|m| msg(LimitEvent::Metres(m))),
                )
                .width(64),
            );
            line = line.push(label::caption("m"));
        }
        "drawing" => {
            line = line.push(
                container(crate::style::fields::select_owned(scales, &pick.scale.to_string(), |s| msg(LimitEvent::Scale(s))))
                    .width(84),
            );
        }
        "method" => {
            let methods: Vec<(String, String)> = rules
                .as_ref()
                .map(|r| r.table.methods.iter().map(|m| (m.id.clone(), m.title.clone())).collect())
                .unwrap_or_default();
            second = Some(
                row![
                    container(label::caption("")).width(84),
                    container(crate::style::fields::select_owned(methods, &pick.method, |m| msg(LimitEvent::Method(m))))
                        .width(Fill),
                    container(crate::style::fields::select_owned(scales, &pick.scale.to_string(), |s| msg(LimitEvent::Scale(s))))
                        .width(84),
                ]
                .spacing(4)
                .align_y(Center),
            );
        }
        _ => {}
    }
    let mut all = column![line].spacing(4);
    if let Some(s) = second {
        all = all.push(s);
    }
    match pick.used() {
        Some(Ok(u)) => {
            // One line in the narrow dock: the value and what follows the article; the whole source in the tip.
            let short = u.source.rsplit_once(": ").map_or(u.source.as_str(), |(_, w)| w);
            let line = label::caption(format!("{} m · {short}", metres_words(u.metres))).style(kentos_ui::style::text::muted);
            all = all.push(kentos_ui::widget::tip(
                line,
                kentos_ui::widget::Tip::new(u.source.clone()),
                iced::widget::tooltip::Position::Bottom,
            ));
        }
        Some(Err(why)) => all = all.push(label::caption(why).style(kentos_ui::style::text::danger)),
        None => {}
    }
    all.into()
}
