//! Records without a geometry (docs/adr/0410 §4): Kayıt ekle in the Tablo tab, and the form's new child
//! record, written through `cad.records.create`.

use std::collections::BTreeMap;

use iced::Task;
use kentos_domain::Slot;
use kentos_interaction::Level;
use kentos_native_application::ExecutionContext;

use crate::app::{App, Message};

impl App {
    /// A record on `layer` with `attrs` (`cad.records.create`), one undo step; its slot, or why not (said).
    pub(crate) fn add_record(
        &mut self,
        layer: &str,
        attrs: BTreeMap<String, String>,
    ) -> Option<Slot> {
        let input = serde_json::json!({
            "layerId": layer,
            "rows": [{ "attrs": attrs }],
        });
        let doc = self.document.as_mut()?;
        let answer = kentos_native_application::registry::run(
            &mut ExecutionContext::new(&mut doc.model),
            kentos_contracts::CAD_RECORDS_CREATE,
            kentos_contracts::CallOp::Execute,
            input,
        );
        match answer {
            Ok(v) if v["status"] == "completed" => {
                let slot = v["output"]["ids"][0].as_u64().map(|s| Slot(s as u32));
                for w in v["warnings"].as_array().into_iter().flatten() {
                    if let Some(m) = w["message"].as_str() {
                        self.say(Level::Warn, m.to_owned());
                    }
                }
                slot
            }
            Ok(v) => {
                let why = v["error"]["message"].as_str().unwrap_or("").to_owned();
                self.say(Level::Warn, format!("Kayıt eklenemedi: {why}"));
                None
            }
            Err(e) => {
                self.say(Level::Warn, format!("Kayıt eklenemedi: {e:?}"));
                None
            }
        }
    }

    /// Whether new objects of `layer` are records: it holds records and nothing drawn.
    pub(crate) fn layer_holds_records(&self, layer: &str) -> bool {
        let Some(doc) = self.document.as_ref() else {
            return false;
        };
        let mut records = false;
        for (_, e) in doc.model.by_layer_with_uids(layer) {
            if e.has_geometry() {
                return false;
            }
            records = true;
        }
        records
    }

    /// Kayıt ekle: an empty record on the table's layer, selected; its form opens when the layer has one,
    /// else the table shows it.
    pub(crate) fn feature_add_record(&mut self) -> Task<Message> {
        let layer = self
            .document
            .as_ref()
            .and_then(|doc| self.feature_layer(doc));
        let Some(layer) = layer else {
            return Task::none();
        };
        let Some(slot) = self.add_record(&layer, BTreeMap::new()) else {
            return Task::none();
        };
        self.selection.set([slot]);
        self.say(Level::Success, "Kayıt eklendi.");
        if let Some(form) = self.form_of_layer(&layer) {
            return self.open_entry(slot, &form);
        }
        Task::none()
    }
}
