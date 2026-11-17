//! A number field's units on the panel (docs/adr/8057 §64): the number may be typed in any of the field's `units`, as
//! KentOS's own number fields take it (`kentos_ui::widget::number::evaluate`: 0,03 km, 2 dönüm, 1 m + 20 cm), and goes
//! to the code in the field's own unit, which stays written after the number. No choice beside it: a unit select
//! took the number's place in a 300-pixel dock (real window), and KentOS's number fields take a unit as it is typed.

use kentos_extensions::ui_panel::{Component, units};
use kentos_ui::widget::number::{self, Unit};

use super::panels::OpenPanel;

impl OpenPanel {
    /// Number field `id`'s units as written, its own first; none when it names no `units`.
    fn units_of(&self, id: &str) -> Option<(&str, &[String])> {
        match self.definition.body.iter().find(|c| c.id() == Some(id)) {
            Some(Component::NumberField {
                unit: Some(own),
                units,
                ..
            }) if !units.is_empty() => Some((own, units)),
            _ => None,
        }
    }

    /// The number typed in field `id`, in its own unit; none when it is not one. In KentOS's number grammar when the
    /// field names units (a unitless number is in its own), else a plain number (a decimal comma or point).
    pub(crate) fn typed_number(&self, id: &str, text: &str) -> Option<f64> {
        let Some((own, list)) = self.units_of(id) else {
            return super::panels::number_of(text);
        };
        let (own, _, size) = units::unit(own)?;
        let mut taken = vec![Unit::new(own, 1.0)];
        taken.extend(
            list.iter()
                .filter_map(|u| units::unit(u))
                .filter(|(symbol, _, _)| *symbol != own)
                .map(|(symbol, _, other)| Unit::new(symbol, other / size)),
        );
        number::evaluate(text, &taken)
            .ok()
            .filter(|n| n.is_finite())
            .map(units::tidy)
    }

    /// What number field `id`'s empty box says: the units it takes ("m ya da km"); empty when it names none.
    pub(crate) fn units_hint(&self, id: &str) -> String {
        match self.units_of(id) {
            Some((_, [only @ .., last])) => format!("{} ya da {last}", only.join(", ")),
            _ => String::new(),
        }
    }

    /// The numbers its command takes, shown in their fields' own units: "0,03 km" typed is "30" before "m" once
    /// taken, as KentOS's number fields write a value once it is entered; the units typed beside the field's own
    /// read twice ("0,03 km m"). A field without units keeps what was typed.
    pub(crate) fn show_in_own_units(&mut self, values: &serde_json::Value) {
        let taken: Vec<(String, f64)> = self
            .definition
            .body
            .iter()
            .filter_map(Component::id)
            .filter(|id| self.units_of(id).is_some())
            .filter_map(|id| Some((id.to_owned(), values.get(id)?.as_f64()?)))
            .collect();
        for (id, n) in taken {
            self.texts.insert(id, super::panels::number_words(n));
        }
    }

    /// Number field `id`'s units named in its wrong value's words ("m, km"); none when it names none.
    pub(crate) fn units_words(&self, id: &str) -> Option<String> {
        self.units_of(id).map(|(_, list)| list.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use kentos_extensions::ui_panel;
    use serde_json::json;

    use super::super::panels::OpenPanel;

    fn panel() -> OpenPanel {
        let value = json!({ "format": ui_panel::FORMAT, "schemaVersion": ui_panel::VERSION, "title": "Yakın yapılar",
            "body": [
                { "type": "numberField", "id": "uzaklik", "label": "Uzaklık", "unit": "m", "units": ["m", "km"],
                  "value": 25, "min": 1, "max": 1000, "step": 5, "required": true },
                { "type": "numberField", "id": "alan", "label": "Alan", "unit": "dönüm", "units": ["m²", "dönüm", "ha"] },
                { "type": "numberField", "id": "adet", "label": "Adet", "unit": "m", "value": 3 }
            ] });
        OpenPanel::new(
            "tr.ornek.deneme",
            "yakin",
            ui_panel::read(&value).expect("a panel"),
        )
    }

    /// A number typed in another of the field's units goes in its own (0,03 km is 30 m; 1 km + 20 m adds; cm is not
    /// one of its units); its bounds are said with its unit; a field without units reads a plain number as before.
    #[test]
    fn a_number_typed_in_another_unit_goes_in_the_fields_own() {
        let mut p = panel();
        assert_eq!(p.typed_number("uzaklik", "0,03 km"), Some(30.0));
        assert_eq!(p.typed_number("uzaklik", "1,1 km"), Some(1100.0));
        assert_eq!(
            p.typed_number("uzaklik", "25"),
            Some(25.0),
            "unitless: its own"
        );
        assert_eq!(p.typed_number("uzaklik", "1 km + 20 m"), Some(1020.0));
        assert_eq!(
            p.typed_number("uzaklik", "20 cm"),
            None,
            "not one of its units"
        );
        assert_eq!(p.typed_number("alan", "5000 m²"), Some(5.0));
        assert_eq!(p.typed_number("alan", "1 ha"), Some(10.0));
        assert_eq!(p.typed_number("adet", "2,5"), Some(2.5));
        assert_eq!(
            p.typed_number("adet", "2 km"),
            None,
            "no units: a plain number"
        );
        assert_eq!(p.units_hint("uzaklik"), "m ya da km");
        assert_eq!(p.units_hint("alan"), "m², dönüm ya da ha");
        assert_eq!(p.units_hint("adet"), "");
        p.texts.insert("uzaklik".into(), "0,03 km".into());
        let values = p.check_params().expect("its values");
        assert_eq!(
            (values["uzaklik"].clone(), values["adet"].clone()),
            (json!(30.0), json!(3.0))
        );
        assert_eq!(p.rule_value("uzaklik").as_deref(), Some("30"));
        p.texts.insert("uzaklik".into(), "2 km".into());
        assert_eq!(
            p.check_params().unwrap_err(),
            (
                "uzaklik".to_owned(),
                "“Uzaklık” 1\u{a0}m ile 1000\u{a0}m arasında olmalı.".to_owned()
            )
        );
        p.texts.insert("uzaklik".into(), "2 mil".into());
        assert_eq!(
            p.check_params().unwrap_err().1,
            "“Uzaklık”: “2 mil” okunamadı: bir sayı ya da birimiyle yazılmış bir sayı olmalı (m, km)."
        );
        p.texts.insert("uzaklik".into(), "0,03 km".into());
        p.step("uzaklik", 1);
        assert_eq!(p.texts["uzaklik"], "35", "a step from 30 m, in metres");
        // Once taken it shows in metres; a field without units keeps its words.
        p.texts.insert("uzaklik".into(), "0,03 km".into());
        p.texts.insert("adet".into(), "3,0".into());
        let values = p.check_params().expect("its values");
        p.show_in_own_units(&values);
        assert_eq!(
            (p.texts["uzaklik"].as_str(), p.texts["adet"].as_str()),
            ("30", "3,0")
        );
    }
}
