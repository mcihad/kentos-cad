//! A panel's worked-out values on the desktop (docs/adr/8057 §35; ui_panel::values): `defaultExpr` fills a field when
//! the panel opens, `calculate` writes a component's value again whenever the user, the code or another calculation
//! changes an input. No Python: the expressions run here, in the order each comes after those it reads.
//!
//! - **A result in its field's words:** a number as the field writes one (`2,5`), a date `GG.AA.YYYY`, a time `SS:DD`,
//!   a moment `GG.AA.YYYY SS:DD`, empty when it is not one of those; `şimdi()` given to a date or a time field gives
//!   its day or its hour.
//! - **A calculated field is not typed** (panels.rs `usable`) and its code does not set it (panel_session.rs); a
//!   calculated text's words are kept by its place, its `text` shown while it gives nothing.

use kentos_expression::Value;
use kentos_extensions::ui_panel::values::{self, Moment};
use kentos_extensions::ui_panel::{Component, typed_date, typed_date_time, typed_time};

use super::panels::{OpenPanel, date_time_words, date_words, number_words};

/// The moment the panel opens, as its expressions read `bugün()` and `şimdi()`: the computer's local day and time.
pub(super) fn moment_now() -> Moment {
    let now = kentos_ui::attribute::DateTime::now(
        kentos_interaction::time_version::LOCAL_OFFSET.load(std::sync::atomic::Ordering::Relaxed)
            / 60,
    );
    let (d, t) = (now.date, now.time);
    let today = format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day());
    Moment {
        now: format!(
            "{today}T{:02}:{:02}:{:02}",
            t.hour(),
            t.minute(),
            t.second()
        ),
        today,
    }
}

/// A worked-out value in component `c`'s words, as its field holds what the user types; none when it is not one.
fn worked_words(c: &Component, value: &Value) -> Option<String> {
    let text = kentos_expression::value::to_text(value);
    let text = text.trim();
    match c {
        Component::NumberField { .. } => {
            kentos_expression::value::to_number(value).map(number_words)
        }
        Component::DateField { .. } => typed_date(text)
            .or_else(|| text.get(..10).and_then(typed_date))
            .map(|iso| date_words(&iso)),
        Component::TimeField { .. } => typed_time(text).or_else(|| {
            let clock = text.split_once('T').map_or(text, |(_, t)| t);
            clock.get(..5).and_then(typed_time)
        }),
        Component::DateTimeField { .. } => typed_date_time(text)
            .or_else(|| text.get(..16).and_then(typed_date_time))
            .map(|iso| date_time_words(&iso)),
        _ => Some(text.to_owned()),
    }
}

impl OpenPanel {
    /// The fields' `defaultExpr`s worked out, in the body's order (each reading the values above it as they are now).
    pub(super) fn apply_defaults(&mut self, moment: &Moment) {
        let body: Vec<serde_json::Value> = self
            .definition
            .body
            .iter()
            .filter_map(|c| serde_json::to_value(c).ok())
            .collect();
        let inputs = kentos_extensions::ui_panel::rules::inputs_of(&body);
        for at in 0..self.definition.body.len() {
            let Some(source) = self
                .definition
                .rules
                .get(at)
                .and_then(|r| r.default_expr.clone())
            else {
                continue;
            };
            let Ok(expr) = values::compile(&source, &inputs, moment) else {
                continue;
            };
            let c = &self.definition.body[at];
            let words =
                values::value_of(&expr, |id| self.rule_value(id)).and_then(|v| worked_words(c, &v));
            if let (Some(id), Some(words)) = (c.id().map(str::to_owned), words) {
                self.texts.insert(id, words);
            }
        }
    }

    /// The calculations worked out again from the panel's values now, each after those it reads.
    pub(crate) fn recalculate(&mut self) {
        for at in self.calc_order.clone() {
            let Some(expr) = self.rules.get(at).and_then(|r| r.calculate.as_ref()) else {
                continue;
            };
            let value = values::value_of(expr, |id| self.rule_value(id));
            let c = &self.definition.body[at];
            match c {
                Component::Text { .. } => {
                    match value.map(|v| kentos_expression::value::to_text(&v).into_owned()) {
                        Some(words) => self.calculated.insert(at, words),
                        None => self.calculated.remove(&at),
                    };
                }
                _ => {
                    let words = value.and_then(|v| worked_words(c, &v)).unwrap_or_default();
                    if let Some(id) = c.id().map(str::to_owned) {
                        self.texts.insert(id, words);
                    }
                }
            }
        }
    }

    /// Whether component `id` is calculated: the user does not type it, its code does not set it.
    pub(crate) fn calculated_id(&self, id: &str) -> bool {
        self.definition
            .body
            .iter()
            .position(|c| c.id() == Some(id))
            .is_some_and(|at| self.calculated_at(at))
    }

    /// Whether component `at` is calculated.
    pub(crate) fn calculated_at(&self, at: usize) -> bool {
        self.definition
            .rules
            .get(at)
            .is_some_and(|r| r.calculate.is_some())
    }
}

#[cfg(test)]
mod tests {
    use kentos_extensions::ui_panel;
    use serde_json::json;

    use super::*;

    fn panel() -> OpenPanel {
        let value = json!({ "format": ui_panel::FORMAT, "schemaVersion": ui_panel::VERSION, "title": "Numarala",
            "body": [
                { "type": "textField", "id": "onek", "label": "Önek", "value": "Y-" },
                { "type": "numberField", "id": "ilk", "label": "İlk numara", "value": 1 },
                { "type": "numberField", "id": "adet", "label": "Adet", "defaultExpr": "ilk + 4" },
                { "type": "textField", "id": "son", "label": "Son numara",
                  "calculate": "onek || doldur(ilk + adet - 1, 2)" },
                { "type": "numberField", "id": "yarim", "label": "Yarısı", "calculate": "adet / 2" },
                { "type": "dateField", "id": "gun", "label": "Gün", "defaultExpr": "bugün()" },
                { "type": "timeField", "id": "saat", "label": "Saat", "calculate": "şimdi()" },
                { "type": "text", "id": "ozet", "text": "Bir önek yazın.",
                  "calculate": "eğer(boş(onek), boş, onek || doldur(ilk, 2) || ' … ' || son || ' yazılır.')" },
                { "type": "button", "id": "numarala", "label": "Numarala", "command": "numarala" }
            ] });
        OpenPanel::new(
            "tr.ornek.deneme",
            "numarala",
            ui_panel::read(&value).expect("a panel"),
        )
    }

    /// docs/adr/8057 §35: a default is worked out when the panel opens (reading the values above it, `bugün()` the
    /// day it opened), a calculation then and whenever a value it reads changes; a calculated field is not typed in
    /// and its value goes to the command as any input's.
    #[test]
    fn defaults_fill_the_panel_when_it_opens_and_calculations_follow_its_values() {
        let mut p = panel();
        let today = moment_now().today;
        assert_eq!(p.texts["adet"], "5", "ilk + 4");
        assert_eq!(p.texts["son"], "Y-05");
        assert_eq!(p.texts["yarim"], "2,5", "a number in the field's words");
        assert_eq!(p.texts["gun"], date_words(&today));
        assert_eq!(
            p.texts["saat"].len(),
            5,
            "şimdi() in a time field: its hour"
        );
        let ozet = p
            .definition
            .body
            .iter()
            .position(|c| c.id() == Some("ozet"))
            .unwrap();
        assert_eq!(p.calculated[&ozet], "Y-01 … Y-05 yazılır.");
        // The user changes what they read: the calculations follow, the default does not come again.
        p.texts.insert("onek".into(), "B-".into());
        p.texts.insert("ilk".into(), "10".into());
        p.recalculate();
        assert_eq!(p.texts["adet"], "5", "a default is the first value only");
        assert_eq!(p.texts["son"], "B-14");
        assert_eq!(p.calculated[&ozet], "B-10 … B-14 yazılır.");
        // Nothing to say: the text's own words show.
        p.texts.insert("onek".into(), String::new());
        p.recalculate();
        assert!(!p.calculated.contains_key(&ozet));
        // Not typed in; read as any input by the command.
        let son = p
            .definition
            .body
            .iter()
            .position(|c| c.id() == Some("son"))
            .unwrap();
        assert!(!p.usable(son, &p.definition.body[son].clone()));
        assert!(p.calculated_id("son") && !p.calculated_id("onek"));
        let params = p.params().expect("the values");
        assert_eq!(params["son"], json!("14"));
        assert_eq!(params["yarim"], json!(2.5));
        assert_eq!(params["gun"], json!(today));
    }
}
