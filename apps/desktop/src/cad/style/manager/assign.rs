//! Giving symbols (the web's `assignSymbol`, `style.assign`,
//! `style.clearSymbol` and the layer style slot's menu): the selected
//! objects' own symbol in one undo step through `cad.entities.set`, locked
//! layers skipped and counted; a symbol picked in Stil yöneticisi for a slot
//! of Katman stili; a slot's own symbol kept in Kitaplığım.

use iced::Task;
use kentos_commands::{EntitiesSetProperties, Entity, PropertiesOperation};
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
                line_pattern: None,
                line_type: None,
                line_type_scale: None,
                transparency: None,
                wipeout: None,
                uids: kentos_interaction::properties::uids_of(model, &slots),
                layer_id: None,
                color: None,
                line_weight: None,
                symbol: Some(id.map(str::to_owned)),
                attrs: None,
                label: None,
                operation: PropertiesOperation::Symbol,
                expected_revision: None,
                unlink: false,
                name_text: false,
                attachments: None,
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

    /// The id a drawing uses for the library item `id` (docs/adr/9317): an item of Kitaplığım is applied as the project's
    /// copy (the one there already, else a new one with its drawings), the project's part written into the drawing, so
    /// the paper and another computer draw it too; a system or project item's own id.
    pub(crate) fn project_copy_of(&mut self, id: &str) -> String {
        if self.document.is_none() {
            return id.to_owned();
        }
        let before = self.styles.library.version();
        match self.styles.library.project_copy(id) {
            Ok(own) => {
                if self.styles.library.version() != before {
                    self.library_changed(Source::Project);
                }
                own
            }
            Err(e) => {
                self.warn(e);
                id.to_owned()
            }
        }
    }

    /// Takes the items of Kitaplığım the drawing still refers to into the project under their own ids (docs/adr/9317),
    /// before it is saved, printed or published: the file, the paper and another computer then draw what the screen
    /// draws. Says which were taken.
    pub(crate) fn take_along_styles(&mut self) {
        let Some(doc) = &self.document else {
            return;
        };
        let refs = kentos_native_application::library_refs::library_refs(&doc.model);
        let taken = self.styles.library.take_along(refs);
        if taken.is_empty() {
            return;
        }
        self.library_changed(Source::Project);
        let names: Vec<String> = taken
            .iter()
            .filter_map(|id| self.styles.library.get(id))
            .map(|(item, _)| format!("“{}”", item.name()))
            .collect();
        self.output(format!("Kitaplığım'dan projeye alındı: {}.", names.join(", ")));
    }

    /// Katman stili's “Kitaplığıma kaydet”: the slot's own symbol goes to
    /// Kitaplığım under Sembollerim, named after the slot, and the slot
    /// refers to the project's copy of it from then on (docs/adr/9317).
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
        let own = self.project_copy_of(&id);
        let _ = self.layer_style_event(LayerEvent::Symbol(at, class, Some(json!({ "ref": own }))));
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
            // A symbol of Kitaplığım is given as the project's copy (docs/adr/9317).
            PickTarget::Selection => {
                self.styles.manager = None;
                self.dialog = None;
                let id = self.project_copy_of(&id);
                let text = self.assign_symbol(Some(&id));
                self.output(text);
            }
            PickTarget::Slot(at, class) => {
                self.close_style_manager();
                let id = self.project_copy_of(&id);
                let _ = self.layer_style_event(LayerEvent::Symbol(
                    at,
                    class,
                    Some(json!({ "ref": id })),
                ));
            }
            PickTarget::Template => {
                self.styles.manager = None;
                self.template_symbol_picked(Some(id));
            }
        }
    }
}
