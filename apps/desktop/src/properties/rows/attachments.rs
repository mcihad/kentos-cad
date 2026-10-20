//! Öznitelikler's Ekler and Eylemler (docs/adr/0225 §2, §4; the web's
//! `ui/properties/attachmentRows.ts`): with one object selected, a row each
//! attachment (its kind and number, its name, a photo's picture or its
//! kind's icon, which opens it, and its menu: Görüntüle or Aç, Düzenle…, Göm
//! for a linked photo, Kaldır; its note under it), then Ekle with Dosya
//! ekle… and Adres ekle…; on a locked layer only opening. Then a row each
//! action of the object's layer: its name, its target filled for the object,
//! Aç and Kopyala in its menu; a broken template's error under it.

use std::borrow::Cow;

use kentos_commands::AttachmentKind;
use kentos_domain::Slot;
use kentos_expression::template::TargetKind;

use super::{Choice, Editor, Row, Section, Thumb};
use crate::app::{App, Message};
use crate::attachments::{Event, kind_icon, kind_word};

fn attachment(e: Event) -> Message {
    Message::Attachment(e)
}

/// The sections of the object at `slot`: Ekler (none on a locked layer
/// without attachments) and Eylemler (none without actions).
pub(crate) fn sections(app: &App, slot: Slot) -> Vec<Section> {
    let Some(doc) = &app.document else {
        return Vec::new();
    };
    let Some(e) = doc.model.get(slot) else {
        return Vec::new();
    };
    let locked = doc.model.layers().is_locked(&e.base().layer_id);
    let views = app.attachment_views(slot);
    let count = views.len();
    let mut rows = Vec::new();
    for (k, v) in views.into_iter().enumerate() {
        let photo = v.kind == AttachmentKind::Photo;
        let mut items = vec![Choice::Action {
            label: if photo { "Görüntüle" } else { "Aç" },
            icon: if photo { "photo" } else { "externalOpen" },
            message: Some(attachment(Event::Open(slot, v.index))),
        }];
        if !locked {
            items.push(Choice::Action {
                label: "Düzenle…",
                icon: "edit",
                message: Some(attachment(Event::Edit(slot, v.index))),
            });
            if photo && v.attachment.file.is_some() {
                items.push(Choice::Action {
                    label: "Göm",
                    icon: "attachment",
                    message: Some(attachment(Event::Embed(slot, v.index))),
                });
            }
            items.push(Choice::Action {
                label: "Kaldır",
                icon: "trash",
                message: Some(attachment(Event::Remove(slot, v.index))),
            });
        }
        rows.push(Row {
            thumb: Some(Thumb {
                key: v.key.clone(),
                icon: kind_icon(v.kind),
                title: v.source.clone(),
                open: attachment(Event::Open(slot, v.index)),
            }),
            editor: Some(Editor::Select {
                text: v.attachment.name.clone(),
                swatch: None,
                icon: None,
                items,
            }),
            ..Row::text(
                format!("{} {}", kind_word(v.kind), k + 1),
                v.attachment.name.clone(),
            )
        });
        if let Some(note) = v
            .attachment
            .note
            .as_deref()
            .filter(|n| !n.trim().is_empty())
        {
            rows.push(Row::note(&note.replace('\n', " ⏎ ")));
        }
    }
    if !locked {
        rows.push(Row {
            editor: Some(Editor::Select {
                text: "Dosya ya da adres…".to_owned(),
                swatch: None,
                icon: Some("attachment"),
                items: vec![
                    Choice::Action {
                        label: "Dosya ekle…",
                        icon: "attachment",
                        message: Some(attachment(Event::Add(slot))),
                    },
                    Choice::Action {
                        label: "Adres ekle…",
                        icon: "attachmentLink",
                        message: Some(attachment(Event::AddLink(slot))),
                    },
                ],
            }),
            ..Row::text("Ekle", "Dosya ya da adres")
        });
    }
    let mut out = Vec::new();
    if !rows.is_empty() {
        out.push(Section {
            id: "attachments",
            title: if count > 0 {
                Cow::Owned(format!("Ekler ({count})"))
            } else {
                Cow::Borrowed("Ekler")
            },
            rows,
        });
    }
    let actions = app.object_actions(e, None);
    if !actions.is_empty() {
        let mut rows = Vec::new();
        for (i, a) in actions.into_iter().enumerate() {
            let mut items = vec![Choice::Action {
                label: "Aç",
                icon: "externalOpen",
                message: a
                    .error
                    .is_none()
                    .then(|| Message::InfoCard(crate::info_card::Event::Action(slot, i))),
            }];
            if let Some(target) = a.target.clone().filter(|t| !t.is_empty()) {
                items.push(Choice::Action {
                    label: "Kopyala",
                    icon: "copy",
                    message: Some(Message::InfoCard(crate::info_card::Event::Copy(target))),
                });
            }
            rows.push(Row {
                editor: Some(Editor::Select {
                    text: a
                        .target
                        .clone()
                        .unwrap_or_else(|| "Şablon bozuk".to_owned()),
                    swatch: None,
                    icon: Some(if a.kind == TargetKind::File {
                        "document"
                    } else {
                        "externalOpen"
                    }),
                    items,
                }),
                ..Row::text(
                    a.name.clone(),
                    a.target.clone().unwrap_or_else(|| "—".to_owned()),
                )
            });
            if let Some(error) = &a.error {
                rows.push(Row {
                    warn: true,
                    ..Row::note(error)
                });
            }
        }
        out.push(Section {
            id: "actions",
            title: Cow::Borrowed("Eylemler"),
            rows,
        });
    }
    out
}
