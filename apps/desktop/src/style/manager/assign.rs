//! Giving symbols (the web's `assignSymbol`, `style.assign`,
//! `style.clearSymbol` and the layer style slot's menu): the selected
//! objects' own symbol in one undo step through `cad.entities.set`, locked
//! layers skipped and counted; a symbol picked in Stil yöneticisi for a slot
//! of Katman stili; a slot's own symbol kept in Kitaplığım.

use iced::Task;
use kentos_contracts::{EntitiesSetProperties, Entity, PropertiesOperation};
use kentos_domain::Slot;
use kentos_native_style::library::{Source, new_item_id};
use kentos_native_style::renderer::GeometryClass;
use serde_json::{Value, json};

use super::{KindFilter, Pick, PickTarget};
use crate::app::{App, Message};
use crate::style::layer_style::{Event as LayerEvent, SetAt};

impl App {
    /// Gives (or takes away) the selected objects' own symbol in one undo step,
    /// through `cad.entities.set`; locked layers are skipped and counted (`assignSymbol`).
    pub(crate) fn assign_symbol(&mut self, id: Option<&str>) -> String {
        let name = id.map_or_else(String::new, |id| {
            self.styles
                .library
                .get(id)
                .map_or_else(|| id.to_owned(), |(i, _)| i.name().to_owned())
        });
        let Some(doc) = &mut self.document else {
            return "Açık çizim yok.".into();
        };
        let model = &mut doc.model;
        let mut locked = 0;
        let mut slots: Vec<Slot> = Vec::new();
        for &slot in self.selection.ids() {
            let Some(e) = model.get(slot) else { continue };
            if e.base().symbol.as_deref() == id {
                continue;
            }
            if model.layers().is_locked(&e.base().layer_id) {
                locked += 1;
            } else {
                slots.push(slot);
            }
        }
        let before = model.generation();
        let mut said = Vec::new();
        if !slots.is_empty() {
            let input = EntitiesSetProperties {
                uids: kentos_interaction::properties::uids_of(model, &slots),
                layer_id: None,
                color: None,
                symbol: Some(id.map(str::to_owned)),
                attrs: None,
                label: None,
                operation: PropertiesOperation::Symbol,
                expected_revision: None,
            };
            // The step keeps the symbol's name, “Sembol: …”: the command knows no library.
            let label = if id.is_some() {
                format!("Sembol: {name}")
            } else {
                "Sembolü kaldır".to_owned()
            };
            said = model
                .transact(&label, |m| {
                    Ok::<_, ()>(kentos_interaction::properties::set_properties(m, input))
                })
                .unwrap_or_default();
        }
        let done = if model.generation() == before {
            0
        } else {
            slots.len()
        };
        let skipped = if locked > 0 {
            format!("; kilitli katmandaki {locked} nesne atlandı")
        } else {
            String::new()
        };
        let mut text = match id {
            Some(_) => format!("{done} nesneye “{name}” verildi{skipped}."),
            None => format!("{done} nesnenin sembolü kaldırıldı{skipped}."),
        };
        // What the command said besides (a hidden layer, a refusal).
        for s in said {
            text.push(' ');
            text.push_str(&s);
        }
        text
    }

    /// `style.assign`: Stil yöneticisi picks a symbol for the selected
    /// objects, of the first one's kind, starting at its symbol.
    pub(crate) fn pick_for_selection(&mut self) -> Task<Message> {
        let first = self
            .selection
            .ids()
            .first()
            .and_then(|&s| self.document.as_ref().and_then(|d| d.model.get(s)))
            .cloned();
        let kind = first
            .as_ref()
            .and_then(kentos_native_style::classify::geometry_class)
            .map(KindFilter::of_class);
        let current = first
            .as_ref()
            .and_then(|e: &Entity| e.base().symbol.clone());
        self.open_style_manager(
            Some(Pick {
                kind,
                title: "Seçili nesnelere sembol verin".into(),
                current,
                target: PickTarget::Selection,
            }),
            None,
        )
    }

    /// Katman stili's “Kitaplıktan seç…”: the manager picks for the slot, over the window.
    pub(crate) fn pick_for_slot(
        &mut self,
        at: SetAt,
        class: GeometryClass,
        title: &str,
        current: Option<String>,
    ) -> Task<Message> {
        self.open_style_manager(
            Some(Pick {
                kind: Some(KindFilter::of_class(class)),
                title: format!("{title}: sembol seçin"),
                current,
                target: PickTarget::Slot(at, class),
            }),
            None,
        )
    }

    /// Katman stili's “Kitaplığıma kaydet”: the slot's own symbol goes to
    /// Kitaplığım under Sembollerim, named after the slot, and the slot
    /// refers to it from then on.
    pub(crate) fn keep_slot_symbol(
        &mut self,
        at: SetAt,
        class: GeometryClass,
        title: &str,
        symbol: Value,
    ) {
        let id = new_item_id("u");
        let item = json!({
            "kind": "symbol",
            "id": id,
            "name": title,
            "path": ["Sembollerim"],
            "symbol": symbol,
        });
        if let Err(e) = self.styles.library.add(Source::User, item) {
            self.warn(e);
            return;
        }
        self.library_changed(Source::User);
        let _ = self.layer_style_event(LayerEvent::Symbol(at, class, Some(json!({ "ref": id }))));
        let text = format!("“{title}” Kitaplığım'a kaydedildi.");
        if let Some(window) = &mut self.styles.layer_style {
            window.say(text.clone(), false);
        }
        self.output(text);
    }

    /// Seç, or a double click in pick mode: the symbol goes where it was asked for.
    pub(super) fn choose(&mut self) {
        let Some(m) = &self.styles.manager else {
            return;
        };
        let (Some(id), Some(pick)) = (m.selected.clone(), m.pick.clone()) else {
            return;
        };
        if !m.pickable(&self.styles.library, &id) {
            return;
        }
        self.commit_fields();
        match pick.target {
            PickTarget::Selection => {
                self.styles.manager = None;
                self.dialog = None;
                let text = self.assign_symbol(Some(&id));
                self.output(text);
            }
            PickTarget::Slot(at, class) => {
                self.close_style_manager();
                let _ = self.layer_style_event(LayerEvent::Symbol(
                    at,
                    class,
                    Some(json!({ "ref": id })),
                ));
            }
        }
    }
}
