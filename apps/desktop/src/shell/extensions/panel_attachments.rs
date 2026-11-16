//! A panel's attachments (plan/eklenti.md ADIM 22, the inventory's "ek"; docs/adr/8057 §68): the photos, documents
//! and addresses of the record a picker holds, as Öznitelikler shows them (attachments.rs): a photo's picture once
//! decoded, decoded off the thread with the others the shell shows, else its kind's icon; Aç opens one (the photo
//! viewer, the system's opener). Dosya ekle… and Adres ekle… are KentOS's own, on a record whose layer is not locked:
//! the user's edit, one undo step. No Python and no capability: the user adds, KentOS writes.

use iced::widget::{button, column, container, row};
use iced::{Element, Fill};
use kentos_domain::Slot;
use kentos_extensions::ui_panel::Component;
use kentos_ui::label;

use super::panels::OpenPanel;
use crate::app::{App, Message};
use crate::attachments::Event;

/// A picture's size in a row, as Öznitelikler's.
const W: f32 = 36.0;
const H: f32 = 27.0;

impl App {
    /// The record panel component `of` names, when the drawing has it.
    fn panel_record(&self, p: &OpenPanel, of: &str) -> Option<Slot> {
        let uid = kentos_domain::Uuid::parse_str(p.parent_of(of)?).ok()?;
        self.document.as_ref()?.model.slot_of(uid)
    }

    /// The records the shown panel's attachment lists are of: their photos are decoded as the shell's others are.
    pub(crate) fn panel_attachment_slots(&self) -> Vec<Slot> {
        let Some(p) = self.extensions.panels.get(self.extensions.active_panel) else {
            return Vec::new();
        };
        p.definition
            .body
            .iter()
            .filter_map(|c| match c {
                Component::Attachments { of, .. } => self.panel_record(p, of),
                _ => None,
            })
            .collect()
    }
}

/// An attachment list: the record's attachments a row each (its picture or icon, its name and where it is, Aç), and
/// KentOS's Dosya ekle… and Adres ekle… where its layer takes edits; asks for a record first.
pub(super) fn attachments_view<'a>(
    app: &'a App,
    p: &OpenPanel,
    words: Option<&str>,
    of: &str,
) -> Element<'a, Message> {
    let title = label::caption(words.unwrap_or("Ekler").to_owned());
    let Some(slot) = app.panel_record(p, of) else {
        return column![title, label::muted("Önce bir kayıt seçin.")]
            .spacing(4)
            .into();
    };
    let views = app.attachment_views(slot);
    let mut list = column![title].spacing(6);
    if views.is_empty() {
        list = list.push(label::muted("Bu kaydın eki yok."));
    }
    for view in views {
        let open = Message::Attachment(Event::Open(slot, view.index));
        let face: Element<'a, Message> = match app.photo_picture(view.key.as_deref()) {
            Some(picture) => {
                crate::photo_view::photo(Some(picture), W, H, crate::photo_view::Fit::Cover)
            }
            None => container(
                kentos_ui::icon::icon(crate::icons::from_web(Some(crate::attachments::kind_icon(
                    view.kind,
                ))))
                .size(16.0),
            )
            .width(W)
            .height(H)
            .center_x(W)
            .center_y(H)
            .style(kentos_ui::style::container::field)
            .into(),
        };
        let name = if view.attachment.name.trim().is_empty() {
            crate::attachments::kind_word(view.kind).to_owned()
        } else {
            view.attachment.name.clone()
        };
        list = list.push(
            row![
                button(face)
                    .on_press(open.clone())
                    .padding(0)
                    .style(kentos_ui::style::button::ghost),
                column![label::body(name), label::muted(view.source.clone())]
                    .spacing(2)
                    .width(Fill),
                button(label::body("Aç"))
                    .on_press(open)
                    .padding([4, 10])
                    .style(kentos_ui::style::button::secondary),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        );
    }
    if app.attachable(slot) {
        let add = |words: &'static str, e: Event| {
            button(label::body(words))
                .on_press(Message::Attachment(e))
                .padding([4, 10])
                .style(kentos_ui::style::button::secondary)
        };
        list = list.push(
            row![
                add("Dosya ekle…", Event::Add(slot)),
                add("Adres ekle…", Event::AddLink(slot))
            ]
            .spacing(6),
        );
    }
    list.into()
}

#[cfg(test)]
mod tests {
    use kentos_extensions::ui_panel;
    use serde_json::json;

    use super::super::panels::OpenPanel;
    use crate::app::{App, Message};

    const RECORDS: &str = include_str!("../../../../../fixtures/kcad/v2/records.json");
    const BINA: &str = "0192f5a1-25a0-7000-8000-000000000301";

    /// The list is of the record its picker holds: none picked, none shown; the building picked, its address attachment
    /// is listed (its photos would be decoded with the shell's), and its layer takes the user's adding.
    #[test]
    fn a_panels_attachments_are_the_picked_records() {
        let (mut app, _) = App::boot(None);
        let mut snapshot: serde_json::Value = serde_json::from_str(RECORDS).expect("reads");
        snapshot["entities"][0]["attachments"] =
            json!([{ "name": "Kat planı", "url": "https://harita.ornek.bel.tr/a-blok" }]);
        let snapshot: kentos_commands::DocumentSnapshotV2 =
            serde_json::from_value(snapshot).expect("a drawing");
        let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        let value = json!({ "format": ui_panel::FORMAT, "schemaVersion": ui_panel::VERSION, "title": "Bina",
            "body": [
                { "type": "featurePicker", "id": "bina", "label": "Bina", "multiple": false },
                { "type": "attachments", "id": "ekler", "label": "Ekler", "of": "bina" }
            ] });
        app.extensions.panels.push(OpenPanel::new(
            "tr.ornek.deneme",
            "bina",
            ui_panel::read(&value).expect("a panel"),
        ));
        assert!(app.panel_attachment_slots().is_empty(), "nothing picked");
        app.extensions.panels[0]
            .picks
            .insert("bina".into(), vec![BINA.into()]);
        let slots = app.panel_attachment_slots();
        assert_eq!(slots.len(), 1);
        let views = app.attachment_views(slots[0]);
        let names: Vec<&str> = views.iter().map(|v| v.attachment.name.as_str()).collect();
        assert_eq!(names, ["Kat planı"]);
        assert!(app.attachable(slots[0]));
        let p = &app.extensions.panels[0];
        let _ = super::attachments_view(&app, p, Some("Ekler"), "bina");
    }
}
