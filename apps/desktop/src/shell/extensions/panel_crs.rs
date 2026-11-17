//! A panel's coordinate system picker (docs/adr/8057 §45): KentOS's systems in a list, each with its datum, its value
//! `EPSG:<code>`. One without a value starts on the open drawing's own system (none on a drawing without one): what an
//! extension most often asks is "in the drawing's system, or another?".

use iced::Element;
use kentos_extensions::ui_panel::Component;
use kentos_ui::widget::select::{Choice, Select};

use super::panels::{OpenPanel, PanelEvent};
use crate::app::Message;

impl OpenPanel {
    /// Each coordinate system picker without a value of its own set to the drawing's system `srid`; the local system
    /// (and no drawing) leaves it empty.
    pub(crate) fn default_crs(&mut self, srid: Option<u32>) {
        let Some(srid) = srid.filter(|s| *s != crate::crs::LOCAL_SRID) else {
            return;
        };
        for c in &self.definition.body {
            if let Component::CrsPicker {
                id, value: None, ..
            } = c
                && self.choices.get(id).is_none_or(String::is_empty)
            {
                self.choices.insert(id.clone(), format!("EPSG:{srid}"));
            }
        }
    }
}

/// Picker `id` as KentOS draws it: the registry's systems but the local one, in its order (by datum), its value's
/// selected; a system chosen is `event`'s, as a choice of a select.
pub(super) fn crs_select<'a>(
    p: &OpenPanel,
    id: &str,
    event: fn(PanelEvent) -> Message,
) -> Element<'a, Message> {
    let systems: Vec<&crate::crs::System> = crate::crs::systems()
        .iter()
        .filter(|s| s.srid != crate::crs::LOCAL_SRID)
        .collect();
    let chosen = p
        .choices
        .get(id)
        .and_then(|v| kentos_extensions::ui_panel::crs::code_of(v));
    let at = systems.iter().position(|s| Some(s.srid) == chosen);
    let labels = systems
        .iter()
        .map(|s| Choice::new(crate::crs::title(s)).detail(crate::crs::datum_label(&s.datum)));
    let codes: Vec<u32> = systems.iter().map(|s| s.srid).collect();
    let id = id.to_owned();
    Select::new(labels, at, move |i| {
        event(PanelEvent::Choose(id.clone(), format!("EPSG:{}", codes[i])))
    })
    .into()
}

#[cfg(test)]
mod tests {
    use kentos_extensions::ui_panel;
    use serde_json::json;

    /// A picker without a value starts on the drawing's system, one with a value keeps it, the local system leaves it
    /// empty (asked when required); its value goes to the command as written.
    #[test]
    fn a_crs_picker_starts_on_the_drawings_system_and_gives_its_code() {
        let value = json!({ "format": ui_panel::FORMAT, "schemaVersion": ui_panel::VERSION, "title": "Sistem",
            "body": [
                { "type": "crsPicker", "id": "kaynak", "label": "Dosyanın sistemi", "required": true },
                { "type": "crsPicker", "id": "hedef", "label": "Hedef", "value": "EPSG:4326" }
            ] });
        let open = || {
            super::OpenPanel::new(
                "tr.ornek.deneme",
                "sistem",
                ui_panel::read(&value).expect("a panel"),
            )
        };
        let mut p = open();
        p.default_crs(Some(crate::crs::LOCAL_SRID));
        assert_eq!(
            p.check_params().unwrap_err(),
            (
                "kaynak".to_owned(),
                "“Dosyanın sistemi” seçilmedi.".to_owned()
            )
        );
        let mut p = open();
        p.default_crs(Some(5256));
        let params = p.check_params().expect("the values");
        assert_eq!(params["kaynak"], "EPSG:5256");
        assert_eq!(params["hedef"], "EPSG:4326", "its own value kept");
        let _ = super::crs_select(&p, "kaynak", |e| {
            super::super::msg(super::super::Event::Panel(e))
        });
    }
}
